"""Urllib3 HTTP client adapter for the HTTP benchmark framework."""

import urllib3
import time
from typing import Dict, Any
from .base import BaseHTTPAdapter
from ..models.http_request import HTTPRequest


class Urllib3Adapter(BaseHTTPAdapter):
    """HTTP adapter for the urllib3 library."""

    def __init__(self):
        super().__init__("urllib3")
        self.pool = None
        self.pool_no_verify = None
        # Disable SSL warnings if not verifying SSL
        urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

    def __enter__(self):
        """Initialize pool managers when entering sync context."""
        urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)
        self.pool = urllib3.PoolManager()
        self.pool_no_verify = urllib3.PoolManager(cert_reqs="CERT_NONE")
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Close pool managers when exiting sync context."""
        if self.pool:
            self.pool.clear()
        if self.pool_no_verify:
            self.pool_no_verify.clear()

    def make_request(self, request: HTTPRequest) -> Dict[str, Any]:
        """Make an HTTP request using the urllib3 library."""
        try:
            method = request.method.upper()
            url = request.url
            headers = request.headers
            timeout = request.timeout
            verify_ssl = request.verify_ssl

            http = self.pool if verify_ssl else self.pool_no_verify

            body = request.body if request.body else None

            start_time = time.time()
            response = http.request(method=method, url=url, headers=headers, body=body, timeout=timeout)
            end_time = time.time()

            return {
                "status_code": response.status,
                "headers": dict(response.headers),
                "content": response.data.decode("utf-8"),
                "response_time": end_time - start_time,
                "url": url,
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
        """Make an async HTTP request using the urllib3 library."""
        raise NotImplementedError("urllib3 is sync-only, use make_request instead")

    def make_request_stream(self, request: HTTPRequest) -> Dict[str, Any]:
        """Make a streaming HTTP request using the urllib3 library."""
        try:
            method = request.method.upper()
            url = request.url
            headers = request.headers
            timeout = request.timeout
            verify_ssl = request.verify_ssl

            http = self.pool if verify_ssl else self.pool_no_verify

            body = request.body if request.body else None

            start_time = time.time()

            # urllib3 doesn't have native streaming, but we can simulate it
            # by reading the response in chunks
            response = http.request(method=method, url=url, headers=headers, body=body, timeout=timeout, preload_content=False)

            content = b""
            chunk_count = 0
            for chunk in response.stream(8192):
                if chunk:
                    content += chunk
                    chunk_count += 1

            response.release_conn()

            end_time = time.time()

            return {
                "status_code": response.status,
                "headers": dict(response.headers),
                "content": content.decode("utf-8") if content else "",
                "response_time": end_time - start_time,
                "url": url,
                "success": True,
                "error": None,
                "streamed": True,
                "chunk_count": chunk_count,
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
        """Make an async streaming HTTP request using the urllib3 library."""
        raise NotImplementedError("urllib3 is sync-only, use make_request_stream instead")
