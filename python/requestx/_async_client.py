import contextlib as _contextlib

from ._core import (
    URL,
    AsyncClient as _AsyncClient,
    Response as _Response,
    AsyncHTTPTransport,
    InvalidURL,
)
from ._compat import (
    USE_CLIENT_DEFAULT,
)
from ._exceptions import (
    _convert_exception,
    TooManyRedirects,
    PoolTimeout,
    UnsupportedProtocol,
    RemoteProtocolError,
    _RequestError,
    _TransportError,
    _TimeoutException,
    _NetworkError,
    _ConnectError,
    _ReadError,
    _WriteError,
    _CloseError,
    _ProxyError,
    _ProtocolError,
    _UnsupportedProtocol,
    _DecodingError,
    _TooManyRedirects,
    _StreamError,
    _ConnectTimeout,
    _ReadTimeout,
    _WriteTimeout,
    _PoolTimeout,
    _LocalProtocolError,
    _RemoteProtocolError,
)
from ._request import _WrappedRequest
from ._response import Response
from ._auth import (
    BasicAuth,
    _convert_auth,
    _normalize_auth,
    _extract_auth_from_url,
)
from ._client_common import (
    extract_cookies_from_response as _extract_cookies_from_response_impl,
    merge_url as _merge_url_impl,
    get_proxy_from_env as _get_proxy_from_env_impl,
    transport_for_url as _transport_for_url_impl,
)


class AsyncClient:
    """Async HTTP client that wraps the Rust implementation with proper auth sentinel handling."""

    def __init__(self, *args, **kwargs):
        import asyncio as _asyncio_mod

        # Extract limits and timeout for pool semaphore before Rust consumes them
        _limits_arg = kwargs.get("limits", None)
        _timeout_arg = kwargs.get("timeout", None)

        _max_connections = None
        if _limits_arg is not None and hasattr(_limits_arg, "max_connections"):
            _max_connections = _limits_arg.max_connections

        _pool_timeout = None
        if _timeout_arg is not None and hasattr(_timeout_arg, "pool"):
            _pool_timeout = _timeout_arg.pool

        self._pool_semaphore = (
            _asyncio_mod.Semaphore(_max_connections)
            if _max_connections is not None
            else None
        )
        self._pool_timeout = _pool_timeout

        # Extract auth from kwargs before passing to Rust client
        auth = kwargs.pop("auth", None)
        # Validate and convert auth value
        if auth is None:
            self._auth = None
        elif isinstance(auth, tuple) and len(auth) == 2:
            self._auth = BasicAuth(auth[0], auth[1])
        elif (
            callable(auth)
            or hasattr(auth, "sync_auth_flow")
            or hasattr(auth, "async_auth_flow")
        ):
            self._auth = auth
        else:
            raise TypeError(
                f"Invalid 'auth' argument. Expected (username, password) tuple, Auth instance, or callable. Got {type(auth).__name__}."
            )

        # Extract proxy and mounts from kwargs
        proxy = kwargs.pop("proxy", None)
        mounts = kwargs.pop("mounts", None)
        trust_env = kwargs.get("trust_env", True)

        # Validate mount keys (must end with "://")
        if mounts:
            for key in mounts.keys():
                if not key.endswith("://") and "://" not in key:
                    raise ValueError(
                        f"Proxy keys must end with '://'. Got {key!r}. "
                        f"Did you mean '{key}://'?"
                    )

        # Store mounts dictionary
        self._mounts = mounts or {}

        # Extract verify parameter for transport (default True)
        verify = kwargs.pop("verify", True)

        # Create default transport (with proxy if specified)
        custom_transport = kwargs.get("transport", None)
        if custom_transport is not None:
            self._default_transport = custom_transport
        elif proxy is not None:
            self._default_transport = AsyncHTTPTransport(verify=verify, proxy=proxy)
        else:
            # Check for proxy env vars if trust_env is True
            env_proxy = None
            if trust_env:
                env_proxy = _get_proxy_from_env_impl()
            if env_proxy:
                self._default_transport = AsyncHTTPTransport(verify=verify, proxy=env_proxy)
            else:
                self._default_transport = AsyncHTTPTransport(verify=verify)

        self._custom_transport = (
            custom_transport  # Keep reference to user-provided transport
        )

        # Extract and store follow_redirects from kwargs before passing to Rust
        self._follow_redirects = kwargs.pop("follow_redirects", False)

        # Always create Rust client with follow_redirects=False so Python handles redirects
        # This allows proper logging and history tracking
        kwargs["follow_redirects"] = False
        # Pass verify to Rust client so it creates its reqwest client with proper TLS settings
        kwargs["verify"] = verify
        self._client = _AsyncClient(*args, **kwargs)
        self._is_closed = False

    @property
    def _transport(self):
        """Get the default transport for this client."""
        return self._default_transport

    def _transport_for_url(self, url):
        return _transport_for_url_impl(self, url, AsyncHTTPTransport)

    async def _invoke_request_hooks(self, request):
        """Invoke all request event hooks (handles both sync and async hooks)."""
        import inspect

        hooks = self.event_hooks.get("request", [])
        for hook in hooks:
            result = hook(request)
            if inspect.iscoroutine(result):
                await result

    async def _invoke_response_hooks(self, response):
        """Invoke all response event hooks (handles both sync and async hooks)."""
        import inspect

        hooks = self.event_hooks.get("response", [])
        for hook in hooks:
            try:
                result = hook(response)
                if inspect.iscoroutine(result):
                    await result
            except BaseException:
                # Close the response when a hook raises an exception
                await response.aclose()
                raise

    def __getattr__(self, name):
        """Delegate attribute access to the underlying client."""
        return getattr(self._client, name)

    async def __aenter__(self):
        if self._is_closed:
            raise RuntimeError("Cannot open a client that has been closed")
        # Call transport's __aenter__ if it exists
        if self._custom_transport is not None and hasattr(
            self._custom_transport, "__aenter__"
        ):
            await self._custom_transport.__aenter__()
        # Call __aenter__ on all mounted transports
        for transport in self._mounts.values():
            if hasattr(transport, "__aenter__"):
                await transport.__aenter__()
        await self._client.__aenter__()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        result = await self._client.__aexit__(exc_type, exc_val, exc_tb)
        # Call transport's __aexit__ if it exists
        if self._custom_transport is not None and hasattr(
            self._custom_transport, "__aexit__"
        ):
            await self._custom_transport.__aexit__(exc_type, exc_val, exc_tb)
        # Call __aexit__ on all mounted transports
        for transport in self._mounts.values():
            if hasattr(transport, "__aexit__"):
                await transport.__aexit__(exc_type, exc_val, exc_tb)
        self._is_closed = True
        return result

    async def aclose(self):
        """Close the client."""
        if hasattr(self._client, "aclose"):
            await self._client.aclose()
        if self._custom_transport is not None and hasattr(
            self._custom_transport, "aclose"
        ):
            await self._custom_transport.aclose()
        # Close all mounted transports
        for transport in self._mounts.values():
            if hasattr(transport, "aclose"):
                await transport.aclose()
        self._is_closed = True

    @property
    def is_closed(self):
        """Return True if the client has been closed."""
        return getattr(self, "_is_closed", False)

    def _check_closed(self):
        """Raise RuntimeError if the client is closed."""
        if self._is_closed:
            raise RuntimeError("Cannot send request on a closed client")

    async def _acquire_pool_permit(self):
        """Acquire a connection slot from the pool semaphore."""
        if self._pool_semaphore is None:
            return
        import asyncio as _asyncio_mod

        if self._pool_timeout is not None:
            try:
                await _asyncio_mod.wait_for(
                    self._pool_semaphore.acquire(), timeout=self._pool_timeout
                )
            except _asyncio_mod.TimeoutError:
                raise PoolTimeout("Timed out waiting for a connection from the pool")
        else:
            await self._pool_semaphore.acquire()

    def _release_pool_permit(self):
        """Release a connection slot back to the pool semaphore."""
        if self._pool_semaphore is not None:
            self._pool_semaphore.release()

    def _warn_per_request_cookies(self, cookies):
        """Emit deprecation warning for per-request cookies."""
        if cookies is not None:
            import warnings

            warnings.warn(
                "Setting per-request cookies is deprecated. Use `client.cookies` instead.",
                DeprecationWarning,
                stacklevel=4,  # go up to user code
            )

    def _extract_cookies_from_response(self, response, request):
        _extract_cookies_from_response_impl(self, response, request)

    @property
    def base_url(self):
        return self._client.base_url

    @base_url.setter
    def base_url(self, value):
        self._client.base_url = value

    @property
    def headers(self):
        return self._client.headers

    @headers.setter
    def headers(self, value):
        self._client.headers = value

    @property
    def cookies(self):
        return self._client.cookies

    @cookies.setter
    def cookies(self, value):
        self._client.cookies = value

    @property
    def timeout(self):
        return self._client.timeout

    @timeout.setter
    def timeout(self, value):
        self._client.timeout = value

    @property
    def event_hooks(self):
        return self._client.event_hooks

    @event_hooks.setter
    def event_hooks(self, value):
        self._client.event_hooks = value

    @property
    def trust_env(self):
        return self._client.trust_env

    @trust_env.setter
    def trust_env(self, value):
        self._client.trust_env = value

    @property
    def auth(self):
        return self._auth

    @auth.setter
    def auth(self, value):
        # Validate and convert auth value
        if value is None:
            self._auth = None
        elif isinstance(value, tuple) and len(value) == 2:
            self._auth = BasicAuth(value[0], value[1])
        elif (
            callable(value)
            or hasattr(value, "sync_auth_flow")
            or hasattr(value, "async_auth_flow")
        ):
            self._auth = value
        else:
            raise TypeError(
                f"Invalid 'auth' argument. Expected (username, password) tuple, Auth instance, or callable. Got {type(value).__name__}."
            )

    def build_request(self, method, url, **kwargs):
        """Build a Request object - wrap result in Python Request class."""
        # Check for sync iterator/generator in content (AsyncClient can't handle these)
        import inspect

        content = kwargs.get("content")
        if content is not None:
            if inspect.isgenerator(content):
                raise RuntimeError(
                    "Attempted to send an sync request with an AsyncClient instance."
                )
            # Also check for sync iterator protocol (but not strings/bytes which have __iter__)
            if (
                hasattr(content, "__next__")
                and hasattr(content, "__iter__")
                and not isinstance(content, (str, bytes, bytearray))
            ):
                raise RuntimeError(
                    "Attempted to send an sync request with an AsyncClient instance."
                )
        # Validate URL before processing
        url_str = str(url)
        # Check for empty scheme (like '://example.org')
        if url_str.startswith("://"):
            raise UnsupportedProtocol(
                "Request URL is missing an 'http://' or 'https://' protocol."
            )
        # Check for missing host (like 'http://' or 'http:///path')
        if url_str.startswith("http://") or url_str.startswith("https://"):
            # Extract the part after scheme
            after_scheme = url_str.split("://", 1)[1] if "://" in url_str else ""
            # Empty host or starts with / means no host
            if not after_scheme or after_scheme.startswith("/"):
                raise UnsupportedProtocol(
                    "Request URL is missing an 'http://' or 'https://' protocol."
                )
        # Handle URL merging with base_url
        merged_url = self._merge_url(url)
        # Filter to only parameters supported by Rust build_request
        supported_kwargs = {}
        if "content" in kwargs and kwargs["content"] is not None:
            supported_kwargs["content"] = kwargs["content"]
        if "params" in kwargs and kwargs["params"] is not None:
            supported_kwargs["params"] = kwargs["params"]
        if "headers" in kwargs and kwargs["headers"] is not None:
            supported_kwargs["headers"] = kwargs["headers"]
        # Handle data, files, json by converting to content
        if "json" in kwargs and kwargs["json"] is not None:
            import json as json_module

            supported_kwargs["content"] = json_module.dumps(kwargs["json"]).encode(
                "utf-8"
            )
            # Add content-type header for JSON
            if "headers" not in supported_kwargs:
                supported_kwargs["headers"] = {}
            if isinstance(supported_kwargs.get("headers"), dict):
                supported_kwargs["headers"] = {
                    **supported_kwargs["headers"],
                    "content-type": "application/json",
                }
        if "data" in kwargs and kwargs["data"] is not None:
            data = kwargs["data"]
            if isinstance(data, dict):
                from urllib.parse import urlencode

                supported_kwargs["content"] = urlencode(data).encode("utf-8")
                if "headers" not in supported_kwargs:
                    supported_kwargs["headers"] = {}
                if isinstance(supported_kwargs.get("headers"), dict):
                    supported_kwargs["headers"] = {
                        **supported_kwargs["headers"],
                        "content-type": "application/x-www-form-urlencoded",
                    }
            elif isinstance(data, (bytes, str)):
                supported_kwargs["content"] = (
                    data if isinstance(data, bytes) else data.encode("utf-8")
                )
        rust_request = self._client.build_request(
            method, merged_url, **supported_kwargs
        )
        # Create a wrapper that delegates to the Rust request but has our headers proxy
        return _WrappedRequest(rust_request)

    def _merge_url(self, url):
        return _merge_url_impl(self, url)

    async def send(self, request, **kwargs):
        """Send a Request object."""
        await self._acquire_pool_permit()
        try:
            auth = kwargs.pop("auth", None)
            if auth is not None:
                return await self._send_with_auth(request, auth)
            return await self._send_single_request(request)
        finally:
            self._release_pool_permit()

    async def _send_single_request(self, request):
        """Send a single request, handling transport properly."""
        if self._is_closed:
            raise RuntimeError("Cannot send request on a closed client")

        # Get the Rust request object
        if isinstance(request, _WrappedRequest):
            rust_request = request._rust_request
            request_url = request.url
        elif hasattr(request, "_rust_request"):
            rust_request = request._rust_request
            request_url = request.url if hasattr(request, "url") else None
        else:
            rust_request = request
            request_url = request.url if hasattr(request, "url") else None

        # Invoke request event hooks before sending
        await self._invoke_request_hooks(request)

        # Get the appropriate transport for this URL
        # First check if there's a mounted transport for this URL
        transport = self._transport_for_url(request_url)

        # Check if we need to use a custom transport (mounted or user-provided)
        # Mounted transports take precedence over the custom transport
        use_custom = transport is not self._default_transport
        if not use_custom and self._custom_transport is not None:
            # No mount matched, use the custom transport
            transport = self._custom_transport
            use_custom = True

        # If we have a custom/mounted transport, use it directly
        if use_custom and transport is not None:
            # For wrapped requests with async streams, pass the wrapper (for stream access)
            request_to_send = (
                request
                if isinstance(request, _WrappedRequest)
                and request._async_stream is not None
                else rust_request
            )
            # Check for async handle method
            if hasattr(transport, "handle_async_request"):
                result = await transport.handle_async_request(request_to_send)
            elif hasattr(transport, "handle_request"):
                result = transport.handle_request(request_to_send)
            elif callable(transport):
                result = transport(request_to_send)
            else:
                raise TypeError(
                    "Transport must have handle_async_request or handle_request method"
                )

            # Wrap result in Response if needed
            if isinstance(result, Response):
                response = result
            elif isinstance(result, _Response):
                response = Response(result)
            else:
                response = Response(result)

            # Set the URL from the request if not already set
            if response._url is None and hasattr(rust_request, "url"):
                response._url = rust_request.url
            # Store the original request
            if response._request is None:
                if isinstance(request, _WrappedRequest):
                    response._request = request
                else:
                    response._request = (
                        _WrappedRequest(rust_request)
                        if hasattr(rust_request, "url")
                        else request
                    )

            # For redirect responses, compute next_request
            if response.status_code in (301, 302, 303, 307, 308):
                location = response.headers.get("location")
                if location:
                    # Build the redirect request
                    response._next_request = self._build_redirect_request(
                        request, response
                    )

            # If response has a stream that hasn't been read, read it now
            # This ensures exceptions during iteration are raised and stream is closed
            if response._stream_content is not None:
                stream_obj = getattr(response, "_stream_object", None)
                try:
                    chunks = []
                    async for chunk in response._stream_content:
                        chunks.append(chunk)
                    response._raw_content = b"".join(chunks)
                    response._stream_content = None
                    response._stream_consumed = True
                    response._response._set_content(response._raw_content)
                except BaseException:
                    # Close the stream on any exception (including KeyboardInterrupt)
                    if stream_obj is not None and hasattr(stream_obj, "aclose"):
                        await stream_obj.aclose()
                    raise

            # Invoke response event hooks before returning
            await self._invoke_response_hooks(response)
            return response
        else:
            # Use the Rust client's send
            try:
                result = await self._client.send(rust_request)
                response = Response(result)
            except (
                _RequestError,
                _TransportError,
                _TimeoutException,
                _NetworkError,
                _ConnectError,
                _ReadError,
                _WriteError,
                _CloseError,
                _ProxyError,
                _ProtocolError,
                _UnsupportedProtocol,
                _DecodingError,
                _TooManyRedirects,
                _StreamError,
                _ConnectTimeout,
                _ReadTimeout,
                _WriteTimeout,
                _PoolTimeout,
                _LocalProtocolError,
                _RemoteProtocolError,
            ) as e:
                raise _convert_exception(e) from None

            # Set URL and request on response
            if response._url is None and hasattr(rust_request, "url"):
                response._url = rust_request.url
            if response._request is None:
                if isinstance(request, _WrappedRequest):
                    response._request = request
                else:
                    response._request = (
                        _WrappedRequest(rust_request)
                        if hasattr(rust_request, "url")
                        else request
                    )

            # Build next_request if this is a redirect
            if response.status_code in (301, 302, 303, 307, 308):
                location = response.headers.get("location")
                if location:
                    response._next_request = self._build_redirect_request(
                        request, response
                    )

            # Invoke response event hooks before returning
            await self._invoke_response_hooks(response)
            return response

    async def _send_handling_redirects(
        self, request, follow_redirects=False, history=None
    ):
        """Send a request, optionally following redirects."""
        if history is None:
            history = []

        # Get original request URL for fragment preservation
        original_url = request.url if hasattr(request, "url") else None
        original_fragment = None
        if original_url and isinstance(original_url, URL):
            original_fragment = original_url.fragment

        response = await self._send_single_request(request)

        # Extract cookies from response and add to client cookies
        self._extract_cookies_from_response(response, request)

        if not follow_redirects or not response.is_redirect:
            response._history = list(history)
            return response

        # Check max redirects
        if len(history) >= 20:
            raise TooManyRedirects("Too many redirects")

        # Add current response to history
        response._history = list(history)
        history = history + [response]

        # Get next request
        next_request = response.next_request
        if next_request is None:
            return response

        # Preserve fragment from original URL
        if original_fragment:
            next_url = next_request.url if hasattr(next_request, "url") else None
            if next_url and isinstance(next_url, URL):
                if not next_url.fragment:
                    next_url_str = str(next_url)
                    if "#" not in next_url_str:
                        next_request = self.build_request(
                            next_request.method,
                            next_url_str + "#" + original_fragment,
                            headers=(
                                dict(next_request.headers.items())
                                if hasattr(next_request, "headers")
                                else None
                            ),
                            content=(
                                next_request.content
                                if hasattr(next_request, "content")
                                else None
                            ),
                        )

        # Recursively follow
        return await self._send_handling_redirects(
            next_request, follow_redirects=True, history=history
        )

    async def _send_with_auth(self, request, auth, follow_redirects=False):
        """Send a request with async auth flow handling."""
        # Ensure we have a wrapped request for proper header mutation
        if isinstance(request, _WrappedRequest):
            wrapped_request = request
        else:
            wrapped_request = _WrappedRequest(request)

        # Get the auth flow generator
        # For Rust auth classes (BasicAuth, DigestAuth), pass the underlying Rust request
        # For Python auth classes (generators), pass the wrapped request
        auth_flow = None
        requires_response_body = getattr(auth, "requires_response_body", False)
        if auth is not None:
            import inspect

            auth_type = type(auth)
            # First check if auth_flow is overridden in a Python subclass (for custom auth like RepeatAuth)
            if "auth_flow" in auth_type.__dict__:
                auth_flow_method = getattr(auth, "auth_flow", None)
                if auth_flow_method and (
                    inspect.isgeneratorfunction(auth_flow_method)
                    or (
                        hasattr(auth_flow_method, "__func__")
                        and inspect.isgeneratorfunction(auth_flow_method.__func__)
                    )
                ):
                    auth_flow = auth.auth_flow(wrapped_request)
            # Then check for async_auth_flow
            if auth_flow is None and hasattr(auth, "async_auth_flow"):
                method = getattr(auth, "async_auth_flow")
                # Check if it's a generator function (Python auth) or not (Rust auth)
                if inspect.isgeneratorfunction(method) or inspect.isasyncgenfunction(
                    method
                ):
                    auth_flow = auth.async_auth_flow(wrapped_request)
                else:
                    # Check if async_auth_flow is overridden in Python class
                    if "async_auth_flow" in auth_type.__dict__:
                        auth_flow = auth.async_auth_flow(wrapped_request)
                    else:
                        # Rust auth - pass the underlying request
                        auth_flow = auth.async_auth_flow(wrapped_request._rust_request)
            elif auth_flow is None and hasattr(auth, "sync_auth_flow"):
                method = getattr(auth, "sync_auth_flow")
                if inspect.isgeneratorfunction(method):
                    auth_flow = auth.sync_auth_flow(wrapped_request)
                else:
                    # Check if sync_auth_flow is overridden in Python class
                    if "sync_auth_flow" in auth_type.__dict__:
                        auth_flow = auth.sync_auth_flow(wrapped_request)
                    else:
                        # Rust auth - pass the underlying request
                        auth_flow = auth.sync_auth_flow(wrapped_request._rust_request)

        if auth_flow is None:
            # No auth flow, send with redirect handling
            return await self._send_handling_redirects(
                wrapped_request, follow_redirects=follow_redirects
            )

        # Check if auth_flow returned a list (Rust base class) or generator
        if isinstance(auth_flow, (list, tuple)):
            # Simple list of requests - just send the last one
            last_request = wrapped_request
            for req in auth_flow:
                last_request = req
            return await self._send_handling_redirects(
                last_request, follow_redirects=follow_redirects
            )

        # Generator-based auth flow
        history = []
        try:
            # Check if it's an async generator
            if hasattr(auth_flow, "__anext__"):
                # Async generator
                request = await auth_flow.__anext__()
                response = await self._send_single_request(request)
                # Read response body if requires_response_body is True
                if requires_response_body:
                    await response.aread()

                while True:
                    try:
                        request = await auth_flow.asend(response)
                        response._history = list(history)
                        history.append(response)
                        response = await self._send_single_request(request)
                        if requires_response_body:
                            await response.aread()
                    except StopAsyncIteration:
                        break
            else:
                # Sync generator
                request = next(auth_flow)
                response = await self._send_single_request(request)
                # Read response body if requires_response_body is True
                if requires_response_body:
                    await response.aread()

                while True:
                    try:
                        request = auth_flow.send(response)
                        response._history = list(history)
                        history.append(response)
                        response = await self._send_single_request(request)
                        if requires_response_body:
                            await response.aread()
                    except StopIteration:
                        break

            if history:
                response._history = history

            # After auth completes, handle redirects if needed
            if follow_redirects and response.is_redirect:
                return await self._send_handling_redirects(
                    response.next_request, follow_redirects=True, history=history
                )
            return response
        except (StopIteration, StopAsyncIteration):
            return await self._send_handling_redirects(
                wrapped_request, follow_redirects=follow_redirects
            )

    async def get(
        self,
        url,
        *,
        params=None,
        headers=None,
        cookies=None,
        auth=USE_CLIENT_DEFAULT,
        follow_redirects=None,
        timeout=None,
    ):
        """HTTP GET with proper auth sentinel handling."""
        self._check_closed()
        await self._acquire_pool_permit()
        try:
            actual_auth = _normalize_auth(
                auth if auth is not USE_CLIENT_DEFAULT else self._auth
            )
            # Extract auth from URL userinfo if no explicit auth provided
            if actual_auth is None:
                actual_auth = _extract_auth_from_url(str(url))

            # Determine follow_redirects behavior
            actual_follow = (
                follow_redirects
                if follow_redirects is not None
                else self._follow_redirects
            )

            # If we have a custom transport, route through redirect handling
            if self._custom_transport is not None:
                request = self.build_request("GET", url, params=params, headers=headers)
                if actual_auth is not None:
                    return await self._send_with_auth(
                        request, actual_auth, follow_redirects=bool(actual_follow)
                    )
                return await self._send_handling_redirects(
                    request, follow_redirects=bool(actual_follow)
                )

            if actual_auth is not None:
                result = await self._handle_auth(
                    "GET", url, actual_auth, params=params, headers=headers
                )
                if result is not None:
                    return result
            try:
                response = await self._client.get(
                    url,
                    params=params,
                    headers=headers,
                    cookies=cookies,
                    auth=_convert_auth(auth),
                    follow_redirects=follow_redirects,
                    timeout=timeout,
                )
                return Response(response)
            except (
                _RequestError,
                _TransportError,
                _TimeoutException,
                _NetworkError,
                _ConnectError,
                _ReadError,
                _WriteError,
                _CloseError,
                _ProxyError,
                _ProtocolError,
                _UnsupportedProtocol,
                _DecodingError,
                _TooManyRedirects,
                _StreamError,
                _ConnectTimeout,
                _ReadTimeout,
                _WriteTimeout,
                _PoolTimeout,
                _LocalProtocolError,
                _RemoteProtocolError,
            ) as e:
                raise _convert_exception(e) from None
        finally:
            self._release_pool_permit()

    def _build_redirect_request(self, request, response):
        """Build the next request for following a redirect."""
        location = response.headers.get("location")
        if not location:
            return None

        # Get the original request URL
        if hasattr(request, "url"):
            original_url = request.url
        else:
            original_url = None

        # Check for invalid characters in location (non-ASCII in host)
        try:
            if location.startswith("//") or location.startswith("/"):
                pass  # Relative URL - will be joined with original
            elif "://" in location:
                from urllib.parse import urlparse

                parsed = urlparse(location)
                if parsed.netloc:
                    host_part = parsed.hostname or ""
                    try:
                        host_part.encode("ascii")
                    except UnicodeEncodeError:
                        raise RemoteProtocolError(f"Invalid redirect URL: {location}")
        except RemoteProtocolError:
            raise
        except Exception:
            pass

        # Parse location - handle relative and absolute URLs
        redirect_url = None
        try:
            if original_url:
                if isinstance(original_url, URL):
                    redirect_url = original_url.join(location)
                else:
                    redirect_url = URL(original_url).join(location)
            else:
                redirect_url = URL(location)
        except InvalidURL as e:
            if "empty host" in str(e).lower() and original_url:
                from urllib.parse import urlparse

                parsed = urlparse(location)
                orig_url = (
                    original_url
                    if isinstance(original_url, URL)
                    else URL(str(original_url))
                )
                scheme = parsed.scheme or orig_url.scheme
                host = orig_url.host
                port = parsed.port if parsed.port else None
                path = parsed.path or "/"
                if port:
                    redirect_url_str = f"{scheme}://{host}:{port}{path}"
                else:
                    redirect_url_str = f"{scheme}://{host}{path}"
                if parsed.query:
                    redirect_url_str += f"?{parsed.query}"
                try:
                    redirect_url = URL(redirect_url_str)
                except Exception:
                    raise RemoteProtocolError(f"Invalid redirect URL: {location}")
            else:
                raise RemoteProtocolError(f"Invalid redirect URL: {location}")
        except Exception:
            raise RemoteProtocolError(f"Invalid redirect URL: {location}")

        # Check scheme
        scheme = redirect_url.scheme
        if scheme not in ("http", "https"):
            raise UnsupportedProtocol(f"Scheme {scheme!r} not supported.")

        # Determine method for redirect
        status_code = response.status_code
        method = request.method if hasattr(request, "method") else "GET"

        # 301, 302, 303 redirects change method to GET (except for GET/HEAD)
        if status_code in (301, 302, 303) and method not in ("GET", "HEAD"):
            method = "GET"

        # Build kwargs for new request
        headers = dict(request.headers.items()) if hasattr(request, "headers") else {}

        # Remove Host header so it gets set correctly for the new URL
        headers.pop("host", None)
        headers.pop("Host", None)

        # Strip Authorization header on cross-domain redirects
        if original_url:
            orig_host = (
                original_url.host
                if isinstance(original_url, URL)
                else URL(str(original_url)).host
            )
            new_host = redirect_url.host
            if orig_host != new_host:
                headers.pop("authorization", None)
                headers.pop("Authorization", None)

        # For 301, 302, 303, don't include body and remove content-length
        content = None
        if status_code in (301, 302, 303):
            headers.pop("content-length", None)
            headers.pop("Content-Length", None)
        elif hasattr(request, "content"):
            content = request.content

        return self.build_request(
            method, str(redirect_url), headers=headers, content=content
        )

    async def _handle_auth(self, method, url, actual_auth, **build_kwargs):
        """Handle auth for async requests - supports generators and callables."""
        # Convert tuple to BasicAuth
        if isinstance(actual_auth, tuple) and len(actual_auth) == 2:
            actual_auth = BasicAuth(actual_auth[0], actual_auth[1])

        request = self.build_request(method, url, **build_kwargs)
        if hasattr(actual_auth, "async_auth_flow") or hasattr(
            actual_auth, "sync_auth_flow"
        ):
            return await self._send_with_auth(request, actual_auth)
        elif callable(actual_auth):
            # Callable auth - call it with the wrapped request
            modified = actual_auth(request)
            return await self._send_single_request(
                modified if modified is not None else request
            )
        else:
            # Invalid auth type
            raise TypeError(
                f"Invalid 'auth' argument. Expected (username, password) tuple, Auth instance, or callable. Got {type(actual_auth).__name__}."
            )

    async def post(
        self,
        url,
        *,
        content=None,
        data=None,
        files=None,
        json=None,
        params=None,
        headers=None,
        cookies=None,
        auth=USE_CLIENT_DEFAULT,
        follow_redirects=None,
        timeout=None,
    ):
        """HTTP POST with proper auth sentinel handling."""
        self._check_closed()
        # Check for sync iterator/generator in content (AsyncClient can't handle these)
        import inspect

        async_stream = None
        if content is not None:
            if inspect.isgenerator(content):
                raise RuntimeError(
                    "Attempted to send an sync request with an AsyncClient instance."
                )
            if (
                hasattr(content, "__next__")
                and hasattr(content, "__iter__")
                and not isinstance(content, (str, bytes, bytearray))
            ):
                raise RuntimeError(
                    "Attempted to send an sync request with an AsyncClient instance."
                )
            # Handle async iterators/generators
            if inspect.isasyncgen(content) or (
                hasattr(content, "__aiter__") and hasattr(content, "__anext__")
            ):
                # Keep the async iterator for stream tracking (for auth retry detection)
                async_stream = content
                content = None  # Don't pass to Rust, keep in Python wrapper
        await self._acquire_pool_permit()
        try:
            actual_auth = _normalize_auth(
                auth if auth is not USE_CLIENT_DEFAULT else self._auth
            )
            if actual_auth is None:
                actual_auth = _extract_auth_from_url(str(url))

            # If we have a custom transport, route through _send_single_request
            if self._custom_transport is not None:
                request = self.build_request(
                    "POST",
                    url,
                    content=content,
                    data=data,
                    files=files,
                    json=json,
                    params=params,
                    headers=headers,
                )
                # If we had an async stream, wrap the request to track it
                if async_stream is not None and isinstance(request, _WrappedRequest):
                    request._async_stream = async_stream
                if actual_auth is not None:
                    return await self._send_with_auth(request, actual_auth)
                return await self._send_single_request(request)

            if actual_auth is not None:
                result = await self._handle_auth(
                    "POST",
                    url,
                    actual_auth,
                    content=content,
                    params=params,
                    headers=headers,
                )
                if result is not None:
                    return result
            try:
                response = await self._client.post(
                    url,
                    content=content,
                    data=data,
                    files=files,
                    json=json,
                    params=params,
                    headers=headers,
                    cookies=cookies,
                    auth=_convert_auth(auth),
                    follow_redirects=follow_redirects,
                    timeout=timeout,
                )
                return Response(response)
            except (
                _RequestError,
                _TransportError,
                _TimeoutException,
                _NetworkError,
                _ConnectError,
                _ReadError,
                _WriteError,
                _CloseError,
                _ProxyError,
                _ProtocolError,
                _UnsupportedProtocol,
                _DecodingError,
                _TooManyRedirects,
                _StreamError,
                _ConnectTimeout,
                _ReadTimeout,
                _WriteTimeout,
                _PoolTimeout,
                _LocalProtocolError,
                _RemoteProtocolError,
            ) as e:
                raise _convert_exception(e) from None
        finally:
            self._release_pool_permit()

    async def put(
        self,
        url,
        *,
        content=None,
        data=None,
        files=None,
        json=None,
        params=None,
        headers=None,
        cookies=None,
        auth=USE_CLIENT_DEFAULT,
        follow_redirects=None,
        timeout=None,
    ):
        """HTTP PUT with proper auth sentinel handling."""
        self._check_closed()
        await self._acquire_pool_permit()
        try:
            actual_auth = _normalize_auth(
                auth if auth is not USE_CLIENT_DEFAULT else self._auth
            )
            if actual_auth is None:
                actual_auth = _extract_auth_from_url(str(url))

            # If we have a custom transport, route through _send_single_request
            if self._custom_transport is not None:
                request = self.build_request(
                    "PUT",
                    url,
                    content=content,
                    data=data,
                    files=files,
                    json=json,
                    params=params,
                    headers=headers,
                )
                if actual_auth is not None:
                    return await self._send_with_auth(request, actual_auth)
                return await self._send_single_request(request)

            if actual_auth is not None:
                result = await self._handle_auth(
                    "PUT",
                    url,
                    actual_auth,
                    content=content,
                    params=params,
                    headers=headers,
                )
                if result is not None:
                    return result
            try:
                response = await self._client.put(
                    url,
                    content=content,
                    data=data,
                    files=files,
                    json=json,
                    params=params,
                    headers=headers,
                    cookies=cookies,
                    auth=_convert_auth(auth),
                    follow_redirects=follow_redirects,
                    timeout=timeout,
                )
                return Response(response)
            except (
                _RequestError,
                _TransportError,
                _TimeoutException,
                _NetworkError,
                _ConnectError,
                _ReadError,
                _WriteError,
                _CloseError,
                _ProxyError,
                _ProtocolError,
                _UnsupportedProtocol,
                _DecodingError,
                _TooManyRedirects,
                _StreamError,
                _ConnectTimeout,
                _ReadTimeout,
                _WriteTimeout,
                _PoolTimeout,
                _LocalProtocolError,
                _RemoteProtocolError,
            ) as e:
                raise _convert_exception(e) from None
        finally:
            self._release_pool_permit()

    async def patch(
        self,
        url,
        *,
        content=None,
        data=None,
        files=None,
        json=None,
        params=None,
        headers=None,
        cookies=None,
        auth=USE_CLIENT_DEFAULT,
        follow_redirects=None,
        timeout=None,
    ):
        """HTTP PATCH with proper auth sentinel handling."""
        self._check_closed()
        await self._acquire_pool_permit()
        try:
            actual_auth = _normalize_auth(
                auth if auth is not USE_CLIENT_DEFAULT else self._auth
            )
            if actual_auth is None:
                actual_auth = _extract_auth_from_url(str(url))

            # If we have a custom transport, route through _send_single_request
            if self._custom_transport is not None:
                request = self.build_request(
                    "PATCH",
                    url,
                    content=content,
                    data=data,
                    files=files,
                    json=json,
                    params=params,
                    headers=headers,
                )
                if actual_auth is not None:
                    return await self._send_with_auth(request, actual_auth)
                return await self._send_single_request(request)

            if actual_auth is not None:
                result = await self._handle_auth(
                    "PATCH",
                    url,
                    actual_auth,
                    content=content,
                    params=params,
                    headers=headers,
                )
                if result is not None:
                    return result
            try:
                response = await self._client.patch(
                    url,
                    content=content,
                    data=data,
                    files=files,
                    json=json,
                    params=params,
                    headers=headers,
                    cookies=cookies,
                    auth=_convert_auth(auth),
                    follow_redirects=follow_redirects,
                    timeout=timeout,
                )
                return Response(response)
            except (
                _RequestError,
                _TransportError,
                _TimeoutException,
                _NetworkError,
                _ConnectError,
                _ReadError,
                _WriteError,
                _CloseError,
                _ProxyError,
                _ProtocolError,
                _UnsupportedProtocol,
                _DecodingError,
                _TooManyRedirects,
                _StreamError,
                _ConnectTimeout,
                _ReadTimeout,
                _WriteTimeout,
                _PoolTimeout,
                _LocalProtocolError,
                _RemoteProtocolError,
            ) as e:
                raise _convert_exception(e) from None
        finally:
            self._release_pool_permit()

    async def delete(
        self,
        url,
        *,
        params=None,
        headers=None,
        cookies=None,
        auth=USE_CLIENT_DEFAULT,
        follow_redirects=None,
        timeout=None,
    ):
        """HTTP DELETE with proper auth sentinel handling."""
        self._check_closed()
        await self._acquire_pool_permit()
        try:
            actual_auth = _normalize_auth(
                auth if auth is not USE_CLIENT_DEFAULT else self._auth
            )
            if actual_auth is None:
                actual_auth = _extract_auth_from_url(str(url))

            # If we have a custom transport, route through _send_single_request
            if self._custom_transport is not None:
                request = self.build_request(
                    "DELETE", url, params=params, headers=headers
                )
                if actual_auth is not None:
                    return await self._send_with_auth(request, actual_auth)
                return await self._send_single_request(request)

            if actual_auth is not None:
                result = await self._handle_auth(
                    "DELETE", url, actual_auth, params=params, headers=headers
                )
                if result is not None:
                    return result
            try:
                response = await self._client.delete(
                    url,
                    params=params,
                    headers=headers,
                    cookies=cookies,
                    auth=_convert_auth(auth),
                    follow_redirects=follow_redirects,
                    timeout=timeout,
                )
                return Response(response)
            except (
                _RequestError,
                _TransportError,
                _TimeoutException,
                _NetworkError,
                _ConnectError,
                _ReadError,
                _WriteError,
                _CloseError,
                _ProxyError,
                _ProtocolError,
                _UnsupportedProtocol,
                _DecodingError,
                _TooManyRedirects,
                _StreamError,
                _ConnectTimeout,
                _ReadTimeout,
                _WriteTimeout,
                _PoolTimeout,
                _LocalProtocolError,
                _RemoteProtocolError,
            ) as e:
                raise _convert_exception(e) from None
        finally:
            self._release_pool_permit()

    async def head(
        self,
        url,
        *,
        params=None,
        headers=None,
        cookies=None,
        auth=USE_CLIENT_DEFAULT,
        follow_redirects=None,
        timeout=None,
    ):
        """HTTP HEAD with proper auth sentinel handling."""
        self._check_closed()
        await self._acquire_pool_permit()
        try:
            actual_auth = _normalize_auth(
                auth if auth is not USE_CLIENT_DEFAULT else self._auth
            )
            if actual_auth is None:
                actual_auth = _extract_auth_from_url(str(url))

            # If we have a custom transport, route through _send_single_request
            if self._custom_transport is not None:
                request = self.build_request(
                    "HEAD", url, params=params, headers=headers
                )
                if actual_auth is not None:
                    return await self._send_with_auth(request, actual_auth)
                return await self._send_single_request(request)

            if actual_auth is not None:
                result = await self._handle_auth(
                    "HEAD", url, actual_auth, params=params, headers=headers
                )
                if result is not None:
                    return result
            try:
                response = await self._client.head(
                    url,
                    params=params,
                    headers=headers,
                    cookies=cookies,
                    auth=_convert_auth(auth),
                    follow_redirects=follow_redirects,
                    timeout=timeout,
                )
                return Response(response)
            except (
                _RequestError,
                _TransportError,
                _TimeoutException,
                _NetworkError,
                _ConnectError,
                _ReadError,
                _WriteError,
                _CloseError,
                _ProxyError,
                _ProtocolError,
                _UnsupportedProtocol,
                _DecodingError,
                _TooManyRedirects,
                _StreamError,
                _ConnectTimeout,
                _ReadTimeout,
                _WriteTimeout,
                _PoolTimeout,
                _LocalProtocolError,
                _RemoteProtocolError,
            ) as e:
                raise _convert_exception(e) from None
        finally:
            self._release_pool_permit()

    async def options(
        self,
        url,
        *,
        params=None,
        headers=None,
        cookies=None,
        auth=USE_CLIENT_DEFAULT,
        follow_redirects=None,
        timeout=None,
    ):
        """HTTP OPTIONS with proper auth sentinel handling."""
        self._check_closed()
        await self._acquire_pool_permit()
        try:
            actual_auth = _normalize_auth(
                auth if auth is not USE_CLIENT_DEFAULT else self._auth
            )
            if actual_auth is None:
                actual_auth = _extract_auth_from_url(str(url))

            # If we have a custom transport, route through _send_single_request
            if self._custom_transport is not None:
                request = self.build_request(
                    "OPTIONS", url, params=params, headers=headers
                )
                if actual_auth is not None:
                    return await self._send_with_auth(request, actual_auth)
                return await self._send_single_request(request)

            if actual_auth is not None:
                result = await self._handle_auth(
                    "OPTIONS", url, actual_auth, params=params, headers=headers
                )
                if result is not None:
                    return result
            try:
                response = await self._client.options(
                    url,
                    params=params,
                    headers=headers,
                    cookies=cookies,
                    auth=_convert_auth(auth),
                    follow_redirects=follow_redirects,
                    timeout=timeout,
                )
                return Response(response)
            except (
                _RequestError,
                _TransportError,
                _TimeoutException,
                _NetworkError,
                _ConnectError,
                _ReadError,
                _WriteError,
                _CloseError,
                _ProxyError,
                _ProtocolError,
                _UnsupportedProtocol,
                _DecodingError,
                _TooManyRedirects,
                _StreamError,
                _ConnectTimeout,
                _ReadTimeout,
                _WriteTimeout,
                _PoolTimeout,
                _LocalProtocolError,
                _RemoteProtocolError,
            ) as e:
                raise _convert_exception(e) from None
        finally:
            self._release_pool_permit()

    async def request(
        self,
        method,
        url,
        *,
        content=None,
        data=None,
        files=None,
        json=None,
        params=None,
        headers=None,
        cookies=None,
        auth=USE_CLIENT_DEFAULT,
        follow_redirects=None,
        timeout=None,
    ):
        """HTTP request with proper auth sentinel handling."""
        self._check_closed()
        await self._acquire_pool_permit()
        try:
            actual_auth = _normalize_auth(
                auth if auth is not USE_CLIENT_DEFAULT else self._auth
            )
            if actual_auth is None:
                actual_auth = _extract_auth_from_url(str(url))

            # If we have a custom transport, route through _send_single_request
            if self._custom_transport is not None:
                request = self.build_request(
                    method,
                    url,
                    content=content,
                    data=data,
                    files=files,
                    json=json,
                    params=params,
                    headers=headers,
                )
                if actual_auth is not None:
                    return await self._send_with_auth(request, actual_auth)
                return await self._send_single_request(request)

            if actual_auth is not None:
                result = await self._handle_auth(
                    method,
                    url,
                    actual_auth,
                    content=content,
                    params=params,
                    headers=headers,
                )
                if result is not None:
                    return result
            try:
                response = await self._client.request(
                    method,
                    url,
                    content=content,
                    data=data,
                    files=files,
                    json=json,
                    params=params,
                    headers=headers,
                    cookies=cookies,
                    auth=_convert_auth(auth),
                    follow_redirects=follow_redirects,
                    timeout=timeout,
                )
                return Response(response)
            except (
                _RequestError,
                _TransportError,
                _TimeoutException,
                _NetworkError,
                _ConnectError,
                _ReadError,
                _WriteError,
                _CloseError,
                _ProxyError,
                _ProtocolError,
                _UnsupportedProtocol,
                _DecodingError,
                _TooManyRedirects,
                _StreamError,
                _ConnectTimeout,
                _ReadTimeout,
                _WriteTimeout,
                _PoolTimeout,
                _LocalProtocolError,
                _RemoteProtocolError,
            ) as e:
                raise _convert_exception(e) from None
        finally:
            self._release_pool_permit()

    @_contextlib.asynccontextmanager
    async def stream(
        self,
        method,
        url,
        *,
        content=None,
        data=None,
        files=None,
        json=None,
        params=None,
        headers=None,
        cookies=None,
        auth=USE_CLIENT_DEFAULT,
        follow_redirects=None,
        timeout=None,
    ):
        """Stream an HTTP request with proper auth handling."""
        actual_auth = _normalize_auth(
            auth if auth is not USE_CLIENT_DEFAULT else self._auth
        )
        if actual_auth is None:
            actual_auth = _extract_auth_from_url(str(url))
        await self._acquire_pool_permit()
        try:
            response = None
            if actual_auth is not None:
                # Build request with auth - build_request only supports certain params
                build_kwargs = {}
                if content is not None:
                    build_kwargs["content"] = content
                if params is not None:
                    build_kwargs["params"] = params
                if headers is not None:
                    build_kwargs["headers"] = headers
                if cookies is not None:
                    build_kwargs["cookies"] = cookies
                if json is not None:
                    build_kwargs["json"] = json
                request = self.build_request(method, url, **build_kwargs)
                # Apply auth
                if hasattr(actual_auth, "async_auth_flow") or hasattr(
                    actual_auth, "sync_auth_flow"
                ):
                    response = await self._send_with_auth(request, actual_auth)
                elif callable(actual_auth):
                    modified = actual_auth(request)
                    response = await self._send_single_request(
                        modified if modified is not None else request
                    )
            if response is None:
                if self._custom_transport is not None:
                    request = self.build_request(
                        method,
                        url,
                        content=content,
                        data=data,
                        files=files,
                        json=json,
                        params=params,
                        headers=headers,
                    )
                    response = await self._send_single_request(request)
                else:
                    # Call Rust client directly to avoid double pool acquisition from self.request()
                    try:
                        resp = await self._client.request(
                            method,
                            url,
                            content=content,
                            data=data,
                            files=files,
                            json=json,
                            params=params,
                            headers=headers,
                            cookies=cookies,
                            auth=_convert_auth(auth),
                            follow_redirects=follow_redirects,
                            timeout=timeout,
                        )
                        response = Response(resp)
                    except (
                        _RequestError,
                        _TransportError,
                        _TimeoutException,
                        _NetworkError,
                        _ConnectError,
                        _ReadError,
                        _WriteError,
                        _CloseError,
                        _ProxyError,
                        _ProtocolError,
                        _UnsupportedProtocol,
                        _DecodingError,
                        _TooManyRedirects,
                        _StreamError,
                        _ConnectTimeout,
                        _ReadTimeout,
                        _WriteTimeout,
                        _PoolTimeout,
                        _LocalProtocolError,
                        _RemoteProtocolError,
                    ) as e:
                        raise _convert_exception(e) from None
            # Mark as a streaming response that requires aread() before content access
            response._stream_not_read = True
            response._is_stream = True
            yield response
        finally:
            self._release_pool_permit()
