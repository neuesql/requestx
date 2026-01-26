# Async Guide

RequestX provides full async/await support through the `AsyncClient` class, built on Rust's tokio async runtime.

## Basic Async Usage

Use `AsyncClient` for asynchronous HTTP requests:

```python
import asyncio
import requestx

async def main():
    async with requestx.AsyncClient() as client:
        response = await client.get("https://httpbin.org/json")
        print(response.json())

asyncio.run(main())
```

## AsyncClient Configuration

`AsyncClient` accepts the same configuration options as `Client`:

```python
import asyncio
import requestx

async def main():
    async with requestx.AsyncClient(
        base_url="https://api.example.com",
        headers={"Authorization": "Bearer token"},
        timeout=requestx.Timeout(timeout=30.0),
        http2=True,
    ) as client:
        response = await client.get("/users")
        users = response.json()

asyncio.run(main())
```

## Making Concurrent Requests

Use `asyncio.gather()` for concurrent requests:

```python
import asyncio
import requestx

async def fetch_url(client, url):
    response = await client.get(url)
    return response.json()

async def main():
    urls = [
        "https://httpbin.org/json",
        "https://httpbin.org/uuid",
        "https://httpbin.org/headers",
    ]

    async with requestx.AsyncClient() as client:
        tasks = [fetch_url(client, url) for url in urls]
        results = await asyncio.gather(*tasks)

        for url, result in zip(urls, results):
            print(f"{url}: {result}")

asyncio.run(main())
```

## HTTP Methods

All standard HTTP methods are available as async methods:

```python
import asyncio
import requestx

async def main():
    async with requestx.AsyncClient() as client:
        # GET
        response = await client.get("https://httpbin.org/get")

        # POST
        response = await client.post(
            "https://httpbin.org/post",
            json={"key": "value"}
        )

        # PUT
        response = await client.put(
            "https://httpbin.org/put",
            json={"updated": True}
        )

        # PATCH
        response = await client.patch(
            "https://httpbin.org/patch",
            json={"patched": True}
        )

        # DELETE
        response = await client.delete("https://httpbin.org/delete")

        # HEAD
        response = await client.head("https://httpbin.org/get")

        # OPTIONS
        response = await client.options("https://httpbin.org/get")

asyncio.run(main())
```

## Error Handling

Handle errors in async code:

```python
import asyncio
import requestx
from requestx import RequestError, HTTPStatusError, ConnectError, TimeoutException

async def fetch_with_retry(client, url, max_retries=3):
    for attempt in range(max_retries):
        try:
            response = await client.get(url)
            response.raise_for_status()
            return response.json()
        except TimeoutException:
            if attempt < max_retries - 1:
                await asyncio.sleep(2 ** attempt)  # Exponential backoff
                continue
            raise
        except HTTPStatusError as e:
            if e.response.status_code >= 500 and attempt < max_retries - 1:
                await asyncio.sleep(1)
                continue
            raise

async def main():
    async with requestx.AsyncClient(
        timeout=requestx.Timeout(timeout=10.0)
    ) as client:
        try:
            data = await fetch_with_retry(client, "https://api.example.com/data")
            print(data)
        except RequestError as e:
            print(f"Request failed: {e}")

asyncio.run(main())
```

## Streaming Responses

Handle streaming responses asynchronously:

```python
import asyncio
import requestx

async def download_file(url, filename):
    async with requestx.AsyncClient() as client:
        async with await client.stream("GET", url) as response:
            with open(filename, "wb") as f:
                async for chunk in response.aiter_bytes(chunk_size=8192):
                    f.write(chunk)

async def main():
    await download_file(
        "https://httpbin.org/bytes/1000000",
        "downloaded_file.bin"
    )

asyncio.run(main())
```

## Rate Limiting

Implement rate limiting with asyncio:

```python
import asyncio
import requestx

class RateLimiter:
    def __init__(self, rate: float, per: float = 1.0):
        self.rate = rate
        self.per = per
        self.tokens = rate
        self.last_update = asyncio.get_event_loop().time()
        self.lock = asyncio.Lock()

    async def acquire(self):
        async with self.lock:
            now = asyncio.get_event_loop().time()
            elapsed = now - self.last_update
            self.tokens = min(self.rate, self.tokens + elapsed * (self.rate / self.per))
            self.last_update = now

            if self.tokens < 1:
                wait_time = (1 - self.tokens) * (self.per / self.rate)
                await asyncio.sleep(wait_time)
                self.tokens = 0
            else:
                self.tokens -= 1

async def main():
    rate_limiter = RateLimiter(rate=10, per=1.0)  # 10 requests per second

    async with requestx.AsyncClient() as client:
        for i in range(20):
            await rate_limiter.acquire()
            response = await client.get(f"https://httpbin.org/get?i={i}")
            print(f"Request {i}: {response.status_code}")

asyncio.run(main())
```

## Semaphore for Concurrency Control

Limit concurrent requests with a semaphore:

```python
import asyncio
import requestx

async def fetch_with_limit(client, url, semaphore):
    async with semaphore:
        response = await client.get(url)
        return response.json()

async def main():
    urls = [f"https://httpbin.org/get?i={i}" for i in range(100)]
    semaphore = asyncio.Semaphore(10)  # Max 10 concurrent requests

    async with requestx.AsyncClient() as client:
        tasks = [fetch_with_limit(client, url, semaphore) for url in urls]
        results = await asyncio.gather(*tasks)
        print(f"Fetched {len(results)} URLs")

asyncio.run(main())
```

## Context Manager Usage

Always use `AsyncClient` as an async context manager:

```python
import asyncio
import requestx

async def main():
    # Recommended: Use as context manager
    async with requestx.AsyncClient() as client:
        response = await client.get("https://httpbin.org/get")

    # Alternative: Manual lifecycle management
    client = requestx.AsyncClient()
    try:
        response = await client.get("https://httpbin.org/get")
    finally:
        await client.aclose()

asyncio.run(main())
```

## Integration with Web Frameworks

### FastAPI Example

```python
from fastapi import FastAPI
import requestx

app = FastAPI()
http_client = None

@app.on_event("startup")
async def startup():
    global http_client
    http_client = requestx.AsyncClient(
        base_url="https://api.external.com",
        timeout=requestx.Timeout(timeout=30.0),
    )

@app.on_event("shutdown")
async def shutdown():
    await http_client.aclose()

@app.get("/proxy/{path:path}")
async def proxy_request(path: str):
    response = await http_client.get(f"/{path}")
    return response.json()
```

## Best Practices

1. **Reuse AsyncClient** - Create one client and reuse it for multiple requests
2. **Use context managers** - Ensures proper resource cleanup
3. **Limit concurrency** - Use semaphores to avoid overwhelming servers
4. **Handle timeouts** - Set appropriate timeouts for your use case
5. **Implement retries** - Use exponential backoff for transient failures

```python
import asyncio
import requestx

async def best_practices_example():
    # Create client once with proper configuration
    async with requestx.AsyncClient(
        timeout=requestx.Timeout(timeout=30.0, connect=5.0),
        http2=True,
    ) as client:
        # Reuse for multiple requests
        semaphore = asyncio.Semaphore(20)

        async def fetch(url):
            async with semaphore:
                for attempt in range(3):
                    try:
                        response = await client.get(url)
                        response.raise_for_status()
                        return response.json()
                    except requestx.TimeoutException:
                        if attempt < 2:
                            await asyncio.sleep(2 ** attempt)
                        else:
                            raise

        urls = [f"https://api.example.com/item/{i}" for i in range(100)]
        results = await asyncio.gather(*[fetch(url) for url in urls])
        return results

asyncio.run(best_practices_example())
```
