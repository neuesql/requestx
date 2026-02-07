"""RequestX HTTP client adapter for the HTTP benchmark framework."""

import time
from typing import Any, Dict

import requestx

from ..models.http_request import HTTPRequest
from .base import BaseHTTPAdapter


class RequestXAdapter(BaseHTTPAdapter):
    """HTTP adapter for the requestx library."""

    def __init__(self):
        super().__init__("requestx")
        self.client = None
        self.async_client = None
        self.verify_ssl = True

    def __enter__(self):
        """Initialize sync client when entering sync context."""
        self.client = requestx.Client(verify=self.verify_ssl)
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Close sync client when exiting sync context."""
        if self.client:
            self.client.close()

    async def __aenter__(self):
        """Initialize async client when entering async context."""
        self.async_client = requestx.AsyncClient(verify=self.verify_ssl)
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Close async client when exiting async context."""
        if self.async_client:
            await self.async_client.aclose()

    def make_request(self, request: HTTPRequest) -> Dict[str, Any]:
        """Make an HTTP request using the requestx library."""
        try:
            method = request.method.upper()
            url = request.url
            headers = request.headers
            timeout = request.timeout

            data = request.body if request.body else None

            response = self.client.request(method=method, url=url, headers=headers, content=data, timeout=timeout)

            return {
                "status_code": response.status_code,
                "headers": dict(response.headers),
                "content": response.text,
                "response_time": response.elapsed.total_seconds(),
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

    async def make_request_async(self, request: HTTPRequest) -> Dict[str, Any]:
        """Make an async HTTP request using the requestx library."""
        try:
            method = request.method.upper()
            url = request.url
            headers = request.headers
            timeout = request.timeout

            data = request.body if request.body else None

            start_time = time.perf_counter()
            response = await self.async_client.request(method=method, url=url, headers=headers, content=data, timeout=timeout)
            end_time = time.perf_counter()

            return {
                "status_code": response.status_code,
                "headers": dict(response.headers),
                "content": response.text,
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
        """Make a streaming HTTP request using the requestx library."""
        try:
            method = request.method.upper()
            url = request.url
            headers = request.headers
            timeout = request.timeout

            data = request.body if request.body else None

            start_time = time.perf_counter()

            with self.client.stream(method=method, url=url, headers=headers, content=data, timeout=timeout) as response:
                content = b""
                for chunk in response.iter_bytes(chunk_size=8192):
                    if chunk:
                        content += chunk

            end_time = time.perf_counter()

            return {
                "status_code": response.status_code,
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

    async def make_request_stream_async(self, request: HTTPRequest) -> Dict[str, Any]:
        """Make an async streaming HTTP request using the requestx library."""
        try:
            method = request.method.upper()
            url = request.url
            headers = request.headers
            timeout = request.timeout

            data = request.body if request.body else None

            start_time = time.perf_counter()

            async with self.async_client.stream(method=method, url=url, headers=headers, content=data, timeout=timeout) as response:
                content = b""
                async for chunk in response.aiter_bytes(chunk_size=8192):
                    if chunk:
                        content += chunk

            end_time = time.perf_counter()

            return {
                "status_code": response.status_code,
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
