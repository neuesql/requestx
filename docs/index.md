# RequestX Documentation

[![PyPI version](https://img.shields.io/pypi/v/requestx.svg)](https://pypi.org/project/requestx/)
[![Python versions](https://img.shields.io/pypi/pyversions/requestx.svg)](https://pypi.org/project/requestx/)
[![Build status](https://github.com/neuesql/requestx/workflows/Test%20and%20Build/badge.svg)](https://github.com/neuesql/requestx/actions)
[![Code style: black](https://img.shields.io/badge/code%20style-black-000000.svg)](https://github.com/psf/black)

RequestX is a high-performance HTTP client library for Python built on Rust's [reqwest](https://docs.rs/reqwest/) library using [PyO3](https://pyo3.rs/) bindings. The API is designed to be compatible with [HTTPX](https://www.python-httpx.org/).

## Key Features

- **High Performance** - Built on Rust's reqwest for speed and memory safety
- **Dual API Support** - Both synchronous and async/await patterns
- **HTTPX Compatible** - Familiar API for easy migration
- **Connection Pooling** - Efficient connection reuse with persistent sessions
- **HTTP/2 Support** - Modern protocol support out of the box
- **Streaming** - Support for streaming request and response bodies
- **TLS** - Secure connections via rustls

## Performance

RequestX delivers significant performance improvements over traditional Python HTTP libraries:

- **2-5x faster** than requests for synchronous operations
- **3-10x faster** than aiohttp for asynchronous operations
- **Lower memory usage** due to Rust's efficient memory management
- **Better connection pooling** with HTTP/2 support

## Quick Installation

```bash
pip install requestx
```

## Quick Start

### Synchronous API

```python
import requestx

# Simple GET request
response = requestx.get("https://httpbin.org/json")
print(response.json())

# POST with JSON data
response = requestx.post(
    "https://httpbin.org/post",
    json={"key": "value"}
)
print(response.status_code)
```

### Asynchronous API

```python
import asyncio
import requestx

async def main():
    async with requestx.AsyncClient() as client:
        response = await client.get("https://httpbin.org/json")
        print(response.json())

asyncio.run(main())
```

### Using Client Sessions

```python
import requestx

# Connection pooling with Client
with requestx.Client(base_url="https://api.example.com") as client:
    response = client.get("/users")
    users = response.json()
```

## Documentation Contents

- **[Quick Start](quickstart.md)** - Get up and running in minutes
- **[Installation](installation.md)** - Detailed installation instructions
- **[Configuration](configuration.md)** - Configure timeouts, proxies, and more
- **[API Reference](api/index.md)** - Complete API documentation
- **[Examples](examples/basic-usage.md)** - Code examples and patterns

## Community & Support

- **GitHub**: [https://github.com/neuesql/requestx](https://github.com/neuesql/requestx)
- **Issues**: [https://github.com/neuesql/requestx/issues](https://github.com/neuesql/requestx/issues)
- **Discussions**: [https://github.com/neuesql/requestx/discussions](https://github.com/neuesql/requestx/discussions)

## License

RequestX is released under the MIT License. See the [LICENSE](https://github.com/neuesql/requestx/blob/main/LICENSE) file for details.
