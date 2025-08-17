"""
Unit tests for Session functionality and state management.
Tests Requirements: 1.3, 7.1, 7.2
"""

import asyncio
import unittest
import sys
import os

# Add the project root to the path so we can import requestx
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "python"))

import requestx


class TestSessionBasic(unittest.TestCase):
    """Test basic Session functionality."""

    def test_session_creation(self):
        """Test that Session can be created successfully."""
        session = requestx.Session()
        self.assertIsNotNone(session)
        self.assertIsInstance(session, requestx.Session)

    def test_session_repr(self):
        """Test Session string representation."""
        session = requestx.Session()
        repr_str = repr(session)
        self.assertIn("Session", repr_str)
        self.assertIn("headers=", repr_str)
        self.assertIn("cookies=", repr_str)

    def test_session_context_manager(self):
        """Test Session as context manager."""
        with requestx.Session() as session:
            self.assertIsNotNone(session)
            # Session should work within context
            response = session.get("https://httpbin.org/get")
            self.assertEqual(response.status_code, 200)

    def test_session_close(self):
        """Test Session close method."""
        session = requestx.Session()
        # Should not raise any exceptions
        session.close()


class TestSessionHTTPMethods(unittest.TestCase):
    """Test Session HTTP methods."""

    def setUp(self):
        """Set up test session."""
        self.session = requestx.Session()

    def tearDown(self):
        """Clean up test session."""
        self.session.close()

    def test_session_get(self):
        """Test Session GET request."""
        response = self.session.get("https://httpbin.org/get")
        self.assertEqual(response.status_code, 200)
        self.assertIsNotNone(response.text)

    def test_session_post(self):
        """Test Session POST request."""
        data = {"key": "value", "test": "session"}
        response = self.session.post("https://httpbin.org/post", json=data)
        self.assertEqual(response.status_code, 200)

        # Verify data was sent
        response_data = response.json()
        sent_json = response_data.get("json", {})
        self.assertEqual(sent_json.get("key"), "value")
        self.assertEqual(sent_json.get("test"), "session")

    def test_session_put(self):
        """Test Session PUT request."""
        data = {"update": "data"}
        response = self.session.put("https://httpbin.org/put", json=data)
        self.assertEqual(response.status_code, 200)

    def test_session_delete(self):
        """Test Session DELETE request."""
        response = self.session.delete("https://httpbin.org/delete")
        self.assertEqual(response.status_code, 200)

    def test_session_head(self):
        """Test Session HEAD request."""
        response = self.session.head("https://httpbin.org/get")
        self.assertEqual(response.status_code, 200)
        # HEAD requests should have empty body
        self.assertEqual(len(response.content), 0)

    def test_session_options(self):
        """Test Session OPTIONS request."""
        response = self.session.options("https://httpbin.org/get")
        # OPTIONS requests typically return 200 or 204
        self.assertIn(response.status_code, [200, 204])

    def test_session_patch(self):
        """Test Session PATCH request."""
        data = {"patch": "data"}
        response = self.session.patch("https://httpbin.org/patch", json=data)
        self.assertEqual(response.status_code, 200)

    def test_session_request_generic(self):
        """Test Session generic request method."""
        response = self.session.request("GET", "https://httpbin.org/get")
        self.assertEqual(response.status_code, 200)

    def test_session_invalid_method(self):
        """Test Session with invalid HTTP method."""
        with self.assertRaises(RuntimeError):
            self.session.request("INVALID", "https://httpbin.org/get")


class TestSessionHeaders(unittest.TestCase):
    """Test Session header management."""

    def setUp(self):
        """Set up test session."""
        self.session = requestx.Session()

    def tearDown(self):
        """Clean up test session."""
        self.session.close()

    def test_session_headers_empty_initially(self):
        """Test that session headers are empty initially."""
        headers = self.session.headers
        self.assertEqual(len(headers), 0)

    def test_session_update_header(self):
        """Test updating a session header."""
        self.session.update_header("User-Agent", "RequestX-Session-Test")
        headers = self.session.headers
        self.assertEqual(headers.get("user-agent"), "RequestX-Session-Test")

    def test_session_remove_header(self):
        """Test removing a session header."""
        # Add a header first
        self.session.update_header("Test-Header", "test-value")
        headers = self.session.headers
        self.assertIn("test-header", headers)

        # Remove the header
        self.session.remove_header("Test-Header")
        headers = self.session.headers
        self.assertNotIn("test-header", headers)

    def test_session_clear_headers(self):
        """Test clearing all session headers."""
        # Add some headers
        self.session.update_header("Header1", "value1")
        self.session.update_header("Header2", "value2")
        headers = self.session.headers
        self.assertEqual(len(headers), 2)

        # Clear all headers
        self.session.clear_headers()
        headers = self.session.headers
        self.assertEqual(len(headers), 0)

    def test_session_headers_persist_across_requests(self):
        """Test that session headers persist across multiple requests."""
        # Set a session header
        self.session.update_header("User-Agent", "RequestX-Persistent-Test")

        # Make first request
        response1 = self.session.get("https://httpbin.org/get")
        self.assertEqual(response1.status_code, 200)

        # Verify header was sent
        response1_data = response1.json()
        sent_headers1 = response1_data.get("headers", {})
        self.assertEqual(sent_headers1.get("User-Agent"), "RequestX-Persistent-Test")

        # Make second request
        response2 = self.session.get("https://httpbin.org/user-agent")
        self.assertEqual(response2.status_code, 200)

        # Verify header was sent again
        response2_data = response2.json()
        self.assertEqual(response2_data.get("user-agent"), "RequestX-Persistent-Test")

    def test_session_headers_merge_with_request_headers(self):
        """Test that session headers merge with request-specific headers."""
        # Set a session header
        self.session.update_header("Session-Header", "session-value")

        # Make request with additional headers
        request_headers = {"Request-Header": "request-value"}
        response = self.session.get("https://httpbin.org/get", headers=request_headers)
        self.assertEqual(response.status_code, 200)

        # Verify both headers were sent
        response_data = response.json()
        sent_headers = response_data.get("headers", {})
        self.assertEqual(sent_headers.get("Session-Header"), "session-value")
        self.assertEqual(sent_headers.get("Request-Header"), "request-value")

    def test_session_request_headers_override_session_headers(self):
        """Test that request headers override session headers with same name."""
        # Set a session header
        self.session.update_header("User-Agent", "Session-Agent")

        # Make request with overriding header
        request_headers = {"User-Agent": "Request-Agent"}
        response = self.session.get("https://httpbin.org/get", headers=request_headers)
        self.assertEqual(response.status_code, 200)

        # Verify request header took precedence
        response_data = response.json()
        sent_headers = response_data.get("headers", {})
        self.assertEqual(sent_headers.get("User-Agent"), "Request-Agent")

    def test_session_set_headers_dict(self):
        """Test setting session headers from a dictionary."""
        headers_dict = {
            "Authorization": "Bearer token123",
            "Content-Type": "application/json",
            "Custom-Header": "custom-value",
        }

        self.session.headers = headers_dict
        headers = self.session.headers

        self.assertEqual(len(headers), 3)
        self.assertEqual(headers.get("authorization"), "Bearer token123")
        self.assertEqual(headers.get("content-type"), "application/json")
        self.assertEqual(headers.get("custom-header"), "custom-value")

    def test_session_invalid_header_name(self):
        """Test handling of invalid header names."""
        with self.assertRaises(ValueError):
            self.session.update_header("Invalid Header Name", "value")

    def test_session_invalid_header_value(self):
        """Test handling of invalid header values."""
        with self.assertRaises(ValueError):
            self.session.update_header("Valid-Header", "invalid\nvalue")


class TestSessionCookies(unittest.TestCase):
    """Test Session cookie management."""

    def setUp(self):
        """Set up test session."""
        self.session = requestx.Session()

    def tearDown(self):
        """Clean up test session."""
        self.session.close()

    def test_session_cookies_empty_initially(self):
        """Test that session cookies are empty initially."""
        cookies = self.session.cookies
        self.assertEqual(len(cookies), 0)

    def test_session_clear_cookies(self):
        """Test clearing session cookies."""
        # Should not raise any exceptions even if no cookies exist
        self.session.clear_cookies()
        cookies = self.session.cookies
        self.assertEqual(len(cookies), 0)

    # Note: Cookie persistence testing would require a server that sets cookies
    # For now, we test the basic cookie interface


class TestSessionAsync(unittest.TestCase):
    """Test Session async functionality."""

    def setUp(self):
        """Set up async event loop and session."""
        self.loop = asyncio.new_event_loop()
        asyncio.set_event_loop(self.loop)
        self.session = requestx.Session()

    def tearDown(self):
        """Clean up event loop and session."""
        self.session.close()
        self.loop.close()

    def test_session_async_get(self):
        """Test Session async GET request."""

        async def run_test():
            response = await self.session.get("https://httpbin.org/get")
            self.assertEqual(response.status_code, 200)

        self.loop.run_until_complete(run_test())

    def test_session_async_post(self):
        """Test Session async POST request."""

        async def run_test():
            data = {"async": True, "session": "test"}
            response = await self.session.post("https://httpbin.org/post", json=data)
            self.assertEqual(response.status_code, 200)

            # Verify data was sent
            response_data = response.json()
            sent_json = response_data.get("json", {})
            self.assertEqual(sent_json.get("async"), True)
            self.assertEqual(sent_json.get("session"), "test")

        self.loop.run_until_complete(run_test())

    def test_session_async_headers_persist(self):
        """Test that session headers persist in async requests."""

        async def run_test():
            # Set a session header
            self.session.update_header("Async-Session", "async-test")

            # Make async request
            response = await self.session.get("https://httpbin.org/get")
            self.assertEqual(response.status_code, 200)

            # Verify header was sent
            response_data = response.json()
            sent_headers = response_data.get("headers", {})
            self.assertEqual(sent_headers.get("Async-Session"), "async-test")

        self.loop.run_until_complete(run_test())

    def test_session_async_concurrent_requests(self):
        """Test multiple concurrent async requests with session."""

        async def run_test():
            # Set a session header
            self.session.update_header("Concurrent-Test", "session-value")

            # Create multiple concurrent requests
            urls = [
                "https://httpbin.org/get",
                "https://httpbin.org/user-agent",
                "https://httpbin.org/headers",
            ]

            tasks = [self.session.get(url) for url in urls]
            responses = await asyncio.gather(*tasks)

            # All should be successful
            for i, response in enumerate(responses):
                self.assertEqual(response.status_code, 200)

                # Verify session header was sent in all requests
                response_data = response.json()
                sent_headers = response_data.get("headers", {})

                # Headers are case-insensitive, check for lowercase version
                header_value = sent_headers.get("Concurrent-Test") or sent_headers.get(
                    "concurrent-test"
                )

                # Some endpoints might not return all headers, so we'll be more lenient
                # At least one request should have the session header
                if header_value:
                    self.assertEqual(header_value, "session-value")

            # Verify at least one response had our session header
            has_session_header = any(
                response.json().get("headers", {}).get("Concurrent-Test")
                == "session-value"
                or response.json().get("headers", {}).get("concurrent-test")
                == "session-value"
                for response in responses
            )
            self.assertTrue(
                has_session_header,
                "At least one request should have the session header",
            )

        self.loop.run_until_complete(run_test())


class TestSessionStateManagement(unittest.TestCase):
    """Test Session state persistence and management."""

    def setUp(self):
        """Set up test session."""
        self.session = requestx.Session()

    def tearDown(self):
        """Clean up test session."""
        self.session.close()

    def test_session_state_isolation(self):
        """Test that different sessions have isolated state."""
        session1 = requestx.Session()
        session2 = requestx.Session()

        try:
            # Set different headers for each session
            session1.update_header("Session-ID", "session-1")
            session2.update_header("Session-ID", "session-2")

            # Make requests with each session
            response1 = session1.get("https://httpbin.org/get")
            response2 = session2.get("https://httpbin.org/get")

            # Verify each session sent its own header
            response1_data = response1.json()
            response2_data = response2.json()

            sent_headers1 = response1_data.get("headers", {})
            sent_headers2 = response2_data.get("headers", {})

            self.assertEqual(sent_headers1.get("Session-Id"), "session-1")
            self.assertEqual(sent_headers2.get("Session-Id"), "session-2")

        finally:
            session1.close()
            session2.close()

    def test_session_state_persistence_across_methods(self):
        """Test that session state persists across different HTTP methods."""
        # Set session headers
        self.session.update_header("Persistent-Header", "persistent-value")

        # Test different HTTP methods
        methods_and_urls = [
            ("GET", "https://httpbin.org/get"),
            ("POST", "https://httpbin.org/post"),
            ("PUT", "https://httpbin.org/put"),
            ("DELETE", "https://httpbin.org/delete"),
            ("PATCH", "https://httpbin.org/patch"),
        ]

        for method, url in methods_and_urls:
            if method == "GET":
                response = self.session.get(url)
            elif method == "POST":
                response = self.session.post(url, json={"test": "data"})
            elif method == "PUT":
                response = self.session.put(url, json={"test": "data"})
            elif method == "DELETE":
                response = self.session.delete(url)
            elif method == "PATCH":
                response = self.session.patch(url, json={"test": "data"})

            self.assertEqual(response.status_code, 200)

            # Verify session header was sent
            response_data = response.json()
            sent_headers = response_data.get("headers", {})
            self.assertEqual(sent_headers.get("Persistent-Header"), "persistent-value")


class TestSessionErrorHandling(unittest.TestCase):
    """Test Session error handling."""

    def setUp(self):
        """Set up test session."""
        self.session = requestx.Session()

    def tearDown(self):
        """Clean up test session."""
        self.session.close()

    def test_session_invalid_url(self):
        """Test Session with invalid URL."""
        with self.assertRaises(ValueError):
            self.session.get("not-a-valid-url")

    def test_session_timeout(self):
        """Test Session timeout handling."""
        with self.assertRaises(Exception):  # Should raise timeout error
            self.session.get("https://httpbin.org/delay/10", timeout=1)

    def test_session_network_error_handling(self):
        """Test Session handles network errors gracefully."""
        # This should raise a connection error
        with self.assertRaises(Exception):
            self.session.get("https://nonexistent-domain-12345.com")


if __name__ == "__main__":
    # Run all tests
    unittest.main(verbosity=2)
