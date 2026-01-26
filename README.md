# Requestx

High-performance Python HTTP client based on [reqwest](https://docs.rs/reqwest/) (Rust), using [PyO3](https://pyo3.rs/) as a bridge. The API is designed to be compatible with [HTTPX](https://www.python-httpx.org/).

## Features

- **High Performance**: Built on Rust's reqwest library for maximum speed
- **Async Support**: Full async/await support using Tokio runtime
- **HTTPX-Compatible API**: Familiar interface for Python developers
- **Connection Pooling**: Automatic connection reuse for better performance
- **HTTP/2 Support**: Optional HTTP/2 with prior knowledge
- **TLS/SSL**: Secure connections via rustls
- **Compression**: Automatic gzip, brotli, and deflate decompression
- **Cookies**: Built-in cookie handling
- **Redirects**: Configurable redirect following
- **Timeouts**: Flexible timeout configuration
- **Proxy Support**: HTTP/HTTPS/SOCKS proxy support
- **Authentication**: Basic, Bearer, and Digest authentication

## Installation

### From PyPI (when published)

```bash
pip install requestx
```

### From Source

Requires Rust toolchain and Python 3.12+.

```bash
# Install maturin
pip install maturin

# Build and install
maturin develop --release
```

## Quick Start

### Synchronous API

```python
import requestx

# Simple GET request
response = requestx.get("https://httpbin.org/get")
print(response.status_code)  # 200
print(response.json())

# POST with JSON
response = requestx.post(
    "https://httpbin.org/post",
    json={"key": "value"}
)

# POST with form data
response = requestx.post(
    "https://httpbin.org/post",
    data={"field": "value"}
)

# Custom headers
response = requestx.get(
    "https://httpbin.org/headers",
    headers={"X-Custom-Header": "value"}
)

# Query parameters
response = requestx.get(
    "https://httpbin.org/get",
    params={"key": "value"}
)

# Using a client for connection pooling
with requestx.Client() as client:
    response = client.get("https://httpbin.org/get")
    print(response.text)
```

### Asynchronous API

```python
import asyncio
import requestx

async def main():
    async with requestx.AsyncClient() as client:
        # Simple GET
        response = await client.get("https://httpbin.org/get")
        print(response.json())

        # Concurrent requests
        tasks = [
            client.get("https://httpbin.org/get"),
            client.get("https://httpbin.org/get"),
            client.get("https://httpbin.org/get"),
        ]
        responses = await asyncio.gather(*tasks)
        for r in responses:
            print(r.status_code)

asyncio.run(main())
```

## Client Configuration

### Sync Client

```python
from requestx import Client, Timeout, Proxy, Auth

client = Client(
    base_url="https://api.example.com",
    headers={"Authorization": "Bearer token"},
    timeout=Timeout(timeout=30.0, connect=5.0),
    follow_redirects=True,
    max_redirects=10,
    verify=True,  # SSL verification
    http2=False,
    proxy=Proxy(url="http://proxy:8080"),
    auth=Auth.basic("user", "pass"),
)
```

### Async Client

```python
from requestx import AsyncClient, Timeout, Auth

client = AsyncClient(
    base_url="https://api.example.com",
    headers={"Authorization": "Bearer token"},
    timeout=Timeout(timeout=30.0, connect=5.0),
    follow_redirects=True,
    max_redirects=10,
    verify=True,
    http2=False,
    auth=Auth.bearer("token"),
)
```

## Response Object

```python
response = requestx.get("https://httpbin.org/get")

# Status
response.status_code  # 200
response.reason_phrase  # "OK"

# Content
response.text  # Decoded text
response.content  # Raw bytes
response.json()  # Parse as JSON

# Headers and cookies
response.headers  # Headers object
response.cookies  # Cookies object

# URL and timing
response.url  # Final URL after redirects
response.elapsed  # Request duration in seconds

# Status checks
response.is_success  # 2xx
response.is_redirect  # 3xx
response.is_client_error  # 4xx
response.is_server_error  # 5xx
response.is_error  # 4xx or 5xx

# Raise exception on error
response.raise_for_status()
```

## Authentication

```python
from requestx import Auth

# Basic authentication
response = requestx.get(
    "https://api.example.com",
    auth=Auth.basic("username", "password")
)

# Bearer token
response = requestx.get(
    "https://api.example.com",
    auth=Auth.bearer("your-token")
)
```

## Timeouts

```python
from requestx import Timeout

# Simple timeout (total)
response = requestx.get("https://example.com", timeout=30.0)

# Detailed timeout configuration
timeout = Timeout(
    timeout=30.0,  # Total timeout
    connect=5.0,   # Connection timeout
    read=10.0,     # Read timeout
    write=10.0,    # Write timeout
    pool=5.0,      # Pool timeout
)
response = requestx.get("https://example.com", timeout=timeout)
```

## Proxy Configuration

```python
from requestx import Proxy, Client

# Single proxy for all protocols
proxy = Proxy(url="http://proxy.example.com:8080")

# Separate proxies
proxy = Proxy(
    http="http://http-proxy:8080",
    https="http://https-proxy:8080",
)

client = Client(proxy=proxy)
```

## File Uploads

```python
# Multipart file upload
files = {
    "file": ("filename.txt", b"file content", "text/plain")
}
response = requestx.post(
    "https://httpbin.org/post",
    files=files
)
```

## Error Handling

```python
from requestx import RequestError

try:
    response = requestx.get("https://example.com")
    response.raise_for_status()
except RequestError as e:
    print(f"Request failed: {e}")
```

## Comparison with HTTPX

| Feature | Requestx | HTTPX |
|---------|----------|-------|
| Language | Rust + Python | Python |
| Async Support | Yes | Yes |
| HTTP/2 | Yes | Yes |
| Connection Pooling | Yes | Yes |
| Performance | Higher | Standard |

## Development

### Building

```bash
# Install development dependencies
pip install maturin pytest pytest-asyncio

# Build in development mode
maturin develop

# Build release wheel
maturin build --release
```

### Testing

```bash
pytest tests/ -v
```

## License

MIT License
