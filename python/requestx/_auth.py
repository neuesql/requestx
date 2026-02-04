# Auth wrappers with generator protocol

from ._core import (
    Auth as _Auth,
    BasicAuth as _BasicAuth,
    DigestAuth as _DigestAuth,
    NetRCAuth as _NetRCAuth,
    FunctionAuth as _FunctionAuth,
)
from ._compat import _AUTH_DISABLED
from ._exceptions import ProtocolError

# Re-export Auth base class directly (it already supports subclassing)
Auth = _Auth


class BasicAuth:
    """HTTP Basic Authentication with generator protocol."""

    def __init__(self, username="", password=""):
        self._auth = _BasicAuth(username, password)
        self.username = username
        self.password = password

    def sync_auth_flow(self, request):
        """Generator-based sync auth flow for Basic auth."""
        import base64
        # Add Authorization header
        credentials = f"{self.username}:{self.password}"
        encoded = base64.b64encode(credentials.encode()).decode('ascii')
        request.set_header("Authorization", f"Basic {encoded}")
        yield request
        # After response, just stop (basic auth doesn't retry)

    async def async_auth_flow(self, request):
        """Generator-based async auth flow for Basic auth."""
        import base64
        # Add Authorization header
        credentials = f"{self.username}:{self.password}"
        encoded = base64.b64encode(credentials.encode()).decode('ascii')
        request.set_header("Authorization", f"Basic {encoded}")
        yield request
        # After response, just stop (basic auth doesn't retry)

    def __repr__(self):
        return f"BasicAuth(username={self.username!r}, password=***)"


class DigestAuth:
    """HTTP Digest Authentication with generator protocol."""

    def __init__(self, username="", password=""):
        self._auth = _DigestAuth(username, password)
        self.username = username
        self.password = password
        self._nonce_count = 0
        # Cached challenge parameters for subsequent requests
        self._challenge = None  # Dict with realm, nonce, qop, opaque, algorithm

    def _get_client_nonce(self, nonce_count: int, nonce: bytes) -> bytes:
        """Generate a client nonce. Signature matches httpx for test mocking."""
        import hashlib, os, time
        s = str(nonce_count).encode()
        s += nonce
        s += time.ctime().encode()
        s += os.urandom(8)
        return hashlib.sha1(s).hexdigest()[:16].encode()

    def _build_auth_header(self, request, challenge):
        """Build the Authorization header from a challenge."""
        import hashlib

        realm = challenge.get("realm", "")
        nonce = challenge.get("nonce", "")
        qop = challenge.get("qop", "")
        opaque = challenge.get("opaque", "")
        algorithm = challenge.get("algorithm", "MD5").upper()

        # Choose hash function
        if algorithm in ("MD5", "MD5-SESS"):
            hash_func = hashlib.md5
        elif algorithm in ("SHA", "SHA-SESS"):
            hash_func = hashlib.sha1
        elif algorithm in ("SHA-256", "SHA-256-SESS"):
            hash_func = hashlib.sha256
        elif algorithm in ("SHA-512", "SHA-512-SESS"):
            hash_func = hashlib.sha512
        else:
            hash_func = hashlib.md5

        def H(data):
            return hash_func(data.encode()).hexdigest()

        # Increment nonce count
        self._nonce_count += 1
        nc = f"{self._nonce_count:08x}"

        # Get client nonce
        cnonce_bytes = self._get_client_nonce(self._nonce_count, nonce.encode())
        if isinstance(cnonce_bytes, bytes):
            cnonce = cnonce_bytes.decode("ascii")
        else:
            cnonce = str(cnonce_bytes)

        # Calculate A1
        a1 = f"{self.username}:{realm}:{self.password}"
        if algorithm.endswith("-SESS"):
            a1 = f"{H(a1)}:{nonce}:{cnonce}"
        ha1 = H(a1)

        # Calculate A2
        method = str(request.method)
        uri = str(request.url.path)
        if request.url.query:
            uri = f"{uri}?{request.url.query}"
        a2 = f"{method}:{uri}"
        ha2 = H(a2)

        # Calculate response
        if qop:
            # Parse qop options
            qop_options = [q.strip() for q in qop.split(",")]
            if "auth" in qop_options:
                qop_value = "auth"
            elif "auth-int" in qop_options:
                raise NotImplementedError("Digest auth qop=auth-int is not implemented")
            else:
                raise ProtocolError(f"Unsupported Digest auth qop value: {qop}")
            response_value = H(f"{ha1}:{nonce}:{nc}:{cnonce}:{qop_value}:{ha2}")
        else:
            # RFC 2069 style
            response_value = H(f"{ha1}:{nonce}:{ha2}")
            qop_value = None

        # Build Authorization header
        auth_parts = [
            f'username="{self.username}"',
            f'realm="{realm}"',
            f'nonce="{nonce}"',
            f'uri="{uri}"',
            f'response="{response_value}"',
        ]
        if opaque:
            auth_parts.append(f'opaque="{opaque}"')
        # Always include algorithm
        auth_parts.append(f'algorithm={algorithm}')
        if qop_value:
            auth_parts.append(f'qop={qop_value}')
            auth_parts.append(f'nc={nc}')
            auth_parts.append(f'cnonce="{cnonce}"')

        return "Digest " + ", ".join(auth_parts)

    def sync_auth_flow(self, request):
        """Generator-based sync auth flow for Digest auth."""
        import re

        # If we have a cached challenge, use it to pre-authenticate
        if self._challenge is not None:
            auth_header_value = self._build_auth_header(request, self._challenge)
            request.headers["Authorization"] = auth_header_value
            response = yield request
            # If we get 401, challenge may have changed - fall through to parse new one
            if response.status_code != 401:
                return
        else:
            # First request without auth to get challenge
            response = yield request

            if response.status_code != 401:
                return

        # Parse WWW-Authenticate header
        auth_header = response.headers.get("www-authenticate", "")
        if not auth_header.lower().startswith("digest"):
            return

        # Parse digest parameters
        params = {}
        # Handle both quoted and unquoted values
        # Check for unclosed quotes (malformed header)
        header_part = auth_header[7:]  # Skip "Digest "
        if header_part.count('"') % 2 != 0:
            raise ProtocolError("Malformed Digest auth header: unclosed quote")

        for match in re.finditer(r'(\w+)=(?:"([^"]*)"|([^\s,]+))', auth_header):
            key = match.group(1).lower()
            value = match.group(2) if match.group(2) is not None else match.group(3)
            # Strip any remaining quotes from unquoted values
            if value and value.startswith('"'):
                value = value[1:]
            if value and value.endswith('"'):
                value = value[:-1]
            params[key] = value

        nonce = params.get("nonce", "")

        # Validate required fields
        if not nonce:
            raise ProtocolError("Malformed Digest auth header: missing required 'nonce' field")

        # Reset nonce count if we get a new challenge (different nonce)
        if self._challenge is None or self._challenge.get("nonce") != nonce:
            self._nonce_count = 0

        # Store challenge for subsequent requests
        self._challenge = {
            "realm": params.get("realm", ""),
            "nonce": nonce,
            "qop": params.get("qop", ""),
            "opaque": params.get("opaque", ""),
            "algorithm": params.get("algorithm", "MD5"),
        }

        # Copy cookies from response to request
        if hasattr(response, 'cookies') and response.cookies:
            cookie_header = "; ".join(f"{name}={value}" for name, value in response.cookies.items())
            if cookie_header:
                request.headers["Cookie"] = cookie_header

        # Build auth header with new challenge
        auth_header_value = self._build_auth_header(request, self._challenge)
        request.headers["Authorization"] = auth_header_value

        yield request

    async def async_auth_flow(self, request):
        """Generator-based async auth flow for Digest auth."""
        # Properly delegate to sync_auth_flow with response handling
        gen = self.sync_auth_flow(request)
        response = None
        try:
            while True:
                if response is None:
                    req = next(gen)
                else:
                    req = gen.send(response)
                response = yield req
        except StopIteration:
            pass

    def __repr__(self):
        return f"DigestAuth(username={self.username!r}, password=***)"


class NetRCAuth:
    """NetRC-based authentication with generator protocol."""

    def __init__(self, file=None):
        import netrc as netrc_module
        import os
        self._file = file
        # Parse the netrc file at construction time (like httpx does)
        if file is None:
            # Use default netrc file
            netrc_path = os.path.expanduser("~/.netrc")
            if os.path.exists(netrc_path):
                self._netrc = netrc_module.netrc(netrc_path)
            else:
                self._netrc = None
        else:
            self._netrc = netrc_module.netrc(file)

    def sync_auth_flow(self, request):
        """Generator-based sync auth flow for NetRC auth."""
        # Look up credentials for the request host
        if self._netrc is not None:
            url = request.url
            host = url.host if hasattr(url, 'host') else str(url).split('/')[2].split(':')[0].split('@')[-1]
            auth_info = self._netrc.authenticators(host)
            if auth_info is not None:
                username, _, password = auth_info
                import base64
                credentials = f"{username}:{password}"
                encoded = base64.b64encode(credentials.encode()).decode('ascii')
                request.headers["Authorization"] = f"Basic {encoded}"
        yield request

    async def async_auth_flow(self, request):
        """Generator-based async auth flow for NetRC auth."""
        # Look up credentials for the request host
        if self._netrc is not None:
            url = request.url
            host = url.host if hasattr(url, 'host') else str(url).split('/')[2].split(':')[0].split('@')[-1]
            auth_info = self._netrc.authenticators(host)
            if auth_info is not None:
                username, _, password = auth_info
                import base64
                credentials = f"{username}:{password}"
                encoded = base64.b64encode(credentials.encode()).decode('ascii')
                request.headers["Authorization"] = f"Basic {encoded}"
        yield request

    def __repr__(self):
        return f"NetRCAuth(file={self._file!r})"


class FunctionAuth:
    """Function-based authentication with generator protocol."""

    def __init__(self, func):
        self._auth = _FunctionAuth(func)
        self._func = func

    def sync_auth_flow(self, request):
        """Generator-based sync auth flow."""
        # Call the function to modify the request
        self._func(request)
        yield request

    async def async_auth_flow(self, request):
        """Generator-based async auth flow."""
        # Call the function to modify the request
        import inspect
        result = self._func(request)
        # Handle case where function returns a coroutine
        if inspect.iscoroutine(result):
            await result
        yield request

    def __repr__(self):
        return f"FunctionAuth({self._func!r})"


# Helper to convert None to _AUTH_DISABLED sentinel for Rust
def _convert_auth(auth):
    """Convert auth parameter: None -> _AUTH_DISABLED, USE_CLIENT_DEFAULT -> USE_CLIENT_DEFAULT, else pass through."""
    if auth is None:
        return _AUTH_DISABLED
    return auth

# Helper to normalize auth (convert tuple to BasicAuth, callable to FunctionAuth)
def _normalize_auth(auth):
    """Convert tuple auth to BasicAuth, callable to FunctionAuth, pass through others."""
    if isinstance(auth, tuple) and len(auth) == 2:
        return BasicAuth(auth[0], auth[1])
    # Wrap plain callables in FunctionAuth (but not Auth subclasses which have auth_flow)
    if callable(auth) and not hasattr(auth, 'sync_auth_flow') and not hasattr(auth, 'async_auth_flow') and not hasattr(auth, 'auth_flow'):
        return FunctionAuth(auth)
    return auth


def _extract_auth_from_url(url_str):
    """Extract BasicAuth from URL userinfo if present."""
    if '@' not in url_str:
        return None
    # Parse URL to extract userinfo
    from urllib.parse import urlparse, unquote
    parsed = urlparse(url_str)
    if parsed.username:
        username = unquote(parsed.username)
        password = unquote(parsed.password) if parsed.password else ""
        return BasicAuth(username, password)
    return None
