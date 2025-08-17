"""
Simple integration tests for Python-Rust binding functionality.
"""

import asyncio
import unittest
import sys
import os

# Add the project root to the path so we can import requestx
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "python"))

import requestx


class TestBasicBindings(unittest.TestCase):
    """Test basic PyO3 bindings functionality."""

    def test_sync_get_basic(self):
        """Test basic synchronous GET request."""
        response = requestx.get("https://httpbin.org/get")
        self.assertEqual(response.status_code, 200)

    def test_sync_get_with_headers(self):
        """Test synchronous GET request with headers."""
        headers = {"User-Agent": "RequestX-Test"}
        response = requestx.get("https://httpbin.org/get", headers=headers)
        self.assertEqual(response.status_code, 200)

        # Verify header was sent
        response_data = response.json()
        sent_headers = response_data.get("headers", {})
        self.assertEqual(sent_headers.get("User-Agent"), "RequestX-Test")

    def test_sync_get_with_params(self):
        """Test synchronous GET request with query parameters."""
        params = {"key": "value", "test": "param"}
        response = requestx.get("https://httpbin.org/get", params=params)
        self.assertEqual(response.status_code, 200)

        # Verify params were sent
        response_data = response.json()
        sent_args = response_data.get("args", {})
        self.assertEqual(sent_args.get("key"), "value")
        self.assertEqual(sent_args.get("test"), "param")

    def test_sync_post_with_json(self):
        """Test synchronous POST request with JSON data."""
        json_data = {"message": "hello", "number": 42}
        response = requestx.post("https://httpbin.org/post", json=json_data)
        self.assertEqual(response.status_code, 200)

        # Verify JSON was sent
        response_data = response.json()
        sent_json = response_data.get("json", {})
        self.assertEqual(sent_json.get("message"), "hello")
        self.assertEqual(sent_json.get("number"), 42)

    def test_sync_post_with_form_data(self):
        """Test synchronous POST request with form data."""
        form_data = {"field1": "value1", "field2": "value2"}
        response = requestx.post("https://httpbin.org/post", data=form_data)
        self.assertEqual(response.status_code, 200)

        # Verify form data was sent
        response_data = response.json()
        sent_form = response_data.get("form", {})
        self.assertEqual(sent_form.get("field1"), "value1")
        self.assertEqual(sent_form.get("field2"), "value2")

    def test_sync_post_with_text_data(self):
        """Test synchronous POST request with text data."""
        text_data = "This is plain text"
        response = requestx.post("https://httpbin.org/post", data=text_data)
        self.assertEqual(response.status_code, 200)

        # Verify text data was sent
        response_data = response.json()
        sent_data = response_data.get("data", "")
        self.assertEqual(sent_data, text_data)

    def test_sync_timeout(self):
        """Test synchronous request with timeout."""
        # This should succeed with a reasonable timeout
        response = requestx.get("https://httpbin.org/get", timeout=10.0)
        self.assertEqual(response.status_code, 200)

    def test_sync_context_detection(self):
        """Test that sync context is properly detected."""
        response = requestx.get("https://httpbin.org/get")
        # Response should be immediately available (not a coroutine)
        self.assertFalse(asyncio.iscoroutine(response))
        self.assertEqual(response.status_code, 200)


class TestAsyncBindings(unittest.TestCase):
    """Test async PyO3 bindings functionality."""

    def setUp(self):
        """Set up async event loop."""
        self.loop = asyncio.new_event_loop()
        asyncio.set_event_loop(self.loop)

    def tearDown(self):
        """Clean up event loop."""
        self.loop.close()

    def test_async_get_basic(self):
        """Test basic asynchronous GET request."""

        async def run_test():
            response = await requestx.get("https://httpbin.org/get")
            self.assertEqual(response.status_code, 200)

        self.loop.run_until_complete(run_test())

    def test_async_get_with_headers(self):
        """Test asynchronous GET request with headers."""

        async def run_test():
            headers = {"User-Agent": "RequestX-Async-Test"}
            response = await requestx.get("https://httpbin.org/get", headers=headers)
            self.assertEqual(response.status_code, 200)

            # Verify header was sent
            response_data = response.json()
            sent_headers = response_data.get("headers", {})
            self.assertEqual(sent_headers.get("User-Agent"), "RequestX-Async-Test")

        self.loop.run_until_complete(run_test())

    def test_async_post_with_json(self):
        """Test asynchronous POST request with JSON data."""

        async def run_test():
            json_data = {"async": True, "test": "data"}
            response = await requestx.post("https://httpbin.org/post", json=json_data)
            self.assertEqual(response.status_code, 200)

            # Verify JSON was sent
            response_data = response.json()
            sent_json = response_data.get("json", {})
            self.assertEqual(sent_json.get("async"), True)
            self.assertEqual(sent_json.get("test"), "data")

        self.loop.run_until_complete(run_test())

    def test_async_context_detection(self):
        """Test that async context is properly detected."""

        async def run_test():
            # In async context, should return a coroutine or future
            result = requestx.get("https://httpbin.org/get")
            self.assertTrue(asyncio.iscoroutine(result) or asyncio.isfuture(result))

            # Await the result to get the response
            response = await result
            self.assertEqual(response.status_code, 200)

        self.loop.run_until_complete(run_test())

    def test_async_concurrent_requests(self):
        """Test multiple concurrent async requests."""

        async def run_test():
            urls = [
                "https://httpbin.org/get",
                "https://httpbin.org/user-agent",
                "https://httpbin.org/headers",
            ]

            # Create coroutines
            tasks = [requestx.get(url) for url in urls]

            # Execute concurrently
            responses = await asyncio.gather(*tasks)

            # All should be successful
            for response in responses:
                self.assertEqual(response.status_code, 200)

        self.loop.run_until_complete(run_test())


class TestErrorHandling(unittest.TestCase):
    """Test error handling in Python-Rust bindings."""

    def test_invalid_url_error(self):
        """Test invalid URL error handling."""
        with self.assertRaises(Exception):  # Should raise some kind of error
            requestx.get("not-a-valid-url")

    def test_invalid_method_error(self):
        """Test invalid HTTP method error handling."""
        with self.assertRaises(RuntimeError):
            requestx.request("INVALID", "https://httpbin.org/get")

    def test_timeout_error(self):
        """Test timeout error handling."""
        with self.assertRaises(Exception):  # Should raise timeout error
            requestx.get("https://httpbin.org/delay/10", timeout=1)


if __name__ == "__main__":
    # Run all tests
    unittest.main(verbosity=2)
