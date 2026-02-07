"""HTTP request model for the HTTP benchmark framework."""

import uuid
from typing import Dict, Optional
from .base import BaseModel


class HTTPRequest(BaseModel):
    """Represents an HTTP request with method, URL, headers, and body for benchmarking."""

    def __init__(
        self,
        method: str,
        url: str,
        headers: Optional[Dict[str, str]] = None,
        body: Optional[str] = None,
        timeout: int = 30,
        verify_ssl: bool = True,
        stream: bool = False,
        id: Optional[str] = None,
    ):
        self.id = id or str(uuid.uuid4())
        self.method = method
        self.url = url
        self.headers = headers or {}
        self.body = body or ""
        self.timeout = timeout
        self.verify_ssl = verify_ssl
        self.stream = stream
