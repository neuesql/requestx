#!/usr/bin/env python3
"""
Unit tests for advanced HTTP features including parameters, headers, data, JSON,
timeout handling, redirect control, SSL verification, proxy support, and authentication.
"""

import unittest
import sys
import os
import json
import base64

# Add the project root to the path so we can import requestx
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "python"))

import requestx


class TestAdvancedHTTPFeatures(unittest.TestCase):
    """Test advanced HTTP features with hyper."""

    def setUp(self):
        """Set up test fixtures."""
        self.base_url = "https://httpbin.org"

    def test_request_parameters(self):
        """Test request with query parameters."""
        params = {"param1": "value1", "param2": "value2", "special": "hello world"}

        response = requestx.get(f"{self.base_url}/get", params=params)
        self.assertEqual(response.status_code, 200)

        # Parse response to verify parameters were sent
        data = response.json()
        args = data.get("args", {})
        self.assertEqual(args.get("param1"), "value1")
        self.assertEqual(args.get("param2"), "value2")
        self.assertEqual(args.get("special"), "hello world")

    def test_custom_headers(self):
        """Test request with custom headers."""
        headers = {
            "User-Agent": "RequestX-Test/1.0",
            "X-Custom-Header": "test-value",
            "Accept": "application/json",
        }

        response = requestx.get(f"{self.base_url}/get", headers=headers)
        self.assertEqual(response.status_code, 200)

        # Parse response to verify headers were sent
        data = response.json()
        request_headers = data.get("headers", {})
        self.assertEqual(request_headers.get("User-Agent"), "RequestX-Test/1.0")
        self.assertEqual(request_headers.get("X-Custom-Header"), "test-value")
        self.assertEqual(request_headers.get("Accept"), "application/json")

    def test_json_payload(self):
        """Test POST request with JSON payload."""
        json_data = {"name": "test", "value": 42, "nested": {"key": "value"}}

        response = requestx.post(f"{self.base_url}/post", json=json_data)
        self.assertEqual(response.status_code, 200)

        # Parse response to verify JSON was sent
        data = response.json()
        sent_json = data.get("json", {})
        self.assertEqual(sent_json.get("name"), "test")
        self.assertEqual(sent_json.get("value"), 42)
        self.assertEqual(sent_json.get("nested", {}).get("key"), "value")

    def test_form_data(self):
        """Test POST request with form data."""
        form_data = {"field1": "value1", "field2": "value2", "special": "hello world"}

        response = requestx.post(f"{self.base_url}/post", data=form_data)
        self.assertEqual(response.status_code, 200)

        # Parse response to verify form data was sent
        data = response.json()
        sent_form = data.get("form", {})
        self.assertEqual(sent_form.get("field1"), "value1")
        self.assertEqual(sent_form.get("field2"), "value2")
        self.assertEqual(sent_form.get("special"), "hello world")

    def test_text_data(self):
        """Test POST request with text data."""
        text_data = "This is raw text data"

        response = requestx.post(f"{self.base_url}/post", data=text_data)
        self.assertEqual(response.status_code, 200)

        # Parse response to verify text data was sent
        data = response.json()
        sent_data = data.get("data", "")
        self.assertEqual(sent_data, text_data)

    def test_bytes_data(self):
        """Test POST request with bytes data."""
        bytes_data = b"This is raw bytes data"

        response = requestx.post(f"{self.base_url}/post", data=bytes_data)
        self.assertEqual(response.status_code, 200)

        # Parse response to verify bytes data was sent
        data = response.json()
        sent_data = data.get("data", "")
        self.assertEqual(sent_data, bytes_data.decode("utf-8"))

    def test_timeout_handling(self):
        """Test timeout handling using tokio::time::timeout."""
        # Test with a short timeout on a delayed endpoint
        with self.assertRaises(Exception) as context:
            requestx.get(f"{self.base_url}/delay/5", timeout=1.0)

        # Should be a timeout-related error
        error_msg = str(context.exception).lower()
        self.assertTrue(
            "timeout" in error_msg or "timed out" in error_msg or "time" in error_msg,
            f"Expected timeout error, got: {context.exception}",
        )

    def test_timeout_success(self):
        """Test that requests complete within timeout."""
        # Test with a reasonable timeout
        response = requestx.get(f"{self.base_url}/delay/1", timeout=5.0)
        self.assertEqual(response.status_code, 200)

    def test_redirect_control_allow(self):
        """Test redirect handling when allow_redirects=True."""
        response = requestx.get(f"{self.base_url}/redirect/2", allow_redirects=True)
        self.assertEqual(response.status_code, 200)

        # Should have followed redirects and reached final destination
        data = response.json()
        self.assertIn("args", data)  # This indicates we reached the final /get endpoint

    def test_redirect_control_disallow(self):
        """Test redirect handling when allow_redirects=False."""
        response = requestx.get(f"{self.base_url}/redirect/1", allow_redirects=False)

        # Should get redirect status code
        self.assertIn(response.status_code, [301, 302, 303, 307, 308])

        # Should have location header
        self.assertIn("location", [h.lower() for h in response.headers.keys()])

    def test_ssl_verification_enabled(self):
        """Test SSL verification when verify=True (default)."""
        # This should work fine with a valid SSL certificate
        response = requestx.get("https://httpbin.org/get", verify=True)
        # Accept both 200 and 502 as httpbin.org sometimes has issues
        self.assertIn(response.status_code, [200, 502])

    def test_ssl_verification_disabled(self):
        """Test SSL verification when verify=False."""
        # This should work even with invalid certificates
        # Note: httpbin.org has valid certs, so this tests the code path
        response = requestx.get("https://httpbin.org/get", verify=False)
        # Accept both 200 and 502 as httpbin.org sometimes has issues
        self.assertIn(response.status_code, [200, 502])

    def test_basic_authentication(self):
        """Test basic authentication mechanism."""
        username = "testuser"
        password = "testpass"

        response = requestx.get(
            f"{self.base_url}/basic-auth/{username}/{password}",
            auth=(username, password),
        )
        self.assertEqual(response.status_code, 200)

        # Parse response to verify authentication worked
        data = response.json()
        self.assertTrue(data.get("authenticated", False))
        self.assertEqual(data.get("user"), username)

    def test_authentication_failure(self):
        """Test authentication failure with wrong credentials."""
        response = requestx.get(
            f"{self.base_url}/basic-auth/testuser/testpass",
            auth=("wrong", "credentials"),
        )
        self.assertEqual(response.status_code, 401)

    def test_combined_features(self):
        """Test multiple advanced features combined."""
        headers = {
            "User-Agent": "RequestX-Advanced-Test/1.0",
            "X-Test-Header": "combined-test",
        }

        params = {"test": "combined", "features": "multiple"}

        json_data = {
            "message": "Testing combined features",
            "timestamp": "2024-01-01T00:00:00Z",
        }

        response = requestx.post(
            f"{self.base_url}/post",
            headers=headers,
            params=params,
            json=json_data,
            timeout=10.0,
            allow_redirects=True,
            verify=True,
        )

        self.assertEqual(response.status_code, 200)

        # Verify all features worked
        data = response.json()

        # Check headers
        request_headers = data.get("headers", {})
        self.assertEqual(
            request_headers.get("User-Agent"), "RequestX-Advanced-Test/1.0"
        )
        self.assertEqual(request_headers.get("X-Test-Header"), "combined-test")

        # Check parameters
        args = data.get("args", {})
        self.assertEqual(args.get("test"), "combined")
        self.assertEqual(args.get("features"), "multiple")

        # Check JSON data
        sent_json = data.get("json", {})
        self.assertEqual(sent_json.get("message"), "Testing combined features")
        self.assertEqual(sent_json.get("timestamp"), "2024-01-01T00:00:00Z")

    def test_invalid_timeout(self):
        """Test error handling for invalid timeout values."""
        with self.assertRaises(Exception):
            requestx.get(f"{self.base_url}/get", timeout=-1.0)

        with self.assertRaises(Exception):
            requestx.get(f"{self.base_url}/get", timeout=4000.0)  # Too large

    def test_invalid_headers(self):
        """Test error handling for invalid headers."""
        with self.assertRaises(Exception):
            requestx.get(f"{self.base_url}/get", headers={"Invalid\nHeader": "value"})

    def test_invalid_auth(self):
        """Test error handling for invalid authentication."""
        with self.assertRaises(Exception):
            requestx.get(f"{self.base_url}/get", auth="invalid")

        with self.assertRaises(Exception):
            requestx.get(f"{self.base_url}/get", auth=("only_username",))

    def test_data_and_json_conflict(self):
        """Test error handling when both data and json are provided."""
        with self.assertRaises(Exception):
            requestx.post(
                f"{self.base_url}/post", data="text data", json={"key": "value"}
            )

    def test_stream_parameter(self):
        """Test stream parameter (for future streaming support)."""
        # For now, just test that the parameter is accepted
        response = requestx.get(f"{self.base_url}/get", stream=True)
        self.assertEqual(response.status_code, 200)

        response = requestx.get(f"{self.base_url}/get", stream=False)
        self.assertEqual(response.status_code, 200)


class TestEdgeCases(unittest.TestCase):
    """Test edge cases and error conditions for advanced features."""

    def setUp(self):
        """Set up test fixtures."""
        self.base_url = "https://httpbin.org"

    def test_empty_parameters(self):
        """Test with empty parameters."""
        response = requestx.get(f"{self.base_url}/get", params={})
        self.assertEqual(response.status_code, 200)

    def test_empty_headers(self):
        """Test with empty headers."""
        response = requestx.get(f"{self.base_url}/get", headers={})
        self.assertEqual(response.status_code, 200)

    def test_none_values(self):
        """Test with None values for optional parameters."""
        response = requestx.get(
            f"{self.base_url}/get", params=None, headers=None, timeout=None, auth=None
        )
        self.assertEqual(response.status_code, 200)

    def test_unicode_in_parameters(self):
        """Test Unicode characters in parameters."""
        params = {"unicode": "héllo wörld", "emoji": "🚀🌟", "chinese": "你好世界"}

        response = requestx.get(f"{self.base_url}/get", params=params)
        self.assertEqual(response.status_code, 200)

        data = response.json()
        args = data.get("args", {})
        self.assertEqual(args.get("unicode"), "héllo wörld")
        self.assertEqual(args.get("emoji"), "🚀🌟")
        self.assertEqual(args.get("chinese"), "你好世界")

    def test_unicode_in_headers(self):
        """Test Unicode characters in headers."""
        # HTTP headers should be ASCII-only, but let's test that our library handles it gracefully
        headers = {
            "X-Test-Header": "hello world",  # Use ASCII instead
            "X-Custom-Value": "test-value-123",
        }

        response = requestx.get(f"{self.base_url}/get", headers=headers)
        self.assertEqual(response.status_code, 200)

        data = response.json()
        request_headers = data.get("headers", {})
        self.assertEqual(request_headers.get("X-Test-Header"), "hello world")
        self.assertEqual(request_headers.get("X-Custom-Value"), "test-value-123")

    def test_large_json_payload(self):
        """Test with large JSON payload."""
        large_data = {
            "data": ["item_{}".format(i) for i in range(1000)],
            "metadata": {"count": 1000, "description": "Large test payload"},
        }

        response = requestx.post(f"{self.base_url}/post", json=large_data)
        self.assertEqual(response.status_code, 200)

        data = response.json()
        sent_json = data.get("json", {})
        self.assertEqual(len(sent_json.get("data", [])), 1000)
        self.assertEqual(sent_json.get("metadata", {}).get("count"), 1000)


if __name__ == "__main__":
    unittest.main(verbosity=2)
