# Transport base classes and implementations

from ._core import (
    Response as _Response,
    MockTransport as _RustMockTransport,
)


class BaseTransport:
    """Base class for sync HTTP transport implementations.

    Subclass and implement handle_request to create custom transports.
    """

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()
        return None

    def close(self):
        pass

    def handle_request(self, request):
        raise NotImplementedError("Subclasses must implement handle_request()")


class AsyncBaseTransport:
    """Base class for async HTTP transport implementations.

    Subclass and implement handle_async_request to create custom transports.
    """

    async def __aenter__(self):
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        await self.aclose()
        return None

    async def aclose(self):
        pass

    async def handle_async_request(self, request):
        raise NotImplementedError("Subclasses must implement handle_async_request()")


class MockTransport(AsyncBaseTransport):
    """Mock transport for testing - calls a handler function to generate responses.

    This is a Python wrapper around the Rust MockTransport that properly preserves
    Response objects with streams.
    """

    def __init__(self, handler=None):
        self._handler = handler
        self._rust_transport = _RustMockTransport(handler)

    @property
    def handler(self):
        """Public access to the handler function."""
        return self._handler

    def handle_request(self, request):
        """Handle a sync request by calling the handler."""
        # Import here to avoid circular imports
        from ._response import Response

        if self._handler is None:
            return Response(200)
        result = self._handler(request)
        if isinstance(result, Response):
            return result
        elif isinstance(result, _Response):
            return Response(result)
        return Response(result)

    async def handle_async_request(self, request):
        """Handle an async request by calling the handler."""
        import inspect

        # Import here to avoid circular imports
        from ._response import Response

        if self._handler is None:
            return Response(200)
        result = self._handler(request)
        if inspect.iscoroutine(result):
            result = await result
        if isinstance(result, Response):
            return result
        elif isinstance(result, _Response):
            return Response(result)
        return Response(result)

    def __repr__(self):
        return "<MockTransport>"


# AsyncMockTransport is an alias for MockTransport (it handles both sync and async)
AsyncMockTransport = MockTransport


class ASGITransport(AsyncBaseTransport):
    """ASGI transport for testing ASGI applications.

    This transport allows you to test ASGI applications directly without
    making actual network requests.

    Example:
        async def app(scope, receive, send):
            await send({
                "type": "http.response.start",
                "status": 200,
                "headers": [[b"content-type", b"text/plain"]],
            })
            await send({
                "type": "http.response.body",
                "body": b"Hello, World!",
            })

        transport = ASGITransport(app=app)
        async with AsyncClient(transport=transport) as client:
            response = await client.get("http://testserver/")
    """

    def __init__(
        self,
        app,
        raise_app_exceptions: bool = True,
        root_path: str = "",
        client: tuple = ("127.0.0.1", 123),
    ):
        self.app = app
        self.raise_app_exceptions = raise_app_exceptions
        self.root_path = root_path
        self.client = client

    async def handle_async_request(self, request):
        """Handle an async request by calling the ASGI app."""
        # Import here to avoid circular imports
        from ._response import Response

        # Get request details
        url = request.url
        method = request.method
        headers = request.headers

        # Build ASGI scope
        scheme = url.scheme if hasattr(url, "scheme") else "http"
        host = url.host if hasattr(url, "host") else "localhost"
        port = url.port
        path = url.path if hasattr(url, "path") else "/"
        query_string = url.query if hasattr(url, "query") else b""

        # Handle query as bytes
        if isinstance(query_string, str):
            query_string = query_string.encode("utf-8")

        # Get raw_path (path without query string, percent-encoded)
        raw_path = path.encode("utf-8") if isinstance(path, str) else path

        # Build headers list for ASGI (Host header should be first)
        asgi_headers = []
        host_header = None
        for key, value in headers.items():
            key_bytes = key.encode("latin-1") if isinstance(key, str) else key
            value_bytes = value.encode("latin-1") if isinstance(value, str) else value
            if key.lower() == "host":
                host_header = [key_bytes, value_bytes]
            else:
                asgi_headers.append([key_bytes, value_bytes])
        # Insert Host header at the beginning
        if host_header:
            asgi_headers.insert(0, host_header)

        # Determine server tuple
        if port is None:
            port = 443 if scheme == "https" else 80

        scope = {
            "type": "http",
            "asgi": {"version": "3.0"},
            "http_version": "1.1",
            "method": method,
            "headers": asgi_headers,
            "path": path,
            "raw_path": raw_path,
            "query_string": query_string,
            "root_path": self.root_path,
            "scheme": scheme,
            "server": (host, port),
            "client": self.client,
            "extensions": {},
        }

        # Get request body
        body = request.content if hasattr(request, "content") else b""
        if body is None:
            body = b""

        # State for receive/send
        body_sent = False
        response_started = False
        response_complete = False
        status_code = None
        response_headers = []
        body_parts = []

        async def receive():
            nonlocal body_sent

            if not body_sent:
                body_sent = True
                return {
                    "type": "http.request",
                    "body": body,
                    "more_body": False,
                }
            else:
                # After body is sent and response is complete, send disconnect
                return {"type": "http.disconnect"}

        async def send(message):
            nonlocal \
                response_started, \
                response_complete, \
                status_code, \
                response_headers, \
                body_parts

            if message["type"] == "http.response.start":
                response_started = True
                status_code = message["status"]
                # Convert headers
                for h in message.get("headers", []):
                    if isinstance(h, (list, tuple)) and len(h) == 2:
                        key = (
                            h[0].decode("latin-1") if isinstance(h[0], bytes) else h[0]
                        )
                        value = (
                            h[1].decode("latin-1")
                            if isinstance(h[1], bytes)
                            else str(h[1])
                        )
                        response_headers.append((key, value))

            elif message["type"] == "http.response.body":
                body_chunk = message.get("body", b"")
                if body_chunk:
                    body_parts.append(body_chunk)
                if not message.get("more_body", False):
                    response_complete = True

        # Run the ASGI app
        try:
            await self.app(scope, receive, send)
        except Exception:
            if self.raise_app_exceptions:
                raise
            # Return 500 error if app raises
            if not response_started:
                status_code = 500
                response_headers = [(b"content-type", b"text/plain")]
                body_parts = [b"Internal Server Error"]

        # If no response was started, return 500
        if status_code is None:
            status_code = 500
            response_headers = []
            body_parts = [b"Internal Server Error"]

        # Build response
        content = b"".join(body_parts)
        response = Response(
            status_code,
            headers=response_headers,
            content=content,
        )

        # Set request on response
        response._request = request
        response._url = request.url if hasattr(request, "url") else None

        return response

    def __repr__(self):
        return f"<ASGITransport app={self.app!r}>"
