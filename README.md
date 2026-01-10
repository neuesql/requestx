# RequestX

RequestX is a high-performance HTTP client library for Python that provides a drop-in replacement for the popular `requests` library. Built with Rust for speed and memory safety, it offers both synchronous and asynchronous APIs while maintaining full compatibility with the familiar requests interface.

## Features

- Drop-in replacement for requests library with identical API
- High performance leveraging Rust's speed and memory safety  
- Dual API support - both sync and async/await patterns
- Cross-platform compatibility (Windows, macOS, Linux)
- Requests compatibility for easy migration from existing codebases
- Native async/await support with automatic context detection
- Session management with persistent connections and cookies
- Comprehensive error handling with requests-compatible exceptions

## Installation

```bash
pip install requestx
```

## Quick Start

```python
import requestx

# Make a simple GET request
response = requestx.get('https://httpbin.org/json')
print(f"Status: {response.status_code}")
print(f"Data: {response.json()}")
```

## Development

This project uses:
- Rust for the core HTTP implementation
- PyO3 for Python bindings
- maturin for building and packaging
- uv for Python dependency management

## License

MIT License