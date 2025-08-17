#!/usr/bin/env python3
"""
Integration tests for advanced HTTP features to verify they work together correctly.
"""

import unittest
import sys
import os

# Add the project root to the path so we can import requestx
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "python"))

import requestx


class TestAdvancedIntegration(unittest.TestCase):
    """Integration tests for advanced HTTP features."""

    def setUp(self):
        """Set up test fixtures."""
        self.base_url = "https://httpbin.org"

    def test_comprehensive_post_request(self):
        """Test a comprehensive POST request with all advanced features."""
        # Test data
        json_payload = {
            "user": "test_user",
            "action": "create_resource",
            "data": {
                "name": "Test Resource",
                "description": "A test resource created by requestx",
                "tags": ["test", "requestx", "http"],
            },
            "metadata": {"timestamp": "2024-01-01T00:00:00Z", "version": "1.0"},
        }

        # Custom headers
        headers = {
            "User-Agent": "RequestX-Integration-Test/1.0",
            "X-API-Version": "v1",
            "X-Test-ID": "test-12345",  # Changed from X-Request-ID to X-Test-ID
            "Accept": "application/json",
            "Content-Type": "application/json",
        }

        # Query parameters
        params = {"format": "json", "validate": "true", "timeout": "30"}

        # Make the request with all advanced features
        response = requestx.post(
            f"{self.base_url}/post",
            json=json_payload,
            headers=headers,
            params=params,
            timeout=15.0,
            allow_redirects=True,
            verify=True,
        )

        # Verify the request was successful
        self.assertEqual(response.status_code, 200)

        # Parse and verify the response
        data = response.json()

        # Verify headers were sent correctly
        request_headers = data.get("headers", {})
        self.assertEqual(
            request_headers.get("User-Agent"), "RequestX-Integration-Test/1.0"
        )
        self.assertEqual(
            request_headers.get("X-Api-Version"), "v1"
        )  # httpbin normalizes header names
        # httpbin.org might normalize header names differently, so check for variations
        test_id = (
            request_headers.get("X-Test-Id")
            or request_headers.get("X-Test-ID")
            or request_headers.get("X-Test-id")
        )
        self.assertEqual(test_id, "test-12345")
        self.assertEqual(request_headers.get("Accept"), "application/json")

        # Verify query parameters were sent correctly
        args = data.get("args", {})
        self.assertEqual(args.get("format"), "json")
        self.assertEqual(args.get("validate"), "true")
        self.assertEqual(args.get("timeout"), "30")

        # Verify JSON payload was sent correctly
        sent_json = data.get("json", {})
        self.assertEqual(sent_json.get("user"), "test_user")
        self.assertEqual(sent_json.get("action"), "create_resource")

        # Verify nested data
        sent_data = sent_json.get("data", {})
        self.assertEqual(sent_data.get("name"), "Test Resource")
        self.assertEqual(
            sent_data.get("description"), "A test resource created by requestx"
        )
        self.assertEqual(sent_data.get("tags"), ["test", "requestx", "http"])

        # Verify metadata
        sent_metadata = sent_json.get("metadata", {})
        self.assertEqual(sent_metadata.get("timestamp"), "2024-01-01T00:00:00Z")
        self.assertEqual(sent_metadata.get("version"), "1.0")

    def test_authenticated_request_with_parameters(self):
        """Test authenticated request with query parameters and custom headers."""
        username = "testuser"
        password = "testpass"

        headers = {"X-Client-Version": "RequestX-1.0", "X-Test-Suite": "integration"}

        params = {"include_auth": "true", "format": "detailed"}

        response = requestx.get(
            f"{self.base_url}/basic-auth/{username}/{password}",
            auth=(username, password),
            headers=headers,
            params=params,
            timeout=10.0,
        )

        self.assertEqual(response.status_code, 200)

        # Verify authentication worked
        data = response.json()
        self.assertTrue(data.get("authenticated", False))
        self.assertEqual(data.get("user"), username)

    def test_form_data_with_custom_headers(self):
        """Test form data submission with custom headers."""
        form_data = {
            "username": "testuser",
            "email": "test@example.com",
            "message": "This is a test message with special chars: àáâãäå",
            "priority": "high",
            "category": "bug_report",
        }

        headers = {"X-Form-Version": "2.0", "X-Submission-ID": "form-test-001"}

        response = requestx.post(
            f"{self.base_url}/post", data=form_data, headers=headers, timeout=10.0
        )

        self.assertEqual(response.status_code, 200)

        data = response.json()

        # Verify headers
        request_headers = data.get("headers", {})
        self.assertEqual(request_headers.get("X-Form-Version"), "2.0")
        self.assertEqual(request_headers.get("X-Submission-Id"), "form-test-001")

        # Verify form data
        sent_form = data.get("form", {})
        self.assertEqual(sent_form.get("username"), "testuser")
        self.assertEqual(sent_form.get("email"), "test@example.com")
        self.assertEqual(sent_form.get("priority"), "high")
        self.assertEqual(sent_form.get("category"), "bug_report")

    def test_redirect_with_parameters(self):
        """Test redirect handling with query parameters."""
        params = {"redirect_count": "2", "preserve_params": "true"}

        # Test with redirects allowed
        response = requestx.get(
            f"{self.base_url}/redirect/2",
            params=params,
            allow_redirects=True,
            timeout=10.0,
        )

        self.assertEqual(response.status_code, 200)

        # Should have followed redirects and reached the final destination
        data = response.json()
        self.assertIn("args", data)  # This indicates we reached /get endpoint

    def test_timeout_with_large_payload(self):
        """Test timeout handling with a large JSON payload."""
        # Create a reasonably large payload
        large_payload = {
            "data": [
                {"id": i, "value": f"item_{i}", "metadata": {"index": i}}
                for i in range(100)
            ],
            "summary": {
                "total_items": 100,
                "created_at": "2024-01-01T00:00:00Z",
                "description": "Large test payload for timeout testing",
            },
        }

        response = requestx.post(
            f"{self.base_url}/post",
            json=large_payload,
            timeout=30.0,  # Generous timeout for large payload
        )

        self.assertEqual(response.status_code, 200)

        data = response.json()
        sent_json = data.get("json", {})

        # Verify the large payload was sent correctly
        self.assertEqual(len(sent_json.get("data", [])), 100)
        self.assertEqual(sent_json.get("summary", {}).get("total_items"), 100)

    def test_error_handling_with_advanced_features(self):
        """Test error handling when using advanced features."""
        # Test with invalid authentication - should get 401
        response = requestx.get(
            f"{self.base_url}/basic-auth/user/pass",
            auth=("wrong", "credentials"),
            timeout=5.0,
        )
        self.assertEqual(response.status_code, 401)

        # Test with very short timeout
        with self.assertRaises(Exception):
            requestx.get(f"{self.base_url}/delay/3", timeout=0.5)  # Very short timeout


if __name__ == "__main__":
    unittest.main(verbosity=2)
