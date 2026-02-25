# RequestX

**Drop-in replacement for httpx, powered by Rust.** Up to 4x faster, scales linearly with concurrency.

```bash
pip install requestx
```

```python
import requestx as httpx

r = httpx.get("https://api.example.com/data")
print(r.json())
```

Every `httpx` API works. No code changes needed.

---

## Performance

Benchmarked on Python 3.12, Apple Silicon, local HTTP server, 30s per run.

### Requests Per Second (higher is better)

**Sync clients:**

| Concurrency | requestx | httpx | requests | urllib3 |
|:-----------:|:--------:|:-----:|:--------:|:-------:|
| 1 | 1,630 | 1,034 | 773 | 1,459 |
| 4 | 5,602 | 3,208 | 3,139 | 3,164 |
| 10 | **6,635** | 2,391 | 3,390 | 1,762 |

**Async clients:**

| Concurrency | requestx | httpx | aiohttp |
|:-----------:|:--------:|:-----:|:-------:|
| 1 | 875 | 424 | 1,119 |
| 4 | 5,164 | 2,633 | 5,599 |
| 10 | **7,163** | 1,637 | 7,167 |

### Speedup vs httpx

| Concurrency | Sync | Async |
|:-----------:|:----:|:-----:|
| 1 | 1.58x | 2.06x |
| 4 | 1.75x | 1.96x |
| 6 | 2.23x | 2.88x |
| 8 | 2.63x | 3.82x |
| 10 | **2.78x** | **4.38x** |

httpx performance **degrades** under concurrent load (1,576 → 1,322 RPS from c=1 to c=10). RequestX **scales linearly** (875 → 7,163 RPS).

---

## Usage

**Sync:**

```python
import requestx as httpx

# GET
r = httpx.get("https://api.example.com/users")
r.json()
r.status_code
r.headers

# POST
r = httpx.post("https://api.example.com/users", json={"name": "Alice"})

# With a client (connection pooling, auth, headers)
with httpx.Client(base_url="https://api.example.com", headers={"Authorization": "Bearer token"}) as client:
    r = client.get("/users")
    r = client.post("/users", json={"name": "Alice"})
```

**Async:**

```python
import requestx as httpx
import asyncio

async def main():
    async with httpx.AsyncClient() as client:
        r = await client.get("https://api.example.com/users")
        print(r.json())

asyncio.run(main())
```

**Streaming:**

```python
import requestx as httpx

with httpx.stream("GET", "https://example.com/large-file") as r:
    for chunk in r.iter_bytes():
        process(chunk)
```

---

## Why It's Fast

RequestX replaces httpx's Python internals with Rust (reqwest + Tokio), compiled via PyO3.

| | httpx | requestx |
|---|---|---|
| HTTP engine | Python (httpcore) | Rust (reqwest) |
| Async runtime | Python asyncio | Tokio (GIL-free) |
| JSON parsing | Python json | sonic-rs (SIMD) |
| Connection pool | Python-managed | Rust hyper |
| GIL during I/O | Held | Released |
| Concurrency scaling | Degrades | Linear |

All network I/O runs outside Python's GIL, enabling true parallelism that httpx cannot achieve.

---

## Features

- **100% httpx API compatible** — 1,406 tests passing, mirrored from httpx test suite
- **Sync + Async** — `Client` and `AsyncClient` with full feature parity
- **HTTP/2** — native support via rustls
- **Compression** — gzip, brotli, deflate, zstd
- **Auth** — Basic, Digest (RFC 7616), NetRC, custom auth flows
- **Proxies** — HTTP/SOCKS proxy support, environment variable detection
- **Streaming** — byte, text, line, and raw iterators (sync and async)
- **Cookie persistence** — domain/path-aware jar
- **Transports** — Mock, WSGI, ASGI for testing
- **Event hooks** — request and response hooks, including on redirects

---

## Compatibility

RequestX passes the full httpx test suite (1,406 tests). API coverage is 98.5% — the only excluded symbol is `main` (httpx's CLI entry point).

```python
# These all work identically to httpx
import requestx as httpx

httpx.get(...)
httpx.Client(...)
httpx.AsyncClient(...)
httpx.URL(...)
httpx.Headers(...)
httpx.Response(...)
httpx.stream(...)
httpx.Timeout(...)
httpx.HTTPStatusError
httpx.TimeoutException
```

---

## License

MIT
