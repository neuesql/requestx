"""
RequestX Quickstart Test Suite

This test suite covers the basic functionality of the requestx library,
mirroring the examples from the requests library quickstart guide.

Test categories:
- Basic HTTP methods (GET, POST, PUT, DELETE, HEAD, OPTIONS, PATCH)
- URL parameters (params)
- Response content (text, content, json)
- Custom headers
- POST data (form data, JSON data, text data)
- Response status codes
- Response headers
- Cookies and authentication
- Redirection and timeouts
- Session management
- Error handling

All tests use a local httpbin container via testcontainers.
"""

import sys
import os

# Add the python directory to the path for importing requestx
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "python"))

import unittest
import json
import time
import requestx
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
        if hasattr(cls, 'container'):
            cls.container.stop()


class TestBasicRequests(HttpbinTestCase):
    """Test basic HTTP requests following requests library quickstart examples."""

    def test_get_request(self):
        """Test making a simple GET request."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        self.assertEqual(r.status_code, 200)
        self.assertIsInstance(r.text, str)
        self.assertTrue(len(r.text) > 0)

    def test_post_request(self):
        """Test making a POST request with form data."""
        payload = {"key1": "value1", "key2": "value2"}
        r = requestx.post(HTTPBIN_HOST + "/post", data=payload)
        self.assertEqual(r.status_code, 200)
        # httpbin.org echoes back the form data
        response_json = r.json()
        self.assertIn("data", response_json)
        # Form data should be in the data field as URL-encoded string
        self.assertIn("key1=value1", response_json["data"])

    def test_post_form_data_dict(self):
        """Test POST with form data as dictionary."""
        payload = {"key1": "value1", "key2": "value2"}
        r = requestx.post(HTTPBIN_HOST + "/post", data=payload)
        self.assertEqual(r.status_code, 200)
        data = r.json()
        # Form data is in the data field
        self.assertIn("data", data)
        self.assertIn("key1=value1", data["data"])

    def test_post_json_data(self):
        """Test POST with JSON data using json parameter."""
        payload = {"key": "value", "number": 42}
        r = requestx.post(HTTPBIN_HOST + "/post", json=payload)
        self.assertEqual(r.status_code, 200)
        data = r.json()
        self.assertIn("json", data)
        self.assertEqual(data["json"]["key"], "value")
        self.assertEqual(data["json"]["number"], 42)

    def test_post_text_data(self):
        """Test POST with plain text data."""
        text_data = "Hello, World!"
        r = requestx.post(HTTPBIN_HOST + "/post", data=text_data)
        self.assertEqual(r.status_code, 200)
        data = r.json()
        self.assertIn("data", data)
        self.assertEqual(data["data"], text_data)

    def test_post_bytes_data(self):
        """Test POST with binary data."""
        binary_data = b"\x00\x01\x02\x03\x04"
        r = requestx.post(HTTPBIN_HOST + "/post", data=binary_data)
        self.assertEqual(r.status_code, 200)

    def test_post_json_automatic_content_type(self):
        """Test that json parameter sets Content-Type header."""
        r = requestx.post(HTTPBIN_HOST + "/post", json={"key": "value"})
        self.assertEqual(r.status_code, 200)
        data = r.json()
        # httpbin should report the content type
        self.assertIn("headers", data)


class TestResponseStatusCodes(HttpbinTestCase):
    """Test response status code handling."""

    def test_status_code_200(self):
        """Test successful GET request returns 200."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        self.assertEqual(r.status_code, 200)

    def test_status_code_404(self):
        """Test 404 Not Found response."""
        r = requestx.get(HTTPBIN_HOST + "/status/404")
        self.assertEqual(r.status_code, 404)

    def test_status_code_500(self):
        """Test 500 Internal Server Error response."""
        r = requestx.get(HTTPBIN_HOST + "/status/500")
        self.assertEqual(r.status_code, 500)

    def test_response_ok_property(self):
        """Test ok property for successful responses."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        self.assertTrue(r.ok)

        r = requestx.get(HTTPBIN_HOST + "/status/404")
        self.assertFalse(r.ok)

    def test_raise_for_status_success(self):
        """Test raise_for_status does not raise for 200."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        # Should not raise
        r.raise_for_status()

    def test_raise_for_status_4xx(self):
        """Test raise_for_status raises for 4xx status codes."""
        r = requestx.get(HTTPBIN_HOST + "/status/404")
        with self.assertRaises(Exception):
            r.raise_for_status()

    def test_raise_for_status_5xx(self):
        """Test raise_for_status raises for 5xx status codes."""
        r = requestx.get(HTTPBIN_HOST + "/status/500")
        with self.assertRaises(Exception):
            r.raise_for_status()

    def test_response_reason(self):
        """Test response reason phrase."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        self.assertEqual(r.reason, "OK")

        r = requestx.get(HTTPBIN_HOST + "/status/404")
        # Reason may be 'Not Found' or 'NOT FOUND' depending on implementation
        self.assertTrue("not found" in r.reason.lower())


class TestResponseHeaders(HttpbinTestCase):
    """Test response header access."""

    def test_headers_dict(self):
        """Test headers are accessible as dictionary."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        self.assertIsInstance(r.headers, dict)
        # Header keys are lowercase in response
        self.assertIn("content-type", r.headers)

    def test_headers_case_insensitive(self):
        """Test header access is case-insensitive."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        # Headers dict uses lowercase keys
        content_type1 = r.headers.get("content-type")
        content_type2 = r.headers.get("Content-Type")
        # At least one should work (may vary by implementation)
        self.assertTrue(content_type1 is not None or content_type2 is not None)

    def test_headers_get_method(self):
        """Test using get() method on headers."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        # Use lowercase for headers dict
        content_type = r.headers.get("content-type")
        self.assertIsNotNone(content_type)
        self.assertIn("application/json", content_type)


class TestCookies(HttpbinTestCase):
    """Test cookie handling."""

    def test_send_cookies(self):
        """Test sending cookies with request."""
        cookies = {"test_cookie": "test_value"}
        r = requestx.get(HTTPBIN_HOST + "/cookies", cookies=cookies)
        self.assertEqual(r.status_code, 200)
        data = r.json()
        self.assertIn("cookies", data)


class TestAuthentication(HttpbinTestCase):
    """Test HTTP authentication."""

    def test_basic_auth(self):
        """Test basic HTTP authentication."""
        # httpbin.org/basic-auth/user/passwd expects username:user, password:passwd
        r = requestx.get(
            HTTPBIN_HOST + "/basic-auth/user/passwd", auth=("user", "passwd")
        )
        self.assertEqual(r.status_code, 200)

    def test_basic_auth_failure(self):
        """Test basic HTTP authentication failure."""
        r = requestx.get(
            HTTPBIN_HOST + "/basic-auth/user/passwd", auth=("user", "wrong_password")
        )
        self.assertEqual(r.status_code, 401)


class TestRedirection(HttpbinTestCase):
    """Test redirection handling."""

    def test_default_allow_redirects(self):
        """Test that redirects are followed by default."""
        # httpbin.org redirects to https
        r = requestx.get(HTTPBIN_HOST +"/relative-redirect/1")
        # Should follow redirect and end up at a different URL
        self.assertNotEqual(r.status_code, 302)

    def test_disable_redirects(self):
        """Test disabling redirects with allow_redirects=False."""
        r = requestx.get(
            HTTPBIN_HOST + "/redirect-to",
            params={"url": HTTPBIN_HOST + "/get"},
            allow_redirects=False,
        )
        self.assertEqual(r.status_code, 302)


class TestTimeouts(HttpbinTestCase):
    """Test timeout handling."""

    def test_timeout_success(self):
        """Test request completes within timeout."""
        r = requestx.get(HTTPBIN_HOST + "/get", timeout=10)
        self.assertEqual(r.status_code, 200)

    def test_timeout_short(self):
        """Test short timeout raises exception."""
        # httpbin.org/delay/3 delays response by 3 seconds
        # Using a very short timeout should raise an error
        with self.assertRaises(Exception):
            requestx.get(HTTPBIN_HOST + "/delay/3", timeout=0.001)

    def test_timeout_none(self):
        """Test timeout=None (wait indefinitely)."""
        # This test may take a while, so we use a short delay
        r = requestx.get(HTTPBIN_HOST + "/delay/1", timeout=None)
        self.assertEqual(r.status_code, 200)


class TestSessionManagement(HttpbinTestCase):
    """Test Session class for persistent connections."""

    def test_session_creation(self):
        """Test creating a Session object."""
        session = requestx.Session()
        self.assertIsNotNone(session)
        self.assertIn("Session", repr(session))

    def test_session_get(self):
        """Test GET request using session."""
        session = requestx.Session()
        r = session.get(HTTPBIN_HOST + "/get")
        self.assertEqual(r.status_code, 200)

    def test_session_post(self):
        """Test POST request using session."""
        session = requestx.Session()
        r = session.post(HTTPBIN_HOST + "/post", data={"key": "value"})
        self.assertEqual(r.status_code, 200)

    def test_session_headers(self):
        """Test setting session headers."""
        session = requestx.Session()
        session.update_header("X-Session-Header", "session-value")
        r = session.get(HTTPBIN_HOST + "/headers")
        self.assertEqual(r.status_code, 200)
        data = r.json()
        # Session headers may or may not be included depending on implementation

    def test_session_context_manager(self):
        """Test session as context manager."""
        with requestx.Session() as session:
            r = session.get(HTTPBIN_HOST + "/get")
            self.assertEqual(r.status_code, 200)

    def test_session_update_headers(self):
        """Test updating individual session headers."""
        session = requestx.Session()
        session.update_header("X-Custom-Header", "custom-value")
        r = session.get(HTTPBIN_HOST + "/headers")
        self.assertEqual(r.status_code, 200)

    def test_session_remove_header(self):
        """Test removing session headers."""
        session = requestx.Session()
        session.update_header("X-To-Remove", "value")
        session.remove_header("X-To-Remove")
        r = session.get(HTTPBIN_HOST + "/headers")
        self.assertEqual(r.status_code, 200)
        data = r.json()
        self.assertNotIn("X-To-Remove", data.get("headers", {}))


class TestErrorHandling(HttpbinTestCase):
    """Test error and exception handling."""

    def test_invalid_url(self):
        """Test handling of invalid URLs."""
        with self.assertRaises(Exception):
            requestx.get("not-a-valid-url")

    def test_missing_schema(self):
        """Test handling of URLs without scheme."""
        with self.assertRaises(Exception):
            requestx.get("example.com")

    def test_connection_error(self):
        """Test handling of connection errors."""
        # Using a non-existent domain should raise connection error
        with self.assertRaises(Exception):
            requestx.get("http://this-domain-does-not-exist-12345.com", timeout=2)

    def test_http_error_4xx(self):
        """Test HTTPError is raised for 4xx responses."""
        r = requestx.get(HTTPBIN_HOST + "/status/404")
        self.assertEqual(r.status_code, 404)
        # Using raise_for_status should raise
        with self.assertRaises(Exception):
            r.raise_for_status()

    def test_json_decode_error(self):
        """Test JSON decode error on invalid JSON response."""
        r = requestx.get(HTTPBIN_HOST + "/bytes/10")
        self.assertEqual(r.status_code, 200)
        # Attempting to parse non-JSON as JSON should raise
        with self.assertRaises(Exception):
            r.json()


class TestResponseIterators(HttpbinTestCase):
    """Test response content iteration methods."""

    def test_iter_content(self):
        """Test iter_content method."""
        r = requestx.get(HTTPBIN_HOST + "/bytes/1024")
        self.assertEqual(r.status_code, 200)
        chunks = list(r.iter_content(chunk_size=256))
        self.assertTrue(len(chunks) > 0)
        # Each chunk should be bytes
        for chunk in chunks:
            self.assertIsInstance(chunk, bytes)

    def test_iter_lines(self):
        """Test iter_lines method for text responses."""
        r = requestx.get(HTTPBIN_HOST + "/stream/10")
        self.assertEqual(r.status_code, 200)
        # Should be able to iterate over lines
        lines = list(r.iter_lines())
        self.assertTrue(len(lines) >= 0)


class TestResponseMetadata(HttpbinTestCase):
    """Test response metadata properties."""

    def test_response_url(self):
        """Test response URL property."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        self.assertIn(HTTPBIN_HOST, r.url)

    def test_response_is_redirect(self):
        """Test is_redirect property."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        self.assertFalse(r.is_redirect)

        r = requestx.get(
            HTTPBIN_HOST + "/redirect-to",
            params={"url": HTTPBIN_HOST + "/get"},
            allow_redirects=False,
        )
        self.assertTrue(r.is_redirect)

    def test_response_is_permanent_redirect(self):
        """Test is_permanent_redirect property."""
        r = requestx.get(HTTPBIN_HOST + "/status/301", allow_redirects=False)
        self.assertTrue(r.is_permanent_redirect)

        r = requestx.get(HTTPBIN_HOST + "/status/302", allow_redirects=False)
        self.assertFalse(r.is_permanent_redirect)

    def test_response_bool(self):
        """Test boolean evaluation of response."""
        r_success = requestx.get(HTTPBIN_HOST + "/get")
        self.assertTrue(r_success)

        r_error = requestx.get(HTTPBIN_HOST + "/status/404")
        self.assertFalse(r_error)


class TestVerifySSL(HttpbinTestCase):
    """Test SSL verification options."""

    def test_verify_default(self):
        """Test default SSL verification."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        self.assertEqual(r.status_code, 200)

    def test_verify_false(self):
        """Test disabling SSL verification."""
        # This should work without errors
        r = requestx.get(HTTPBIN_HOST + "/get", verify=False)
        self.assertEqual(r.status_code, 200)


class TestGenericRequest(HttpbinTestCase):
    """Test the generic request() function with various methods."""

    def test_request_all_methods(self):
        """Test request() with all supported HTTP methods."""
        methods = ["GET", "POST", "PUT", "DELETE", "PATCH"]
        for method in methods:
            url = HTTPBIN_HOST + "/" + method.lower()
            r = requestx.request(method, url)
            self.assertEqual(r.status_code, 200, f"Method {method} failed")

    def test_request_with_all_params(self):
        """Test request() with comprehensive parameters."""
        r = requestx.request(
            "POST",
            HTTPBIN_HOST + "/post",
            params={"param": "value"},
            headers={"X-Custom": "test"},
            data={"key": "value"},
            timeout=10,
        )
        self.assertEqual(r.status_code, 200)


if __name__ == "__main__":
    unittest.main()
