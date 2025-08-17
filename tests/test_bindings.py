"""
Integration tests for Python-Rust binding functionality.

Tests the PyO3 bindings with native async/await support, parameter conversion,
and sync/async context detection.
"""

import asyncio
import json
import unittest
from unittest.mock import patch
import sys
import os

# Add the project root to the path so we can import requestx
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "python"))

import requestx


class TestPyO3Bindings(unittest.TestCase):
    """Test PyO3 bindings with sync/async context detection."""

    def setUp(self):
        """Set up test fixtures."""
        self.test_url = "https://httpbin.org/get"
        self.test_post_url = "https://httpbin.org/post"

    def test_sync_get_request(self):
        """Test synchronous GET request."""
        response = requestx.get(self.test_url)

        self.assertEqual(response.status_code, 200)
        self.assertIsInstance(response.text, str)
        self.assertIsInstance(response.json(), dict)
        self.assertTrue(response.url.startswith("https://httpbin.org"))

    def test_sync_post_request(self):
        """Test synchronous POST request."""
        response = requestx.post(self.test_post_url)

        self.assertEqual(response.status_code, 200)
        self.assertIsInstance(response.text, str)

    def test_sync_put_request(self):
        """Test synchronous PUT request."""
        response = requestx.put("https://httpbin.org/put")

        self.assertEqual(response.status_code, 200)

    def test_sync_delete_request(self):
        """Test synchronous DELETE request."""
        response = requestx.delete("https://httpbin.org/delete")

        self.assertEqual(response.status_code, 200)

    def test_sync_head_request(self):
        """Test synchronous HEAD request."""
        response = requestx.head(self.test_url)

        self.assertEqual(response.status_code, 200)
        # HEAD requests should have empty body
        self.assertEqual(len(response.content), 0)

    def test_sync_options_request(self):
        """Test synchronous OPTIONS request."""
        response = requestx.options(self.test_url)

        # OPTIONS requests typically return 200 or 204
        self.assertIn(response.status_code, [200, 204])

    def test_sync_patch_request(self):
        """Test synchronous PATCH request."""
        response = requestx.patch("https://httpbin.org/patch")

        self.assertEqual(response.status_code, 200)

    def test_sync_generic_request(self):
        """Test synchronous generic request method."""
        response = requestx.request("GET", self.test_url)

        self.assertEqual(response.status_code, 200)

    async def test_async_get_request(self):
        """Test asynchronous GET request."""
        response = await requestx.get(self.test_url)

        self.assertEqual(response.status_code, 200)
        self.assertIsInstance(response.text, str)
        self.assertIsInstance(response.json(), dict)

    async def test_async_post_request(self):
        """Test asynchronous POST request."""
        response = await requestx.post(self.test_post_url)

        self.assertEqual(response.status_code, 200)

    async def test_async_put_request(self):
        """Test asynchronous PUT request."""
        response = await requestx.put("https://httpbin.org/put")

        self.assertEqual(response.status_code, 200)

    async def test_async_delete_request(self):
        """Test asynchronous DELETE request."""
        response = await requestx.delete("https://httpbin.org/delete")

        self.assertEqual(response.status_code, 200)

    async def test_async_head_request(self):
        """Test asynchronous HEAD request."""
        response = await requestx.head(self.test_url)

        self.assertEqual(response.status_code, 200)
        # HEAD requests should have empty body
        self.assertEqual(len(response.content), 0)

    async def test_async_options_request(self):
        """Test asynchronous OPTIONS request."""
        response = await requestx.options(self.test_url)

        # OPTIONS requests typically return 200 or 204
        self.assertIn(response.status_code, [200, 204])

    async def test_async_patch_request(self):
        """Test asynchronous PATCH request."""
        response = await requestx.patch("https://httpbin.org/patch")

        self.assertEqual(response.status_code, 200)

    async def test_async_generic_request(self):
        """Test asynchronous generic request method."""
        response = await requestx.request("GET", self.test_url)

        self.assertEqual(response.status_code, 200)


class TestParameterConversion(unittest.TestCase):
    """Test parameter conversion from Python kwargs to Rust RequestConfig."""

    def setUp(self):
        """Set up test fixtures."""
        self.test_url = "https://httpbin.org/get"
        self.test_post_url = "https://httpbin.org/post"

    def test_headers_parameter(self):
        """Test headers parameter conversion."""
        headers = {
            "User-Agent": "RequestX/0.2.0",
            "Accept": "application/json",
            "Custom-Header": "test-value",
        }

        response = requestx.get(self.test_url, headers=headers)

        self.assertEqual(response.status_code, 200)
        # Verify headers were sent by checking the response
        response_data = response.json()
        sent_headers = response_data.get("headers", {})
        self.assertEqual(sent_headers.get("User-Agent"), "RequestX/0.2.0")
        self.assertEqual(sent_headers.get("Accept"), "application/json")
        self.assertEqual(sent_headers.get("Custom-Header"), "test-value")

    def test_params_parameter(self):
        """Test query parameters conversion."""
        params = {"key1": "value1", "key2": "value2", "special": "hello world"}

        response = requestx.get(self.test_url, params=params)

        self.assertEqual(response.status_code, 200)
        # Verify params were sent by checking the response
        response_data = response.json()
        sent_args = response_data.get("args", {})
        self.assertEqual(sent_args.get("key1"), "value1")
        self.assertEqual(sent_args.get("key2"), "value2")
        self.assertEqual(sent_args.get("special"), "hello world")

    def test_json_parameter(self):
        """Test JSON data parameter conversion."""
        json_data = {
            "name": "test",
            "value": 42,
            "nested": {"key": "value"},
            "array": [1, 2, 3],
        }

        response = requestx.post(self.test_post_url, json=json_data)

        self.assertEqual(response.status_code, 200)
        # Verify JSON was sent correctly
        response_data = response.json()
        sent_json = response_data.get("json", {})
        self.assertEqual(sent_json.get("name"), "test")
        self.assertEqual(sent_json.get("value"), 42)
        self.assertEqual(sent_json.get("nested"), {"key": "value"})
        self.assertEqual(sent_json.get("array"), [1, 2, 3])

    def test_form_data_parameter(self):
        """Test form data parameter conversion."""
        form_data = {
            "field1": "value1",
            "field2": "value2",
            "special_chars": "hello@world.com",
        }

        response = requestx.post(self.test_post_url, data=form_data)

        self.assertEqual(response.status_code, 200)
        # Verify form data was sent correctly
        response_data = response.json()
        sent_form = response_data.get("form", {})
        self.assertEqual(sent_form.get("field1"), "value1")
        self.assertEqual(sent_form.get("field2"), "value2")
        self.assertEqual(sent_form.get("special_chars"), "hello@world.com")

    def test_text_data_parameter(self):
        """Test text data parameter conversion."""
        text_data = "This is plain text data"

        response = requestx.post(self.test_post_url, data=text_data)

        self.assertEqual(response.status_code, 200)
        # Verify text data was sent correctly
        response_data = response.json()
        sent_data = response_data.get("data", "")
        self.assertEqual(sent_data, text_data)

    def test_bytes_data_parameter(self):
        """Test bytes data parameter conversion."""
        bytes_data = b"This is binary data"

        response = requestx.post(self.test_post_url, data=bytes_data)

        self.assertEqual(response.status_code, 200)
        # Verify bytes data was sent correctly
        response_data = response.json()
        sent_data = response_data.get("data", "")
        self.assertEqual(sent_data, bytes_data.decode("utf-8"))

    def test_timeout_parameter(self):
        """Test timeout parameter conversion."""
        # Test with float timeout
        response = requestx.get(self.test_url, timeout=5.0)
        self.assertEqual(response.status_code, 200)

        # Test with int timeout
        response = requestx.get(self.test_url, timeout=5)
        self.assertEqual(response.status_code, 200)

    def test_allow_redirects_parameter(self):
        """Test allow_redirects parameter conversion."""
        # Test with True
        response = requestx.get("https://httpbin.org/redirect/1", allow_redirects=True)
        self.assertEqual(response.status_code, 200)

        # Test with False (should get redirect status)
        response = requestx.get("https://httpbin.org/redirect/1", allow_redirects=False)
        self.assertIn(response.status_code, [301, 302, 303, 307, 308])

    def test_verify_parameter(self):
        """Test verify parameter conversion."""
        # Test with True (default)
        response = requestx.get(self.test_url, verify=True)
        self.assertEqual(response.status_code, 200)

        # Test with False
        response = requestx.get(self.test_url, verify=False)
        self.assertEqual(response.status_code, 200)

    def test_combined_parameters(self):
        """Test multiple parameters combined."""
        headers = {"User-Agent": "RequestX-Test"}
        params = {"test": "combined"}
        json_data = {"message": "hello"}

        response = requestx.post(
            self.test_post_url,
            headers=headers,
            params=params,
            json=json_data,
            timeout=10.0,
            allow_redirects=True,
            verify=True,
        )

        self.assertEqual(response.status_code, 200)
        response_data = response.json()

        # Verify all parameters were processed
        self.assertEqual(response_data["headers"]["User-Agent"], "RequestX-Test")
        self.assertEqual(response_data["args"]["test"], "combined")
        self.assertEqual(response_data["json"]["message"], "hello")


class TestAsyncSyncContextDetection(unittest.TestCase):
    """Test async/sync context detection functionality."""

    def setUp(self):
        """Set up test fixtures."""
        self.test_url = "https://httpbin.org/get"

    def test_sync_context_detection(self):
        """Test that sync context is properly detected."""
        # This should execute synchronously
        response = requestx.get(self.test_url)

        self.assertEqual(response.status_code, 200)
        # Response should be immediately available (not a coroutine)
        self.assertFalse(asyncio.iscoroutine(response))

    async def test_async_context_detection(self):
        """Test that async context is properly detected."""
        # This should return a coroutine or future in async context
        result = requestx.get(self.test_url)

        # In async context, should return a coroutine or future (awaitable)
        self.assertTrue(asyncio.iscoroutine(result) or asyncio.isfuture(result))

        # Await the result to get the response
        response = await result
        self.assertEqual(response.status_code, 200)

    def test_multiple_sync_requests(self):
        """Test multiple synchronous requests."""
        urls = [
            "https://httpbin.org/get",
            "https://httpbin.org/user-agent",
            "https://httpbin.org/headers",
        ]

        responses = []
        for url in urls:
            response = requestx.get(url)
            responses.append(response)

        # All should be successful
        for response in responses:
            self.assertEqual(response.status_code, 200)

    async def test_multiple_async_requests(self):
        """Test multiple asynchronous requests."""
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


class TestErrorHandling(unittest.TestCase):
    """Test error handling in Python-Rust bindings."""

    def test_invalid_url_error(self):
        """Test invalid URL error handling."""
        with self.assertRaises(ValueError):
            requestx.get("not-a-valid-url")

    def test_invalid_method_error(self):
        """Test invalid HTTP method error handling."""
        with self.assertRaises(RuntimeError):
            requestx.request("INVALID", "https://httpbin.org/get")

    def test_invalid_header_error(self):
        """Test invalid header error handling."""
        with self.assertRaises(ValueError):
            requestx.get(
                "https://httpbin.org/get", headers={"invalid\nheader": "value"}
            )

    def test_invalid_timeout_error(self):
        """Test invalid timeout error handling."""
        with self.assertRaises(ValueError):
            requestx.get("https://httpbin.org/get", timeout=-1)

    def test_network_timeout_error(self):
        """Test network timeout error handling."""
        with self.assertRaises(Exception):  # Should raise timeout error
            requestx.get("https://httpbin.org/delay/10", timeout=1)

    async def test_async_invalid_url_error(self):
        """Test invalid URL error handling in async context."""
        with self.assertRaises(ValueError):
            await requestx.get("not-a-valid-url")

    async def test_async_network_timeout_error(self):
        """Test network timeout error handling in async context."""
        with self.assertRaises(Exception):  # Should raise timeout error
            await requestx.get("https://httpbin.org/delay/10", timeout=1)


def run_async_test(coro):
    """Helper function to run async tests."""
    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    try:
        return loop.run_until_complete(coro)
    finally:
        loop.close()


if __name__ == "__main__":
    # Create a test suite that includes both sync and async tests
    suite = unittest.TestSuite()

    # Add sync tests
    suite.addTest(unittest.makeSuite(TestPyO3Bindings))
    suite.addTest(unittest.makeSuite(TestParameterConversion))
    suite.addTest(unittest.makeSuite(TestErrorHandling))

    # Add async context detection tests (sync parts)
    suite.addTest(TestAsyncSyncContextDetection("test_sync_context_detection"))
    suite.addTest(TestAsyncSyncContextDetection("test_multiple_sync_requests"))

    # Run sync tests
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)

    # Run async tests separately
    print("\n" + "=" * 70)
    print("Running async tests...")
    print("=" * 70)

    # Create test instances and call setUp
    pyo3_test = TestPyO3Bindings()
    pyo3_test.setUp()

    context_test = TestAsyncSyncContextDetection()
    context_test.setUp()

    error_test = TestErrorHandling()
    error_test.setUp()

    async_tests = [
        pyo3_test.test_async_get_request(),
        pyo3_test.test_async_post_request(),
        pyo3_test.test_async_put_request(),
        pyo3_test.test_async_delete_request(),
        pyo3_test.test_async_head_request(),
        pyo3_test.test_async_options_request(),
        pyo3_test.test_async_patch_request(),
        pyo3_test.test_async_generic_request(),
        context_test.test_async_context_detection(),
        context_test.test_multiple_async_requests(),
        error_test.test_async_invalid_url_error(),
        error_test.test_async_network_timeout_error(),
    ]

    async_passed = 0
    async_total = len(async_tests)

    for i, test_coro in enumerate(async_tests):
        test_name = (
            test_coro.__name__ if hasattr(test_coro, "__name__") else f"async_test_{i}"
        )
        try:
            run_async_test(test_coro)
            print(f"✓ {test_name}")
            async_passed += 1
        except Exception as e:
            print(f"✗ {test_name}: {e}")

    print(f"\nAsync tests: {async_passed}/{async_total} passed")

    # Summary
    total_passed = (
        result.testsRun - len(result.failures) - len(result.errors) + async_passed
    )
    total_tests = result.testsRun + async_total

    print(f"\nOverall: {total_passed}/{total_tests} tests passed")

    if result.failures or result.errors or async_passed < async_total:
        sys.exit(1)
