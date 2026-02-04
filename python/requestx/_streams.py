# Stream classes - Python wrappers with proper isinstance support

from ._exceptions import StreamConsumed


class SyncByteStream:
    """Base class for synchronous byte streams.

    Implements the sync iteration protocol (__iter__, __next__).
    """

    def __init__(self, data=b""):
        if isinstance(data, (bytes, bytearray)):
            self._data = bytes(data)
        else:
            self._data = data
        self._consumed = False

    def __iter__(self):
        self._consumed = False
        return self

    def __next__(self):
        if self._consumed:
            raise StopIteration
        if isinstance(self._data, bytes):
            self._consumed = True
            if self._data:
                return self._data
            raise StopIteration
        # For other iterables, raise as consumed
        self._consumed = True
        raise StopIteration

    def read(self):
        """Read all bytes."""
        if isinstance(self._data, bytes):
            return self._data
        return b""

    def close(self):
        """Close the stream."""
        pass

    def __repr__(self):
        if isinstance(self._data, bytes):
            return f"<SyncByteStream [{len(self._data)} bytes]>"
        return "<SyncByteStream>"


class _GeneratorByteStream(SyncByteStream):
    """SyncByteStream wrapper for generators/iterators that tracks consumption.

    This allows generators to be passed as content while tracking whether
    the stream has been consumed (for detecting StreamConsumed on redirects).
    """

    def __init__(self, generator, owner=None):
        # Don't call super().__init__ since we don't have bytes data
        self._generator = generator
        self._owner = owner  # Reference to _WrappedRequest for tracking
        self._consumed = False
        self._started = False
        self._chunks = []  # Store chunks for potential re-read

    def __iter__(self):
        if self._consumed:
            raise StreamConsumed()
        return self

    def __next__(self):
        if self._consumed:
            raise StopIteration
        self._started = True
        try:
            chunk = next(self._generator)
            self._chunks.append(chunk)
            return chunk
        except StopIteration:
            self._consumed = True
            if self._owner is not None:
                self._owner._stream_consumed = True
            raise

    def read(self):
        """Read all bytes."""
        if self._consumed:
            raise StreamConsumed()
        # Consume remaining generator
        for chunk in self._generator:
            self._chunks.append(chunk)
        self._consumed = True
        if self._owner is not None:
            self._owner._stream_consumed = True
        return b''.join(self._chunks)

    def close(self):
        """Close the stream."""
        pass

    def __repr__(self):
        return "<GeneratorByteStream>"


class AsyncByteStream:
    """Base class for asynchronous byte streams.

    Implements the async iteration protocol (__aiter__, __anext__).
    """

    def __init__(self, data=b""):
        if isinstance(data, (bytes, bytearray)):
            self._data = bytes(data)
        else:
            self._data = data
        self._consumed = False

    def __aiter__(self):
        self._consumed = False
        return self

    async def __anext__(self):
        if self._consumed:
            raise StopAsyncIteration
        if isinstance(self._data, bytes):
            self._consumed = True
            if self._data:
                return self._data
            raise StopAsyncIteration
        self._consumed = True
        raise StopAsyncIteration

    async def aread(self):
        """Read all bytes asynchronously."""
        if isinstance(self._data, bytes):
            return self._data
        return b""

    async def aclose(self):
        """Close the stream asynchronously."""
        pass

    def __repr__(self):
        if isinstance(self._data, bytes):
            return f"<AsyncByteStream [{len(self._data)} bytes]>"
        return "<AsyncByteStream>"


class ByteStream(SyncByteStream, AsyncByteStream):
    """Dual-mode byte stream that supports both sync and async iteration.

    This class inherits from both SyncByteStream and AsyncByteStream,
    so isinstance checks for either will return True.
    """

    def __init__(self, data=b""):
        if isinstance(data, (bytes, bytearray)):
            self._data = bytes(data)
        else:
            self._data = data
        self._sync_consumed = False
        self._async_consumed = False

    # Sync iteration
    def __iter__(self):
        self._sync_consumed = False
        return self

    def __next__(self):
        if self._sync_consumed:
            raise StopIteration
        if isinstance(self._data, bytes):
            self._sync_consumed = True
            if self._data:
                return self._data
            raise StopIteration
        self._sync_consumed = True
        raise StopIteration

    # Async iteration
    def __aiter__(self):
        self._async_consumed = False
        return self

    async def __anext__(self):
        if self._async_consumed:
            raise StopAsyncIteration
        if isinstance(self._data, bytes):
            self._async_consumed = True
            if self._data:
                return self._data
            raise StopAsyncIteration
        self._async_consumed = True
        raise StopAsyncIteration

    # Common methods
    def read(self):
        """Read all bytes synchronously."""
        if isinstance(self._data, bytes):
            return self._data
        return b""

    async def aread(self):
        """Read all bytes asynchronously."""
        if isinstance(self._data, bytes):
            return self._data
        return b""

    def close(self):
        """Close the stream."""
        pass

    async def aclose(self):
        """Close the stream asynchronously."""
        pass

    def __repr__(self):
        if isinstance(self._data, bytes):
            return f"<ByteStream [{len(self._data)} bytes]>"
        return "<ByteStream>"


class _SyncIteratorStream:
    """Sync-only stream wrapper for iterators."""

    def __init__(self, iterator, owner=None):
        self._iterator = iterator
        self._owner = owner
        self._consumed = False
        self._started = False

    def __iter__(self):
        # Check if owner's stream was already consumed
        if self._owner is not None and getattr(self._owner, '_py_stream_consumed', False):
            raise StreamConsumed()
        if self._consumed:
            raise StreamConsumed()
        self._started = True
        return self

    def __next__(self):
        if self._consumed:
            raise StopIteration
        try:
            return next(self._iterator)
        except StopIteration:
            self._consumed = True
            if self._owner is not None:
                object.__setattr__(self._owner, '_py_stream_consumed', True)
            raise

    def read(self):
        """Read all bytes."""
        if self._owner is not None and getattr(self._owner, '_py_stream_consumed', False):
            raise StreamConsumed()
        if self._consumed:
            raise StreamConsumed()
        result = b"".join(self)
        return result

    def close(self):
        pass

    def __repr__(self):
        return "<SyncIteratorStream>"


class _AsyncIteratorStream:
    """Async-only stream wrapper for async iterators and async file-like objects."""

    def __init__(self, iterator, owner=None):
        self._iterator = iterator
        self._owner = owner
        self._consumed = False
        # Check if this is an async file-like object (has aread but no __anext__)
        self._is_file_like = hasattr(iterator, 'aread') and not hasattr(iterator, '__anext__')
        # For file-like objects, we need to track if we got the aiter
        self._aiter = None

    def __aiter__(self):
        # Check if owner's stream was already consumed
        if self._owner is not None and getattr(self._owner, '_py_stream_consumed', False):
            raise StreamConsumed()
        if self._consumed:
            raise StreamConsumed()
        return self

    async def __anext__(self):
        if self._consumed:
            raise StopAsyncIteration
        try:
            if self._is_file_like:
                # For async file-like objects, use __aiter__ if available
                if self._aiter is None:
                    if hasattr(self._iterator, '__aiter__'):
                        self._aiter = self._iterator.__aiter__()
                    else:
                        # Fall back to reading all at once
                        content = await self._iterator.aread(65536)
                        if not content:
                            self._consumed = True
                            if self._owner is not None:
                                object.__setattr__(self._owner, '_py_stream_consumed', True)
                            raise StopAsyncIteration
                        return content
                return await self._aiter.__anext__()
            else:
                return await self._iterator.__anext__()
        except StopAsyncIteration:
            self._consumed = True
            if self._owner is not None:
                object.__setattr__(self._owner, '_py_stream_consumed', True)
            raise

    async def aread(self):
        """Read all bytes asynchronously."""
        if self._owner is not None and getattr(self._owner, '_py_stream_consumed', False):
            raise StreamConsumed()
        if self._consumed:
            raise StreamConsumed()
        result = b"".join([part async for part in self])
        return result

    async def aclose(self):
        pass

    def __repr__(self):
        return "<AsyncIteratorStream>"


class _DualIteratorStream:
    """Dual-mode stream wrapper for bytes content."""

    def __init__(self, data, owner=None):
        self._data = data
        self._owner = owner
        self._sync_consumed = False
        self._async_consumed = False

    def __iter__(self):
        self._sync_consumed = False
        return self

    def __next__(self):
        if self._sync_consumed:
            raise StopIteration
        if isinstance(self._data, bytes):
            self._sync_consumed = True
            if self._data:
                return self._data
        raise StopIteration

    def __aiter__(self):
        self._async_consumed = False
        return self

    async def __anext__(self):
        if self._async_consumed:
            raise StopAsyncIteration
        if isinstance(self._data, bytes):
            self._async_consumed = True
            if self._data:
                return self._data
        raise StopAsyncIteration

    def read(self):
        """Read all bytes."""
        if isinstance(self._data, bytes):
            return self._data
        return b""

    async def aread(self):
        """Read all bytes asynchronously."""
        if isinstance(self._data, bytes):
            return self._data
        return b""

    def close(self):
        pass

    async def aclose(self):
        pass

    def __repr__(self):
        return "<DualIteratorStream>"


class _ResponseSyncIteratorStream:
    """Sync-only stream wrapper for Response iterators that tracks consumption."""

    def __init__(self, iterator, owner):
        # Handle iterables that aren't iterators
        if hasattr(iterator, '__iter__') and not hasattr(iterator, '__next__'):
            self._iterator = iter(iterator)
        else:
            self._iterator = iterator
        self._owner = owner
        self._consumed = False

    def __iter__(self):
        if self._consumed or self._owner._stream_consumed:
            raise StreamConsumed()
        return self

    def __next__(self):
        if self._consumed:
            raise StopIteration
        try:
            return next(self._iterator)
        except StopIteration:
            self._consumed = True
            self._owner._stream_consumed = True
            raise

    def read(self):
        """Read all bytes."""
        if self._consumed or self._owner._stream_consumed:
            raise StreamConsumed()
        result = b"".join(self)
        return result

    def close(self):
        pass

    def __repr__(self):
        return "<ResponseSyncIteratorStream>"


class _ResponseAsyncIteratorStream:
    """Async-only stream wrapper for Response async iterators that tracks consumption."""

    def __init__(self, iterator, owner):
        self._iterator = iterator
        self._owner = owner
        self._consumed = False

    def __aiter__(self):
        if self._consumed or self._owner._stream_consumed:
            raise StreamConsumed()
        return self

    async def __anext__(self):
        if self._consumed:
            raise StopAsyncIteration
        try:
            return await self._iterator.__anext__()
        except StopAsyncIteration:
            self._consumed = True
            self._owner._stream_consumed = True
            raise

    async def aread(self):
        """Read all bytes asynchronously."""
        if self._consumed or self._owner._stream_consumed:
            raise StreamConsumed()
        result = b"".join([part async for part in self])
        return result

    async def aclose(self):
        pass

    def __repr__(self):
        return "<ResponseAsyncIteratorStream>"
