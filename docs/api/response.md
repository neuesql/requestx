# Response Object

The `Response` class represents an HTTP response from a server.

## Properties

### status_code

The HTTP status code as an integer.

```python
response = requestx.get("https://httpbin.org/status/200")
print(response.status_code)  # 200
```

### reason_phrase

The HTTP reason phrase.

```python
response = requestx.get("https://httpbin.org/status/404")
print(response.reason_phrase)  # "Not Found"
```

### headers

Response headers as a `Headers` object (case-insensitive).

```python
response = requestx.get("https://httpbin.org/get")
print(response.headers.get("content-type"))  # "application/json"
print(response.headers.get("Content-Type"))  # Same result
```

### url

The final URL after any redirects.

```python
response = requestx.get("https://httpbin.org/redirect/1")
print(response.url)  # "https://httpbin.org/get"
```

### content

The response body as bytes.

```python
response = requestx.get("https://httpbin.org/bytes/100")
print(len(response.content))  # 100
print(type(response.content))  # <class 'bytes'>
```

### text

The response body decoded as a string.

```python
response = requestx.get("https://httpbin.org/html")
print(response.text)  # HTML content as string
```

### cookies

Response cookies as a `Cookies` object.

```python
response = requestx.get("https://httpbin.org/cookies/set/name/value")
print(response.cookies.get("name"))  # "value"
```

### elapsed

Time elapsed for the request in seconds.

```python
response = requestx.get("https://httpbin.org/delay/1")
print(f"Request took {response.elapsed:.2f} seconds")
```

### http_version

The HTTP version used for the response.

```python
response = requestx.get("https://httpbin.org/get")
print(response.http_version)  # "HTTP/1.1" or "HTTP/2"
```

## Status Check Properties

### is_success

`True` if the status code is 2xx.

```python
response = requestx.get("https://httpbin.org/status/200")
print(response.is_success)  # True

response = requestx.get("https://httpbin.org/status/404")
print(response.is_success)  # False
```

### is_redirect

`True` if the status code is 3xx.

```python
response = requestx.get(
    "https://httpbin.org/redirect/1",
    follow_redirects=False
)
print(response.is_redirect)  # True
```

### is_client_error

`True` if the status code is 4xx.

```python
response = requestx.get("https://httpbin.org/status/404")
print(response.is_client_error)  # True
```

### is_server_error

`True` if the status code is 5xx.

```python
response = requestx.get("https://httpbin.org/status/500")
print(response.is_server_error)  # True
```

### is_error

`True` if the status code is 4xx or 5xx.

```python
response = requestx.get("https://httpbin.org/status/404")
print(response.is_error)  # True
```

## Methods

### json()

Parse the response body as JSON.

```python
Response.json() -> dict | list
```

**Returns:** Parsed JSON data

**Raises:** `DecodingError` if the response is not valid JSON

**Example:**

```python
response = requestx.get("https://httpbin.org/json")
data = response.json()
print(type(data))  # <class 'dict'>
```

### raise_for_status()

Raise an exception for 4xx/5xx status codes.

```python
Response.raise_for_status() -> None
```

**Raises:** `HTTPStatusError` for 4xx/5xx responses

**Example:**

```python
import requestx
from requestx import HTTPStatusError

response = requestx.get("https://httpbin.org/status/404")

try:
    response.raise_for_status()
except HTTPStatusError as e:
    print(f"Error: {e}")
    print(f"Status: {e.response.status_code}")
```

## Boolean Conversion

Response objects can be used in boolean contexts. Returns `True` for successful responses (2xx).

```python
response = requestx.get("https://httpbin.org/get")
if response:
    print("Success!")

response = requestx.get("https://httpbin.org/status/404")
if not response:
    print("Request failed")
```

## Complete Example

```python
import requestx

response = requestx.get("https://httpbin.org/json")

# Check status
print(f"Status: {response.status_code} {response.reason_phrase}")
print(f"Success: {response.is_success}")

# Access headers
print(f"Content-Type: {response.headers.get('content-type')}")
print(f"Content-Length: {response.headers.get('content-length')}")

# Get content
print(f"Text length: {len(response.text)}")
print(f"Bytes length: {len(response.content)}")

# Parse JSON
data = response.json()
print(f"JSON data: {data}")

# Timing
print(f"Elapsed: {response.elapsed:.3f}s")

# URL info
print(f"URL: {response.url}")
print(f"HTTP Version: {response.http_version}")

# Error handling
try:
    response.raise_for_status()
    print("No errors!")
except requestx.HTTPStatusError as e:
    print(f"HTTP Error: {e}")
```

## Headers Class

The `Headers` class provides case-insensitive access to HTTP headers.

### get(name, default=None)

Get a header value by name.

```python
content_type = response.headers.get("content-type")
custom = response.headers.get("x-custom", "default")
```

### keys()

Get all header names.

```python
for name in response.headers.keys():
    print(name)
```

### values()

Get all header values.

```python
for value in response.headers.values():
    print(value)
```

### items()

Get all header name-value pairs.

```python
for name, value in response.headers.items():
    print(f"{name}: {value}")
```

## Cookies Class

The `Cookies` class provides access to response cookies.

### get(name, default=None)

Get a cookie value by name.

```python
session = response.cookies.get("session")
```

### keys()

Get all cookie names.

```python
for name in response.cookies.keys():
    print(name)
```

### items()

Get all cookie name-value pairs.

```python
for name, value in response.cookies.items():
    print(f"{name}={value}")
```
