# RequestX

[![PyPI version](https://img.shields.io/pypi/v/requestx.svg)](https://pypi.org/project/requestx/)
[![Python versions](https://img.shields.io/pypi/pyversions/requestx.svg)](https://pypi.org/project/requestx/)
[![Build status](https://github.com/neuesql/requestx/workflows/Test%20and%20Build/badge.svg)](https://github.com/neuesql/requestx/actions)
[![Code style: black](https://img.shields.io/badge/code%20style-black-000000.svg)](https://github.com/psf/black)

RequestX is a high-performance HTTP client library for Python that provides a **drop-in replacement** for the popular `requests` library. Built with Rust for speed and memory safety, it offers both synchronous and asynchronous APIs while maintaining full compatibility with the familiar requests interface.

## 🚀 Key Features

* **Drop-in replacement** for requests library with identical API
* **High performance** leveraging Rust's speed and memory safety  
* **Dual API support** - both sync and async/await patterns
* **Cross-platform** compatibility (Windows, macOS, Linux)
* **Requests compatibility** for easy migration from existing codebases
* **Native async/await** support with automatic context detection
* **Session management** with persistent connections and cookies
* **Comprehensive error handling** with requests-compatible exceptions

## ⚡ Performance

RequestX delivers significant performance improvements over traditional Python HTTP libraries:

* **2-5x faster** than requests for synchronous operations
* **3-10x faster** than aiohttp for asynchronous operations  
* **Lower memory usage** due to Rust's efficient memory management
* **Better connection pooling** with hyper's advanced HTTP/2 support

## 📦 Installation

### Requirements

* **Python**: 3.8 or higher
* **Operating System**: Windows, macOS, or Linux
* **Architecture**: x86_64, ARM64 (Apple Silicon, ARM64 Windows)

No additional dependencies or build tools are required - RequestX comes with all Rust dependencies pre-compiled and bundled.

### Standard Installation

Install RequestX using pip:

```bash
pip install requestx
```



## 🚀 Quick Start

### Basic Usage

RequestX provides the exact same API as the popular `requests` library. If you're familiar with requests, you already know how to use RequestX!

```python
import requestx

# Make a simple GET request
response = requestx.get('https://httpbin.org/json')

# Check the status
print(f"Status: {response.status_code}")

# Get JSON data
data = response.json()
print(f"Data: {data}")
```

### Common HTTP Methods

```python
import requestx

# GET request
response = requestx.get('https://httpbin.org/get')

# POST request with JSON data
data = {'name': 'John Doe', 'email': 'john@example.com'}
response = requestx.post('https://httpbin.org/post', json=data)

# PUT request with form data
form_data = {'key': 'value'}
response = requestx.put('https://httpbin.org/put', data=form_data)

# DELETE request
response = requestx.delete('https://httpbin.org/delete')

# Custom headers
headers = {'Authorization': 'Bearer your-api-token'}
response = requestx.get('https://httpbin.org/headers', headers=headers)
```

### Session Usage

```python
import requestx

# Create a session for connection reuse
session = requestx.Session()

# Set default headers
session.headers.update({'Authorization': 'Bearer token'})

# Make requests using the session
response = session.get('https://httpbin.org/get')
print(response.status_code)
```

### Asynchronous Usage

RequestX automatically detects whether you're in a synchronous or asynchronous context:

```python
import asyncio
import requestx

# Synchronous context - runs immediately
def sync_function():
    response = requestx.get('https://httpbin.org/json')
    return response.json()

# Asynchronous context - returns awaitable
async def async_function():
    response = await requestx.get('https://httpbin.org/json')
    return response.json()

# Usage
sync_data = sync_function()  # Immediate result
async_data = asyncio.run(async_function())  # Awaitable result
```

### Concurrent Async Requests

```python
import asyncio
import requestx

async def fetch_url(url):
    response = await requestx.get(url)
    return response.json()

async def main():
    urls = [
        'https://httpbin.org/delay/1',
        'https://httpbin.org/delay/2',
        'https://httpbin.org/delay/3'
    ]
    
    # Run requests concurrently
    results = await asyncio.gather(*[fetch_url(url) for url in urls])
    return results

# Execute
results = asyncio.run(main())
```

## 🔄 Migration from Requests

RequestX is designed as a **drop-in replacement** for requests. The easiest way to migrate is to simply change your import statement:

**Before (requests):**
```python
import requests

response = requests.get('https://api.example.com/data')
print(response.json())
```

**After (requestx):**
```python
import requestx as requests  # Drop-in replacement

response = requests.get('https://api.example.com/data')
print(response.json())
```

Or use RequestX directly:
```python
import requestx

response = requestx.get('https://api.example.com/data')
print(response.json())
```

## 🏗️ Development

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
# Run Python tests
uv run pytest

# Run Rust tests
cargo test
```

### Building

```bash
# Build wheel
uv run maturin build --release

# Build and install locally
uv run maturin develop --release
```





## 📄 License

MIT License - see LICENSE file for details.

## 📧 Contact

For questions, issues, or contributions, please contact: **wu.qunfei@gmail.com**

## 🤝 Contributing

We welcome contributions! Please see our contributing guidelines for more information on how to get started.

## 📚 Documentation

For comprehensive documentation, examples, and advanced usage patterns, visit our [documentation site](https://requestx.readthedocs.io/).