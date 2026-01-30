# RequestX - High-performance Python HTTP client
# API-compatible with httpx, powered by Rust's reqwest via PyO3

import contextlib

# Sentinel for "auth not specified" - distinct from auth=None which disables auth
class _AuthUnset:
    """Sentinel to indicate auth was not specified."""
    _instance = None
    def __new__(cls):
        if cls._instance is None:
            cls._instance = super().__new__(cls)
        return cls._instance
    def __repr__(self):
        return '<USE_CLIENT_AUTH>'
    def __bool__(self):
        return False

USE_CLIENT_DEFAULT = _AuthUnset()

# Sentinel for "auth explicitly disabled" - used to pass auth=None to Rust
class _AuthDisabled:
    """Sentinel to indicate auth is explicitly disabled."""
    _instance = None
    def __new__(cls):
        if cls._instance is None:
            cls._instance = super().__new__(cls)
        return cls._instance
    def __repr__(self):
        return '<AUTH_DISABLED>'
    def __bool__(self):
        return False

_AUTH_DISABLED = _AuthDisabled()

from ._core import (
    # Version info
    __version__,
    __title__,
    __description__,
    # Core types
    URL,
    Headers,
    QueryParams,
    Cookies,
    Request as _Request,  # Import as _Request, we'll wrap it
    Response as _Response,  # Import as _Response, we'll wrap it
    # Clients
    Client as _Client,  # Import as _Client, we'll wrap it
    AsyncClient as _AsyncClient,  # Import as _AsyncClient, we'll wrap it
    # Configuration
    Timeout,
    Limits,
    Proxy,
    # Stream types - raw Rust types, we'll wrap them
    SyncByteStream as _SyncByteStream,
    AsyncByteStream as _AsyncByteStream,
    # Auth types (import as _AuthType to wrap with generator protocol)
    BasicAuth as _BasicAuth,
    DigestAuth as _DigestAuth,
    NetRCAuth as _NetRCAuth,
    Auth as _Auth,
    FunctionAuth as _FunctionAuth,
    # Transport types
    MockTransport,
    AsyncMockTransport,
    HTTPTransport,
    AsyncHTTPTransport,
    WSGITransport,
    # Top-level functions
    get,
    post,
    put,
    patch,
    delete,
    head,
    options,
    request,
    stream,
    # Exceptions (import HTTPStatusError as _HTTPStatusError to wrap it)
    HTTPStatusError as _HTTPStatusError,
    RequestError,
    TransportError,
    TimeoutException,
    ConnectTimeout,
    ReadTimeout,
    WriteTimeout,
    PoolTimeout,
    NetworkError,
    ConnectError,
    ReadError,
    WriteError,
    CloseError,
    ProxyError,
    ProtocolError,
    LocalProtocolError,
    RemoteProtocolError,
    UnsupportedProtocol,
    DecodingError,
    TooManyRedirects,
    StreamError,
    StreamConsumed,
    StreamClosed,
    ResponseNotRead,
    RequestNotRead,
    InvalidURL,
    HTTPError,
    CookieConflict,
    # Status codes (import as _codes to wrap)
    codes as _codes,
)


# ============================================================================
# Transport Base Classes
# ============================================================================

class BaseTransport:
    """Base class for sync HTTP transport implementations.

    Subclass and implement handle_request to create custom transports.
    """

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()
        return None

    def close(self):
        pass

    def handle_request(self, request):
        raise NotImplementedError("Subclasses must implement handle_request()")


class AsyncBaseTransport:
    """Base class for async HTTP transport implementations.

    Subclass and implement handle_async_request to create custom transports.
    """

    async def __aenter__(self):
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        await self.aclose()
        return None

    async def aclose(self):
        pass

    async def handle_async_request(self, request):
        raise NotImplementedError("Subclasses must implement handle_async_request()")


# ============================================================================
# Stream Classes - Python wrappers with proper isinstance support
# ============================================================================

class SyncByteStream:
    """Base class for synchronous byte streams.

    Implements the sync iteration protocol (__iter__, __next__).
    """

    def __init__(self, data=b""):
        if isinstance(data, (bytes, bytearray)):
            self._data = bytes(data)
        else:
            self._data = data
        self._consumed = False

    def __iter__(self):
        self._consumed = False
        return self

    def __next__(self):
        if self._consumed:
            raise StopIteration
        if isinstance(self._data, bytes):
            self._consumed = True
            if self._data:
                return self._data
            raise StopIteration
        # For other iterables, raise as consumed
        self._consumed = True
        raise StopIteration

    def read(self):
        """Read all bytes."""
        if isinstance(self._data, bytes):
            return self._data
        return b""

    def close(self):
        """Close the stream."""
        pass

    def __repr__(self):
        if isinstance(self._data, bytes):
            return f"<SyncByteStream [{len(self._data)} bytes]>"
        return "<SyncByteStream>"


class AsyncByteStream:
    """Base class for asynchronous byte streams.

    Implements the async iteration protocol (__aiter__, __anext__).
    """

    def __init__(self, data=b""):
        if isinstance(data, (bytes, bytearray)):
            self._data = bytes(data)
        else:
            self._data = data
        self._consumed = False

    def __aiter__(self):
        self._consumed = False
        return self

    async def __anext__(self):
        if self._consumed:
            raise StopAsyncIteration
        if isinstance(self._data, bytes):
            self._consumed = True
            if self._data:
                return self._data
            raise StopAsyncIteration
        self._consumed = True
        raise StopAsyncIteration

    async def aread(self):
        """Read all bytes asynchronously."""
        if isinstance(self._data, bytes):
            return self._data
        return b""

    async def aclose(self):
        """Close the stream asynchronously."""
        pass

    def __repr__(self):
        if isinstance(self._data, bytes):
            return f"<AsyncByteStream [{len(self._data)} bytes]>"
        return "<AsyncByteStream>"


class ByteStream(SyncByteStream, AsyncByteStream):
    """Dual-mode byte stream that supports both sync and async iteration.

    This class inherits from both SyncByteStream and AsyncByteStream,
    so isinstance checks for either will return True.
    """

    def __init__(self, data=b""):
        if isinstance(data, (bytes, bytearray)):
            self._data = bytes(data)
        else:
            self._data = data
        self._sync_consumed = False
        self._async_consumed = False

    # Sync iteration
    def __iter__(self):
        self._sync_consumed = False
        return self

    def __next__(self):
        if self._sync_consumed:
            raise StopIteration
        if isinstance(self._data, bytes):
            self._sync_consumed = True
            if self._data:
                return self._data
            raise StopIteration
        self._sync_consumed = True
        raise StopIteration

    # Async iteration
    def __aiter__(self):
        self._async_consumed = False
        return self

    async def __anext__(self):
        if self._async_consumed:
            raise StopAsyncIteration
        if isinstance(self._data, bytes):
            self._async_consumed = True
            if self._data:
                return self._data
            raise StopAsyncIteration
        self._async_consumed = True
        raise StopAsyncIteration

    # Common methods
    def read(self):
        """Read all bytes synchronously."""
        if isinstance(self._data, bytes):
            return self._data
        return b""

    async def aread(self):
        """Read all bytes asynchronously."""
        if isinstance(self._data, bytes):
            return self._data
        return b""

    def close(self):
        """Close the stream."""
        pass

    async def aclose(self):
        """Close the stream asynchronously."""
        pass

    def __repr__(self):
        if isinstance(self._data, bytes):
            return f"<ByteStream [{len(self._data)} bytes]>"
        return "<ByteStream>"


# ============================================================================
# Request wrapper with proper stream property
# ============================================================================

class _WrappedRequest:
    """Wrapper for Rust Request that provides mutable headers."""

    def __init__(self, rust_request):
        self._rust_request = rust_request
        self._headers_modified = False

    def __getattr__(self, name):
        return getattr(self._rust_request, name)

    @property
    def headers(self):
        return _WrappedRequestHeadersProxy(self)

    @headers.setter
    def headers(self, value):
        self._rust_request.headers = value

    def set_header(self, name, value):
        self._rust_request.set_header(name, value)

    def get_header(self, name, default=None):
        return self._rust_request.get_header(name, default)


class _WrappedRequestHeadersProxy:
    """Proxy for wrapped request headers that syncs changes back."""

    def __init__(self, wrapped_request):
        self._wrapped_request = wrapped_request
        # Get headers from rust request and convert to a new Headers object
        rust_headers = wrapped_request._rust_request.headers
        # Create a new Headers from the multi_items (preserves duplicates)
        self._headers = Headers(list(rust_headers.multi_items()))

    def _sync_back(self):
        self._wrapped_request._rust_request.headers = self._headers

    def __getitem__(self, key):
        return self._headers[key]

    def __setitem__(self, key, value):
        self._headers[key] = value
        self._sync_back()

    def __delitem__(self, key):
        del self._headers[key]
        self._sync_back()

    def __contains__(self, key):
        return key in self._headers

    def __iter__(self):
        return iter(self._headers)

    def __len__(self):
        return len(self._headers)

    def __eq__(self, other):
        return self._headers == other

    def __repr__(self):
        return repr(self._headers)

    def get(self, key, default=None):
        return self._headers.get(key, default)

    def get_list(self, key, split_commas=False):
        return self._headers.get_list(key, split_commas)

    def keys(self):
        return self._headers.keys()

    def values(self):
        return self._headers.values()

    def items(self):
        return self._headers.items()

    def multi_items(self):
        return self._headers.multi_items()

    def update(self, other):
        self._headers.update(other)
        self._sync_back()

    def setdefault(self, key, default=None):
        result = self._headers.setdefault(key, default)
        self._sync_back()
        return result

    def copy(self):
        return self._headers.copy()

    @property
    def raw(self):
        return self._headers.raw

    @property
    def encoding(self):
        return self._headers.encoding


class _RequestHeadersProxy:
    """Proxy object that wraps Headers and syncs changes back to the request."""

    def __init__(self, request):
        self._request = request
        self._headers = request._get_headers()  # Get current headers

    def __getitem__(self, key):
        return self._headers[key]

    def __setitem__(self, key, value):
        self._headers[key] = value
        self._request._set_headers(self._headers)

    def __delitem__(self, key):
        del self._headers[key]
        self._request._set_headers(self._headers)

    def __contains__(self, key):
        return key in self._headers

    def __iter__(self):
        return iter(self._headers)

    def __len__(self):
        return len(self._headers)

    def __eq__(self, other):
        return self._headers == other

    def __repr__(self):
        return repr(self._headers)

    def get(self, key, default=None):
        return self._headers.get(key, default)

    def get_list(self, key, split_commas=False):
        return self._headers.get_list(key, split_commas)

    def keys(self):
        return self._headers.keys()

    def values(self):
        return self._headers.values()

    def items(self):
        return self._headers.items()

    def multi_items(self):
        return self._headers.multi_items()

    def update(self, other):
        self._headers.update(other)
        self._request._set_headers(self._headers)

    def setdefault(self, key, default=None):
        result = self._headers.setdefault(key, default)
        self._request._set_headers(self._headers)
        return result

    def copy(self):
        return self._headers.copy()

    @property
    def raw(self):
        return self._headers.raw

    @property
    def encoding(self):
        return self._headers.encoding

    @encoding.setter
    def encoding(self, value):
        self._headers.encoding = value
        self._request._set_headers(self._headers)


class Request(_Request):
    """HTTP Request with proper stream support."""

    @property
    def stream(self):
        """Get the request body as a ByteStream (dual-mode)."""
        content = super().content
        return ByteStream(content)

    @property
    def headers(self):
        """Get headers proxy that syncs changes back to the request."""
        return _RequestHeadersProxy(self)

    @headers.setter
    def headers(self, value):
        self._set_headers(value)

    def _get_headers(self):
        """Get the underlying headers object from Rust."""
        # Use super() to access the Rust property
        return super(Request, self).headers

    def _set_headers(self, value):
        """Set the underlying headers object on Rust."""
        # Use setattr on the parent class type descriptor
        super(Request, type(self)).headers.__set__(self, value)


# ============================================================================
# Response wrapper with proper stream property
# ============================================================================

class HTTPStatusError(_HTTPStatusError):
    """HTTP Status Error with request and response attributes.

    Raised by Response.raise_for_status() when the response has a non-2xx status code.
    """

    def __init__(self, message, *, request=None, response=None):
        super().__init__(message)
        self._request = request
        self._response = response

    @property
    def request(self):
        return self._request

    @property
    def response(self):
        return self._response


class Response:
    """HTTP Response wrapper with proper stream support and raise_for_status.

    Wraps the Rust Response to provide additional Python functionality.
    Can be constructed either by wrapping a Rust Response or directly with status_code.
    """

    def __init__(self, status_code_or_response, *, content=None, headers=None,
                 text=None, html=None, json=None, stream=None, request=None):
        # If passed a Rust _Response, wrap it
        if isinstance(status_code_or_response, _Response):
            self._response = status_code_or_response
        else:
            # Construct a new Rust _Response
            self._response = _Response(
                status_code_or_response,
                content=content,
                headers=headers,
                text=text,
                html=html,
                json=json,
                stream=stream,
                request=request,
            )
        # Initialize history to empty list
        self._history = []

    def __getattr__(self, name):
        """Delegate attribute access to the underlying Rust response."""
        return getattr(self._response, name)

    @property
    def stream(self):
        """Get the response body as a ByteStream (dual-mode)."""
        content = self._response.content
        return ByteStream(content)

    @property
    def status_code(self):
        return self._response.status_code

    @property
    def reason_phrase(self):
        return self._response.reason_phrase

    @property
    def headers(self):
        return self._response.headers

    @property
    def url(self):
        return self._response.url

    @property
    def content(self):
        return self._response.content

    @property
    def text(self):
        return self._response.text

    @property
    def request(self):
        return self._response.request

    @request.setter
    def request(self, value):
        self._response.request = value

    @property
    def is_success(self):
        return self._response.is_success

    @property
    def is_informational(self):
        return self._response.is_informational

    @property
    def is_redirect(self):
        return self._response.is_redirect

    @property
    def is_client_error(self):
        return self._response.is_client_error

    @property
    def is_server_error(self):
        return self._response.is_server_error

    @property
    def history(self):
        """List of responses in redirect/auth chain."""
        return self._history

    def __repr__(self):
        return f"<Response [{self.status_code} {self.reason_phrase}]>"

    def json(self, **kwargs):
        import json
        # If no kwargs, use the fast Rust implementation
        if not kwargs:
            return self._response.json()
        # Otherwise, use Python's json.loads with kwargs
        return json.loads(self.text, **kwargs)

    def raise_for_status(self):
        """Raise HTTPStatusError for non-2xx status codes.

        Returns self for chaining on success.
        """
        if self.is_success:
            return self

        # Get URL from response
        url_str = str(self.url) if self.url else ""

        # Determine message prefix based on status type
        if self.is_informational:
            message_prefix = "Informational response"
        elif self.is_redirect:
            message_prefix = "Redirect response"
        elif self.is_client_error:
            message_prefix = "Client error"
        elif self.is_server_error:
            message_prefix = "Server error"
        else:
            message_prefix = "Error"

        # Build error message
        message = f"{message_prefix} '{self.status_code} {self.reason_phrase}' for url '{url_str}'"

        # Add redirect location for redirect responses
        if self.is_redirect:
            location = self.headers.get("location")
            if location:
                message += f"\nRedirect location: '{location}'"

        message += f"\nFor more information check: https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/{self.status_code}"

        raise HTTPStatusError(message, request=self.request, response=self)


# ============================================================================
# Auth wrappers with generator protocol
# ============================================================================

# Re-export Auth base class directly (it already supports subclassing)
Auth = _Auth


class BasicAuth:
    """HTTP Basic Authentication with generator protocol."""

    def __init__(self, username="", password=""):
        self._auth = _BasicAuth(username, password)
        self.username = username
        self.password = password

    def sync_auth_flow(self, request):
        """Generator-based sync auth flow for Basic auth."""
        import base64
        # Add Authorization header
        credentials = f"{self.username}:{self.password}"
        encoded = base64.b64encode(credentials.encode()).decode('ascii')
        request.set_header("Authorization", f"Basic {encoded}")
        yield request
        # After response, just stop (basic auth doesn't retry)

    async def async_auth_flow(self, request):
        """Generator-based async auth flow for Basic auth."""
        import base64
        # Add Authorization header
        credentials = f"{self.username}:{self.password}"
        encoded = base64.b64encode(credentials.encode()).decode('ascii')
        request.set_header("Authorization", f"Basic {encoded}")
        yield request
        # After response, just stop (basic auth doesn't retry)

    def __repr__(self):
        return f"BasicAuth(username={self.username!r}, password=***)"


class DigestAuth:
    """HTTP Digest Authentication with generator protocol."""

    def __init__(self, username="", password=""):
        self._auth = _DigestAuth(username, password)
        self.username = username
        self.password = password
        self._nonce_count = 0

    def _get_client_nonce(self):
        """Generate a client nonce."""
        import os
        return os.urandom(8).hex()  # 8 bytes = 16 hex characters

    def sync_auth_flow(self, request):
        """Generator-based sync auth flow for Digest auth."""
        import hashlib
        import re

        # First request without auth to get challenge
        response = yield request

        if response.status_code != 401:
            return

        # Parse WWW-Authenticate header
        auth_header = response.headers.get("www-authenticate", "")
        if not auth_header.lower().startswith("digest"):
            return

        # Parse digest parameters
        params = {}
        # Handle both quoted and unquoted values
        # Check for unclosed quotes (malformed header)
        header_part = auth_header[7:]  # Skip "Digest "
        if header_part.count('"') % 2 != 0:
            raise ProtocolError("Malformed Digest auth header: unclosed quote")

        for match in re.finditer(r'(\w+)=(?:"([^"]*)"|([^\s,]+))', auth_header):
            key = match.group(1).lower()
            value = match.group(2) if match.group(2) is not None else match.group(3)
            # Strip any remaining quotes from unquoted values
            if value and value.startswith('"'):
                value = value[1:]
            if value and value.endswith('"'):
                value = value[:-1]
            params[key] = value

        realm = params.get("realm", "")
        nonce = params.get("nonce", "")
        qop = params.get("qop", "")
        opaque = params.get("opaque", "")
        algorithm = params.get("algorithm", "MD5").upper()

        # Validate required fields
        if not nonce:
            raise ProtocolError("Malformed Digest auth header: missing required 'nonce' field")

        # Choose hash function
        if algorithm in ("MD5", "MD5-SESS"):
            hash_func = hashlib.md5
        elif algorithm in ("SHA", "SHA-SESS"):
            hash_func = hashlib.sha1
        elif algorithm in ("SHA-256", "SHA-256-SESS"):
            hash_func = hashlib.sha256
        elif algorithm in ("SHA-512", "SHA-512-SESS"):
            hash_func = hashlib.sha512
        else:
            hash_func = hashlib.md5

        def H(data):
            return hash_func(data.encode()).hexdigest()

        # Calculate A1
        a1 = f"{self.username}:{realm}:{self.password}"
        if algorithm.endswith("-SESS"):
            cnonce = self._get_client_nonce()
            a1 = f"{H(a1)}:{nonce}:{cnonce}"
        ha1 = H(a1)

        # Calculate A2
        method = str(request.method)
        uri = str(request.url.path)
        if request.url.query:
            uri = f"{uri}?{request.url.query}"
        a2 = f"{method}:{uri}"
        ha2 = H(a2)

        # Calculate response
        self._nonce_count += 1
        nc = f"{self._nonce_count:08x}"
        cnonce = self._get_client_nonce()

        if qop:
            # Parse qop options
            qop_options = [q.strip() for q in qop.split(",")]
            if "auth" in qop_options:
                qop_value = "auth"
            elif "auth-int" in qop_options:
                raise ProtocolError("Digest auth qop=auth-int is not implemented")
            else:
                raise ProtocolError(f"Unsupported Digest auth qop value: {qop}")
            response_value = H(f"{ha1}:{nonce}:{nc}:{cnonce}:{qop_value}:{ha2}")
        else:
            # RFC 2069 style
            response_value = H(f"{ha1}:{nonce}:{ha2}")
            qop_value = None

        # Build Authorization header
        auth_parts = [
            f'username="{self.username}"',
            f'realm="{realm}"',
            f'nonce="{nonce}"',
            f'uri="{uri}"',
            f'response="{response_value}"',
        ]
        if opaque:
            auth_parts.append(f'opaque="{opaque}"')
        # Always include algorithm
        auth_parts.append(f'algorithm={algorithm}')
        if qop_value:
            auth_parts.append(f'qop={qop_value}')
            auth_parts.append(f'nc={nc}')
            auth_parts.append(f'cnonce="{cnonce}"')

        auth_header_value = "Digest " + ", ".join(auth_parts)
        request.set_header("Authorization", auth_header_value)

        yield request

    async def async_auth_flow(self, request):
        """Generator-based async auth flow for Digest auth."""
        # Properly delegate to sync_auth_flow with response handling
        gen = self.sync_auth_flow(request)
        response = None
        try:
            while True:
                if response is None:
                    req = next(gen)
                else:
                    req = gen.send(response)
                response = yield req
        except StopIteration:
            pass

    def __repr__(self):
        return f"DigestAuth(username={self.username!r}, password=***)"


class NetRCAuth:
    """NetRC-based authentication with generator protocol."""

    def __init__(self, file=None):
        self._auth = _NetRCAuth(file)
        self._file = file

    def sync_auth_flow(self, request):
        """Generator-based sync auth flow for NetRC auth."""
        # NetRCAuth applies credentials from .netrc file
        yield request

    async def async_auth_flow(self, request):
        """Generator-based async auth flow for NetRC auth."""
        yield request

    def __repr__(self):
        return f"NetRCAuth(file={self._file!r})"


class FunctionAuth:
    """Function-based authentication with generator protocol."""

    def __init__(self, func):
        self._auth = _FunctionAuth(func)
        self._func = func

    def sync_auth_flow(self, request):
        """Generator-based sync auth flow."""
        yield request

    async def async_auth_flow(self, request):
        """Generator-based async auth flow."""
        yield request

    def __repr__(self):
        return f"FunctionAuth({self._func!r})"


# Wrap codes to support codes(404) returning int
class codes(_codes):
    """HTTP status codes with flexible access patterns."""

    def __new__(cls, code):
        """Allow codes(404) to return 404."""
        return code


# Helper to convert None to _AUTH_DISABLED sentinel for Rust
def _convert_auth(auth):
    """Convert auth parameter: None → _AUTH_DISABLED, USE_CLIENT_DEFAULT → USE_CLIENT_DEFAULT, else pass through."""
    if auth is None:
        return _AUTH_DISABLED
    return auth

# Wrap AsyncClient to support auth=None vs auth not specified
# We use a wrapper class that delegates to the Rust implementation
class AsyncClient:
    """Async HTTP client that wraps the Rust implementation with proper auth sentinel handling."""

    def __init__(self, *args, **kwargs):
        # Extract auth from kwargs before passing to Rust client
        auth = kwargs.pop('auth', None)
        # Validate and convert auth value
        if auth is None:
            self._auth = None
        elif isinstance(auth, tuple) and len(auth) == 2:
            self._auth = BasicAuth(auth[0], auth[1])
        elif callable(auth) or hasattr(auth, 'sync_auth_flow') or hasattr(auth, 'async_auth_flow'):
            self._auth = auth
        else:
            raise TypeError(f"Invalid 'auth' argument. Expected (username, password) tuple, Auth instance, or callable. Got {type(auth).__name__}.")
        # Store transport reference for Python-level handling
        self._transport = kwargs.get('transport', None)
        self._client = _AsyncClient(*args, **kwargs)
        self._is_closed = False

    def __getattr__(self, name):
        """Delegate attribute access to the underlying client."""
        return getattr(self._client, name)

    async def __aenter__(self):
        if self._is_closed:
            raise RuntimeError("Cannot open a client that has been closed")
        # Call transport's __aenter__ if it exists
        if self._transport is not None and hasattr(self._transport, '__aenter__'):
            await self._transport.__aenter__()
        await self._client.__aenter__()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        result = await self._client.__aexit__(exc_type, exc_val, exc_tb)
        # Call transport's __aexit__ if it exists
        if self._transport is not None and hasattr(self._transport, '__aexit__'):
            await self._transport.__aexit__(exc_type, exc_val, exc_tb)
        self._is_closed = True
        return result

    async def aclose(self):
        """Close the client."""
        if hasattr(self._client, 'aclose'):
            await self._client.aclose()
        if self._transport is not None and hasattr(self._transport, 'aclose'):
            await self._transport.aclose()
        self._is_closed = True

    @property
    def is_closed(self):
        """Return True if the client has been closed."""
        return getattr(self, '_is_closed', False)

    def _check_closed(self):
        """Raise RuntimeError if the client is closed."""
        if self._is_closed:
            raise RuntimeError("Cannot send request on a closed client")

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
        elif callable(value) or hasattr(value, 'sync_auth_flow') or hasattr(value, 'async_auth_flow'):
            self._auth = value
        else:
            raise TypeError(f"Invalid 'auth' argument. Expected (username, password) tuple, Auth instance, or callable. Got {type(value).__name__}.")

    def build_request(self, method, url, **kwargs):
        """Build a Request object - wrap result in Python Request class."""
        rust_request = self._client.build_request(method, url, **kwargs)
        # Create a wrapper that delegates to the Rust request but has our headers proxy
        return _WrappedRequest(rust_request)

    async def send(self, request, **kwargs):
        """Send a Request object."""
        auth = kwargs.pop('auth', None)
        if auth is not None:
            return await self._send_with_auth(request, auth)
        return await self._send_single_request(request)

    async def _send_single_request(self, request):
        """Send a single request, handling transport properly."""
        if self._is_closed:
            raise RuntimeError("Cannot send request on a closed client")

        # Get the Rust request object
        if isinstance(request, _WrappedRequest):
            rust_request = request._rust_request
        elif hasattr(request, '_rust_request'):
            rust_request = request._rust_request
        else:
            rust_request = request

        # If we have a custom transport, use it directly
        if self._transport is not None:
            # Check for async handle method
            if hasattr(self._transport, 'handle_async_request'):
                result = await self._transport.handle_async_request(rust_request)
            elif hasattr(self._transport, 'handle_request'):
                result = self._transport.handle_request(rust_request)
            elif callable(self._transport):
                result = self._transport(rust_request)
            else:
                raise TypeError("Transport must have handle_async_request or handle_request method")

            # Wrap result in Response if needed
            if isinstance(result, Response):
                return result
            elif isinstance(result, _Response):
                return Response(result)
            else:
                return Response(result)
        else:
            # Use the Rust client's send
            result = await self._client.send(rust_request)
            return Response(result)

    async def _send_with_auth(self, request, auth):
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
        if auth is not None:
            import inspect
            if hasattr(auth, 'async_auth_flow'):
                method = getattr(auth, 'async_auth_flow')
                # Check if it's a generator function (Python auth) or not (Rust auth)
                if inspect.isgeneratorfunction(method) or inspect.isasyncgenfunction(method):
                    auth_flow = auth.async_auth_flow(wrapped_request)
                else:
                    # Rust auth - pass the underlying request
                    auth_flow = auth.async_auth_flow(wrapped_request._rust_request)
            elif hasattr(auth, 'sync_auth_flow'):
                method = getattr(auth, 'sync_auth_flow')
                if inspect.isgeneratorfunction(method):
                    auth_flow = auth.sync_auth_flow(wrapped_request)
                else:
                    # Rust auth - pass the underlying request
                    auth_flow = auth.sync_auth_flow(wrapped_request._rust_request)

        if auth_flow is None:
            # No auth flow, send directly
            return await self._send_single_request(wrapped_request)

        # Check if auth_flow returned a list (Rust base class) or generator
        import types
        if isinstance(auth_flow, (list, tuple)):
            # Simple list of requests - just send the last one
            last_request = wrapped_request
            for req in auth_flow:
                last_request = req
            return await self._send_single_request(last_request)

        # Generator-based auth flow
        history = []
        try:
            # Check if it's an async generator
            if hasattr(auth_flow, '__anext__'):
                # Async generator
                request = await auth_flow.__anext__()
                response = await self._send_single_request(request)

                while True:
                    try:
                        request = await auth_flow.asend(response)
                        response._history = list(history)
                        history.append(response)
                        response = await self._send_single_request(request)
                    except StopAsyncIteration:
                        break
            else:
                # Sync generator
                request = next(auth_flow)
                response = await self._send_single_request(request)

                while True:
                    try:
                        request = auth_flow.send(response)
                        response._history = list(history)
                        history.append(response)
                        response = await self._send_single_request(request)
                    except StopIteration:
                        break

            if history:
                response._history = history
            return response
        except (StopIteration, StopAsyncIteration):
            return await self._send_single_request(wrapped_request)

    async def get(self, url, *, params=None, headers=None, cookies=None,
                  auth=USE_CLIENT_DEFAULT, follow_redirects=None, timeout=None):
        """HTTP GET with proper auth sentinel handling."""
        self._check_closed()
        actual_auth = auth if auth is not USE_CLIENT_DEFAULT else self._auth
        if actual_auth is not None:
            result = await self._handle_auth("GET", url, actual_auth, params=params, headers=headers)
            if result is not None:
                return result
        response = await self._client.get(url, params=params, headers=headers, cookies=cookies,
                                      auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)
        return Response(response)

    async def _handle_auth(self, method, url, actual_auth, **build_kwargs):
        """Handle auth for async requests - supports generators and callables."""
        # Convert tuple to BasicAuth
        if isinstance(actual_auth, tuple) and len(actual_auth) == 2:
            actual_auth = BasicAuth(actual_auth[0], actual_auth[1])

        request = self.build_request(method, url, **build_kwargs)
        if hasattr(actual_auth, 'async_auth_flow') or hasattr(actual_auth, 'sync_auth_flow'):
            return await self._send_with_auth(request, actual_auth)
        elif callable(actual_auth):
            # Callable auth - call it with the wrapped request
            modified = actual_auth(request)
            return await self._send_single_request(modified if modified is not None else request)
        else:
            # Invalid auth type
            raise TypeError(f"Invalid 'auth' argument. Expected (username, password) tuple, Auth instance, or callable. Got {type(actual_auth).__name__}.")

    async def post(self, url, *, content=None, data=None, files=None, json=None,
                   params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
                   follow_redirects=None, timeout=None):
        """HTTP POST with proper auth sentinel handling."""
        self._check_closed()
        actual_auth = auth if auth is not USE_CLIENT_DEFAULT else self._auth
        if actual_auth is not None:
            result = await self._handle_auth("POST", url, actual_auth, content=content, params=params, headers=headers)
            if result is not None:
                return result
        response = await self._client.post(url, content=content, data=data, files=files, json=json,
                                       params=params, headers=headers, cookies=cookies,
                                       auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)
        return Response(response)

    async def put(self, url, *, content=None, data=None, files=None, json=None,
                  params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
                  follow_redirects=None, timeout=None):
        """HTTP PUT with proper auth sentinel handling."""
        self._check_closed()
        actual_auth = auth if auth is not USE_CLIENT_DEFAULT else self._auth
        if actual_auth is not None:
            result = await self._handle_auth("PUT", url, actual_auth, content=content, params=params, headers=headers)
            if result is not None:
                return result
        response = await self._client.put(url, content=content, data=data, files=files, json=json,
                                      params=params, headers=headers, cookies=cookies,
                                      auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)
        return Response(response)

    async def patch(self, url, *, content=None, data=None, files=None, json=None,
                    params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
                    follow_redirects=None, timeout=None):
        """HTTP PATCH with proper auth sentinel handling."""
        self._check_closed()
        actual_auth = auth if auth is not USE_CLIENT_DEFAULT else self._auth
        if actual_auth is not None:
            result = await self._handle_auth("PATCH", url, actual_auth, content=content, params=params, headers=headers)
            if result is not None:
                return result
        response = await self._client.patch(url, content=content, data=data, files=files, json=json,
                                        params=params, headers=headers, cookies=cookies,
                                        auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)
        return Response(response)

    async def delete(self, url, *, params=None, headers=None, cookies=None,
                     auth=USE_CLIENT_DEFAULT, follow_redirects=None, timeout=None):
        """HTTP DELETE with proper auth sentinel handling."""
        self._check_closed()
        actual_auth = auth if auth is not USE_CLIENT_DEFAULT else self._auth
        if actual_auth is not None:
            result = await self._handle_auth("DELETE", url, actual_auth, params=params, headers=headers)
            if result is not None:
                return result
        response = await self._client.delete(url, params=params, headers=headers, cookies=cookies,
                                         auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)
        return Response(response)

    async def head(self, url, *, params=None, headers=None, cookies=None,
                   auth=USE_CLIENT_DEFAULT, follow_redirects=None, timeout=None):
        """HTTP HEAD with proper auth sentinel handling."""
        self._check_closed()
        actual_auth = auth if auth is not USE_CLIENT_DEFAULT else self._auth
        if actual_auth is not None:
            result = await self._handle_auth("HEAD", url, actual_auth, params=params, headers=headers)
            if result is not None:
                return result
        response = await self._client.head(url, params=params, headers=headers, cookies=cookies,
                                       auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)
        return Response(response)

    async def options(self, url, *, params=None, headers=None, cookies=None,
                      auth=USE_CLIENT_DEFAULT, follow_redirects=None, timeout=None):
        """HTTP OPTIONS with proper auth sentinel handling."""
        self._check_closed()
        actual_auth = auth if auth is not USE_CLIENT_DEFAULT else self._auth
        if actual_auth is not None:
            result = await self._handle_auth("OPTIONS", url, actual_auth, params=params, headers=headers)
            if result is not None:
                return result
        response = await self._client.options(url, params=params, headers=headers, cookies=cookies,
                                          auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)
        return Response(response)

    async def request(self, method, url, *, content=None, data=None, files=None, json=None,
                      params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
                      follow_redirects=None, timeout=None):
        """HTTP request with proper auth sentinel handling."""
        self._check_closed()
        actual_auth = auth if auth is not USE_CLIENT_DEFAULT else self._auth
        if actual_auth is not None:
            result = await self._handle_auth(method, url, actual_auth, content=content, params=params, headers=headers)
            if result is not None:
                return result
        response = await self._client.request(method, url, content=content, data=data, files=files,
                                          json=json, params=params, headers=headers, cookies=cookies,
                                          auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)
        return Response(response)

    @contextlib.asynccontextmanager
    async def stream(self, method, url, *, content=None, data=None, files=None, json=None,
                     params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
                     follow_redirects=None, timeout=None):
        """Stream an HTTP request with proper auth handling."""
        actual_auth = auth if auth is not USE_CLIENT_DEFAULT else self._auth
        response = None
        try:
            if actual_auth is not None:
                # Build request with auth - build_request only supports certain params
                build_kwargs = {}
                if content is not None:
                    build_kwargs['content'] = content
                if params is not None:
                    build_kwargs['params'] = params
                if headers is not None:
                    build_kwargs['headers'] = headers
                if cookies is not None:
                    build_kwargs['cookies'] = cookies
                if json is not None:
                    build_kwargs['json'] = json
                request = self.build_request(method, url, **build_kwargs)
                # Apply auth
                if hasattr(actual_auth, 'async_auth_flow') or hasattr(actual_auth, 'sync_auth_flow'):
                    response = await self._send_with_auth(request, actual_auth)
                elif callable(actual_auth):
                    modified = actual_auth(request)
                    response = await self._send_single_request(modified if modified is not None else request)
            if response is None:
                response = await self.request(method, url, content=content, data=data, files=files,
                                            json=json, params=params, headers=headers, cookies=cookies,
                                            auth=auth, follow_redirects=follow_redirects, timeout=timeout)
            yield response
        finally:
            # Cleanup if needed
            pass


# Wrap sync Client to support auth=None vs auth not specified
class _HeadersProxy:
    """Proxy object that wraps Headers and syncs changes back to the client."""

    def __init__(self, client):
        self._client = client
        self._headers = client._client.headers

    def __getitem__(self, key):
        return self._headers[key]

    def __setitem__(self, key, value):
        self._headers[key] = value
        self._client._client.headers = self._headers

    def __delitem__(self, key):
        del self._headers[key]
        self._client._client.headers = self._headers

    def __contains__(self, key):
        return key in self._headers

    def __iter__(self):
        return iter(self._headers)

    def __len__(self):
        return len(self._headers)

    def __eq__(self, other):
        return self._headers == other

    def __repr__(self):
        return repr(self._headers)

    def get(self, key, default=None):
        return self._headers.get(key, default)

    def get_list(self, key, split_commas=False):
        return self._headers.get_list(key, split_commas)

    def keys(self):
        return self._headers.keys()

    def values(self):
        return self._headers.values()

    def items(self):
        return self._headers.items()

    def multi_items(self):
        return self._headers.multi_items()

    def update(self, other):
        self._headers.update(other)
        self._client._client.headers = self._headers

    def setdefault(self, key, default=None):
        result = self._headers.setdefault(key, default)
        self._client._client.headers = self._headers
        return result

    def copy(self):
        return self._headers.copy()

    @property
    def raw(self):
        return self._headers.raw

    @property
    def encoding(self):
        return self._headers.encoding

    @encoding.setter
    def encoding(self, value):
        self._headers.encoding = value
        self._client._client.headers = self._headers


class Client:
    """Sync HTTP client that wraps the Rust implementation with proper auth sentinel handling."""

    def __init__(self, *args, **kwargs):
        # Extract auth and transport from kwargs before passing to Rust client
        auth = kwargs.pop('auth', None)
        # Validate and convert auth value
        if auth is None:
            self._auth = None
        elif isinstance(auth, tuple) and len(auth) == 2:
            self._auth = BasicAuth(auth[0], auth[1])
        elif callable(auth) or hasattr(auth, 'sync_auth_flow') or hasattr(auth, 'async_auth_flow'):
            self._auth = auth
        else:
            raise TypeError(f"Invalid 'auth' argument. Expected (username, password) tuple, Auth instance, or callable. Got {type(auth).__name__}.")
        self._transport = kwargs.get('transport', None)  # Keep in kwargs for Rust
        self._client = _Client(*args, **kwargs)
        self._headers_proxy = None
        self._is_closed = False

    def __getattr__(self, name):
        """Delegate attribute access to the underlying client."""
        return getattr(self._client, name)

    def __enter__(self):
        if self._is_closed:
            raise RuntimeError("Cannot open a client that has been closed")
        # Call transport's __enter__ if it exists
        if self._transport is not None and hasattr(self._transport, '__enter__'):
            self._transport.__enter__()
        self._client.__enter__()
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        result = self._client.__exit__(exc_type, exc_val, exc_tb)
        # Call transport's __exit__ if it exists
        if self._transport is not None and hasattr(self._transport, '__exit__'):
            self._transport.__exit__(exc_type, exc_val, exc_tb)
        self._is_closed = True
        return result

    def close(self):
        """Close the client."""
        if hasattr(self._client, 'close'):
            self._client.close()
        if self._transport is not None and hasattr(self._transport, 'close'):
            self._transport.close()
        self._is_closed = True

    @property
    def is_closed(self):
        """Return True if the client has been closed."""
        return getattr(self, '_is_closed', False)

    @property
    def base_url(self):
        return self._client.base_url

    @base_url.setter
    def base_url(self, value):
        self._client.base_url = value

    @property
    def headers(self):
        # Create a new proxy each time to ensure it has the latest headers
        return _HeadersProxy(self)

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
        elif callable(value) or hasattr(value, 'sync_auth_flow') or hasattr(value, 'async_auth_flow'):
            self._auth = value
        else:
            raise TypeError(f"Invalid 'auth' argument. Expected (username, password) tuple, Auth instance, or callable. Got {type(value).__name__}.")

    def build_request(self, method, url, **kwargs):
        """Build a Request object - wrap result in Python Request class."""
        rust_request = self._client.build_request(method, url, **kwargs)
        # Create a wrapper that delegates to the Rust request but has our headers proxy
        return _WrappedRequest(rust_request)

    def _wrap_response(self, rust_response):
        """Wrap a Rust response in a Python Response."""
        return Response(rust_response)

    def _send_single_request(self, request):
        """Send a single request, handling transport properly."""
        if self._is_closed:
            raise RuntimeError("Cannot send request on a closed client")

        if isinstance(request, _WrappedRequest):
            rust_request = request._rust_request
        elif hasattr(request, '_rust_request'):
            rust_request = request._rust_request
        else:
            rust_request = request

        if self._transport is not None:
            if hasattr(self._transport, 'handle_request'):
                result = self._transport.handle_request(rust_request)
            elif callable(self._transport):
                result = self._transport(rust_request)
            else:
                raise TypeError("Transport must have handle_request method")
            # Wrap result in Response if needed
            if isinstance(result, Response):
                return result
            elif isinstance(result, _Response):
                return Response(result)
            else:
                return Response(result)
        else:
            result = self._client.send(rust_request)
            return Response(result)

    def _handle_auth(self, method, url, actual_auth, **build_kwargs):
        """Handle auth for sync requests - supports generators and callables."""
        # Convert tuple to BasicAuth
        if isinstance(actual_auth, tuple) and len(actual_auth) == 2:
            actual_auth = BasicAuth(actual_auth[0], actual_auth[1])

        request = self.build_request(method, url, **build_kwargs)
        # Check for generator-based auth
        if hasattr(actual_auth, 'sync_auth_flow') or hasattr(actual_auth, 'auth_flow'):
            return self._send_with_auth(request, actual_auth)
        # Check for callable auth (function that modifies request)
        elif callable(actual_auth):
            modified = actual_auth(request)
            return self._send_single_request(modified if modified is not None else request)
        else:
            # Invalid auth type
            raise TypeError(f"Invalid 'auth' argument. Expected (username, password) tuple, Auth instance, or callable. Got {type(actual_auth).__name__}.")

    def _send_with_auth(self, request, auth):
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
            if 'auth_flow' in auth_type.__dict__ or (hasattr(auth, 'auth_flow') and callable(getattr(auth, 'auth_flow'))):
                auth_flow_method = getattr(auth, 'auth_flow', None)
                if auth_flow_method and (inspect.isgeneratorfunction(auth_flow_method) or
                                         (hasattr(auth_flow_method, '__func__') and
                                          inspect.isgeneratorfunction(auth_flow_method.__func__))):
                    # Python generator - pass wrapped request for header mutations
                    auth_flow = auth.auth_flow(wrapped_request)
            if auth_flow is None and hasattr(auth, 'sync_auth_flow'):
                method = getattr(auth, 'sync_auth_flow')
                if inspect.isgeneratorfunction(method) or (hasattr(method, '__func__') and inspect.isgeneratorfunction(method.__func__)):
                    # Python generator - pass wrapped request
                    auth_flow = auth.sync_auth_flow(wrapped_request)
                else:
                    # Rust auth - pass the underlying request
                    auth_flow = auth.sync_auth_flow(wrapped_request._rust_request)

        if auth_flow is None:
            # No auth flow, send directly
            return self._send_single_request(wrapped_request)

        # Check if auth_flow returned a list (Rust base class) or generator
        import types
        if isinstance(auth_flow, (list, tuple)):
            # Simple list of requests - just send the last one
            last_request = wrapped_request
            for req in auth_flow:
                last_request = req
            return self._send_single_request(last_request)

        # Generator-based auth flow
        history = []  # Track intermediate responses
        try:
            # Get the first yielded request (possibly with auth headers added)
            request = next(auth_flow)
            # Send it and get the response
            response = self._send_single_request(request)

            # Continue the auth flow with the response (for digest auth, etc.)
            while True:
                try:
                    # Try to get next request - if this succeeds, current response is intermediate
                    request = auth_flow.send(response)
                    # Set cumulative history on current response before adding to history
                    response._history = list(history)  # Copy current history to this response
                    # Add current response to history since there's a next request
                    history.append(response)
                    # Send next request
                    response = self._send_single_request(request)
                except StopIteration:
                    # No more requests - current response is the final one
                    break

            # Set history on final response
            if history:
                response._history = history
            return response
        except StopIteration:
            # Auth flow returned without yielding, send request as-is
            return self._send_single_request(wrapped_request)

    def send(self, request, **kwargs):
        """Send a Request object."""
        auth = kwargs.pop('auth', None)
        if auth is not None:
            return self._send_with_auth(request, auth)
        # Route through _send_single_request which handles transport
        return self._send_single_request(request)

    def _check_closed(self):
        """Raise RuntimeError if the client is closed."""
        if self._is_closed:
            raise RuntimeError("Cannot send request on a closed client")

    def get(self, url, *, params=None, headers=None, cookies=None,
            auth=USE_CLIENT_DEFAULT, follow_redirects=None, timeout=None):
        """HTTP GET with proper auth sentinel handling."""
        self._check_closed()
        actual_auth = auth if auth is not USE_CLIENT_DEFAULT else self._auth
        if actual_auth is not None:
            result = self._handle_auth("GET", url, actual_auth, params=params, headers=headers, cookies=cookies)
            if result is not None:
                return result
        return self._wrap_response(self._client.get(url, params=params, headers=headers, cookies=cookies,
                                auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout))

    def post(self, url, *, content=None, data=None, files=None, json=None,
             params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
             follow_redirects=None, timeout=None):
        """HTTP POST with proper auth sentinel handling."""
        self._check_closed()
        actual_auth = auth if auth is not USE_CLIENT_DEFAULT else self._auth
        if actual_auth is not None:
            result = self._handle_auth("POST", url, actual_auth, content=content, data=data, files=files,
                                      json=json, params=params, headers=headers, cookies=cookies)
            if result is not None:
                return result
        return self._wrap_response(self._client.post(url, content=content, data=data, files=files, json=json,
                                 params=params, headers=headers, cookies=cookies,
                                 auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout))

    def put(self, url, *, content=None, data=None, files=None, json=None,
            params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
            follow_redirects=None, timeout=None):
        """HTTP PUT with proper auth sentinel handling."""
        self._check_closed()
        actual_auth = auth if auth is not USE_CLIENT_DEFAULT else self._auth
        if actual_auth is not None:
            result = self._handle_auth("PUT", url, actual_auth, content=content, data=data, files=files,
                                      json=json, params=params, headers=headers, cookies=cookies)
            if result is not None:
                return result
        return self._wrap_response(self._client.put(url, content=content, data=data, files=files, json=json,
                                params=params, headers=headers, cookies=cookies,
                                auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout))

    def patch(self, url, *, content=None, data=None, files=None, json=None,
              params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
              follow_redirects=None, timeout=None):
        """HTTP PATCH with proper auth sentinel handling."""
        self._check_closed()
        actual_auth = auth if auth is not USE_CLIENT_DEFAULT else self._auth
        if actual_auth is not None:
            result = self._handle_auth("PATCH", url, actual_auth, content=content, data=data, files=files,
                                      json=json, params=params, headers=headers, cookies=cookies)
            if result is not None:
                return result
        return self._wrap_response(self._client.patch(url, content=content, data=data, files=files, json=json,
                                  params=params, headers=headers, cookies=cookies,
                                  auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout))

    def delete(self, url, *, params=None, headers=None, cookies=None,
               auth=USE_CLIENT_DEFAULT, follow_redirects=None, timeout=None):
        """HTTP DELETE with proper auth sentinel handling."""
        self._check_closed()
        actual_auth = auth if auth is not USE_CLIENT_DEFAULT else self._auth
        if actual_auth is not None:
            result = self._handle_auth("DELETE", url, actual_auth, params=params, headers=headers, cookies=cookies)
            if result is not None:
                return result
        return self._wrap_response(self._client.delete(url, params=params, headers=headers, cookies=cookies,
                                   auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout))

    def head(self, url, *, params=None, headers=None, cookies=None,
             auth=USE_CLIENT_DEFAULT, follow_redirects=None, timeout=None):
        """HTTP HEAD with proper auth sentinel handling."""
        self._check_closed()
        actual_auth = auth if auth is not USE_CLIENT_DEFAULT else self._auth
        if actual_auth is not None:
            result = self._handle_auth("HEAD", url, actual_auth, params=params, headers=headers, cookies=cookies)
            if result is not None:
                return result
        return self._wrap_response(self._client.head(url, params=params, headers=headers, cookies=cookies,
                                 auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout))

    def options(self, url, *, params=None, headers=None, cookies=None,
                auth=USE_CLIENT_DEFAULT, follow_redirects=None, timeout=None):
        """HTTP OPTIONS with proper auth sentinel handling."""
        self._check_closed()
        actual_auth = auth if auth is not USE_CLIENT_DEFAULT else self._auth
        if actual_auth is not None:
            result = self._handle_auth("OPTIONS", url, actual_auth, params=params, headers=headers, cookies=cookies)
            if result is not None:
                return result
        return self._wrap_response(self._client.options(url, params=params, headers=headers, cookies=cookies,
                                    auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout))

    def request(self, method, url, *, content=None, data=None, files=None, json=None,
                params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
                follow_redirects=None, timeout=None):
        """HTTP request with proper auth sentinel handling."""
        self._check_closed()
        actual_auth = auth if auth is not USE_CLIENT_DEFAULT else self._auth
        if actual_auth is not None:
            result = self._handle_auth(method, url, actual_auth, content=content, data=data, files=files,
                                      json=json, params=params, headers=headers, cookies=cookies)
            if result is not None:
                return result
        return self._wrap_response(self._client.request(method, url, content=content, data=data, files=files,
                                    json=json, params=params, headers=headers, cookies=cookies,
                                    auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout))

    @contextlib.contextmanager
    def stream(self, method, url, *, content=None, data=None, files=None, json=None,
               params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
               follow_redirects=None, timeout=None):
        """Stream an HTTP request with proper auth handling."""
        actual_auth = auth if auth is not USE_CLIENT_DEFAULT else self._auth
        response = None
        try:
            if actual_auth is not None:
                # Build request with auth - build_request only supports certain params
                build_kwargs = {}
                if content is not None:
                    build_kwargs['content'] = content
                if params is not None:
                    build_kwargs['params'] = params
                if headers is not None:
                    build_kwargs['headers'] = headers
                if cookies is not None:
                    build_kwargs['cookies'] = cookies
                if json is not None:
                    build_kwargs['json'] = json
                request = self.build_request(method, url, **build_kwargs)
                # Apply auth
                if hasattr(actual_auth, 'sync_auth_flow') or hasattr(actual_auth, 'auth_flow'):
                    response = self._send_with_auth(request, actual_auth)
                elif callable(actual_auth):
                    modified = actual_auth(request)
                    response = self._send_single_request(modified if modified is not None else request)
            if response is None:
                response = self.request(method, url, content=content, data=data, files=files,
                                       json=json, params=params, headers=headers, cookies=cookies,
                                       auth=auth, follow_redirects=follow_redirects, timeout=timeout)
            yield response
        finally:
            # Cleanup if needed
            pass


# Import _utils module for utility functions
from . import _utils


def create_ssl_context(
    cert=None,
    verify=True,
    trust_env=True,
    http2=False,
):
    """
    Create an SSL context for use with httpx.

    Args:
        cert: Optional SSL certificate to use for client authentication.
              Can be:
              - A path to a certificate file (str or Path)
              - A tuple of (cert_file, key_file)
              - A tuple of (cert_file, key_file, password)
        verify: SSL verification mode. Can be:
                - True: Verify server certificates (default)
                - False: Disable verification (not recommended)
                - str or Path: Path to a CA bundle file
        trust_env: Whether to trust environment variables for SSL configuration.
        http2: Whether to use HTTP/2.

    Returns:
        An ssl.SSLContext instance configured with the specified options.
    """
    import ssl
    import os
    from pathlib import Path

    # Create default SSL context
    context = ssl.create_default_context()

    # Handle verify argument
    if verify is False:
        context.check_hostname = False
        context.verify_mode = ssl.CERT_NONE
    elif verify is not True:
        # verify is a path to CA bundle
        verify_path = Path(verify) if not isinstance(verify, Path) else verify
        if verify_path.is_dir():
            context.load_verify_locations(capath=str(verify_path))
        elif verify_path.is_file():
            context.load_verify_locations(cafile=str(verify_path))
        else:
            raise IOError(f"Could not find a suitable TLS CA certificate bundle, invalid path: {verify}")

    # Handle client certificate
    if cert is not None:
        if isinstance(cert, str) or isinstance(cert, Path):
            context.load_cert_chain(certfile=str(cert))
        elif isinstance(cert, tuple):
            if len(cert) == 2:
                certfile, keyfile = cert
                context.load_cert_chain(certfile=str(certfile), keyfile=str(keyfile))
            elif len(cert) == 3:
                certfile, keyfile, password = cert
                context.load_cert_chain(certfile=str(certfile), keyfile=str(keyfile), password=password)

    # Handle trust_env for SSL_CERT_FILE and SSL_CERT_DIR
    if trust_env:
        ssl_cert_file = os.environ.get("SSL_CERT_FILE")
        ssl_cert_dir = os.environ.get("SSL_CERT_DIR")
        if ssl_cert_file:
            context.load_verify_locations(cafile=ssl_cert_file)
        if ssl_cert_dir:
            context.load_verify_locations(capath=ssl_cert_dir)

    # Configure SSLKEYLOGFILE for debugging
    if trust_env:
        sslkeylogfile = os.environ.get("SSLKEYLOGFILE")
        if sslkeylogfile:
            context.keylog_filename = sslkeylogfile

    return context


__all__ = [
    "__description__",
    "__title__",
    "__version__",
    "AsyncByteStream",
    "AsyncClient",
    "AsyncBaseTransport",
    "AsyncHTTPTransport",
    "AsyncMockTransport",
    "Auth",
    "BaseTransport",
    "BasicAuth",
    "ByteStream",
    "Client",
    "CloseError",
    "codes",
    "ConnectError",
    "ConnectTimeout",
    "CookieConflict",
    "Cookies",
    "create_ssl_context",
    "DecodingError",
    "delete",
    "DigestAuth",
    "FunctionAuth",
    "get",
    "head",
    "Headers",
    "HTTPError",
    "HTTPStatusError",
    "HTTPTransport",
    "InvalidURL",
    "Limits",
    "LocalProtocolError",
    "MockTransport",
    "NetRCAuth",
    "NetworkError",
    "options",
    "patch",
    "PoolTimeout",
    "post",
    "ProtocolError",
    "Proxy",
    "ProxyError",
    "put",
    "QueryParams",
    "ReadError",
    "ReadTimeout",
    "RemoteProtocolError",
    "request",
    "Request",
    "RequestError",
    "RequestNotRead",
    "Response",
    "ResponseNotRead",
    "stream",
    "StreamClosed",
    "StreamConsumed",
    "StreamError",
    "SyncByteStream",
    "Timeout",
    "TimeoutException",
    "TooManyRedirects",
    "TransportError",
    "UnsupportedProtocol",
    "URL",
    "USE_CLIENT_DEFAULT",
    "WriteError",
    "WriteTimeout",
    "WSGITransport",
]
