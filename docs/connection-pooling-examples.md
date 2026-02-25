# Connection Pooling Examples: httpx, requestx, urllib3, pycurl, aiohttp

This is an informational research document comparing connection pooling across Python HTTP clients.

---

## 1. httpx / requestx

Both use the same API. Connection pooling is configured via `Limits` class.

```python
import httpx

# Configure connection pool limits
limits = httpx.Limits(
    max_connections=100,           # Total concurrent connections
    max_keepalive_connections=20,  # Idle connections to keep alive
    keepalive_expiry=5.0           # Seconds before idle connection closes
)

# With pool timeout (wait for connection from pool)
timeout = httpx.Timeout(10.0, pool=2.0)  # 2 second pool timeout

# Sync client
with httpx.Client(limits=limits, timeout=timeout) as client:
    response = client.get("https://example.com")

# Async client
async with httpx.AsyncClient(limits=limits, timeout=timeout) as client:
    response = await client.get("https://example.com")
```

**Key parameters:**
- `max_connections`: Hard limit on concurrent connections (default: 100)
- `max_keepalive_connections`: Max idle connections kept alive (default: 20)
- `keepalive_expiry`: Idle timeout in seconds (default: 5.0)
- `Timeout(pool=...)`: Time to wait for a connection from pool

---

## 2. urllib3

Uses `PoolManager` for connection pooling across hosts.

```python
import urllib3

# Basic pool manager
http = urllib3.PoolManager(
    num_pools=10,        # Number of connection pools to cache (per host)
    maxsize=10,          # Max connections per pool
    block=False,         # If True, block when pool is full instead of creating new
    retries=3,           # Default retries
    timeout=30.0         # Default timeout
)

# Make requests - connections are pooled automatically
response = http.request("GET", "https://example.com/page1")
response = http.request("GET", "https://example.com/page2")  # Reuses connection

# For single host, use HTTPConnectionPool directly
pool = urllib3.HTTPConnectionPool(
    "example.com",
    port=443,
    maxsize=20,          # Max connections in this pool
    block=True           # Block when full
)
response = pool.request("GET", "/api/endpoint")
```

**Key parameters:**
- `num_pools`: Number of different host pools to cache (default: 10)
- `maxsize`: Max connections per pool (default: 1)
- `block`: If True, block when pool exhausted; if False, create temporary connection

---

## 3. aiohttp

Uses `TCPConnector` for async connection pooling.

```python
import aiohttp

# Create connector with pool limits
connector = aiohttp.TCPConnector(
    limit=100,              # Total concurrent connections (default: 100)
    limit_per_host=10,      # Connections per host (default: 0 = unlimited)
    ttl_dns_cache=300,      # DNS cache TTL in seconds
    keepalive_timeout=30,   # Idle connection timeout
    enable_cleanup_closed=True
)

# Use with ClientSession
async with aiohttp.ClientSession(connector=connector) as session:
    async with session.get("https://example.com") as response:
        data = await response.text()

# Connection pool is managed by the session
# Connections are reused for same host
```

**Key parameters:**
- `limit`: Total concurrent connections (default: 100)
- `limit_per_host`: Max connections per (host, port, ssl) triple (default: 0 = no limit)
- `keepalive_timeout`: How long to keep idle connections (default: 15 seconds)
- `force_close`: If True, close connections after each request

---

## 4. pycurl

Uses `CurlMulti` for connection pooling with multiple handles.

```python
import pycurl
from io import BytesIO

# Create multi handle (the connection pool manager)
multi = pycurl.CurlMulti()

# Configure pool size
multi.setopt(pycurl.M_MAXCONNECTS, 50)  # Max connections in pool

# Create and configure curl handles
def create_curl_handle(url):
    c = pycurl.Curl()
    buffer = BytesIO()
    c.setopt(pycurl.URL, url)
    c.setopt(pycurl.WRITEDATA, buffer)
    c.setopt(pycurl.FOLLOWLOCATION, True)
    c.setopt(pycurl.MAXREDIRS, 5)
    c.setopt(pycurl.CONNECTTIMEOUT, 30)
    c.setopt(pycurl.TIMEOUT, 300)
    # Enable keep-alive
    c.setopt(pycurl.TCP_KEEPALIVE, 1)
    c.setopt(pycurl.TCP_KEEPIDLE, 120)
    c.setopt(pycurl.TCP_KEEPINTVL, 60)
    # HTTP keep-alive header
    c.setopt(pycurl.HTTPHEADER, ['Connection: Keep-Alive', 'Keep-Alive: 300'])
    return c, buffer

# Add handles to multi for concurrent requests
handles = []
urls = ["https://example.com/1", "https://example.com/2", "https://example.com/3"]

for url in urls:
    c, buf = create_curl_handle(url)
    multi.add_handle(c)
    handles.append((c, buf))

# Perform requests
while True:
    ret, num_handles = multi.perform()
    if ret != pycurl.E_CALL_MULTI_PERFORM:
        break

# Wait for completion
while num_handles:
    multi.select(1.0)
    while True:
        ret, num_handles = multi.perform()
        if ret != pycurl.E_CALL_MULTI_PERFORM:
            break

# Read results and cleanup
for c, buf in handles:
    print(buf.getvalue())
    multi.remove_handle(c)
    c.close()

multi.close()
```

**Key options:**
- `M_MAXCONNECTS`: Max connections in the pool
- `TCP_KEEPALIVE`: Enable TCP keep-alive
- `HTTPHEADER`: Set `Connection: Keep-Alive` header
- Reuse `CurlMulti` object across requests to maintain connection pool

---

## Quick Comparison Table

| Library | Pool Class | Max Connections | Per-Host Limit | Keepalive |
|---------|------------|-----------------|----------------|-----------|
| httpx/requestx | `Limits` | `max_connections=100` | N/A | `keepalive_expiry=5.0` |
| urllib3 | `PoolManager` | `num_pools * maxsize` | `maxsize=1` | Built-in |
| aiohttp | `TCPConnector` | `limit=100` | `limit_per_host=0` | `keepalive_timeout=15` |
| pycurl | `CurlMulti` | `M_MAXCONNECTS` | N/A | `TCP_KEEPALIVE=1` |

---

## Sources

- [aiohttp Advanced Client Usage](https://docs.aiohttp.org/en/stable/client_advanced.html)
- [aiohttp Client Reference](https://docs.aiohttp.org/en/stable/client_reference.html)
- [urllib3 Pool Manager](https://urllib3.readthedocs.io/en/stable/reference/urllib3.poolmanager.html)
- [urllib3 Advanced Usage](https://urllib3.readthedocs.io/en/latest/advanced-usage.html)
- [pycurl CurlMulti Object](http://pycurl.io/docs/latest/curlmultiobject.html)
- [curl Connection Reuse](https://everything.curl.dev/transfers/conn/reuse.html)
- [CURLOPT_MAXCONNECTS](https://curl.se/libcurl/c/CURLOPT_MAXCONNECTS.html)
