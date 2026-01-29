# RequestX - Utility functions and classes

import os
import re
import typing
from urllib.parse import urlparse


class URLPattern:
    """
    A pattern for matching URLs.

    Example usage:
        pattern = URLPattern("https://example.com/*")
        pattern.matches(URL("https://example.com/path"))  # True
        pattern.matches(URL("http://example.com/path"))   # False
    """

    def __init__(self, pattern: str) -> None:
        self._pattern = pattern
        self._parsed = self._parse_pattern(pattern)

    def _parse_pattern(self, pattern: str) -> dict:
        """Parse the URL pattern into components."""
        # Handle "all://" as matching any scheme
        if pattern.startswith("all://"):
            scheme = None
            rest = pattern[6:]
        else:
            # Parse normally
            parsed = urlparse(pattern)
            scheme = parsed.scheme or None
            rest = pattern[len(scheme) + 3:] if scheme else pattern

        # Handle wildcards in host
        if rest.startswith("*"):
            host_pattern = rest.split("/")[0] if "/" in rest else rest
            path_pattern = rest[len(host_pattern):] if "/" in rest else ""
        else:
            parts = rest.split("/", 1)
            host_pattern = parts[0]
            path_pattern = "/" + parts[1] if len(parts) > 1 else ""

        return {
            "scheme": scheme,
            "host": host_pattern,
            "path": path_pattern,
        }

    def matches(self, url) -> bool:
        """Check if the given URL matches this pattern."""
        # Convert URL object to string if needed
        if hasattr(url, "scheme"):
            url_scheme = url.scheme
            url_host = url.host or ""
            url_path = url.path or ""
        else:
            parsed = urlparse(str(url))
            url_scheme = parsed.scheme
            url_host = parsed.netloc
            url_path = parsed.path

        # Check scheme
        if self._parsed["scheme"] is not None:
            if self._parsed["scheme"] != url_scheme:
                return False

        # Check host with wildcard support
        host_pattern = self._parsed["host"]
        if host_pattern == "*":
            pass  # Matches any host
        elif host_pattern.startswith("*."):
            # Wildcard subdomain
            suffix = host_pattern[2:]
            if not (url_host == suffix or url_host.endswith("." + suffix)):
                return False
        elif host_pattern != url_host:
            return False

        # Check path with wildcard support
        path_pattern = self._parsed["path"]
        if path_pattern == "" or path_pattern == "*" or path_pattern == "/*":
            pass  # Matches any path
        elif path_pattern.endswith("*"):
            prefix = path_pattern[:-1]
            if not url_path.startswith(prefix):
                return False
        elif path_pattern != url_path:
            return False

        return True

    @property
    def pattern(self) -> str:
        return self._pattern

    def __repr__(self) -> str:
        return f"URLPattern({self._pattern!r})"

    def __eq__(self, other: object) -> bool:
        if isinstance(other, URLPattern):
            return self._pattern == other._pattern
        return False

    def __hash__(self) -> int:
        return hash(self._pattern)


def get_environment_proxies() -> typing.Dict[str, typing.Optional[str]]:
    """
    Get proxy settings from environment variables.

    Returns a dictionary with 'http', 'https', and 'all' keys.
    """
    proxies: typing.Dict[str, typing.Optional[str]] = {}

    # Check for HTTP proxy
    http_proxy = os.environ.get("HTTP_PROXY") or os.environ.get("http_proxy")
    if http_proxy:
        proxies["http://"] = http_proxy

    # Check for HTTPS proxy
    https_proxy = os.environ.get("HTTPS_PROXY") or os.environ.get("https_proxy")
    if https_proxy:
        proxies["https://"] = https_proxy

    # Check for ALL proxy
    all_proxy = os.environ.get("ALL_PROXY") or os.environ.get("all_proxy")
    if all_proxy:
        proxies["all://"] = all_proxy

    return proxies


def get_no_proxy_list() -> typing.List[str]:
    """Get the list of hosts that should not use a proxy."""
    no_proxy = os.environ.get("NO_PROXY") or os.environ.get("no_proxy") or ""
    return [host.strip() for host in no_proxy.split(",") if host.strip()]


def should_not_use_proxy(url: str, no_proxy_list: typing.Optional[typing.List[str]] = None) -> bool:
    """
    Check if a URL should bypass the proxy based on NO_PROXY settings.
    """
    if no_proxy_list is None:
        no_proxy_list = get_no_proxy_list()

    if not no_proxy_list:
        return False

    parsed = urlparse(url)
    host = parsed.netloc.lower()

    # Remove port from host for comparison
    if ":" in host:
        host = host.split(":")[0]

    for no_proxy in no_proxy_list:
        no_proxy = no_proxy.lower().strip()

        # Handle "*" meaning no proxy for anything
        if no_proxy == "*":
            return True

        # Handle leading dot (e.g., ".example.com")
        if no_proxy.startswith("."):
            if host.endswith(no_proxy) or host == no_proxy[1:]:
                return True
        else:
            # Exact match or subdomain match
            if host == no_proxy or host.endswith("." + no_proxy):
                return True

    return False


def is_https_redirect(url: str, location: str) -> bool:
    """
    Check if a redirect from 'url' to 'location' is an HTTPS upgrade.
    """
    url_parsed = urlparse(url)
    location_parsed = urlparse(location)

    # Must be HTTP -> HTTPS
    if url_parsed.scheme != "http" or location_parsed.scheme != "https":
        return False

    # Host must match
    if url_parsed.netloc.lower() != location_parsed.netloc.lower():
        return False

    # Path must match
    if url_parsed.path != location_parsed.path:
        return False

    return True


def same_origin(url1: str, url2: str) -> bool:
    """
    Check if two URLs have the same origin (scheme + host + port).
    """
    parsed1 = urlparse(url1)
    parsed2 = urlparse(url2)

    # Compare scheme
    if parsed1.scheme != parsed2.scheme:
        return False

    # Compare host (case-insensitive)
    if parsed1.hostname and parsed2.hostname:
        if parsed1.hostname.lower() != parsed2.hostname.lower():
            return False
    elif parsed1.hostname != parsed2.hostname:
        return False

    # Compare port (use default ports if not specified)
    port1 = parsed1.port
    port2 = parsed2.port

    if port1 is None:
        port1 = 443 if parsed1.scheme == "https" else 80
    if port2 is None:
        port2 = 443 if parsed2.scheme == "https" else 80

    return port1 == port2


def normalize_header_key(key: str) -> str:
    """Normalize a header key to title case."""
    return "-".join(word.capitalize() for word in key.split("-"))


def normalize_header_value(value: str) -> str:
    """Normalize a header value by stripping whitespace."""
    return value.strip()


def parse_content_type(content_type: str) -> typing.Tuple[str, typing.Dict[str, str]]:
    """
    Parse a Content-Type header value.

    Returns (media_type, parameters).
    """
    parts = content_type.split(";")
    media_type = parts[0].strip().lower()

    params = {}
    for part in parts[1:]:
        part = part.strip()
        if "=" in part:
            key, value = part.split("=", 1)
            # Remove quotes if present
            value = value.strip('"\'')
            params[key.strip().lower()] = value

    return media_type, params


def get_encoding_from_content_type(content_type: str) -> typing.Optional[str]:
    """Extract the charset/encoding from a Content-Type header."""
    _, params = parse_content_type(content_type)
    return params.get("charset")


# Re-export at module level for direct access
__all__ = [
    "URLPattern",
    "get_environment_proxies",
    "get_no_proxy_list",
    "should_not_use_proxy",
    "is_https_redirect",
    "same_origin",
    "normalize_header_key",
    "normalize_header_value",
    "parse_content_type",
    "get_encoding_from_content_type",
]
