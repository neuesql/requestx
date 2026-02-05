# Request wrapper with proper stream property

from ._core import (
    Headers,
    Request as _Request,
)
from ._exceptions import RequestNotRead
from ._streams import (
    AsyncByteStream,
    SyncByteStream,
    ByteStream,
    _SyncIteratorStream,
    _AsyncIteratorStream,
    _DualIteratorStream,
    StreamConsumed,
)


class _WrappedRequest:
    """Wrapper for Rust Request that provides mutable headers."""

    def __init__(
        self, rust_request, async_stream=None, sync_stream=None, explicit_url=None
    ):
        self._rust_request = rust_request
        self._headers_modified = False
        self._async_stream = async_stream  # Original async iterator if any
        self._sync_stream = sync_stream  # Sync iterator/generator if any
        self._stream_consumed = False
        self._explicit_url = explicit_url  # URL string that should not be normalized

    def __getattr__(self, name):
        return getattr(self._rust_request, name)

    @property
    def headers(self):
        return _WrappedRequestHeadersProxy(self)

    @headers.setter
    def headers(self, value):
        self._rust_request.headers = value

    def set_header(self, name, value):
        self._rust_request.set_header(name, value)

    def get_header(self, name, default=None):
        return self._rust_request.get_header(name, default)

    @property
    def stream(self):
        """Get the request body stream."""
        if self._async_stream is not None:
            # Return an AsyncByteStream wrapper that tracks consumption
            return _WrappedAsyncByteStream(self._async_stream, self)
        if self._sync_stream is not None:
            # Return the sync stream wrapper (already a SyncByteStream)
            return self._sync_stream
        return self._rust_request.stream


class _WrappedAsyncByteStream(AsyncByteStream):
    """Async byte stream wrapper that tracks consumption for retry detection."""

    def __init__(self, iterator, owner):
        self._iterator = iterator
        self._owner = owner
        self._consumed = False
        self._started = False

    def __aiter__(self):
        # Check if stream was already consumed (by a previous request)
        if self._owner._stream_consumed:
            raise StreamConsumed()
        return self

    async def __anext__(self):
        self._started = True
        try:
            chunk = await self._iterator.__anext__()
            return chunk
        except StopAsyncIteration:
            self._consumed = True
            self._owner._stream_consumed = True
            raise

    async def aread(self):
        """Read all bytes."""
        if self._owner._stream_consumed:
            raise StreamConsumed()
        chunks = []
        async for chunk in self:
            chunks.append(chunk)
        return b"".join(chunks)


class _WrappedRequestHeadersProxy:
    """Proxy for wrapped request headers that syncs changes back."""

    def __init__(self, wrapped_request):
        self._wrapped_request = wrapped_request
        # Get headers from rust request and convert to a new Headers object
        rust_headers = wrapped_request._rust_request.headers
        # Use _internal_items to preserve original header casing for .raw access
        self._headers = Headers(list(rust_headers._internal_items()))

    def _sync_back(self):
        self._wrapped_request._rust_request.headers = self._headers

    def __getitem__(self, key):
        return self._headers[key]

    def __setitem__(self, key, value):
        self._headers[key] = value
        self._sync_back()

    def __delitem__(self, key):
        del self._headers[key]
        self._sync_back()

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
        self._sync_back()

    def setdefault(self, key, default=None):
        result = self._headers.setdefault(key, default)
        self._sync_back()
        return result

    def copy(self):
        return self._headers.copy()

    @property
    def raw(self):
        return self._headers.raw

    @property
    def encoding(self):
        return self._headers.encoding


class _RequestHeadersProxy:
    """Proxy object that wraps Headers and syncs changes back to the request."""

    def __init__(self, request):
        self._request = request
        self._headers = request._get_headers()  # Get current headers

    def __getitem__(self, key):
        return self._headers[key]

    def __setitem__(self, key, value):
        self._headers[key] = value
        self._request._set_headers(self._headers)

    def __delitem__(self, key):
        del self._headers[key]
        self._request._set_headers(self._headers)

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
        self._request._set_headers(self._headers)

    def setdefault(self, key, default=None):
        result = self._headers.setdefault(key, default)
        self._request._set_headers(self._headers)
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
        self._request._set_headers(self._headers)


class Request(_Request):
    """HTTP Request with proper stream support."""

    # Instance attribute to store async content - set lazily
    _py_async_content = None
    _py_was_async_read = False
    _py_stream_consumed = False

    @property
    def stream(self):
        """Get the request body as a ByteStream based on content type."""
        # Get stream mode from Rust
        mode = super().stream_mode

        # For streaming content (iterators/generators), return appropriate stream wrapper
        stream_ref = super().stream_ref
        if stream_ref is not None:
            if mode == "async":
                return _AsyncIteratorStream(stream_ref, self)
            elif mode == "sync":
                return _SyncIteratorStream(stream_ref, self)
            else:
                return _DualIteratorStream(stream_ref, self)

        # If async-read was done, return an async-compatible stream
        if getattr(self, "_py_was_async_read", False):
            content = getattr(self, "_py_async_content", None)
            if content is not None:
                return AsyncByteStream(content)
            try:
                return AsyncByteStream(super().content)
            except RequestNotRead:
                return AsyncByteStream(b"")

        # Return stream based on mode
        try:
            content = super().content
        except RequestNotRead:
            content = b""

        if mode == "async":
            return AsyncByteStream(content)
        elif mode == "sync":
            return SyncByteStream(content)
        else:
            return ByteStream(content)

    @property
    def content(self):
        """Get the request body content."""
        # If async content is available (from aread), return it
        content = getattr(self, "_py_async_content", None)
        if content is not None:
            return content
        return super().content

    async def aread(self):
        """Async read method that stores content after reading."""
        object.__setattr__(self, "_py_was_async_read", True)
        # Call parent aread which returns a coroutine
        result = await super().aread()
        # Store the result in Rust side for proper pickling
        if result:
            self._set_content_from_aread(result)
            object.__setattr__(self, "_py_async_content", result)
        return result

    @property
    def headers(self):
        """Get headers proxy that syncs changes back to the request."""
        return _RequestHeadersProxy(self)

    @headers.setter
    def headers(self, value):
        self._set_headers(value)

    def _get_headers(self):
        """Get the underlying headers object from Rust."""
        # Use super() to access the Rust property
        return super(Request, self).headers

    def _set_headers(self, value):
        """Set the underlying headers object on Rust."""
        # Use setattr on the parent class type descriptor
        super(Request, type(self)).headers.__set__(self, value)
