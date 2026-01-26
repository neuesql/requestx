# Streaming Guide

RequestX supports streaming for both request and response bodies, enabling efficient handling of large data transfers.

## Streaming Responses

### Synchronous Streaming

Use `client.stream()` for streaming responses:

```python
import requestx

with requestx.Client() as client:
    with client.stream("GET", "https://httpbin.org/bytes/10000") as response:
        for chunk in response.iter_bytes(chunk_size=1024):
            print(f"Received {len(chunk)} bytes")
```

### Asynchronous Streaming

Use async streaming with `AsyncClient`:

```python
import asyncio
import requestx

async def main():
    async with requestx.AsyncClient() as client:
        async with await client.stream("GET", "https://httpbin.org/bytes/10000") as response:
            async for chunk in response.aiter_bytes(chunk_size=1024):
                print(f"Received {len(chunk)} bytes")

asyncio.run(main())
```

## Iteration Methods

### iter_bytes / aiter_bytes

Iterate over raw bytes:

```python
# Sync
with client.stream("GET", url) as response:
    for chunk in response.iter_bytes(chunk_size=1024):
        process_bytes(chunk)

# Async
async with await client.stream("GET", url) as response:
    async for chunk in response.aiter_bytes(chunk_size=1024):
        process_bytes(chunk)
```

### iter_text / aiter_text

Iterate over decoded text:

```python
# Sync
with client.stream("GET", url) as response:
    for text in response.iter_text():
        process_text(text)

# Async
async with await client.stream("GET", url) as response:
    async for text in response.aiter_text():
        process_text(text)
```

### iter_lines / aiter_lines

Iterate over lines:

```python
# Sync
with client.stream("GET", url) as response:
    for line in response.iter_lines():
        print(line)

# Async
async with await client.stream("GET", url) as response:
    async for line in response.aiter_lines():
        print(line)
```

## Download Files

### Basic File Download

```python
import requestx

def download_file(url: str, filename: str):
    with requestx.Client() as client:
        with client.stream("GET", url) as response:
            response.raise_for_status()
            with open(filename, "wb") as f:
                for chunk in response.iter_bytes(chunk_size=8192):
                    f.write(chunk)

download_file("https://example.com/large-file.zip", "downloaded.zip")
```

### Download with Progress

```python
import requestx

def download_with_progress(url: str, filename: str):
    with requestx.Client() as client:
        with client.stream("GET", url) as response:
            response.raise_for_status()

            total_size = int(response.headers.get("content-length", 0))
            downloaded = 0

            with open(filename, "wb") as f:
                for chunk in response.iter_bytes(chunk_size=8192):
                    f.write(chunk)
                    downloaded += len(chunk)

                    if total_size:
                        percent = (downloaded / total_size) * 100
                        print(f"\rProgress: {percent:.1f}%", end="")

            print("\nDownload complete!")

download_with_progress("https://httpbin.org/bytes/100000", "file.bin")
```

### Async File Download

```python
import asyncio
import aiofiles
import requestx

async def download_file_async(url: str, filename: str):
    async with requestx.AsyncClient() as client:
        async with await client.stream("GET", url) as response:
            response.raise_for_status()

            async with aiofiles.open(filename, "wb") as f:
                async for chunk in response.aiter_bytes(chunk_size=8192):
                    await f.write(chunk)

asyncio.run(download_file_async("https://example.com/file.zip", "downloaded.zip"))
```

## Streaming Server-Sent Events (SSE)

Handle SSE streams:

```python
import requestx

def handle_sse(url: str):
    with requestx.Client() as client:
        with client.stream("GET", url) as response:
            for line in response.iter_lines():
                if line.startswith("data: "):
                    data = line[6:]
                    print(f"Event: {data}")

# Async version
async def handle_sse_async(url: str):
    async with requestx.AsyncClient() as client:
        async with await client.stream("GET", url) as response:
            async for line in response.aiter_lines():
                if line.startswith("data: "):
                    data = line[6:]
                    print(f"Event: {data}")
```

## Streaming JSON Lines (JSONL)

Process JSONL streams:

```python
import json
import requestx

def process_jsonl(url: str):
    with requestx.Client() as client:
        with client.stream("GET", url) as response:
            for line in response.iter_lines():
                if line.strip():
                    data = json.loads(line)
                    process_record(data)

# Async version
async def process_jsonl_async(url: str):
    async with requestx.AsyncClient() as client:
        async with await client.stream("GET", url) as response:
            async for line in response.aiter_lines():
                if line.strip():
                    data = json.loads(line)
                    process_record(data)
```

## Response Properties

Access response metadata before streaming:

```python
import requestx

with requestx.Client() as client:
    with client.stream("GET", url) as response:
        # Check status before consuming
        print(f"Status: {response.status_code}")
        print(f"Headers: {response.headers}")
        print(f"Content-Length: {response.headers.get('content-length')}")

        # Raise for errors
        response.raise_for_status()

        # Then stream the content
        for chunk in response.iter_bytes():
            process(chunk)
```

## Memory Efficiency

Streaming is essential for large responses to avoid memory issues:

```python
import requestx

# Bad: Loads entire response into memory
response = client.get("https://example.com/huge-file.zip")
data = response.content  # Potentially gigabytes in memory!

# Good: Stream to process without loading all into memory
with client.stream("GET", "https://example.com/huge-file.zip") as response:
    for chunk in response.iter_bytes(chunk_size=8192):
        # Process chunk by chunk
        process_chunk(chunk)
```

## Best Practices

1. **Always use context managers** - Ensures streams are properly closed
2. **Set appropriate chunk sizes** - Balance between memory usage and I/O overhead
3. **Check status before streaming** - Verify the response is successful first
4. **Handle timeouts** - Set read timeouts for long-running streams
5. **Use async for concurrent downloads** - Better resource utilization

```python
import asyncio
import requestx

async def download_multiple(urls: list[str], output_dir: str):
    async with requestx.AsyncClient(
        timeout=requestx.Timeout(timeout=300.0, connect=10.0)
    ) as client:

        async def download_one(url: str):
            filename = url.split("/")[-1]
            filepath = f"{output_dir}/{filename}"

            async with await client.stream("GET", url) as response:
                response.raise_for_status()

                with open(filepath, "wb") as f:
                    async for chunk in response.aiter_bytes(chunk_size=65536):
                        f.write(chunk)

            return filepath

        # Download all concurrently
        tasks = [download_one(url) for url in urls]
        results = await asyncio.gather(*tasks, return_exceptions=True)

        for url, result in zip(urls, results):
            if isinstance(result, Exception):
                print(f"Failed: {url} - {result}")
            else:
                print(f"Downloaded: {result}")

asyncio.run(download_multiple([
    "https://example.com/file1.zip",
    "https://example.com/file2.zip",
], "./downloads"))
```
