"""
RequestX - High-performance HTTP client for Python

A drop-in replacement for the requests library, built with Rust for speed and memory safety.
Provides both synchronous and asynchronous APIs while maintaining full compatibility with
the familiar requests interface.
"""

from ._requestx import (
    # HTTP method functions
    get as _get,
    post as _post,
    put as _put,
    delete as _delete,
    head as _head,
    options as _options,
    patch as _patch,
    request as _request,
    # Classes
    Response as _Response,
    Session,
)


# Exception hierarchy matching requests library
class RequestException(Exception):
    """Base exception for all requestx errors.

    This is the base exception class for all errors that occur during
    HTTP requests. It matches the requests.RequestException interface.
    """

    pass


class ConnectionError(RequestException):
    """A connection error occurred.

    This exception is raised when there are network-level connection
    problems, such as DNS resolution failures, connection timeouts,
    or connection refused errors.
    """

    pass


class HTTPError(RequestException):
    """An HTTP error occurred.

    This exception is raised when an HTTP request returns an unsuccessful
    status code (4xx or 5xx). It matches the requests.HTTPError interface.
    """

    pass


class URLRequired(RequestException):
    """A valid URL is required to make a request."""

    pass


class TooManyRedirects(RequestException):
    """Too many redirects were encountered."""

    pass


class Timeout(RequestException):
    """The request timed out.

    This is the base timeout exception. More specific timeout exceptions
    inherit from this class.
    """

    pass


class ConnectTimeout(ConnectionError, Timeout):
    """The request timed out while trying to connect to the remote server."""

    pass


class ReadTimeout(Timeout):
    """The server did not send any data in the allotted amount of time."""

    pass


class JSONDecodeError(RequestException):
    """Failed to decode JSON response."""

    pass


class InvalidURL(RequestException):
    """The URL provided was invalid."""

    pass


class InvalidHeader(RequestException):
    """The header provided was invalid."""

    pass


class SSLError(ConnectionError):
    """An SSL/TLS error occurred."""

    pass


class ProxyError(ConnectionError):
    """A proxy error occurred."""

    pass


class RetryError(RequestException):
    """Custom retries logic failed."""

    pass


class UnreachableCodeError(RequestException):
    """Unreachable code was executed."""

    pass


class InvalidSchema(RequestException):
    """The URL schema (e.g. http or https) is invalid."""

    pass


class MissingSchema(RequestException):
    """The URL schema (e.g. http or https) is missing."""

    pass


class ChunkedEncodingError(ConnectionError):
    """The server declared chunked encoding but sent an invalid chunk."""

    pass


class ContentDecodingError(RequestException):
    """Failed to decode response content."""

    pass


class StreamConsumedError(RequestException):
    """The content for this response was already consumed."""

    pass


class FileModeWarning(RequestException):
    """A file was opened in text mode, but binary mode was expected."""

    pass


class RequestsWarning(UserWarning):
    """Base warning for requests."""

    pass


class DependencyWarning(RequestsWarning):
    """Warning about a dependency issue."""

    pass


# Version information
__version__ = "0.2.0"
__author__ = "RequestX Team"
__email__ = "wu.qunfei@gmail.com"

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
    # Exceptions
    "RequestException",
    "ConnectionError",
    "HTTPError",
    "URLRequired",
    "TooManyRedirects",
    "ConnectTimeout",
    "ReadTimeout",
    "Timeout",
    "JSONDecodeError",
    "InvalidURL",
    "InvalidHeader",
    "SSLError",
    "ProxyError",
    "RetryError",
    "UnreachableCodeError",
    "InvalidSchema",
    "MissingSchema",
    "ChunkedEncodingError",
    "ContentDecodingError",
    "StreamConsumedError",
    "FileModeWarning",
    "RequestsWarning",
    "DependencyWarning",
    # Metadata
    "__version__",
]


# Exception mapping functions
def _map_exception(e):
    """Map basic Python exceptions to requestx exceptions."""
    import builtins

    if isinstance(e, builtins.ValueError):
        error_msg = str(e)
        if "Invalid URL" in error_msg:
            return InvalidURL(error_msg)
        elif "Invalid header" in error_msg:
            return InvalidHeader(error_msg)
        elif "URL required" in error_msg or "A valid URL is required" in error_msg:
            return URLRequired(error_msg)
        elif "Invalid URL schema" in error_msg:
            return InvalidSchema(error_msg)
        elif "No connection adapters" in error_msg:
            return MissingSchema(error_msg)
        elif "JSON" in error_msg or "decode" in error_msg:
            return JSONDecodeError(error_msg)
        else:
            return RequestException(error_msg)
    elif isinstance(e, builtins.ConnectionError):
        return ConnectionError(str(e))
    elif isinstance(e, builtins.TimeoutError):
        return ReadTimeout(str(e))
    elif isinstance(e, builtins.RuntimeError):
        error_msg = str(e)
        if "Client Error" in error_msg:
            return HTTPError(error_msg)
        elif "Too many redirects" in error_msg:
            return TooManyRedirects(error_msg)
        else:
            return RequestException(error_msg)
    else:
        return RequestException(str(e))


def _wrap_request_function(func):
    """Wrap a request function to map exceptions."""

    def wrapper(*args, **kwargs):
        try:
            return func(*args, **kwargs)
        except Exception as e:
            raise _map_exception(e) from e

    return wrapper


# Monkey patch the Response class to map exceptions
_original_raise_for_status = _Response.raise_for_status
_original_json = _Response.json


def _wrapped_raise_for_status(self):
    """Raise HTTPError for bad status codes."""
    try:
        return _original_raise_for_status(self)
    except Exception as e:
        raise _map_exception(e) from e


def _wrapped_json(self, *args, **kwargs):
    """Parse JSON response with proper exception mapping."""
    try:
        return _original_json(self, *args, **kwargs)
    except Exception as e:
        raise _map_exception(e) from e


_Response.raise_for_status = _wrapped_raise_for_status
_Response.json = _wrapped_json
Response = _Response

# Wrapped HTTP method functions
get = _wrap_request_function(_get)
post = _wrap_request_function(_post)
put = _wrap_request_function(_put)
delete = _wrap_request_function(_delete)
head = _wrap_request_function(_head)
options = _wrap_request_function(_options)
patch = _wrap_request_function(_patch)
request = _wrap_request_function(_request)


# Compatibility aliases (for requests compatibility)
# These can be used for drop-in replacement
def session():
    """Create a new Session object for persistent connections."""
    return Session()
