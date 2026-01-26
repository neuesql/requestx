# Quick Start Guide

This guide will get you up and running with RequestX in just a few minutes.

## Installation

Install RequestX using pip:

```bash
pip install requestx
```

That's it! RequestX comes with all dependencies bundled, so no additional setup is required.

## Basic Usage

RequestX provides a familiar API similar to HTTPX. If you're familiar with HTTPX or requests, you already know how to use RequestX!

### Making Your First Request

```python
import requestx

# Make a simple GET request
response = requestx.get("https://httpbin.org/json")

# Check the status
print(f"Status: {response.status_code}")

# Get JSON data
data = response.json()
print(f"Data: {data}")
```

### Common HTTP Methods

RequestX supports all standard HTTP methods:

```python
import requestx

# GET request
response = requestx.get("https://httpbin.org/get")

# POST request with JSON data
response = requestx.post("https://httpbin.org/post", json={"key": "value"})

# PUT request
response = requestx.put("https://httpbin.org/put", json={"updated": True})

# DELETE request
response = requestx.delete("https://httpbin.org/delete")

# HEAD request
response = requestx.head("https://httpbin.org/get")

# OPTIONS request
response = requestx.options("https://httpbin.org/get")

# PATCH request
response = requestx.patch("https://httpbin.org/patch", json={"patched": True})
```

### Working with Query Parameters

Add URL parameters using the `params` argument:

```python
import requestx

params = {"key1": "value1", "key2": "value2"}
response = requestx.get("https://httpbin.org/get", params=params)

# This makes a request to: https://httpbin.org/get?key1=value1&key2=value2
print(response.url)
```

### Sending Data

Send data in various formats:

```python
import requestx

# Send form data
data = {"username": "user", "password": "pass"}
response = requestx.post("https://httpbin.org/post", data=data)

# Send JSON data
json_data = {"name": "John", "age": 30}
response = requestx.post("https://httpbin.org/post", json=json_data)
```

### Custom Headers

Add custom headers to your requests:

```python
import requestx

headers = {
    "User-Agent": "RequestX/1.0",
    "Authorization": "Bearer your-token-here",
    "Content-Type": "application/json"
}

response = requestx.get("https://httpbin.org/headers", headers=headers)
```

## Response Handling

Work with response data:

```python
import requestx

response = requestx.get("https://httpbin.org/json")

# Status code
print(f"Status: {response.status_code}")

# Response headers
print(f"Content-Type: {response.headers.get('content-type')}")

# Text content
print(f"Text: {response.text}")

# JSON content
data = response.json()
print(f"JSON: {data}")

# Raw bytes
print(f"Content length: {len(response.content)} bytes")

# Check response status
print(f"Success: {response.is_success}")
print(f"Is error: {response.is_error}")
```

## Error Handling

Handle errors gracefully:

```python
import requestx
from requestx import RequestError, HTTPStatusError, ConnectError, TimeoutException

try:
    response = requestx.get("https://httpbin.org/status/404")
    response.raise_for_status()  # Raises HTTPStatusError for 4xx/5xx
except HTTPStatusError as e:
    print(f"HTTP Error: {e}")
except ConnectError as e:
    print(f"Connection Error: {e}")
except TimeoutException as e:
    print(f"Timeout Error: {e}")
except RequestError as e:
    print(f"Request Error: {e}")
```

## Async/Await Support

RequestX provides native async support with `AsyncClient`:

```python
import asyncio
import requestx

async def fetch_data():
    async with requestx.AsyncClient() as client:
        response = await client.get("https://httpbin.org/json")
        return response.json()

async def main():
    data = await fetch_data()
    print(f"Received: {data}")

asyncio.run(main())
```

## Using Client Sessions

Use `Client` for better performance when making multiple requests:

```python
import requestx

# Sync client with connection pooling
with requestx.Client() as client:
    # Set default headers for all requests
    response1 = client.get("https://httpbin.org/get")
    response2 = client.get("https://httpbin.org/json")
    response3 = client.post("https://httpbin.org/post", json={"data": "value"})

# Client with base URL
with requestx.Client(base_url="https://api.example.com") as client:
    response = client.get("/users")  # Requests https://api.example.com/users
```

## Next Steps

Now that you've learned the basics, explore more advanced features:

- [Installation Guide](installation.md) - Detailed installation options
- [Configuration](configuration.md) - Timeouts, proxies, SSL settings
- [Async Guide](async-guide.md) - Deep dive into async/await usage
- [API Reference](api/index.md) - Complete API documentation
- [Examples](examples/basic-usage.md) - More code examples

## Performance Tips

To get the best performance from RequestX:

1. **Use Client sessions** for multiple requests to the same host
2. **Enable connection pooling** by reusing Client objects
3. **Use async/await** for I/O-bound operations
4. **Set appropriate timeouts** to avoid hanging requests

```python
import requestx

# Good: Reuse client for multiple requests
with requestx.Client() as client:
    for i in range(10):
        response = client.get(f"https://api.example.com/item/{i}")
        process_response(response)
```
