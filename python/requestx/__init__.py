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


__version__ = "1.0.7"
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
