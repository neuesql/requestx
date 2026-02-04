import contextlib as _contextlib

from ._core import (
    URL,
    QueryParams,
    Client as _Client,
    Response as _Response,
    HTTPTransport,
    InvalidURL,
)
from ._compat import (
    USE_CLIENT_DEFAULT,
    _ExplicitPortURL,
    _logger,
)
from ._exceptions import (
    _convert_exception,
    _RUST_EXCEPTIONS,
    StreamConsumed,
    TooManyRedirects,
    UnsupportedProtocol,
    RemoteProtocolError,
)
from ._streams import (
    _GeneratorByteStream,
    SyncByteStream,
)
from ._request import _WrappedRequest
from ._response import Response
from ._auth import (
    BasicAuth,
    _normalize_auth,
    _extract_auth_from_url,
)
from ._transports import (
    MockTransport,
    BaseTransport,
    AsyncBaseTransport,
)
from ._client_common import (
    _HeadersProxy,
    extract_cookies_from_response as _extract_cookies_from_response_impl,
    merge_url as _merge_url_impl,
    get_proxy_from_env as _get_proxy_from_env_impl,
    transport_for_url as _transport_for_url_impl,
)


class Client:
    """Sync HTTP client that wraps the Rust implementation with proper auth sentinel handling."""

    def __init__(self, *args, **kwargs):
        # Extract auth and transport from kwargs before passing to Rust client
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

        # Create default transport (with proxy if specified)
        custom_transport = kwargs.get("transport", None)
        if custom_transport is not None:
            self._default_transport = custom_transport
        elif proxy is not None:
            self._default_transport = HTTPTransport(proxy=proxy)
        else:
            # Check for proxy env vars if trust_env is True
            env_proxy = None
            if trust_env:
                env_proxy = _get_proxy_from_env_impl()
            if env_proxy:
                self._default_transport = HTTPTransport(proxy=env_proxy)
            else:
                self._default_transport = HTTPTransport()

        self._custom_transport = (
            custom_transport  # Keep reference to user-provided transport
        )

        # Extract and store follow_redirects from kwargs before passing to Rust
        self._follow_redirects = kwargs.pop("follow_redirects", False)

        # Extract and store default_encoding for response text decoding
        self._default_encoding = kwargs.pop("default_encoding", None)

        # Extract and store params from kwargs
        params = kwargs.pop("params", None)
        if params is not None:
            self._params = QueryParams(params)
        else:
            self._params = QueryParams()

        # Always create Rust client with follow_redirects=False so Python handles redirects
        # This allows proper logging and history tracking
        kwargs["follow_redirects"] = False
        self._client = _Client(*args, **kwargs)
        self._headers_proxy = None
        self._is_closed = False

    @property
    def _transport(self):
        """Get the default transport for this client."""
        return self._default_transport

    def _transport_for_url(self, url):
        return _transport_for_url_impl(self, url, HTTPTransport)

    def _invoke_request_hooks(self, request):
        """Invoke all request event hooks."""
        hooks = self.event_hooks.get("request", [])
        for hook in hooks:
            hook(request)

    def _invoke_response_hooks(self, response):
        """Invoke all response event hooks."""
        hooks = self.event_hooks.get("response", [])
        for hook in hooks:
            try:
                hook(response)
            except BaseException:
                # Close the response when a hook raises an exception
                response.close()
                raise

    def __getattr__(self, name):
        """Delegate attribute access to the underlying client."""
        return getattr(self._client, name)

    def __enter__(self):
        if self._is_closed:
            raise RuntimeError("Cannot open a client that has been closed")
        # Call transport's __enter__ if it exists
        if self._transport is not None and hasattr(self._transport, "__enter__"):
            self._transport.__enter__()
        # Call __enter__ on all mounted transports
        for transport in self._mounts.values():
            if hasattr(transport, "__enter__"):
                transport.__enter__()
        self._client.__enter__()
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        result = self._client.__exit__(exc_type, exc_val, exc_tb)
        # Call transport's __exit__ if it exists
        if self._transport is not None and hasattr(self._transport, "__exit__"):
            self._transport.__exit__(exc_type, exc_val, exc_tb)
        # Call __exit__ on all mounted transports
        for transport in self._mounts.values():
            if hasattr(transport, "__exit__"):
                transport.__exit__(exc_type, exc_val, exc_tb)
        self._is_closed = True
        return result

    def close(self):
        """Close the client."""
        if hasattr(self._client, "close"):
            self._client.close()
        if self._transport is not None and hasattr(self._transport, "close"):
            self._transport.close()
        # Close all mounted transports
        for transport in self._mounts.values():
            if hasattr(transport, "close"):
                transport.close()
        self._is_closed = True

    @property
    def is_closed(self):
        """Return True if the client has been closed."""
        return getattr(self, "_is_closed", False)

    @property
    def base_url(self):
        return self._client.base_url

    @base_url.setter
    def base_url(self, value):
        self._client.base_url = value

    @property
    def params(self):
        """Return the client's default query parameters."""
        return self._params

    @params.setter
    def params(self, value):
        """Set the client's default query parameters."""
        if value is not None:
            self._params = QueryParams(value)
        else:
            self._params = QueryParams()

    @property
    def headers(self):
        # Return a proxy that syncs changes back to the client
        # Use cached proxy if available, but refresh if underlying headers changed
        if not hasattr(self, "_headers_proxy") or self._headers_proxy is None:
            self._headers_proxy = _HeadersProxy(self)
        return self._headers_proxy

    @headers.setter
    def headers(self, value):
        self._client.headers = value
        # Clear cached proxy so it gets refreshed on next access
        self._headers_proxy = None

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
        # Check for async iterator/generator in content (sync Client can't handle these)
        import inspect
        import types

        content = kwargs.get("content")
        sync_stream = None  # Track if we're using a generator stream
        if content is not None:
            if inspect.isasyncgen(content) or inspect.iscoroutine(content):
                raise RuntimeError(
                    "Attempted to send an async request with a sync Client instance."
                )
            # Also check for async iterator protocol
            if hasattr(content, "__anext__") or hasattr(content, "__aiter__"):
                raise RuntimeError(
                    "Attempted to send an async request with a sync Client instance."
                )
            # Handle sync generators/iterators - wrap them in a trackable stream
            if isinstance(content, types.GeneratorType):
                # Create a wrapper that tracks consumption
                # Pass None to Rust - the body will be read from the stream by the transport
                sync_stream = _GeneratorByteStream(content)
                kwargs["content"] = None  # Don't pass generator to Rust
            elif (
                hasattr(content, "__iter__")
                and hasattr(content, "__next__")
                and not isinstance(content, (bytes, str, list, tuple))
            ):
                # It's an iterator - wrap it
                sync_stream = _GeneratorByteStream(content)
                kwargs["content"] = None
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

        # Merge client params with request params
        request_params = kwargs.get("params")
        if self._params:
            if request_params is not None:
                # Merge: client params first, then request params
                merged_params = QueryParams(self._params)
                merged_params = merged_params.merge(QueryParams(request_params))
                kwargs["params"] = merged_params
            else:
                kwargs["params"] = self._params

        rust_request = self._client.build_request(method, merged_url, **kwargs)
        # Create a wrapper that delegates to the Rust request but has our headers proxy
        wrapped = _WrappedRequest(rust_request, sync_stream=sync_stream)
        # Link the stream back to the owner for consumption tracking
        if sync_stream is not None:
            sync_stream._owner = wrapped
        return wrapped

    def _merge_url(self, url):
        return _merge_url_impl(self, url)

    def _wrap_response(self, rust_response):
        """Wrap a Rust response in a Python Response."""
        return Response(rust_response, default_encoding=self._default_encoding)

    def _send_single_request(self, request, url=None):
        """Send a single request, handling transport properly."""
        if self._is_closed:
            raise RuntimeError("Cannot send request on a closed client")

        if isinstance(request, _WrappedRequest):
            rust_request = request._rust_request
            request_url = url or request.url
        elif hasattr(request, "_rust_request"):
            rust_request = request._rust_request
            request_url = url or request.url
        else:
            rust_request = request
            request_url = url or (request.url if hasattr(request, "url") else None)

        # Invoke request event hooks before sending
        self._invoke_request_hooks(request)

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

        if use_custom and transport is not None:
            # Determine which request to send based on transport type
            # Python-based transports (MockTransport, BaseTransport subclasses) can handle _WrappedRequest
            # Rust-based transports (WSGITransport, HTTPTransport) need the Rust Request
            if isinstance(
                transport, (MockTransport, BaseTransport, AsyncBaseTransport)
            ):
                # Python transport - pass wrapped request for stream tracking
                request_to_send = (
                    request if isinstance(request, _WrappedRequest) else rust_request
                )
            else:
                # Rust transport - pass raw Rust request
                request_to_send = rust_request
            if hasattr(transport, "handle_request"):
                result = transport.handle_request(request_to_send)
            elif callable(transport):
                result = transport(request_to_send)
            else:
                raise TypeError("Transport must have handle_request method")
            # Wrap result in Response if needed
            if isinstance(result, Response):
                response = result
                if (
                    response._default_encoding is None
                    and self._default_encoding is not None
                ):
                    response._default_encoding = self._default_encoding
            elif isinstance(result, _Response):
                response = Response(result, default_encoding=self._default_encoding)
            else:
                response = Response(result, default_encoding=self._default_encoding)
        else:
            try:
                result = self._client.send(rust_request)
                response = Response(result, default_encoding=self._default_encoding)
            except _RUST_EXCEPTIONS as e:
                raise _convert_exception(e) from None

        # Set URL and request on response
        # Use explicit URL if available (preserves non-normalized port like :443)
        if isinstance(request, _WrappedRequest) and request._explicit_url is not None:
            response._url = _ExplicitPortURL(request._explicit_url)
        elif request_url is not None:
            response._url = request_url
        response._request = request

        # Build next_request if this is a redirect
        if response.is_redirect:
            location = response.headers.get("location")
            if location:
                response._next_request = self._build_redirect_request(request, response)

        # Invoke response event hooks after receiving
        self._invoke_response_hooks(response)

        # Log the request/response
        method = request.method if hasattr(request, "method") else "GET"
        url_str = str(request_url) if request_url else ""
        status_code = response.status_code
        reason_phrase = response.reason_phrase or ""
        _logger.info(
            f'HTTP Request: {method} {url_str} "HTTP/1.1 {status_code} {reason_phrase}"'
        )

        return response

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
        # Emojis and other non-ASCII characters in the host portion are invalid
        try:
            # First try to parse the location URL
            if location.startswith("//") or location.startswith("/"):
                # Relative URL - will be joined with original
                pass
            elif "://" in location:
                # Absolute URL - check if host contains invalid characters
                from urllib.parse import urlparse

                parsed = urlparse(location)
                if parsed.netloc:
                    # Check for non-ASCII characters in host (excluding punycode)
                    host_part = parsed.hostname or ""
                    try:
                        # Try to encode as ASCII - if it fails and it's not punycode, it's invalid
                        host_part.encode("ascii")
                    except UnicodeEncodeError:
                        # Non-ASCII in host - invalid URL
                        raise RemoteProtocolError(f"Invalid redirect URL: {location}")
        except RemoteProtocolError:
            raise
        except Exception:
            pass  # Let URL parsing handle other errors

        # Parse location - handle relative and absolute URLs
        redirect_url = None
        try:
            if original_url:
                # Join with original URL to handle relative redirects
                if isinstance(original_url, URL):
                    redirect_url = original_url.join(location)
                else:
                    redirect_url = URL(original_url).join(location)
            else:
                redirect_url = URL(location)
        except InvalidURL as e:
            # Handle malformed URLs like https://:443/ by trying to fix empty host
            explicit_url_str = None  # Track manually constructed URL with explicit port
            if "empty host" in str(e).lower() and original_url:
                # Try to extract what we can from the location
                from urllib.parse import urlparse

                parsed = urlparse(location)
                orig_url = (
                    original_url
                    if isinstance(original_url, URL)
                    else URL(str(original_url))
                )

                # Build URL manually using original host
                scheme = parsed.scheme or orig_url.scheme
                host = orig_url.host  # Use original host since location has empty host
                port = parsed.port if parsed.port else None
                path = parsed.path or "/"

                # Construct the redirect URL - preserve explicit port even if it's the default
                if port:
                    redirect_url_str = f"{scheme}://{host}:{port}{path}"
                    explicit_url_str = redirect_url_str  # Mark as explicit (has non-standard port repr)
                else:
                    redirect_url_str = f"{scheme}://{host}{path}"
                if parsed.query:
                    redirect_url_str += f"?{parsed.query}"
                    if explicit_url_str:
                        explicit_url_str += f"?{parsed.query}"

                try:
                    redirect_url = URL(redirect_url_str)
                    # Keep the manually constructed URL string - don't let URL normalize the port
                    # redirect_url_str is already set correctly above
                except Exception:
                    raise RemoteProtocolError(f"Invalid redirect URL: {location}")
            else:
                raise RemoteProtocolError(f"Invalid redirect URL: {location}")
        except Exception:
            raise RemoteProtocolError(f"Invalid redirect URL: {location}")
        else:
            # Normal case - get URL string from the parsed redirect_url
            # Check for invalid URL (e.g., non-ASCII characters)
            explicit_url_str = None
            try:
                redirect_url_str = str(redirect_url)
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
            # Remove Content-Length for body-less redirects
            headers.pop("content-length", None)
            headers.pop("Content-Length", None)
        elif hasattr(request, "content"):
            # 307/308 preserve body
            content = request.content
            # Check if stream was consumed
            if hasattr(request, "stream"):
                stream = request.stream
                # Check various consumed indicators
                if hasattr(stream, "_consumed") and stream._consumed:
                    raise StreamConsumed()
                # For SyncByteStream, check if it's already been iterated
                if isinstance(stream, SyncByteStream) and getattr(
                    stream, "_consumed", False
                ):
                    raise StreamConsumed()
            # Also check if the request was built with a generator/iterator stream
            if hasattr(request, "_stream_consumed") and request._stream_consumed:
                raise StreamConsumed()
            if isinstance(request, _WrappedRequest) and request._stream_consumed:
                raise StreamConsumed()

        # Add client cookies to redirect request
        # This ensures cookies set via Set-Cookie headers are sent on subsequent requests
        if self.cookies:
            cookie_header = "; ".join(
                f"{name}={value}" for name, value in self.cookies.items()
            )
            if cookie_header:
                headers["Cookie"] = cookie_header

        wrapped_request = self.build_request(
            method, redirect_url_str, headers=headers, content=content
        )
        # Store explicit URL if we have one (preserves non-normalized port)
        if explicit_url_str:
            wrapped_request._explicit_url = explicit_url_str
        return wrapped_request

    def _send_handling_redirects(self, request, follow_redirects=False, history=None):
        """Send a request, optionally following redirects."""
        if history is None:
            history = []

        # Get original request URL for fragment preservation
        original_url = request.url if hasattr(request, "url") else None
        original_fragment = None
        if original_url and isinstance(original_url, URL):
            original_fragment = original_url.fragment

        response = self._send_single_request(request, url=original_url)

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

        # Update cookies on the redirect request (they were extracted after next_request was built)
        # This handles both adding new cookies AND removing expired ones
        if isinstance(next_request, _WrappedRequest):
            if self.cookies:
                cookie_header = "; ".join(
                    f"{name}={value}" for name, value in self.cookies.items()
                )
                next_request.headers["Cookie"] = cookie_header
            else:
                # Cookies might have been deleted (e.g., expired), remove the Cookie header
                try:
                    del next_request.headers["Cookie"]
                except KeyError:
                    pass

        # Preserve fragment from original URL
        if original_fragment:
            next_url = next_request.url if hasattr(next_request, "url") else None
            if next_url and isinstance(next_url, URL):
                if not next_url.fragment:
                    # Add fragment to URL
                    next_url_str = str(next_url)
                    if "#" not in next_url_str:
                        next_request = self.build_request(
                            next_request.method,
                            next_url_str + "#" + original_fragment,
                            headers=dict(next_request.headers.items())
                            if hasattr(next_request, "headers")
                            else None,
                            content=next_request.content
                            if hasattr(next_request, "content")
                            else None,
                        )

        # Recursively follow
        return self._send_handling_redirects(
            next_request, follow_redirects=True, history=history
        )

    def _handle_auth(self, method, url, actual_auth, **build_kwargs):
        """Handle auth for sync requests - supports generators and callables."""
        # Convert tuple to BasicAuth
        if isinstance(actual_auth, tuple) and len(actual_auth) == 2:
            actual_auth = BasicAuth(actual_auth[0], actual_auth[1])

        request = self.build_request(method, url, **build_kwargs)
        # Check for generator-based auth
        if hasattr(actual_auth, "sync_auth_flow") or hasattr(actual_auth, "auth_flow"):
            return self._send_with_auth(request, actual_auth)
        # Check for callable auth (function that modifies request)
        elif callable(actual_auth):
            modified = actual_auth(request)
            return self._send_single_request(
                modified if modified is not None else request
            )
        else:
            # Invalid auth type
            raise TypeError(
                f"Invalid 'auth' argument. Expected (username, password) tuple, Auth instance, or callable. Got {type(actual_auth).__name__}."
            )

    def _send_with_auth(self, request, auth, follow_redirects=False):
        """Send a request with auth flow handling.

        If auth has sync_auth_flow or auth_flow, use the generator protocol.
        Otherwise, send directly.
        """
        import inspect

        # Ensure we have a wrapped request for proper header mutation
        if isinstance(request, _WrappedRequest):
            wrapped_request = request
        else:
            wrapped_request = _WrappedRequest(request)

        # Get the auth flow generator
        # For Rust auth classes (BasicAuth, DigestAuth), pass the underlying Rust request
        # For Python auth classes (generators), pass the wrapped request
        auth_flow = None
        if auth is not None:
            # Check for custom auth_flow defined on the class (not the Rust base class)
            auth_type = type(auth)
            if "auth_flow" in auth_type.__dict__ or (
                hasattr(auth, "auth_flow") and callable(getattr(auth, "auth_flow"))
            ):
                auth_flow_method = getattr(auth, "auth_flow", None)
                if auth_flow_method and (
                    inspect.isgeneratorfunction(auth_flow_method)
                    or (
                        hasattr(auth_flow_method, "__func__")
                        and inspect.isgeneratorfunction(auth_flow_method.__func__)
                    )
                ):
                    # Python generator - pass wrapped request for header mutations
                    auth_flow = auth.auth_flow(wrapped_request)
            if auth_flow is None and hasattr(auth, "sync_auth_flow"):
                method = getattr(auth, "sync_auth_flow")
                if inspect.isgeneratorfunction(method) or (
                    hasattr(method, "__func__")
                    and inspect.isgeneratorfunction(method.__func__)
                ):
                    # Python generator - pass wrapped request
                    auth_flow = auth.sync_auth_flow(wrapped_request)
                else:
                    # Rust auth - pass the underlying request
                    auth_flow = auth.sync_auth_flow(wrapped_request._rust_request)

        if auth_flow is None:
            # No auth flow, send with redirect handling
            return self._send_handling_redirects(
                wrapped_request, follow_redirects=follow_redirects
            )

        # Check if auth_flow returned a list (Rust base class) or generator
        if isinstance(auth_flow, (list, tuple)):
            # Simple list of requests - just send the last one
            last_request = wrapped_request
            for req in auth_flow:
                last_request = req
            return self._send_handling_redirects(
                last_request, follow_redirects=follow_redirects
            )

        # Generator-based auth flow
        history = []  # Track intermediate responses
        try:
            # Get the first yielded request (possibly with auth headers added)
            request = next(auth_flow)
            # Send it and get the response (without redirect handling - auth flow controls this)
            response = self._send_single_request(request)
            # Extract cookies from response
            self._extract_cookies_from_response(response, request)

            # Continue the auth flow with the response (for digest auth, etc.)
            while True:
                try:
                    # Try to get next request - if this succeeds, current response is intermediate
                    request = auth_flow.send(response)
                    # Set cumulative history on current response before adding to history
                    response._history = list(
                        history
                    )  # Copy current history to this response
                    # Add current response to history since there's a next request
                    history.append(response)
                    # Send next request
                    response = self._send_single_request(request)
                    # Extract cookies from response
                    self._extract_cookies_from_response(response, request)
                except StopIteration:
                    # No more requests - current response is the final one
                    break

            # Set history on final response and handle redirects if needed
            if history:
                response._history = history

            # After auth completes, handle redirects if needed
            if follow_redirects and response.is_redirect:
                return self._send_handling_redirects(
                    response.next_request, follow_redirects=True, history=history
                )

            return response
        except StopIteration:
            # Auth flow returned without yielding, send request as-is
            return self._send_handling_redirects(
                wrapped_request, follow_redirects=follow_redirects
            )

    def send(self, request, **kwargs):
        """Send a Request object."""
        auth = kwargs.pop("auth", None)
        follow_redirects = kwargs.pop("follow_redirects", None)
        actual_follow = (
            follow_redirects if follow_redirects is not None else self._follow_redirects
        )
        if auth is not None:
            return self._send_with_auth(request, auth, follow_redirects=actual_follow)
        # Route through redirect handling
        return self._send_handling_redirects(
            request, follow_redirects=bool(actual_follow)
        )

    def _check_closed(self):
        """Raise RuntimeError if the client is closed."""
        if self._is_closed:
            raise RuntimeError("Cannot send request on a closed client")

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

    def get(
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
        """HTTP GET with proper auth and redirect handling."""
        self._check_closed()
        self._warn_per_request_cookies(cookies)
        request = self.build_request(
            "GET", url, params=params, headers=headers, cookies=cookies
        )
        actual_auth = _normalize_auth(
            auth if auth is not USE_CLIENT_DEFAULT else self._auth
        )
        if actual_auth is None:
            actual_auth = _extract_auth_from_url(str(url))
        actual_follow = (
            follow_redirects if follow_redirects is not None else self._follow_redirects
        )
        if actual_auth is not None:
            return self._send_with_auth(
                request, actual_auth, follow_redirects=actual_follow
            )
        return self._send_handling_redirects(
            request, follow_redirects=bool(actual_follow)
        )

    def post(
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
        """HTTP POST with proper auth and redirect handling."""
        self._check_closed()
        self._warn_per_request_cookies(cookies)
        request = self.build_request(
            "POST",
            url,
            content=content,
            data=data,
            files=files,
            json=json,
            params=params,
            headers=headers,
            cookies=cookies,
        )
        actual_auth = _normalize_auth(
            auth if auth is not USE_CLIENT_DEFAULT else self._auth
        )
        if actual_auth is None:
            actual_auth = _extract_auth_from_url(str(url))
        actual_follow = (
            follow_redirects if follow_redirects is not None else self._follow_redirects
        )
        if actual_auth is not None:
            return self._send_with_auth(
                request, actual_auth, follow_redirects=actual_follow
            )
        return self._send_handling_redirects(
            request, follow_redirects=bool(actual_follow)
        )

    def put(
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
        """HTTP PUT with proper auth and redirect handling."""
        self._check_closed()
        self._warn_per_request_cookies(cookies)
        request = self.build_request(
            "PUT",
            url,
            content=content,
            data=data,
            files=files,
            json=json,
            params=params,
            headers=headers,
            cookies=cookies,
        )
        actual_auth = _normalize_auth(
            auth if auth is not USE_CLIENT_DEFAULT else self._auth
        )
        if actual_auth is None:
            actual_auth = _extract_auth_from_url(str(url))
        actual_follow = (
            follow_redirects if follow_redirects is not None else self._follow_redirects
        )
        if actual_auth is not None:
            return self._send_with_auth(
                request, actual_auth, follow_redirects=actual_follow
            )
        return self._send_handling_redirects(
            request, follow_redirects=bool(actual_follow)
        )

    def patch(
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
        """HTTP PATCH with proper auth and redirect handling."""
        self._check_closed()
        self._warn_per_request_cookies(cookies)
        request = self.build_request(
            "PATCH",
            url,
            content=content,
            data=data,
            files=files,
            json=json,
            params=params,
            headers=headers,
            cookies=cookies,
        )
        actual_auth = _normalize_auth(
            auth if auth is not USE_CLIENT_DEFAULT else self._auth
        )
        if actual_auth is None:
            actual_auth = _extract_auth_from_url(str(url))
        actual_follow = (
            follow_redirects if follow_redirects is not None else self._follow_redirects
        )
        if actual_auth is not None:
            return self._send_with_auth(
                request, actual_auth, follow_redirects=actual_follow
            )
        return self._send_handling_redirects(
            request, follow_redirects=bool(actual_follow)
        )

    def delete(
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
        """HTTP DELETE with proper auth and redirect handling."""
        self._check_closed()
        self._warn_per_request_cookies(cookies)
        request = self.build_request(
            "DELETE", url, params=params, headers=headers, cookies=cookies
        )
        actual_auth = _normalize_auth(
            auth if auth is not USE_CLIENT_DEFAULT else self._auth
        )
        if actual_auth is None:
            actual_auth = _extract_auth_from_url(str(url))
        actual_follow = (
            follow_redirects if follow_redirects is not None else self._follow_redirects
        )
        if actual_auth is not None:
            return self._send_with_auth(
                request, actual_auth, follow_redirects=actual_follow
            )
        return self._send_handling_redirects(
            request, follow_redirects=bool(actual_follow)
        )

    def head(
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
        """HTTP HEAD with proper auth and redirect handling."""
        self._check_closed()
        self._warn_per_request_cookies(cookies)
        request = self.build_request(
            "HEAD", url, params=params, headers=headers, cookies=cookies
        )
        actual_auth = _normalize_auth(
            auth if auth is not USE_CLIENT_DEFAULT else self._auth
        )
        if actual_auth is None:
            actual_auth = _extract_auth_from_url(str(url))
        actual_follow = (
            follow_redirects if follow_redirects is not None else self._follow_redirects
        )
        if actual_auth is not None:
            return self._send_with_auth(
                request, actual_auth, follow_redirects=actual_follow
            )
        return self._send_handling_redirects(
            request, follow_redirects=bool(actual_follow)
        )

    def options(
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
        """HTTP OPTIONS with proper auth and redirect handling."""
        self._check_closed()
        self._warn_per_request_cookies(cookies)
        request = self.build_request(
            "OPTIONS", url, params=params, headers=headers, cookies=cookies
        )
        actual_auth = _normalize_auth(
            auth if auth is not USE_CLIENT_DEFAULT else self._auth
        )
        if actual_auth is None:
            actual_auth = _extract_auth_from_url(str(url))
        actual_follow = (
            follow_redirects if follow_redirects is not None else self._follow_redirects
        )
        if actual_auth is not None:
            return self._send_with_auth(
                request, actual_auth, follow_redirects=actual_follow
            )
        return self._send_handling_redirects(
            request, follow_redirects=bool(actual_follow)
        )

    def request(
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
        """HTTP request with proper auth and redirect handling."""
        self._check_closed()
        self._warn_per_request_cookies(cookies)
        request = self.build_request(
            method,
            url,
            content=content,
            data=data,
            files=files,
            json=json,
            params=params,
            headers=headers,
            cookies=cookies,
        )
        actual_auth = _normalize_auth(
            auth if auth is not USE_CLIENT_DEFAULT else self._auth
        )
        if actual_auth is None:
            actual_auth = _extract_auth_from_url(str(url))
        actual_follow = (
            follow_redirects if follow_redirects is not None else self._follow_redirects
        )
        if actual_auth is not None:
            return self._send_with_auth(
                request, actual_auth, follow_redirects=actual_follow
            )
        return self._send_handling_redirects(
            request, follow_redirects=bool(actual_follow)
        )

    @_contextlib.contextmanager
    def stream(
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
        response = None
        try:
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
                if hasattr(actual_auth, "sync_auth_flow") or hasattr(
                    actual_auth, "auth_flow"
                ):
                    response = self._send_with_auth(request, actual_auth)
                elif callable(actual_auth):
                    modified = actual_auth(request)
                    response = self._send_single_request(
                        modified if modified is not None else request
                    )
            if response is None:
                response = self.request(
                    method,
                    url,
                    content=content,
                    data=data,
                    files=files,
                    json=json,
                    params=params,
                    headers=headers,
                    cookies=cookies,
                    auth=auth,
                    follow_redirects=follow_redirects,
                    timeout=timeout,
                )
            yield response
        finally:
            # Cleanup if needed
            pass
