# RequestX

High-performance Python HTTP client, API-compatible with httpx, powered by Rust's reqwest via PyO3.

## Installation

```bash
pip install requestx
```

## Usage

```python
import requestx

# Synchronous requests
response = requestx.get("https://httpbin.org/get")
print(response.json())

# Async requests
import asyncio

async def main():
    async with requestx.AsyncClient() as client:
        response = await client.get("https://httpbin.org/get")
        print(response.json())

asyncio.run(main())
```

## Features

- Drop-in replacement for httpx
- Powered by Rust's reqwest for high performance
- Full support for HTTP/1.1 and HTTP/2
- SIMD-accelerated JSON parsing via sonic-rs
- Compression support: gzip, brotli, deflate, zstd

## License

MIT
