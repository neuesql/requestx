"""
Internal utility classes for requestx (HTTPX compatibility).
"""

import os
import re
import typing


class URLPattern:
    """
    URL pattern matching for proxy configuration.

    Patterns can include:
    - "all://" to match any scheme
    - "http://" or "https://" for specific schemes
    - Domain names with optional ports
    """

    def __init__(self, pattern: str) -> None:
        self.pattern = pattern
        self._parsed = self._parse_pattern(pattern)

    def _parse_pattern(self, pattern: str) -> dict:
        """Parse pattern into components."""
        result = {
            "scheme": None,
            "host": None,
            "port": None,
        }

        if not pattern:
            return result

        # Handle scheme
        if "://" in pattern:
            scheme, rest = pattern.split("://", 1)
            if scheme == "all":
                result["scheme"] = None  # Match any scheme
            else:
                result["scheme"] = scheme
        else:
            rest = pattern

        # Handle host and port
        if rest:
            if ":" in rest and not rest.startswith("["):
                # Has port
                host, port = rest.rsplit(":", 1)
                result["host"] = host if host else None
                try:
                    result["port"] = int(port)
                except ValueError:
                    result["host"] = rest
            elif rest.startswith("[") and "]:" in rest:
                # IPv6 with port
                host, port = rest.rsplit(":", 1)
                result["host"] = host
                try:
                    result["port"] = int(port)
                except ValueError:
                    pass
            else:
                result["host"] = rest if rest else None

        return result

    def matches(self, url: "URL") -> bool:
        """Check if URL matches this pattern."""
        from requestx import URL

        # Empty pattern matches everything
        if not self.pattern:
            return True

        # Check scheme
        if self._parsed["scheme"] is not None:
            if url.scheme != self._parsed["scheme"]:
                return False

        # Check host
        if self._parsed["host"] is not None:
            url_host = url.host or ""
            pattern_host = self._parsed["host"]

            # Handle wildcard patterns
            if pattern_host.startswith("*"):
                suffix = pattern_host[1:]
                if not url_host.endswith(suffix):
                    return False
            elif url_host != pattern_host:
                return False

        # Check port
        if self._parsed["port"] is not None:
            if url.port != self._parsed["port"]:
                return False

        return True

    def __eq__(self, other: object) -> bool:
        if isinstance(other, URLPattern):
            return self.pattern == other.pattern
        return NotImplemented

    def __lt__(self, other: "URLPattern") -> bool:
        """Sort by specificity (more specific patterns come first)."""
        # More specific = higher priority = should come first
        self_specificity = self._get_specificity()
        other_specificity = other._get_specificity()
        # Higher specificity should come first, so reverse comparison
        return self_specificity > other_specificity

    def _get_specificity(self) -> int:
        """Calculate pattern specificity (higher = more specific)."""
        score = 0
        if self._parsed["port"] is not None:
            score += 4
        if self._parsed["host"] is not None:
            score += 2
        if self._parsed["scheme"] is not None:
            score += 1
        return score

    def __repr__(self) -> str:
        return f"URLPattern({self.pattern!r})"

    def __hash__(self) -> int:
        return hash(self.pattern)


def get_environment_proxies() -> typing.Dict[str, typing.Optional[str]]:
    """
    Get proxy configuration from environment variables.

    Returns a dict mapping URL patterns to proxy URLs.
    """
    proxies: typing.Dict[str, typing.Optional[str]] = {}

    # Standard proxy environment variables
    http_proxy = os.environ.get("HTTP_PROXY") or os.environ.get("http_proxy")
    https_proxy = os.environ.get("HTTPS_PROXY") or os.environ.get("https_proxy")
    all_proxy = os.environ.get("ALL_PROXY") or os.environ.get("all_proxy")
    no_proxy = os.environ.get("NO_PROXY") or os.environ.get("no_proxy")

    if http_proxy:
        proxies["http://"] = http_proxy

    if https_proxy:
        proxies["https://"] = https_proxy

    if all_proxy:
        proxies["all://"] = all_proxy

    # Handle no_proxy
    if no_proxy:
        for host in no_proxy.split(","):
            host = host.strip()
            if not host:
                continue

            # Check if it's a URL with scheme
            if "://" in host:
                proxies[host] = None
            elif "/" in host:
                # CIDR notation
                proxies[f"all://{host}"] = None
            elif host.startswith("."):
                # Wildcard domain
                proxies[f"all://*{host}"] = None
            elif host == "localhost" or _is_ip_address(host):
                proxies[f"all://{host}"] = None
            elif host.startswith("[") and host.endswith("]"):
                # IPv6
                proxies[f"all://{host}"] = None
            elif host == "::1":
                # IPv6 localhost
                proxies[f"all://[{host}]"] = None
            else:
                # Domain name - treat as suffix match
                proxies[f"all://*{host}"] = None

    return proxies


def _is_ip_address(value: str) -> bool:
    """Check if value looks like an IP address."""
    # Simple check for IPv4
    parts = value.split(".")
    if len(parts) == 4:
        try:
            return all(0 <= int(p) <= 255 for p in parts)
        except ValueError:
            pass
    return False
