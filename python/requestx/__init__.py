"""
RequestX - High-performance HTTP client for Python

A drop-in replacement for the requests library, built with Rust for speed and memory safety.
Provides both synchronous and asynchronous APIs while maintaining full compatibility with
the familiar requests interface.
"""

from ._requestx import (
    # Classes
    Response as _Response,
    CaseInsensitivePyDict,
)
from ._requestx import (
    Session,
)
from ._requestx import (
    delete as _delete,
)
from ._requestx import (
    # HTTP method functions
    get as _get,
)
from ._requestx import (
    head as _head,
)
from ._requestx import (
    options as _options,
)
from ._requestx import (
    patch as _patch,
)
from ._requestx import (
    post as _post,
)
from ._requestx import (
    put as _put,
)
from ._requestx import (
    request as _request,
)

# Auth classes and utilities
from ._requestx import (
    HTTPDigestAuth,
)
from ._requestx import (
    HTTPProxyAuth,
)
from ._requestx import (
    get_auth_from_url,
)
from ._requestx import (
    urldefragauth,
)

# Status codes module (matches requests.codes)
class _Codes:
    """HTTP status codes with human-readable names.
    
    Examples:
        >>> requestx.codes.ok
        200
        >>> requestx.codes.not_found
        404
        >>> requestx.codes['temporary_redirect']
        307
    """
    
    # Informational
    continue_100 = 100
    switching_protocols = 101
    processing = 102
    
    # Success
    ok = 200
    created = 201
    accepted = 202
    non_authoritative_information = 203
    no_content = 204
    reset_content = 205
    partial_content = 206
    multi_status = 207
    already_reported = 208
    im_used = 226
    
    # Redirection
    multiple_choices = 300
    moved_permanently = 301
    found = 302
    see_other = 303
    not_modified = 304
    use_proxy = 305
    temporary_redirect = 307
    permanent_redirect = 308
    
    # Client Error
    bad_request = 400
    unauthorized = 401
    payment_required = 402
    forbidden = 403
    not_found = 404
    method_not_allowed = 405
    not_acceptable = 406
    proxy_authentication_required = 407
    request_timeout = 408
    conflict = 409
    gone = 410
    length_required = 411
    precondition_failed = 412
    payload_too_large = 413
    uri_too_long = 414
    unsupported_media_type = 415
    range_not_satisfiable = 416
    expectation_failed = 417
    misdirected_request = 421
    unprocessable_entity = 422
    locked = 423
    failed_dependency = 424
    upgrade_required = 426
    precondition_required = 428
    too_many_requests = 429
    request_header_fields_too_large = 431
    unavailable_for_legal_reasons = 451
    
    # Server Error
    internal_server_error = 500
    not_implemented = 501
    bad_gateway = 502
    service_unavailable = 503
    gateway_timeout = 504
    http_version_not_supported = 505
    variant_also_negotiates = 506
    insufficient_storage = 507
    loop_detected = 508
    not_extended = 510
    network_authentication_required = 511
    
    # Common aliases
    # 1xx
    informational = continue_100
    # 2xx
    success = ok
    # 3xx
    redirection = multiple_choices
    redirect = multiple_choices
    moved = moved_permanently
    found_redirect = found
    # 4xx
    client_error = bad_request
    unauthorized_401 = unauthorized
    forbidden_403 = forbidden
    not_found_404 = not_found
    # 5xx
    server_error = internal_server_error
    bad_gateway_502 = bad_gateway
    service_unavailable_503 = service_unavailable
    
    def __getitem__(self, key):
        """Allow dict-like access: codes['not_found']"""
        if hasattr(self, key):
            return getattr(self, key)
        raise KeyError(key)
    
    def __contains__(self, key):
        """Allow 'in' operator"""
        return hasattr(self, key)
    
    def get(self, key, default=None):
        """Allow .get() method"""
        return getattr(self, key, default)


# Create a single instance
codes = _Codes()

# CaseInsensitiveDict - exported as CaseInsensitiveDict
CaseInsensitiveDict = CaseInsensitivePyDict


class Retry:
    """Retry configuration for HTTP requests.
    
    This class matches the requests Retry interface and controls how many times
    a request should be retried when certain errors occur.
    
    Examples:
        >>> import requestx
        >>> from requestx import Retry
        >>> retry = Retry(total=3, backoff_factor=0.1, status_forcelist=[502, 503, 504])
        >>> adapter = HTTPAdapter(max_retries=retry)
        >>> s = requestx.Session()
        >>> s.mount('https://', adapter)
    
    Attributes:
        total: Total number of retries allowed.
        connect: Number of retries for connection errors.
        read: Number of retries for read errors.
        status_forcelist: A set of HTTP status codes to retry on.
        allowed_methods: A set of HTTP methods to retry on.
        backoff_factor: Factor to apply between retry attempts.
        raise_on_redirect: Whether to raise on redirect (not implemented).
        raise_on_status: Whether to raise on status errors (not implemented).
    """
    
    DEFAULT_METHODS = frozenset(['GET', 'HEAD', 'OPTIONS', 'PUT', 'DELETE', 'TRACE'])
    RETRY_AFTER_STATUS_CODES = frozenset([413, 429, 503])
    
    def __init__(
        self,
        total=0,
        connect=0,
        read=0,
        status_forcelist=None,
        allowed_methods=None,
        backoff_factor=0.1,
        raise_on_redirect=False,
        raise_on_status=False,
        history=None,
        respect_retry_after_header=True,
    ):
        """Initialize a Retry configuration.
        
        Args:
            total: Total number of retries allowed.
            connect: Number of retries for connection errors.
            read: Number of retries for read errors.
            status_forcelist: A set of HTTP status codes to retry on.
            allowed_methods: A set of HTTP methods to retry on.
            backoff_factor: Factor to apply between retry attempts.
            raise_on_redirect: Whether to raise on redirect.
            raise_on_status: Whether to raise on status errors.
            history: Tuple of historical retries (not used internally).
            respect_retry_after_header: Whether to respect Retry-After header.
        """
        self.total = total
        self.connect = connect
        self.read = read
        # Default status_forcelist includes 502, 503, 504 (server errors that warrant retry)
        self.status_forcelist = status_forcelist if status_forcelist is not None else frozenset([502, 503, 504])
        self.allowed_methods = allowed_methods or self.DEFAULT_METHODS
        self.backoff_factor = backoff_factor
        self.raise_on_redirect = raise_on_redirect
        self.raise_on_status = raise_on_status
        self.history = history or ()
        self.respect_retry_after_header = respect_retry_after_header
    
    def new(self, **kwargs):
        """Create a new Retry instance with updated parameters."""
        return Retry(
            total=kwargs.get('total', self.total),
            connect=kwargs.get('connect', self.connect),
            read=kwargs.get('read', self.read),
            status_forcelist=kwargs.get('status_forcelist', self.status_forcelist),
            allowed_methods=kwargs.get('allowed_methods', self.allowed_methods),
            backoff_factor=kwargs.get('backoff_factor', self.backoff_factor),
            raise_on_redirect=kwargs.get('raise_on_redirect', self.raise_on_redirect),
            raise_on_status=kwargs.get('raise_on_status', self.raise_on_status),
            history=kwargs.get('history', self.history),
            respect_retry_after_header=kwargs.get('respect_retry_after_header', self.respect_retry_after_header),
        )
    
    def __repr__(self):
        return f'Retry(total={self.total}, connect={self.connect}, read={self.read}, backoff_factor={self.backoff_factor})'


class HTTPAdapter:
    """HTTP adapter with retry configuration.
    
    This class provides request retry functionality similar to requests.HTTPAdapter.
    It can be mounted on a Session to apply retry logic to specific URL prefixes.
    
    Examples:
        >>> import requestx
        >>> from requestx import Retry, HTTPAdapter
        >>> retry = Retry(total=3, status_forcelist=[502, 503, 504])
        >>> adapter = HTTPAdapter(max_retries=retry)
        >>> s = requestx.Session()
        >>> s.mount('http://', adapter)
        >>> s.mount('https://', adapter)
    
    Attributes:
        max_retries: Retry configuration (Retry instance or int).
    """
    
    def __init__(self, max_retries=0):
        """Initialize the HTTP adapter.
        
        Args:
            max_retries: Either an integer (simple retry count) or a Retry instance.
        """
        if isinstance(max_retries, int):
            self.max_retries = Retry(total=max_retries)
        else:
            self.max_retries = max_retries
    
    def __repr__(self):
        return f'HTTPAdapter(max_retries={self.max_retries})'


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


class ContentDecodingError(RequestException):
    """Failed to decode response content."""

    pass


class StreamConsumedError(RequestException):
    """The content for this response was already consumed."""


class UnrewindableBodyError(RequestException):
    """The request body could not be rewinded to a position it was at."""


# InvalidJSONError is an alias for JSONDecodeError for compatibility
InvalidJSONError = JSONDecodeError


class FileModeWarning(RequestException):
    """A file was opened in text mode, but binary mode was expected."""

    pass


class RequestsWarning(UserWarning):
    """Base warning for requests."""

    pass


class DependencyWarning(RequestsWarning):
    """Warning about a dependency issue."""

    pass


# =============================================================================
# Cookie Utilities Module
# =============================================================================

class _CookieJarWrapper:
    """Internal wrapper for cookie jar functionality."""
    
    def __init__(self):
        self._cookies = {}
    
    def get(self, key, default=None):
        """Get a cookie value."""
        return self._cookies.get(key, default)
    
    def set(self, key, value, **kwargs):
        """Set a cookie value."""
        self._cookies[key] = value
    
    def __contains__(self, key):
        """Check if a cookie exists."""
        return key in self._cookies
    
    def __getitem__(self, key):
        """Get a cookie value by key."""
        return self._cookies[key]
    
    def __setitem__(self, key, value):
        """Set a cookie value."""
        self._cookies[key] = value
    
    def __delitem__(self, key):
        """Delete a cookie."""
        del self._cookies[key]
    
    def __iter__(self):
        """Iterate over cookie keys."""
        return iter(self._cookies)
    
    def __len__(self):
        """Return the number of cookies."""
        return len(self._cookies)
    
    def keys(self):
        """Return cookie keys."""
        return self._cookies.keys()
    
    def values(self):
        """Return cookie values."""
        return self._cookies.values()
    
    def items(self):
        """Return cookie items."""
        return self._cookies.items()


class _CookiesModule:
    """Cookie utilities module."""
    
    @staticmethod
    def cookiejar_from_dict(cookie_dict=None, cookiejar=None):
        """Create a CookieJar from a dictionary.
        
        Args:
            cookie_dict: Dictionary of cookies to add.
            cookiejar: Optional existing cookiejar to add to.
        
        Returns:
            A cookie jar-like object with the cookies from cookie_dict.
        
        Examples:
            >>> jar = cookies.cookiejar_from_dict({"key": "value"})
            >>> jar["key"]
            'value'
        """
        jar = cookiejar or _CookieJarWrapper()
        if cookie_dict:
            for key, value in cookie_dict.items():
                jar.set(key, value)
        return jar
    
    @staticmethod
    def dict_from_cookiejar(cookiejar):
        """Convert a CookieJar to a dictionary.
        
        Args:
            cookiejar: A cookie jar-like object.
        
        Returns:
            A dictionary containing the cookies.
        
        Examples:
            >>> jar = _CookieJarWrapper()
            >>> jar.set("key", "value")
            >>> cookies.dict_from_cookiejar(jar)
            {'key': 'value'}
        """
        if hasattr(cookiejar, '_cookies'):
            return dict(cookiejar._cookies)
        elif hasattr(cookiejar, 'items'):
            return dict(cookiejar.items())
        elif hasattr(cookiejar, '__iter__'):
            result = {}
            for key in cookiejar:
                val = cookiejar.get(key) if hasattr(cookiejar, 'get') else None
                if val is not None:
                    result[key] = val
            return result
        return {}
    
    @staticmethod
    def merge_cookies(cookiejar, cookies):
        """Merge cookies into a CookieJar.
        
        Args:
            cookiejar: The target cookie jar (dict or cookiejar-like object).
            cookies: Cookies to add (dict or cookiejar-like).
        
        Returns:
            The merged cookie jar.
        
        Examples:
            >>> jar = cookies.cookiejar_from_dict({"existing": "value"})
            >>> jar = cookies.merge_cookies(jar, {"new": "value2"})
            >>> jar["existing"]
            'value'
            >>> jar["new"]
            'value2'
        """
        if hasattr(cookies, 'items'):
            for key, value in cookies.items():
                # Handle dict objects
                if isinstance(cookiejar, dict):
                    cookiejar[key] = value
                # Handle cookiejar-like objects with set() method
                elif hasattr(cookiejar, 'set'):
                    cookiejar.set(key, value)
        return cookiejar
    
    @staticmethod
    def add_dict_to_cookiejar(cookiejar, cookie_dict):
        """Add a dictionary of cookies to a CookieJar.
        
        Args:
            cookiejar: The target cookie jar.
            cookie_dict: Dictionary of cookies to add.
        
        Returns:
            The modified cookie jar.
        
        Examples:
            >>> jar = _CookieJarWrapper()
            >>> cookies.add_dict_to_cookiejar(jar, {"key": "value"})
        """
        return _CookiesModule.merge_cookies(cookiejar, cookie_dict)


# Create the cookies module instance
cookies = _CookiesModule()


# =============================================================================
# URL Utilities
# =============================================================================

def requote_uri(uri):
    """Requote a URI.
    
    This function quotes URL path components and unicode characters
    using the quoting rules from requests library.
    
    Args:
        uri: The URI to requote.
    
    Returns:
        The requoted URI.
    
    Examples:
        >>> requote_uri("http://example.com/path with spaces")
        'http://example.com/path%20with%20spaces'
    """
    import re
    from urllib.parse import quote, unquote
    
    # Unquote first to avoid double-quoting
    uri = unquote(uri)
    
    # Re-quote using safe characters for URI
    # This is a simplified version that handles the common cases
    return quote(uri, safe='/:@!$&\'()*+,;=-_.~')


# Version information
__version__ = "0.3.0"
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
    "Retry",
    "HTTPAdapter",
    # Auth classes
    "HTTPDigestAuth",
    "HTTPProxyAuth",
    # Auth utilities
    "get_auth_from_url",
    "urldefragauth",
    # Advanced features
    "codes",
    "CaseInsensitiveDict",
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
    "UnrewindableBodyError",
    "InvalidJSONError",
    "FileModeWarning",
    "RequestsWarning",
    "DependencyWarning",
    # Cookie utilities
    "cookies",
    # URL utilities
    "requote_uri",
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
        elif "Invalid HTTP method:" in error_msg:
            # Map invalid HTTP method errors to RuntimeError for test compatibility
            return builtins.RuntimeError(error_msg)
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


# Monkey patch the Response class to map exceptions and add streaming generators
_original_raise_for_status = _Response.raise_for_status
_original_json = _Response.json
# Store the original Rust methods before monkey-patching
_original_iter_content_rust = _Response.iter_content
_original_iter_lines_rust = _Response.iter_lines


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


def _iter_content_generator(self, chunk_size=512):
    """Generator that yields chunks from the response content.

    This provides true streaming behavior when iterating over large responses.
    Each chunk is decoded bytes of the specified size.
    """
    yield from _original_iter_content_rust(self, chunk_size)


def _iter_lines_generator(self):
    """Generator that yields lines from the response content.

    This provides line-by-line iteration over the response body,
    useful for processing large text streams or SSE responses.
    """
    yield from _original_iter_lines_rust(self)


_Response.raise_for_status = _wrapped_raise_for_status
_Response.json = _wrapped_json
_Response.iter_content = _iter_content_generator
_Response.iter_lines = _iter_lines_generator
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
