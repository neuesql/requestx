# Shared utilities for Client and AsyncClient

from ._core import URL, Headers


class _HeadersProxy(Headers):
    """Proxy object that wraps Headers and syncs changes back to the client.

    Inherits from Headers to pass isinstance checks while proxying to client headers.
    """

    def __new__(cls, client):
        instance = Headers.__new__(cls)
        return instance

    def __init__(self, client):
        self._client = client
        self._headers = client._client.headers

    def __getitem__(self, key):
        return self._headers[key]

    def __setitem__(self, key, value):
        self._headers[key] = value
        self._client._client.headers = self._headers

    def __delitem__(self, key):
        del self._headers[key]
        self._client._client.headers = self._headers

    def __contains__(self, key):
        return key in self._headers

    def __iter__(self):
        return iter(self._headers)

    def __len__(self):
        return len(self._headers)

    def __eq__(self, other):
        return self._headers == other

    def __repr__(self):
        return repr(self._headers)

    def get(self, key, default=None):
        return self._headers.get(key, default)

    def get_list(self, key, split_commas=False):
        return self._headers.get_list(key, split_commas)

    def keys(self):
        return self._headers.keys()

    def values(self):
        return self._headers.values()

    def items(self):
        return self._headers.items()

    def multi_items(self):
        return self._headers.multi_items()

    def update(self, other):
        self._headers.update(other)
        self._client._client.headers = self._headers

    def setdefault(self, key, default=None):
        result = self._headers.setdefault(key, default)
        self._client._client.headers = self._headers
        return result

    def copy(self):
        return self._headers.copy()

    @property
    def raw(self):
        return self._headers.raw

    @property
    def encoding(self):
        return self._headers.encoding

    @encoding.setter
    def encoding(self, value):
        self._headers.encoding = value
        self._client._client.headers = self._headers


def extract_cookies_from_response(client, response, request):
    """Extract Set-Cookie headers from response and add to client cookies."""
    set_cookie_headers = []
    if hasattr(response, "headers"):
        if hasattr(response.headers, "multi_items"):
            for key, value in response.headers.multi_items():
                if key.lower() == "set-cookie":
                    set_cookie_headers.append(value)
        elif hasattr(response.headers, "get_list"):
            set_cookie_headers = response.headers.get_list("set-cookie")
        else:
            cookie_header = response.headers.get("set-cookie")
            if cookie_header:
                set_cookie_headers = [cookie_header]

    if set_cookie_headers:
        from email.utils import parsedate_to_datetime
        import datetime

        cookies = client.cookies
        for cookie_str in set_cookie_headers:
            parts = cookie_str.split(";")
            if parts:
                name_value = parts[0].strip()
                if "=" in name_value:
                    name, value = name_value.split("=", 1)
                    name = name.strip()
                    value = value.strip()

                    is_expired = False
                    for part in parts[1:]:
                        part = part.strip()
                        if part.lower().startswith("expires="):
                            expires_str = part[8:].strip()
                            try:
                                expires_dt = parsedate_to_datetime(expires_str)
                                if expires_dt < datetime.datetime.now(
                                    datetime.timezone.utc
                                ):
                                    is_expired = True
                            except Exception:
                                pass
                            break

                    if is_expired:
                        cookies.delete(name)
                    else:
                        cookies.set(name, value)
        client.cookies = cookies


def merge_url(client, url):
    """Merge a URL with the client's base_url.

    Unlike RFC 3986 URL resolution, this concatenates paths when the
    relative URL starts with '/'.
    """
    if isinstance(url, URL):
        url_str = str(url)
    else:
        url_str = str(url)

    if "://" in url_str:
        return url_str

    base_url = client.base_url
    if base_url is None:
        return url_str

    base_url_str = str(base_url)

    if base_url_str.endswith("/"):
        base_url_str = base_url_str[:-1]

    if url_str.startswith("/"):
        return base_url_str + url_str
    elif url_str.startswith("../"):
        base = URL(base_url_str)
        base_path = base.path or ""
        if base_path.endswith("/"):
            base_path = base_path[:-1]
        path_parts = base_path.split("/")
        rel_parts = url_str.split("/")
        while rel_parts and rel_parts[0] == "..":
            rel_parts.pop(0)
            if path_parts:
                path_parts.pop()
        new_path = "/".join(path_parts + rel_parts)
        result = f"{base.scheme}://{base.host}"
        if base.port:
            result += f":{base.port}"
        if new_path:
            if not new_path.startswith("/"):
                new_path = "/" + new_path
            result += new_path
        return result
    else:
        return base_url_str + "/" + url_str


def get_proxy_from_env():
    """Get proxy URL from environment variables."""
    import os

    for var in (
        "ALL_PROXY",
        "all_proxy",
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
    ):
        proxy = os.environ.get(var)
        if proxy:
            if "://" not in proxy:
                proxy = "http://" + proxy
            return proxy
    return None


def should_use_proxy(url):
    """Check if URL should use proxy based on NO_PROXY env var."""
    import os

    no_proxy = os.environ.get("NO_PROXY", os.environ.get("no_proxy", ""))

    if not no_proxy:
        return True

    if no_proxy == "*":
        return False

    if isinstance(url, str):
        url = URL(url)
    host = url.host

    for pattern in no_proxy.split(","):
        pattern = pattern.strip()
        if not pattern:
            continue

        if "://" in pattern:
            pattern_scheme, pattern_host = pattern.split("://", 1)
            if pattern_scheme != url.scheme:
                continue
            pattern = pattern_host

        if host == pattern:
            return False

        if pattern.startswith("."):
            if host.endswith(pattern):
                return False
        elif host.endswith("." + pattern):
            return False

    return True


def get_proxy_for_url(url):
    """Get proxy URL from environment for a specific URL."""
    import os

    scheme = url.scheme if hasattr(url, "scheme") else "http"

    if scheme == "https":
        proxy = os.environ.get("HTTPS_PROXY", os.environ.get("https_proxy"))
        if proxy:
            if "://" not in proxy:
                proxy = "http://" + proxy
            return proxy

    if scheme == "http":
        proxy = os.environ.get("HTTP_PROXY", os.environ.get("http_proxy"))
        if proxy:
            if "://" not in proxy:
                proxy = "http://" + proxy
            return proxy

    proxy = os.environ.get("ALL_PROXY", os.environ.get("all_proxy"))
    if proxy:
        if "://" not in proxy:
            proxy = "http://" + proxy
        return proxy

    return None


def match_pattern(url_scheme, url_host, url_port, pattern):
    """Match URL against a mount pattern. Returns score (higher is better match), or -1 if no match."""
    if "://" in pattern:
        pattern_scheme, pattern_rest = pattern.split("://", 1)
    else:
        return -1

    if pattern_scheme not in ("all", url_scheme):
        return -1

    score = 0 if pattern_scheme == "all" else 1

    if not pattern_rest:
        return score

    if ":" in pattern_rest and not pattern_rest.startswith("["):
        pattern_host, pattern_port_str = pattern_rest.rsplit(":", 1)
        try:
            pattern_port = int(pattern_port_str)
        except ValueError:
            pattern_host = pattern_rest
            pattern_port = None
    else:
        pattern_host = pattern_rest
        pattern_port = None

    if pattern_host == "*":
        score += 2
    elif pattern_host.startswith("*."):
        suffix = pattern_host[1:]
        if url_host.endswith(suffix) and url_host != suffix[1:]:
            score += 2
        else:
            return -1
    elif pattern_host.startswith("*"):
        suffix = pattern_host[1:]
        if url_host == suffix or url_host.endswith("." + suffix):
            score += 2
        else:
            return -1
    else:
        if url_host.lower() != pattern_host.lower():
            return -1
        score += 2

    if pattern_port is not None:
        if url_port == pattern_port:
            score += 4

    return score


def transport_for_url(client, url, transport_class):
    """Get the transport to use for a given URL.

    Returns the most specific matching mount, or the default transport if no match.
    transport_class should be HTTPTransport or AsyncHTTPTransport.
    """
    if isinstance(url, str):
        url = URL(url)

    url_scheme = url.scheme
    url_host = url.host or ""
    url_port = url.port

    best_match = None
    best_score = -1

    for pattern, transport in client._mounts.items():
        score = match_pattern(url_scheme, url_host, url_port, pattern)
        if score > best_score:
            best_score = score
            best_match = transport

    if best_match is not None:
        return best_match

    if getattr(client._client, "trust_env", True):
        proxy_url = get_proxy_for_url(url)
        if proxy_url:
            if not should_use_proxy(url):
                return client._default_transport
            return transport_class(proxy=proxy_url)

    return client._default_transport
