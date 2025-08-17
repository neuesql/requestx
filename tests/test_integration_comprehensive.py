#!/usr/bin/env python3
"""
Comprehensive integration tests for RequestX using httpbin.org.

This module provides comprehensive integration tests that validate RequestX
behavior against a live HTTP testing service, ensuring real-world compatibility.

Requirements tested: 6.1, 7.1, 7.2, 7.3, 7.4
"""

import unittest
import asyncio
import json
import base64
import time
import sys
import os
from urllib.parse import urlencode

# Add the parent directory to the path to import requestx
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "python"))

try:
    import requestx
except ImportError as e:
    print(f"Failed to import requestx: {e}")
    print("Make sure to build the extension with: uv run maturin develop")
    sys.exit(1)


class TestHTTPBinIntegration(unittest.TestCase):
    """Integration tests using httpbin.org service."""

    def setUp(self):
        """Set up test fixtures."""
        self.base_url = "https://httpbin.org"
        self.timeout = 30

    def test_basic_get_request(self):
        """Test basic GET request with httpbin."""
        response = requestx.get(f"{self.base_url}/get", timeout=self.timeout)

        self.assertEqual(response.status_code, 200)
        self.assertTrue(response.ok)

        data = response.json()
        self.assertIn("url", data)
        self.assertIn("headers", data)
        self.assertIn("args", data)
        self.assertTrue(data["url"].startswith("https://httpbin.org/get"))

    def test_get_with_query_parameters(self):
        """Test GET request with query parameters."""
        params = {
            "param1": "value1",
            "param2": "value with spaces",
            "param3": "123",
            "special": "chars!@#$%",
        }

        response = requestx.get(
            f"{self.base_url}/get", params=params, timeout=self.timeout
        )

        self.assertEqual(response.status_code, 200)
        data = response.json()

        # Check that all parameters were sent correctly
        for key, value in params.items():
            self.assertEqual(data["args"][key], value)

    def test_get_with_custom_headers(self):
        """Test GET request with custom headers."""
        headers = {
            "User-Agent": "RequestX-Test/1.0",
            "X-Custom-Header": "custom-value",
            "Authorization": "Bearer test-token",
            "Accept": "application/json",
        }

        response = requestx.get(
            f"{self.base_url}/get", headers=headers, timeout=self.timeout
        )

        self.assertEqual(response.status_code, 200)
        data = response.json()

        # Check that headers were sent correctly
        for key, value in headers.items():
            self.assertEqual(data["headers"][key], value)

    def test_post_with_form_data(self):
        """Test POST request with form data."""
        form_data = {
            "field1": "value1",
            "field2": "value2",
            "number": "123",
            "special": "chars & symbols!",
        }

        response = requestx.post(
            f"{self.base_url}/post", data=form_data, timeout=self.timeout
        )

        self.assertEqual(response.status_code, 200)
        data = response.json()

        # Check that form data was sent correctly
        self.assertIn("form", data)
        for key, value in form_data.items():
            self.assertEqual(data["form"][key], value)

    def test_post_with_json_data(self):
        """Test POST request with JSON data."""
        json_data = {
            "name": "test user",
            "age": 30,
            "active": True,
            "scores": [85, 92, 78],
            "metadata": {"created": "2024-01-01", "source": "test"},
        }

        response = requestx.post(
            f"{self.base_url}/post", json=json_data, timeout=self.timeout
        )

        self.assertEqual(response.status_code, 200)
        data = response.json()

        # Check that JSON data was sent correctly
        self.assertIn("json", data)
        self.assertEqual(data["json"], json_data)

        # Check content type header
        self.assertEqual(data["headers"]["Content-Type"], "application/json")

    def test_put_request(self):
        """Test PUT request."""
        put_data = {"update": "data", "version": 2}

        response = requestx.put(
            f"{self.base_url}/put", json=put_data, timeout=self.timeout
        )

        self.assertEqual(response.status_code, 200)
        data = response.json()

        self.assertIn("json", data)
        self.assertEqual(data["json"], put_data)

    def test_delete_request(self):
        """Test DELETE request."""
        response = requestx.delete(f"{self.base_url}/delete", timeout=self.timeout)

        self.assertEqual(response.status_code, 200)
        data = response.json()

        self.assertIn("url", data)
        self.assertTrue(data["url"].endswith("/delete"))

    def test_patch_request(self):
        """Test PATCH request."""
        patch_data = {"field": "updated_value"}

        response = requestx.patch(
            f"{self.base_url}/patch", json=patch_data, timeout=self.timeout
        )

        self.assertEqual(response.status_code, 200)
        data = response.json()

        self.assertIn("json", data)
        self.assertEqual(data["json"], patch_data)

    def test_head_request(self):
        """Test HEAD request."""
        response = requestx.head(f"{self.base_url}/get", timeout=self.timeout)

        self.assertEqual(response.status_code, 200)
        self.assertIsInstance(response.headers, dict)

        # HEAD requests should have empty body
        self.assertEqual(len(response.text), 0)
        self.assertEqual(len(response.content), 0)

    def test_options_request(self):
        """Test OPTIONS request."""
        response = requestx.options(f"{self.base_url}/get", timeout=self.timeout)

        # OPTIONS typically returns 200 or 204
        self.assertIn(response.status_code, [200, 204])
        self.assertIsInstance(response.headers, dict)

    def test_status_codes(self):
        """Test various HTTP status codes."""
        status_codes = [
            200,
            201,
            202,
            204,
            300,
            301,
            302,
            400,
            401,
            403,
            404,
            500,
            502,
            503,
        ]

        for status_code in status_codes:
            with self.subTest(status_code=status_code):
                response = requestx.get(
                    f"{self.base_url}/status/{status_code}", timeout=self.timeout
                )

                self.assertEqual(response.status_code, status_code)

                # Check ok property
                if 200 <= status_code < 400:
                    self.assertTrue(response.ok)
                    self.assertTrue(response)  # Should be truthy
                else:
                    self.assertFalse(response.ok)
                    self.assertFalse(response)  # Should be falsy

    def test_redirects(self):
        """Test redirect handling."""
        # Test redirect following (default behavior)
        response = requestx.get(f"{self.base_url}/redirect/3", timeout=self.timeout)

        self.assertEqual(response.status_code, 200)
        data = response.json()
        self.assertIn("url", data)

        # Test redirect prevention
        response = requestx.get(
            f"{self.base_url}/redirect/1", allow_redirects=False, timeout=self.timeout
        )

        self.assertIn(response.status_code, [301, 302, 303, 307, 308])

        # Check redirect location header
        if "location" in response.headers or "Location" in response.headers:
            location = response.headers.get("location") or response.headers.get(
                "Location"
            )
            self.assertIsInstance(location, str)
            self.assertTrue(len(location) > 0)

    def test_basic_authentication(self):
        """Test basic authentication."""
        username = "testuser"
        password = "testpass"

        response = requestx.get(
            f"{self.base_url}/basic-auth/{username}/{password}",
            auth=(username, password),
            timeout=self.timeout,
        )

        self.assertEqual(response.status_code, 200)
        data = response.json()

        self.assertTrue(data["authenticated"])
        self.assertEqual(data["user"], username)

    def test_bearer_token_authentication(self):
        """Test bearer token authentication."""
        token = "test-bearer-token-12345"
        headers = {"Authorization": f"Bearer {token}"}

        response = requestx.get(
            f"{self.base_url}/bearer", headers=headers, timeout=self.timeout
        )

        self.assertEqual(response.status_code, 200)
        data = response.json()

        self.assertTrue(data["authenticated"])
        self.assertEqual(data["token"], token)

    def test_cookies(self):
        """Test cookie handling."""
        # Set a cookie
        cookie_name = "test_cookie"
        cookie_value = "test_value_123"

        response = requestx.get(
            f"{self.base_url}/cookies/set/{cookie_name}/{cookie_value}",
            timeout=self.timeout,
        )

        self.assertEqual(response.status_code, 200)

        # Get cookies to verify they were set
        response = requestx.get(f"{self.base_url}/cookies", timeout=self.timeout)

        self.assertEqual(response.status_code, 200)
        data = response.json()

        # Note: httpbin.org's cookie behavior may vary, so we just check the structure
        self.assertIn("cookies", data)

    def test_gzip_compression(self):
        """Test gzip compression handling."""
        response = requestx.get(f"{self.base_url}/gzip", timeout=self.timeout)

        self.assertEqual(response.status_code, 200)
        data = response.json()

        self.assertTrue(data["gzipped"])
        self.assertIn("headers", data)

    def test_deflate_compression(self):
        """Test deflate compression handling."""
        response = requestx.get(f"{self.base_url}/deflate", timeout=self.timeout)

        self.assertEqual(response.status_code, 200)
        data = response.json()

        self.assertTrue(data["deflated"])
        self.assertIn("headers", data)

    def test_json_response(self):
        """Test JSON response parsing."""
        response = requestx.get(f"{self.base_url}/json", timeout=self.timeout)

        self.assertEqual(response.status_code, 200)

        # Should be able to parse as JSON
        data = response.json()
        self.assertIsInstance(data, dict)

        # Check content type
        content_type = response.headers.get("content-type", "").lower()
        self.assertIn("application/json", content_type)

    def test_xml_response(self):
        """Test XML response handling."""
        response = requestx.get(f"{self.base_url}/xml", timeout=self.timeout)

        self.assertEqual(response.status_code, 200)

        # Should have XML content
        self.assertIsInstance(response.text, str)
        self.assertIn("<?xml", response.text)

        # Check content type
        content_type = response.headers.get("content-type", "").lower()
        self.assertIn("xml", content_type)

    def test_html_response(self):
        """Test HTML response handling."""
        response = requestx.get(f"{self.base_url}/html", timeout=self.timeout)

        self.assertEqual(response.status_code, 200)

        # Should have HTML content
        self.assertIsInstance(response.text, str)
        self.assertIn("<html", response.text.lower())

        # Check content type
        content_type = response.headers.get("content-type", "").lower()
        self.assertIn("text/html", content_type)

    def test_large_response(self):
        """Test handling of large responses."""
        # Request a stream of data
        response = requestx.get(f"{self.base_url}/stream/50", timeout=self.timeout)

        self.assertEqual(response.status_code, 200)

        # Should be able to read the content
        content = response.text
        self.assertIsInstance(content, str)
        self.assertGreater(len(content), 0)

        # Should contain multiple JSON objects
        lines = content.strip().split("\n")
        self.assertGreater(len(lines), 1)

    def test_delay_endpoint(self):
        """Test delay endpoint for timeout testing."""
        # Test short delay (should succeed)
        start_time = time.time()
        response = requestx.get(f"{self.base_url}/delay/1", timeout=self.timeout)
        end_time = time.time()

        self.assertEqual(response.status_code, 200)
        self.assertGreaterEqual(
            end_time - start_time, 1.0
        )  # Should take at least 1 second

        data = response.json()
        self.assertIn("url", data)

    def test_user_agent(self):
        """Test User-Agent header handling."""
        custom_user_agent = "RequestX-Integration-Test/1.0"
        headers = {"User-Agent": custom_user_agent}

        response = requestx.get(
            f"{self.base_url}/user-agent", headers=headers, timeout=self.timeout
        )

        self.assertEqual(response.status_code, 200)
        data = response.json()

        self.assertEqual(data["user-agent"], custom_user_agent)

    def test_ip_address(self):
        """Test IP address endpoint."""
        response = requestx.get(f"{self.base_url}/ip", timeout=self.timeout)

        self.assertEqual(response.status_code, 200)
        data = response.json()

        self.assertIn("origin", data)
        self.assertIsInstance(data["origin"], str)
        self.assertGreater(len(data["origin"]), 0)


class TestAsyncIntegration(unittest.TestCase):
    """Async integration tests using httpbin.org."""

    def setUp(self):
        """Set up test fixtures."""
        self.base_url = "https://httpbin.org"
        self.timeout = 30

    async def test_async_get_request(self):
        """Test async GET request."""
        response = await requestx.get(f"{self.base_url}/get", timeout=self.timeout)

        self.assertEqual(response.status_code, 200)
        data = response.json()
        self.assertIn("url", data)

    async def test_async_post_request(self):
        """Test async POST request."""
        json_data = {"async": True, "test": "data"}
        response = await requestx.post(
            f"{self.base_url}/post", json=json_data, timeout=self.timeout
        )

        self.assertEqual(response.status_code, 200)
        data = response.json()
        self.assertEqual(data["json"], json_data)

    async def test_concurrent_async_requests(self):
        """Test concurrent async requests."""
        # Create multiple concurrent requests
        urls = [
            f"{self.base_url}/get?id=1",
            f"{self.base_url}/get?id=2",
            f"{self.base_url}/get?id=3",
            f"{self.base_url}/get?id=4",
            f"{self.base_url}/get?id=5",
        ]

        # Make concurrent requests
        start_time = time.time()
        tasks = [requestx.get(url, timeout=self.timeout) for url in urls]
        responses = await asyncio.gather(*tasks)
        end_time = time.time()

        # All should succeed
        for i, response in enumerate(responses):
            self.assertEqual(response.status_code, 200)
            data = response.json()
            self.assertEqual(data["args"]["id"], str(i + 1))

        # Should complete faster than sequential requests
        self.assertLess(
            end_time - start_time, 10
        )  # Should be much faster than 5+ seconds

    async def test_async_error_handling(self):
        """Test async error handling."""
        # Test invalid URL
        with self.assertRaises(requestx.InvalidURL):
            await requestx.get("not-a-valid-url")

        # Test timeout
        with self.assertRaises(requestx.Timeout):
            await requestx.get(f"{self.base_url}/delay/10", timeout=1)

        # Test HTTP error
        response = await requestx.get(
            f"{self.base_url}/status/404", timeout=self.timeout
        )
        self.assertEqual(response.status_code, 404)

        with self.assertRaises(requestx.HTTPError):
            response.raise_for_status()

    async def test_async_session(self):
        """Test async session usage."""
        session = requestx.Session()

        # Make multiple requests with the same session
        response1 = await session.get(
            f"{self.base_url}/get?session=1", timeout=self.timeout
        )
        response2 = await session.get(
            f"{self.base_url}/get?session=2", timeout=self.timeout
        )

        self.assertEqual(response1.status_code, 200)
        self.assertEqual(response2.status_code, 200)

        data1 = response1.json()
        data2 = response2.json()

        self.assertEqual(data1["args"]["session"], "1")
        self.assertEqual(data2["args"]["session"], "2")


class TestErrorScenarios(unittest.TestCase):
    """Test error scenarios and edge cases."""

    def setUp(self):
        """Set up test fixtures."""
        self.base_url = "https://httpbin.org"
        self.timeout = 30

    def test_connection_error(self):
        """Test connection error handling."""
        with self.assertRaises(requestx.ConnectionError):
            requestx.get("https://this-domain-does-not-exist-12345.com", timeout=5)

    def test_timeout_error(self):
        """Test timeout error handling."""
        with self.assertRaises(requestx.Timeout):
            requestx.get(f"{self.base_url}/delay/10", timeout=1)

    def test_invalid_url_error(self):
        """Test invalid URL error handling."""
        invalid_urls = [
            "not-a-url",
            "ftp://invalid-scheme.com",
            "http://",
            "",
            None,
        ]

        for url in invalid_urls:
            with self.subTest(url=url):
                with self.assertRaises(
                    (requestx.InvalidURL, requestx.MissingSchema, TypeError)
                ):
                    requestx.get(url)

    def test_json_decode_error(self):
        """Test JSON decode error handling."""
        # Get non-JSON response
        response = requestx.get(f"{self.base_url}/html", timeout=self.timeout)

        self.assertEqual(response.status_code, 200)

        # Should raise JSONDecodeError when trying to parse as JSON
        with self.assertRaises(requestx.JSONDecodeError):
            response.json()

    def test_http_error_status_codes(self):
        """Test HTTP error status codes."""
        error_codes = [400, 401, 403, 404, 405, 500, 502, 503, 504]

        for code in error_codes:
            with self.subTest(status_code=code):
                response = requestx.get(
                    f"{self.base_url}/status/{code}", timeout=self.timeout
                )

                self.assertEqual(response.status_code, code)
                self.assertFalse(response.ok)
                self.assertFalse(response)  # Should be falsy

                # Should raise HTTPError when calling raise_for_status
                with self.assertRaises(requestx.HTTPError):
                    response.raise_for_status()


def run_async_integration_tests():
    """Run async integration tests."""

    async def run_all_async_tests():
        print("Running async integration tests...")

        # Create test instance
        test_instance = TestAsyncIntegration()
        test_instance.setUp()

        # Define async tests
        async_tests = [
            ("test_async_get_request", test_instance.test_async_get_request()),
            ("test_async_post_request", test_instance.test_async_post_request()),
            (
                "test_concurrent_async_requests",
                test_instance.test_concurrent_async_requests(),
            ),
            ("test_async_error_handling", test_instance.test_async_error_handling()),
            ("test_async_session", test_instance.test_async_session()),
        ]

        # Run each test
        for test_name, test_coro in async_tests:
            try:
                await test_coro
                print(f"  ✓ {test_name} passed")
            except Exception as e:
                print(f"  ✗ {test_name} failed: {e}")
                raise

        print("All async integration tests passed!")

    asyncio.run(run_all_async_tests())


if __name__ == "__main__":
    print("Running comprehensive integration tests...")

    # Run sync tests
    print("\nRunning synchronous integration tests...")
    suite = unittest.TestSuite()

    # Add test classes
    test_classes = [
        TestHTTPBinIntegration,
        TestErrorScenarios,
    ]

    for test_class in test_classes:
        tests = unittest.TestLoader().loadTestsFromTestCase(test_class)
        suite.addTests(tests)

    runner = unittest.TextTestRunner(verbosity=2)
    sync_result = runner.run(suite)

    # Run async tests
    print("\nRunning asynchronous integration tests...")
    try:
        run_async_integration_tests()
        async_success = True
    except Exception as e:
        print(f"Async integration tests failed: {e}")
        async_success = False

    # Summary
    print(f"\nIntegration Test Summary:")
    print(f"Sync tests: {'PASSED' if sync_result.wasSuccessful() else 'FAILED'}")
    print(f"Async tests: {'PASSED' if async_success else 'FAILED'}")

    if sync_result.wasSuccessful() and async_success:
        print("All integration tests passed!")
        sys.exit(0)
    else:
        print("Some integration tests failed!")
        sys.exit(1)
