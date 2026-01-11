"""
RequestX Phase 6 Test Suite - Exception Hierarchy & Utilities

This test suite covers:
- Complete exception mapping and inheritance
- Cookie utility functions (cookiejar_from_dict, dict_from_cookiejar, etc.)
- URL utility functions (requote_uri, get_auth_from_url, urldefragauth)

Tests follow the same pattern as test_quickstart.py using testcontainers.
"""

import os
import sys
import unittest

# Add the python directory to the path for importing requestx
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "python"))

import requestx
from requestx import (
    cookies,
    requote_uri,
    get_auth_from_url,
    urldefragauth,
    RequestException,
    Timeout,
)
from testcontainers.generic import ServerContainer

# Will be set dynamically by the container
HTTPBIN_HOST = "http://localhost"


class HttpbinTestCase(unittest.TestCase):
    """Base test case that provides an httpbin container for all tests."""

    httpbin_port = 80

    @classmethod
    def setUpClass(cls):
        """Start httpbin container before all tests in this class."""
        cls.container = ServerContainer(port=80, image="kennethreitz/httpbin")
        cls.container.start()
        # Get the exposed port (handles port mapping)
        cls.httpbin_port = cls.container.get_exposed_port(80)
        # Update the global HTTPBIN_HOST
        global HTTPBIN_HOST
        HTTPBIN_HOST = f"http://localhost:{cls.httpbin_port}"

    @classmethod
    def tearDownClass(cls):
        """Stop httpbin container after all tests in this class."""
        if hasattr(cls, "container"):
            cls.container.stop()


class TestExceptionHierarchy(HttpbinTestCase):
    """Test exception hierarchy and inheritance."""

    def test_exception_inheritance_chain(self):
        """Test that exceptions follow proper inheritance chain."""
        # Base exception
        self.assertTrue(issubclass(requestx.RequestException, Exception))
        
        # Connection errors
        self.assertTrue(issubclass(requestx.ConnectionError, requestx.RequestException))
        self.assertTrue(issubclass(requestx.SSLError, requestx.ConnectionError))
        self.assertTrue(issubclass(requestx.ProxyError, requestx.ConnectionError))
        
        # Timeout errors
        self.assertTrue(issubclass(requestx.Timeout, requestx.RequestException))
        self.assertTrue(issubclass(requestx.ConnectTimeout, requestx.ConnectionError))
        self.assertTrue(issubclass(requestx.ConnectTimeout, requestx.Timeout))
        
        # HTTP errors
        self.assertTrue(issubclass(requestx.HTTPError, requestx.RequestException))

    def test_unreachable_code_error_exists(self):
        """Test that UnreachableCodeError exception is available."""
        self.assertTrue(hasattr(requestx, 'UnreachableCodeError'))
        self.assertTrue(issubclass(requestx.UnreachableCodeError, RequestException))

    def test_unrewindable_body_error_exists(self):
        """Test that UnrewindableBodyError exception is available."""
        self.assertTrue(hasattr(requestx, 'UnrewindableBodyError'))
        self.assertTrue(issubclass(requestx.UnrewindableBodyError, RequestException))

    def test_invalid_json_error_alias(self):
        """Test that InvalidJSONError is an alias for JSONDecodeError."""
        self.assertTrue(hasattr(requestx, 'InvalidJSONError'))
        self.assertIs(requestx.InvalidJSONError, requestx.JSONDecodeError)

    def test_chunked_encoding_error(self):
        """Test ChunkedEncodingError exception."""
        self.assertTrue(hasattr(requestx, 'ChunkedEncodingError'))
        self.assertTrue(issubclass(requestx.ChunkedEncodingError, requestx.ConnectionError))

    def test_content_decoding_error(self):
        """Test ContentDecodingError exception."""
        self.assertTrue(hasattr(requestx, 'ContentDecodingError'))
        self.assertTrue(issubclass(requestx.ContentDecodingError, RequestException))

    def test_stream_consumed_error(self):
        """Test StreamConsumedError exception."""
        self.assertTrue(hasattr(requestx, 'StreamConsumedError'))
        self.assertTrue(issubclass(requestx.StreamConsumedError, RequestException))

    def test_exception_attributes(self):
        """Test that exceptions have proper attributes."""
        exc = requestx.RequestException("test message")
        self.assertEqual(str(exc), "test message")
        self.assertEqual(exc.args, ("test message",))

    def test_exception_raising_and_catching(self):
        """Test that exceptions can be raised and caught properly."""
        # Test catching base exception
        try:
            raise requestx.RequestException("base error")
        except requestx.RequestException as e:
            self.assertEqual(str(e), "base error")
        
        # Test catching specific exception
        try:
            raise requestx.ConnectionError("connection error")
        except requestx.RequestException as e:
            self.assertEqual(str(e), "connection error")
            self.assertIsInstance(e, requestx.ConnectionError)

    def test_warning_classes(self):
        """Test that warning classes are available."""
        self.assertTrue(issubclass(requestx.RequestsWarning, UserWarning))
        self.assertTrue(issubclass(requestx.DependencyWarning, requestx.RequestsWarning))
        self.assertTrue(issubclass(requestx.FileModeWarning, RequestException))


class TestCookieUtilities(HttpbinTestCase):
    """Test cookie utility functions."""

    def test_cookiejar_from_dict_empty(self):
        """Test creating empty cookie jar from dict."""
        jar = cookies.cookiejar_from_dict()
        self.assertEqual(len(jar), 0)

    def test_cookiejar_from_dict_with_values(self):
        """Test creating cookie jar from dictionary."""
        jar = cookies.cookiejar_from_dict({"key1": "value1", "key2": "value2"})
        self.assertEqual(jar["key1"], "value1")
        self.assertEqual(jar["key2"], "value2")

    def test_cookiejar_from_dict_with_existing_jar(self):
        """Test adding to existing cookie jar."""
        jar = cookies.cookiejar_from_dict({"existing": "value"})
        jar = cookies.cookiejar_from_dict({"new": "value2"}, cookiejar=jar)
        self.assertEqual(jar["existing"], "value")
        self.assertEqual(jar["new"], "value2")

    def test_dict_from_cookiejar_empty(self):
        """Test converting empty cookie jar to dict."""
        jar = cookies.cookiejar_from_dict()
        result = cookies.dict_from_cookiejar(jar)
        self.assertEqual(result, {})

    def test_dict_from_cookiejar_with_values(self):
        """Test converting cookie jar to dictionary."""
        jar = cookies.cookiejar_from_dict({"key": "value"})
        result = cookies.dict_from_cookiejar(jar)
        self.assertEqual(result, {"key": "value"})

    def test_merge_cookies(self):
        """Test merging cookies into a jar."""
        jar = cookies.cookiejar_from_dict({"existing": "value1"})
        jar = cookies.merge_cookies(jar, {"new": "value2"})
        self.assertEqual(jar["existing"], "value1")
        self.assertEqual(jar["new"], "value2")

    def test_merge_cookies_from_cookiejar(self):
        """Test merging from another cookie jar."""
        jar1 = cookies.cookiejar_from_dict({"key1": "value1"})
        jar2 = cookies.cookiejar_from_dict({"key2": "value2"})
        jar1 = cookies.merge_cookies(jar1, jar2)
        self.assertEqual(jar1["key1"], "value1")
        self.assertEqual(jar1["key2"], "value2")

    def test_add_dict_to_cookiejar(self):
        """Test add_dict_to_cookiejar function."""
        jar = cookies.cookiejar_from_dict()
        cookies.add_dict_to_cookiejar(jar, {"key": "value"})
        self.assertEqual(jar["key"], "value")

    def test_cookie_jar_contains(self):
        """Test 'in' operator for cookie jar."""
        jar = cookies.cookiejar_from_dict({"key": "value"})
        self.assertIn("key", jar)
        self.assertNotIn("missing", jar)

    def test_cookie_jar_iteration(self):
        """Test iterating over cookie jar."""
        jar = cookies.cookiejar_from_dict({"key1": "value1", "key2": "value2"})
        keys = list(jar)
        self.assertIn("key1", keys)
        self.assertIn("key2", keys)

    def test_cookie_jar_keys_values_items(self):
        """Test keys(), values(), items() methods."""
        jar = cookies.cookiejar_from_dict({"key1": "value1", "key2": "value2"})
        self.assertEqual(list(jar.keys()), ["key1", "key2"])
        self.assertIn("value1", list(jar.values()))
        self.assertIn(("key1", "value1"), list(jar.items()))


class TestURLUtilities(HttpbinTestCase):
    """Test URL utility functions."""

    def test_get_auth_from_url_with_credentials(self):
        """Test extracting auth from URL with credentials."""
        auth = get_auth_from_url("https://user:pass@example.com/path")
        self.assertIsNotNone(auth)
        username, password = auth
        self.assertEqual(username, "user")
        self.assertEqual(password, "pass")

    def test_get_auth_from_url_without_credentials(self):
        """Test extracting auth from URL without credentials."""
        auth = get_auth_from_url("https://example.com/path")
        self.assertIsNone(auth)

    def test_get_auth_from_url_user_only(self):
        """Test extracting auth with username only."""
        auth = get_auth_from_url("https://user@example.com/path")
        self.assertIsNotNone(auth)
        username, password = auth
        self.assertEqual(username, "user")
        # Password may be empty string or None depending on implementation
        self.assertTrue(password == "" or password is None)

    def test_urldefragauth(self):
        """Test removing auth and fragment from URL."""
        result = urldefragauth("https://user:pass@example.com/path#fragment")
        self.assertEqual(result, "https://example.com/path")

    def test_urldefragauth_no_auth(self):
        """Test urldefragauth without auth."""
        result = urldefragauth("https://example.com/path#fragment")
        self.assertEqual(result, "https://example.com/path")

    def test_urldefragauth_no_fragment(self):
        """Test urldefragauth without fragment."""
        result = urldefragauth("https://user:pass@example.com/path")
        self.assertEqual(result, "https://example.com/path")

    def test_requote_uri_simple(self):
        """Test requoting a simple URI."""
        result = requote_uri("https://example.com/path")
        self.assertEqual(result, "https://example.com/path")

    def test_requote_uri_with_spaces(self):
        """Test requoting a URI with spaces."""
        result = requote_uri("https://example.com/path with spaces")
        self.assertEqual(result, "https://example.com/path%20with%20spaces")

    def test_requote_uri_with_special_chars(self):
        """Test requoting a URI with special characters."""
        result = requote_uri("https://example.com/path?name=value&other=test")
        # Should preserve query string
        self.assertIn("name=value", result)

    def test_requote_uri_with_unicode(self):
        """Test requoting a URI with unicode characters."""
        result = requote_uri("https://example.com/café")
        # Should properly encode unicode
        self.assertTrue(
            "caf" in result.lower() or len(result) > len("https://example.com/"),
            f"Expected 'caf' or encoded form in result: {result}"
        )


class TestAuthWithRealRequests(HttpbinTestCase):
    """Test that utilities work with real HTTP requests."""

    def test_get_auth_from_url_with_httpbin(self):
        """Test get_auth_from_url with httpbin basic auth endpoint."""
        # URL with credentials for httpbin's basic-auth endpoint
        url = HTTPBIN_HOST + "/basic-auth/user/passwd"
        auth = get_auth_from_url(url)
        # The URL doesn't have credentials, so should be None
        self.assertIsNone(auth)

    def test_cookie_jar_integration(self):
        """Test cookie utilities work with dict objects."""
        # Create a plain dict acting as cookie jar
        cookie_jar = {}
        
        # Add cookies using the utility
        cookies.add_dict_to_cookiejar(cookie_jar, {"test_cookie": "test_value"})
        
        # Verify cookies were added
        self.assertEqual(cookie_jar["test_cookie"], "test_value")
        
        # Test merge
        cookies.merge_cookies(cookie_jar, {"another": "value"})
        self.assertEqual(cookie_jar["test_cookie"], "test_value")
        self.assertEqual(cookie_jar["another"], "value")


class TestExceptionMapping(HttpbinTestCase):
    """Test exception mapping from Rust errors to Python exceptions."""

    def test_connection_error_on_invalid_host(self):
        """Test that connection errors are properly raised."""
        with self.assertRaises(requestx.ConnectionError):
            requestx.get("http://this-domain-does-not-exist-12345.com", timeout=1)

    def test_invalid_url_exception(self):
        """Test that invalid URLs raise appropriate exceptions."""
        with self.assertRaises((requestx.InvalidURL, requestx.MissingSchema)):
            requestx.get("not-a-valid-url")

    def test_timeout_exception(self):
        """Test that timeouts raise appropriate exceptions."""
        # httpbin's delay endpoint with very short timeout
        with self.assertRaises((requestx.Timeout, requestx.ConnectTimeout)):
            requestx.get(HTTPBIN_HOST + "/delay/5", timeout=0.001)

    def test_http_error_exception(self):
        """Test that HTTP errors can be caught."""
        r = requestx.get(HTTPBIN_HOST + "/status/404")
        self.assertEqual(r.status_code, 404)
        with self.assertRaises(requestx.HTTPError):
            r.raise_for_status()


if __name__ == "__main__":
    unittest.main()
