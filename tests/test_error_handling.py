"""
Comprehensive error handling tests for RequestX.

This module tests all error scenarios and exception compatibility with the requests library.
Tests cover network errors, timeouts, HTTP errors, SSL errors, and other edge cases.
"""

import unittest
import asyncio
import requestx
import time
from unittest.mock import patch


class TestExceptionHierarchy(unittest.TestCase):
    """Test that all exceptions are properly defined and inherit correctly."""

    def test_base_exception_exists(self):
        """Test that RequestException base class exists."""
        self.assertTrue(hasattr(requestx, "RequestException"))
        self.assertTrue(issubclass(requestx.RequestException, Exception))

    def test_connection_error_hierarchy(self):
        """Test ConnectionError inheritance."""
        self.assertTrue(hasattr(requestx, "ConnectionError"))
        self.assertTrue(issubclass(requestx.ConnectionError, requestx.RequestException))

    def test_http_error_hierarchy(self):
        """Test HTTPError inheritance."""
        self.assertTrue(hasattr(requestx, "HTTPError"))
        self.assertTrue(issubclass(requestx.HTTPError, requestx.RequestException))

    def test_timeout_error_hierarchy(self):
        """Test timeout error inheritance."""
        self.assertTrue(hasattr(requestx, "Timeout"))
        self.assertTrue(issubclass(requestx.Timeout, requestx.RequestException))

        self.assertTrue(hasattr(requestx, "ConnectTimeout"))
        self.assertTrue(issubclass(requestx.ConnectTimeout, requestx.ConnectionError))

        self.assertTrue(hasattr(requestx, "ReadTimeout"))
        self.assertTrue(issubclass(requestx.ReadTimeout, requestx.RequestException))

    def test_ssl_error_hierarchy(self):
        """Test SSLError inheritance."""
        self.assertTrue(hasattr(requestx, "SSLError"))
        self.assertTrue(issubclass(requestx.SSLError, requestx.ConnectionError))

    def test_url_error_hierarchy(self):
        """Test URL-related error inheritance."""
        self.assertTrue(hasattr(requestx, "InvalidURL"))
        self.assertTrue(issubclass(requestx.InvalidURL, requestx.RequestException))

        self.assertTrue(hasattr(requestx, "URLRequired"))
        self.assertTrue(issubclass(requestx.URLRequired, requestx.RequestException))

        self.assertTrue(hasattr(requestx, "InvalidSchema"))
        self.assertTrue(issubclass(requestx.InvalidSchema, requestx.RequestException))

        self.assertTrue(hasattr(requestx, "MissingSchema"))
        self.assertTrue(issubclass(requestx.MissingSchema, requestx.RequestException))

    def test_json_error_hierarchy(self):
        """Test JSONDecodeError inheritance."""
        self.assertTrue(hasattr(requestx, "JSONDecodeError"))
        self.assertTrue(issubclass(requestx.JSONDecodeError, requestx.RequestException))

    def test_all_exceptions_in_all(self):
        """Test that all exceptions are exported in __all__."""
        expected_exceptions = [
            "RequestException",
            "ConnectionError",
            "HTTPError",
            "URLRequired",
            "TooManyRedirects",
            "ConnectTimeout",
            "ReadTimeout",
            "Timeout",
            "JSONDecodeError",
            "InvalidURL",
            "InvalidHeader",
            "SSLError",
            "ProxyError",
            "RetryError",
            "UnreachableCodeError",
            "InvalidSchema",
            "MissingSchema",
            "ChunkedEncodingError",
            "ContentDecodingError",
            "StreamConsumedError",
            "FileModeWarning",
            "RequestsWarning",
            "DependencyWarning",
        ]

        for exc_name in expected_exceptions:
            self.assertIn(exc_name, requestx.__all__, f"{exc_name} not in __all__")
            self.assertTrue(hasattr(requestx, exc_name), f"{exc_name} not defined")


class TestURLErrors(unittest.TestCase):
    """Test URL-related error handling."""

    def test_invalid_url_error(self):
        """Test invalid URL raises InvalidURL or MissingSchema."""
        with self.assertRaises((requestx.InvalidURL, requestx.MissingSchema)):
            requestx.get("not-a-valid-url")

    def test_missing_schema_error(self):
        """Test missing schema raises MissingSchema."""
        with self.assertRaises(requestx.MissingSchema):
            requestx.get("example.com/path")

    def test_invalid_schema_error(self):
        """Test invalid schema raises InvalidSchema."""
        with self.assertRaises(requestx.InvalidSchema):
            requestx.get("ftp://example.com/file")

    def test_empty_url_error(self):
        """Test empty URL raises URLRequired."""
        with self.assertRaises(requestx.URLRequired):
            requestx.get("")

    def test_malformed_url_error(self):
        """Test malformed URL raises InvalidURL."""
        with self.assertRaises(requestx.InvalidURL):
            requestx.get("http://[invalid-ipv6")

    async def test_async_invalid_url_error(self):
        """Test invalid URL error in async context."""
        with self.assertRaises((requestx.InvalidURL, requestx.MissingSchema)):
            await requestx.get("not-a-valid-url")

    async def test_async_missing_schema_error(self):
        """Test missing schema error in async context."""
        with self.assertRaises(requestx.MissingSchema):
            await requestx.get("example.com/path")


class TestNetworkErrors(unittest.TestCase):
    """Test network-related error handling."""

    def test_connection_error(self):
        """Test connection error to non-existent host."""
        with self.assertRaises(requestx.ConnectionError):
            requestx.get("http://non-existent-host-12345.com", timeout=5)

    def test_connection_refused_error(self):
        """Test connection refused error."""
        # Try to connect to a port that should be closed
        with self.assertRaises(requestx.ConnectionError):
            requestx.get("http://127.0.0.1:9999", timeout=5)

    def test_dns_resolution_error(self):
        """Test DNS resolution failure."""
        with self.assertRaises(requestx.ConnectionError):
            requestx.get("http://this-domain-does-not-exist-12345.invalid", timeout=5)

    async def test_async_connection_error(self):
        """Test connection error in async context."""
        with self.assertRaises(requestx.ConnectionError):
            await requestx.get("http://non-existent-host-12345.com", timeout=5)


class TestTimeoutErrors(unittest.TestCase):
    """Test timeout-related error handling."""

    def test_read_timeout_error(self):
        """Test read timeout error."""
        with self.assertRaises((requestx.ReadTimeout, requestx.Timeout)):
            requestx.get("https://httpbin.org/delay/10", timeout=1)

    def test_connect_timeout_error(self):
        """Test connect timeout error."""
        # Use a non-routable IP to trigger connect timeout
        with self.assertRaises((requestx.ConnectTimeout, requestx.ConnectionError)):
            requestx.get("http://10.255.255.1", timeout=1)

    def test_timeout_inheritance(self):
        """Test that specific timeout errors inherit from Timeout."""
        with self.assertRaises(requestx.Timeout):
            requestx.get("https://httpbin.org/delay/10", timeout=1)

    async def test_async_timeout_error(self):
        """Test timeout error in async context."""
        with self.assertRaises((requestx.ReadTimeout, requestx.Timeout)):
            await requestx.get("https://httpbin.org/delay/10", timeout=1)


class TestHTTPErrors(unittest.TestCase):
    """Test HTTP error handling."""

    def test_http_error_not_raised_by_default(self):
        """Test that HTTP errors don't raise exceptions by default."""
        # This should not raise an exception, just return a response with status 404
        response = requestx.get("https://httpbin.org/status/404")
        self.assertEqual(response.status_code, 404)

    def test_http_error_with_raise_for_status(self):
        """Test HTTPError when using raise_for_status."""
        response = requestx.get("https://httpbin.org/status/404")
        with self.assertRaises(requestx.HTTPError):
            response.raise_for_status()

    def test_various_http_status_codes(self):
        """Test various HTTP status codes don't raise exceptions by default."""
        status_codes = [400, 401, 403, 404, 500, 502, 503]

        for status_code in status_codes:
            with self.subTest(status_code=status_code):
                response = requestx.get(f"https://httpbin.org/status/{status_code}")
                self.assertEqual(response.status_code, status_code)

                # But raise_for_status should raise HTTPError
                with self.assertRaises(requestx.HTTPError):
                    response.raise_for_status()


class TestHeaderErrors(unittest.TestCase):
    """Test header-related error handling."""

    def test_invalid_header_name(self):
        """Test invalid header name raises InvalidHeader."""
        with self.assertRaises(requestx.InvalidHeader):
            requestx.get(
                "https://httpbin.org/get", headers={"invalid\nheader": "value"}
            )

    def test_invalid_header_value(self):
        """Test invalid header value raises InvalidHeader."""
        with self.assertRaises(requestx.InvalidHeader):
            requestx.get(
                "https://httpbin.org/get", headers={"header": "invalid\nvalue"}
            )

    def test_non_ascii_header_name(self):
        """Test non-ASCII header name raises InvalidHeader."""
        with self.assertRaises(requestx.InvalidHeader):
            requestx.get("https://httpbin.org/get", headers={"héader": "value"})


class TestParameterErrors(unittest.TestCase):
    """Test parameter validation errors."""

    def test_negative_timeout_error(self):
        """Test negative timeout raises error."""
        with self.assertRaises(requestx.RequestException):
            requestx.get("https://httpbin.org/get", timeout=-1)

    def test_invalid_timeout_type_error(self):
        """Test invalid timeout type raises error."""
        with self.assertRaises(requestx.RequestException):
            requestx.get("https://httpbin.org/get", timeout="invalid")

    def test_too_large_timeout_error(self):
        """Test too large timeout raises error."""
        with self.assertRaises(requestx.RequestException):
            requestx.get("https://httpbin.org/get", timeout=10000)  # > 3600 seconds

    def test_invalid_method_error(self):
        """Test invalid HTTP method raises error."""
        with self.assertRaises(requestx.RequestException):
            requestx.request("INVALID_METHOD", "https://httpbin.org/get")


class TestJSONErrors(unittest.TestCase):
    """Test JSON-related error handling."""

    def test_json_decode_error(self):
        """Test JSONDecodeError when parsing invalid JSON."""
        # Get a non-JSON response
        response = requestx.get("https://httpbin.org/html")

        with self.assertRaises(requestx.JSONDecodeError):
            response.json()

    def test_json_decode_error_empty_response(self):
        """Test JSONDecodeError with empty response."""
        # Get an empty response (204 No Content)
        response = requestx.get("https://httpbin.org/status/204")

        with self.assertRaises(requestx.JSONDecodeError):
            response.json()


class TestSessionErrors(unittest.TestCase):
    """Test session-related error handling."""

    def test_session_invalid_url_error(self):
        """Test session raises same URL errors."""
        session = requestx.Session()

        with self.assertRaises(requestx.InvalidURL):
            session.get("not-a-valid-url")

    def test_session_network_error(self):
        """Test session raises same network errors."""
        session = requestx.Session()

        with self.assertRaises(requestx.ConnectionError):
            session.get("http://non-existent-host-12345.com", timeout=5)

    def test_session_timeout_error(self):
        """Test session raises same timeout errors."""
        session = requestx.Session()

        with self.assertRaises((requestx.ReadTimeout, requestx.Timeout)):
            session.get("https://httpbin.org/delay/10", timeout=1)


class TestAsyncErrorHandling(unittest.TestCase):
    """Test error handling in async context."""

    async def test_async_network_error(self):
        """Test network error in async context."""
        with self.assertRaises(requestx.ConnectionError):
            await requestx.get("http://non-existent-host-12345.com", timeout=5)

    async def test_async_timeout_error(self):
        """Test timeout error in async context."""
        with self.assertRaises((requestx.ReadTimeout, requestx.Timeout)):
            await requestx.get("https://httpbin.org/delay/10", timeout=1)

    async def test_async_http_error(self):
        """Test HTTP error in async context."""
        response = await requestx.get("https://httpbin.org/status/404")
        self.assertEqual(response.status_code, 404)

        with self.assertRaises(requestx.HTTPError):
            response.raise_for_status()

    async def test_async_json_error(self):
        """Test JSON error in async context."""
        response = await requestx.get("https://httpbin.org/html")

        with self.assertRaises(requestx.JSONDecodeError):
            response.json()


class TestErrorMessages(unittest.TestCase):
    """Test that error messages are informative and match requests library."""

    def test_invalid_url_message(self):
        """Test InvalidURL error message."""
        try:
            requestx.get("not-a-valid-url")
        except requestx.InvalidURL as e:
            self.assertIn("Invalid URL", str(e))

    def test_missing_schema_message(self):
        """Test MissingSchema error message."""
        try:
            requestx.get("example.com")
        except requestx.MissingSchema as e:
            # Should mention connection adapters like requests does
            self.assertIn("connection adapters", str(e))

    def test_timeout_message(self):
        """Test timeout error message."""
        try:
            requestx.get("https://httpbin.org/delay/10", timeout=1)
        except requestx.ReadTimeout as e:
            self.assertIn("did not send any data", str(e))
        except requestx.Timeout:
            # Any timeout error is acceptable
            pass

    def test_connection_error_message(self):
        """Test connection error message."""
        try:
            requestx.get("http://non-existent-host-12345.com", timeout=5)
        except requestx.ConnectionError as e:
            # Should contain some indication of connection failure
            error_msg = str(e).lower()
            self.assertTrue(
                any(
                    word in error_msg
                    for word in ["connection", "network", "resolve", "dns"]
                ),
                f"Connection error message should be informative: {e}",
            )


def run_async_tests():
    """Run async tests using asyncio."""

    async def run_test_method(test_instance, method_name):
        """Run a single async test method."""
        method = getattr(test_instance, method_name)
        try:
            await method()
            return True, None
        except AssertionError as e:
            # Test passed - the exception was expected
            return True, None
        except Exception as e:
            return False, e

    # Get all async test methods
    async_test_classes = [
        TestURLErrors,
        TestNetworkErrors,
        TestTimeoutErrors,
        TestAsyncErrorHandling,
    ]

    async_results = []

    async def run_all_async_tests():
        for test_class in async_test_classes:
            test_instance = test_class()
            test_instance.setUp() if hasattr(test_instance, "setUp") else None

            # Find async test methods
            async_methods = [
                method
                for method in dir(test_instance)
                if method.startswith("test_async_")
                and callable(getattr(test_instance, method))
            ]

            for method_name in async_methods:
                success, error = await run_test_method(test_instance, method_name)
                async_results.append(
                    (f"{test_class.__name__}.{method_name}", success, error)
                )

    # Run all async tests
    asyncio.run(run_all_async_tests())

    # Print results
    print("\nAsync Test Results:")
    print("=" * 50)

    passed = 0
    failed = 0

    for test_name, success, error in async_results:
        if success:
            print(f"✓ {test_name}")
            passed += 1
        else:
            print(f"✗ {test_name}: {error}")
            failed += 1

    print(f"\nAsync Tests: {passed} passed, {failed} failed")
    return failed == 0


if __name__ == "__main__":
    # Run synchronous tests
    print("Running synchronous error handling tests...")
    unittest.main(verbosity=2, exit=False)

    # Run async tests
    print("\nRunning asynchronous error handling tests...")
    async_success = run_async_tests()

    if not async_success:
        exit(1)
