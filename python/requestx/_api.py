# Top-level API functions with exception conversion

from ._core import (
    get as _get,
    post as _post,
    put as _put,
    patch as _patch,
    delete as _delete,
    head as _head,
    options as _options,
    request as _request,
    stream as _stream,
)
from ._exceptions import _convert_exception, _RUST_EXCEPTIONS


def _prepare_content(kwargs):
    """Prepare content argument, consuming iterators/generators to bytes."""
    import inspect
    import types
    content = kwargs.get('content')
    if content is not None:
        # Check if it's a generator or iterator (but not bytes, str, or file-like)
        if isinstance(content, types.GeneratorType):
            # Consume generator to bytes
            kwargs['content'] = b''.join(content)
        elif hasattr(content, '__iter__') and hasattr(content, '__next__'):
            # It's an iterator - consume it
            kwargs['content'] = b''.join(content)
        elif hasattr(content, '__iter__') and not isinstance(content, (bytes, str, list, tuple, dict)):
            # It's an iterable object (like SyncByteStream) - consume it
            try:
                kwargs['content'] = b''.join(content)
            except TypeError:
                pass  # Let Rust handle it if join fails
    return kwargs


def get(url, **kwargs):
    """Send a GET request."""
    try:
        return _get(url, **kwargs)
    except _RUST_EXCEPTIONS as e:
        raise _convert_exception(e) from None


def post(url, **kwargs):
    """Send a POST request."""
    try:
        kwargs = _prepare_content(kwargs)
        return _post(url, **kwargs)
    except _RUST_EXCEPTIONS as e:
        raise _convert_exception(e) from None


def put(url, **kwargs):
    """Send a PUT request."""
    try:
        kwargs = _prepare_content(kwargs)
        return _put(url, **kwargs)
    except _RUST_EXCEPTIONS as e:
        raise _convert_exception(e) from None


def patch(url, **kwargs):
    """Send a PATCH request."""
    try:
        kwargs = _prepare_content(kwargs)
        return _patch(url, **kwargs)
    except _RUST_EXCEPTIONS as e:
        raise _convert_exception(e) from None


def delete(url, **kwargs):
    """Send a DELETE request."""
    try:
        return _delete(url, **kwargs)
    except _RUST_EXCEPTIONS as e:
        raise _convert_exception(e) from None


def head(url, **kwargs):
    """Send a HEAD request."""
    try:
        return _head(url, **kwargs)
    except _RUST_EXCEPTIONS as e:
        raise _convert_exception(e) from None


def options(url, **kwargs):
    """Send an OPTIONS request."""
    try:
        return _options(url, **kwargs)
    except _RUST_EXCEPTIONS as e:
        raise _convert_exception(e) from None


def request(method, url, **kwargs):
    """Send an HTTP request."""
    try:
        return _request(method, url, **kwargs)
    except _RUST_EXCEPTIONS as e:
        raise _convert_exception(e) from None


def stream(method, url, **kwargs):
    """Stream an HTTP request."""
    try:
        return _stream(method, url, **kwargs)
    except _RUST_EXCEPTIONS as e:
        raise _convert_exception(e) from None
