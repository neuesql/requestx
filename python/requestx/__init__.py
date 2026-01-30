# RequestX - High-performance Python HTTP client
# API-compatible with httpx, powered by Rust's reqwest via PyO3

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
    # Auth types
    BasicAuth,
    DigestAuth,
    NetRCAuth,
    Auth,
    FunctionAuth,
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
    # Exceptions
    HTTPStatusError,
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

class Request(_Request):
    """HTTP Request with proper stream support."""

    @property
    def stream(self):
        """Get the request body as a ByteStream (dual-mode)."""
        content = super().content
        return ByteStream(content)


# ============================================================================
# Response wrapper with proper stream property
# ============================================================================

class Response(_Response):
    """HTTP Response with proper stream support."""

    @property
    def stream(self):
        """Get the response body as a ByteStream (dual-mode)."""
        content = super().content
        return ByteStream(content)


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
        self._client = _AsyncClient(*args, **kwargs)

    def __getattr__(self, name):
        """Delegate attribute access to the underlying client."""
        return getattr(self._client, name)

    async def __aenter__(self):
        await self._client.__aenter__()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        return await self._client.__aexit__(exc_type, exc_val, exc_tb)

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
        return self._client.auth

    @auth.setter
    def auth(self, value):
        self._client.auth = value

    async def get(self, url, *, params=None, headers=None, cookies=None,
                  auth=USE_CLIENT_DEFAULT, follow_redirects=None, timeout=None):
        """HTTP GET with proper auth sentinel handling."""
        return await self._client.get(url, params=params, headers=headers, cookies=cookies,
                                      auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)

    async def post(self, url, *, content=None, data=None, files=None, json=None,
                   params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
                   follow_redirects=None, timeout=None):
        """HTTP POST with proper auth sentinel handling."""
        return await self._client.post(url, content=content, data=data, files=files, json=json,
                                       params=params, headers=headers, cookies=cookies,
                                       auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)

    async def put(self, url, *, content=None, data=None, files=None, json=None,
                  params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
                  follow_redirects=None, timeout=None):
        """HTTP PUT with proper auth sentinel handling."""
        return await self._client.put(url, content=content, data=data, files=files, json=json,
                                      params=params, headers=headers, cookies=cookies,
                                      auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)

    async def patch(self, url, *, content=None, data=None, files=None, json=None,
                    params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
                    follow_redirects=None, timeout=None):
        """HTTP PATCH with proper auth sentinel handling."""
        return await self._client.patch(url, content=content, data=data, files=files, json=json,
                                        params=params, headers=headers, cookies=cookies,
                                        auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)

    async def delete(self, url, *, params=None, headers=None, cookies=None,
                     auth=USE_CLIENT_DEFAULT, follow_redirects=None, timeout=None):
        """HTTP DELETE with proper auth sentinel handling."""
        return await self._client.delete(url, params=params, headers=headers, cookies=cookies,
                                         auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)

    async def head(self, url, *, params=None, headers=None, cookies=None,
                   auth=USE_CLIENT_DEFAULT, follow_redirects=None, timeout=None):
        """HTTP HEAD with proper auth sentinel handling."""
        return await self._client.head(url, params=params, headers=headers, cookies=cookies,
                                       auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)

    async def options(self, url, *, params=None, headers=None, cookies=None,
                      auth=USE_CLIENT_DEFAULT, follow_redirects=None, timeout=None):
        """HTTP OPTIONS with proper auth sentinel handling."""
        return await self._client.options(url, params=params, headers=headers, cookies=cookies,
                                          auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)

    async def request(self, method, url, *, content=None, data=None, files=None, json=None,
                      params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
                      follow_redirects=None, timeout=None):
        """HTTP request with proper auth sentinel handling."""
        return await self._client.request(method, url, content=content, data=data, files=files,
                                          json=json, params=params, headers=headers, cookies=cookies,
                                          auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)


# Wrap sync Client to support auth=None vs auth not specified
class Client:
    """Sync HTTP client that wraps the Rust implementation with proper auth sentinel handling."""

    def __init__(self, *args, **kwargs):
        self._client = _Client(*args, **kwargs)

    def __getattr__(self, name):
        """Delegate attribute access to the underlying client."""
        return getattr(self._client, name)

    def __enter__(self):
        self._client.__enter__()
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        return self._client.__exit__(exc_type, exc_val, exc_tb)

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
        return self._client.auth

    @auth.setter
    def auth(self, value):
        self._client.auth = value

    def get(self, url, *, params=None, headers=None, cookies=None,
            auth=USE_CLIENT_DEFAULT, follow_redirects=None, timeout=None):
        """HTTP GET with proper auth sentinel handling."""
        return self._client.get(url, params=params, headers=headers, cookies=cookies,
                                auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)

    def post(self, url, *, content=None, data=None, files=None, json=None,
             params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
             follow_redirects=None, timeout=None):
        """HTTP POST with proper auth sentinel handling."""
        return self._client.post(url, content=content, data=data, files=files, json=json,
                                 params=params, headers=headers, cookies=cookies,
                                 auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)

    def put(self, url, *, content=None, data=None, files=None, json=None,
            params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
            follow_redirects=None, timeout=None):
        """HTTP PUT with proper auth sentinel handling."""
        return self._client.put(url, content=content, data=data, files=files, json=json,
                                params=params, headers=headers, cookies=cookies,
                                auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)

    def patch(self, url, *, content=None, data=None, files=None, json=None,
              params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
              follow_redirects=None, timeout=None):
        """HTTP PATCH with proper auth sentinel handling."""
        return self._client.patch(url, content=content, data=data, files=files, json=json,
                                  params=params, headers=headers, cookies=cookies,
                                  auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)

    def delete(self, url, *, params=None, headers=None, cookies=None,
               auth=USE_CLIENT_DEFAULT, follow_redirects=None, timeout=None):
        """HTTP DELETE with proper auth sentinel handling."""
        return self._client.delete(url, params=params, headers=headers, cookies=cookies,
                                   auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)

    def head(self, url, *, params=None, headers=None, cookies=None,
             auth=USE_CLIENT_DEFAULT, follow_redirects=None, timeout=None):
        """HTTP HEAD with proper auth sentinel handling."""
        return self._client.head(url, params=params, headers=headers, cookies=cookies,
                                 auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)

    def options(self, url, *, params=None, headers=None, cookies=None,
                auth=USE_CLIENT_DEFAULT, follow_redirects=None, timeout=None):
        """HTTP OPTIONS with proper auth sentinel handling."""
        return self._client.options(url, params=params, headers=headers, cookies=cookies,
                                    auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)

    def request(self, method, url, *, content=None, data=None, files=None, json=None,
                params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
                follow_redirects=None, timeout=None):
        """HTTP request with proper auth sentinel handling."""
        return self._client.request(method, url, content=content, data=data, files=files,
                                    json=json, params=params, headers=headers, cookies=cookies,
                                    auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)


# Import _utils module for utility functions
from . import _utils

__all__ = [
    # Version info
    "__description__",
    "__title__",
    "__version__",
    # Core types
    "AsyncByteStream",
    "AsyncClient",
    "AsyncHTTPTransport",
    "AsyncMockTransport",
    "Auth",
    "BasicAuth",
    "Client",
    "CloseError",
    "codes",
    "ConnectError",
    "ConnectTimeout",
    "CookieConflict",
    "Cookies",
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
    "Request",
    "request",
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
    "WriteError",
    "WriteTimeout",
    "WSGITransport",
]
