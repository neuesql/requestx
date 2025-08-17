"""
Unit tests for Response object behavior and requests library compatibility.

Tests the Response PyO3 class with status_code, text, content, headers properties,
json(), raise_for_status(), and other requests-compatible methods.
"""

import json
import unittest
import sys
import os

# Add the project root to the path so we can import requestx
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "python"))

import requestx


class TestResponseProperties(unittest.TestCase):
    """Test Response object properties and basic functionality."""

    def setUp(self):
        """Set up test fixtures."""
        self.test_url = "https://httpbin.org/get"
        self.json_url = "https://httpbin.org/json"
        self.status_url = "https://httpbin.org/status/{}"

    def test_status_code_property(self):
        """Test status_code property."""
        response = requestx.get(self.test_url)
        self.assertEqual(response.status_code, 200)
        self.assertIsInstance(response.status_code, int)

    def test_url_property(self):
        """Test url property."""
        response = requestx.get(self.test_url)
        self.assertTrue(response.url.startswith("https://httpbin.org"))
        self.assertIsInstance(response.url, str)

    def test_ok_property(self):
        """Test ok property for successful responses."""
        response = requestx.get(self.test_url)
        self.assertTrue(response.ok)

        # Test with error status
        response_404 = requestx.get(self.status_url.format(404))
        self.assertFalse(response_404.ok)

    def test_reason_property(self):
        """Test reason property (status text)."""
        response = requestx.get(self.test_url)
        self.assertEqual(response.reason, "OK")

        # Test with different status codes
        response_404 = requestx.get(self.status_url.format(404))
        self.assertEqual(response_404.reason, "Not Found")

    def test_headers_property(self):
        """Test headers property returns dict-like object."""
        response = requestx.get(self.test_url)
        headers = response.headers

        # Should be dict-like
        self.assertTrue(hasattr(headers, "keys"))
        self.assertTrue(hasattr(headers, "values"))
        self.assertTrue(hasattr(headers, "items"))

        # Should contain common headers
        self.assertIn("content-type", [k.lower() for k in headers.keys()])

    def test_text_property(self):
        """Test text property returns string content."""
        response = requestx.get(self.test_url)
        text = response.text

        self.assertIsInstance(text, str)
        self.assertGreater(len(text), 0)

        # Should be valid JSON for httpbin.org/get
        try:
            json.loads(text)
        except json.JSONDecodeError:
            self.fail("Response text should be valid JSON for httpbin.org/get")

    def test_content_property(self):
        """Test content property returns bytes."""
        response = requestx.get(self.test_url)
        content = response.content

        self.assertIsInstance(content, bytes)
        self.assertGreater(len(content), 0)

    def test_text_content_consistency(self):
        """Test that text and content properties are consistent."""
        response = requestx.get(self.test_url)

        # Text should be the decoded version of content
        text_from_property = response.text
        text_from_content = response.content.decode("utf-8")

        self.assertEqual(text_from_property, text_from_content)

    def test_encoding_property(self):
        """Test encoding property and setter."""
        response = requestx.get(self.test_url)

        # Initially might be None or detected from headers
        initial_encoding = response.encoding

        # Should be able to set encoding
        response.encoding = "utf-8"
        self.assertEqual(response.encoding, "utf-8")

        # Should be able to set to None
        response.encoding = None
        self.assertIsNone(response.encoding)

    def test_apparent_encoding_property(self):
        """Test apparent_encoding property."""
        response = requestx.get(self.test_url)
        apparent_encoding = response.apparent_encoding

        self.assertIsInstance(apparent_encoding, str)
        # Should be a reasonable encoding
        self.assertIn(
            apparent_encoding.lower(), ["utf-8", "utf-8-sig", "utf-16-le", "utf-16-be"]
        )


class TestResponseMethods(unittest.TestCase):
    """Test Response object methods."""

    def setUp(self):
        """Set up test fixtures."""
        self.json_url = "https://httpbin.org/json"
        self.status_url = "https://httpbin.org/status/{}"

    def test_json_method(self):
        """Test json() method parses JSON correctly."""
        response = requestx.get(self.json_url)
        json_data = response.json()

        self.assertIsInstance(json_data, dict)
        # httpbin.org/json returns a specific structure
        self.assertIn("slideshow", json_data)

    def test_json_method_with_custom_data(self):
        """Test json() method with custom JSON data."""
        json_payload = {"test": "data", "number": 42, "nested": {"key": "value"}}
        response = requestx.post("https://httpbin.org/post", json=json_payload)

        response_data = response.json()
        sent_json = response_data.get("json", {})

        self.assertEqual(sent_json["test"], "data")
        self.assertEqual(sent_json["number"], 42)
        self.assertEqual(sent_json["nested"]["key"], "value")

    def test_json_method_invalid_json(self):
        """Test json() method with invalid JSON raises error."""
        # Get plain text response
        response = requestx.get("https://httpbin.org/html")

        with self.assertRaises(Exception):  # Should raise JSON decode error
            response.json()

    def test_raise_for_status_success(self):
        """Test raise_for_status() with successful response."""
        response = requestx.get("https://httpbin.org/get")

        # Should not raise any exception
        try:
            response.raise_for_status()
        except Exception as e:
            self.fail(f"raise_for_status() raised {e} for successful response")

    def test_raise_for_status_client_error(self):
        """Test raise_for_status() with 4xx client error."""
        response = requestx.get(self.status_url.format(404))

        with self.assertRaises(Exception):
            response.raise_for_status()

    def test_raise_for_status_server_error(self):
        """Test raise_for_status() with 5xx server error."""
        response = requestx.get(self.status_url.format(500))

        with self.assertRaises(Exception):
            response.raise_for_status()

    def test_raise_for_status_redirect(self):
        """Test raise_for_status() with 3xx redirect (should not raise)."""
        response = requestx.get("https://httpbin.org/redirect/1", allow_redirects=False)

        # 3xx redirects should not raise an exception
        try:
            response.raise_for_status()
        except Exception as e:
            self.fail(f"raise_for_status() raised {e} for redirect response")


class TestResponseCompatibility(unittest.TestCase):
    """Test Response object compatibility with requests library."""

    def setUp(self):
        """Set up test fixtures."""
        self.test_url = "https://httpbin.org/get"

    def test_boolean_evaluation(self):
        """Test Response object boolean evaluation."""
        # Successful response should be truthy
        response_200 = requestx.get(self.test_url)
        self.assertTrue(bool(response_200))
        self.assertTrue(response_200)  # Direct boolean evaluation

        # Error response should be falsy
        response_404 = requestx.get("https://httpbin.org/status/404")
        self.assertFalse(bool(response_404))
        self.assertFalse(response_404)  # Direct boolean evaluation

    def test_string_representation(self):
        """Test Response object string representation."""
        response = requestx.get(self.test_url)

        str_repr = str(response)
        repr_repr = repr(response)

        self.assertIn("200", str_repr)
        self.assertIn("Response", str_repr)
        self.assertIn("200", repr_repr)
        self.assertIn("Response", repr_repr)

    def test_redirect_properties(self):
        """Test redirect-related properties."""
        # Test with redirect response
        redirect_response = requestx.get(
            "https://httpbin.org/redirect/1", allow_redirects=False
        )

        # Should detect redirects
        if redirect_response.status_code in [301, 302, 303, 307, 308]:
            self.assertTrue(redirect_response.is_redirect)

            # Test permanent redirect detection
            if redirect_response.status_code in [301, 308]:
                self.assertTrue(redirect_response.is_permanent_redirect)
            else:
                self.assertFalse(redirect_response.is_permanent_redirect)

    def test_status_text_property(self):
        """Test status_text property (alias for reason)."""
        response = requestx.get(self.test_url)

        self.assertEqual(response.status_text, response.reason)
        self.assertEqual(response.status_text, "OK")

    def test_cookies_property(self):
        """Test cookies property (placeholder implementation)."""
        response = requestx.get(self.test_url)
        cookies = response.cookies

        # Should be dict-like (even if empty for now)
        self.assertTrue(hasattr(cookies, "keys"))

    def test_history_property(self):
        """Test history property (placeholder implementation)."""
        response = requestx.get(self.test_url)
        history = response.history

        # Should be list-like (even if empty for now)
        self.assertTrue(hasattr(history, "__iter__"))

    def test_links_property(self):
        """Test links property (placeholder implementation)."""
        response = requestx.get(self.test_url)
        links = response.links

        # Should be dict-like (even if empty for now)
        self.assertTrue(hasattr(links, "keys"))

    def test_next_property(self):
        """Test next property (placeholder implementation)."""
        response = requestx.get(self.test_url)
        next_response = response.next

        # Should be None for now
        self.assertIsNone(next_response)


class TestResponseEncoding(unittest.TestCase):
    """Test Response object encoding detection and handling."""

    def setUp(self):
        """Set up test fixtures."""
        self.test_url = "https://httpbin.org/get"

    def test_encoding_detection_from_headers(self):
        """Test encoding detection from Content-Type header."""
        # This is a basic test - actual encoding detection would need
        # responses with specific Content-Type headers
        response = requestx.get(self.test_url)

        # Should have some encoding (detected or default)
        encoding = response.encoding
        if encoding is not None:
            self.assertIsInstance(encoding, str)

    def test_encoding_override(self):
        """Test manual encoding override."""
        response = requestx.get(self.test_url)

        # Set custom encoding
        response.encoding = "latin-1"
        self.assertEqual(response.encoding, "latin-1")

        # Text should be re-decoded with new encoding
        # (This is a basic test - full implementation would re-decode)
        text = response.text
        self.assertIsInstance(text, str)

    def test_encoding_none(self):
        """Test behavior when encoding is None."""
        response = requestx.get(self.test_url)

        response.encoding = None
        self.assertIsNone(response.encoding)

        # Should still be able to get text (using default encoding)
        text = response.text
        self.assertIsInstance(text, str)


class TestResponseEdgeCases(unittest.TestCase):
    """Test Response object edge cases and error conditions."""

    def setUp(self):
        """Set up test fixtures."""
        self.test_url = "https://httpbin.org/get"

    def test_empty_response(self):
        """Test handling of empty response body."""
        # HEAD request should have empty body
        response = requestx.head("https://httpbin.org/get")

        self.assertEqual(len(response.content), 0)
        self.assertEqual(response.text, "")

    def test_large_response(self):
        """Test handling of large response."""
        # Get a reasonably large response
        response = requestx.get("https://httpbin.org/bytes/10000")

        self.assertEqual(response.status_code, 200)
        self.assertEqual(len(response.content), 10000)

    def test_binary_response(self):
        """Test handling of binary response."""
        response = requestx.get("https://httpbin.org/bytes/100")

        self.assertEqual(response.status_code, 200)
        self.assertIsInstance(response.content, bytes)
        self.assertEqual(len(response.content), 100)

        # Text should be decodable (even if it's binary data)
        text = response.text
        self.assertIsInstance(text, str)

    def test_json_with_different_content_types(self):
        """Test json() method with different content types."""
        # Test with explicit JSON content type
        response = requestx.get("https://httpbin.org/json")
        json_data = response.json()
        self.assertIsInstance(json_data, dict)

    def test_multiple_property_access(self):
        """Test multiple access to properties (caching behavior)."""
        response = requestx.get(self.test_url)

        # Multiple access should return consistent results
        text1 = response.text
        text2 = response.text
        self.assertEqual(text1, text2)

        content1 = response.content
        content2 = response.content
        self.assertEqual(content1, content2)

        headers1 = response.headers
        headers2 = response.headers
        # Headers should be consistent (though may be different objects)
        self.assertEqual(list(headers1.keys()), list(headers2.keys()))


if __name__ == "__main__":
    unittest.main(verbosity=2)
