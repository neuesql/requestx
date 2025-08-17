"""
Test requests library compatibility for Response object.

This test verifies that the Response object behaves identically to requests.Response
for common usage patterns.
"""

import unittest
import sys
import os

# Add the project root to the path so we can import requestx
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "python"))

import requestx


class TestRequestsCompatibility(unittest.TestCase):
    """Test that Response object is compatible with requests library patterns."""

    def setUp(self):
        """Set up test fixtures."""
        self.test_url = "https://httpbin.org/get"

    def test_basic_response_pattern(self):
        """Test basic response usage pattern from requests."""
        # This is the most common requests pattern
        response = requestx.get(self.test_url)

        # Basic properties
        self.assertIsInstance(response.status_code, int)
        self.assertEqual(response.status_code, 200)
        self.assertTrue(response.ok)

        # Content access
        self.assertIsInstance(response.text, str)
        self.assertIsInstance(response.content, bytes)

        # JSON parsing
        json_data = response.json()
        self.assertIsInstance(json_data, dict)

        # Headers access
        headers = response.headers
        self.assertTrue(hasattr(headers, "keys"))

    def test_error_handling_pattern(self):
        """Test error handling pattern from requests."""
        response = requestx.get("https://httpbin.org/status/404")

        # Should not be ok
        self.assertFalse(response.ok)
        self.assertEqual(response.status_code, 404)

        # Should raise exception
        with self.assertRaises(Exception):
            response.raise_for_status()

    def test_conditional_processing_pattern(self):
        """Test conditional processing pattern from requests."""
        response = requestx.get(self.test_url)

        # Common pattern: if response:
        if response:
            data = response.json()
            self.assertIsInstance(data, dict)
        else:
            self.fail("Response should be truthy for successful request")

        # Common pattern: if response.ok:
        if response.ok:
            self.assertEqual(response.status_code, 200)
        else:
            self.fail("Response should be ok for successful request")

    def test_response_inspection_pattern(self):
        """Test response inspection pattern from requests."""
        response = requestx.get(self.test_url)

        # Common inspection patterns
        print(f"Status: {response.status_code}")
        print(f"URL: {response.url}")
        print(f"Headers: {len(response.headers)}")
        print(f"Content length: {len(response.content)}")

        # String representation
        response_str = str(response)
        self.assertIn("200", response_str)
        self.assertIn("Response", response_str)

    def test_json_error_handling_pattern(self):
        """Test JSON error handling pattern from requests."""
        # Get non-JSON response
        response = requestx.get("https://httpbin.org/html")

        # Common pattern: try to parse JSON, handle errors
        try:
            data = response.json()
            self.fail("Should have raised exception for non-JSON content")
        except Exception:
            # This is expected
            pass

    def test_header_access_patterns(self):
        """Test header access patterns from requests."""
        response = requestx.get(self.test_url)

        # Headers should be accessible like a dict
        headers = response.headers

        # Should be able to iterate
        header_count = 0
        for key in headers:
            header_count += 1
        self.assertGreater(header_count, 0)

        # Should be able to access values
        for key in headers:
            value = headers[key]
            self.assertIsInstance(value, str)
            break

    def test_encoding_patterns(self):
        """Test encoding handling patterns from requests."""
        response = requestx.get(self.test_url)

        # Should have encoding (or None)
        encoding = response.encoding
        if encoding is not None:
            self.assertIsInstance(encoding, str)

        # Should have apparent encoding
        apparent_encoding = response.apparent_encoding
        self.assertIsInstance(apparent_encoding, str)

        # Should be able to set encoding
        response.encoding = "utf-8"
        self.assertEqual(response.encoding, "utf-8")

    def test_redirect_patterns(self):
        """Test redirect handling patterns from requests."""
        # Test with redirect
        response = requestx.get("https://httpbin.org/redirect/1", allow_redirects=False)

        if response.status_code in [301, 302, 303, 307, 308]:
            # Should detect redirects
            self.assertTrue(response.is_redirect)

            # Should have history (even if empty for now)
            history = response.history
            self.assertTrue(hasattr(history, "__iter__"))

    def test_boolean_and_string_patterns(self):
        """Test boolean and string representation patterns from requests."""
        # Successful response
        response_ok = requestx.get(self.test_url)
        self.assertTrue(bool(response_ok))
        self.assertTrue(response_ok)  # Direct boolean evaluation

        # Error response
        response_error = requestx.get("https://httpbin.org/status/404")
        self.assertFalse(bool(response_error))
        self.assertFalse(response_error)  # Direct boolean evaluation

        # String representations
        self.assertIn("200", str(response_ok))
        self.assertIn("404", str(response_error))
        self.assertIn("Response", repr(response_ok))


if __name__ == "__main__":
    unittest.main(verbosity=2)
