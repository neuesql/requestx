# Exceptions

RequestX provides an HTTPX-compatible exception hierarchy for handling various error conditions.

## Exception Hierarchy

```
RequestError (base)
├── TransportError
│   ├── ConnectError
│   ├── ReadError
│   ├── WriteError
│   ├── CloseError
│   ├── ProxyError
│   ├── UnsupportedProtocol
│   └── ProtocolError
│       ├── LocalProtocolError
│       └── RemoteProtocolError
├── TimeoutException
│   ├── ConnectTimeout
│   ├── ReadTimeout
│   ├── WriteTimeout
│   └── PoolTimeout
├── HTTPStatusError
├── TooManyRedirects
├── DecodingError
├── InvalidURL
├── StreamError
│   ├── StreamConsumed
│   ├── StreamClosed
│   ├── ResponseNotRead
│   └── RequestNotRead
└── CookieConflict
```

## Base Exception

### RequestError

Base exception for all RequestX errors.

```python
from requestx import RequestError

try:
    response = requestx.get("https://invalid-url")
except RequestError as e:
    print(f"Request failed: {e}")
```

## Transport Errors

### TransportError

Base class for transport-level errors.

```python
from requestx import TransportError

try:
    response = requestx.get("https://example.com")
except TransportError as e:
    print(f"Transport error: {e}")
```

### ConnectError

Connection to the server failed.

```python
from requestx import ConnectError

try:
    response = requestx.get("https://nonexistent.example.com")
except ConnectError as e:
    print(f"Could not connect: {e}")
```

### ReadError

Error reading from the server.

```python
from requestx import ReadError

try:
    response = requestx.get("https://example.com/stream")
except ReadError as e:
    print(f"Read error: {e}")
```

### WriteError

Error writing to the server.

```python
from requestx import WriteError

try:
    response = requestx.post("https://example.com", data=large_data)
except WriteError as e:
    print(f"Write error: {e}")
```

### ProxyError

Error with proxy connection.

```python
from requestx import ProxyError

try:
    response = requestx.get(
        "https://example.com",
        proxy=requestx.Proxy("http://bad-proxy:8080")
    )
except ProxyError as e:
    print(f"Proxy error: {e}")
```

### UnsupportedProtocol

The protocol is not supported.

```python
from requestx import UnsupportedProtocol

try:
    response = requestx.get("ftp://example.com")
except UnsupportedProtocol as e:
    print(f"Unsupported protocol: {e}")
```

## Timeout Exceptions

### TimeoutException

Base class for all timeout errors.

```python
from requestx import TimeoutException

try:
    response = requestx.get("https://httpbin.org/delay/10", timeout=1.0)
except TimeoutException as e:
    print(f"Request timed out: {e}")
```

### ConnectTimeout

Timeout while establishing connection.

```python
from requestx import ConnectTimeout

try:
    response = requestx.get(
        "https://example.com",
        timeout=requestx.Timeout(connect=0.001)
    )
except ConnectTimeout as e:
    print(f"Connection timed out: {e}")
```

### ReadTimeout

Timeout while reading response.

```python
from requestx import ReadTimeout

try:
    response = requestx.get(
        "https://httpbin.org/delay/10",
        timeout=requestx.Timeout(read=1.0)
    )
except ReadTimeout as e:
    print(f"Read timed out: {e}")
```

### WriteTimeout

Timeout while sending request.

```python
from requestx import WriteTimeout

try:
    response = requestx.post(
        "https://example.com",
        data=large_data,
        timeout=requestx.Timeout(write=1.0)
    )
except WriteTimeout as e:
    print(f"Write timed out: {e}")
```

### PoolTimeout

Timeout waiting for a connection from the pool.

```python
from requestx import PoolTimeout

try:
    response = client.get(
        "https://example.com",
        timeout=requestx.Timeout(pool=1.0)
    )
except PoolTimeout as e:
    print(f"Pool timeout: {e}")
```

## HTTP Errors

### HTTPStatusError

HTTP 4xx or 5xx response received.

```python
from requestx import HTTPStatusError

try:
    response = requestx.get("https://httpbin.org/status/404")
    response.raise_for_status()
except HTTPStatusError as e:
    print(f"HTTP error: {e}")
    print(f"Status code: {e.response.status_code}")
    print(f"Response: {e.response.text}")
```

**Attributes:**

- `response`: The `Response` object

### TooManyRedirects

Exceeded the maximum number of redirects.

```python
from requestx import TooManyRedirects

try:
    with requestx.Client(max_redirects=5) as client:
        response = client.get("https://httpbin.org/redirect/10")
except TooManyRedirects as e:
    print(f"Too many redirects: {e}")
```

## Data Errors

### DecodingError

Failed to decode response content.

```python
from requestx import DecodingError

try:
    response = requestx.get("https://httpbin.org/html")
    data = response.json()  # HTML is not valid JSON
except DecodingError as e:
    print(f"Failed to decode: {e}")
```

### InvalidURL

The provided URL is invalid.

```python
from requestx import InvalidURL

try:
    response = requestx.get("not-a-valid-url")
except InvalidURL as e:
    print(f"Invalid URL: {e}")
```

## Stream Errors

### StreamError

Base class for streaming errors.

### StreamConsumed

The stream has already been consumed.

```python
from requestx import StreamConsumed

with client.stream("GET", url) as response:
    data = response.read()  # Consume the stream
    try:
        data = response.read()  # Try to read again
    except StreamConsumed as e:
        print(f"Stream already consumed: {e}")
```

### StreamClosed

The stream has been closed.

```python
from requestx import StreamClosed

response = client.stream("GET", url)
response.close()
try:
    for chunk in response.iter_bytes():
        pass
except StreamClosed as e:
    print(f"Stream closed: {e}")
```

## Error Handling Best Practices

### Catch Specific Exceptions

Handle specific exceptions for different error cases:

```python
import requestx
from requestx import (
    RequestError,
    HTTPStatusError,
    ConnectError,
    TimeoutException,
)

def fetch_data(url: str) -> dict:
    try:
        response = requestx.get(url, timeout=10.0)
        response.raise_for_status()
        return response.json()
    except ConnectError:
        print("Could not connect to server")
        raise
    except TimeoutException:
        print("Request timed out")
        raise
    except HTTPStatusError as e:
        if e.response.status_code == 404:
            print("Resource not found")
        elif e.response.status_code >= 500:
            print("Server error")
        raise
    except RequestError as e:
        print(f"Request failed: {e}")
        raise
```

### Retry on Transient Errors

Implement retry logic for transient failures:

```python
import time
import requestx
from requestx import ConnectError, TimeoutException

def fetch_with_retry(url: str, max_retries: int = 3) -> requestx.Response:
    last_error = None

    for attempt in range(max_retries):
        try:
            response = requestx.get(url, timeout=10.0)
            response.raise_for_status()
            return response
        except (ConnectError, TimeoutException) as e:
            last_error = e
            wait_time = 2 ** attempt  # Exponential backoff
            print(f"Attempt {attempt + 1} failed, retrying in {wait_time}s")
            time.sleep(wait_time)

    raise last_error
```

### Log Errors

Log errors for debugging:

```python
import logging
import requestx
from requestx import RequestError

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

def fetch_data(url: str):
    try:
        response = requestx.get(url)
        response.raise_for_status()
        return response.json()
    except RequestError as e:
        logger.error(f"Request to {url} failed: {e}", exc_info=True)
        raise
```
