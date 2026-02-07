"""Base HTTP client adapter for the HTTP benchmark framework."""

from abc import ABC, abstractmethod
from typing import Dict, Any
from ..models.http_request import HTTPRequest


class BaseHTTPAdapter(ABC):
    """Base class for all HTTP client adapters."""

    def __init__(self, name: str):
        self.name = name
        self._session = None

    @abstractmethod
    def make_request(self, request: HTTPRequest) -> Dict[str, Any]:
        """Make an HTTP request and return response data."""
        pass

    @abstractmethod
    async def make_request_async(self, request: HTTPRequest) -> Dict[str, Any]:
        """Make an async HTTP request and return response data."""
        pass

    @abstractmethod
    def make_request_stream(self, request: HTTPRequest) -> Dict[str, Any]:
        """Make a streaming HTTP request and return response data with stream info."""
        pass

    @abstractmethod
    async def make_request_stream_async(self, request: HTTPRequest) -> Dict[str, Any]:
        """Make an async streaming HTTP request and return response data with stream info."""
        pass

    def __enter__(self):
        """Initialize session when entering sync context."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Close session when exiting sync context."""
        pass

    async def __aenter__(self):
        """Initialize session when entering async context."""
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Close session when exiting async context."""
        pass
