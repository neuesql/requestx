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
        # Empty pattern matches everything
        if not pattern:
            return {
                "scheme": None,
                "host": None,
                "port": None,
                "path": "",
            }

        # Handle "all://" as matching any scheme
        if pattern.startswith("all://"):
            scheme = None
            rest = pattern[6:]
        else:
            # Parse normally
            parsed = urlparse(pattern)
            scheme = parsed.scheme or None
            rest = pattern[len(scheme) + 3:] if scheme else pattern

        # Empty rest means match any host
        if not rest:
            return {
                "scheme": scheme,
                "host": None,
                "port": None,
                "path": "",
            }

        # Handle wildcards in host
        if rest.startswith("*"):
            host_pattern = rest.split("/")[0] if "/" in rest else rest
            path_pattern = rest[len(host_pattern):] if "/" in rest else ""
            port = None
        else:
            parts = rest.split("/", 1)
            host_with_port = parts[0]
            path_pattern = "/" + parts[1] if len(parts) > 1 else ""

            # Extract port from host
            if ":" in host_with_port:
                host_parts = host_with_port.rsplit(":", 1)
                host_pattern = host_parts[0]
                try:
                    port = int(host_parts[1])
                except ValueError:
                    port = None
            else:
                host_pattern = host_with_port
                port = None

        return {
            "scheme": scheme,
            "host": host_pattern if host_pattern else None,
            "port": port,
            "path": path_pattern,
        }

    def matches(self, url) -> bool:
        """Check if the given URL matches this pattern."""
        # Convert URL object to string if needed
        if hasattr(url, "scheme"):
            url_scheme = url.scheme
            url_host = url.host or ""
            url_port = url.port
            url_path = url.path or ""
        else:
            parsed = urlparse(str(url))
            url_scheme = parsed.scheme
            url_host = parsed.hostname or ""
            url_port = parsed.port
            url_path = parsed.path

        # Check scheme
        if self._parsed["scheme"] is not None:
            if self._parsed["scheme"] != url_scheme:
                return False

        # Check host with wildcard support
        host_pattern = self._parsed["host"]
        if host_pattern is None:
            pass  # None means match any host
        elif host_pattern == "*":
            pass  # Matches any host
        elif host_pattern.startswith("*."):
            # Wildcard subdomain
            suffix = host_pattern[2:]
            if not (url_host == suffix or url_host.endswith("." + suffix)):
                return False
        elif host_pattern != url_host:
            return False

        # Check port if specified in pattern
        port_pattern = self._parsed.get("port")
        if port_pattern is not None:
            if url_port != port_pattern:
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

    def __lt__(self, other: object) -> bool:
        if not isinstance(other, URLPattern):
            return NotImplemented
        # More specific patterns should come first
        # Priority: scheme + host + port > scheme + host > scheme > all
        self_score = self._specificity_score()
        other_score = other._specificity_score()
        # Higher score = more specific = should come first, so reverse comparison
        return self_score > other_score

    def __le__(self, other: object) -> bool:
        if not isinstance(other, URLPattern):
            return NotImplemented
        return self == other or self < other

    def __gt__(self, other: object) -> bool:
        if not isinstance(other, URLPattern):
            return NotImplemented
        return other < self

    def __ge__(self, other: object) -> bool:
        if not isinstance(other, URLPattern):
            return NotImplemented
        return self == other or self > other

    def _specificity_score(self) -> int:
        """Calculate a specificity score for sorting patterns."""
        score = 0
        if self._parsed["scheme"] is not None:
            score += 1
        if self._parsed["host"] is not None:
            score += 2
        if self._parsed.get("port") is not None:
            score += 4
        if self._parsed.get("path"):
            score += 8
        return score


def _is_ip_address(host: str) -> bool:
    """Check if host is an IP address."""
    import ipaddress
    try:
        # Remove brackets for IPv6
        if host.startswith("[") and host.endswith("]"):
            host = host[1:-1]
        ipaddress.ip_address(host)
        return True
    except ValueError:
        return False


def get_environment_proxies() -> typing.Dict[str, typing.Optional[str]]:
    """
    Get proxy settings from environment variables.

    Returns a dictionary mapping URL patterns to proxy URLs.
    For no_proxy entries, the value is None.
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

    # Handle NO_PROXY
    no_proxy = os.environ.get("NO_PROXY") or os.environ.get("no_proxy")
    if no_proxy:
        for host in no_proxy.split(","):
            host = host.strip()
            if not host:
                continue

            # Check if it's a URL (has scheme)
            if "://" in host:
                proxies[host] = None
            elif host.startswith("."):
                # Leading dot means wildcard subdomain
                proxies[f"all://*{host}"] = None
            elif _is_ip_address(host) or "/" in host:
                # IP address or CIDR notation
                if ":" in host and not host.startswith("["):
                    # IPv6 without brackets
                    proxies[f"all://[{host}]"] = None
                else:
                    proxies[f"all://{host}"] = None
            elif host == "localhost" or not "." in host:
                # localhost or single-label hostname - no wildcard
                proxies[f"all://{host}"] = None
            else:
                # Regular domain hostname - add wildcard prefix for subdomains
                proxies[f"all://*{host}"] = None

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


def guess_json_utf(data: bytes) -> typing.Optional[str]:
    """
    Detect the encoding of JSON data based on BOM or null byte patterns.

    JSON can be encoded in UTF-8, UTF-16 (BE/LE), or UTF-32 (BE/LE).
    This function detects the encoding by looking at the byte order mark (BOM)
    or the pattern of null bytes in the first few characters.

    Returns the encoding name suitable for Python's decode(), or None if
    the data appears to be plain UTF-8 (no BOM needed).
    """
    if len(data) < 2:
        return None

    # Check for BOM (Byte Order Mark)
    # UTF-32 BOMs must be checked before UTF-16 since UTF-32 LE starts with FF FE 00 00
    if data[:4] == b'\x00\x00\xfe\xff':
        return 'utf-32-be'
    if data[:4] == b'\xff\xfe\x00\x00':
        return 'utf-32-le'
    if data[:2] == b'\xfe\xff':
        return 'utf-16-be'
    if data[:2] == b'\xff\xfe':
        return 'utf-16-le'
    if data[:3] == b'\xef\xbb\xbf':
        return 'utf-8-sig'

    # No BOM found, detect by null byte patterns
    # JSON must start with ASCII character: { [ " or whitespace
    # Look at the pattern of null bytes in the first 4 bytes

    if len(data) >= 4:
        null_count = sum(1 for b in data[:4] if b == 0)

        # UTF-32: 3 null bytes per character
        if null_count == 3:
            if data[0] == 0 and data[1] == 0 and data[2] == 0:
                return 'utf-32-be'
            if data[1] == 0 and data[2] == 0 and data[3] == 0:
                return 'utf-32-le'

        # UTF-16: 1 null byte per character (for ASCII range)
        if null_count >= 1:
            if data[0] == 0 and data[2] == 0:
                return 'utf-16-be'
            if data[1] == 0 and data[3] == 0:
                return 'utf-16-le'

    elif len(data) >= 2:
        # For shorter data, check UTF-16 patterns
        if data[0] == 0:
            return 'utf-16-be'
        if data[1] == 0:
            return 'utf-16-le'

    # Default to UTF-8 (no special encoding needed)
    return None


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
    "guess_json_utf",
]
