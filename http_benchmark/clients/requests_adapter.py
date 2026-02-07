"""Requests HTTP client adapter for the HTTP benchmark framework."""

import requests
from typing import Dict, Any
from .base import BaseHTTPAdapter
from ..models.http_request import HTTPRequest


class RequestsAdapter(BaseHTTPAdapter):
    """HTTP adapter for the requests library."""

    def __init__(self):
        super().__init__("requests")
        self.session = None

    def __enter__(self):
        """Initialize session when entering sync context."""
        self.session = requests.Session()
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Close session when exiting sync context."""
        if self.session:
            self.session.close()

    def make_request(self, request: HTTPRequest) -> Dict[str, Any]:
        """Make an HTTP request using the requests library."""
        try:
            method = request.method.upper()
            url = request.url
            headers = request.headers
            timeout = request.timeout
            verify_ssl = request.verify_ssl

            data = request.body if request.body else None

            response = self.session.request(
                method=method,
                url=url,
                headers=headers,
                data=data,
                timeout=timeout,
                verify=verify_ssl,
            )

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
        """Make an async HTTP request using the requests library."""
        raise NotImplementedError("requests is sync-only")

    def make_request_stream(self, request: HTTPRequest) -> Dict[str, Any]:
        """Make a streaming HTTP request using the requests library."""
        try:
            method = request.method.upper()
            url = request.url
            headers = request.headers
            timeout = request.timeout
            verify_ssl = request.verify_ssl

            data = request.body if request.body else None

            response = self.session.request(
                method=method,
                url=url,
                headers=headers,
                data=data,
                timeout=timeout,
                verify=verify_ssl,
                stream=True,
            )

            # Read content from stream
            content = b""
            for chunk in response.iter_content(chunk_size=8192, decode_unicode=True):
                if chunk:
                    content += chunk.encode("utf-8") if isinstance(chunk, str) else chunk

            return {
                "status_code": response.status_code,
                "headers": dict(response.headers),
                "content": content.decode("utf-8") if content else "",
                "response_time": response.elapsed.total_seconds(),
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
        """Make an async streaming HTTP request using the requests library."""
        raise NotImplementedError("requests is sync-only, use make_request_stream instead")
