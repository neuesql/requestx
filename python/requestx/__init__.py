# RequestX - High-performance Python HTTP client
# API-compatible with httpx, powered by Rust's reqwest via PyO3

import contextlib as _contextlib
import http.cookiejar as _http_cookiejar  # Import for side effect (httpx compatibility)
import logging as _logging

# Set up the httpx logger (for compatibility)
_logger = _logging.getLogger("httpx")

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
    MockTransport as _RustMockTransport,
    AsyncMockTransport as _RustAsyncMockTransport,
    HTTPTransport,
    AsyncHTTPTransport,
    WSGITransport,
    # Top-level functions (import with underscore to wrap for exception conversion)
    get as _get,
    post as _post,
    put as _put,
    patch as _patch,
    delete as _delete,
    head as _head,
    options as _options,
    request as _request,
    stream as _stream,
    # Exceptions (import with underscore prefix to wrap with request attribute support)
    HTTPStatusError as _HTTPStatusError,
    RequestError as _RequestError,
    TransportError as _TransportError,
    TimeoutException as _TimeoutException,
    ConnectTimeout as _ConnectTimeout,
    ReadTimeout as _ReadTimeout,
    WriteTimeout as _WriteTimeout,
    PoolTimeout as _PoolTimeout,
    NetworkError as _NetworkError,
    ConnectError as _ConnectError,
    ReadError as _ReadError,
    WriteError as _WriteError,
    CloseError as _CloseError,
    ProxyError as _ProxyError,
    ProtocolError as _ProtocolError,
    LocalProtocolError as _LocalProtocolError,
    RemoteProtocolError as _RemoteProtocolError,
    UnsupportedProtocol as _UnsupportedProtocol,
    DecodingError as _DecodingError,
    TooManyRedirects as _TooManyRedirects,
    StreamError as _StreamError,
    StreamConsumed as _StreamConsumed,
    StreamClosed as _StreamClosed,
    ResponseNotRead as _ResponseNotRead,
    RequestNotRead as _RequestNotRead,
    InvalidURL,
    HTTPError,
    CookieConflict,
    # Status codes (import as _codes to wrap)
    codes as _codes,
)


# ============================================================================
# URL wrapper for explicit port preservation
# ============================================================================

class _ExplicitPortURL:
    """URL wrapper that preserves explicit port in string representation.

    The standard URL class normalizes away default ports (e.g., :443 for https).
    This wrapper preserves the explicit port string for cases like malformed
    redirect URLs that specify the default port explicitly.
    """

    def __init__(self, url_str):
        self._url_str = url_str
        self._url = URL(url_str)  # Underlying URL for property access

    def __str__(self):
        return self._url_str

    def __repr__(self):
        return f"URL('{self._url_str}')"

    def __eq__(self, other):
        if isinstance(other, str):
            return self._url_str == other
        if isinstance(other, (_ExplicitPortURL, URL)):
            return str(self) == str(other)
        return False

    def __hash__(self):
        return hash(self._url_str)

    @property
    def scheme(self):
        return self._url.scheme

    @property
    def host(self):
        return self._url.host

    @property
    def port(self):
        return self._url.port

    @property
    def path(self):
        return self._url.path

    @property
    def query(self):
        return self._url.query

    @property
    def fragment(self):
        return self._url.fragment

    def join(self, url):
        return self._url.join(url)


# ============================================================================
# Exception Classes with request attribute support
# ============================================================================

class RequestError(Exception):
    """Base class for request errors."""
    def __init__(self, message="", *, request=None):
        super().__init__(message)
        self._request = request

    @property
    def request(self):
        if self._request is None:
            raise RuntimeError(
                "The request instance has not been set on this exception."
            )
        return self._request


class TransportError(RequestError):
    """Base class for transport errors."""
    pass


# Exception classes with request attribute support
# These wrap the Rust exceptions to add the request property


class TimeoutException(TransportError):
    """Base class for timeout exceptions."""
    pass


class ConnectTimeout(TimeoutException):
    """Timeout during connection."""
    pass


class ReadTimeout(TimeoutException):
    """Timeout while reading response."""
    pass


class WriteTimeout(TimeoutException):
    """Timeout while writing request."""
    pass


class PoolTimeout(TimeoutException):
    """Timeout waiting for connection pool."""
    pass


class NetworkError(TransportError):
    """Network-related errors."""
    pass


class ConnectError(NetworkError):
    """Error connecting to host."""
    pass


class ReadError(NetworkError):
    """Error reading from connection."""
    pass


class WriteError(NetworkError):
    """Error writing to connection."""
    pass


class CloseError(NetworkError):
    """Error closing connection."""
    pass


class ProxyError(TransportError):
    """Proxy-related errors."""
    pass


class ProtocolError(TransportError):
    """Protocol-related errors."""
    pass


class LocalProtocolError(ProtocolError):
    """Local protocol error."""
    pass


class RemoteProtocolError(ProtocolError):
    """Remote protocol error."""
    pass


class UnsupportedProtocol(TransportError):
    """Unsupported protocol error."""
    pass


class DecodingError(RequestError):
    """Decoding error."""
    pass


class TooManyRedirects(RequestError):
    """Too many redirects error."""
    pass


class StreamError(RequestError):
    """Stream error."""
    pass


class StreamConsumed(StreamError):
    """Stream consumed error."""
    pass


class StreamClosed(StreamError):
    """Stream closed error."""
    pass


class ResponseNotRead(StreamError):
    """Response not read error."""
    pass


class RequestNotRead(StreamError):
    """Request not read error."""
    pass


def _convert_exception(exc):
    """Convert a Rust exception to the appropriate Python exception."""
    msg = str(exc)
    if isinstance(exc, _ConnectTimeout):
        return ConnectTimeout(msg)
    elif isinstance(exc, _ReadTimeout):
        return ReadTimeout(msg)
    elif isinstance(exc, _WriteTimeout):
        return WriteTimeout(msg)
    elif isinstance(exc, _PoolTimeout):
        return PoolTimeout(msg)
    elif isinstance(exc, _TimeoutException):
        return TimeoutException(msg)
    elif isinstance(exc, _ConnectError):
        return ConnectError(msg)
    elif isinstance(exc, _ReadError):
        return ReadError(msg)
    elif isinstance(exc, _WriteError):
        return WriteError(msg)
    elif isinstance(exc, _CloseError):
        return CloseError(msg)
    elif isinstance(exc, _NetworkError):
        return NetworkError(msg)
    elif isinstance(exc, _ProxyError):
        return ProxyError(msg)
    elif isinstance(exc, _LocalProtocolError):
        return LocalProtocolError(msg)
    elif isinstance(exc, _RemoteProtocolError):
        return RemoteProtocolError(msg)
    elif isinstance(exc, _ProtocolError):
        return ProtocolError(msg)
    elif isinstance(exc, _UnsupportedProtocol):
        return UnsupportedProtocol(msg)
    elif isinstance(exc, _DecodingError):
        return DecodingError(msg)
    elif isinstance(exc, _TooManyRedirects):
        return TooManyRedirects(msg)
    elif isinstance(exc, _StreamConsumed):
        return StreamConsumed(msg)
    elif isinstance(exc, _StreamClosed):
        return StreamClosed(msg)
    elif isinstance(exc, _ResponseNotRead):
        return ResponseNotRead(msg)
    elif isinstance(exc, _RequestNotRead):
        return RequestNotRead(msg)
    elif isinstance(exc, _StreamError):
        return StreamError(msg)
    elif isinstance(exc, _TransportError):
        return TransportError(msg)
    elif isinstance(exc, _RequestError):
        return RequestError(msg)
    else:
        return exc


# ============================================================================
# Top-level API functions with exception conversion
# ============================================================================


def _prepare_content(kwargs):
    """Prepare content argument, consuming iterators/generators to bytes."""
    import inspect
    import types
    content = kwargs.get('content')
    if content is not None:
        # Check if it's a generator or iterator (but not bytes, str, or file-like)
        if isinstance(content, types.GeneratorType):
            # Consume generator to bytes
            kwargs['content'] = b''.join(content)
        elif hasattr(content, '__iter__') and hasattr(content, '__next__'):
            # It's an iterator - consume it
            kwargs['content'] = b''.join(content)
        elif hasattr(content, '__iter__') and not isinstance(content, (bytes, str, list, tuple, dict)):
            # It's an iterable object (like SyncByteStream) - consume it
            try:
                kwargs['content'] = b''.join(content)
            except TypeError:
                pass  # Let Rust handle it if join fails
    return kwargs


def get(url, **kwargs):
    """Send a GET request."""
    try:
        return _get(url, **kwargs)
    except (_RequestError, _TransportError, _TimeoutException, _NetworkError,
            _ConnectError, _ReadError, _WriteError, _CloseError, _ProxyError,
            _ProtocolError, _UnsupportedProtocol, _DecodingError, _TooManyRedirects,
            _StreamError, _ConnectTimeout, _ReadTimeout, _WriteTimeout, _PoolTimeout) as e:
        raise _convert_exception(e) from None


def post(url, **kwargs):
    """Send a POST request."""
    try:
        kwargs = _prepare_content(kwargs)
        return _post(url, **kwargs)
    except (_RequestError, _TransportError, _TimeoutException, _NetworkError,
            _ConnectError, _ReadError, _WriteError, _CloseError, _ProxyError,
            _ProtocolError, _UnsupportedProtocol, _DecodingError, _TooManyRedirects,
            _StreamError, _ConnectTimeout, _ReadTimeout, _WriteTimeout, _PoolTimeout) as e:
        raise _convert_exception(e) from None


def put(url, **kwargs):
    """Send a PUT request."""
    try:
        kwargs = _prepare_content(kwargs)
        return _put(url, **kwargs)
    except (_RequestError, _TransportError, _TimeoutException, _NetworkError,
            _ConnectError, _ReadError, _WriteError, _CloseError, _ProxyError,
            _ProtocolError, _UnsupportedProtocol, _DecodingError, _TooManyRedirects,
            _StreamError, _ConnectTimeout, _ReadTimeout, _WriteTimeout, _PoolTimeout) as e:
        raise _convert_exception(e) from None


def patch(url, **kwargs):
    """Send a PATCH request."""
    try:
        kwargs = _prepare_content(kwargs)
        return _patch(url, **kwargs)
    except (_RequestError, _TransportError, _TimeoutException, _NetworkError,
            _ConnectError, _ReadError, _WriteError, _CloseError, _ProxyError,
            _ProtocolError, _UnsupportedProtocol, _DecodingError, _TooManyRedirects,
            _StreamError, _ConnectTimeout, _ReadTimeout, _WriteTimeout, _PoolTimeout) as e:
        raise _convert_exception(e) from None


def delete(url, **kwargs):
    """Send a DELETE request."""
    try:
        return _delete(url, **kwargs)
    except (_RequestError, _TransportError, _TimeoutException, _NetworkError,
            _ConnectError, _ReadError, _WriteError, _CloseError, _ProxyError,
            _ProtocolError, _UnsupportedProtocol, _DecodingError, _TooManyRedirects,
            _StreamError, _ConnectTimeout, _ReadTimeout, _WriteTimeout, _PoolTimeout) as e:
        raise _convert_exception(e) from None


def head(url, **kwargs):
    """Send a HEAD request."""
    try:
        return _head(url, **kwargs)
    except (_RequestError, _TransportError, _TimeoutException, _NetworkError,
            _ConnectError, _ReadError, _WriteError, _CloseError, _ProxyError,
            _ProtocolError, _UnsupportedProtocol, _DecodingError, _TooManyRedirects,
            _StreamError, _ConnectTimeout, _ReadTimeout, _WriteTimeout, _PoolTimeout) as e:
        raise _convert_exception(e) from None


def options(url, **kwargs):
    """Send an OPTIONS request."""
    try:
        return _options(url, **kwargs)
    except (_RequestError, _TransportError, _TimeoutException, _NetworkError,
            _ConnectError, _ReadError, _WriteError, _CloseError, _ProxyError,
            _ProtocolError, _UnsupportedProtocol, _DecodingError, _TooManyRedirects,
            _StreamError, _ConnectTimeout, _ReadTimeout, _WriteTimeout, _PoolTimeout) as e:
        raise _convert_exception(e) from None


def request(method, url, **kwargs):
    """Send an HTTP request."""
    try:
        return _request(method, url, **kwargs)
    except (_RequestError, _TransportError, _TimeoutException, _NetworkError,
            _ConnectError, _ReadError, _WriteError, _CloseError, _ProxyError,
            _ProtocolError, _UnsupportedProtocol, _DecodingError, _TooManyRedirects,
            _StreamError, _ConnectTimeout, _ReadTimeout, _WriteTimeout, _PoolTimeout) as e:
        raise _convert_exception(e) from None


def stream(method, url, **kwargs):
    """Stream an HTTP request."""
    try:
        return _stream(method, url, **kwargs)
    except (_RequestError, _TransportError, _TimeoutException, _NetworkError,
            _ConnectError, _ReadError, _WriteError, _CloseError, _ProxyError,
            _ProtocolError, _UnsupportedProtocol, _DecodingError, _TooManyRedirects,
            _StreamError, _ConnectTimeout, _ReadTimeout, _WriteTimeout, _PoolTimeout) as e:
        raise _convert_exception(e) from None


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


class MockTransport(AsyncBaseTransport):
    """Mock transport for testing - calls a handler function to generate responses.

    This is a Python wrapper around the Rust MockTransport that properly preserves
    Response objects with streams.
    """

    def __init__(self, handler=None):
        self._handler = handler
        self._rust_transport = _RustMockTransport(handler)

    @property
    def handler(self):
        """Public access to the handler function."""
        return self._handler

    def handle_request(self, request):
        """Handle a sync request by calling the handler."""
        if self._handler is None:
            return Response(200)
        result = self._handler(request)
        if isinstance(result, Response):
            return result
        elif isinstance(result, _Response):
            return Response(result)
        return Response(result)

    async def handle_async_request(self, request):
        """Handle an async request by calling the handler."""
        import inspect
        if self._handler is None:
            return Response(200)
        result = self._handler(request)
        if inspect.iscoroutine(result):
            result = await result
        if isinstance(result, Response):
            return result
        elif isinstance(result, _Response):
            return Response(result)
        return Response(result)

    def __repr__(self):
        return "<MockTransport>"


# AsyncMockTransport is an alias for MockTransport (it handles both sync and async)
AsyncMockTransport = MockTransport


class ASGITransport(AsyncBaseTransport):
    """ASGI transport for testing ASGI applications.

    This transport allows you to test ASGI applications directly without
    making actual network requests.

    Example:
        async def app(scope, receive, send):
            await send({
                "type": "http.response.start",
                "status": 200,
                "headers": [[b"content-type", b"text/plain"]],
            })
            await send({
                "type": "http.response.body",
                "body": b"Hello, World!",
            })

        transport = ASGITransport(app=app)
        async with AsyncClient(transport=transport) as client:
            response = await client.get("http://testserver/")
    """

    def __init__(
        self,
        app,
        raise_app_exceptions: bool = True,
        root_path: str = "",
        client: tuple = ("127.0.0.1", 123),
    ):
        self.app = app
        self.raise_app_exceptions = raise_app_exceptions
        self.root_path = root_path
        self.client = client

    async def handle_async_request(self, request):
        """Handle an async request by calling the ASGI app."""
        import asyncio

        # Get request details
        url = request.url
        method = request.method
        headers = request.headers

        # Build ASGI scope
        scheme = url.scheme if hasattr(url, 'scheme') else 'http'
        host = url.host if hasattr(url, 'host') else 'localhost'
        port = url.port
        path = url.path if hasattr(url, 'path') else '/'
        query_string = url.query if hasattr(url, 'query') else b''

        # Handle query as bytes
        if isinstance(query_string, str):
            query_string = query_string.encode('utf-8')

        # Get raw_path (path without query string, percent-encoded)
        raw_path = path.encode('utf-8') if isinstance(path, str) else path

        # Build headers list for ASGI (Host header should be first)
        asgi_headers = []
        host_header = None
        for key, value in headers.items():
            key_bytes = key.encode('latin-1') if isinstance(key, str) else key
            value_bytes = value.encode('latin-1') if isinstance(value, str) else value
            if key.lower() == 'host':
                host_header = [key_bytes, value_bytes]
            else:
                asgi_headers.append([key_bytes, value_bytes])
        # Insert Host header at the beginning
        if host_header:
            asgi_headers.insert(0, host_header)

        # Determine server tuple
        if port is None:
            port = 443 if scheme == 'https' else 80

        scope = {
            "type": "http",
            "asgi": {"version": "3.0"},
            "http_version": "1.1",
            "method": method,
            "headers": asgi_headers,
            "path": path,
            "raw_path": raw_path,
            "query_string": query_string,
            "root_path": self.root_path,
            "scheme": scheme,
            "server": (host, port),
            "client": self.client,
            "extensions": {},
        }

        # Get request body
        body = request.content if hasattr(request, 'content') else b''
        if body is None:
            body = b''

        # State for receive/send
        body_sent = False
        disconnect_sent = False
        response_started = False
        response_complete = False
        status_code = None
        response_headers = []
        body_parts = []
        exc_to_raise = None

        async def receive():
            nonlocal body_sent, disconnect_sent

            if not body_sent:
                body_sent = True
                return {
                    "type": "http.request",
                    "body": body,
                    "more_body": False,
                }
            else:
                # After body is sent and response is complete, send disconnect
                disconnect_sent = True
                return {"type": "http.disconnect"}

        async def send(message):
            nonlocal response_started, response_complete, status_code, response_headers, body_parts

            if message["type"] == "http.response.start":
                response_started = True
                status_code = message["status"]
                # Convert headers
                for h in message.get("headers", []):
                    if isinstance(h, (list, tuple)) and len(h) == 2:
                        key = h[0].decode('latin-1') if isinstance(h[0], bytes) else h[0]
                        value = h[1].decode('latin-1') if isinstance(h[1], bytes) else str(h[1])
                        response_headers.append((key, value))

            elif message["type"] == "http.response.body":
                body_chunk = message.get("body", b"")
                if body_chunk:
                    body_parts.append(body_chunk)
                if not message.get("more_body", False):
                    response_complete = True

        # Run the ASGI app
        try:
            await self.app(scope, receive, send)
        except Exception as exc:
            if self.raise_app_exceptions:
                raise
            exc_to_raise = exc
            # Return 500 error if app raises
            if not response_started:
                status_code = 500
                response_headers = [(b"content-type", b"text/plain")]
                body_parts = [b"Internal Server Error"]

        # If no response was started, return 500
        if status_code is None:
            status_code = 500
            response_headers = []
            body_parts = [b"Internal Server Error"]

        # Build response
        content = b"".join(body_parts)
        response = Response(
            status_code,
            headers=response_headers,
            content=content,
        )

        # Set request on response
        response._request = request
        response._url = request.url if hasattr(request, 'url') else None

        return response

    def __repr__(self):
        return f"<ASGITransport app={self.app!r}>"


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


class _GeneratorByteStream(SyncByteStream):
    """SyncByteStream wrapper for generators/iterators that tracks consumption.

    This allows generators to be passed as content while tracking whether
    the stream has been consumed (for detecting StreamConsumed on redirects).
    """

    def __init__(self, generator, owner=None):
        # Don't call super().__init__ since we don't have bytes data
        self._generator = generator
        self._owner = owner  # Reference to _WrappedRequest for tracking
        self._consumed = False
        self._started = False
        self._chunks = []  # Store chunks for potential re-read

    def __iter__(self):
        if self._consumed:
            raise StreamConsumed()
        return self

    def __next__(self):
        if self._consumed:
            raise StopIteration
        self._started = True
        try:
            chunk = next(self._generator)
            self._chunks.append(chunk)
            return chunk
        except StopIteration:
            self._consumed = True
            if self._owner is not None:
                self._owner._stream_consumed = True
            raise

    def read(self):
        """Read all bytes."""
        if self._consumed:
            raise StreamConsumed()
        # Consume remaining generator
        for chunk in self._generator:
            self._chunks.append(chunk)
        self._consumed = True
        if self._owner is not None:
            self._owner._stream_consumed = True
        return b''.join(self._chunks)

    def close(self):
        """Close the stream."""
        pass

    def __repr__(self):
        return "<GeneratorByteStream>"


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


class _SyncIteratorStream:
    """Sync-only stream wrapper for iterators."""

    def __init__(self, iterator, owner=None):
        self._iterator = iterator
        self._owner = owner
        self._consumed = False
        self._started = False

    def __iter__(self):
        # Check if owner's stream was already consumed
        if self._owner is not None and getattr(self._owner, '_py_stream_consumed', False):
            raise StreamConsumed()
        if self._consumed:
            raise StreamConsumed()
        self._started = True
        return self

    def __next__(self):
        if self._consumed:
            raise StopIteration
        try:
            return next(self._iterator)
        except StopIteration:
            self._consumed = True
            if self._owner is not None:
                object.__setattr__(self._owner, '_py_stream_consumed', True)
            raise

    def read(self):
        """Read all bytes."""
        if self._owner is not None and getattr(self._owner, '_py_stream_consumed', False):
            raise StreamConsumed()
        if self._consumed:
            raise StreamConsumed()
        result = b"".join(self)
        return result

    def close(self):
        pass

    def __repr__(self):
        return "<SyncIteratorStream>"


class _AsyncIteratorStream:
    """Async-only stream wrapper for async iterators and async file-like objects."""

    def __init__(self, iterator, owner=None):
        self._iterator = iterator
        self._owner = owner
        self._consumed = False
        # Check if this is an async file-like object (has aread but no __anext__)
        self._is_file_like = hasattr(iterator, 'aread') and not hasattr(iterator, '__anext__')
        # For file-like objects, we need to track if we got the aiter
        self._aiter = None

    def __aiter__(self):
        # Check if owner's stream was already consumed
        if self._owner is not None and getattr(self._owner, '_py_stream_consumed', False):
            raise StreamConsumed()
        if self._consumed:
            raise StreamConsumed()
        return self

    async def __anext__(self):
        if self._consumed:
            raise StopAsyncIteration
        try:
            if self._is_file_like:
                # For async file-like objects, use __aiter__ if available
                if self._aiter is None:
                    if hasattr(self._iterator, '__aiter__'):
                        self._aiter = self._iterator.__aiter__()
                    else:
                        # Fall back to reading all at once
                        content = await self._iterator.aread(65536)
                        if not content:
                            self._consumed = True
                            if self._owner is not None:
                                object.__setattr__(self._owner, '_py_stream_consumed', True)
                            raise StopAsyncIteration
                        return content
                return await self._aiter.__anext__()
            else:
                return await self._iterator.__anext__()
        except StopAsyncIteration:
            self._consumed = True
            if self._owner is not None:
                object.__setattr__(self._owner, '_py_stream_consumed', True)
            raise

    async def aread(self):
        """Read all bytes asynchronously."""
        if self._owner is not None and getattr(self._owner, '_py_stream_consumed', False):
            raise StreamConsumed()
        if self._consumed:
            raise StreamConsumed()
        result = b"".join([part async for part in self])
        return result

    async def aclose(self):
        pass

    def __repr__(self):
        return "<AsyncIteratorStream>"


class _DualIteratorStream:
    """Dual-mode stream wrapper for bytes content."""

    def __init__(self, data, owner=None):
        self._data = data
        self._owner = owner
        self._sync_consumed = False
        self._async_consumed = False

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

    def read(self):
        """Read all bytes."""
        if isinstance(self._data, bytes):
            return self._data
        return b""

    async def aread(self):
        """Read all bytes asynchronously."""
        if isinstance(self._data, bytes):
            return self._data
        return b""

    def close(self):
        pass

    async def aclose(self):
        pass

    def __repr__(self):
        return "<DualIteratorStream>"


class _ResponseSyncIteratorStream:
    """Sync-only stream wrapper for Response iterators that tracks consumption."""

    def __init__(self, iterator, owner):
        # Handle iterables that aren't iterators
        if hasattr(iterator, '__iter__') and not hasattr(iterator, '__next__'):
            self._iterator = iter(iterator)
        else:
            self._iterator = iterator
        self._owner = owner
        self._consumed = False

    def __iter__(self):
        if self._consumed or self._owner._stream_consumed:
            raise StreamConsumed()
        return self

    def __next__(self):
        if self._consumed:
            raise StopIteration
        try:
            return next(self._iterator)
        except StopIteration:
            self._consumed = True
            self._owner._stream_consumed = True
            raise

    def read(self):
        """Read all bytes."""
        if self._consumed or self._owner._stream_consumed:
            raise StreamConsumed()
        result = b"".join(self)
        return result

    def close(self):
        pass

    def __repr__(self):
        return "<ResponseSyncIteratorStream>"


class _ResponseAsyncIteratorStream:
    """Async-only stream wrapper for Response async iterators that tracks consumption."""

    def __init__(self, iterator, owner):
        self._iterator = iterator
        self._owner = owner
        self._consumed = False

    def __aiter__(self):
        if self._consumed or self._owner._stream_consumed:
            raise StreamConsumed()
        return self

    async def __anext__(self):
        if self._consumed:
            raise StopAsyncIteration
        try:
            return await self._iterator.__anext__()
        except StopAsyncIteration:
            self._consumed = True
            self._owner._stream_consumed = True
            raise

    async def aread(self):
        """Read all bytes asynchronously."""
        if self._consumed or self._owner._stream_consumed:
            raise StreamConsumed()
        result = b"".join([part async for part in self])
        return result

    async def aclose(self):
        pass

    def __repr__(self):
        return "<ResponseAsyncIteratorStream>"


# ============================================================================
# Request wrapper with proper stream property
# ============================================================================

class _WrappedRequest:
    """Wrapper for Rust Request that provides mutable headers."""

    def __init__(self, rust_request, async_stream=None, sync_stream=None, explicit_url=None):
        self._rust_request = rust_request
        self._headers_modified = False
        self._async_stream = async_stream  # Original async iterator if any
        self._sync_stream = sync_stream  # Sync iterator/generator if any
        self._stream_consumed = False
        self._explicit_url = explicit_url  # URL string that should not be normalized

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

    @property
    def stream(self):
        """Get the request body stream."""
        if self._async_stream is not None:
            # Return an AsyncByteStream wrapper that tracks consumption
            return _WrappedAsyncByteStream(self._async_stream, self)
        if self._sync_stream is not None:
            # Return the sync stream wrapper (already a SyncByteStream)
            return self._sync_stream
        return self._rust_request.stream


class _WrappedAsyncByteStream(AsyncByteStream):
    """Async byte stream wrapper that tracks consumption for retry detection."""

    def __init__(self, iterator, owner):
        self._iterator = iterator
        self._owner = owner
        self._consumed = False
        self._started = False

    def __aiter__(self):
        # Check if stream was already consumed (by a previous request)
        if self._owner._stream_consumed:
            raise StreamConsumed()
        return self

    async def __anext__(self):
        self._started = True
        try:
            chunk = await self._iterator.__anext__()
            return chunk
        except StopAsyncIteration:
            self._consumed = True
            self._owner._stream_consumed = True
            raise

    async def aread(self):
        """Read all bytes."""
        if self._owner._stream_consumed:
            raise StreamConsumed()
        chunks = []
        async for chunk in self:
            chunks.append(chunk)
        return b''.join(chunks)


class _WrappedRequestHeadersProxy:
    """Proxy for wrapped request headers that syncs changes back."""

    def __init__(self, wrapped_request):
        self._wrapped_request = wrapped_request
        # Get headers from rust request and convert to a new Headers object
        rust_headers = wrapped_request._rust_request.headers
        # Use _internal_items to preserve original header casing for .raw access
        self._headers = Headers(list(rust_headers._internal_items()))

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

    # Instance attribute to store async content - set lazily
    _py_async_content = None
    _py_was_async_read = False
    _py_stream_consumed = False

    @property
    def stream(self):
        """Get the request body as a ByteStream based on content type."""
        # Get stream mode from Rust
        mode = super().stream_mode

        # For streaming content (iterators/generators), return appropriate stream wrapper
        stream_ref = super().stream_ref
        if stream_ref is not None:
            if mode == "async":
                return _AsyncIteratorStream(stream_ref, self)
            elif mode == "sync":
                return _SyncIteratorStream(stream_ref, self)
            else:
                return _DualIteratorStream(stream_ref, self)

        # If async-read was done, return an async-compatible stream
        if getattr(self, '_py_was_async_read', False):
            content = getattr(self, '_py_async_content', None)
            if content is not None:
                return AsyncByteStream(content)
            try:
                return AsyncByteStream(super().content)
            except RequestNotRead:
                return AsyncByteStream(b"")

        # Return stream based on mode
        try:
            content = super().content
        except RequestNotRead:
            content = b""

        if mode == "async":
            return AsyncByteStream(content)
        elif mode == "sync":
            return SyncByteStream(content)
        else:
            return ByteStream(content)

    @property
    def content(self):
        """Get the request body content."""
        # If async content is available (from aread), return it
        content = getattr(self, '_py_async_content', None)
        if content is not None:
            return content
        return super().content

    async def aread(self):
        """Async read method that stores content after reading."""
        object.__setattr__(self, '_py_was_async_read', True)
        # Call parent aread which returns a coroutine
        result = await super().aread()
        # Store the result in Rust side for proper pickling
        if result:
            self._set_content_from_aread(result)
            object.__setattr__(self, '_py_async_content', result)
        return result

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

    def __init__(self, status_code_or_response=None, *, content=None, headers=None,
                 text=None, html=None, json=None, stream=None, request=None,
                 default_encoding=None, status_code=None):
        # Initialize attributes
        self._history = []
        self._url = None
        self._next_request = None
        self._request = None
        self._decoded_content = None
        self._default_encoding = default_encoding
        self._stream_content = None  # For storing async iterators
        self._sync_stream_content = None  # For storing sync iterators
        self._raw_content = None  # For caching consumed stream content
        self._raw_chunks = None  # For storing individual chunks for streaming
        self._num_bytes_downloaded = 0  # Track bytes downloaded during streaming
        self._stream_consumed = False  # Track if stream was consumed via iteration
        self._is_stream = False  # Track if this is a streaming response
        self._unpickled_stream_not_read = False  # Track if unpickled from unread stream
        self._text_accessed = False  # Track if .text was accessed
        self._stream_not_read = False  # Track if streaming response needs aread() before accessing content
        self._stream_object = None  # Reference to stream object for aclose()

        # Handle status_code as keyword argument
        if status_code is not None and status_code_or_response is None:
            status_code_or_response = status_code

        # Unwrap _WrappedRequest to get the underlying Rust request
        rust_request = request
        if request is not None and hasattr(request, '_rust_request'):
            rust_request = request._rust_request
            # Store the wrapped request for later access
            self._request = request

        # If passed a Rust _Response, wrap it
        if isinstance(status_code_or_response, _Response):
            self._response = status_code_or_response
        else:
            # Handle stream parameter (AsyncByteStream or similar)
            # If stream is provided, it takes precedence over content
            if stream is not None and content is None:
                # Check if stream is an async iterator
                if hasattr(stream, '__aiter__'):
                    self._stream_content = stream
                    self._is_stream = True
                    self._stream_object = stream  # Keep reference for aclose()
                    self._response = _Response(
                        status_code_or_response,
                        content=b'',
                        headers=headers,
                        request=rust_request,
                    )
                    return
                elif hasattr(stream, '__iter__'):
                    self._sync_stream_content = stream
                    self._is_stream = True
                    self._stream_object = stream  # Keep reference for close()
                    self._response = _Response(
                        status_code_or_response,
                        content=b'',
                        headers=headers,
                        request=rust_request,
                    )
                    return

            # Check if content is an async iterator or sync iterator
            is_async_iter = hasattr(content, '__aiter__') and hasattr(content, '__anext__')
            # Check for sync iterator/iterable (has __iter__ but not a built-in type)
            # This handles both generators (__iter__ + __next__) and iterables (just __iter__)
            is_sync_iter = (
                hasattr(content, '__iter__') and
                not isinstance(content, (bytes, str, list, dict, type(None))) and
                not hasattr(content, '__aiter__')  # Not an async iterable
            )

            if is_async_iter:
                # Store async iterator for later consumption
                self._stream_content = content
                self._is_stream = True
                # Check if Content-Length was provided
                has_content_length = False
                if headers is not None:
                    if isinstance(headers, dict):
                        has_content_length = any(k.lower() == 'content-length' for k in headers.keys())
                    elif isinstance(headers, list):
                        has_content_length = any(k.lower() == 'content-length' for k, v in headers)
                    else:
                        has_content_length = any(k.lower() == 'content-length' for k, v in headers.items())
                # Only add Transfer-Encoding: chunked if Content-Length is not provided
                if has_content_length:
                    stream_headers = headers
                elif headers is None:
                    stream_headers = [("transfer-encoding", "chunked")]
                elif isinstance(headers, list):
                    stream_headers = list(headers) + [("transfer-encoding", "chunked")]
                elif isinstance(headers, dict):
                    stream_headers = list(headers.items()) + [("transfer-encoding", "chunked")]
                else:
                    stream_headers = list(headers.items()) + [("transfer-encoding", "chunked")]
                # Create response without content - will be filled in aread()
                self._response = _Response(
                    status_code_or_response,
                    content=b'',
                    headers=stream_headers,
                    text=text,
                    html=html,
                    json=json,
                    stream=stream,
                    request=rust_request,
                )
            elif is_sync_iter:
                # Store sync iterator for lazy consumption, like async iterators
                self._sync_stream_content = content
                self._is_stream = True
                # Check if Content-Length was provided
                has_content_length = False
                if headers is not None:
                    if isinstance(headers, dict):
                        has_content_length = any(k.lower() == 'content-length' for k in headers.keys())
                    elif isinstance(headers, list):
                        has_content_length = any(k.lower() == 'content-length' for k, v in headers)
                    else:
                        has_content_length = any(k.lower() == 'content-length' for k, v in headers.items())
                # Only add Transfer-Encoding: chunked if Content-Length is not provided
                if has_content_length:
                    stream_headers = headers
                elif headers is None:
                    stream_headers = [("transfer-encoding", "chunked")]
                elif isinstance(headers, list):
                    stream_headers = list(headers) + [("transfer-encoding", "chunked")]
                elif isinstance(headers, dict):
                    stream_headers = list(headers.items()) + [("transfer-encoding", "chunked")]
                else:
                    stream_headers = list(headers.items()) + [("transfer-encoding", "chunked")]
                self._response = _Response(
                    status_code_or_response,
                    content=b'',
                    headers=stream_headers,
                    text=text,
                    html=html,
                    json=json,
                    stream=stream,
                    request=rust_request,
                )
            elif isinstance(content, list):
                # Content is a list of bytes chunks
                consumed_content = b''.join(content)
                self._raw_content = consumed_content
                self._response = _Response(
                    status_code_or_response,
                    content=consumed_content,
                    headers=headers,
                    text=text,
                    html=html,
                    json=json,
                    stream=stream,
                    request=rust_request,
                )
            else:
                # Regular content (bytes, str, or None)
                self._response = _Response(
                    status_code_or_response,
                    content=content,
                    headers=headers,
                    text=text,
                    html=html,
                    json=json,
                    stream=stream,
                    request=rust_request,
                )

        # Eagerly decode content if provided directly (not streaming)
        # This ensures DecodingError is raised during construction for invalid data
        if content is not None and not hasattr(content, '__aiter__') and not hasattr(content, '__next__'):
            if isinstance(content, (bytes, str, list)):
                # Trigger decompression to catch errors early
                _ = self.content

    def __getattr__(self, name):
        """Delegate attribute access to the underlying Rust response."""
        return getattr(self._response, name)

    @property
    def stream(self):
        """Get the response body as a stream based on content type."""
        # Check if this is a sync iterator stream
        if self._sync_stream_content is not None:
            return _ResponseSyncIteratorStream(self._sync_stream_content, self)
        # Check if this is an async iterator stream
        if self._stream_content is not None:
            return _ResponseAsyncIteratorStream(self._stream_content, self)
        # Check if stream was already consumed (but content is not available)
        # If content is available, we can still return a ByteStream
        if self._stream_consumed and self._raw_content is None and not self._response.content:
            raise StreamConsumed()
        # Regular content - return dual-mode stream
        content = self._raw_content if self._raw_content is not None else self._response.content
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
        # Return stored URL if set, otherwise from response
        if self._url is not None:
            return self._url
        return self._response.url

    @url.setter
    def url(self, value):
        self._url = value

    @property
    def content(self):
        # If this was unpickled from an unread async stream, raise ResponseNotRead
        if self._unpickled_stream_not_read:
            raise ResponseNotRead()
        # If this is a streaming response that hasn't been read via aread(), raise ResponseNotRead
        if self._stream_not_read:
            raise ResponseNotRead()
        if self._decoded_content is not None:
            return self._decoded_content

        # Use raw_content if we consumed a stream, otherwise use response content
        raw_content = self._raw_content if self._raw_content is not None else self._response.content
        if not raw_content:
            return raw_content

        # Check Content-Encoding header for decompression
        content_encoding = self.headers.get('content-encoding', '').lower()
        if not content_encoding or content_encoding == 'identity':
            return raw_content

        # Decode content based on encoding(s) - handle multiple encodings
        decompressed = raw_content
        encodings = [e.strip() for e in content_encoding.split(',')]

        # Process encodings in reverse order (last applied first)
        for encoding in reversed(encodings):
            if encoding == 'identity':
                continue
            decompressed = self._decompress(decompressed, encoding)

        self._decoded_content = decompressed
        return decompressed

    def _decompress(self, data, encoding):
        """Decompress data based on encoding."""
        import zlib

        if not data:
            return data

        encoding = encoding.lower().strip()

        if encoding == 'gzip':
            try:
                import gzip
                return gzip.decompress(data)
            except Exception as e:
                raise DecodingError(f"Failed to decode gzip content: {e}")

        elif encoding == 'deflate':
            # Deflate can be raw deflate or zlib-wrapped
            try:
                # Try raw deflate first
                return zlib.decompress(data, -zlib.MAX_WBITS)
            except zlib.error:
                try:
                    # Try zlib-wrapped deflate
                    return zlib.decompress(data)
                except zlib.error as e:
                    raise DecodingError(f"Failed to decode deflate content: {e}")

        elif encoding == 'br':
            try:
                import brotli
                return brotli.decompress(data)
            except Exception as e:
                raise DecodingError(f"Failed to decode brotli content: {e}")

        elif encoding == 'zstd':
            try:
                import zstandard as zstd
                # Use streaming decompression to handle multiple frames
                dctx = zstd.ZstdDecompressor()
                # Handle BytesIO or bytes
                if hasattr(data, 'read'):
                    reader = dctx.stream_reader(data)
                    result = reader.read()
                    reader.close()
                    return result
                else:
                    # For bytes, use decompress with allow multiple frames
                    import io
                    reader = dctx.stream_reader(io.BytesIO(data))
                    result = reader.read()
                    reader.close()
                    return result
            except Exception as e:
                raise DecodingError(f"Failed to decode zstd content: {e}")

        # Unknown encoding - return as-is
        return data

    @property
    def text(self):
        # Mark text as accessed (for encoding setter validation)
        self._text_accessed = True
        # If we have consumed raw content, decode it ourselves
        raw_content = self._raw_content if self._raw_content is not None else self._response.content
        if not raw_content:
            return ''
        encoding = self._get_encoding()
        return raw_content.decode(encoding, errors='replace')

    @property
    def encoding(self):
        """Get the encoding used for text decoding."""
        return self._get_encoding()

    @property
    def charset_encoding(self):
        """Get the charset from the Content-Type header, or None if not specified."""
        content_type = self.headers.get('content-type', '')
        # Parse charset from Content-Type header: text/plain; charset=utf-8
        for part in content_type.split(';'):
            part = part.strip()
            if part.lower().startswith('charset='):
                charset = part[8:].strip().strip('"').strip("'")
                return charset if charset else None
        return None

    @encoding.setter
    def encoding(self, value):
        """Set explicit encoding for text decoding."""
        # If text was already accessed, raise ValueError
        if getattr(self, '_text_accessed', False):
            raise ValueError(
                "The encoding cannot be set after .text has been accessed."
            )
        # Store explicit encoding in Python wrapper
        self._explicit_encoding = value
        # Clear any cached decoded content
        self._decoded_content = None

    def _get_encoding(self):
        """Get the encoding for text decoding."""
        import codecs
        # First check explicit encoding set via property
        if hasattr(self, '_explicit_encoding') and self._explicit_encoding is not None:
            return self._explicit_encoding
        # Check Content-Type header for charset
        content_type = self.headers.get('content-type', '')
        if 'charset=' in content_type:
            for part in content_type.split(';'):
                part = part.strip()
                if part.lower().startswith('charset='):
                    charset = part[8:].strip('"\'')
                    # Validate the encoding - if invalid, fall back to utf-8
                    try:
                        codecs.lookup(charset)
                        return charset
                    except LookupError:
                        # Invalid encoding, fall back to utf-8
                        return 'utf-8'
        # Use default_encoding if provided
        if self._default_encoding is not None:
            if callable(self._default_encoding):
                detected = self._default_encoding(self.content)
                if detected:
                    return detected
            else:
                return self._default_encoding
        return 'utf-8'

    @property
    def request(self):
        if self._request is not None:
            return self._request
        return self._response.request

    @request.setter
    def request(self, value):
        self._request = value
        self._response.request = value

    @property
    def next_request(self):
        """Return the next request for following redirects, or None if not a redirect."""
        return self._next_request

    @next_request.setter
    def next_request(self, value):
        self._next_request = value

    @property
    def elapsed(self):
        """Get elapsed time. Raises RuntimeError if response is not closed."""
        # If this is a streaming response that hasn't been closed, raise RuntimeError
        if self._is_stream and not self.is_closed:
            raise RuntimeError(
                ".elapsed accessed before the response was read or the stream was closed."
            )
        return self._response.elapsed

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
    def is_stream_consumed(self):
        """Return True if the stream has been consumed."""
        return self._stream_consumed

    @property
    def history(self):
        """List of responses in redirect/auth chain."""
        return self._history

    @property
    def num_bytes_downloaded(self):
        """Number of bytes downloaded so far."""
        # If we have a streaming counter, use it
        if self._num_bytes_downloaded > 0:
            return self._num_bytes_downloaded
        # Otherwise delegate to Rust response
        return self._response.num_bytes_downloaded

    def __repr__(self):
        return f"<Response [{self.status_code} {self.reason_phrase}]>"

    def __getstate__(self):
        """Pickle support - get state."""
        # Get request - try Python side first, then Rust side
        request = self._request
        if request is None:
            try:
                request = self._response.request
            except RuntimeError:
                request = None
        return {
            'status_code': self.status_code,
            'headers': list(self.headers.multi_items()),
            'content': self.content if not self._is_stream or self._raw_content else b'',
            'request': request,
            'url': self._url,
            'history': self._history,
            'default_encoding': self._default_encoding,
            'is_stream': self._is_stream,
            'stream_consumed': self._stream_consumed,
            'is_closed': self.is_closed,
            'has_stream_content': self._stream_content is not None,
        }

    def __setstate__(self, state):
        """Pickle support - restore state."""
        # Create a new Rust response with the saved state
        self._response = _Response(
            state['status_code'],
            content=state['content'],
            headers=state['headers'],
            request=state['request'],
        )
        self._request = state['request']
        self._url = state['url']
        self._history = state['history']
        self._default_encoding = state['default_encoding']
        self._is_stream = state['is_stream']
        # If we have content, mark stream as consumed (content is available)
        # If no content but it was a stream that wasn't read, keep original state
        if state['content']:
            self._stream_consumed = True
        else:
            self._stream_consumed = state['stream_consumed']
        self._stream_content = None  # Can't pickle stream content
        self._raw_content = state['content'] if state['content'] else None
        self._raw_chunks = None
        self._decoded_content = None
        self._next_request = None
        self._num_bytes_downloaded = 0
        self._sync_stream_content = None  # Initialize sync stream content
        self._text_accessed = False  # Text hasn't been accessed after unpickling
        self._stream_not_read = False  # Not a live stream after unpickling
        # Track if this was an async stream that wasn't read before pickling
        self._unpickled_stream_not_read = state.get('has_stream_content') and not state['content']
        # Mark Rust response as closed/consumed (since we have the content)
        self._response.read()

    def read(self):
        """Read and return the response body."""
        # Check if response is closed before we can read
        if self._is_stream and self.is_closed:
            raise StreamClosed()
        # Check if stream was already consumed via iteration
        if self._is_stream and self._stream_consumed:
            raise StreamConsumed()
        # If we have a pending sync stream, consume it
        if self._sync_stream_content is not None:
            chunks = list(self._sync_stream_content)
            consumed_content = b''.join(chunks)
            self._raw_content = consumed_content
            self._raw_chunks = chunks
            self._response._set_content(consumed_content)
            self._sync_stream_content = None
            self._stream_consumed = True
            return consumed_content
        # Call Rust read() to mark as closed
        self._response.read()
        return self.content

    async def aread(self):
        """Async read and return the response body."""
        # Check if stream was already consumed via iteration
        if self._is_stream and self._stream_consumed:
            raise StreamConsumed()
        # Check if this is an unpickled stream that wasn't read - stream is lost
        if self._unpickled_stream_not_read:
            raise StreamClosed()
        # Check if response is closed before we can read (only for true async streams)
        if self._stream_content is not None and self.is_closed:
            raise StreamClosed()
        # Clear the stream_not_read flag since we're reading now
        self._stream_not_read = False
        # If we have a pending async stream, consume it
        if self._stream_content is not None:
            chunks = []
            async for chunk in self._stream_content:
                chunks.append(chunk)
            self._raw_content = b''.join(chunks)
            self._stream_content = None  # Mark as consumed
            self._stream_consumed = True  # Mark stream as consumed
            # Clear decoded cache to force re-decode with new content
            self._decoded_content = None
            # Set content on Rust side to mark as closed
            self._response._set_content(self._raw_content)
        else:
            # Call Rust aread() to mark as closed
            await self._response.aread()
            self._stream_consumed = True  # Mark stream as consumed
        return self.content

    def iter_bytes(self, chunk_size=None):
        """Iterate over the response body as bytes chunks."""
        # If we have a sync stream that hasn't been consumed, iterate over it
        if self._sync_stream_content is not None:
            chunks = []
            consumed_content = b''
            for chunk in self._sync_stream_content:
                chunks.append(chunk)
                consumed_content += chunk
                self._num_bytes_downloaded += len(chunk)
                if chunk_size is None:
                    if chunk:  # Skip empty chunks
                        yield chunk
                else:
                    # Buffer chunks and yield at chunk_size boundaries
                    pass  # Will handle below
            # Store for later use (don't close the response yet)
            self._raw_content = consumed_content
            self._raw_chunks = chunks
            self._response._set_content_only(consumed_content)
            self._sync_stream_content = None
            self._stream_consumed = True
            # If chunk_size was specified, re-yield from stored content
            if chunk_size is not None:
                for i in range(0, len(consumed_content), chunk_size):
                    yield consumed_content[i:i + chunk_size]
            return
        # Mark stream as consumed after iteration
        self._stream_consumed = True
        # If we have individual chunks, yield them
        if self._raw_chunks is not None and chunk_size is None:
            for chunk in self._raw_chunks:
                if chunk:  # Skip empty chunks
                    yield chunk
        else:
            content = self.content
            if chunk_size is None:
                if content:
                    yield content
            else:
                for i in range(0, len(content), chunk_size):
                    yield content[i:i + chunk_size]

    def iter_text(self, chunk_size=None):
        """Iterate over the response body as text chunks."""
        # Get encoding from content-type or default to utf-8
        encoding = self._get_encoding()
        for chunk in self.iter_bytes(chunk_size):
            if chunk:
                yield chunk.decode(encoding, errors='replace')

    async def aiter_text(self, chunk_size=None):
        """Async iterate over the response body as text chunks."""
        encoding = self._get_encoding()
        for chunk in self.iter_bytes(chunk_size):
            yield chunk.decode(encoding, errors='replace')

    def iter_lines(self):
        """Iterate over the response body as lines."""
        pending = ""
        for text in self.iter_text():
            lines = (pending + text).splitlines(keepends=True)
            pending = ""
            for line in lines:
                if line.endswith(('\r\n', '\r', '\n')):
                    yield line.rstrip('\r\n')
                else:
                    pending = line
        if pending:
            yield pending

    def iter_raw(self, chunk_size=None):
        """Iterate over the raw response body (uncompressed bytes)."""
        # If we have an async stream stored, raise RuntimeError
        if self._stream_content is not None:
            raise RuntimeError("Attempted to call a sync iterator method on an async stream.")
        # Use iter_bytes for raw iteration (no decompression in this implementation)
        return self.iter_bytes(chunk_size)

    async def aiter_raw(self, chunk_size=None):
        """Async iterate over the raw response body."""
        # Mark stream as consumed
        self._stream_consumed = True
        # If we have a sync stream (either unconsumed or consumed), raise RuntimeError
        if self._sync_stream_content is not None or self._raw_chunks is not None:
            raise RuntimeError("Attempted to call an async iterator method on a sync stream.")

        # If we have an async stream, iterate over it
        if self._stream_content is not None:
            all_content = b''
            buffer = b''
            async for chunk in self._stream_content:
                all_content += chunk
                if chunk_size is None:
                    self._num_bytes_downloaded += len(chunk)
                    yield chunk
                else:
                    buffer += chunk
                    while len(buffer) >= chunk_size:
                        yielded = buffer[:chunk_size]
                        self._num_bytes_downloaded += len(yielded)
                        yield yielded
                        buffer = buffer[chunk_size:]
            # Yield any remaining data (only when using chunk_size)
            if chunk_size is not None and buffer:
                self._num_bytes_downloaded += len(buffer)
                yield buffer
            # Mark stream as consumed and store content
            self._raw_content = all_content
            self._stream_content = None
        else:
            # No async stream, yield from content
            content = self.content
            if chunk_size is None:
                if content:
                    self._num_bytes_downloaded += len(content)
                    yield content
            else:
                for i in range(0, len(content), chunk_size):
                    chunk = content[i:i + chunk_size]
                    self._num_bytes_downloaded += len(chunk)
                    yield chunk

    async def aiter_bytes(self, chunk_size=None):
        """Async iterate over the response body as bytes chunks."""
        # If we have a sync stream (raw_chunks), raise RuntimeError
        if self._stream_content is None and self._raw_chunks is not None:
            raise RuntimeError("Attempted to call an async iterator method on a sync stream.")

        # Use aiter_raw for bytes iteration
        async for chunk in self.aiter_raw(chunk_size):
            yield chunk

    async def aiter_lines(self):
        """Async iterate over the response body as lines."""
        # If we have a sync stream (raw_chunks), raise RuntimeError
        if self._stream_content is None and self._raw_chunks is not None:
            raise RuntimeError("Attempted to call an async iterator method on a sync stream.")

        encoding = self._get_encoding()
        pending = ""
        async for chunk in self.aiter_bytes():
            text = chunk.decode(encoding, errors='replace')
            lines = (pending + text).splitlines(keepends=True)
            pending = ""
            for line in lines:
                if line.endswith(('\r\n', '\r', '\n')):
                    yield line.rstrip('\r\n')
                else:
                    pending = line
        if pending:
            yield pending

    def close(self):
        """Close the response."""
        # If we have an async stream, raise RuntimeError
        if self._stream_content is not None:
            raise RuntimeError("Attempted to call a sync method on an async stream.")
        self._response.close()

    async def aclose(self):
        """Async close the response."""
        # If we have a sync stream that hasn't been consumed, raise RuntimeError
        if self._sync_stream_content is not None:
            raise RuntimeError("Attempted to call an async method on a sync stream.")
        # Note: Nothing to close for async streams in Python
        self._response.close()

    def json(self, **kwargs):
        import json as json_module
        from ._utils import guess_json_utf

        # Get raw content bytes (use decoded content if available)
        content = self.content

        # Detect encoding from content
        encoding = guess_json_utf(content)

        if encoding is not None:
            # Decode with detected encoding
            text = content.decode(encoding)
        else:
            # Try UTF-8 first (most common), fall back to text property
            try:
                text = content.decode('utf-8')
            except UnicodeDecodeError:
                text = self.text

        # Strip BOM character if present (can appear after decoding UTF-16/UTF-32)
        if text.startswith('\ufeff'):
            text = text[1:]

        # Parse JSON
        return json_module.loads(text, **kwargs)

    def raise_for_status(self):
        """Raise HTTPStatusError for non-2xx status codes.

        Returns self for chaining on success.
        """
        # Check that request is set (accessing self.request will raise if not)
        _ = self.request

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
        # Cached challenge parameters for subsequent requests
        self._challenge = None  # Dict with realm, nonce, qop, opaque, algorithm

    def _get_client_nonce(self, nonce_count: int, nonce: bytes) -> bytes:
        """Generate a client nonce. Signature matches httpx for test mocking."""
        import os
        return os.urandom(16)

    def _build_auth_header(self, request, challenge):
        """Build the Authorization header from a challenge."""
        import hashlib

        realm = challenge.get("realm", "")
        nonce = challenge.get("nonce", "")
        qop = challenge.get("qop", "")
        opaque = challenge.get("opaque", "")
        algorithm = challenge.get("algorithm", "MD5").upper()

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

        # Increment nonce count
        self._nonce_count += 1
        nc = f"{self._nonce_count:08x}"

        # Get client nonce
        cnonce_bytes = self._get_client_nonce(self._nonce_count, nonce.encode())
        if isinstance(cnonce_bytes, bytes):
            # Always hex-encode the cnonce for proper header formatting (like httpx does)
            cnonce = cnonce_bytes[:8].hex()  # Use first 8 bytes as hex (16 chars)
        else:
            cnonce = str(cnonce_bytes)

        # Calculate A1
        a1 = f"{self.username}:{realm}:{self.password}"
        if algorithm.endswith("-SESS"):
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
        if qop:
            # Parse qop options
            qop_options = [q.strip() for q in qop.split(",")]
            if "auth" in qop_options:
                qop_value = "auth"
            elif "auth-int" in qop_options:
                raise NotImplementedError("Digest auth qop=auth-int is not implemented")
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

        return "Digest " + ", ".join(auth_parts)

    def sync_auth_flow(self, request):
        """Generator-based sync auth flow for Digest auth."""
        import re

        # If we have a cached challenge, use it to pre-authenticate
        if self._challenge is not None:
            auth_header_value = self._build_auth_header(request, self._challenge)
            request.headers["Authorization"] = auth_header_value
            response = yield request
            # If we get 401, challenge may have changed - fall through to parse new one
            if response.status_code != 401:
                return
        else:
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

        nonce = params.get("nonce", "")

        # Validate required fields
        if not nonce:
            raise ProtocolError("Malformed Digest auth header: missing required 'nonce' field")

        # Reset nonce count if we get a new challenge (different nonce)
        if self._challenge is None or self._challenge.get("nonce") != nonce:
            self._nonce_count = 0

        # Store challenge for subsequent requests
        self._challenge = {
            "realm": params.get("realm", ""),
            "nonce": nonce,
            "qop": params.get("qop", ""),
            "opaque": params.get("opaque", ""),
            "algorithm": params.get("algorithm", "MD5"),
        }

        # Copy cookies from response to request
        if hasattr(response, 'cookies') and response.cookies:
            cookie_header = "; ".join(f"{name}={value}" for name, value in response.cookies.items())
            if cookie_header:
                request.headers["Cookie"] = cookie_header

        # Build auth header with new challenge
        auth_header_value = self._build_auth_header(request, self._challenge)
        request.headers["Authorization"] = auth_header_value

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
        import netrc as netrc_module
        import os
        self._file = file
        # Parse the netrc file at construction time (like httpx does)
        if file is None:
            # Use default netrc file
            netrc_path = os.path.expanduser("~/.netrc")
            if os.path.exists(netrc_path):
                self._netrc = netrc_module.netrc(netrc_path)
            else:
                self._netrc = None
        else:
            self._netrc = netrc_module.netrc(file)

    def sync_auth_flow(self, request):
        """Generator-based sync auth flow for NetRC auth."""
        # Look up credentials for the request host
        if self._netrc is not None:
            url = request.url
            host = url.host if hasattr(url, 'host') else str(url).split('/')[2].split(':')[0].split('@')[-1]
            auth_info = self._netrc.authenticators(host)
            if auth_info is not None:
                username, _, password = auth_info
                import base64
                credentials = f"{username}:{password}"
                encoded = base64.b64encode(credentials.encode()).decode('ascii')
                request.headers["Authorization"] = f"Basic {encoded}"
        yield request

    async def async_auth_flow(self, request):
        """Generator-based async auth flow for NetRC auth."""
        # Look up credentials for the request host
        if self._netrc is not None:
            url = request.url
            host = url.host if hasattr(url, 'host') else str(url).split('/')[2].split(':')[0].split('@')[-1]
            auth_info = self._netrc.authenticators(host)
            if auth_info is not None:
                username, _, password = auth_info
                import base64
                credentials = f"{username}:{password}"
                encoded = base64.b64encode(credentials.encode()).decode('ascii')
                request.headers["Authorization"] = f"Basic {encoded}"
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
        # Call the function to modify the request
        self._func(request)
        yield request

    async def async_auth_flow(self, request):
        """Generator-based async auth flow."""
        # Call the function to modify the request
        import inspect
        result = self._func(request)
        # Handle case where function returns a coroutine
        if inspect.iscoroutine(result):
            await result
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

# Helper to normalize auth (convert tuple to BasicAuth, callable to FunctionAuth)
def _normalize_auth(auth):
    """Convert tuple auth to BasicAuth, callable to FunctionAuth, pass through others."""
    if isinstance(auth, tuple) and len(auth) == 2:
        return BasicAuth(auth[0], auth[1])
    # Wrap plain callables in FunctionAuth (but not Auth subclasses which have auth_flow)
    if callable(auth) and not hasattr(auth, 'sync_auth_flow') and not hasattr(auth, 'async_auth_flow') and not hasattr(auth, 'auth_flow'):
        return FunctionAuth(auth)
    return auth


def _extract_auth_from_url(url_str):
    """Extract BasicAuth from URL userinfo if present."""
    if '@' not in url_str:
        return None
    # Parse URL to extract userinfo
    from urllib.parse import urlparse, unquote
    parsed = urlparse(url_str)
    if parsed.username:
        username = unquote(parsed.username)
        password = unquote(parsed.password) if parsed.password else ""
        return BasicAuth(username, password)
    return None


# Wrap AsyncClient to support auth=None vs auth not specified
# We use a wrapper class that delegates to the Rust implementation
class AsyncClient:
    """Async HTTP client that wraps the Rust implementation with proper auth sentinel handling."""

    def __init__(self, *args, **kwargs):
        import os
        import asyncio as _asyncio_mod

        # Extract limits and timeout for pool semaphore before Rust consumes them
        _limits_arg = kwargs.get('limits', None)
        _timeout_arg = kwargs.get('timeout', None)

        _max_connections = None
        if _limits_arg is not None and hasattr(_limits_arg, 'max_connections'):
            _max_connections = _limits_arg.max_connections

        _pool_timeout = None
        if _timeout_arg is not None and hasattr(_timeout_arg, 'pool'):
            _pool_timeout = _timeout_arg.pool

        self._pool_semaphore = _asyncio_mod.Semaphore(_max_connections) if _max_connections is not None else None
        self._pool_timeout = _pool_timeout

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

        # Extract proxy and mounts from kwargs
        proxy = kwargs.pop('proxy', None)
        mounts = kwargs.pop('mounts', None)
        trust_env = kwargs.get('trust_env', True)

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
        custom_transport = kwargs.get('transport', None)
        if custom_transport is not None:
            self._default_transport = custom_transport
        elif proxy is not None:
            self._default_transport = AsyncHTTPTransport(proxy=proxy)
        else:
            # Check for proxy env vars if trust_env is True
            env_proxy = None
            if trust_env:
                env_proxy = self._get_proxy_from_env()
            if env_proxy:
                self._default_transport = AsyncHTTPTransport(proxy=env_proxy)
            else:
                self._default_transport = AsyncHTTPTransport()

        self._custom_transport = custom_transport  # Keep reference to user-provided transport

        # Extract and store follow_redirects from kwargs before passing to Rust
        self._follow_redirects = kwargs.pop('follow_redirects', False)

        # Always create Rust client with follow_redirects=False so Python handles redirects
        # This allows proper logging and history tracking
        kwargs['follow_redirects'] = False
        self._client = _AsyncClient(*args, **kwargs)
        self._is_closed = False

    def _get_proxy_from_env(self):
        """Get proxy URL from environment variables."""
        import os
        for var in ('ALL_PROXY', 'all_proxy', 'HTTPS_PROXY', 'https_proxy', 'HTTP_PROXY', 'http_proxy'):
            proxy = os.environ.get(var)
            if proxy:
                if '://' not in proxy:
                    proxy = 'http://' + proxy
                return proxy
        return None

    def _should_use_proxy(self, url):
        """Check if URL should use proxy based on NO_PROXY env var."""
        import os
        no_proxy = os.environ.get('NO_PROXY', os.environ.get('no_proxy', ''))

        if not no_proxy:
            return True

        if no_proxy == '*':
            return False

        if isinstance(url, str):
            url = URL(url)
        host = url.host

        for pattern in no_proxy.split(','):
            pattern = pattern.strip()
            if not pattern:
                continue

            if '://' in pattern:
                pattern_scheme, pattern_host = pattern.split('://', 1)
                if pattern_scheme != url.scheme:
                    continue
                pattern = pattern_host

            if host == pattern:
                return False

            if pattern.startswith('.'):
                if host.endswith(pattern):
                    return False
            elif host.endswith('.' + pattern):
                return False

        return True

    @property
    def _transport(self):
        """Get the default transport for this client."""
        return self._default_transport

    def _transport_for_url(self, url):
        """Get the transport to use for a given URL."""
        import os
        if isinstance(url, str):
            url = URL(url)

        url_scheme = url.scheme
        url_host = url.host or ''
        url_port = url.port

        best_match = None
        best_score = -1

        for pattern, transport in self._mounts.items():
            score = self._match_pattern(url_scheme, url_host, url_port, pattern)
            if score > best_score:
                best_score = score
                best_match = transport

        if best_match is not None:
            return best_match

        if getattr(self._client, 'trust_env', True):
            proxy_url = self._get_proxy_for_url(url)
            if proxy_url:
                if not self._should_use_proxy(url):
                    return self._default_transport
                return AsyncHTTPTransport(proxy=proxy_url)

        return self._default_transport

    def _get_proxy_for_url(self, url):
        """Get proxy URL from environment for a specific URL."""
        import os
        scheme = url.scheme if hasattr(url, 'scheme') else 'http'

        if scheme == 'https':
            proxy = os.environ.get('HTTPS_PROXY', os.environ.get('https_proxy'))
            if proxy:
                if '://' not in proxy:
                    proxy = 'http://' + proxy
                return proxy

        if scheme == 'http':
            proxy = os.environ.get('HTTP_PROXY', os.environ.get('http_proxy'))
            if proxy:
                if '://' not in proxy:
                    proxy = 'http://' + proxy
                return proxy

        proxy = os.environ.get('ALL_PROXY', os.environ.get('all_proxy'))
        if proxy:
            if '://' not in proxy:
                proxy = 'http://' + proxy
            return proxy

        return None

    def _match_pattern(self, url_scheme, url_host, url_port, pattern):
        """Match URL against a mount pattern. Returns score (higher is better match), or -1 if no match."""
        if '://' in pattern:
            pattern_scheme, pattern_rest = pattern.split('://', 1)
        else:
            return -1

        if pattern_scheme not in ('all', url_scheme):
            return -1

        score = 0 if pattern_scheme == 'all' else 1

        if not pattern_rest:
            return score

        if ':' in pattern_rest and not pattern_rest.startswith('['):
            pattern_host, pattern_port_str = pattern_rest.rsplit(':', 1)
            try:
                pattern_port = int(pattern_port_str)
            except ValueError:
                pattern_host = pattern_rest
                pattern_port = None
        else:
            pattern_host = pattern_rest
            pattern_port = None

        if pattern_host == '*':
            score += 2
        elif pattern_host.startswith('*.'):
            suffix = pattern_host[1:]
            if url_host.endswith(suffix) and url_host != suffix[1:]:
                score += 2
            else:
                return -1
        elif pattern_host.startswith('*'):
            suffix = pattern_host[1:]
            if url_host == suffix or url_host.endswith('.' + suffix):
                score += 2
            else:
                return -1
        else:
            if url_host.lower() != pattern_host.lower():
                return -1
            score += 2

        if pattern_port is not None:
            if url_port == pattern_port:
                score += 4

        return score

    async def _invoke_request_hooks(self, request):
        """Invoke all request event hooks (handles both sync and async hooks)."""
        import inspect
        hooks = self.event_hooks.get('request', [])
        for hook in hooks:
            result = hook(request)
            if inspect.iscoroutine(result):
                await result

    async def _invoke_response_hooks(self, response):
        """Invoke all response event hooks (handles both sync and async hooks)."""
        import inspect
        hooks = self.event_hooks.get('response', [])
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
        if self._custom_transport is not None and hasattr(self._custom_transport, '__aenter__'):
            await self._custom_transport.__aenter__()
        # Call __aenter__ on all mounted transports
        for transport in self._mounts.values():
            if hasattr(transport, '__aenter__'):
                await transport.__aenter__()
        await self._client.__aenter__()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        result = await self._client.__aexit__(exc_type, exc_val, exc_tb)
        # Call transport's __aexit__ if it exists
        if self._custom_transport is not None and hasattr(self._custom_transport, '__aexit__'):
            await self._custom_transport.__aexit__(exc_type, exc_val, exc_tb)
        # Call __aexit__ on all mounted transports
        for transport in self._mounts.values():
            if hasattr(transport, '__aexit__'):
                await transport.__aexit__(exc_type, exc_val, exc_tb)
        self._is_closed = True
        return result

    async def aclose(self):
        """Close the client."""
        if hasattr(self._client, 'aclose'):
            await self._client.aclose()
        if self._custom_transport is not None and hasattr(self._custom_transport, 'aclose'):
            await self._custom_transport.aclose()
        # Close all mounted transports
        for transport in self._mounts.values():
            if hasattr(transport, 'aclose'):
                await transport.aclose()
        self._is_closed = True

    @property
    def is_closed(self):
        """Return True if the client has been closed."""
        return getattr(self, '_is_closed', False)

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
                await _asyncio_mod.wait_for(self._pool_semaphore.acquire(), timeout=self._pool_timeout)
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
                stacklevel=4  # go up to user code
            )

    def _extract_cookies_from_response(self, response, request):
        """Extract Set-Cookie headers from response and add to client cookies."""
        # Get all Set-Cookie headers
        set_cookie_headers = []
        if hasattr(response, 'headers'):
            # Try multi_items to get all Set-Cookie headers
            if hasattr(response.headers, 'multi_items'):
                for key, value in response.headers.multi_items():
                    if key.lower() == 'set-cookie':
                        set_cookie_headers.append(value)
            elif hasattr(response.headers, 'get_list'):
                set_cookie_headers = response.headers.get_list('set-cookie')
            else:
                # Fallback: get single value
                cookie_header = response.headers.get('set-cookie')
                if cookie_header:
                    set_cookie_headers = [cookie_header]

        # Parse and add each cookie
        # Note: client.cookies returns a copy, so we need to get it, modify it, and set it back
        if set_cookie_headers:
            from email.utils import parsedate_to_datetime
            import datetime
            cookies = self.cookies
            for cookie_str in set_cookie_headers:
                # Parse Set-Cookie header: "name=value; attr1; attr2=val"
                parts = cookie_str.split(';')
                if parts:
                    # First part is name=value
                    name_value = parts[0].strip()
                    if '=' in name_value:
                        name, value = name_value.split('=', 1)
                        name = name.strip()
                        value = value.strip()

                        # Check for expires attribute to handle cookie deletion
                        is_expired = False
                        for part in parts[1:]:
                            part = part.strip()
                            if part.lower().startswith('expires='):
                                expires_str = part[8:].strip()
                                try:
                                    expires_dt = parsedate_to_datetime(expires_str)
                                    if expires_dt < datetime.datetime.now(datetime.timezone.utc):
                                        is_expired = True
                                except Exception:
                                    pass
                                break

                        if is_expired:
                            # Delete the cookie
                            cookies.delete(name)
                        else:
                            # Add to cookies
                            cookies.set(name, value)
            # Set cookies back to client
            self.cookies = cookies

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
        # Check for sync iterator/generator in content (AsyncClient can't handle these)
        import inspect
        content = kwargs.get('content')
        if content is not None:
            if inspect.isgenerator(content):
                raise RuntimeError("Attempted to send an sync request with an AsyncClient instance.")
            # Also check for sync iterator protocol (but not strings/bytes which have __iter__)
            if hasattr(content, '__next__') and hasattr(content, '__iter__') and not isinstance(content, (str, bytes, bytearray)):
                raise RuntimeError("Attempted to send an sync request with an AsyncClient instance.")
        # Validate URL before processing
        url_str = str(url)
        # Check for empty scheme (like '://example.org')
        if url_str.startswith('://'):
            raise UnsupportedProtocol("Request URL is missing an 'http://' or 'https://' protocol.")
        # Check for missing host (like 'http://' or 'http:///path')
        if url_str.startswith('http://') or url_str.startswith('https://'):
            # Extract the part after scheme
            after_scheme = url_str.split('://', 1)[1] if '://' in url_str else ''
            # Empty host or starts with / means no host
            if not after_scheme or after_scheme.startswith('/'):
                raise UnsupportedProtocol("Request URL is missing an 'http://' or 'https://' protocol.")
        # Handle URL merging with base_url
        merged_url = self._merge_url(url)
        # Filter to only parameters supported by Rust build_request
        supported_kwargs = {}
        if 'content' in kwargs and kwargs['content'] is not None:
            supported_kwargs['content'] = kwargs['content']
        if 'params' in kwargs and kwargs['params'] is not None:
            supported_kwargs['params'] = kwargs['params']
        if 'headers' in kwargs and kwargs['headers'] is not None:
            supported_kwargs['headers'] = kwargs['headers']
        # Handle data, files, json by converting to content
        if 'json' in kwargs and kwargs['json'] is not None:
            import json as json_module
            supported_kwargs['content'] = json_module.dumps(kwargs['json']).encode('utf-8')
            # Add content-type header for JSON
            if 'headers' not in supported_kwargs:
                supported_kwargs['headers'] = {}
            if isinstance(supported_kwargs.get('headers'), dict):
                supported_kwargs['headers'] = {**supported_kwargs['headers'], 'content-type': 'application/json'}
        if 'data' in kwargs and kwargs['data'] is not None:
            data = kwargs['data']
            if isinstance(data, dict):
                from urllib.parse import urlencode
                supported_kwargs['content'] = urlencode(data).encode('utf-8')
                if 'headers' not in supported_kwargs:
                    supported_kwargs['headers'] = {}
                if isinstance(supported_kwargs.get('headers'), dict):
                    supported_kwargs['headers'] = {**supported_kwargs['headers'], 'content-type': 'application/x-www-form-urlencoded'}
            elif isinstance(data, (bytes, str)):
                supported_kwargs['content'] = data if isinstance(data, bytes) else data.encode('utf-8')
        rust_request = self._client.build_request(method, merged_url, **supported_kwargs)
        # Create a wrapper that delegates to the Rust request but has our headers proxy
        return _WrappedRequest(rust_request)

    def _merge_url(self, url):
        """Merge a URL with the base_url.

        Unlike RFC 3986 URL resolution, this concatenates paths when the
        relative URL starts with '/'.
        """
        if isinstance(url, URL):
            url_str = str(url)
        else:
            url_str = str(url)

        # If URL is absolute (has scheme), return as-is
        if '://' in url_str:
            return url_str

        # Get base_url from client
        base_url = self.base_url
        if base_url is None:
            return url_str

        base_url_str = str(base_url)

        # If base_url ends with '/', remove it for concatenation
        if base_url_str.endswith('/'):
            base_url_str = base_url_str[:-1]

        # Handle relative URLs
        if url_str.startswith('/'):
            # URL like '/testing/123' - append to base path
            return base_url_str + url_str
        elif url_str.startswith('../'):
            # URL like '../testing/123' - handle relative path navigation
            # Parse base URL to get components
            base = URL(base_url_str)
            base_path = base.path or ''
            # Remove trailing component from base path
            if base_path.endswith('/'):
                base_path = base_path[:-1]
            path_parts = base_path.split('/')
            # Process ../ in relative URL
            rel_parts = url_str.split('/')
            while rel_parts and rel_parts[0] == '..':
                rel_parts.pop(0)
                if path_parts:
                    path_parts.pop()
            new_path = '/'.join(path_parts + rel_parts)
            # Rebuild URL with new path
            result = f"{base.scheme}://{base.host}"
            if base.port:
                result += f":{base.port}"
            if new_path:
                if not new_path.startswith('/'):
                    new_path = '/' + new_path
                result += new_path
            return result
        else:
            # URL like 'testing/123' - append to base path
            return base_url_str + '/' + url_str

    async def send(self, request, **kwargs):
        """Send a Request object."""
        await self._acquire_pool_permit()
        try:
            auth = kwargs.pop('auth', None)
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
        elif hasattr(request, '_rust_request'):
            rust_request = request._rust_request
            request_url = request.url if hasattr(request, 'url') else None
        else:
            rust_request = request
            request_url = request.url if hasattr(request, 'url') else None

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
            request_to_send = request if isinstance(request, _WrappedRequest) and request._async_stream is not None else rust_request
            # Check for async handle method
            if hasattr(transport, 'handle_async_request'):
                result = await transport.handle_async_request(request_to_send)
            elif hasattr(transport, 'handle_request'):
                result = transport.handle_request(request_to_send)
            elif callable(transport):
                result = transport(request_to_send)
            else:
                raise TypeError("Transport must have handle_async_request or handle_request method")

            # Wrap result in Response if needed
            if isinstance(result, Response):
                response = result
            elif isinstance(result, _Response):
                response = Response(result)
            else:
                response = Response(result)

            # Set the URL from the request if not already set
            if response._url is None and hasattr(rust_request, 'url'):
                response._url = rust_request.url
            # Store the original request
            if response._request is None:
                if isinstance(request, _WrappedRequest):
                    response._request = request
                else:
                    response._request = _WrappedRequest(rust_request) if hasattr(rust_request, 'url') else request

            # For redirect responses, compute next_request
            if response.status_code in (301, 302, 303, 307, 308):
                location = response.headers.get('location')
                if location:
                    # Build the redirect request
                    response._next_request = self._build_redirect_request(request, response)

            # If response has a stream that hasn't been read, read it now
            # This ensures exceptions during iteration are raised and stream is closed
            if response._stream_content is not None:
                stream_obj = getattr(response, '_stream_object', None)
                try:
                    chunks = []
                    async for chunk in response._stream_content:
                        chunks.append(chunk)
                    response._raw_content = b''.join(chunks)
                    response._stream_content = None
                    response._stream_consumed = True
                    response._response._set_content(response._raw_content)
                except BaseException:
                    # Close the stream on any exception (including KeyboardInterrupt)
                    if stream_obj is not None and hasattr(stream_obj, 'aclose'):
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
            except (_RequestError, _TransportError, _TimeoutException, _NetworkError,
                    _ConnectError, _ReadError, _WriteError, _CloseError, _ProxyError,
                    _ProtocolError, _UnsupportedProtocol, _DecodingError, _TooManyRedirects,
                    _StreamError, _ConnectTimeout, _ReadTimeout, _WriteTimeout, _PoolTimeout,
                    _LocalProtocolError, _RemoteProtocolError) as e:
                raise _convert_exception(e) from None

            # Set URL and request on response
            if response._url is None and hasattr(rust_request, 'url'):
                response._url = rust_request.url
            if response._request is None:
                if isinstance(request, _WrappedRequest):
                    response._request = request
                else:
                    response._request = _WrappedRequest(rust_request) if hasattr(rust_request, 'url') else request

            # Build next_request if this is a redirect
            if response.status_code in (301, 302, 303, 307, 308):
                location = response.headers.get('location')
                if location:
                    response._next_request = self._build_redirect_request(request, response)

            # Invoke response event hooks before returning
            await self._invoke_response_hooks(response)
            return response

    async def _send_handling_redirects(self, request, follow_redirects=False, history=None):
        """Send a request, optionally following redirects."""
        if history is None:
            history = []

        # Get original request URL for fragment preservation
        original_url = request.url if hasattr(request, 'url') else None
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
            next_url = next_request.url if hasattr(next_request, 'url') else None
            if next_url and isinstance(next_url, URL):
                if not next_url.fragment:
                    next_url_str = str(next_url)
                    if '#' not in next_url_str:
                        next_request = self.build_request(
                            next_request.method,
                            next_url_str + '#' + original_fragment,
                            headers=dict(next_request.headers.items()) if hasattr(next_request, 'headers') else None,
                            content=next_request.content if hasattr(next_request, 'content') else None,
                        )

        # Recursively follow
        return await self._send_handling_redirects(next_request, follow_redirects=True, history=history)

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
        requires_response_body = getattr(auth, 'requires_response_body', False)
        if auth is not None:
            import inspect
            auth_type = type(auth)
            # First check if auth_flow is overridden in a Python subclass (for custom auth like RepeatAuth)
            if 'auth_flow' in auth_type.__dict__:
                auth_flow_method = getattr(auth, 'auth_flow', None)
                if auth_flow_method and (inspect.isgeneratorfunction(auth_flow_method) or
                                         (hasattr(auth_flow_method, '__func__') and
                                          inspect.isgeneratorfunction(auth_flow_method.__func__))):
                    auth_flow = auth.auth_flow(wrapped_request)
            # Then check for async_auth_flow
            if auth_flow is None and hasattr(auth, 'async_auth_flow'):
                method = getattr(auth, 'async_auth_flow')
                # Check if it's a generator function (Python auth) or not (Rust auth)
                if inspect.isgeneratorfunction(method) or inspect.isasyncgenfunction(method):
                    auth_flow = auth.async_auth_flow(wrapped_request)
                else:
                    # Check if async_auth_flow is overridden in Python class
                    if 'async_auth_flow' in auth_type.__dict__:
                        auth_flow = auth.async_auth_flow(wrapped_request)
                    else:
                        # Rust auth - pass the underlying request
                        auth_flow = auth.async_auth_flow(wrapped_request._rust_request)
            elif auth_flow is None and hasattr(auth, 'sync_auth_flow'):
                method = getattr(auth, 'sync_auth_flow')
                if inspect.isgeneratorfunction(method):
                    auth_flow = auth.sync_auth_flow(wrapped_request)
                else:
                    # Check if sync_auth_flow is overridden in Python class
                    if 'sync_auth_flow' in auth_type.__dict__:
                        auth_flow = auth.sync_auth_flow(wrapped_request)
                    else:
                        # Rust auth - pass the underlying request
                        auth_flow = auth.sync_auth_flow(wrapped_request._rust_request)

        if auth_flow is None:
            # No auth flow, send with redirect handling
            return await self._send_handling_redirects(wrapped_request, follow_redirects=follow_redirects)

        # Check if auth_flow returned a list (Rust base class) or generator
        import types
        if isinstance(auth_flow, (list, tuple)):
            # Simple list of requests - just send the last one
            last_request = wrapped_request
            for req in auth_flow:
                last_request = req
            return await self._send_handling_redirects(last_request, follow_redirects=follow_redirects)

        # Generator-based auth flow
        history = []
        try:
            # Check if it's an async generator
            if hasattr(auth_flow, '__anext__'):
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
                return await self._send_handling_redirects(response.next_request, follow_redirects=True, history=history)
            return response
        except (StopIteration, StopAsyncIteration):
            return await self._send_handling_redirects(wrapped_request, follow_redirects=follow_redirects)

    async def get(self, url, *, params=None, headers=None, cookies=None,
                  auth=USE_CLIENT_DEFAULT, follow_redirects=None, timeout=None):
        """HTTP GET with proper auth sentinel handling."""
        self._check_closed()
        await self._acquire_pool_permit()
        try:
            actual_auth = _normalize_auth(auth if auth is not USE_CLIENT_DEFAULT else self._auth)
            # Extract auth from URL userinfo if no explicit auth provided
            if actual_auth is None:
                actual_auth = _extract_auth_from_url(str(url))

            # Determine follow_redirects behavior
            actual_follow = follow_redirects if follow_redirects is not None else self._follow_redirects

            # If we have a custom transport, route through redirect handling
            if self._custom_transport is not None:
                request = self.build_request("GET", url, params=params, headers=headers)
                if actual_auth is not None:
                    return await self._send_with_auth(request, actual_auth, follow_redirects=bool(actual_follow))
                return await self._send_handling_redirects(request, follow_redirects=bool(actual_follow))

            if actual_auth is not None:
                result = await self._handle_auth("GET", url, actual_auth, params=params, headers=headers)
                if result is not None:
                    return result
            try:
                response = await self._client.get(url, params=params, headers=headers, cookies=cookies,
                                              auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)
                return Response(response)
            except (_RequestError, _TransportError, _TimeoutException, _NetworkError,
                    _ConnectError, _ReadError, _WriteError, _CloseError, _ProxyError,
                    _ProtocolError, _UnsupportedProtocol, _DecodingError, _TooManyRedirects,
                    _StreamError, _ConnectTimeout, _ReadTimeout, _WriteTimeout, _PoolTimeout,
                    _LocalProtocolError, _RemoteProtocolError) as e:
                raise _convert_exception(e) from None
        finally:
            self._release_pool_permit()

    def _build_redirect_request(self, request, response):
        """Build the next request for following a redirect."""
        location = response.headers.get("location")
        if not location:
            return None

        # Get the original request URL
        if hasattr(request, 'url'):
            original_url = request.url
        else:
            original_url = None

        # Check for invalid characters in location (non-ASCII in host)
        try:
            if location.startswith('//') or location.startswith('/'):
                pass  # Relative URL - will be joined with original
            elif '://' in location:
                from urllib.parse import urlparse
                parsed = urlparse(location)
                if parsed.netloc:
                    host_part = parsed.hostname or ''
                    try:
                        host_part.encode('ascii')
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
            if 'empty host' in str(e).lower() and original_url:
                from urllib.parse import urlparse
                parsed = urlparse(location)
                orig_url = original_url if isinstance(original_url, URL) else URL(str(original_url))
                scheme = parsed.scheme or orig_url.scheme
                host = orig_url.host
                port = parsed.port if parsed.port else None
                path = parsed.path or '/'
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
        if scheme not in ('http', 'https'):
            raise UnsupportedProtocol(f"Scheme {scheme!r} not supported.")

        # Determine method for redirect
        status_code = response.status_code
        method = request.method if hasattr(request, 'method') else 'GET'

        # 301, 302, 303 redirects change method to GET (except for GET/HEAD)
        if status_code in (301, 302, 303) and method not in ('GET', 'HEAD'):
            method = 'GET'

        # Build kwargs for new request
        headers = dict(request.headers.items()) if hasattr(request, 'headers') else {}

        # Remove Host header so it gets set correctly for the new URL
        headers.pop('host', None)
        headers.pop('Host', None)

        # Strip Authorization header on cross-domain redirects
        if original_url:
            orig_host = original_url.host if isinstance(original_url, URL) else URL(str(original_url)).host
            new_host = redirect_url.host
            if orig_host != new_host:
                headers.pop('authorization', None)
                headers.pop('Authorization', None)

        # For 301, 302, 303, don't include body and remove content-length
        content = None
        if status_code in (301, 302, 303):
            headers.pop('content-length', None)
            headers.pop('Content-Length', None)
        elif hasattr(request, 'content'):
            content = request.content

        return self.build_request(method, str(redirect_url), headers=headers, content=content)

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
        # Check for sync iterator/generator in content (AsyncClient can't handle these)
        import inspect
        async_stream = None
        if content is not None:
            if inspect.isgenerator(content):
                raise RuntimeError("Attempted to send an sync request with an AsyncClient instance.")
            if hasattr(content, '__next__') and hasattr(content, '__iter__') and not isinstance(content, (str, bytes, bytearray)):
                raise RuntimeError("Attempted to send an sync request with an AsyncClient instance.")
            # Handle async iterators/generators
            if inspect.isasyncgen(content) or (hasattr(content, '__aiter__') and hasattr(content, '__anext__')):
                # Keep the async iterator for stream tracking (for auth retry detection)
                async_stream = content
                content = None  # Don't pass to Rust, keep in Python wrapper
        await self._acquire_pool_permit()
        try:
            actual_auth = _normalize_auth(auth if auth is not USE_CLIENT_DEFAULT else self._auth)
            if actual_auth is None:
                actual_auth = _extract_auth_from_url(str(url))

            # If we have a custom transport, route through _send_single_request
            if self._custom_transport is not None:
                request = self.build_request("POST", url, content=content, data=data, files=files,
                                            json=json, params=params, headers=headers)
                # If we had an async stream, wrap the request to track it
                if async_stream is not None and isinstance(request, _WrappedRequest):
                    request._async_stream = async_stream
                if actual_auth is not None:
                    return await self._send_with_auth(request, actual_auth)
                return await self._send_single_request(request)

            if actual_auth is not None:
                result = await self._handle_auth("POST", url, actual_auth, content=content, params=params, headers=headers)
                if result is not None:
                    return result
            try:
                response = await self._client.post(url, content=content, data=data, files=files, json=json,
                                               params=params, headers=headers, cookies=cookies,
                                               auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)
                return Response(response)
            except (_RequestError, _TransportError, _TimeoutException, _NetworkError,
                    _ConnectError, _ReadError, _WriteError, _CloseError, _ProxyError,
                    _ProtocolError, _UnsupportedProtocol, _DecodingError, _TooManyRedirects,
                    _StreamError, _ConnectTimeout, _ReadTimeout, _WriteTimeout, _PoolTimeout,
                    _LocalProtocolError, _RemoteProtocolError) as e:
                raise _convert_exception(e) from None
        finally:
            self._release_pool_permit()

    async def put(self, url, *, content=None, data=None, files=None, json=None,
                  params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
                  follow_redirects=None, timeout=None):
        """HTTP PUT with proper auth sentinel handling."""
        self._check_closed()
        await self._acquire_pool_permit()
        try:
            actual_auth = _normalize_auth(auth if auth is not USE_CLIENT_DEFAULT else self._auth)
            if actual_auth is None:
                actual_auth = _extract_auth_from_url(str(url))

            # If we have a custom transport, route through _send_single_request
            if self._custom_transport is not None:
                request = self.build_request("PUT", url, content=content, data=data, files=files,
                                            json=json, params=params, headers=headers)
                if actual_auth is not None:
                    return await self._send_with_auth(request, actual_auth)
                return await self._send_single_request(request)

            if actual_auth is not None:
                result = await self._handle_auth("PUT", url, actual_auth, content=content, params=params, headers=headers)
                if result is not None:
                    return result
            try:
                response = await self._client.put(url, content=content, data=data, files=files, json=json,
                                              params=params, headers=headers, cookies=cookies,
                                              auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)
                return Response(response)
            except (_RequestError, _TransportError, _TimeoutException, _NetworkError,
                    _ConnectError, _ReadError, _WriteError, _CloseError, _ProxyError,
                    _ProtocolError, _UnsupportedProtocol, _DecodingError, _TooManyRedirects,
                    _StreamError, _ConnectTimeout, _ReadTimeout, _WriteTimeout, _PoolTimeout,
                    _LocalProtocolError, _RemoteProtocolError) as e:
                raise _convert_exception(e) from None
        finally:
            self._release_pool_permit()

    async def patch(self, url, *, content=None, data=None, files=None, json=None,
                    params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
                    follow_redirects=None, timeout=None):
        """HTTP PATCH with proper auth sentinel handling."""
        self._check_closed()
        await self._acquire_pool_permit()
        try:
            actual_auth = _normalize_auth(auth if auth is not USE_CLIENT_DEFAULT else self._auth)
            if actual_auth is None:
                actual_auth = _extract_auth_from_url(str(url))

            # If we have a custom transport, route through _send_single_request
            if self._custom_transport is not None:
                request = self.build_request("PATCH", url, content=content, data=data, files=files,
                                            json=json, params=params, headers=headers)
                if actual_auth is not None:
                    return await self._send_with_auth(request, actual_auth)
                return await self._send_single_request(request)

            if actual_auth is not None:
                result = await self._handle_auth("PATCH", url, actual_auth, content=content, params=params, headers=headers)
                if result is not None:
                    return result
            try:
                response = await self._client.patch(url, content=content, data=data, files=files, json=json,
                                                params=params, headers=headers, cookies=cookies,
                                                auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)
                return Response(response)
            except (_RequestError, _TransportError, _TimeoutException, _NetworkError,
                    _ConnectError, _ReadError, _WriteError, _CloseError, _ProxyError,
                    _ProtocolError, _UnsupportedProtocol, _DecodingError, _TooManyRedirects,
                    _StreamError, _ConnectTimeout, _ReadTimeout, _WriteTimeout, _PoolTimeout,
                    _LocalProtocolError, _RemoteProtocolError) as e:
                raise _convert_exception(e) from None
        finally:
            self._release_pool_permit()

    async def delete(self, url, *, params=None, headers=None, cookies=None,
                     auth=USE_CLIENT_DEFAULT, follow_redirects=None, timeout=None):
        """HTTP DELETE with proper auth sentinel handling."""
        self._check_closed()
        await self._acquire_pool_permit()
        try:
            actual_auth = _normalize_auth(auth if auth is not USE_CLIENT_DEFAULT else self._auth)
            if actual_auth is None:
                actual_auth = _extract_auth_from_url(str(url))

            # If we have a custom transport, route through _send_single_request
            if self._custom_transport is not None:
                request = self.build_request("DELETE", url, params=params, headers=headers)
                if actual_auth is not None:
                    return await self._send_with_auth(request, actual_auth)
                return await self._send_single_request(request)

            if actual_auth is not None:
                result = await self._handle_auth("DELETE", url, actual_auth, params=params, headers=headers)
                if result is not None:
                    return result
            try:
                response = await self._client.delete(url, params=params, headers=headers, cookies=cookies,
                                                 auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)
                return Response(response)
            except (_RequestError, _TransportError, _TimeoutException, _NetworkError,
                    _ConnectError, _ReadError, _WriteError, _CloseError, _ProxyError,
                    _ProtocolError, _UnsupportedProtocol, _DecodingError, _TooManyRedirects,
                    _StreamError, _ConnectTimeout, _ReadTimeout, _WriteTimeout, _PoolTimeout,
                    _LocalProtocolError, _RemoteProtocolError) as e:
                raise _convert_exception(e) from None
        finally:
            self._release_pool_permit()

    async def head(self, url, *, params=None, headers=None, cookies=None,
                   auth=USE_CLIENT_DEFAULT, follow_redirects=None, timeout=None):
        """HTTP HEAD with proper auth sentinel handling."""
        self._check_closed()
        await self._acquire_pool_permit()
        try:
            actual_auth = _normalize_auth(auth if auth is not USE_CLIENT_DEFAULT else self._auth)
            if actual_auth is None:
                actual_auth = _extract_auth_from_url(str(url))

            # If we have a custom transport, route through _send_single_request
            if self._custom_transport is not None:
                request = self.build_request("HEAD", url, params=params, headers=headers)
                if actual_auth is not None:
                    return await self._send_with_auth(request, actual_auth)
                return await self._send_single_request(request)

            if actual_auth is not None:
                result = await self._handle_auth("HEAD", url, actual_auth, params=params, headers=headers)
                if result is not None:
                    return result
            try:
                response = await self._client.head(url, params=params, headers=headers, cookies=cookies,
                                               auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)
                return Response(response)
            except (_RequestError, _TransportError, _TimeoutException, _NetworkError,
                    _ConnectError, _ReadError, _WriteError, _CloseError, _ProxyError,
                    _ProtocolError, _UnsupportedProtocol, _DecodingError, _TooManyRedirects,
                    _StreamError, _ConnectTimeout, _ReadTimeout, _WriteTimeout, _PoolTimeout,
                    _LocalProtocolError, _RemoteProtocolError) as e:
                raise _convert_exception(e) from None
        finally:
            self._release_pool_permit()

    async def options(self, url, *, params=None, headers=None, cookies=None,
                      auth=USE_CLIENT_DEFAULT, follow_redirects=None, timeout=None):
        """HTTP OPTIONS with proper auth sentinel handling."""
        self._check_closed()
        await self._acquire_pool_permit()
        try:
            actual_auth = _normalize_auth(auth if auth is not USE_CLIENT_DEFAULT else self._auth)
            if actual_auth is None:
                actual_auth = _extract_auth_from_url(str(url))

            # If we have a custom transport, route through _send_single_request
            if self._custom_transport is not None:
                request = self.build_request("OPTIONS", url, params=params, headers=headers)
                if actual_auth is not None:
                    return await self._send_with_auth(request, actual_auth)
                return await self._send_single_request(request)

            if actual_auth is not None:
                result = await self._handle_auth("OPTIONS", url, actual_auth, params=params, headers=headers)
                if result is not None:
                    return result
            try:
                response = await self._client.options(url, params=params, headers=headers, cookies=cookies,
                                                  auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)
                return Response(response)
            except (_RequestError, _TransportError, _TimeoutException, _NetworkError,
                    _ConnectError, _ReadError, _WriteError, _CloseError, _ProxyError,
                    _ProtocolError, _UnsupportedProtocol, _DecodingError, _TooManyRedirects,
                    _StreamError, _ConnectTimeout, _ReadTimeout, _WriteTimeout, _PoolTimeout,
                    _LocalProtocolError, _RemoteProtocolError) as e:
                raise _convert_exception(e) from None
        finally:
            self._release_pool_permit()

    async def request(self, method, url, *, content=None, data=None, files=None, json=None,
                      params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
                      follow_redirects=None, timeout=None):
        """HTTP request with proper auth sentinel handling."""
        self._check_closed()
        await self._acquire_pool_permit()
        try:
            actual_auth = _normalize_auth(auth if auth is not USE_CLIENT_DEFAULT else self._auth)
            if actual_auth is None:
                actual_auth = _extract_auth_from_url(str(url))

            # If we have a custom transport, route through _send_single_request
            if self._custom_transport is not None:
                request = self.build_request(method, url, content=content, data=data, files=files,
                                            json=json, params=params, headers=headers)
                if actual_auth is not None:
                    return await self._send_with_auth(request, actual_auth)
                return await self._send_single_request(request)

            if actual_auth is not None:
                result = await self._handle_auth(method, url, actual_auth, content=content, params=params, headers=headers)
                if result is not None:
                    return result
            try:
                response = await self._client.request(method, url, content=content, data=data, files=files,
                                                  json=json, params=params, headers=headers, cookies=cookies,
                                                  auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)
                return Response(response)
            except (_RequestError, _TransportError, _TimeoutException, _NetworkError,
                    _ConnectError, _ReadError, _WriteError, _CloseError, _ProxyError,
                    _ProtocolError, _UnsupportedProtocol, _DecodingError, _TooManyRedirects,
                    _StreamError, _ConnectTimeout, _ReadTimeout, _WriteTimeout, _PoolTimeout,
                    _LocalProtocolError, _RemoteProtocolError) as e:
                raise _convert_exception(e) from None
        finally:
            self._release_pool_permit()

    @_contextlib.asynccontextmanager
    async def stream(self, method, url, *, content=None, data=None, files=None, json=None,
                     params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
                     follow_redirects=None, timeout=None):
        """Stream an HTTP request with proper auth handling."""
        actual_auth = _normalize_auth(auth if auth is not USE_CLIENT_DEFAULT else self._auth)
        if actual_auth is None:
            actual_auth = _extract_auth_from_url(str(url))
        await self._acquire_pool_permit()
        try:
            response = None
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
                if self._custom_transport is not None:
                    request = self.build_request(method, url, content=content, data=data, files=files,
                                                json=json, params=params, headers=headers)
                    response = await self._send_single_request(request)
                else:
                    # Call Rust client directly to avoid double pool acquisition from self.request()
                    try:
                        resp = await self._client.request(method, url, content=content, data=data, files=files,
                                                          json=json, params=params, headers=headers, cookies=cookies,
                                                          auth=_convert_auth(auth), follow_redirects=follow_redirects, timeout=timeout)
                        response = Response(resp)
                    except (_RequestError, _TransportError, _TimeoutException, _NetworkError,
                            _ConnectError, _ReadError, _WriteError, _CloseError, _ProxyError,
                            _ProtocolError, _UnsupportedProtocol, _DecodingError, _TooManyRedirects,
                            _StreamError, _ConnectTimeout, _ReadTimeout, _WriteTimeout, _PoolTimeout,
                            _LocalProtocolError, _RemoteProtocolError) as e:
                        raise _convert_exception(e) from None
            # Mark as a streaming response that requires aread() before content access
            response._stream_not_read = True
            response._is_stream = True
            yield response
        finally:
            self._release_pool_permit()


# Wrap sync Client to support auth=None vs auth not specified
class _HeadersProxy(Headers):
    """Proxy object that wraps Headers and syncs changes back to the client.

    Inherits from Headers to pass isinstance checks while proxying to client headers.
    """

    def __new__(cls, client):
        # Use Headers.__new__ as required by PyO3 subclasses
        instance = Headers.__new__(cls)
        return instance

    def __init__(self, client):
        # Don't call super().__init__() - we're proxying, not wrapping
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
        import os
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

        # Extract proxy and mounts from kwargs
        proxy = kwargs.pop('proxy', None)
        mounts = kwargs.pop('mounts', None)
        trust_env = kwargs.get('trust_env', True)

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
        custom_transport = kwargs.get('transport', None)
        if custom_transport is not None:
            self._default_transport = custom_transport
        elif proxy is not None:
            self._default_transport = HTTPTransport(proxy=proxy)
        else:
            # Check for proxy env vars if trust_env is True
            env_proxy = None
            if trust_env:
                env_proxy = self._get_proxy_from_env()
            if env_proxy:
                self._default_transport = HTTPTransport(proxy=env_proxy)
            else:
                self._default_transport = HTTPTransport()

        self._custom_transport = custom_transport  # Keep reference to user-provided transport

        # Extract and store follow_redirects from kwargs before passing to Rust
        self._follow_redirects = kwargs.pop('follow_redirects', False)

        # Extract and store default_encoding for response text decoding
        self._default_encoding = kwargs.pop('default_encoding', None)

        # Extract and store params from kwargs
        params = kwargs.pop('params', None)
        if params is not None:
            self._params = QueryParams(params)
        else:
            self._params = QueryParams()

        # Always create Rust client with follow_redirects=False so Python handles redirects
        # This allows proper logging and history tracking
        kwargs['follow_redirects'] = False
        self._client = _Client(*args, **kwargs)
        self._headers_proxy = None
        self._is_closed = False

    def _get_proxy_from_env(self):
        """Get proxy URL from environment variables."""
        import os
        # Check common proxy env vars
        for var in ('ALL_PROXY', 'all_proxy', 'HTTPS_PROXY', 'https_proxy', 'HTTP_PROXY', 'http_proxy'):
            proxy = os.environ.get(var)
            if proxy:
                # Auto-prepend http:// if no scheme
                if '://' not in proxy:
                    proxy = 'http://' + proxy
                return proxy
        return None

    def _should_use_proxy(self, url):
        """Check if URL should use proxy based on NO_PROXY env var."""
        import os
        no_proxy = os.environ.get('NO_PROXY', os.environ.get('no_proxy', ''))

        if not no_proxy:
            return True

        if no_proxy == '*':
            return False

        # Get host from URL
        if isinstance(url, str):
            url = URL(url)
        host = url.host

        for pattern in no_proxy.split(','):
            pattern = pattern.strip()
            if not pattern:
                continue

            # Check if pattern has scheme
            if '://' in pattern:
                pattern_scheme, pattern_host = pattern.split('://', 1)
                # Check scheme matches
                if pattern_scheme != url.scheme:
                    continue
                pattern = pattern_host

            # Check for exact match
            if host == pattern:
                return False

            # Check if host ends with pattern (with dot separator)
            if pattern.startswith('.'):
                # .example.com matches www.example.com
                if host.endswith(pattern):
                    return False
            elif host.endswith('.' + pattern):
                # example.com matches www.example.com but not wwwexample.com
                return False

        return True

    @property
    def _transport(self):
        """Get the default transport for this client."""
        return self._default_transport

    def _transport_for_url(self, url):
        """Get the transport to use for a given URL.

        Returns the most specific matching mount, or the default transport if no match.
        """
        import os
        if isinstance(url, str):
            url = URL(url)

        url_scheme = url.scheme
        url_host = url.host or ''
        url_port = url.port

        # First check mounts dictionary for a matching pattern
        best_match = None
        best_score = -1

        for pattern, transport in self._mounts.items():
            score = self._match_pattern(url_scheme, url_host, url_port, pattern)
            if score > best_score:
                best_score = score
                best_match = transport

        if best_match is not None:
            return best_match

        # If trust_env is enabled, check environment variables
        if getattr(self._client, 'trust_env', True):
            proxy_url = self._get_proxy_for_url(url)
            if proxy_url:
                if not self._should_use_proxy(url):
                    return self._default_transport
                return HTTPTransport(proxy=proxy_url)

        return self._default_transport

    def _get_proxy_for_url(self, url):
        """Get proxy URL from environment for a specific URL."""
        import os
        scheme = url.scheme if hasattr(url, 'scheme') else 'http'

        # Check scheme-specific proxy first
        if scheme == 'https':
            proxy = os.environ.get('HTTPS_PROXY', os.environ.get('https_proxy'))
            if proxy:
                if '://' not in proxy:
                    proxy = 'http://' + proxy
                return proxy

        if scheme == 'http':
            proxy = os.environ.get('HTTP_PROXY', os.environ.get('http_proxy'))
            if proxy:
                if '://' not in proxy:
                    proxy = 'http://' + proxy
                return proxy

        # Fallback to ALL_PROXY
        proxy = os.environ.get('ALL_PROXY', os.environ.get('all_proxy'))
        if proxy:
            if '://' not in proxy:
                proxy = 'http://' + proxy
            return proxy

        return None

    def _match_pattern(self, url_scheme, url_host, url_port, pattern):
        """Match URL against a mount pattern. Returns score (higher is better match), or -1 if no match."""
        # Parse pattern
        if '://' in pattern:
            pattern_scheme, pattern_rest = pattern.split('://', 1)
        else:
            return -1  # Invalid pattern

        # Check scheme match
        if pattern_scheme not in ('all', url_scheme):
            return -1

        # Score: all:// = 0, http:// = 1, with host = +2, with port = +4
        score = 0 if pattern_scheme == 'all' else 1

        if not pattern_rest:
            # Pattern is just "http://" or "all://"
            return score

        # Parse host and port from pattern
        if ':' in pattern_rest and not pattern_rest.startswith('['):
            pattern_host, pattern_port_str = pattern_rest.rsplit(':', 1)
            try:
                pattern_port = int(pattern_port_str)
            except ValueError:
                pattern_host = pattern_rest
                pattern_port = None
        else:
            pattern_host = pattern_rest
            pattern_port = None

        # Match host
        if pattern_host == '*':
            # Matches any host
            score += 2
        elif pattern_host.startswith('*.'):
            # Wildcard subdomain: *.example.com matches www.example.com but not example.com
            suffix = pattern_host[1:]  # ".example.com"
            if url_host.endswith(suffix) and url_host != suffix[1:]:
                score += 2
            else:
                return -1
        elif pattern_host.startswith('*'):
            # Pattern like "*example.com" - must end with .example.com or be example.com
            suffix = pattern_host[1:]  # "example.com"
            if url_host == suffix or url_host.endswith('.' + suffix):
                score += 2
            else:
                return -1
        else:
            # Exact host match (case insensitive)
            if url_host.lower() != pattern_host.lower():
                return -1
            score += 2

        # Match port if specified
        if pattern_port is not None:
            if url_port == pattern_port:
                score += 4
            # Don't return -1 if port doesn't match - host without port matches any port
            # But if pattern has port, it should match for higher score

        return score

    def _invoke_request_hooks(self, request):
        """Invoke all request event hooks."""
        hooks = self.event_hooks.get('request', [])
        for hook in hooks:
            hook(request)

    def _invoke_response_hooks(self, response):
        """Invoke all response event hooks."""
        hooks = self.event_hooks.get('response', [])
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
        if self._transport is not None and hasattr(self._transport, '__enter__'):
            self._transport.__enter__()
        # Call __enter__ on all mounted transports
        for transport in self._mounts.values():
            if hasattr(transport, '__enter__'):
                transport.__enter__()
        self._client.__enter__()
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        result = self._client.__exit__(exc_type, exc_val, exc_tb)
        # Call transport's __exit__ if it exists
        if self._transport is not None and hasattr(self._transport, '__exit__'):
            self._transport.__exit__(exc_type, exc_val, exc_tb)
        # Call __exit__ on all mounted transports
        for transport in self._mounts.values():
            if hasattr(transport, '__exit__'):
                transport.__exit__(exc_type, exc_val, exc_tb)
        self._is_closed = True
        return result

    def close(self):
        """Close the client."""
        if hasattr(self._client, 'close'):
            self._client.close()
        if self._transport is not None and hasattr(self._transport, 'close'):
            self._transport.close()
        # Close all mounted transports
        for transport in self._mounts.values():
            if hasattr(transport, 'close'):
                transport.close()
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
        if not hasattr(self, '_headers_proxy') or self._headers_proxy is None:
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
        elif callable(value) or hasattr(value, 'sync_auth_flow') or hasattr(value, 'async_auth_flow'):
            self._auth = value
        else:
            raise TypeError(f"Invalid 'auth' argument. Expected (username, password) tuple, Auth instance, or callable. Got {type(value).__name__}.")

    def build_request(self, method, url, **kwargs):
        """Build a Request object - wrap result in Python Request class."""
        # Check for async iterator/generator in content (sync Client can't handle these)
        import inspect
        import types
        content = kwargs.get('content')
        sync_stream = None  # Track if we're using a generator stream
        if content is not None:
            if inspect.isasyncgen(content) or inspect.iscoroutine(content):
                raise RuntimeError("Attempted to send an async request with a sync Client instance.")
            # Also check for async iterator protocol
            if hasattr(content, '__anext__') or hasattr(content, '__aiter__'):
                raise RuntimeError("Attempted to send an async request with a sync Client instance.")
            # Handle sync generators/iterators - wrap them in a trackable stream
            if isinstance(content, types.GeneratorType):
                # Create a wrapper that tracks consumption
                # Pass None to Rust - the body will be read from the stream by the transport
                sync_stream = _GeneratorByteStream(content)
                kwargs['content'] = None  # Don't pass generator to Rust
            elif hasattr(content, '__iter__') and hasattr(content, '__next__') and not isinstance(content, (bytes, str, list, tuple)):
                # It's an iterator - wrap it
                sync_stream = _GeneratorByteStream(content)
                kwargs['content'] = None
        # Validate URL before processing
        url_str = str(url)
        # Check for empty scheme (like '://example.org')
        if url_str.startswith('://'):
            raise UnsupportedProtocol("Request URL is missing an 'http://' or 'https://' protocol.")
        # Check for missing host (like 'http://' or 'http:///path')
        if url_str.startswith('http://') or url_str.startswith('https://'):
            # Extract the part after scheme
            after_scheme = url_str.split('://', 1)[1] if '://' in url_str else ''
            # Empty host or starts with / means no host
            if not after_scheme or after_scheme.startswith('/'):
                raise UnsupportedProtocol("Request URL is missing an 'http://' or 'https://' protocol.")
        # Handle URL merging with base_url
        merged_url = self._merge_url(url)

        # Merge client params with request params
        request_params = kwargs.get('params')
        if self._params:
            if request_params is not None:
                # Merge: client params first, then request params
                merged_params = QueryParams(self._params)
                merged_params = merged_params.merge(QueryParams(request_params))
                kwargs['params'] = merged_params
            else:
                kwargs['params'] = self._params

        rust_request = self._client.build_request(method, merged_url, **kwargs)
        # Create a wrapper that delegates to the Rust request but has our headers proxy
        wrapped = _WrappedRequest(rust_request, sync_stream=sync_stream)
        # Link the stream back to the owner for consumption tracking
        if sync_stream is not None:
            sync_stream._owner = wrapped
        return wrapped

    def _merge_url(self, url):
        """Merge a URL with the base_url.

        Unlike RFC 3986 URL resolution, this concatenates paths when the
        relative URL starts with '/'.
        """
        if isinstance(url, URL):
            url_str = str(url)
        else:
            url_str = str(url)

        # If URL is absolute (has scheme), return as-is
        if '://' in url_str:
            return url_str

        # Get base_url from client
        base_url = self.base_url
        if base_url is None:
            return url_str

        base_url_str = str(base_url)

        # If base_url ends with '/', remove it for concatenation
        if base_url_str.endswith('/'):
            base_url_str = base_url_str[:-1]

        # Handle relative URLs
        if url_str.startswith('/'):
            # URL like '/testing/123' - append to base path
            return base_url_str + url_str
        elif url_str.startswith('../'):
            # URL like '../testing/123' - handle relative path navigation
            # Parse base URL to get components
            base = URL(base_url_str)
            base_path = base.path or ''
            # Remove trailing component from base path
            if base_path.endswith('/'):
                base_path = base_path[:-1]
            path_parts = base_path.split('/')
            # Process ../ in relative URL
            rel_parts = url_str.split('/')
            while rel_parts and rel_parts[0] == '..':
                rel_parts.pop(0)
                if path_parts:
                    path_parts.pop()
            new_path = '/'.join(path_parts + rel_parts)
            # Rebuild URL with new path
            result = f"{base.scheme}://{base.host}"
            if base.port:
                result += f":{base.port}"
            if new_path:
                if not new_path.startswith('/'):
                    new_path = '/' + new_path
                result += new_path
            return result
        else:
            # URL like 'testing/123' - append to base path
            return base_url_str + '/' + url_str

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
        elif hasattr(request, '_rust_request'):
            rust_request = request._rust_request
            request_url = url or request.url
        else:
            rust_request = request
            request_url = url or (request.url if hasattr(request, 'url') else None)

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
            if isinstance(transport, (MockTransport, BaseTransport, AsyncBaseTransport)):
                # Python transport - pass wrapped request for stream tracking
                request_to_send = request if isinstance(request, _WrappedRequest) else rust_request
            else:
                # Rust transport - pass raw Rust request
                request_to_send = rust_request
            if hasattr(transport, 'handle_request'):
                result = transport.handle_request(request_to_send)
            elif callable(transport):
                result = transport(request_to_send)
            else:
                raise TypeError("Transport must have handle_request method")
            # Wrap result in Response if needed
            if isinstance(result, Response):
                response = result
                if response._default_encoding is None and self._default_encoding is not None:
                    response._default_encoding = self._default_encoding
            elif isinstance(result, _Response):
                response = Response(result, default_encoding=self._default_encoding)
            else:
                response = Response(result, default_encoding=self._default_encoding)
        else:
            try:
                result = self._client.send(rust_request)
                response = Response(result, default_encoding=self._default_encoding)
            except (_RequestError, _TransportError, _TimeoutException, _NetworkError,
                    _ConnectError, _ReadError, _WriteError, _CloseError, _ProxyError,
                    _ProtocolError, _UnsupportedProtocol, _DecodingError, _TooManyRedirects,
                    _StreamError, _ConnectTimeout, _ReadTimeout, _WriteTimeout, _PoolTimeout,
                    _LocalProtocolError, _RemoteProtocolError) as e:
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
        method = request.method if hasattr(request, 'method') else 'GET'
        url_str = str(request_url) if request_url else ''
        status_code = response.status_code
        reason_phrase = response.reason_phrase or ''
        _logger.info(f'HTTP Request: {method} {url_str} "HTTP/1.1 {status_code} {reason_phrase}"')

        return response

    def _build_redirect_request(self, request, response):
        """Build the next request for following a redirect."""
        location = response.headers.get("location")
        if not location:
            return None

        # Get the original request URL
        if hasattr(request, 'url'):
            original_url = request.url
        else:
            original_url = None

        # Check for invalid characters in location (non-ASCII in host)
        # Emojis and other non-ASCII characters in the host portion are invalid
        try:
            # First try to parse the location URL
            if location.startswith('//') or location.startswith('/'):
                # Relative URL - will be joined with original
                pass
            elif '://' in location:
                # Absolute URL - check if host contains invalid characters
                from urllib.parse import urlparse
                parsed = urlparse(location)
                if parsed.netloc:
                    # Check for non-ASCII characters in host (excluding punycode)
                    host_part = parsed.hostname or ''
                    try:
                        # Try to encode as ASCII - if it fails and it's not punycode, it's invalid
                        host_part.encode('ascii')
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
            if 'empty host' in str(e).lower() and original_url:
                # Try to extract what we can from the location
                from urllib.parse import urlparse
                parsed = urlparse(location)
                orig_url = original_url if isinstance(original_url, URL) else URL(str(original_url))

                # Build URL manually using original host
                scheme = parsed.scheme or orig_url.scheme
                host = orig_url.host  # Use original host since location has empty host
                port = parsed.port if parsed.port else None
                path = parsed.path or '/'

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
        if scheme not in ('http', 'https'):
            raise UnsupportedProtocol(f"Scheme {scheme!r} not supported.")

        # Determine method for redirect
        status_code = response.status_code
        method = request.method if hasattr(request, 'method') else 'GET'

        # 301, 302, 303 redirects change method to GET (except for GET/HEAD)
        if status_code in (301, 302, 303) and method not in ('GET', 'HEAD'):
            method = 'GET'

        # Build kwargs for new request
        headers = dict(request.headers.items()) if hasattr(request, 'headers') else {}

        # Remove Host header so it gets set correctly for the new URL
        headers.pop('host', None)
        headers.pop('Host', None)

        # Strip Authorization header on cross-domain redirects
        if original_url:
            orig_host = original_url.host if isinstance(original_url, URL) else URL(str(original_url)).host
            new_host = redirect_url.host
            if orig_host != new_host:
                headers.pop('authorization', None)
                headers.pop('Authorization', None)

        # For 301, 302, 303, don't include body and remove content-length
        content = None
        if status_code in (301, 302, 303):
            # Remove Content-Length for body-less redirects
            headers.pop('content-length', None)
            headers.pop('Content-Length', None)
        elif hasattr(request, 'content'):
            # 307/308 preserve body
            content = request.content
            # Check if stream was consumed
            if hasattr(request, 'stream'):
                stream = request.stream
                # Check various consumed indicators
                if hasattr(stream, '_consumed') and stream._consumed:
                    raise StreamConsumed()
                # For SyncByteStream, check if it's already been iterated
                if isinstance(stream, SyncByteStream) and getattr(stream, '_consumed', False):
                    raise StreamConsumed()
            # Also check if the request was built with a generator/iterator stream
            if hasattr(request, '_stream_consumed') and request._stream_consumed:
                raise StreamConsumed()
            if isinstance(request, _WrappedRequest) and request._stream_consumed:
                raise StreamConsumed()

        # Add client cookies to redirect request
        # This ensures cookies set via Set-Cookie headers are sent on subsequent requests
        if self.cookies:
            cookie_header = "; ".join(f"{name}={value}" for name, value in self.cookies.items())
            if cookie_header:
                headers['Cookie'] = cookie_header

        wrapped_request = self.build_request(method, redirect_url_str, headers=headers, content=content)
        # Store explicit URL if we have one (preserves non-normalized port)
        if explicit_url_str:
            wrapped_request._explicit_url = explicit_url_str
        return wrapped_request

    def _send_handling_redirects(self, request, follow_redirects=False, history=None):
        """Send a request, optionally following redirects."""
        if history is None:
            history = []

        # Get original request URL for fragment preservation
        original_url = request.url if hasattr(request, 'url') else None
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
                cookie_header = "; ".join(f"{name}={value}" for name, value in self.cookies.items())
                next_request.headers['Cookie'] = cookie_header
            else:
                # Cookies might have been deleted (e.g., expired), remove the Cookie header
                try:
                    del next_request.headers['Cookie']
                except KeyError:
                    pass

        # Preserve fragment from original URL
        if original_fragment:
            next_url = next_request.url if hasattr(next_request, 'url') else None
            if next_url and isinstance(next_url, URL):
                if not next_url.fragment:
                    # Add fragment to URL
                    next_url_str = str(next_url)
                    if '#' not in next_url_str:
                        next_request = self.build_request(
                            next_request.method,
                            next_url_str + '#' + original_fragment,
                            headers=dict(next_request.headers.items()) if hasattr(next_request, 'headers') else None,
                            content=next_request.content if hasattr(next_request, 'content') else None,
                        )

        # Recursively follow
        return self._send_handling_redirects(next_request, follow_redirects=True, history=history)

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
            # No auth flow, send with redirect handling
            return self._send_handling_redirects(wrapped_request, follow_redirects=follow_redirects)

        # Check if auth_flow returned a list (Rust base class) or generator
        import types
        if isinstance(auth_flow, (list, tuple)):
            # Simple list of requests - just send the last one
            last_request = wrapped_request
            for req in auth_flow:
                last_request = req
            return self._send_handling_redirects(last_request, follow_redirects=follow_redirects)

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
                    response._history = list(history)  # Copy current history to this response
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
                return self._send_handling_redirects(response.next_request, follow_redirects=True, history=history)

            return response
        except StopIteration:
            # Auth flow returned without yielding, send request as-is
            return self._send_handling_redirects(wrapped_request, follow_redirects=follow_redirects)

    def send(self, request, **kwargs):
        """Send a Request object."""
        auth = kwargs.pop('auth', None)
        follow_redirects = kwargs.pop('follow_redirects', None)
        actual_follow = follow_redirects if follow_redirects is not None else self._follow_redirects
        if auth is not None:
            return self._send_with_auth(request, auth, follow_redirects=actual_follow)
        # Route through redirect handling
        return self._send_handling_redirects(request, follow_redirects=bool(actual_follow))

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
                stacklevel=4  # go up to user code
            )

    def _extract_cookies_from_response(self, response, request):
        """Extract Set-Cookie headers from response and add to client cookies."""
        # Get all Set-Cookie headers
        set_cookie_headers = []
        if hasattr(response, 'headers'):
            # Try multi_items to get all Set-Cookie headers
            if hasattr(response.headers, 'multi_items'):
                for key, value in response.headers.multi_items():
                    if key.lower() == 'set-cookie':
                        set_cookie_headers.append(value)
            elif hasattr(response.headers, 'get_list'):
                set_cookie_headers = response.headers.get_list('set-cookie')
            else:
                # Fallback: get single value
                cookie_header = response.headers.get('set-cookie')
                if cookie_header:
                    set_cookie_headers = [cookie_header]

        # Parse and add each cookie
        # Note: client.cookies returns a copy, so we need to get it, modify it, and set it back
        if set_cookie_headers:
            from email.utils import parsedate_to_datetime
            import datetime
            cookies = self.cookies
            for cookie_str in set_cookie_headers:
                # Parse Set-Cookie header: "name=value; attr1; attr2=val"
                parts = cookie_str.split(';')
                if parts:
                    # First part is name=value
                    name_value = parts[0].strip()
                    if '=' in name_value:
                        name, value = name_value.split('=', 1)
                        name = name.strip()
                        value = value.strip()

                        # Check for expires attribute to handle cookie deletion
                        is_expired = False
                        for part in parts[1:]:
                            part = part.strip()
                            if part.lower().startswith('expires='):
                                expires_str = part[8:].strip()
                                try:
                                    expires_dt = parsedate_to_datetime(expires_str)
                                    if expires_dt < datetime.datetime.now(datetime.timezone.utc):
                                        is_expired = True
                                except Exception:
                                    pass
                                break

                        if is_expired:
                            # Delete the cookie
                            cookies.delete(name)
                        else:
                            # Add to cookies
                            cookies.set(name, value)
            # Set cookies back to client
            self.cookies = cookies

    def get(self, url, *, params=None, headers=None, cookies=None,
            auth=USE_CLIENT_DEFAULT, follow_redirects=None, timeout=None):
        """HTTP GET with proper auth and redirect handling."""
        self._check_closed()
        self._warn_per_request_cookies(cookies)
        request = self.build_request("GET", url, params=params, headers=headers, cookies=cookies)
        actual_auth = _normalize_auth(auth if auth is not USE_CLIENT_DEFAULT else self._auth)
        if actual_auth is None:
            actual_auth = _extract_auth_from_url(str(url))
        actual_follow = follow_redirects if follow_redirects is not None else self._follow_redirects
        if actual_auth is not None:
            return self._send_with_auth(request, actual_auth, follow_redirects=actual_follow)
        return self._send_handling_redirects(request, follow_redirects=bool(actual_follow))

    def post(self, url, *, content=None, data=None, files=None, json=None,
             params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
             follow_redirects=None, timeout=None):
        """HTTP POST with proper auth and redirect handling."""
        self._check_closed()
        self._warn_per_request_cookies(cookies)
        request = self.build_request("POST", url, content=content, data=data, files=files,
                                    json=json, params=params, headers=headers, cookies=cookies)
        actual_auth = _normalize_auth(auth if auth is not USE_CLIENT_DEFAULT else self._auth)
        if actual_auth is None:
            actual_auth = _extract_auth_from_url(str(url))
        actual_follow = follow_redirects if follow_redirects is not None else self._follow_redirects
        if actual_auth is not None:
            return self._send_with_auth(request, actual_auth, follow_redirects=actual_follow)
        return self._send_handling_redirects(request, follow_redirects=bool(actual_follow))

    def put(self, url, *, content=None, data=None, files=None, json=None,
            params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
            follow_redirects=None, timeout=None):
        """HTTP PUT with proper auth and redirect handling."""
        self._check_closed()
        self._warn_per_request_cookies(cookies)
        request = self.build_request("PUT", url, content=content, data=data, files=files,
                                    json=json, params=params, headers=headers, cookies=cookies)
        actual_auth = _normalize_auth(auth if auth is not USE_CLIENT_DEFAULT else self._auth)
        if actual_auth is None:
            actual_auth = _extract_auth_from_url(str(url))
        actual_follow = follow_redirects if follow_redirects is not None else self._follow_redirects
        if actual_auth is not None:
            return self._send_with_auth(request, actual_auth, follow_redirects=actual_follow)
        return self._send_handling_redirects(request, follow_redirects=bool(actual_follow))

    def patch(self, url, *, content=None, data=None, files=None, json=None,
              params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
              follow_redirects=None, timeout=None):
        """HTTP PATCH with proper auth and redirect handling."""
        self._check_closed()
        self._warn_per_request_cookies(cookies)
        request = self.build_request("PATCH", url, content=content, data=data, files=files,
                                    json=json, params=params, headers=headers, cookies=cookies)
        actual_auth = _normalize_auth(auth if auth is not USE_CLIENT_DEFAULT else self._auth)
        if actual_auth is None:
            actual_auth = _extract_auth_from_url(str(url))
        actual_follow = follow_redirects if follow_redirects is not None else self._follow_redirects
        if actual_auth is not None:
            return self._send_with_auth(request, actual_auth, follow_redirects=actual_follow)
        return self._send_handling_redirects(request, follow_redirects=bool(actual_follow))

    def delete(self, url, *, params=None, headers=None, cookies=None,
               auth=USE_CLIENT_DEFAULT, follow_redirects=None, timeout=None):
        """HTTP DELETE with proper auth and redirect handling."""
        self._check_closed()
        self._warn_per_request_cookies(cookies)
        request = self.build_request("DELETE", url, params=params, headers=headers, cookies=cookies)
        actual_auth = _normalize_auth(auth if auth is not USE_CLIENT_DEFAULT else self._auth)
        if actual_auth is None:
            actual_auth = _extract_auth_from_url(str(url))
        actual_follow = follow_redirects if follow_redirects is not None else self._follow_redirects
        if actual_auth is not None:
            return self._send_with_auth(request, actual_auth, follow_redirects=actual_follow)
        return self._send_handling_redirects(request, follow_redirects=bool(actual_follow))

    def head(self, url, *, params=None, headers=None, cookies=None,
             auth=USE_CLIENT_DEFAULT, follow_redirects=None, timeout=None):
        """HTTP HEAD with proper auth and redirect handling."""
        self._check_closed()
        self._warn_per_request_cookies(cookies)
        request = self.build_request("HEAD", url, params=params, headers=headers, cookies=cookies)
        actual_auth = _normalize_auth(auth if auth is not USE_CLIENT_DEFAULT else self._auth)
        if actual_auth is None:
            actual_auth = _extract_auth_from_url(str(url))
        actual_follow = follow_redirects if follow_redirects is not None else self._follow_redirects
        if actual_auth is not None:
            return self._send_with_auth(request, actual_auth, follow_redirects=actual_follow)
        return self._send_handling_redirects(request, follow_redirects=bool(actual_follow))

    def options(self, url, *, params=None, headers=None, cookies=None,
                auth=USE_CLIENT_DEFAULT, follow_redirects=None, timeout=None):
        """HTTP OPTIONS with proper auth and redirect handling."""
        self._check_closed()
        self._warn_per_request_cookies(cookies)
        request = self.build_request("OPTIONS", url, params=params, headers=headers, cookies=cookies)
        actual_auth = _normalize_auth(auth if auth is not USE_CLIENT_DEFAULT else self._auth)
        if actual_auth is None:
            actual_auth = _extract_auth_from_url(str(url))
        actual_follow = follow_redirects if follow_redirects is not None else self._follow_redirects
        if actual_auth is not None:
            return self._send_with_auth(request, actual_auth, follow_redirects=actual_follow)
        return self._send_handling_redirects(request, follow_redirects=bool(actual_follow))

    def request(self, method, url, *, content=None, data=None, files=None, json=None,
                params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
                follow_redirects=None, timeout=None):
        """HTTP request with proper auth and redirect handling."""
        self._check_closed()
        self._warn_per_request_cookies(cookies)
        request = self.build_request(method, url, content=content, data=data, files=files,
                                    json=json, params=params, headers=headers, cookies=cookies)
        actual_auth = _normalize_auth(auth if auth is not USE_CLIENT_DEFAULT else self._auth)
        if actual_auth is None:
            actual_auth = _extract_auth_from_url(str(url))
        actual_follow = follow_redirects if follow_redirects is not None else self._follow_redirects
        if actual_auth is not None:
            return self._send_with_auth(request, actual_auth, follow_redirects=actual_follow)
        return self._send_handling_redirects(request, follow_redirects=bool(actual_follow))

    @_contextlib.contextmanager
    def stream(self, method, url, *, content=None, data=None, files=None, json=None,
               params=None, headers=None, cookies=None, auth=USE_CLIENT_DEFAULT,
               follow_redirects=None, timeout=None):
        """Stream an HTTP request with proper auth handling."""
        actual_auth = _normalize_auth(auth if auth is not USE_CLIENT_DEFAULT else self._auth)
        if actual_auth is None:
            actual_auth = _extract_auth_from_url(str(url))
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


__all__ = sorted([
    "__description__",
    "__title__",
    "__version__",
    "ASGITransport",
    "AsyncBaseTransport",
    "AsyncByteStream",
    "AsyncClient",
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
], key=str.casefold)
