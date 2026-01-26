# HTTP Functions

RequestX provides top-level functions for making HTTP requests.

## get

Send a GET request.

```python
requestx.get(url, params=None, **kwargs) -> Response
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `url` | `str` | URL for the request |
| `params` | `dict` | URL query parameters |
| `headers` | `dict` | HTTP headers |
| `cookies` | `dict` | Cookies to send |
| `auth` | `Auth` | Authentication |
| `timeout` | `Timeout` | Request timeout |
| `follow_redirects` | `bool` | Follow redirects (default: True) |

**Returns:** `Response` object

**Example:**

```python
import requestx

# Simple GET
response = requestx.get("https://httpbin.org/get")

# With parameters
response = requestx.get(
    "https://httpbin.org/get",
    params={"key": "value"},
    headers={"Accept": "application/json"},
)
```

## post

Send a POST request.

```python
requestx.post(url, data=None, json=None, **kwargs) -> Response
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `url` | `str` | URL for the request |
| `data` | `dict/bytes` | Form data or raw bytes |
| `json` | `dict/list` | JSON data (auto-serialized) |
| `content` | `bytes` | Raw content |
| `headers` | `dict` | HTTP headers |
| `timeout` | `Timeout` | Request timeout |

**Returns:** `Response` object

**Example:**

```python
import requestx

# POST with JSON
response = requestx.post(
    "https://httpbin.org/post",
    json={"name": "John", "age": 30}
)

# POST with form data
response = requestx.post(
    "https://httpbin.org/post",
    data={"username": "john", "password": "secret"}
)
```

## put

Send a PUT request.

```python
requestx.put(url, data=None, json=None, **kwargs) -> Response
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `url` | `str` | URL for the request |
| `data` | `dict/bytes` | Form data or raw bytes |
| `json` | `dict/list` | JSON data |
| `headers` | `dict` | HTTP headers |
| `timeout` | `Timeout` | Request timeout |

**Returns:** `Response` object

**Example:**

```python
import requestx

response = requestx.put(
    "https://httpbin.org/put",
    json={"updated": True}
)
```

## patch

Send a PATCH request.

```python
requestx.patch(url, data=None, json=None, **kwargs) -> Response
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `url` | `str` | URL for the request |
| `data` | `dict/bytes` | Form data or raw bytes |
| `json` | `dict/list` | JSON data |
| `headers` | `dict` | HTTP headers |
| `timeout` | `Timeout` | Request timeout |

**Returns:** `Response` object

**Example:**

```python
import requestx

response = requestx.patch(
    "https://httpbin.org/patch",
    json={"field": "new_value"}
)
```

## delete

Send a DELETE request.

```python
requestx.delete(url, **kwargs) -> Response
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `url` | `str` | URL for the request |
| `headers` | `dict` | HTTP headers |
| `timeout` | `Timeout` | Request timeout |

**Returns:** `Response` object

**Example:**

```python
import requestx

response = requestx.delete("https://httpbin.org/delete")
```

## head

Send a HEAD request.

```python
requestx.head(url, **kwargs) -> Response
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `url` | `str` | URL for the request |
| `headers` | `dict` | HTTP headers |
| `timeout` | `Timeout` | Request timeout |
| `follow_redirects` | `bool` | Follow redirects |

**Returns:** `Response` object (with empty body)

**Example:**

```python
import requestx

response = requestx.head("https://httpbin.org/get")
print(f"Content-Length: {response.headers.get('content-length')}")
```

## options

Send an OPTIONS request.

```python
requestx.options(url, **kwargs) -> Response
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `url` | `str` | URL for the request |
| `headers` | `dict` | HTTP headers |
| `timeout` | `Timeout` | Request timeout |

**Returns:** `Response` object

**Example:**

```python
import requestx

response = requestx.options("https://httpbin.org/get")
print(f"Allowed: {response.headers.get('allow')}")
```

## request

Send a request with a custom HTTP method.

```python
requestx.request(method, url, **kwargs) -> Response
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `method` | `str` | HTTP method (GET, POST, etc.) |
| `url` | `str` | URL for the request |
| `**kwargs` | | Same as other methods |

**Returns:** `Response` object

**Example:**

```python
import requestx

# Custom method
response = requestx.request("CUSTOM", "https://api.example.com/endpoint")

# Equivalent to requestx.get()
response = requestx.request("GET", "https://httpbin.org/get")
```

## Common Parameters

All functions accept these common parameters:

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `params` | `dict` | `None` | URL query parameters |
| `headers` | `dict` | `None` | HTTP headers |
| `cookies` | `dict` | `None` | Cookies to send |
| `auth` | `Auth` | `None` | Authentication |
| `timeout` | `Timeout/float` | `None` | Request timeout |
| `follow_redirects` | `bool` | `True` | Follow HTTP redirects |

## Timeout Examples

```python
import requestx

# Simple timeout (seconds)
response = requestx.get("https://httpbin.org/get", timeout=10.0)

# Detailed timeout configuration
timeout = requestx.Timeout(
    timeout=30.0,   # Total timeout
    connect=5.0,    # Connection timeout
    read=10.0,      # Read timeout
)
response = requestx.get("https://httpbin.org/get", timeout=timeout)
```

## Authentication Examples

```python
import requestx

# Basic auth
response = requestx.get(
    "https://httpbin.org/basic-auth/user/pass",
    auth=requestx.Auth.basic("user", "pass")
)

# Bearer token
response = requestx.get(
    "https://api.example.com/data",
    auth=requestx.Auth.bearer("your-token")
)
```
