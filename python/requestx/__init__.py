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

__version__ = "0.1.0"
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
