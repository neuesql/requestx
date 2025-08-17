#!/usr/bin/env python3
"""Unit tests for the core HTTP client functionality using unittest."""

import unittest
import requestx


class TestModuleImport(unittest.TestCase):
    """Test cases for module import and basic functionality."""

    def test_module_import(self):
        """Test that we can import the module successfully."""
        # If we get here, the import worked
        self.assertTrue(hasattr(requestx, "get"))
        self.assertTrue(hasattr(requestx, "post"))
        self.assertTrue(hasattr(requestx, "put"))
        self.assertTrue(hasattr(requestx, "delete"))
        self.assertTrue(hasattr(requestx, "head"))
        self.assertTrue(hasattr(requestx, "options"))
        self.assertTrue(hasattr(requestx, "patch"))
        self.assertTrue(hasattr(requestx, "request"))

    def test_session_object_creation(self):
        """Test that Session objects can be created."""
        session = requestx.Session()
        self.assertIsNotNone(session)


class TestHTTPMethods(unittest.TestCase):
    """Test cases for HTTP method functionality."""

    def test_get_request(self):
        """Test basic GET request functionality."""
        response = requestx.get("https://httpbin.org/get")
        self.assertEqual(response.status_code, 200)
        self.assertIsNotNone(response.url)
        self.assertIsInstance(response.headers, dict)

        # Test that we can access response content
        text = response.text
        self.assertIsInstance(text, str)
        self.assertGreater(len(text), 0)

    def test_post_request(self):
        """Test basic POST request functionality."""
        response = requestx.post("https://httpbin.org/post")
        self.assertEqual(response.status_code, 200)
        self.assertIsNotNone(response.url)

    def test_put_request(self):
        """Test basic PUT request functionality."""
        response = requestx.put("https://httpbin.org/put")
        self.assertEqual(response.status_code, 200)
        self.assertIsNotNone(response.url)

    def test_delete_request(self):
        """Test basic DELETE request functionality."""
        response = requestx.delete("https://httpbin.org/delete")
        self.assertEqual(response.status_code, 200)
        self.assertIsNotNone(response.url)

    def test_head_request(self):
        """Test basic HEAD request functionality."""
        response = requestx.head("https://httpbin.org/get")
        self.assertEqual(response.status_code, 200)
        self.assertIsNotNone(response.url)
        # HEAD requests should have empty body
        self.assertEqual(len(response.text), 0)

    def test_options_request(self):
        """Test basic OPTIONS request functionality."""
        response = requestx.options("https://httpbin.org/get")
        # OPTIONS requests typically return 200 or 204
        self.assertIn(response.status_code, [200, 204])
        self.assertIsNotNone(response.url)

    def test_patch_request(self):
        """Test basic PATCH request functionality."""
        response = requestx.patch("https://httpbin.org/patch")
        self.assertEqual(response.status_code, 200)
        self.assertIsNotNone(response.url)

    def test_generic_request_method(self):
        """Test the generic request method with different HTTP methods."""
        # Test GET via generic request method
        response = requestx.request("GET", "https://httpbin.org/get")
        self.assertEqual(response.status_code, 200)

        # Test POST via generic request method
        response = requestx.request("POST", "https://httpbin.org/post")
        self.assertEqual(response.status_code, 200)


class TestResponseObject(unittest.TestCase):
    """Test cases for Response object functionality."""

    def test_response_object_properties(self):
        """Test that Response objects have the expected properties."""
        response = requestx.get("https://httpbin.org/get")

        # Test status_code property
        self.assertIsInstance(response.status_code, int)
        self.assertEqual(response.status_code, 200)

        # Test url property
        self.assertIsInstance(response.url, str)
        self.assertTrue(response.url.startswith("https://"))

        # Test headers property
        self.assertIsInstance(response.headers, dict)
        self.assertGreater(len(response.headers), 0)

        # Test text property
        text = response.text
        self.assertIsInstance(text, str)
        self.assertGreater(len(text), 0)

        # Test content property
        content = response.content
        self.assertIsNotNone(content)

    def test_json_response_parsing(self):
        """Test JSON response parsing functionality."""
        response = requestx.get("https://httpbin.org/json")
        self.assertEqual(response.status_code, 200)

        # Test that we can parse JSON
        json_data = response.json()
        self.assertIsInstance(json_data, dict)

    def test_error_handling(self):
        """Test error handling for HTTP error status codes."""
        # Test 404 error
        response = requestx.get("https://httpbin.org/status/404")
        self.assertEqual(response.status_code, 404)

        # Test raise_for_status method
        with self.assertRaises(Exception):
            response.raise_for_status()


class TestErrorHandling(unittest.TestCase):
    """Test cases for error handling in the HTTP client."""

    def test_invalid_url(self):
        """Test that invalid URLs raise appropriate errors."""
        with self.assertRaises(Exception):
            requestx.get("not-a-valid-url")

    def test_invalid_method_error(self):
        """Test handling of invalid HTTP methods."""
        with self.assertRaises(Exception):
            requestx.request("INVALID_METHOD", "https://httpbin.org/get")

    def test_network_error_handling(self):
        """Test handling of network errors."""
        # Test connection to non-existent domain
        with self.assertRaises(Exception):
            requestx.get("https://this-domain-does-not-exist-12345.com")


if __name__ == "__main__":
    # Run the tests
    unittest.main(verbosity=2)
