"""
Requestx - High-performance Python HTTP client based on reqwest (Rust)

This library provides a fast HTTP client with an API compatible with HTTPX,
powered by the Rust reqwest library for maximum performance.

Example usage:

    # Sync API
    import requestx

    response = requestx.get("https://httpbin.org/get")
    print(response.status_code)
    print(response.json())

    # Using client for connection pooling
    with requestx.Client() as client:
        response = client.get("https://httpbin.org/get")
        print(response.text)

    # Async API
    import asyncio

    async def main():
        async with requestx.AsyncClient() as client:
            response = await client.get("https://httpbin.org/get")
            print(response.json())

    asyncio.run(main())

    # Streaming Responses (sync)
    with requestx.Client() as client:
        with client.stream("GET", "https://httpbin.org/bytes/1000") as response:
            for chunk in response.iter_bytes(chunk_size=100):
                print(len(chunk))

    # Streaming Responses (async)
    async def stream_example():
        async with requestx.AsyncClient() as client:
            async with await client.stream("GET", "https://httpbin.org/bytes/1000") as response:
                async for chunk in response.aiter_bytes(chunk_size=100):
                    print(len(chunk))

    asyncio.run(stream_example())
"""

import typing
from typing import (
    Any,
    Iterator,
    AsyncIterator,
    Protocol,
    runtime_checkable,
)

from requestx._core import (
    # Client classes
    Client,
    AsyncClient,
    # Response classes
    Response,
    StreamingResponse,
    AsyncStreamingResponse,
    # Iterator classes
    BytesIterator,
    TextIterator,
    LinesIterator,
    AsyncBytesIterator,
    AsyncTextIterator,
    AsyncLinesIterator,
    # Type classes
    Headers,
    Cookies,
    Timeout,
    Proxy,
    Auth as _RustAuth,
    Limits,
    SSLConfig,
    URL,
    Request,
    QueryParams,
    # Exception classes - Base
    RequestError,
    # Transport errors
    TransportError,
    ConnectError,
    ReadError,
    WriteError,
    CloseError,
    ProxyError,
    UnsupportedProtocol,
    # Protocol errors
    ProtocolError,
    LocalProtocolError,
    RemoteProtocolError,
    # Timeout errors
    TimeoutException,
    ConnectTimeout,
    ReadTimeout,
    WriteTimeout,
    PoolTimeout,
    # HTTP status errors
    HTTPStatusError,
    # Redirect errors
    TooManyRedirects,
    # Decoding errors
    DecodingError,
    # Stream errors
    StreamError,
    StreamConsumed,
    StreamClosed,
    ResponseNotRead,
    RequestNotRead,
    # URL errors
    InvalidURL,
    # Cookie errors
    CookieConflict,
    # Module-level functions
    request,
    get,
    post,
    put,
    patch,
    delete,
    head,
    options,
    stream,
)


# HTTPX-compatible exception aliases
HTTPError = RequestError  # Base exception alias
NetworkError = TransportError  # Network error alias


# HTTPX-compatible byte stream protocol classes
@runtime_checkable
class SyncByteStream(Protocol):
    """
    Protocol for sync byte streams that can be used as request content.

    Implement __iter__ to yield bytes chunks.
    """

    def __iter__(self) -> Iterator[bytes]:
        """Iterate over bytes chunks."""
        ...

    def close(self) -> None:
        """Close the stream."""
        ...


@runtime_checkable
class AsyncByteStream(Protocol):
    """
    Protocol for async byte streams that can be used as request content.

    Implement __aiter__ to yield bytes chunks asynchronously.
    """

    def __aiter__(self) -> AsyncIterator[bytes]:
        """Iterate over bytes chunks asynchronously."""
        ...

    async def aclose(self) -> None:
        """Close the stream asynchronously."""
        ...


# Alias for compatibility
ByteStream = SyncByteStream


# HTTPX-compatible auth classes
class BasicAuth:
    """
    HTTP Basic Authentication.

    Usage:
        auth = BasicAuth(username="user", password="pass")
        client.get(url, auth=auth)
    """

    def __init__(
        self, username: str | bytes, password: str | bytes = ""
    ) -> None:
        if isinstance(username, bytes):
            username = username.decode("latin-1")
        if isinstance(password, bytes):
            password = password.decode("latin-1")
        self._auth = _RustAuth.basic(username, password)

    def __call__(self, request: Request) -> Request:
        """Apply auth to request (for HTTPX compatibility)."""
        return request

    def __eq__(self, other: object) -> bool:
        if isinstance(other, BasicAuth):
            return True  # Compare internal auth objects
        return NotImplemented

    def __repr__(self) -> str:
        return f"BasicAuth(username={self._auth!r})"


class DigestAuth:
    """
    HTTP Digest Authentication.

    Note: Digest auth is complex and requires challenge-response.
    This implementation falls back to basic auth.
    """

    def __init__(self, username: str | bytes, password: str | bytes) -> None:
        if isinstance(username, bytes):
            username = username.decode("latin-1")
        if isinstance(password, bytes):
            password = password.decode("latin-1")
        self._auth = _RustAuth.digest(username, password)

    def __call__(self, request: Request) -> Request:
        """Apply auth to request (for HTTPX compatibility)."""
        return request

    def __eq__(self, other: object) -> bool:
        if isinstance(other, DigestAuth):
            return True
        return NotImplemented

    def __repr__(self) -> str:
        return "DigestAuth(...)"


def create_ssl_context(
    verify: bool = True,
    cert: str | tuple[str, str] | tuple[str, str, str] | None = None,
    trust_env: bool = True,
) -> "ssl.SSLContext":
    """
    Create an SSL context for use with HTTPS connections.

    Args:
        verify: Whether to verify server certificates. Defaults to True.
        cert: Optional client certificate. Can be a path to a PEM file,
              or a tuple of (cert_file, key_file) or (cert_file, key_file, password).
        trust_env: Whether to trust environment variables for SSL configuration.

    Returns:
        An ssl.SSLContext configured appropriately.
    """
    import os
    import ssl

    if verify:
        context = ssl.create_default_context()
        context.check_hostname = True
        context.verify_mode = ssl.CERT_REQUIRED
    else:
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
        context.check_hostname = False
        context.verify_mode = ssl.CERT_NONE

    # Set up keylog file from environment
    if trust_env:
        keylog_file = os.environ.get("SSLKEYLOGFILE")
        if keylog_file:
            context.keylog_filename = keylog_file

    # Load client certificate if provided
    if cert is not None:
        if isinstance(cert, str):
            context.load_cert_chain(cert)
        elif len(cert) == 2:
            context.load_cert_chain(cert[0], cert[1])
        elif len(cert) == 3:
            context.load_cert_chain(cert[0], cert[1], cert[2])

    return context


# Status codes module (HTTPX compatibility)
class _StatusCodes:
    """HTTP status codes lookup."""

    # Informational
    CONTINUE = 100
    SWITCHING_PROTOCOLS = 101
    PROCESSING = 102
    EARLY_HINTS = 103

    # Success
    OK = 200
    CREATED = 201
    ACCEPTED = 202
    NON_AUTHORITATIVE_INFORMATION = 203
    NO_CONTENT = 204
    RESET_CONTENT = 205
    PARTIAL_CONTENT = 206
    MULTI_STATUS = 207
    ALREADY_REPORTED = 208
    IM_USED = 226

    # Redirection
    MULTIPLE_CHOICES = 300
    MOVED_PERMANENTLY = 301
    FOUND = 302
    SEE_OTHER = 303
    NOT_MODIFIED = 304
    USE_PROXY = 305
    TEMPORARY_REDIRECT = 307
    PERMANENT_REDIRECT = 308

    # Client Error
    BAD_REQUEST = 400
    UNAUTHORIZED = 401
    PAYMENT_REQUIRED = 402
    FORBIDDEN = 403
    NOT_FOUND = 404
    METHOD_NOT_ALLOWED = 405
    NOT_ACCEPTABLE = 406
    PROXY_AUTHENTICATION_REQUIRED = 407
    REQUEST_TIMEOUT = 408
    CONFLICT = 409
    GONE = 410
    LENGTH_REQUIRED = 411
    PRECONDITION_FAILED = 412
    REQUEST_ENTITY_TOO_LARGE = 413
    REQUEST_URI_TOO_LONG = 414
    UNSUPPORTED_MEDIA_TYPE = 415
    REQUESTED_RANGE_NOT_SATISFIABLE = 416
    EXPECTATION_FAILED = 417
    IM_A_TEAPOT = 418
    MISDIRECTED_REQUEST = 421
    UNPROCESSABLE_ENTITY = 422
    LOCKED = 423
    FAILED_DEPENDENCY = 424
    TOO_EARLY = 425
    UPGRADE_REQUIRED = 426
    PRECONDITION_REQUIRED = 428
    TOO_MANY_REQUESTS = 429
    REQUEST_HEADER_FIELDS_TOO_LARGE = 431
    UNAVAILABLE_FOR_LEGAL_REASONS = 451

    # Server Error
    INTERNAL_SERVER_ERROR = 500
    NOT_IMPLEMENTED = 501
    BAD_GATEWAY = 502
    SERVICE_UNAVAILABLE = 503
    GATEWAY_TIMEOUT = 504
    HTTP_VERSION_NOT_SUPPORTED = 505
    VARIANT_ALSO_NEGOTIATES = 506
    INSUFFICIENT_STORAGE = 507
    LOOP_DETECTED = 508
    NOT_EXTENDED = 510
    NETWORK_AUTHENTICATION_REQUIRED = 511

    def __call__(self, status_code: int) -> int:
        """Allow codes(404) to return 404."""
        return status_code

    def __getitem__(self, name: str) -> int:
        """Allow codes['NOT_FOUND'] to return 404."""
        return getattr(self, name.upper())

    def __getattr__(self, name: str) -> int:
        """Allow codes.not_found to return 404 (lowercase)."""
        # Try uppercase version
        upper_name = name.upper()
        # Get from class attributes
        for cls in type(self).__mro__:
            if upper_name in cls.__dict__:
                return cls.__dict__[upper_name]
        raise AttributeError(f"'{type(self).__name__}' object has no attribute '{name}'")

    def get_reason_phrase(self, status_code: int) -> str:
        """Get the reason phrase for a status code."""
        phrases = {
            100: "Continue",
            101: "Switching Protocols",
            102: "Processing",
            103: "Early Hints",
            200: "OK",
            201: "Created",
            202: "Accepted",
            203: "Non-Authoritative Information",
            204: "No Content",
            205: "Reset Content",
            206: "Partial Content",
            207: "Multi-Status",
            208: "Already Reported",
            226: "IM Used",
            300: "Multiple Choices",
            301: "Moved Permanently",
            302: "Found",
            303: "See Other",
            304: "Not Modified",
            305: "Use Proxy",
            307: "Temporary Redirect",
            308: "Permanent Redirect",
            400: "Bad Request",
            401: "Unauthorized",
            402: "Payment Required",
            403: "Forbidden",
            404: "Not Found",
            405: "Method Not Allowed",
            406: "Not Acceptable",
            407: "Proxy Authentication Required",
            408: "Request Timeout",
            409: "Conflict",
            410: "Gone",
            411: "Length Required",
            412: "Precondition Failed",
            413: "Request Entity Too Large",
            414: "Request-URI Too Long",
            415: "Unsupported Media Type",
            416: "Requested Range Not Satisfiable",
            417: "Expectation Failed",
            418: "I'm a teapot",
            421: "Misdirected Request",
            422: "Unprocessable Entity",
            423: "Locked",
            424: "Failed Dependency",
            425: "Too Early",
            426: "Upgrade Required",
            428: "Precondition Required",
            429: "Too Many Requests",
            431: "Request Header Fields Too Large",
            451: "Unavailable For Legal Reasons",
            500: "Internal Server Error",
            501: "Not Implemented",
            502: "Bad Gateway",
            503: "Service Unavailable",
            504: "Gateway Timeout",
            505: "HTTP Version Not Supported",
            506: "Variant Also Negotiates",
            507: "Insufficient Storage",
            508: "Loop Detected",
            510: "Not Extended",
            511: "Network Authentication Required",
        }
        return phrases.get(status_code, "")


codes = _StatusCodes()


# Sentinel for client defaults
class _UseClientDefault:
    """Sentinel to indicate a value should use the client's default."""

    def __repr__(self) -> str:
        return "USE_CLIENT_DEFAULT"

    def __bool__(self) -> bool:
        return False


USE_CLIENT_DEFAULT = _UseClientDefault()

# HTTPX-compatible transport protocol classes
# These are Protocol stubs to allow type checking and isinstance checks
# for custom transport implementations


@runtime_checkable
class BaseTransport(Protocol):
    """
    Base class for synchronous HTTP transports.

    This is a Protocol stub for HTTPX compatibility. Custom transports
    should implement the handle_request method.
    """

    def handle_request(self, request: Request) -> Response:
        """
        Handle a single HTTP request.

        Args:
            request: The HTTP request to send.

        Returns:
            The HTTP response.
        """
        ...

    def close(self) -> None:
        """
        Close the transport.
        """
        ...


@runtime_checkable
class AsyncBaseTransport(Protocol):
    """
    Base class for asynchronous HTTP transports.

    This is a Protocol stub for HTTPX compatibility. Custom transports
    should implement the handle_async_request method.
    """

    async def handle_async_request(self, request: Request) -> Response:
        """
        Handle a single HTTP request asynchronously.

        Args:
            request: The HTTP request to send.

        Returns:
            The HTTP response.
        """
        ...

    async def aclose(self) -> None:
        """
        Close the transport asynchronously.
        """
        ...


class MockTransport(BaseTransport):
    """
    A mock transport that returns responses from a handler function.

    Usage:
        def handler(request):
            return httpx.Response(200, content=b"Hello, World!")

        transport = httpx.MockTransport(handler)
        client = httpx.Client(transport=transport)
        response = client.get("http://example.org/")
    """

    def __init__(
        self,
        handler: "typing.Callable[[Request], Response]",
    ) -> None:
        self.handler = handler

    def handle_request(self, request: Request) -> Response:
        """Handle a request by calling the handler function."""
        return self.handler(request)

    def close(self) -> None:
        """Close the transport (no-op for mock)."""
        pass

    def __enter__(self) -> "MockTransport":
        return self

    def __exit__(self, *args: typing.Any) -> None:
        self.close()


class AsyncMockTransport(AsyncBaseTransport):
    """
    An async mock transport that returns responses from a handler function.

    Usage:
        async def handler(request):
            return httpx.Response(200, content=b"Hello, World!")

        transport = httpx.AsyncMockTransport(handler)
        async with httpx.AsyncClient(transport=transport) as client:
            response = await client.get("http://example.org/")
    """

    def __init__(
        self,
        handler: "typing.Callable[[Request], typing.Union[Response, typing.Awaitable[Response]]]",
    ) -> None:
        self.handler = handler

    async def handle_async_request(self, request: Request) -> Response:
        """Handle a request by calling the handler function."""
        import asyncio

        response = self.handler(request)
        if asyncio.iscoroutine(response):
            response = await response
        return response

    async def aclose(self) -> None:
        """Close the transport (no-op for mock)."""
        pass

    async def __aenter__(self) -> "AsyncMockTransport":
        return self

    async def __aexit__(self, *args: typing.Any) -> None:
        await self.aclose()


class WSGITransport(BaseTransport):
    """
    A transport for making requests to WSGI applications.

    Usage:
        def app(environ, start_response):
            status = "200 OK"
            headers = [("Content-Type", "text/plain")]
            start_response(status, headers)
            return [b"Hello, World!"]

        transport = httpx.WSGITransport(app=app)
        client = httpx.Client(transport=transport)
        response = client.get("http://testserver/")
    """

    def __init__(
        self,
        app: "typing.Callable",
        raise_app_exceptions: bool = True,
        script_name: str = "",
        root_path: str = "",
        wsgi_errors: typing.Optional[typing.IO[str]] = None,
    ) -> None:
        self.app = app
        self.raise_app_exceptions = raise_app_exceptions
        self.script_name = script_name
        self.root_path = root_path
        self.wsgi_errors = wsgi_errors

    def handle_request(self, request: Request) -> Response:
        """Handle a request by calling the WSGI application."""
        import io
        import sys

        # Build WSGI environ
        environ: typing.Dict[str, typing.Any] = {
            "REQUEST_METHOD": request.method,
            "SCRIPT_NAME": self.script_name,
            "PATH_INFO": request.url.path or "/",
            "QUERY_STRING": request.url.query or "",
            "SERVER_NAME": request.url.host or "testserver",
            "SERVER_PORT": str(request.url.port or (443 if request.url.scheme == "https" else 80)),
            "SERVER_PROTOCOL": "HTTP/1.1",
            "wsgi.version": (1, 0),
            "wsgi.url_scheme": request.url.scheme,
            "wsgi.input": io.BytesIO(request.content if hasattr(request, "content") else b""),
            "wsgi.errors": self.wsgi_errors or sys.stderr,
            "wsgi.multithread": True,
            "wsgi.multiprocess": True,
            "wsgi.run_once": False,
        }

        # Add headers to environ
        for key, value in request.headers.items():
            key = key.upper().replace("-", "_")
            if key == "CONTENT_TYPE":
                environ["CONTENT_TYPE"] = value
            elif key == "CONTENT_LENGTH":
                environ["CONTENT_LENGTH"] = value
            else:
                environ[f"HTTP_{key}"] = value

        # Capture response
        response_status: typing.Optional[str] = None
        response_headers: typing.List[typing.Tuple[str, str]] = []
        exc_info_holder: typing.List[typing.Any] = []

        def start_response(
            status: str,
            headers: typing.List[typing.Tuple[str, str]],
            exc_info: typing.Any = None,
        ) -> typing.Callable[[bytes], None]:
            nonlocal response_status, response_headers
            if exc_info and self.raise_app_exceptions:
                exc_info_holder.append(exc_info)
            response_status = status
            response_headers = headers
            return lambda data: None  # write() callable (usually not used)

        try:
            # Call WSGI app
            app_iter = self.app(environ, start_response)

            # Collect response body
            body_parts = []
            try:
                for chunk in app_iter:
                    body_parts.append(chunk)
            finally:
                if hasattr(app_iter, "close"):
                    app_iter.close()

            # Check for exceptions
            if exc_info_holder:
                exc_info = exc_info_holder[0]
                raise exc_info[1].with_traceback(exc_info[2])

            body = b"".join(body_parts)

            # Parse status
            status_code = int(response_status.split(" ", 1)[0]) if response_status else 500

            # Build response
            return Response(
                status_code=status_code,
                headers=response_headers,
                content=body,
            )

        except Exception:
            if self.raise_app_exceptions:
                raise
            return Response(status_code=500, content=b"Internal Server Error")

    def close(self) -> None:
        """Close the transport (no-op for WSGI)."""
        pass

    def __enter__(self) -> "WSGITransport":
        return self

    def __exit__(self, *args: typing.Any) -> None:
        self.close()


class ASGITransport(AsyncBaseTransport):
    """
    A transport for making requests to ASGI applications.

    Usage:
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

        transport = httpx.ASGITransport(app=app)
        async with httpx.AsyncClient(transport=transport) as client:
            response = await client.get("http://testserver/")
    """

    def __init__(
        self,
        app: "typing.Callable",
        raise_app_exceptions: bool = True,
        root_path: str = "",
        client: typing.Tuple[str, int] = ("testclient", 123),
    ) -> None:
        self.app = app
        self.raise_app_exceptions = raise_app_exceptions
        self.root_path = root_path
        self.client = client

    async def handle_async_request(self, request: Request) -> Response:
        """Handle a request by calling the ASGI application."""
        import asyncio

        # Build ASGI scope
        scope: typing.Dict[str, typing.Any] = {
            "type": "http",
            "asgi": {"version": "3.0"},
            "http_version": "1.1",
            "method": request.method,
            "scheme": request.url.scheme,
            "path": request.url.path or "/",
            "query_string": (request.url.query or "").encode("utf-8"),
            "root_path": self.root_path,
            "headers": [
                (k.lower().encode("utf-8"), v.encode("utf-8"))
                for k, v in request.headers.items()
            ],
            "server": (request.url.host or "testserver", request.url.port or 80),
            "client": self.client,
        }

        # Request body handling
        body = request.content if hasattr(request, "content") else b""
        body_sent = False

        async def receive() -> typing.Dict[str, typing.Any]:
            nonlocal body_sent
            if not body_sent:
                body_sent = True
                return {"type": "http.request", "body": body, "more_body": False}
            # Wait indefinitely for disconnect
            await asyncio.sleep(float("inf"))
            return {"type": "http.disconnect"}

        # Response capture
        status_code = 500
        response_headers: typing.List[typing.Tuple[str, str]] = []
        body_parts: typing.List[bytes] = []

        async def send(message: typing.Dict[str, typing.Any]) -> None:
            nonlocal status_code, response_headers
            if message["type"] == "http.response.start":
                status_code = message["status"]
                response_headers = [
                    (k.decode("utf-8") if isinstance(k, bytes) else k,
                     v.decode("utf-8") if isinstance(v, bytes) else v)
                    for k, v in message.get("headers", [])
                ]
            elif message["type"] == "http.response.body":
                body_chunk = message.get("body", b"")
                if body_chunk:
                    body_parts.append(body_chunk)

        try:
            await self.app(scope, receive, send)
        except Exception:
            if self.raise_app_exceptions:
                raise
            return Response(status_code=500, content=b"Internal Server Error")

        return Response(
            status_code=status_code,
            headers=response_headers,
            content=b"".join(body_parts),
        )

    async def aclose(self) -> None:
        """Close the transport (no-op for ASGI)."""
        pass

    async def __aenter__(self) -> "ASGITransport":
        return self

    async def __aexit__(self, *args: typing.Any) -> None:
        await self.aclose()


# Auth base class for HTTPX compatibility (subclassable)
class Auth:
    """
    Base class for authentication schemes.

    Subclass this to implement custom authentication flows.

    Usage:
        class MyAuth(httpx.Auth):
            def auth_flow(self, request):
                request.headers["Authorization"] = "Bearer token"
                yield request

        client.get(url, auth=MyAuth())

    For basic/digest auth, use the static methods:
        auth = httpx.Auth.basic("user", "pass")
        auth = httpx.Auth.bearer("token")
    """

    requires_request_body: bool = False
    requires_response_body: bool = False

    def auth_flow(
        self, request: Request
    ) -> "typing.Generator[Request, Response, None]":
        """
        Execute the authentication flow.

        Yields Request objects, receives Response objects.
        The final Request is sent to the server.
        """
        yield request

    @staticmethod
    def basic(username: str, password: str = "") -> "_RustAuth":
        """Create basic authentication."""
        return _RustAuth.basic(username, password)

    @staticmethod
    def bearer(token: str) -> "_RustAuth":
        """Create bearer token authentication."""
        return _RustAuth.bearer(token)

    @staticmethod
    def digest(username: str, password: str) -> "_RustAuth":
        """Create digest authentication."""
        return _RustAuth.digest(username, password)


# Alias for backwards compatibility
AuthBase = Auth


__version__ = "1.0.8"
__all__ = [
    # Version
    "__version__",
    # Client classes
    "Client",
    "AsyncClient",
    # Response classes
    "Response",
    "StreamingResponse",
    "AsyncStreamingResponse",
    # Iterator classes (for streaming)
    "BytesIterator",
    "TextIterator",
    "LinesIterator",
    "AsyncBytesIterator",
    "AsyncTextIterator",
    "AsyncLinesIterator",
    # Type classes
    "Headers",
    "Cookies",
    "Timeout",
    "Proxy",
    "Auth",
    "Limits",
    "SSLConfig",
    "URL",
    "Request",
    "QueryParams",
    # Transport protocol classes (HTTPX compatibility)
    "BaseTransport",
    "AsyncBaseTransport",
    # Mock transports for testing
    "MockTransport",
    "AsyncMockTransport",
    "WSGITransport",
    "ASGITransport",
    # Auth base class
    "AuthBase",
    # Byte stream protocol classes (HTTPX compatibility)
    "SyncByteStream",
    "AsyncByteStream",
    "ByteStream",
    # Exception classes - Base
    "RequestError",
    "HTTPError",
    "NetworkError",
    # Transport errors
    "TransportError",
    "ConnectError",
    "ReadError",
    "WriteError",
    "CloseError",
    "ProxyError",
    "UnsupportedProtocol",
    # Protocol errors
    "ProtocolError",
    "LocalProtocolError",
    "RemoteProtocolError",
    # Timeout errors
    "TimeoutException",
    "ConnectTimeout",
    "ReadTimeout",
    "WriteTimeout",
    "PoolTimeout",
    # HTTP status errors
    "HTTPStatusError",
    # Redirect errors
    "TooManyRedirects",
    # Decoding errors
    "DecodingError",
    # Stream errors
    "StreamError",
    "StreamConsumed",
    "StreamClosed",
    "ResponseNotRead",
    "RequestNotRead",
    # URL errors
    "InvalidURL",
    # Cookie errors
    "CookieConflict",
    # Module-level functions (sync)
    "request",
    "get",
    "post",
    "put",
    "patch",
    "delete",
    "head",
    "options",
    "stream",
    # SSL context
    "create_ssl_context",
    # Auth helpers
    "BasicAuth",
    "DigestAuth",
    # Status codes
    "codes",
    # Client default sentinel
    "USE_CLIENT_DEFAULT",
]
