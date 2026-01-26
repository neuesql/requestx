# Configuration Guide

RequestX provides flexible configuration options for timeouts, proxies, SSL, authentication, and more.

## Client Configuration

The `Client` and `AsyncClient` classes accept various configuration options:

```python
import requestx

client = requestx.Client(
    base_url="https://api.example.com",
    headers={"User-Agent": "MyApp/1.0"},
    cookies={"session": "abc123"},
    timeout=requestx.Timeout(timeout=30.0, connect=5.0),
    follow_redirects=True,
    max_redirects=10,
    verify_ssl=True,
    http2=True,
)
```

## Timeout Configuration

Configure timeouts using the `Timeout` class:

```python
import requestx

# Simple timeout (applies to all operations)
timeout = requestx.Timeout(timeout=30.0)

# Granular timeouts
timeout = requestx.Timeout(
    timeout=30.0,      # Total timeout
    connect=5.0,       # Connection timeout
    read=10.0,         # Read timeout
    write=10.0,        # Write timeout
    pool=5.0,          # Pool timeout
)

# Use with requests
response = requestx.get("https://httpbin.org/get", timeout=timeout)

# Use with client
with requestx.Client(timeout=timeout) as client:
    response = client.get("/endpoint")
```

### Timeout Values

| Parameter | Description | Default |
|-----------|-------------|---------|
| `timeout` | Total request timeout | None |
| `connect` | Connection establishment timeout | None |
| `read` | Time to wait for data | None |
| `write` | Time to wait for sending data | None |
| `pool` | Time to wait for a connection from pool | None |

## Headers Configuration

Set default headers for all requests:

```python
import requestx

# Using dict
headers = {"Authorization": "Bearer token", "User-Agent": "MyApp/1.0"}

# Using Headers class
headers = requestx.Headers({"Content-Type": "application/json"})
headers.set("X-Custom-Header", "value")

# Apply to client
with requestx.Client(headers=headers) as client:
    response = client.get("https://api.example.com/data")
```

## Cookies Configuration

Manage cookies across requests:

```python
import requestx

# Using dict
cookies = {"session": "abc123", "user": "john"}

# Using Cookies class
cookies = requestx.Cookies({"session": "abc123"})
cookies.set("preference", "dark_mode")

# Apply to client
with requestx.Client(cookies=cookies) as client:
    response = client.get("https://api.example.com/profile")
```

## Authentication

RequestX supports various authentication methods:

### Basic Authentication

```python
import requestx

auth = requestx.Auth.basic("username", "password")

response = requestx.get(
    "https://httpbin.org/basic-auth/user/pass",
    auth=auth
)
```

### Bearer Token Authentication

```python
import requestx

auth = requestx.Auth.bearer("your-api-token")

response = requestx.get(
    "https://api.example.com/protected",
    auth=auth
)
```

### Using with Client

```python
import requestx

with requestx.Client(auth=requestx.Auth.bearer("token")) as client:
    response = client.get("https://api.example.com/data")
```

## Proxy Configuration

Configure HTTP/HTTPS proxies:

```python
import requestx

# Single proxy for all protocols
proxy = requestx.Proxy(url="http://proxy.example.com:8080")

# Proxy with authentication
proxy = requestx.Proxy(
    url="http://proxy.example.com:8080",
    username="user",
    password="pass"
)

# Apply to client
with requestx.Client(proxy=proxy) as client:
    response = client.get("https://api.example.com/data")
```

## SSL/TLS Configuration

Configure SSL verification and certificates:

```python
import requestx

# Disable SSL verification (not recommended for production)
with requestx.Client(verify_ssl=False) as client:
    response = client.get("https://self-signed.example.com")

# Use custom CA bundle
with requestx.Client(ca_bundle="/path/to/ca-bundle.crt") as client:
    response = client.get("https://internal.example.com")

# Use client certificate
with requestx.Client(cert_file="/path/to/client.pem") as client:
    response = client.get("https://mtls.example.com")
```

## HTTP/2 Configuration

Enable HTTP/2 support:

```python
import requestx

# Enable HTTP/2
with requestx.Client(http2=True) as client:
    response = client.get("https://http2.example.com")
```

## Redirect Configuration

Control redirect behavior:

```python
import requestx

# Disable redirects
response = requestx.get(
    "https://httpbin.org/redirect/3",
    follow_redirects=False
)

# Limit redirects
with requestx.Client(
    follow_redirects=True,
    max_redirects=5
) as client:
    response = client.get("https://httpbin.org/redirect/3")
```

## Connection Limits

Configure connection pool limits:

```python
import requestx

limits = requestx.Limits(
    max_connections=100,
    max_keepalive_connections=20,
    keepalive_expiry=30.0,
)

with requestx.Client(limits=limits) as client:
    response = client.get("https://api.example.com/data")
```

## Environment Variables

RequestX can read configuration from environment variables when `trust_env=True`:

```python
import requestx

# Trust environment variables for proxy and SSL settings
with requestx.Client(trust_env=True) as client:
    response = client.get("https://api.example.com/data")
```

Supported environment variables:

| Variable | Description |
|----------|-------------|
| `HTTP_PROXY` | HTTP proxy URL |
| `HTTPS_PROXY` | HTTPS proxy URL |
| `NO_PROXY` | Comma-separated list of hosts to bypass proxy |
| `SSL_CERT_FILE` | Path to CA certificate bundle |

## Complete Example

```python
import requestx

# Full client configuration
client = requestx.Client(
    base_url="https://api.example.com",
    headers={
        "User-Agent": "MyApp/1.0",
        "Accept": "application/json",
    },
    cookies={"session": "abc123"},
    timeout=requestx.Timeout(
        timeout=30.0,
        connect=5.0,
        read=15.0,
    ),
    auth=requestx.Auth.bearer("api-token"),
    follow_redirects=True,
    max_redirects=10,
    verify_ssl=True,
    http2=True,
    trust_env=False,
)

with client:
    # All requests inherit the configuration
    users = client.get("/users").json()
    profile = client.get("/profile").json()

    # Override per-request
    response = client.post(
        "/upload",
        headers={"Content-Type": "multipart/form-data"},
        timeout=requestx.Timeout(timeout=120.0),
    )
```
