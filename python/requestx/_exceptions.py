# Exception classes with request attribute support

from ._core import (
    RequestError as _RequestError,
    TransportError as _TransportError,
    TimeoutException as _TimeoutException,
    ConnectTimeout as _ConnectTimeout,
    ReadTimeout as _ReadTimeout,
    WriteTimeout as _WriteTimeout,
    PoolTimeout as _PoolTimeout,
    NetworkError as _NetworkError,
    ConnectError as _ConnectError,
    ReadError as _ReadError,
    WriteError as _WriteError,
    CloseError as _CloseError,
    ProxyError as _ProxyError,
    ProtocolError as _ProtocolError,
    LocalProtocolError as _LocalProtocolError,
    RemoteProtocolError as _RemoteProtocolError,
    UnsupportedProtocol as _UnsupportedProtocol,
    DecodingError as _DecodingError,
    TooManyRedirects as _TooManyRedirects,
    StreamError as _StreamError,
    StreamConsumed as _StreamConsumed,
    StreamClosed as _StreamClosed,
    ResponseNotRead as _ResponseNotRead,
    RequestNotRead as _RequestNotRead,
)


class RequestError(Exception):
    """Base class for request errors."""
    def __init__(self, message="", *, request=None):
        super().__init__(message)
        self._request = request

    @property
    def request(self):
        if self._request is None:
            raise RuntimeError(
                "The request instance has not been set on this exception."
            )
        return self._request


class TransportError(RequestError):
    """Base class for transport errors."""
    pass


class TimeoutException(TransportError):
    """Base class for timeout exceptions."""
    pass


class ConnectTimeout(TimeoutException):
    """Timeout during connection."""
    pass


class ReadTimeout(TimeoutException):
    """Timeout while reading response."""
    pass


class WriteTimeout(TimeoutException):
    """Timeout while writing request."""
    pass


class PoolTimeout(TimeoutException):
    """Timeout waiting for connection pool."""
    pass


class NetworkError(TransportError):
    """Network-related errors."""
    pass


class ConnectError(NetworkError):
    """Error connecting to host."""
    pass


class ReadError(NetworkError):
    """Error reading from connection."""
    pass


class WriteError(NetworkError):
    """Error writing to connection."""
    pass


class CloseError(NetworkError):
    """Error closing connection."""
    pass


class ProxyError(TransportError):
    """Proxy-related errors."""
    pass


class ProtocolError(TransportError):
    """Protocol-related errors."""
    pass


class LocalProtocolError(ProtocolError):
    """Local protocol error."""
    pass


class RemoteProtocolError(ProtocolError):
    """Remote protocol error."""
    pass


class UnsupportedProtocol(TransportError):
    """Unsupported protocol error."""
    pass


class DecodingError(RequestError):
    """Decoding error."""
    pass


class TooManyRedirects(RequestError):
    """Too many redirects error."""
    pass


class StreamError(RequestError):
    """Stream error."""
    pass


class StreamConsumed(StreamError):
    """Stream consumed error."""
    pass


class StreamClosed(StreamError):
    """Stream closed error."""
    pass


class ResponseNotRead(StreamError):
    """Response not read error."""
    pass


class RequestNotRead(StreamError):
    """Request not read error."""
    pass


def _convert_exception(exc):
    """Convert a Rust exception to the appropriate Python exception."""
    msg = str(exc)
    if isinstance(exc, _ConnectTimeout):
        return ConnectTimeout(msg)
    elif isinstance(exc, _ReadTimeout):
        return ReadTimeout(msg)
    elif isinstance(exc, _WriteTimeout):
        return WriteTimeout(msg)
    elif isinstance(exc, _PoolTimeout):
        return PoolTimeout(msg)
    elif isinstance(exc, _TimeoutException):
        return TimeoutException(msg)
    elif isinstance(exc, _ConnectError):
        return ConnectError(msg)
    elif isinstance(exc, _ReadError):
        return ReadError(msg)
    elif isinstance(exc, _WriteError):
        return WriteError(msg)
    elif isinstance(exc, _CloseError):
        return CloseError(msg)
    elif isinstance(exc, _NetworkError):
        return NetworkError(msg)
    elif isinstance(exc, _ProxyError):
        return ProxyError(msg)
    elif isinstance(exc, _LocalProtocolError):
        return LocalProtocolError(msg)
    elif isinstance(exc, _RemoteProtocolError):
        return RemoteProtocolError(msg)
    elif isinstance(exc, _ProtocolError):
        return ProtocolError(msg)
    elif isinstance(exc, _UnsupportedProtocol):
        return UnsupportedProtocol(msg)
    elif isinstance(exc, _DecodingError):
        return DecodingError(msg)
    elif isinstance(exc, _TooManyRedirects):
        return TooManyRedirects(msg)
    elif isinstance(exc, _StreamConsumed):
        return StreamConsumed(msg)
    elif isinstance(exc, _StreamClosed):
        return StreamClosed(msg)
    elif isinstance(exc, _ResponseNotRead):
        return ResponseNotRead(msg)
    elif isinstance(exc, _RequestNotRead):
        return RequestNotRead(msg)
    elif isinstance(exc, _StreamError):
        return StreamError(msg)
    elif isinstance(exc, _TransportError):
        return TransportError(msg)
    elif isinstance(exc, _RequestError):
        return RequestError(msg)
    else:
        return exc


# Tuple of all Rust exception types for use in except clauses
_RUST_EXCEPTIONS = (
    _RequestError, _TransportError, _TimeoutException, _NetworkError,
    _ConnectError, _ReadError, _WriteError, _CloseError, _ProxyError,
    _ProtocolError, _UnsupportedProtocol, _DecodingError, _TooManyRedirects,
    _StreamError, _ConnectTimeout, _ReadTimeout, _WriteTimeout, _PoolTimeout,
)
