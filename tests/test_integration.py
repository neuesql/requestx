#!/usr/bin/env python3
"""Integration tests for requests compatibility using unittest."""

import unittest
import requestx


class TestRequestsCompatibility(unittest.TestCase):
    """Test cases for requests library compatibility."""

    def test_basic_api_compatibility(self):
        """Test that basic API matches requests library."""
        # Test that all main functions exist
        self.assertTrue(hasattr(requestx, 'get'))
        self.assertTrue(hasattr(requestx, 'post'))
        self.assertTrue(hasattr(requestx, 'put'))
        self.assertTrue(hasattr(requestx, 'delete'))
        self.assertTrue(hasattr(requestx, 'head'))
        self.assertTrue(hasattr(requestx, 'options'))
        self.assertTrue(hasattr(requestx, 'patch'))
        self.assertTrue(hasattr(requestx, 'request'))
        self.assertTrue(hasattr(requestx, 'Session'))

    def test_response_api_compatibility(self):
        """Test that Response object API matches requests.Response."""
        response = requestx.get("https://httpbin.org/get")
        
        # Test that response has expected attributes
        self.assertTrue(hasattr(response, 'status_code'))
        self.assertTrue(hasattr(response, 'url'))
        self.assertTrue(hasattr(response, 'headers'))
        self.assertTrue(hasattr(response, 'text'))
        self.assertTrue(hasattr(response, 'content'))
        self.assertTrue(hasattr(response, 'json'))
        self.assertTrue(hasattr(response, 'raise_for_status'))

    def test_session_api_compatibility(self):
        """Test that Session object API matches requests.Session."""
        session = requestx.Session()
        
        # Test that session has expected methods
        self.assertTrue(hasattr(session, 'get'))
        self.assertTrue(hasattr(session, 'post'))
        self.assertTrue(hasattr(session, 'put'))
        self.assertTrue(hasattr(session, 'delete'))
        self.assertTrue(hasattr(session, 'head'))
        self.assertTrue(hasattr(session, 'options'))
        self.assertTrue(hasattr(session, 'patch'))
        self.assertTrue(hasattr(session, 'close'))

    def test_drop_in_replacement_workflow(self):
        """Test common requests library usage patterns."""
        # Test simple GET request
        response = requestx.get("https://httpbin.org/get")
        self.assertEqual(response.status_code, 200)
        
        # Test JSON response
        response = requestx.get("https://httpbin.org/json")
        data = response.json()
        self.assertIsInstance(data, dict)
        
        # Test error handling
        response = requestx.get("https://httpbin.org/status/404")
        self.assertEqual(response.status_code, 404)
        with self.assertRaises(Exception):
            response.raise_for_status()

    def test_http_methods_with_real_endpoints(self):
        """Test all HTTP methods with real endpoints."""
        # Test GET
        response = requestx.get("https://httpbin.org/get")
        self.assertEqual(response.status_code, 200)
        
        # Test POST
        response = requestx.post("https://httpbin.org/post")
        self.assertEqual(response.status_code, 200)
        
        # Test PUT
        response = requestx.put("https://httpbin.org/put")
        self.assertEqual(response.status_code, 200)
        
        # Test DELETE
        response = requestx.delete("https://httpbin.org/delete")
        self.assertEqual(response.status_code, 200)
        
        # Test PATCH
        response = requestx.patch("https://httpbin.org/patch")
        self.assertEqual(response.status_code, 200)

    def test_response_content_types(self):
        """Test handling of different response content types."""
        # Test JSON response
        response = requestx.get("https://httpbin.org/json")
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertIsInstance(json_data, dict)
        
        # Test HTML response
        response = requestx.get("https://httpbin.org/html")
        self.assertEqual(response.status_code, 200)
        text = response.text
        self.assertIn("<html>", text.lower())
        
        # Test XML response
        response = requestx.get("https://httpbin.org/xml")
        self.assertEqual(response.status_code, 200)
        text = response.text
        self.assertIn("<?xml", text)

    def test_status_code_handling(self):
        """Test handling of various HTTP status codes."""
        # Test 200 OK
        response = requestx.get("https://httpbin.org/status/200")
        self.assertEqual(response.status_code, 200)
        
        # Test 201 Created
        response = requestx.get("https://httpbin.org/status/201")
        self.assertEqual(response.status_code, 201)
        
        # Test 400 Bad Request
        response = requestx.get("https://httpbin.org/status/400")
        self.assertEqual(response.status_code, 400)
        
        # Test 404 Not Found
        response = requestx.get("https://httpbin.org/status/404")
        self.assertEqual(response.status_code, 404)
        
        # Test 500 Internal Server Error
        response = requestx.get("https://httpbin.org/status/500")
        self.assertEqual(response.status_code, 500)


class TestAsyncSyncBehavior(unittest.TestCase):
    """Test cases for async/sync behavior compatibility."""

    def test_synchronous_execution(self):
        """Test that requests execute synchronously by default."""
        import time
        
        start_time = time.time()
        response = requestx.get("https://httpbin.org/delay/1")
        end_time = time.time()
        
        # Should take at least 1 second due to delay
        self.assertGreaterEqual(end_time - start_time, 1.0)
        self.assertEqual(response.status_code, 200)

    def test_multiple_requests_blocking(self):
        """Test that multiple requests execute in sequence (blocking)."""
        import time
        
        start_time = time.time()
        
        # Make two requests that each take ~0.5 seconds
        response1 = requestx.get("https://httpbin.org/delay/0.5")
        response2 = requestx.get("https://httpbin.org/delay/0.5")
        
        end_time = time.time()
        
        # Should take at least 1 second total (sequential execution)
        self.assertGreaterEqual(end_time - start_time, 1.0)
        self.assertEqual(response1.status_code, 200)
        self.assertEqual(response2.status_code, 200)


if __name__ == '__main__':
    # Run the tests
    unittest.main(verbosity=2)