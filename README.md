# RequestX

High-performance HTTP client for Python with requests-compatible API, powered by Rust.

## Features

- **Drop-in replacement** for the popular `requests` library
- **High performance** leveraging Rust's speed and memory safety
- **Async/await support** with native Python asyncio integration
- **Same familiar API** as requests for easy migration
- **Cross-platform** support (Windows, macOS, Linux)

## Installation

```bash
pip install requestx
```

## Quick Start

### Synchronous Usage

```python
import requestx

# Same API as requests
response = requestx.get('https://httpbin.org/get')
print(response.status_code)
print(response.json())
```

### Asynchronous Usage

```python
import asyncio
import requestx

async def main():
    # Same function, just use await
    response = await requestx.get('https://httpbin.org/get')
    print(response.status_code)
    print(response.json())

asyncio.run(main())
```

### Session Usage

```python
import requestx

session = requestx.Session()
response = session.get('https://httpbin.org/get')
print(response.status_code)
```

## Development

This project uses:
- **Rust** for the core HTTP implementation
- **PyO3** for Python bindings
- **maturin** for building and packaging
- **uv** for Python dependency management

### Setup Development Environment

```bash
# Install uv for Python dependency management
curl -LsSf https://astral.sh/uv/install.sh | sh

# Install development dependencies
uv sync --dev

# Build the extension
uv run maturin develop
```

### Running Tests

```bash
uv run pytest
```

## License

MIT License - see LICENSE file for details.