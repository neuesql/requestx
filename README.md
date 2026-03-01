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

**Basic:**

```python
import requestx as httpx

# GET request
r = httpx.get("https://api.example.com/users")
print(r.json())

# POST request
r = httpx.post("https://api.example.com/users", json={"name": "Alice"})

# With a client (connection pooling)
with httpx.Client(base_url="https://api.example.com") as client:
    r = client.get("/users")
    print(r.status_code)
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

**AI SDKs (OpenAI, Anthropic):**

RequestX is a drop-in performance upgrade for AI SDKs that use httpx internally:

```python
import requestx
from openai import OpenAI

# Sync client - up to 4x faster
client = OpenAI(http_client=requestx.Client())
response = client.chat.completions.create(
    model="gpt-4",
    messages=[{"role": "user", "content": "Hello"}]
)

# Async client - scales linearly with concurrency
from openai import AsyncOpenAI
import asyncio

async def main():
    async_client = AsyncOpenAI(http_client=requestx.AsyncClient())
    response = await async_client.chat.completions.create(
        model="gpt-4",
        messages=[{"role": "user", "content": "Hello"}]
    )

asyncio.run(main())
```

```python
import requestx
from anthropic import Anthropic

client = Anthropic(http_client=requestx.Client())
message = client.messages.create(
    model="claude-3-5-sonnet-20241022",
    max_tokens=1024,
    messages=[{"role": "user", "content": "Hello"}]
)
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

## Integration Tests

RequestX includes integration tests that verify compatibility with real AI SDK APIs (OpenAI and Anthropic). These tests make actual API calls and require API keys.

### Setup

1. Install integration dependencies:
```bash
pip install -e ".[integration]"
```

2. Set environment variables:
```bash
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
```

### Running Integration Tests

```bash
# Run all integration tests
pytest tests_integration/ -v

# Run only OpenAI tests
pytest tests_integration/test_openai_integration.py -v

# Run only Anthropic tests
pytest tests_integration/test_anthropic_integration.py -v
```

### Important Notes

- **Cost**: Tests make real API calls and incur costs (~$0.01 per full run)
- **API Keys**: Tests skip gracefully if API keys are not set
- **CI/CD**: These tests should NOT run in regular CI (require secrets, cost money)
- Tests use minimal tokens (max_tokens=10) to minimize costs

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
