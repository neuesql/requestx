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

from typing import (
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
    Auth,
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
)

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


class HTTPTransport(BaseTransport):
    """
    HTTP Transport class for HTTPX compatibility.

    This is a stub class that provides basic compatibility with httpx.HTTPTransport.
    In requestx, proxy configuration is handled at the Client level using the proxy parameter.

    For proxy support in requestx, use:
        client = requestx.Client(proxy=requestx.Proxy(http="http://proxy:8080"))

    Example:
        # Instead of httpx-style:
        # transport = httpx.HTTPTransport(proxy="http://proxy:8080")
        # client = httpx.Client(mounts={"http://": transport})

        # Use requestx-style:
        client = requestx.Client(proxy=Proxy(http="http://proxy:8080"))
    """

    def __init__(
        self,
        verify: bool = True,
        cert: str | None = None,
        http1: bool = True,
        http2: bool = False,
        limits: Limits | None = None,
        trust_env: bool = True,
        proxy: Proxy | str | None = None,
        uds: str | None = None,
        local_address: str | None = None,
        retries: int = 0,
        socket_options=None,
    ):
        """
        Initialize HTTP transport.

        Note: This is a stub for HTTPX compatibility. Most parameters are stored
        but not actively used since requestx handles these at the Client level.

        Args:
            verify: SSL verification setting
            cert: Client certificate path
            http1: Enable HTTP/1.1
            http2: Enable HTTP/2
            limits: Connection limits
            trust_env: Trust environment variables
            proxy: Proxy configuration (can be string URL or Proxy object)
            uds: Unix domain socket path (not supported)
            local_address: Local address to bind (not supported)
            retries: Number of retries (not supported at transport level)
            socket_options: Socket options (not supported)
        """
        self._verify = verify
        self._cert = cert
        self._http1 = http1
        self._http2 = http2
        self._limits = limits
        self._trust_env = trust_env
        self._retries = retries

        # Handle proxy - convert string to Proxy if needed
        if isinstance(proxy, str):
            self._proxy = Proxy(all=proxy)
        else:
            self._proxy = proxy

        # These are not supported but stored for compatibility
        self._uds = uds
        self._local_address = local_address
        self._socket_options = socket_options

    def handle_request(self, request: Request) -> Response:
        """
        Handle a single HTTP request.

        Note: This stub method creates a temporary client to handle the request.
        For better performance, use requestx.Client directly.
        """
        with Client(
            verify=self._verify,
            cert=self._cert,
            http2=self._http2,
            limits=self._limits,
            trust_env=self._trust_env,
            proxy=self._proxy,
        ) as client:
            return client.send(request)

    def close(self) -> None:
        """Close the transport."""
        pass


class AsyncHTTPTransport(AsyncBaseTransport):
    """
    Async HTTP Transport class for HTTPX compatibility.

    This is a stub class that provides basic compatibility with httpx.AsyncHTTPTransport.
    In requestx, proxy configuration is handled at the AsyncClient level using the proxy parameter.

    For proxy support in requestx, use:
        client = requestx.AsyncClient(proxy=requestx.Proxy(http="http://proxy:8080"))
    """

    def __init__(
        self,
        verify: bool = True,
        cert: str | None = None,
        http1: bool = True,
        http2: bool = False,
        limits: Limits | None = None,
        trust_env: bool = True,
        proxy: Proxy | str | None = None,
        uds: str | None = None,
        local_address: str | None = None,
        retries: int = 0,
        socket_options=None,
    ):
        """
        Initialize async HTTP transport.

        Note: This is a stub for HTTPX compatibility. Most parameters are stored
        but not actively used since requestx handles these at the AsyncClient level.
        """
        self._verify = verify
        self._cert = cert
        self._http1 = http1
        self._http2 = http2
        self._limits = limits
        self._trust_env = trust_env
        self._retries = retries

        # Handle proxy - convert string to Proxy if needed
        if isinstance(proxy, str):
            self._proxy = Proxy(all=proxy)
        else:
            self._proxy = proxy

        # These are not supported but stored for compatibility
        self._uds = uds
        self._local_address = local_address
        self._socket_options = socket_options

    async def handle_async_request(self, request: Request) -> Response:
        """
        Handle a single HTTP request asynchronously.

        Note: This stub method creates a temporary client to handle the request.
        For better performance, use requestx.AsyncClient directly.
        """
        async with AsyncClient(
            verify=self._verify,
            cert=self._cert,
            http2=self._http2,
            limits=self._limits,
            trust_env=self._trust_env,
            proxy=self._proxy,
        ) as client:
            return await client.send(request)

    async def aclose(self) -> None:
        """Close the transport asynchronously."""
        pass


__version__ = "1.0.9"
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
    "HTTPTransport",
    "AsyncHTTPTransport",
    # Exception classes - Base
    "RequestError",
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
]
