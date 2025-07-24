"""
RequestX - High-performance HTTP client for Python

A drop-in replacement for the requests library, built with Rust for speed and memory safety.
Provides both synchronous and asynchronous APIs while maintaining full compatibility with 
the familiar requests interface.
"""

from ._requestx import (
    # HTTP method functions
    get,
    post,
    put,
    delete,
    head,
    options,
    patch,
    request,
    # Classes
    Response,
    Session,
)

# Version information
__version__ = "0.1.0"
__author__ = "RequestX Team"
__email__ = "team@requestx.dev"

# Public API
__all__ = [
    # HTTP methods
    "get",
    "post", 
    "put",
    "delete",
    "head",
    "options",
    "patch",
    "request",
    # Classes
    "Response",
    "Session",
    # Metadata
    "__version__",
]

# Compatibility aliases (for requests compatibility)
# These can be used for drop-in replacement
def session():
    """Create a new Session object for persistent connections."""
    return Session()