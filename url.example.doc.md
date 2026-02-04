# RequestX URL Implementation Guide

This document explains the complete HTTPX-compatible URL implementation in Rust with PyO3 bindings for RequestX.

## Overview

The URL implementation provides full compatibility with `httpx.URL`, including:

- **URL Parsing**: Complete RFC 3986 compliant parsing
- **IDNA Support**: Internationalized domain name handling (punycode encoding)
- **Percent Encoding**: Proper encoding/decoding for all URL components
- **Path Normalization**: Resolving `.` and `..` segments
- **IPv4/IPv6 Support**: Full address validation and handling
- **Query Parameters**: Manipulation via `QueryParams` and form-urlencoding
- **URL Joining**: RFC 3986 compliant URL resolution
- **copy_with()**: Immutable URL modifications

## API Reference

### Constructor

```python
# From string
url = URL("https://example.com/path?query=value#fragment")

# From components
url = URL(scheme="https", host="example.com", path="/", params={"key": "value"})

# From existing URL with modifications
url = URL("https://example.com", params={"a": "123"})
```

### Properties

| Property | Type | Description |
|----------|------|-------------|
| `scheme` | `str` | URL scheme (e.g., "https") |
| `host` | `str` | Decoded host (e.g., "中国.icom.museum") |
| `raw_host` | `bytes` | ASCII/punycode encoded host |
| `port` | `int \| None` | Port number (None if default) |
| `path` | `str` | Decoded path |
| `raw_path` | `bytes` | Encoded path + query |
| `query` | `bytes` | Query string (without '?') |
| `fragment` | `str` | Fragment (without '#') |
| `userinfo` | `bytes` | username:password (encoded) |
| `username` | `str` | Decoded username |
| `password` | `str \| None` | Decoded password |
| `netloc` | `bytes` | host:port |
| `origin` | `str` | scheme://host:port |
| `params` | `QueryParams` | Query parameters object |
| `is_relative_url` | `bool` | True if no scheme |
| `is_absolute_url` | `bool` | True if has scheme |
| `is_default_port` | `bool` | True if using default port |

### Methods

#### `copy_with(**kwargs) -> URL`

Create a modified copy of the URL:

```python
url = URL("https://example.com/path")
new_url = url.copy_with(scheme="http", path="/new-path", params={"key": "value"})
```

Supported kwargs: `scheme`, `netloc`, `path`, `query`, `fragment`, `username`, `password`, `host`, `port`, `raw_path`, `params`

#### `join(url: str) -> URL`

Join with another URL (RFC 3986 compliant):

```python
url = URL("https://example.com/a/b/c")
url.join("/x")         # "https://example.com/x"
url.join("../y")       # "https://example.com/a/y"
url.join("//other.com") # "https://other.com"
```

#### Query Parameter Methods

```python
url = URL("https://example.com/?a=1")

url.copy_set_param("a", "2")      # Replaces: ?a=2
url.copy_add_param("b", "3")      # Appends: ?a=1&b=3
url.copy_remove_param("a")        # Removes: (empty query)
url.copy_merge_params({"c": "4"}) # Merges: ?a=1&c=4
```

## Key Implementation Details

### 1. Percent Encoding

Different URL components have different safe character sets:

- **Path**: Allows `!$&'()*+,;=:@/[]` plus alphanumerics
- **Query**: Allows `!$&'()*+,;=:@/?[]` plus alphanumerics  
- **Userinfo**: Allows `!$&'()*+,;=%` plus alphanumerics

The implementation normalizes percent encoding:
- Already-encoded safe characters are decoded
- Unsafe characters are encoded
- Uppercase hex digits are used

### 2. IDNA Hostname Handling

Internationalized hostnames are handled via punycode:

```python
url = URL("https://中国.icom.museum/")
url.host       # "中国.icom.museum" (decoded)
url.raw_host   # b"xn--fiqs8s.icom.museum" (punycode)
```

### 3. Port Normalization

Default ports are normalized to `None`:

```python
URL("https://example.com:443/").port  # None (default for https)
URL("https://example.com:8080/").port # 8080
URL("http://example.com:80/").port    # None (default for http)
```

### 4. Path Normalization

Paths are normalized by resolving `.` and `..`:

```python
URL("https://example.com/a/b/../c/./d").path  # "/a/c/d"
URL("https://example.com/../abc").path         # "/abc" (can't go above root)
URL("../abc").path                             # "../abc" (relative preserved)
```

### 5. Query String vs Params

- `query`: Raw bytes, preserves existing encoding
- `params`: Dict/QueryParams, applies form-urlencoding

```python
# From URL string - preserves encoding
URL("https://example.com?a=hello%20world").query  # b"a=hello%20world"

# From params - applies form encoding  
URL("https://example.com", params={"a": "hello world"}).raw_path  # b"/?a=hello+world"
```

## Integration with RequestX

### File Structure

```
requestx/
├── src/
│   ├── lib.rs          # Main module, register URL
│   ├── url.rs          # This implementation
│   └── query_params.rs # QueryParams (required dependency)
└── python/
    └── requestx/
        └── __init__.py # Re-export URL, InvalidURL
```

### In `lib.rs`

```rust
mod url;
mod query_params;

use pyo3::prelude::*;

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    url::register_url_module(m)?;
    query_params::register_query_params_module(m)?;
    // ... other registrations
    Ok(())
}
```

### In `__init__.py`

```python
from ._core import URL, InvalidURL, QueryParams

__all__ = ["URL", "InvalidURL", "QueryParams", ...]
```

## Dependencies

Add to `Cargo.toml`:

```toml
[dependencies]
pyo3 = { version = "0.21", features = ["extension-module"] }
```

No external URL parsing libraries are needed - this is a complete self-contained implementation.

## Error Handling

The `InvalidURL` exception is raised for:

- Invalid port (non-numeric or out of range)
- Invalid IPv4/IPv6 addresses
- Invalid IDNA hostnames
- Non-printable characters
- URL/component too long
- Invalid path for URL type

```python
try:
    url = URL("https://example.com:abc/")
except InvalidURL as e:
    print(e)  # "Invalid port: 'abc'"
```

## Test Coverage

The implementation passes all httpx URL tests including:

- Basic URL parsing and properties
- Percent encoding normalization
- Username/password handling
- IDNA hostname conversion
- IPv4/IPv6 address validation
- Path normalization
- Query parameter manipulation
- URL joining (RFC 3986)
- copy_with() modifications
- Error cases and edge cases

## Performance Notes

- Zero-copy where possible (uses references)
- Minimal allocations in hot paths
- Efficient percent encoding/decoding
- Lazy property computation

The Rust implementation should be significantly faster than the pure Python httpx URL implementation, especially for URL-heavy workloads in AI applications.
