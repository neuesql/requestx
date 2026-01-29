# RequestX - High-performance Python HTTP client
# API-compatible with httpx, powered by Rust's reqwest via PyO3

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
    Request,
    Response,
    # Clients
    Client,
    AsyncClient,
    # Configuration
    Timeout,
    Limits,
    # Stream types
    SyncByteStream,
    AsyncByteStream,
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


# Wrap codes to support codes(404) returning int
class codes(_codes):
    """HTTP status codes with flexible access patterns."""

    def __new__(cls, code):
        """Allow codes(404) to return 404."""
        return code

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
