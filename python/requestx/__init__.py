# RequestX - High-performance Python HTTP client
# API-compatible with httpx, powered by Rust's reqwest via PyO3

import http.cookiejar as _http_cookiejar  # noqa: F401  # Import for side effect (httpx compat)

from ._core import (  # noqa: F401
    # Version info
    __version__,
    __title__,
    __description__,
    # Core types
    URL,
    Headers,
    QueryParams,
    Cookies,
    # Configuration
    Timeout,
    Limits,
    Proxy,
    # Transport types (Rust implementations)
    HTTPTransport,
    AsyncHTTPTransport,
    WSGITransport,
    # Exceptions (pass-through from Rust)
    InvalidURL,
    HTTPError,
    CookieConflict,
)

# Compatibility: sentinels, codes wrapper, SSL context, ExplicitPortURL
from ._compat import (  # noqa: F401
    USE_CLIENT_DEFAULT,
    _AuthUnset,
    _AUTH_DISABLED,
    _ExplicitPortURL,
    codes,
    create_ssl_context,
)

# Exception hierarchy with request attribute support
from ._exceptions import (  # noqa: F401
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
    _convert_exception,
)

# Stream classes
from ._streams import (  # noqa: F401
    SyncByteStream,
    AsyncByteStream,
    ByteStream,
)

# Transport base classes and implementations
from ._transports import (  # noqa: F401
    BaseTransport,
    AsyncBaseTransport,
    MockTransport,
    AsyncMockTransport,
    ASGITransport,
)

# Top-level API functions
from ._api import (  # noqa: F401
    get,
    post,
    put,
    patch,
    delete,
    head,
    options,
    request,
    stream,
)

# Request wrapper
from ._request import Request  # noqa: F401

# Response wrapper (includes HTTPStatusError)
from ._response import Response, HTTPStatusError  # noqa: F401

# Auth wrappers
from ._auth import (  # noqa: F401
    Auth,
    BasicAuth,
    DigestAuth,
    NetRCAuth,
    FunctionAuth,
)

# Client classes
from ._async_client import AsyncClient  # noqa: F401
from ._client import Client  # noqa: F401

# Import _utils module for utility functions
from . import _utils  # noqa: F401

__all__ = sorted(
    [
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
    ],
    key=str.casefold,
)
