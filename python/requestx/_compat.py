# Compatibility utilities, sentinels, and helpers

import logging as _logging

from ._core import (
    URL,
    codes as _codes,
)

# Set up the httpx logger (for compatibility)
_logger = _logging.getLogger("httpx")


# Sentinel for "auth not specified" - distinct from auth=None which disables auth
class _AuthUnset:
    """Sentinel to indicate auth was not specified."""

    _instance = None

    def __new__(cls):
        if cls._instance is None:
            cls._instance = super().__new__(cls)
        return cls._instance

    def __repr__(self):
        return "<USE_CLIENT_AUTH>"

    def __bool__(self):
        return False


USE_CLIENT_DEFAULT = _AuthUnset()


# Sentinel for "auth explicitly disabled" - used to pass auth=None to Rust
class _AuthDisabled:
    """Sentinel to indicate auth is explicitly disabled."""

    _instance = None

    def __new__(cls):
        if cls._instance is None:
            cls._instance = super().__new__(cls)
        return cls._instance

    def __repr__(self):
        return "<AUTH_DISABLED>"

    def __bool__(self):
        return False


_AUTH_DISABLED = _AuthDisabled()


class _ExplicitPortURL:
    """URL wrapper that preserves explicit port in string representation.

    The standard URL class normalizes away default ports (e.g., :443 for https).
    This wrapper preserves the explicit port string for cases like malformed
    redirect URLs that specify the default port explicitly.
    """

    def __init__(self, url_str):
        self._url_str = url_str
        self._url = URL(url_str)  # Underlying URL for property access

    def __str__(self):
        return self._url_str

    def __repr__(self):
        return f"URL('{self._url_str}')"

    def __eq__(self, other):
        if isinstance(other, str):
            return self._url_str == other
        if isinstance(other, (_ExplicitPortURL, URL)):
            return str(self) == str(other)
        return False

    def __hash__(self):
        return hash(self._url_str)

    @property
    def scheme(self):
        return self._url.scheme

    @property
    def host(self):
        return self._url.host

    @property
    def port(self):
        return self._url.port

    @property
    def path(self):
        return self._url.path

    @property
    def query(self):
        return self._url.query

    @property
    def fragment(self):
        return self._url.fragment

    def join(self, url):
        return self._url.join(url)


# Wrap codes to support codes(404) returning int
class codes(_codes):
    """HTTP status codes with flexible access patterns."""

    def __new__(cls, code):
        """Allow codes(404) to return 404."""
        return code


def create_ssl_context(
    cert=None,
    verify=True,
    trust_env=True,
    http2=False,
):
    """
    Create an SSL context for use with httpx.

    Args:
        cert: Optional SSL certificate to use for client authentication.
              Can be:
              - A path to a certificate file (str or Path)
              - A tuple of (cert_file, key_file)
              - A tuple of (cert_file, key_file, password)
        verify: SSL verification mode. Can be:
                - True: Verify server certificates (default)
                - False: Disable verification (not recommended)
                - str or Path: Path to a CA bundle file
        trust_env: Whether to trust environment variables for SSL configuration.
        http2: Whether to use HTTP/2.

    Returns:
        An ssl.SSLContext instance configured with the specified options.
    """
    import ssl
    import os
    from pathlib import Path

    # Create default SSL context
    context = ssl.create_default_context()

    # Handle verify argument
    if verify is False:
        context.check_hostname = False
        context.verify_mode = ssl.CERT_NONE
    elif verify is not True:
        # verify is a path to CA bundle
        verify_path = Path(verify) if not isinstance(verify, Path) else verify
        if verify_path.is_dir():
            context.load_verify_locations(capath=str(verify_path))
        elif verify_path.is_file():
            context.load_verify_locations(cafile=str(verify_path))
        else:
            raise IOError(
                f"Could not find a suitable TLS CA certificate bundle, invalid path: {verify}"
            )

    # Handle client certificate
    if cert is not None:
        if isinstance(cert, str) or isinstance(cert, Path):
            context.load_cert_chain(certfile=str(cert))
        elif isinstance(cert, tuple):
            if len(cert) == 2:
                certfile, keyfile = cert
                context.load_cert_chain(certfile=str(certfile), keyfile=str(keyfile))
            elif len(cert) == 3:
                certfile, keyfile, password = cert
                context.load_cert_chain(
                    certfile=str(certfile), keyfile=str(keyfile), password=password
                )

    # Handle trust_env for SSL_CERT_FILE and SSL_CERT_DIR
    if trust_env:
        ssl_cert_file = os.environ.get("SSL_CERT_FILE")
        ssl_cert_dir = os.environ.get("SSL_CERT_DIR")
        if ssl_cert_file:
            context.load_verify_locations(cafile=ssl_cert_file)
        if ssl_cert_dir:
            context.load_verify_locations(capath=ssl_cert_dir)

    # Configure SSLKEYLOGFILE for debugging
    if trust_env:
        sslkeylogfile = os.environ.get("SSLKEYLOGFILE")
        if sslkeylogfile:
            context.keylog_filename = sslkeylogfile

    return context
