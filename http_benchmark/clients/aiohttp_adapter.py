"""AIOHTTP HTTP client adapter for the HTTP benchmark framework."""

import aiohttp
import asyncio
import time
from typing import Dict, Any
from .base import BaseHTTPAdapter
from ..models.http_request import HTTPRequest


class AiohttpAdapter(BaseHTTPAdapter):
    """HTTP adapter for the aiohttp library."""

    def __init__(self):
        super().__init__("aiohttp")
        self.session = None

    def __enter__(self):
        """aiohttp is async-only, sync context not supported."""
        raise NotImplementedError("aiohttp is async-only")

    def __exit__(self, exc_type, exc_val, exc_tb):
        pass

    async def __aenter__(self):
        """Initialize session when entering async context."""
        connector = aiohttp.TCPConnector(limit=0, ttl_dns_cache=300)
        self.session = aiohttp.ClientSession(connector=connector)
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Close session when exiting async context."""
        if self.session and not self.session.closed:
            await self.session.close()
            await asyncio.sleep(0.250)

    def make_request(self, request: HTTPRequest) -> Dict[str, Any]:
        """Make an HTTP request using the aiohttp library."""
        raise NotImplementedError("aiohttp is async-only, use make_request_async instead")

    async def make_request_async(self, request: HTTPRequest) -> Dict[str, Any]:
        """Make an async HTTP request using the aiohttp library."""
        try:
            method = request.method.upper()
            url = request.url
            headers = request.headers
            timeout = aiohttp.ClientTimeout(total=request.timeout)
            ssl = True if request.verify_ssl else False

            data = request.body if request.body else None

            start_time = time.perf_counter()

            async with self.session.request(
                method=method,
                url=url,
                headers=headers,
                data=data,
                timeout=timeout,
                ssl=ssl,
            ) as response:
                content = await response.text()

            end_time = time.perf_counter()

            return {
                "status_code": response.status,
                "headers": dict(response.headers),
                "content": content,
                "response_time": end_time - start_time,
                "url": str(response.url),
                "success": True,
                "error": None,
            }
        except Exception as e:
            return {
                "status_code": None,
                "headers": {},
                "content": "",
                "response_time": 0,
                "url": request.url,
                "success": False,
                "error": str(e),
            }

    def make_request_stream(self, request: HTTPRequest) -> Dict[str, Any]:
        """Make a streaming HTTP request using the aiohttp library."""
        raise NotImplementedError("aiohttp is async-only, use make_request_stream_async instead")

    async def make_request_stream_async(self, request: HTTPRequest) -> Dict[str, Any]:
        """Make an async streaming HTTP request using the aiohttp library."""
        try:
            method = request.method.upper()
            url = request.url
            headers = request.headers
            timeout = aiohttp.ClientTimeout(total=request.timeout)
            ssl = True if request.verify_ssl else False

            data = request.body if request.body else None

            start_time = time.perf_counter()

            async with self.session.request(
                method=method,
                url=url,
                headers=headers,
                data=data,
                timeout=timeout,
                ssl=ssl,
            ) as response:
                content = b""
                async for chunk in response.content.iter_chunked(8192):
                    if chunk:
                        content += chunk

            end_time = time.perf_counter()

            return {
                "status_code": response.status,
                "headers": dict(response.headers),
                "content": content.decode("utf-8") if content else "",
                "response_time": end_time - start_time,
                "url": str(response.url),
                "success": True,
                "error": None,
                "streamed": True,
                "chunk_count": len(content) // 8192 + (1 if len(content) % 8192 > 0 else 0),
            }
        except Exception as e:
            return {
                "status_code": None,
                "headers": {},
                "content": "",
                "response_time": 0,
                "url": request.url,
                "success": False,
                "error": str(e),
                "streamed": False,
            }
