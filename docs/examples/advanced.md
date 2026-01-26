# Advanced Examples

This page contains advanced usage patterns for RequestX.

## Concurrent Async Requests

```python
import asyncio
import requestx

async def fetch_url(client: requestx.AsyncClient, url: str) -> dict:
    response = await client.get(url)
    return {"url": url, "status": response.status_code}

async def main():
    urls = [
        "https://httpbin.org/get",
        "https://httpbin.org/uuid",
        "https://httpbin.org/json",
        "https://httpbin.org/headers",
    ]

    async with requestx.AsyncClient() as client:
        tasks = [fetch_url(client, url) for url in urls]
        results = await asyncio.gather(*tasks)

        for result in results:
            print(f"{result['url']}: {result['status']}")

asyncio.run(main())
```

## Rate-Limited API Client

```python
import asyncio
import requestx

class RateLimitedClient:
    def __init__(self, base_url: str, requests_per_second: float):
        self.client = requestx.AsyncClient(base_url=base_url)
        self.semaphore = asyncio.Semaphore(int(requests_per_second))
        self.delay = 1.0 / requests_per_second

    async def get(self, path: str, **kwargs) -> requestx.Response:
        async with self.semaphore:
            response = await self.client.get(path, **kwargs)
            await asyncio.sleep(self.delay)
            return response

    async def close(self):
        await self.client.aclose()

    async def __aenter__(self):
        return self

    async def __aexit__(self, *args):
        await self.close()

async def main():
    async with RateLimitedClient(
        "https://api.example.com",
        requests_per_second=5
    ) as client:
        for i in range(20):
            response = await client.get(f"/item/{i}")
            print(f"Item {i}: {response.status_code}")

asyncio.run(main())
```

## Retry with Exponential Backoff

```python
import asyncio
import random
import requestx
from requestx import ConnectError, TimeoutException, HTTPStatusError

async def fetch_with_retry(
    client: requestx.AsyncClient,
    url: str,
    max_retries: int = 3,
    base_delay: float = 1.0,
) -> requestx.Response:
    last_error = None

    for attempt in range(max_retries):
        try:
            response = await client.get(url)
            response.raise_for_status()
            return response

        except (ConnectError, TimeoutException) as e:
            last_error = e
            delay = base_delay * (2 ** attempt) + random.uniform(0, 1)
            print(f"Attempt {attempt + 1} failed: {e}. Retrying in {delay:.1f}s")
            await asyncio.sleep(delay)

        except HTTPStatusError as e:
            if e.response.status_code >= 500:
                last_error = e
                delay = base_delay * (2 ** attempt)
                print(f"Server error. Retrying in {delay:.1f}s")
                await asyncio.sleep(delay)
            else:
                raise

    raise last_error

async def main():
    async with requestx.AsyncClient(
        timeout=requestx.Timeout(timeout=10.0)
    ) as client:
        response = await fetch_with_retry(
            client,
            "https://httpbin.org/get"
        )
        print(response.json())

asyncio.run(main())
```

## API Pagination

```python
import requestx

def paginated_fetch(base_url: str, endpoint: str, per_page: int = 100):
    """Fetch all pages from a paginated API."""
    with requestx.Client(base_url=base_url) as client:
        page = 1
        all_items = []

        while True:
            response = client.get(
                endpoint,
                params={"page": page, "per_page": per_page}
            )
            response.raise_for_status()
            items = response.json()

            if not items:
                break

            all_items.extend(items)
            print(f"Fetched page {page}: {len(items)} items")

            page += 1

        return all_items

# Usage
items = paginated_fetch(
    "https://api.example.com",
    "/items"
)
print(f"Total items: {len(items)}")
```

## Async Pagination

```python
import asyncio
import requestx

async def async_paginated_fetch(
    base_url: str,
    endpoint: str,
    per_page: int = 100
) -> list:
    """Fetch all pages concurrently."""
    async with requestx.AsyncClient(base_url=base_url) as client:
        # First, get total count
        response = await client.get(endpoint, params={"per_page": 1})
        total = int(response.headers.get("x-total-count", 100))
        total_pages = (total + per_page - 1) // per_page

        # Fetch all pages concurrently
        async def fetch_page(page: int) -> list:
            response = await client.get(
                endpoint,
                params={"page": page, "per_page": per_page}
            )
            return response.json()

        tasks = [fetch_page(page) for page in range(1, total_pages + 1)]
        pages = await asyncio.gather(*tasks)

        # Flatten results
        return [item for page in pages for item in page]

# Usage
asyncio.run(async_paginated_fetch("https://api.example.com", "/items"))
```

## File Download with Progress

```python
import requestx
import sys

def download_with_progress(url: str, filename: str):
    with requestx.Client() as client:
        with client.stream("GET", url) as response:
            response.raise_for_status()

            total = int(response.headers.get("content-length", 0))
            downloaded = 0

            with open(filename, "wb") as f:
                for chunk in response.iter_bytes(chunk_size=8192):
                    f.write(chunk)
                    downloaded += len(chunk)

                    if total:
                        percent = downloaded / total * 100
                        bar_len = 50
                        filled = int(bar_len * downloaded / total)
                        bar = "=" * filled + "-" * (bar_len - filled)
                        sys.stdout.write(f"\r[{bar}] {percent:.1f}%")
                        sys.stdout.flush()

            print(f"\nDownloaded {filename}")

# Usage
download_with_progress(
    "https://httpbin.org/bytes/1000000",
    "downloaded_file.bin"
)
```

## Multipart File Upload

```python
import requestx

def upload_file(url: str, file_path: str):
    with open(file_path, "rb") as f:
        files = {"file": (file_path.split("/")[-1], f.read())}

        response = requestx.post(url, files=files)
        response.raise_for_status()
        return response.json()

# Usage
result = upload_file(
    "https://httpbin.org/post",
    "document.pdf"
)
```

## Webhook Handler

```python
import asyncio
import requestx
from typing import Callable, Any

class WebhookSender:
    def __init__(self, webhook_url: str, secret: str):
        self.webhook_url = webhook_url
        self.secret = secret
        self.client = requestx.AsyncClient(
            timeout=requestx.Timeout(timeout=30.0)
        )

    async def send(self, event: str, data: dict) -> bool:
        try:
            response = await self.client.post(
                self.webhook_url,
                json={"event": event, "data": data},
                headers={
                    "X-Webhook-Secret": self.secret,
                    "Content-Type": "application/json"
                }
            )
            response.raise_for_status()
            return True
        except requestx.RequestError as e:
            print(f"Webhook failed: {e}")
            return False

    async def close(self):
        await self.client.aclose()

# Usage
async def main():
    webhook = WebhookSender(
        "https://example.com/webhook",
        "secret-key"
    )

    try:
        await webhook.send("user.created", {"id": 123, "name": "John"})
    finally:
        await webhook.close()

asyncio.run(main())
```

## API Client with Automatic Token Refresh

```python
import asyncio
from datetime import datetime, timedelta
import requestx

class APIClient:
    def __init__(
        self,
        base_url: str,
        client_id: str,
        client_secret: str,
        token_url: str
    ):
        self.base_url = base_url
        self.client_id = client_id
        self.client_secret = client_secret
        self.token_url = token_url
        self.access_token = None
        self.token_expires = None
        self.client = requestx.AsyncClient(base_url=base_url)
        self._lock = asyncio.Lock()

    async def _refresh_token(self):
        response = await self.client.post(
            self.token_url,
            data={
                "grant_type": "client_credentials",
                "client_id": self.client_id,
                "client_secret": self.client_secret,
            }
        )
        response.raise_for_status()
        data = response.json()

        self.access_token = data["access_token"]
        expires_in = data.get("expires_in", 3600)
        self.token_expires = datetime.now() + timedelta(seconds=expires_in - 60)

    async def _ensure_token(self):
        async with self._lock:
            if not self.access_token or datetime.now() >= self.token_expires:
                await self._refresh_token()

    async def request(self, method: str, path: str, **kwargs) -> requestx.Response:
        await self._ensure_token()

        headers = kwargs.pop("headers", {})
        headers["Authorization"] = f"Bearer {self.access_token}"

        response = await self.client.request(
            method, path, headers=headers, **kwargs
        )
        return response

    async def get(self, path: str, **kwargs) -> requestx.Response:
        return await self.request("GET", path, **kwargs)

    async def post(self, path: str, **kwargs) -> requestx.Response:
        return await self.request("POST", path, **kwargs)

    async def close(self):
        await self.client.aclose()

    async def __aenter__(self):
        return self

    async def __aexit__(self, *args):
        await self.close()

# Usage
async def main():
    async with APIClient(
        base_url="https://api.example.com",
        client_id="my-client",
        client_secret="my-secret",
        token_url="https://auth.example.com/oauth/token"
    ) as api:
        users = (await api.get("/users")).json()
        print(f"Users: {users}")

asyncio.run(main())
```

## Health Check Endpoint

```python
import asyncio
import requestx

async def check_health(urls: list[str]) -> dict:
    """Check health of multiple endpoints."""
    results = {}

    async with requestx.AsyncClient(
        timeout=requestx.Timeout(timeout=5.0)
    ) as client:

        async def check_one(url: str) -> tuple[str, dict]:
            try:
                response = await client.get(url)
                return url, {
                    "status": "healthy",
                    "code": response.status_code,
                    "latency": response.elapsed
                }
            except requestx.TimeoutException:
                return url, {"status": "timeout"}
            except requestx.ConnectError:
                return url, {"status": "unreachable"}
            except Exception as e:
                return url, {"status": "error", "message": str(e)}

        tasks = [check_one(url) for url in urls]
        results_list = await asyncio.gather(*tasks)

        return dict(results_list)

# Usage
async def main():
    urls = [
        "https://httpbin.org/get",
        "https://jsonplaceholder.typicode.com/posts/1",
        "https://invalid.example.com",
    ]

    health = await check_health(urls)
    for url, status in health.items():
        print(f"{url}: {status}")

asyncio.run(main())
```
