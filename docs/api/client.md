# Client Classes

RequestX provides `Client` and `AsyncClient` classes for making HTTP requests with connection pooling and shared configuration.

## Client

The synchronous HTTP client.

### Constructor

```python
requestx.Client(
    base_url=None,
    headers=None,
    cookies=None,
    timeout=None,
    auth=None,
    proxy=None,
    follow_redirects=True,
    max_redirects=20,
    verify_ssl=True,
    ca_bundle=None,
    cert_file=None,
    http2=False,
    trust_env=True,
    limits=None,
)
```

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `base_url` | `str` | `None` | Base URL for relative requests |
| `headers` | `dict` | `None` | Default headers for all requests |
| `cookies` | `dict` | `None` | Default cookies for all requests |
| `timeout` | `Timeout` | `None` | Default timeout configuration |
| `auth` | `Auth` | `None` | Default authentication |
| `proxy` | `Proxy` | `None` | Proxy configuration |
| `follow_redirects` | `bool` | `True` | Follow HTTP redirects |
| `max_redirects` | `int` | `20` | Maximum number of redirects |
| `verify_ssl` | `bool` | `True` | Verify SSL certificates |
| `ca_bundle` | `str` | `None` | Path to CA certificate bundle |
| `cert_file` | `str` | `None` | Path to client certificate |
| `http2` | `bool` | `False` | Enable HTTP/2 |
| `trust_env` | `bool` | `True` | Read settings from environment |
| `limits` | `Limits` | `None` | Connection pool limits |

### Methods

All HTTP methods are available:

```python
client.get(url, **kwargs) -> Response
client.post(url, data=None, json=None, **kwargs) -> Response
client.put(url, data=None, json=None, **kwargs) -> Response
client.patch(url, data=None, json=None, **kwargs) -> Response
client.delete(url, **kwargs) -> Response
client.head(url, **kwargs) -> Response
client.options(url, **kwargs) -> Response
client.request(method, url, **kwargs) -> Response
```

### Streaming

```python
client.stream(method, url, **kwargs) -> StreamingResponse
```

### Context Manager

```python
with requestx.Client() as client:
    response = client.get("https://httpbin.org/get")
# Client is automatically closed
```

### Manual Lifecycle

```python
client = requestx.Client()
try:
    response = client.get("https://httpbin.org/get")
finally:
    client.close()
```

### Example

```python
import requestx

# Basic usage with context manager
with requestx.Client() as client:
    response = client.get("https://httpbin.org/get")
    print(response.json())

# With configuration
with requestx.Client(
    base_url="https://api.example.com",
    headers={"Authorization": "Bearer token"},
    timeout=requestx.Timeout(timeout=30.0),
) as client:
    users = client.get("/users").json()
    user = client.get("/users/1").json()
    client.post("/users", json={"name": "John"})
```

## AsyncClient

The asynchronous HTTP client.

### Constructor

Same parameters as `Client`:

```python
requestx.AsyncClient(
    base_url=None,
    headers=None,
    cookies=None,
    timeout=None,
    auth=None,
    proxy=None,
    follow_redirects=True,
    max_redirects=20,
    verify_ssl=True,
    ca_bundle=None,
    cert_file=None,
    http2=False,
    trust_env=True,
    limits=None,
)
```

### Methods

All HTTP methods are async:

```python
await client.get(url, **kwargs) -> Response
await client.post(url, data=None, json=None, **kwargs) -> Response
await client.put(url, data=None, json=None, **kwargs) -> Response
await client.patch(url, data=None, json=None, **kwargs) -> Response
await client.delete(url, **kwargs) -> Response
await client.head(url, **kwargs) -> Response
await client.options(url, **kwargs) -> Response
await client.request(method, url, **kwargs) -> Response
```

### Streaming

```python
await client.stream(method, url, **kwargs) -> AsyncStreamingResponse
```

### Async Context Manager

```python
async with requestx.AsyncClient() as client:
    response = await client.get("https://httpbin.org/get")
# Client is automatically closed
```

### Manual Lifecycle

```python
client = requestx.AsyncClient()
try:
    response = await client.get("https://httpbin.org/get")
finally:
    await client.aclose()
```

### Example

```python
import asyncio
import requestx

async def main():
    # Basic usage
    async with requestx.AsyncClient() as client:
        response = await client.get("https://httpbin.org/get")
        print(response.json())

    # With configuration
    async with requestx.AsyncClient(
        base_url="https://api.example.com",
        headers={"Authorization": "Bearer token"},
        timeout=requestx.Timeout(timeout=30.0),
    ) as client:
        users = (await client.get("/users")).json()
        user = (await client.get("/users/1")).json()

asyncio.run(main())
```

## Configuration Classes

### Timeout

Configure request timeouts.

```python
requestx.Timeout(
    timeout=None,   # Total timeout in seconds
    connect=None,   # Connection timeout
    read=None,      # Read timeout
    write=None,     # Write timeout
    pool=None,      # Pool timeout
)
```

**Example:**

```python
timeout = requestx.Timeout(
    timeout=30.0,
    connect=5.0,
    read=15.0,
)

with requestx.Client(timeout=timeout) as client:
    response = client.get("https://httpbin.org/delay/2")
```

### Proxy

Configure HTTP/HTTPS proxy.

```python
requestx.Proxy(
    url,            # Proxy URL
    username=None,  # Proxy username
    password=None,  # Proxy password
)
```

**Example:**

```python
proxy = requestx.Proxy(
    url="http://proxy.example.com:8080",
    username="user",
    password="pass",
)

with requestx.Client(proxy=proxy) as client:
    response = client.get("https://httpbin.org/get")
```

### Auth

Configure authentication.

```python
# Basic authentication
requestx.Auth.basic(username, password)

# Bearer token authentication
requestx.Auth.bearer(token)
```

**Example:**

```python
# Basic auth
auth = requestx.Auth.basic("user", "pass")

# Bearer token
auth = requestx.Auth.bearer("your-api-token")

with requestx.Client(auth=auth) as client:
    response = client.get("https://api.example.com/protected")
```

### Headers

Case-insensitive header dictionary.

```python
headers = requestx.Headers({"Content-Type": "application/json"})
headers.set("X-Custom", "value")
value = headers.get("content-type")  # Case-insensitive
```

### Cookies

Cookie container.

```python
cookies = requestx.Cookies({"session": "abc123"})
cookies.set("user", "john")
value = cookies.get("session")
```

### Limits

Connection pool limits.

```python
requestx.Limits(
    max_connections=100,
    max_keepalive_connections=20,
    keepalive_expiry=30.0,
)
```

## Best Practices

### Reuse Clients

Create a client once and reuse it:

```python
# Good
with requestx.Client() as client:
    for i in range(100):
        response = client.get(f"https://api.example.com/item/{i}")

# Bad - creates new connections each time
for i in range(100):
    response = requestx.get(f"https://api.example.com/item/{i}")
```

### Use Base URL

Set a base URL for cleaner code:

```python
with requestx.Client(base_url="https://api.example.com/v1") as client:
    users = client.get("/users").json()
    posts = client.get("/posts").json()
```

### Configure Once

Set common configuration at client level:

```python
with requestx.Client(
    base_url="https://api.example.com",
    headers={"Authorization": "Bearer token"},
    timeout=requestx.Timeout(timeout=30.0),
) as client:
    # All requests inherit the configuration
    response = client.get("/data")
```

### Handle Errors

Always handle potential errors:

```python
import requestx
from requestx import RequestError, HTTPStatusError

with requestx.Client() as client:
    try:
        response = client.get("https://api.example.com/data")
        response.raise_for_status()
        data = response.json()
    except HTTPStatusError as e:
        print(f"HTTP error: {e.response.status_code}")
    except RequestError as e:
        print(f"Request failed: {e}")
```
