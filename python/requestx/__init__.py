"""
RequestX - High-performance HTTP client for Python with requests-compatible API
"""

from .requestx import (
    get,
    post,
    put,
    delete,
    head,
    options,
    patch,
    request,
    Response,
    Session,
)

__version__ = "0.1.0"
__author__ = "RequestX Team"
__email__ = "team@requestx.dev"

__all__ = [
    "get",
    "post", 
    "put",
    "delete",
    "head",
    "options",
    "patch",
    "request",
    "Response",
    "Session",
]