"""Benchmark configuration model for the HTTP benchmark framework."""

import uuid
from typing import Dict, Optional
from .base import BaseModel


class BenchmarkConfiguration(BaseModel):
    """Holds configurable parameters for benchmark execution."""

    def __init__(
        self,
        target_url: str,
        http_method: str = "GET",
        headers: Optional[Dict[str, str]] = None,
        body: Optional[str] = None,
        concurrency: int = 10,
        duration_seconds: int = 30,
        total_requests: Optional[int] = None,
        client_library: str = "requests",
        is_async: bool = False,
        timeout: int = 30,
        verify_ssl: bool = True,
        retry_attempts: int = 3,
        delay_between_requests: float = 0.0,
        name: Optional[str] = None,
        id: Optional[str] = None,
    ):
        self.id = id or str(uuid.uuid4())
        self.name = name or f"Benchmark config for {target_url}"
        self.target_url = target_url
        self.http_method = http_method
        self.headers = headers or {}
        self.body = body or ""
        self.concurrency = concurrency
        self.duration_seconds = duration_seconds
        self.total_requests = total_requests
        self.client_library = client_library
        self.is_async = is_async
        self.timeout = timeout
        self.verify_ssl = verify_ssl
        self.retry_attempts = retry_attempts
        self.delay_between_requests = delay_between_requests
