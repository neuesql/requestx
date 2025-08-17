#!/usr/bin/env python3
"""
Comprehensive test suite for RequestX HTTP client library.

This module provides a complete test suite covering all HTTP methods, scenarios,
integration tests, compatibility tests, and both sync/async usage patterns.

Requirements tested: 6.1, 7.1, 7.2, 7.3, 7.4
"""

import unittest
import asyncio
import json
import time
import threading
import sys
import os
from unittest.mock import patch, MagicMock

# Add the parent directory to the path to import requestx
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "python"))

try:
    import requestx
except ImportError as e:
    print(f"Failed to import requestx: {e}")
    print("Make sure to build the extension with: uv run maturin develop")
    sys.exit(1)


class TestHTTPMethods(unittest.TestCase):
    """Test all HTTP methods with various scenarios."""

    def setUp(self):
        """Set up test fixtures."""
        self.base_url = "https://httpbin.org"
        self.timeout = 30  # Generous timeout for CI environments

    def test_get_request_basic(self):
        """Test basic GET request functionality."""
        response = requestx.get(f"{self.base_url}/get", timeout=self.timeout)

        self.assertEqual(response.status_code, 200)
        self.assertIsInstance(response.text, str)
        self.assertIsInstance(response.content, bytes)
        self.assertIsInstance(response.headers, dict)
        self.assertTrue(response.url.startswith("https://"))

        # Test JSON parsing
        data = response.json()
        self.assertIsInstance(data, dict)
        self.assertIn("url", data)

    def test_get_request_with_params(self):
        """Test GET request with query parameters."""
        params = {"param1": "value1", "param2": "value2"}
        response = requestx.get(
            f"{self.base_url}/get", params=params, timeout=self.timeout
        )

        self.assertEqual(response.status_code, 200)
        data = response.json()
        self.assertIn("args", data)
        self.assertEqual(data["args"]["param1"], "value1")
        self.assertEqual(data["args"]["param2"], "value2")

    def test_get_request_with_headers(self):
        """Test GET request with custom headers."""
        headers = {"User-Agent": "RequestX-Test/1.0", "X-Custom-Header": "test-value"}
        response = requestx.get(
            f"{self.base_url}/get", headers=headers, timeout=self.timeout
        )

        self.assertEqual(response.status_code, 200)
        data = response.json()
        self.assertIn("headers", data)
        self.assertEqual(data["headers"]["User-Agent"], "RequestX-Test/1.0")
        self.assertEqual(data["headers"]["X-Custom-Header"], "test-value")

    def test_post_request_basic(self):
        """Test basic POST request functionality."""
        response = requestx.post(f"{self.base_url}/post", timeout=self.timeout)

        self.assertEqual(response.status_code, 200)
        data = response.json()
        self.assertIn("url", data)

    def test_post_request_with_data(self):
        """Test POST request with form data."""
        data = {"key1": "value1", "key2": "value2"}
        response = requestx.post(
            f"{self.base_url}/post", data=data, timeout=self.timeout
        )

        self.assertEqual(response.status_code, 200)
        response_data = response.json()
        self.assertIn("form", response_data)
        self.assertEqual(response_data["form"]["key1"], "value1")
        self.assertEqual(response_data["form"]["key2"], "value2")

    def test_post_request_with_json(self):
        """Test POST request with JSON data."""
        json_data = {"name": "test", "value": 123, "nested": {"key": "value"}}
        response = requestx.post(
            f"{self.base_url}/post", json=json_data, timeout=self.timeout
        )

        self.assertEqual(response.status_code, 200)
        response_data = response.json()
        self.assertIn("json", response_data)
        self.assertEqual(response_data["json"], json_data)

    def test_put_request(self):
        """Test PUT request functionality."""
        data = {"update": "value"}
        response = requestx.put(f"{self.base_url}/put", json=data, timeout=self.timeout)

        self.assertEqual(response.status_code, 200)
        response_data = response.json()
        self.assertIn("json", response_data)
        self.assertEqual(response_data["json"], data)

    def test_delete_request(self):
        """Test DELETE request functionality."""
        response = requestx.delete(f"{self.base_url}/delete", timeout=self.timeout)

        self.assertEqual(response.status_code, 200)
        data = response.json()
        self.assertIn("url", data)

    def test_head_request(self):
        """Test HEAD request functionality."""
        response = requestx.head(f"{self.base_url}/get", timeout=self.timeout)

        self.assertEqual(response.status_code, 200)
        self.assertIsInstance(response.headers, dict)
        # HEAD requests should have empty body
        self.assertEqual(len(response.text), 0)
        self.assertEqual(len(response.content), 0)

    def test_options_request(self):
        """Test OPTIONS request functionality."""
        response = requestx.options(f"{self.base_url}/get", timeout=self.timeout)

        # OPTIONS requests typically return 200 or 204
        self.assertIn(response.status_code, [200, 204])
        self.assertIsInstance(response.headers, dict)

    def test_patch_request(self):
        """Test PATCH request functionality."""
        data = {"patch": "value"}
        response = requestx.patch(
            f"{self.base_url}/patch", json=data, timeout=self.timeout
        )

        self.assertEqual(response.status_code, 200)
        response_data = response.json()
        self.assertIn("json", response_data)
        self.assertEqual(response_data["json"], data)

    def test_generic_request_method(self):
        """Test generic request method with different HTTP methods."""
        methods = ["GET", "POST", "PUT", "DELETE", "HEAD", "OPTIONS", "PATCH"]

        for method in methods:
            with self.subTest(method=method):
                url = f"{self.base_url}/{method.lower()}"
                if method in ["HEAD", "OPTIONS"]:
                    url = f"{self.base_url}/get"

                response = requestx.request(method, url, timeout=self.timeout)

                if method == "HEAD":
                    self.assertEqual(response.status_code, 200)
                    self.assertEqual(len(response.text), 0)
                elif method == "OPTIONS":
                    self.assertIn(response.status_code, [200, 204])
                else:
                    self.assertEqual(response.status_code, 200)
                    if method != "HEAD":
                        data = response.json()
                        self.assertIsInstance(data, dict)


class TestAsyncHTTPMethods(unittest.TestCase):
    """Test all HTTP methods in async context."""

    def setUp(self):
        """Set up test fixtures."""
        self.base_url = "https://httpbin.org"
        self.timeout = 30

    async def test_async_get_request(self):
        """Test async GET request functionality."""
        response = await requestx.get(f"{self.base_url}/get", timeout=self.timeout)

        self.assertEqual(response.status_code, 200)
        self.assertIsInstance(response.text, str)
        self.assertIsInstance(response.content, bytes)
        data = response.json()
        self.assertIsInstance(data, dict)

    async def test_async_post_request(self):
        """Test async POST request functionality."""
        json_data = {"async": True, "test": "value"}
        response = await requestx.post(
            f"{self.base_url}/post", json=json_data, timeout=self.timeout
        )

        self.assertEqual(response.status_code, 200)
        response_data = response.json()
        self.assertEqual(response_data["json"], json_data)

    async def test_async_concurrent_requests(self):
        """Test concurrent async requests."""
        urls = [
            f"{self.base_url}/get?id=1",
            f"{self.base_url}/get?id=2",
            f"{self.base_url}/get?id=3",
        ]

        # Make concurrent requests
        tasks = [requestx.get(url, timeout=self.timeout) for url in urls]
        responses = await asyncio.gather(*tasks)

        # All should succeed
        for i, response in enumerate(responses):
            self.assertEqual(response.status_code, 200)
            data = response.json()
            self.assertEqual(data["args"]["id"], str(i + 1))

    async def test_async_all_methods(self):
        """Test all HTTP methods in async context."""
        methods_and_data = [
            ("GET", f"{self.base_url}/get", None),
            ("POST", f"{self.base_url}/post", {"post": "data"}),
            ("PUT", f"{self.base_url}/put", {"put": "data"}),
            ("DELETE", f"{self.base_url}/delete", None),
            ("HEAD", f"{self.base_url}/get", None),
            ("OPTIONS", f"{self.base_url}/get", None),
            ("PATCH", f"{self.base_url}/patch", {"patch": "data"}),
        ]

        for method, url, data in methods_and_data:
            with self.subTest(method=method):
                if method == "GET":
                    response = await requestx.get(url, timeout=self.timeout)
                elif method == "POST":
                    response = await requestx.post(url, json=data, timeout=self.timeout)
                elif method == "PUT":
                    response = await requestx.put(url, json=data, timeout=self.timeout)
                elif method == "DELETE":
                    response = await requestx.delete(url, timeout=self.timeout)
                elif method == "HEAD":
                    response = await requestx.head(url, timeout=self.timeout)
                elif method == "OPTIONS":
                    response = await requestx.options(url, timeout=self.timeout)
                elif method == "PATCH":
                    response = await requestx.patch(
                        url, json=data, timeout=self.timeout
                    )

                if method == "HEAD":
                    self.assertEqual(response.status_code, 200)
                    self.assertEqual(len(response.text), 0)
                elif method == "OPTIONS":
                    self.assertIn(response.status_code, [200, 204])
                else:
                    self.assertEqual(response.status_code, 200)


class TestResponseObject(unittest.TestCase):
    """Test Response object functionality and requests compatibility."""

    def setUp(self):
        """Set up test fixtures."""
        self.base_url = "https://httpbin.org"
        self.timeout = 30

    def test_response_properties(self):
        """Test Response object properties."""
        response = requestx.get(f"{self.base_url}/get", timeout=self.timeout)

        # Basic properties
        self.assertIsInstance(response.status_code, int)
        self.assertEqual(response.status_code, 200)
        self.assertIsInstance(response.url, str)
        self.assertIsInstance(response.headers, dict)
        self.assertIsInstance(response.text, str)
        self.assertIsInstance(response.content, bytes)

        # Boolean evaluation
        self.assertTrue(response)  # 200 response should be truthy
        self.assertTrue(response.ok)

    def test_response_json_parsing(self):
        """Test JSON response parsing."""
        response = requestx.get(f"{self.base_url}/json", timeout=self.timeout)

        self.assertEqual(response.status_code, 200)
        data = response.json()
        self.assertIsInstance(data, dict)

    def test_response_json_error_handling(self):
        """Test JSON parsing error handling."""
        response = requestx.get(f"{self.base_url}/html", timeout=self.timeout)

        self.assertEqual(response.status_code, 200)
        with self.assertRaises(requestx.JSONDecodeError):
            response.json()

    def test_response_raise_for_status(self):
        """Test raise_for_status method."""
        # Successful response should not raise
        response = requestx.get(f"{self.base_url}/get", timeout=self.timeout)
        response.raise_for_status()  # Should not raise

        # Error response should raise
        error_response = requestx.get(
            f"{self.base_url}/status/404", timeout=self.timeout
        )
        self.assertEqual(error_response.status_code, 404)
        self.assertFalse(error_response.ok)
        self.assertFalse(error_response)  # 404 response should be falsy

        with self.assertRaises(requestx.HTTPError):
            error_response.raise_for_status()

    def test_response_headers_access(self):
        """Test response headers access patterns."""
        headers = {"X-Test-Header": "test-value"}
        response = requestx.get(
            f"{self.base_url}/get", headers=headers, timeout=self.timeout
        )

        self.assertEqual(response.status_code, 200)
        self.assertIsInstance(response.headers, dict)

        # Headers should be accessible
        self.assertGreater(len(response.headers), 0)

        # Should be able to iterate over headers
        header_count = 0
        for key in response.headers:
            self.assertIsInstance(key, str)
            self.assertIsInstance(response.headers[key], str)
            header_count += 1
        self.assertGreater(header_count, 0)

    def test_response_encoding(self):
        """Test response encoding handling."""
        response = requestx.get(f"{self.base_url}/get", timeout=self.timeout)

        # Should have encoding information
        if hasattr(response, "encoding"):
            encoding = response.encoding
            if encoding is not None:
                self.assertIsInstance(encoding, str)

        if hasattr(response, "apparent_encoding"):
            apparent_encoding = response.apparent_encoding
            self.assertIsInstance(apparent_encoding, str)

    def test_response_string_representation(self):
        """Test response string representation."""
        response = requestx.get(f"{self.base_url}/get", timeout=self.timeout)

        response_str = str(response)
        self.assertIn("200", response_str)
        self.assertIn("Response", response_str)

        response_repr = repr(response)
        self.assertIn("Response", response_repr)


class TestSessionManagement(unittest.TestCase):
    """Test Session object functionality."""

    def setUp(self):
        """Set up test fixtures."""
        self.base_url = "https://httpbin.org"
        self.timeout = 30

    def test_session_creation(self):
        """Test Session object creation."""
        session = requestx.Session()
        self.assertIsNotNone(session)
        self.assertTrue(hasattr(session, "get"))
        self.assertTrue(hasattr(session, "post"))

    def test_session_get_request(self):
        """Test Session GET request."""
        session = requestx.Session()
        response = session.get(f"{self.base_url}/get", timeout=self.timeout)

        self.assertEqual(response.status_code, 200)
        data = response.json()
        self.assertIsInstance(data, dict)

    def test_session_post_request(self):
        """Test Session POST request."""
        session = requestx.Session()
        json_data = {"session": "test"}
        response = session.post(
            f"{self.base_url}/post", json=json_data, timeout=self.timeout
        )

        self.assertEqual(response.status_code, 200)
        response_data = response.json()
        self.assertEqual(response_data["json"], json_data)

    def test_session_persistent_headers(self):
        """Test Session persistent headers."""
        session = requestx.Session()

        # Set session headers
        if hasattr(session, "headers"):
            session.headers["X-Session-Header"] = "session-value"

            response = session.get(f"{self.base_url}/get", timeout=self.timeout)
            self.assertEqual(response.status_code, 200)

            data = response.json()
            if "headers" in data:
                self.assertEqual(
                    data["headers"].get("X-Session-Header"), "session-value"
                )

    def test_session_multiple_requests(self):
        """Test multiple requests with same session."""
        session = requestx.Session()

        # Make multiple requests
        for i in range(3):
            response = session.get(
                f"{self.base_url}/get?request={i}", timeout=self.timeout
            )
            self.assertEqual(response.status_code, 200)
            data = response.json()
            self.assertEqual(data["args"]["request"], str(i))


class TestErrorHandling(unittest.TestCase):
    """Test comprehensive error handling."""

    def setUp(self):
        """Set up test fixtures."""
        self.base_url = "https://httpbin.org"
        self.timeout = 30

    def test_invalid_url_error(self):
        """Test invalid URL error handling."""
        with self.assertRaises(requestx.InvalidURL):
            requestx.get("not-a-valid-url")

    def test_connection_error(self):
        """Test connection error handling."""
        with self.assertRaises(requestx.ConnectionError):
            requestx.get("https://this-domain-does-not-exist-12345.com", timeout=5)

    def test_timeout_error(self):
        """Test timeout error handling."""
        with self.assertRaises(requestx.Timeout):
            requestx.get(f"{self.base_url}/delay/10", timeout=1)

    def test_http_error_status_codes(self):
        """Test HTTP error status codes."""
        error_codes = [400, 401, 403, 404, 500, 502, 503]

        for code in error_codes:
            with self.subTest(status_code=code):
                response = requestx.get(
                    f"{self.base_url}/status/{code}", timeout=self.timeout
                )
                self.assertEqual(response.status_code, code)
                self.assertFalse(response.ok)

                with self.assertRaises(requestx.HTTPError):
                    response.raise_for_status()

    def test_json_decode_error(self):
        """Test JSON decode error handling."""
        response = requestx.get(f"{self.base_url}/html", timeout=self.timeout)

        with self.assertRaises(requestx.JSONDecodeError):
            response.json()

    def test_invalid_method_error(self):
        """Test invalid HTTP method error."""
        with self.assertRaises(Exception):
            requestx.request("INVALID_METHOD", f"{self.base_url}/get")

    async def test_async_error_handling(self):
        """Test error handling in async context."""
        # Invalid URL
        with self.assertRaises(requestx.InvalidURL):
            await requestx.get("not-a-valid-url")

        # Connection error
        with self.assertRaises(requestx.ConnectionError):
            await requestx.get(
                "https://this-domain-does-not-exist-12345.com", timeout=5
            )

        # Timeout error
        with self.assertRaises(requestx.Timeout):
            await requestx.get(f"{self.base_url}/delay/10", timeout=1)


class TestRequestsCompatibility(unittest.TestCase):
    """Test compatibility with requests library patterns."""

    def setUp(self):
        """Set up test fixtures."""
        self.base_url = "https://httpbin.org"
        self.timeout = 30

    def test_basic_usage_pattern(self):
        """Test basic requests usage pattern."""
        # This is the most common requests pattern
        response = requestx.get(f"{self.base_url}/get", timeout=self.timeout)

        # Should work exactly like requests
        self.assertEqual(response.status_code, 200)
        self.assertTrue(response.ok)
        self.assertIsInstance(response.text, str)
        self.assertIsInstance(response.content, bytes)

        data = response.json()
        self.assertIsInstance(data, dict)

    def test_post_with_data_pattern(self):
        """Test POST with data pattern from requests."""
        data = {"key": "value", "number": 123}
        response = requestx.post(
            f"{self.base_url}/post", data=data, timeout=self.timeout
        )

        self.assertEqual(response.status_code, 200)
        response_data = response.json()
        self.assertIn("form", response_data)

    def test_post_with_json_pattern(self):
        """Test POST with JSON pattern from requests."""
        json_data = {"name": "test", "value": 123}
        response = requestx.post(
            f"{self.base_url}/post", json=json_data, timeout=self.timeout
        )

        self.assertEqual(response.status_code, 200)
        response_data = response.json()
        self.assertEqual(response_data["json"], json_data)

    def test_headers_pattern(self):
        """Test headers usage pattern from requests."""
        headers = {"User-Agent": "RequestX-Test", "Authorization": "Bearer token"}
        response = requestx.get(
            f"{self.base_url}/get", headers=headers, timeout=self.timeout
        )

        self.assertEqual(response.status_code, 200)
        data = response.json()
        self.assertEqual(data["headers"]["User-Agent"], "RequestX-Test")
        self.assertEqual(data["headers"]["Authorization"], "Bearer token")

    def test_params_pattern(self):
        """Test params usage pattern from requests."""
        params = {"q": "search term", "page": 1, "limit": 10}
        response = requestx.get(
            f"{self.base_url}/get", params=params, timeout=self.timeout
        )

        self.assertEqual(response.status_code, 200)
        data = response.json()
        self.assertEqual(data["args"]["q"], "search term")
        self.assertEqual(data["args"]["page"], "1")
        self.assertEqual(data["args"]["limit"], "10")

    def test_session_pattern(self):
        """Test session usage pattern from requests."""
        session = requestx.Session()

        # Multiple requests with same session
        response1 = session.get(f"{self.base_url}/get", timeout=self.timeout)
        response2 = session.post(
            f"{self.base_url}/post", json={"test": "data"}, timeout=self.timeout
        )

        self.assertEqual(response1.status_code, 200)
        self.assertEqual(response2.status_code, 200)

    def test_error_handling_pattern(self):
        """Test error handling pattern from requests."""
        # Test successful request
        response = requestx.get(f"{self.base_url}/get", timeout=self.timeout)
        response.raise_for_status()  # Should not raise

        # Test error request
        error_response = requestx.get(
            f"{self.base_url}/status/404", timeout=self.timeout
        )
        with self.assertRaises(requestx.HTTPError):
            error_response.raise_for_status()

    def test_boolean_evaluation_pattern(self):
        """Test boolean evaluation pattern from requests."""
        # Successful response should be truthy
        response = requestx.get(f"{self.base_url}/get", timeout=self.timeout)
        if response:
            self.assertTrue(True)  # Should reach here
        else:
            self.fail("Successful response should be truthy")

        # Error response should be falsy
        error_response = requestx.get(
            f"{self.base_url}/status/404", timeout=self.timeout
        )
        if error_response:
            self.fail("Error response should be falsy")
        else:
            self.assertTrue(True)  # Should reach here


class TestIntegrationScenarios(unittest.TestCase):
    """Test integration scenarios using httpbin.org."""

    def setUp(self):
        """Set up test fixtures."""
        self.base_url = "https://httpbin.org"
        self.timeout = 30

    def test_authentication_scenario(self):
        """Test authentication scenario."""
        # Basic auth
        response = requestx.get(
            f"{self.base_url}/basic-auth/user/pass",
            auth=("user", "pass"),
            timeout=self.timeout,
        )
        self.assertEqual(response.status_code, 200)
        data = response.json()
        self.assertTrue(data["authenticated"])

    def test_redirect_scenario(self):
        """Test redirect handling scenario."""
        # Test redirect following
        response = requestx.get(f"{self.base_url}/redirect/3", timeout=self.timeout)
        self.assertEqual(response.status_code, 200)

        # Test redirect prevention
        response = requestx.get(
            f"{self.base_url}/redirect/1", allow_redirects=False, timeout=self.timeout
        )
        self.assertIn(response.status_code, [301, 302, 303, 307, 308])

    def test_cookie_scenario(self):
        """Test cookie handling scenario."""
        # Set cookie
        response = requestx.get(
            f"{self.base_url}/cookies/set/test/value", timeout=self.timeout
        )
        self.assertEqual(response.status_code, 200)

        # Get cookies
        response = requestx.get(f"{self.base_url}/cookies", timeout=self.timeout)
        self.assertEqual(response.status_code, 200)

    def test_compression_scenario(self):
        """Test compression handling scenario."""
        # Test gzip compression
        response = requestx.get(f"{self.base_url}/gzip", timeout=self.timeout)
        self.assertEqual(response.status_code, 200)
        data = response.json()
        self.assertTrue(data["gzipped"])

        # Test deflate compression
        response = requestx.get(f"{self.base_url}/deflate", timeout=self.timeout)
        self.assertEqual(response.status_code, 200)
        data = response.json()
        self.assertTrue(data["deflated"])

    def test_large_response_scenario(self):
        """Test handling of large responses."""
        # Test streaming large response
        response = requestx.get(f"{self.base_url}/stream/100", timeout=self.timeout)
        self.assertEqual(response.status_code, 200)

        # Should be able to read content
        content = response.text
        self.assertGreater(len(content), 0)

    def test_various_content_types(self):
        """Test handling of various content types."""
        content_types = [
            ("/json", "application/json"),
            ("/xml", "application/xml"),
            ("/html", "text/html"),
        ]

        for endpoint, expected_type in content_types:
            with self.subTest(endpoint=endpoint):
                response = requestx.get(
                    f"{self.base_url}{endpoint}", timeout=self.timeout
                )
                self.assertEqual(response.status_code, 200)

                # Check content type in headers
                content_type = response.headers.get("content-type", "").lower()
                self.assertIn(expected_type.split("/")[0], content_type)

    def test_concurrent_requests_scenario(self):
        """Test concurrent requests scenario."""

        async def run_concurrent_test():
            # Create multiple concurrent requests
            urls = [f"{self.base_url}/get?id={i}" for i in range(10)]
            tasks = [requestx.get(url, timeout=self.timeout) for url in urls]

            start_time = time.time()
            responses = await asyncio.gather(*tasks)
            end_time = time.time()

            # All should succeed
            for i, response in enumerate(responses):
                self.assertEqual(response.status_code, 200)
                data = response.json()
                self.assertEqual(data["args"]["id"], str(i))

            # Should complete in reasonable time (concurrent should be faster)
            self.assertLess(end_time - start_time, 30)

        asyncio.run(run_concurrent_test())


def run_async_tests():
    """Run async tests using asyncio."""

    async def run_all_async_tests():
        print("Running async HTTP method tests...")

        # Create test instances
        async_http_test = TestAsyncHTTPMethods()
        async_http_test.setUp()

        error_test = TestErrorHandling()
        error_test.setUp()

        # Run async tests
        async_tests = [
            ("test_async_get_request", async_http_test.test_async_get_request()),
            ("test_async_post_request", async_http_test.test_async_post_request()),
            (
                "test_async_concurrent_requests",
                async_http_test.test_async_concurrent_requests(),
            ),
            ("test_async_all_methods", async_http_test.test_async_all_methods()),
            ("test_async_error_handling", error_test.test_async_error_handling()),
        ]

        for test_name, test_coro in async_tests:
            try:
                await test_coro
                print(f"  ✓ {test_name} passed")
            except Exception as e:
                print(f"  ✗ {test_name} failed: {e}")
                raise

        print("All async tests passed!")

    asyncio.run(run_all_async_tests())


if __name__ == "__main__":
    print("Running comprehensive RequestX test suite...")

    # Create test suite
    suite = unittest.TestSuite()

    # Add all test classes
    test_classes = [
        TestHTTPMethods,
        TestResponseObject,
        TestSessionManagement,
        TestErrorHandling,
        TestRequestsCompatibility,
        TestIntegrationScenarios,
    ]

    for test_class in test_classes:
        tests = unittest.TestLoader().loadTestsFromTestCase(test_class)
        suite.addTests(tests)

    # Run sync tests
    print("\nRunning synchronous tests...")
    runner = unittest.TextTestRunner(verbosity=2)
    sync_result = runner.run(suite)

    # Run async tests
    print("\nRunning asynchronous tests...")
    try:
        run_async_tests()
        async_success = True
    except Exception as e:
        print(f"Async tests failed: {e}")
        async_success = False

    # Summary
    print(f"\nTest Summary:")
    print(f"Sync tests: {'PASSED' if sync_result.wasSuccessful() else 'FAILED'}")
    print(f"Async tests: {'PASSED' if async_success else 'FAILED'}")

    if sync_result.wasSuccessful() and async_success:
        print("All comprehensive tests passed!")
        sys.exit(0)
    else:
        print("Some tests failed!")
        sys.exit(1)
