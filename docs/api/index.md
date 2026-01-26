# API Reference

This section contains the complete API reference for RequestX.

## Overview

RequestX provides a simple, intuitive API that's compatible with HTTPX. The API is organized into several main components:

| Component | Description |
|-----------|-------------|
| **HTTP Functions** | Top-level functions for making HTTP requests |
| **Client Classes** | `Client` and `AsyncClient` for persistent connections |
| **Response Object** | The `Response` class for HTTP responses |
| **Exceptions** | Exception classes for error handling |

## Quick Reference

### Making Requests

```python
import requestx

# Module-level functions
response = requestx.get(url, **kwargs)
response = requestx.post(url, data=None, json=None, **kwargs)
response = requestx.put(url, data=None, **kwargs)
response = requestx.patch(url, data=None, **kwargs)
response = requestx.delete(url, **kwargs)
response = requestx.head(url, **kwargs)
response = requestx.options(url, **kwargs)
```

### Common Parameters

```python
requestx.get(
    url,
    params=None,           # URL query parameters
    headers=None,          # HTTP headers
    cookies=None,          # Cookies to send
    auth=None,             # Authentication
    timeout=None,          # Request timeout
    follow_redirects=True, # Follow redirects
)
```

### Response Properties

```python
response.status_code     # HTTP status code (int)
response.headers         # Response headers (Headers)
response.text            # Response text (str)
response.content         # Response bytes (bytes)
response.json()          # Parse JSON response (dict/list)
response.url             # Final URL (str)
response.cookies         # Response cookies (Cookies)
response.elapsed         # Request duration (float)
response.http_version    # HTTP version (str)
```

### Status Checks

```python
response.is_success      # True for 2xx status
response.is_redirect     # True for 3xx status
response.is_client_error # True for 4xx status
response.is_server_error # True for 5xx status
response.is_error        # True for 4xx or 5xx
```

### Client Usage

```python
# Synchronous client
with requestx.Client(base_url="https://api.example.com") as client:
    response = client.get("/users")

# Asynchronous client
async with requestx.AsyncClient() as client:
    response = await client.get("https://api.example.com/users")
```

### Error Handling

```python
from requestx import (
    RequestError,
    HTTPStatusError,
    ConnectError,
    TimeoutException,
)

try:
    response = requestx.get(url, timeout=10)
    response.raise_for_status()
except HTTPStatusError as e:
    print(f"HTTP error: {e}")
except ConnectError as e:
    print(f"Connection error: {e}")
except TimeoutException as e:
    print(f"Timeout: {e}")
except RequestError as e:
    print(f"Request error: {e}")
```

## Module Contents

### Classes

| Class | Description |
|-------|-------------|
| `Client` | Synchronous HTTP client with connection pooling |
| `AsyncClient` | Asynchronous HTTP client |
| `Response` | HTTP response object |
| `Headers` | Case-insensitive header dictionary |
| `Cookies` | Cookie jar |
| `Timeout` | Timeout configuration |
| `Proxy` | Proxy configuration |
| `Auth` | Authentication configuration |
| `Limits` | Connection limits configuration |

### Functions

| Function | Description |
|----------|-------------|
| `get()` | Send a GET request |
| `post()` | Send a POST request |
| `put()` | Send a PUT request |
| `patch()` | Send a PATCH request |
| `delete()` | Send a DELETE request |
| `head()` | Send a HEAD request |
| `options()` | Send an OPTIONS request |
| `request()` | Send a request with custom method |

### Exceptions

| Exception | Description |
|-----------|-------------|
| `RequestError` | Base exception for all request errors |
| `TransportError` | Transport-level errors |
| `ConnectError` | Connection establishment failed |
| `TimeoutException` | Request timed out |
| `HTTPStatusError` | HTTP 4xx/5xx response |
| `TooManyRedirects` | Exceeded redirect limit |
| `DecodingError` | Response decoding failed |
| `InvalidURL` | Invalid URL provided |

## Detailed Reference

- [HTTP Functions](functions.md) - Module-level request functions
- [Response Object](response.md) - Response class and properties
- [Client Classes](client.md) - Client and AsyncClient
- [Exceptions](exceptions.md) - Exception hierarchy
