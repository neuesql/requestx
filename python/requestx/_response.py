# Response wrapper with proper stream property

from ._core import (
    Response as _Response,
    HTTPStatusError as _HTTPStatusError,
    decompress as _decompress,
)
from ._exceptions import (
    DecodingError,
    ResponseNotRead,
    StreamConsumed,
    StreamClosed,
)
from ._streams import (
    ByteStream,
    _ResponseSyncIteratorStream,
    _ResponseAsyncIteratorStream,
)


class HTTPStatusError(_HTTPStatusError):
    """HTTP Status Error with request and response attributes.

    Raised by Response.raise_for_status() when the response has a non-2xx status code.
    """

    def __init__(self, message, *, request=None, response=None):
        super().__init__(message)
        self._request = request
        self._response = response

    @property
    def request(self):
        return self._request

    @property
    def response(self):
        return self._response


class Response:
    """HTTP Response wrapper with proper stream support and raise_for_status.

    Wraps the Rust Response to provide additional Python functionality.
    Can be constructed either by wrapping a Rust Response or directly with status_code.
    """

    def __init__(
        self,
        status_code_or_response=None,
        *,
        content=None,
        headers=None,
        text=None,
        html=None,
        json=None,
        stream=None,
        request=None,
        default_encoding=None,
        status_code=None,
    ):
        # Initialize attributes
        self._history = []
        self._url = None
        self._next_request = None
        self._request = None
        self._decoded_content = None
        self._default_encoding = default_encoding
        self._stream_content = None  # For storing async iterators
        self._sync_stream_content = None  # For storing sync iterators
        self._raw_content = None  # For caching consumed stream content
        self._raw_chunks = None  # For storing individual chunks for streaming
        self._num_bytes_downloaded = 0  # Track bytes downloaded during streaming
        self._stream_consumed = False  # Track if stream was consumed via iteration
        self._is_stream = False  # Track if this is a streaming response
        self._unpickled_stream_not_read = False  # Track if unpickled from unread stream
        self._text_accessed = False  # Track if .text was accessed
        self._stream_not_read = (
            False  # Track if streaming response needs aread() before accessing content
        )
        self._stream_object = None  # Reference to stream object for aclose()

        # Handle status_code as keyword argument
        if status_code is not None and status_code_or_response is None:
            status_code_or_response = status_code

        # Unwrap _WrappedRequest to get the underlying Rust request
        rust_request = request
        if request is not None and hasattr(request, "_rust_request"):
            rust_request = request._rust_request
            # Store the wrapped request for later access
            self._request = request

        # If passed a Rust _Response, wrap it
        if isinstance(status_code_or_response, _Response):
            self._response = status_code_or_response
        else:
            # Handle stream parameter (AsyncByteStream or similar)
            # If stream is provided, it takes precedence over content
            if stream is not None and content is None:
                # Check if stream is an async iterator
                if hasattr(stream, "__aiter__"):
                    self._stream_content = stream
                    self._is_stream = True
                    self._stream_object = stream  # Keep reference for aclose()
                    self._response = _Response(
                        status_code_or_response,
                        content=b"",
                        headers=headers,
                        request=rust_request,
                    )
                    return
                elif hasattr(stream, "__iter__"):
                    self._sync_stream_content = stream
                    self._is_stream = True
                    self._stream_object = stream  # Keep reference for close()
                    self._response = _Response(
                        status_code_or_response,
                        content=b"",
                        headers=headers,
                        request=rust_request,
                    )
                    return

            # Check if content is an async iterator or sync iterator
            is_async_iter = hasattr(content, "__aiter__") and hasattr(
                content, "__anext__"
            )
            # Check for sync iterator/iterable (has __iter__ but not a built-in type)
            # This handles both generators (__iter__ + __next__) and iterables (just __iter__)
            is_sync_iter = (
                hasattr(content, "__iter__")
                and not isinstance(content, (bytes, str, list, dict, type(None)))
                and not hasattr(content, "__aiter__")  # Not an async iterable
            )

            if is_async_iter:
                # Store async iterator for later consumption
                self._stream_content = content
                self._is_stream = True
                # Check if Content-Length was provided
                has_content_length = False
                if headers is not None:
                    if isinstance(headers, dict):
                        has_content_length = any(
                            k.lower() == "content-length" for k in headers.keys()
                        )
                    elif isinstance(headers, list):
                        has_content_length = any(
                            k.lower() == "content-length" for k, v in headers
                        )
                    else:
                        has_content_length = any(
                            k.lower() == "content-length" for k, v in headers.items()
                        )
                # Only add Transfer-Encoding: chunked if Content-Length is not provided
                if has_content_length:
                    stream_headers = headers
                elif headers is None:
                    stream_headers = [("transfer-encoding", "chunked")]
                elif isinstance(headers, list):
                    stream_headers = list(headers) + [("transfer-encoding", "chunked")]
                elif isinstance(headers, dict):
                    stream_headers = list(headers.items()) + [
                        ("transfer-encoding", "chunked")
                    ]
                else:
                    stream_headers = list(headers.items()) + [
                        ("transfer-encoding", "chunked")
                    ]
                # Create response without content - will be filled in aread()
                self._response = _Response(
                    status_code_or_response,
                    content=b"",
                    headers=stream_headers,
                    text=text,
                    html=html,
                    json=json,
                    stream=stream,
                    request=rust_request,
                )
            elif is_sync_iter:
                # Store sync iterator for lazy consumption, like async iterators
                self._sync_stream_content = content
                self._is_stream = True
                # Check if Content-Length was provided
                has_content_length = False
                if headers is not None:
                    if isinstance(headers, dict):
                        has_content_length = any(
                            k.lower() == "content-length" for k in headers.keys()
                        )
                    elif isinstance(headers, list):
                        has_content_length = any(
                            k.lower() == "content-length" for k, v in headers
                        )
                    else:
                        has_content_length = any(
                            k.lower() == "content-length" for k, v in headers.items()
                        )
                # Only add Transfer-Encoding: chunked if Content-Length is not provided
                if has_content_length:
                    stream_headers = headers
                elif headers is None:
                    stream_headers = [("transfer-encoding", "chunked")]
                elif isinstance(headers, list):
                    stream_headers = list(headers) + [("transfer-encoding", "chunked")]
                elif isinstance(headers, dict):
                    stream_headers = list(headers.items()) + [
                        ("transfer-encoding", "chunked")
                    ]
                else:
                    stream_headers = list(headers.items()) + [
                        ("transfer-encoding", "chunked")
                    ]
                self._response = _Response(
                    status_code_or_response,
                    content=b"",
                    headers=stream_headers,
                    text=text,
                    html=html,
                    json=json,
                    stream=stream,
                    request=rust_request,
                )
            elif isinstance(content, list):
                # Content is a list of bytes chunks
                consumed_content = b"".join(content)
                self._raw_content = consumed_content
                self._response = _Response(
                    status_code_or_response,
                    content=consumed_content,
                    headers=headers,
                    text=text,
                    html=html,
                    json=json,
                    stream=stream,
                    request=rust_request,
                )
            else:
                # Regular content (bytes, str, or None)
                self._response = _Response(
                    status_code_or_response,
                    content=content,
                    headers=headers,
                    text=text,
                    html=html,
                    json=json,
                    stream=stream,
                    request=rust_request,
                )

        # Eagerly decode content if provided directly (not streaming)
        # This ensures DecodingError is raised during construction for invalid data
        if (
            content is not None
            and not hasattr(content, "__aiter__")
            and not hasattr(content, "__next__")
        ):
            if isinstance(content, (bytes, str, list)):
                # Trigger decompression to catch errors early
                _ = self.content

    def __getattr__(self, name):
        """Delegate attribute access to the underlying Rust response."""
        return getattr(self._response, name)

    @property
    def stream(self):
        """Get the response body as a stream based on content type."""
        # Check if this is a sync iterator stream
        if self._sync_stream_content is not None:
            return _ResponseSyncIteratorStream(self._sync_stream_content, self)
        # Check if this is an async iterator stream
        if self._stream_content is not None:
            return _ResponseAsyncIteratorStream(self._stream_content, self)
        # Check if stream was already consumed (but content is not available)
        # If content is available, we can still return a ByteStream
        if (
            self._stream_consumed
            and self._raw_content is None
            and not self._response.content
        ):
            raise StreamConsumed()
        # Regular content - return dual-mode stream
        content = (
            self._raw_content
            if self._raw_content is not None
            else self._response.content
        )
        return ByteStream(content)

    @property
    def status_code(self):
        return self._response.status_code

    @property
    def reason_phrase(self):
        return self._response.reason_phrase

    @property
    def headers(self):
        return self._response.headers

    @property
    def url(self):
        # Return stored URL if set, otherwise from response
        if self._url is not None:
            return self._url
        return self._response.url

    @url.setter
    def url(self, value):
        self._url = value

    @property
    def content(self):
        # If this was unpickled from an unread async stream, raise ResponseNotRead
        if self._unpickled_stream_not_read:
            raise ResponseNotRead()
        # If this is a streaming response that hasn't been read via aread(), raise ResponseNotRead
        if self._stream_not_read:
            raise ResponseNotRead()
        if self._decoded_content is not None:
            return self._decoded_content

        # Use raw_content if we consumed a stream, otherwise use response content
        raw_content = (
            self._raw_content
            if self._raw_content is not None
            else self._response.content
        )
        if not raw_content:
            return raw_content

        # Check Content-Encoding header for decompression
        content_encoding = self.headers.get("content-encoding", "").lower()
        if not content_encoding or content_encoding == "identity":
            return raw_content

        # Decode content based on encoding(s) - handle multiple encodings
        decompressed = raw_content
        encodings = [e.strip() for e in content_encoding.split(",")]

        # Process encodings in reverse order (last applied first)
        for encoding in reversed(encodings):
            if encoding == "identity":
                continue
            decompressed = self._decompress(decompressed, encoding)

        self._decoded_content = decompressed
        return decompressed

    def _decompress(self, data, encoding):
        """Decompress data based on encoding. Delegates to Rust."""
        if not data:
            return data
        try:
            return _decompress(data, encoding)
        except Exception as e:
            # Convert Rust DecodingError to Python DecodingError
            raise DecodingError(str(e)) from None

    @property
    def text(self):
        # Mark text as accessed (for encoding setter validation)
        self._text_accessed = True
        # If we have consumed raw content, decode it ourselves
        raw_content = (
            self._raw_content
            if self._raw_content is not None
            else self._response.content
        )
        if not raw_content:
            return ""
        encoding = self._get_encoding()
        return raw_content.decode(encoding, errors="replace")

    @property
    def encoding(self):
        """Get the encoding used for text decoding."""
        return self._get_encoding()

    @property
    def charset_encoding(self):
        """Get the charset from the Content-Type header, or None if not specified."""
        return self._response._extract_charset()

    @encoding.setter
    def encoding(self, value):
        """Set explicit encoding for text decoding."""
        # If text was already accessed, raise ValueError
        if getattr(self, "_text_accessed", False):
            raise ValueError(
                "The encoding cannot be set after .text has been accessed."
            )
        # Store explicit encoding in Python wrapper
        self._explicit_encoding = value
        # Clear any cached decoded content
        self._decoded_content = None

    def _get_encoding(self):
        """Get the encoding for text decoding."""
        # First check explicit encoding set via property
        if hasattr(self, "_explicit_encoding") and self._explicit_encoding is not None:
            return self._explicit_encoding
        # Delegate charset extraction from Content-Type to Rust
        charset = self._response._extract_charset()
        if charset is not None:
            import codecs

            try:
                codecs.lookup(charset)
                return charset
            except LookupError:
                return "utf-8"
        # Use default_encoding if provided
        if self._default_encoding is not None:
            if callable(self._default_encoding):
                detected = self._default_encoding(self.content)
                if detected:
                    return detected
            else:
                return self._default_encoding
        return "utf-8"

    @property
    def request(self):
        if self._request is not None:
            return self._request
        return self._response.request

    @request.setter
    def request(self, value):
        self._request = value
        self._response.request = value

    @property
    def next_request(self):
        """Return the next request for following redirects, or None if not a redirect."""
        return self._next_request

    @next_request.setter
    def next_request(self, value):
        self._next_request = value

    @property
    def elapsed(self):
        """Get elapsed time. Raises RuntimeError if response is not closed."""
        # If this is a streaming response that hasn't been closed, raise RuntimeError
        if self._is_stream and not self.is_closed:
            raise RuntimeError(
                ".elapsed accessed before the response was read or the stream was closed."
            )
        return self._response.elapsed

    @property
    def is_success(self):
        return self._response.is_success

    @property
    def is_informational(self):
        return self._response.is_informational

    @property
    def is_redirect(self):
        return self._response.is_redirect

    @property
    def is_client_error(self):
        return self._response.is_client_error

    @property
    def is_server_error(self):
        return self._response.is_server_error

    @property
    def is_stream_consumed(self):
        """Return True if the stream has been consumed."""
        return self._stream_consumed

    @property
    def history(self):
        """List of responses in redirect/auth chain."""
        return self._history

    @property
    def num_bytes_downloaded(self):
        """Number of bytes downloaded so far."""
        # If we have a streaming counter, use it
        if self._num_bytes_downloaded > 0:
            return self._num_bytes_downloaded
        # Otherwise delegate to Rust response
        return self._response.num_bytes_downloaded

    def __repr__(self):
        return f"<Response [{self.status_code} {self.reason_phrase}]>"

    def __getstate__(self):
        """Pickle support - get state."""
        # Get request - try Python side first, then Rust side
        request = self._request
        if request is None:
            try:
                request = self._response.request
            except RuntimeError:
                request = None
        return {
            "status_code": self.status_code,
            "headers": list(self.headers.multi_items()),
            "content": (
                self.content if not self._is_stream or self._raw_content else b""
            ),
            "request": request,
            "url": self._url,
            "history": self._history,
            "default_encoding": self._default_encoding,
            "is_stream": self._is_stream,
            "stream_consumed": self._stream_consumed,
            "is_closed": self.is_closed,
            "has_stream_content": self._stream_content is not None,
        }

    def __setstate__(self, state):
        """Pickle support - restore state."""
        # Create a new Rust response with the saved state
        self._response = _Response(
            state["status_code"],
            content=state["content"],
            headers=state["headers"],
            request=state["request"],
        )
        self._request = state["request"]
        self._url = state["url"]
        self._history = state["history"]
        self._default_encoding = state["default_encoding"]
        self._is_stream = state["is_stream"]
        # If we have content, mark stream as consumed (content is available)
        # If no content but it was a stream that wasn't read, keep original state
        if state["content"]:
            self._stream_consumed = True
        else:
            self._stream_consumed = state["stream_consumed"]
        self._stream_content = None  # Can't pickle stream content
        self._raw_content = state["content"] if state["content"] else None
        self._raw_chunks = None
        self._decoded_content = None
        self._next_request = None
        self._num_bytes_downloaded = 0
        self._sync_stream_content = None  # Initialize sync stream content
        self._text_accessed = False  # Text hasn't been accessed after unpickling
        self._stream_not_read = False  # Not a live stream after unpickling
        # Track if this was an async stream that wasn't read before pickling
        self._unpickled_stream_not_read = (
            state.get("has_stream_content") and not state["content"]
        )
        # Mark Rust response as closed/consumed (since we have the content)
        self._response.read()

    def read(self):
        """Read and return the response body."""
        # Check if response is closed before we can read
        if self._is_stream and self.is_closed:
            raise StreamClosed()
        # Check if stream was already consumed via iteration
        if self._is_stream and self._stream_consumed:
            raise StreamConsumed()
        # If we have a pending sync stream, consume it
        if self._sync_stream_content is not None:
            chunks = list(self._sync_stream_content)
            consumed_content = b"".join(chunks)
            self._raw_content = consumed_content
            self._raw_chunks = chunks
            self._response._set_content(consumed_content)
            self._sync_stream_content = None
            self._stream_consumed = True
            return consumed_content
        # Call Rust read() to mark as closed
        self._response.read()
        return self.content

    async def aread(self):
        """Async read and return the response body."""
        # Check if stream was already consumed via iteration
        if self._is_stream and self._stream_consumed:
            raise StreamConsumed()
        # Check if this is an unpickled stream that wasn't read - stream is lost
        if self._unpickled_stream_not_read:
            raise StreamClosed()
        # Check if response is closed before we can read (only for true async streams)
        if self._stream_content is not None and self.is_closed:
            raise StreamClosed()
        # Clear the stream_not_read flag since we're reading now
        self._stream_not_read = False
        # If we have a pending async stream, consume it
        if self._stream_content is not None:
            chunks = []
            async for chunk in self._stream_content:
                chunks.append(chunk)
            self._raw_content = b"".join(chunks)
            self._stream_content = None  # Mark as consumed
            self._stream_consumed = True  # Mark stream as consumed
            # Clear decoded cache to force re-decode with new content
            self._decoded_content = None
            # Set content on Rust side to mark as closed
            self._response._set_content(self._raw_content)
        else:
            # Call Rust aread() to mark as closed
            await self._response.aread()
            self._stream_consumed = True  # Mark stream as consumed
        return self.content

    def iter_bytes(self, chunk_size=None):
        """Iterate over the response body as bytes chunks."""
        # If we have a sync stream that hasn't been consumed, iterate over it
        if self._sync_stream_content is not None:
            chunks = []
            consumed_content = b""
            for chunk in self._sync_stream_content:
                chunks.append(chunk)
                consumed_content += chunk
                self._num_bytes_downloaded += len(chunk)
                if chunk_size is None:
                    if chunk:  # Skip empty chunks
                        yield chunk
                else:
                    # Buffer chunks and yield at chunk_size boundaries
                    pass  # Will handle below
            # Store for later use (don't close the response yet)
            self._raw_content = consumed_content
            self._raw_chunks = chunks
            self._response._set_content_only(consumed_content)
            self._sync_stream_content = None
            self._stream_consumed = True
            # If chunk_size was specified, re-yield from stored content
            if chunk_size is not None:
                for i in range(0, len(consumed_content), chunk_size):
                    yield consumed_content[i : i + chunk_size]
            return
        # Mark stream as consumed after iteration
        self._stream_consumed = True
        # If we have individual chunks, yield them
        if self._raw_chunks is not None and chunk_size is None:
            for chunk in self._raw_chunks:
                if chunk:  # Skip empty chunks
                    yield chunk
        else:
            content = self.content
            if chunk_size is None:
                if content:
                    yield content
            else:
                for i in range(0, len(content), chunk_size):
                    yield content[i : i + chunk_size]

    def iter_text(self, chunk_size=None):
        """Iterate over the response body as text chunks."""
        # Get encoding from content-type or default to utf-8
        encoding = self._get_encoding()
        for chunk in self.iter_bytes(chunk_size):
            if chunk:
                yield chunk.decode(encoding, errors="replace")

    async def aiter_text(self, chunk_size=None):
        """Async iterate over the response body as text chunks."""
        encoding = self._get_encoding()
        for chunk in self.iter_bytes(chunk_size):
            yield chunk.decode(encoding, errors="replace")

    def iter_lines(self):
        """Iterate over the response body as lines."""
        pending = ""
        for text in self.iter_text():
            lines = (pending + text).splitlines(keepends=True)
            pending = ""
            for line in lines:
                if line.endswith(("\r\n", "\r", "\n")):
                    yield line.rstrip("\r\n")
                else:
                    pending = line
        if pending:
            yield pending

    def iter_raw(self, chunk_size=None):
        """Iterate over the raw response body (uncompressed bytes)."""
        # If we have an async stream stored, raise RuntimeError
        if self._stream_content is not None:
            raise RuntimeError(
                "Attempted to call a sync iterator method on an async stream."
            )
        # Use iter_bytes for raw iteration (no decompression in this implementation)
        return self.iter_bytes(chunk_size)

    async def aiter_raw(self, chunk_size=None):
        """Async iterate over the raw response body."""
        # Mark stream as consumed
        self._stream_consumed = True
        # If we have a sync stream (either unconsumed or consumed), raise RuntimeError
        if self._sync_stream_content is not None or self._raw_chunks is not None:
            raise RuntimeError(
                "Attempted to call an async iterator method on a sync stream."
            )

        # If we have an async stream, iterate over it
        if self._stream_content is not None:
            all_content = b""
            buffer = b""
            async for chunk in self._stream_content:
                all_content += chunk
                if chunk_size is None:
                    self._num_bytes_downloaded += len(chunk)
                    yield chunk
                else:
                    buffer += chunk
                    while len(buffer) >= chunk_size:
                        yielded = buffer[:chunk_size]
                        self._num_bytes_downloaded += len(yielded)
                        yield yielded
                        buffer = buffer[chunk_size:]
            # Yield any remaining data (only when using chunk_size)
            if chunk_size is not None and buffer:
                self._num_bytes_downloaded += len(buffer)
                yield buffer
            # Mark stream as consumed and store content
            self._raw_content = all_content
            self._stream_content = None
        else:
            # No async stream, yield from content
            content = self.content
            if chunk_size is None:
                if content:
                    self._num_bytes_downloaded += len(content)
                    yield content
            else:
                for i in range(0, len(content), chunk_size):
                    chunk = content[i : i + chunk_size]
                    self._num_bytes_downloaded += len(chunk)
                    yield chunk

    async def aiter_bytes(self, chunk_size=None):
        """Async iterate over the response body as bytes chunks."""
        # If we have a sync stream (raw_chunks), raise RuntimeError
        if self._stream_content is None and self._raw_chunks is not None:
            raise RuntimeError(
                "Attempted to call an async iterator method on a sync stream."
            )

        # Use aiter_raw for bytes iteration
        async for chunk in self.aiter_raw(chunk_size):
            yield chunk

    async def aiter_lines(self):
        """Async iterate over the response body as lines."""
        # If we have a sync stream (raw_chunks), raise RuntimeError
        if self._stream_content is None and self._raw_chunks is not None:
            raise RuntimeError(
                "Attempted to call an async iterator method on a sync stream."
            )

        encoding = self._get_encoding()
        pending = ""
        async for chunk in self.aiter_bytes():
            text = chunk.decode(encoding, errors="replace")
            lines = (pending + text).splitlines(keepends=True)
            pending = ""
            for line in lines:
                if line.endswith(("\r\n", "\r", "\n")):
                    yield line.rstrip("\r\n")
                else:
                    pending = line
        if pending:
            yield pending

    def close(self):
        """Close the response."""
        # If we have an async stream, raise RuntimeError
        if self._stream_content is not None:
            raise RuntimeError("Attempted to call a sync method on an async stream.")
        self._response.close()

    async def aclose(self):
        """Async close the response."""
        # If we have a sync stream that hasn't been consumed, raise RuntimeError
        if self._sync_stream_content is not None:
            raise RuntimeError("Attempted to call an async method on a sync stream.")
        # Note: Nothing to close for async streams in Python
        self._response.close()

    def json(self, **kwargs):
        # Fast path: no kwargs, delegate entirely to Rust (sonic-rs with BOM detection)
        if not kwargs:
            import json as _json_module
            from ._core import json_from_bytes

            try:
                return json_from_bytes(self.content)
            except ValueError as e:
                # Re-raise as JSONDecodeError for compatibility with tests
                # that catch json.decoder.JSONDecodeError specifically
                raise _json_module.JSONDecodeError(str(e), "", 0) from None

        # Slow path: kwargs passed (e.g. parse_float), fall back to Python json.loads
        import json as json_module
        from ._utils import guess_json_utf

        content = self.content
        encoding = guess_json_utf(content)

        if encoding is not None:
            text = content.decode(encoding)
        else:
            try:
                text = content.decode("utf-8")
            except UnicodeDecodeError:
                text = self.text

        if text.startswith("\ufeff"):
            text = text[1:]

        return json_module.loads(text, **kwargs)

    def raise_for_status(self):
        """Raise HTTPStatusError for non-2xx status codes.

        Returns self for chaining on success.
        """
        # Check that request is set (accessing self.request will raise if not)
        _ = self.request

        # Delegate message building to Rust
        message = self._response._raise_for_status_message()
        if message is None:
            return self

        raise HTTPStatusError(message, request=self.request, response=self)
