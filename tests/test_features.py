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

        # Test GET with query parameters using /anything endpoint
        response = requestx.get("https://httpbin.org/anything", params={"key": "value", "test": "data"})
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertEqual(json_data["args"]["key"], "value")
        self.assertEqual(json_data["args"]["test"], "data")

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

        # Test PUT with JSON data using /anything endpoint
        json_payload = {"updated": True, "id": 123}
        response = requestx.put("https://httpbin.org/anything", json=json_payload)
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertEqual(json_data["json"], json_payload)

    def test_delete_request(self):
        """Test basic DELETE request functionality."""
        response = requestx.delete("https://httpbin.org/delete")
        self.assertEqual(response.status_code, 200)
        self.assertIsNotNone(response.url)

        # Test DELETE with query parameters using /anything endpoint
        response = requestx.delete("https://httpbin.org/anything", params={"resource": "123"})
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertEqual(json_data["args"]["resource"], "123")

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

        # Test OPTIONS with /anything endpoint
        response = requestx.options("https://httpbin.org/anything")
        self.assertIn(response.status_code, [200, 204])

    def test_patch_request(self):
        """Test basic PATCH request functionality."""
        response = requestx.patch("https://httpbin.org/patch")
        self.assertEqual(response.status_code, 200)
        self.assertIsNotNone(response.url)

        # Test PATCH with JSON data using /anything endpoint
        json_payload = {"partial": "update", "field": "value"}
        response = requestx.patch("https://httpbin.org/anything", json=json_payload)
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertEqual(json_data["json"], json_payload)

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

        # Test response from /get endpoint includes expected fields
        json_data = response.json()
        self.assertIn("url", json_data)
        self.assertIn("headers", json_data)
        self.assertIn("origin", json_data)

    def test_json_response_parsing(self):
        """Test JSON response parsing functionality."""
        response = requestx.get("https://httpbin.org/json")
        self.assertEqual(response.status_code, 200)

        # Test that we can parse JSON
        json_data = response.json()
        self.assertIsInstance(json_data, dict)
        self.assertIn("slideshow", json_data)

        # Test JSON parsing from /get endpoint
        response = requestx.get("https://httpbin.org/get")
        json_data = response.json()
        self.assertIn("args", json_data)
        self.assertIn("headers", json_data)
        self.assertIn("url", json_data)

    def test_error_handling(self):
        """Test error handling for HTTP error status codes."""
        # Test 404 error
        response = requestx.get("https://httpbin.org/status/404")
        self.assertEqual(response.status_code, 404)

        # Test raise_for_status method
        with self.assertRaises(Exception):  # noqa: B017 - Expected exception for test
            response.raise_for_status()

        # Test various error status codes
        for status_code in [400, 401, 403, 404, 500, 502, 503]:
            response = requestx.get(f"https://httpbin.org/status/{status_code}")
            self.assertEqual(response.status_code, status_code)
            if status_code >= 400:
                with self.assertRaises(Exception):
                    response.raise_for_status()


class TestHTTPHeadersAndParams(unittest.TestCase):
    """Test cases for HTTP headers and query parameters."""

    def test_custom_headers(self):
        """Test custom headers functionality."""
        headers = {"User-Agent": "requestx-test", "X-Custom-Header": "test-value"}
        response = requestx.get("https://httpbin.org/headers", headers=headers)
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertEqual(json_data["headers"]["User-Agent"], "requestx-test")
        self.assertEqual(json_data["headers"]["X-Custom-Header"], "test-value")

    def test_query_parameters(self):
        """Test query parameter functionality."""
        params = {"param1": "value1", "param2": "value2"}
        response = requestx.get("https://httpbin.org/get", params=params)
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertEqual(json_data["args"]["param1"], "value1")
        self.assertEqual(json_data["args"]["param2"], "value2")

    def test_user_agent_header(self):
        """Test User-Agent header functionality."""
        response = requestx.get("https://httpbin.org/user-agent", 
                               headers={"User-Agent": "requestx/1.0"})
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertEqual(json_data["user-agent"], "requestx/1.0")


class TestHTTPBinSpecificFeatures(unittest.TestCase):
    """Test cases for httpbin.org specific features."""

    def test_anything_endpoint(self):
        """Test /anything endpoint functionality."""
        # Test with different HTTP methods
        for method in ["GET", "POST", "PUT", "DELETE", "PATCH"]:
            response = requestx.request(method, "https://httpbin.org/anything/test/path")
            self.assertEqual(response.status_code, 200)
            json_data = response.json()
            self.assertEqual(json_data["method"], method)
            self.assertEqual(json_data["url"], "https://httpbin.org/anything/test/path")

    def test_anything_with_path_params(self):
        """Test /anything/{anything} endpoint with path parameters."""
        response = requestx.get("https://httpbin.org/anything/custom/path/123")
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertEqual(json_data["url"], "https://httpbin.org/anything/custom/path/123")

    def test_absolute_redirect(self):
        """Test /absolute-redirect/{n} endpoint."""
        response = requestx.get("https://httpbin.org/absolute-redirect/3", allow_redirects=False)
        self.assertEqual(response.status_code, 302)
        self.assertIn("Location", response.headers)

    def test_base64_endpoint(self):
        """Test /base64/{value} endpoint."""
        # Test with base64 encoded string "hello"
        import base64
        encoded = base64.urlsafe_b64encode(b"hello").decode().rstrip("=")
        response = requestx.get(f"https://httpbin.org/base64/{encoded}")
        self.assertEqual(response.status_code, 200)
        self.assertEqual(response.text.strip(), "hello")

    def test_basic_auth(self):
        """Test /basic-auth/{user}/{passwd} endpoint."""
        response = requestx.get("https://httpbin.org/basic-auth/user/pass", 
                               auth=("user", "pass"))
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertEqual(json_data["authenticated"], True)
        self.assertEqual(json_data["user"], "user")

    def test_bearer_auth(self):
        """Test /bearer endpoint."""
        token = "test-token-123"
        response = requestx.get("https://httpbin.org/bearer", 
                               headers={"Authorization": f"Bearer {token}"})
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertEqual(json_data["authenticated"], True)
        self.assertEqual(json_data["token"], token)

    def test_brotli_endpoint(self):
        """Test /brotli endpoint."""
        response = requestx.get("https://httpbin.org/brotli")
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertEqual(json_data["brotli"], True)

    def test_bytes_endpoint(self):
        """Test /bytes/{n} endpoint."""
        n = 10
        response = requestx.get(f"https://httpbin.org/bytes/{n}")
        self.assertEqual(response.status_code, 200)
        self.assertEqual(len(response.content), n)

    def test_cache_endpoint(self):
        """Test /cache endpoint."""
        response = requestx.get("https://httpbin.org/cache")
        self.assertEqual(response.status_code, 200)
        self.assertIn("Cache-Control", response.headers)

    def test_cache_with_duration(self):
        """Test /cache/{value} endpoint."""
        duration = 60
        response = requestx.get(f"https://httpbin.org/cache/{duration}")
        self.assertEqual(response.status_code, 200)
        self.assertIn("Cache-Control", response.headers)

    def test_cookies_endpoint(self):
        """Test /cookies endpoint functionality."""
        # Test setting cookies via headers
        response = requestx.get("https://httpbin.org/cookies", 
                               headers={"Cookie": "test=value; session=abc123"})
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertEqual(json_data["cookies"]["test"], "value")
        self.assertEqual(json_data["cookies"]["session"], "abc123")

    def test_cookies_set_endpoint(self):
        """Test /cookies/set endpoint."""
        response = requestx.get("https://httpbin.org/cookies/set?name=test&value=cookie123")
        self.assertEqual(response.status_code, 200)
        self.assertIn("Set-Cookie", response.headers)

    def test_cookies_delete_endpoint(self):
        """Test /cookies/delete endpoint."""
        response = requestx.get("https://httpbin.org/cookies/delete?name=test")
        self.assertEqual(response.status_code, 200)

    def test_deflate_endpoint(self):
        """Test /deflate endpoint."""
        response = requestx.get("https://httpbin.org/deflate")
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertEqual(json_data["deflated"], True)

    def test_delay_endpoint(self):
        """Test /delay/{delay} endpoint."""
        # Test small delay (1 second)
        import time
        start_time = time.time()
        response = requestx.get("https://httpbin.org/delay/1")
        end_time = time.time()
        self.assertEqual(response.status_code, 200)
        self.assertGreaterEqual(end_time - start_time, 1.0)

    def test_delete_endpoint(self):
        """Test /delete endpoint."""
        response = requestx.delete("https://httpbin.org/delete")
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertEqual(json_data["method"], "DELETE")

    def test_deny_endpoint(self):
        """Test /deny endpoint."""
        response = requestx.get("https://httpbin.org/deny")
        self.assertEqual(response.status_code, 200)
        self.assertIn("DENIED", response.text)

    def test_digest_auth(self):
        """Test /digest-auth endpoint."""
        # Note: Digest auth requires proper implementation, testing 401 case
        response = requestx.get("https://httpbin.org/digest-auth/auth/user/pass")
        self.assertEqual(response.status_code, 401)

    def test_encoding_utf8(self):
        """Test /encoding/utf8 endpoint."""
        response = requestx.get("https://httpbin.org/encoding/utf8")
        self.assertEqual(response.status_code, 200)
        self.assertIn("text/html", response.headers.get("Content-Type"))

    def test_etag_endpoint(self):
        """Test /etag/{etag} endpoint."""
        etag = "test-etag-123"
        response = requestx.get(f"https://httpbin.org/etag/{etag}")
        self.assertEqual(response.status_code, 200)
        self.assertEqual(response.headers.get("ETag"), f'"{etag}"')

    def test_get_endpoint(self):
        """Test /get endpoint."""
        response = requestx.get("https://httpbin.org/get")
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertEqual(json_data["method"], "GET")

    def test_gzip_endpoint(self):
        """Test /gzip endpoint."""
        response = requestx.get("https://httpbin.org/gzip")
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertEqual(json_data["gzipped"], True)

    def test_headers_endpoint(self):
        """Test /headers endpoint."""
        custom_headers = {"X-Test": "value", "X-Another": "test"}
        response = requestx.get("https://httpbin.org/headers", headers=custom_headers)
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertEqual(json_data["headers"]["X-Test"], "value")

    def test_html_endpoint(self):
        """Test /html endpoint."""
        response = requestx.get("https://httpbin.org/html")
        self.assertEqual(response.status_code, 200)
        self.assertIn("text/html", response.headers.get("Content-Type"))

    def test_ip_endpoint(self):
        """Test /ip endpoint."""
        response = requestx.get("https://httpbin.org/ip")
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertIn("origin", json_data)

    def test_json_endpoint(self):
        """Test /json endpoint."""
        response = requestx.get("https://httpbin.org/json")
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertIn("slideshow", json_data)

    def test_patch_endpoint(self):
        """Test /patch endpoint."""
        response = requestx.patch("https://httpbin.org/patch")
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertEqual(json_data["method"], "PATCH")

    def test_post_endpoint(self):
        """Test /post endpoint."""
        response = requestx.post("https://httpbin.org/post")
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertEqual(json_data["method"], "POST")

    def test_put_endpoint(self):
        """Test /put endpoint."""
        response = requestx.put("https://httpbin.org/put")
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertEqual(json_data["method"], "PUT")

    def test_redirect_endpoint(self):
        """Test redirect endpoints."""
        response = requestx.get("https://httpbin.org/redirect/3", allow_redirects=False)
        self.assertEqual(response.status_code, 302)
        self.assertIn("Location", response.headers)

    def test_relative_redirect(self):
        """Test /relative-redirect/{n} endpoint."""
        response = requestx.get("https://httpbin.org/relative-redirect/2", allow_redirects=False)
        self.assertEqual(response.status_code, 302)

    def test_response_headers(self):
        """Test custom response headers."""
        response = requestx.get("https://httpbin.org/response-headers", 
                               params={"X-Custom": "value", "Content-Type": "application/json"})
        self.assertEqual(response.status_code, 200)
        self.assertEqual(response.headers.get("X-Custom"), "value")

    def test_status_codes(self):
        """Test various HTTP status codes."""
        for status_code in [200, 201, 400, 404, 500]:
            response = requestx.get(f"https://httpbin.org/status/{status_code}")
            self.assertEqual(response.status_code, status_code)

    def test_user_agent(self):
        """Test /user-agent endpoint."""
        ua = "requestx-test/1.0"
        response = requestx.get("https://httpbin.org/user-agent", headers={"User-Agent": ua})
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertEqual(json_data["user-agent"], ua)

    def test_uuid_endpoint(self):
        """Test /uuid endpoint."""
        response = requestx.get("https://httpbin.org/uuid")
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertIn("uuid", json_data)
        # Validate UUID format
        import uuid
        uuid.UUID(json_data["uuid"])

    def test_xml_endpoint(self):
        """Test /xml endpoint."""
        response = requestx.get("https://httpbin.org/xml")
        self.assertEqual(response.status_code, 200)
        self.assertIn("application/xml", response.headers.get("Content-Type"))

    def test_forms_post_endpoint(self):
        """Test form data handling with /post endpoint."""
        form_data = {"name": "test", "value": "123"}
        response = requestx.post("https://httpbin.org/post", data=form_data)
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertEqual(json_data["form"]["name"], "test")
        self.assertEqual(json_data["form"]["value"], "123")

    def test_redirect_to_endpoint(self):
        """Test /redirect-to endpoint."""
        target_url = "https://httpbin.org/get"
        response = requestx.get("https://httpbin.org/redirect-to", 
                               params={"url": target_url}, 
                               allow_redirects=False)
        self.assertEqual(response.status_code, 302)
        self.assertEqual(response.headers.get("Location"), target_url)

    def test_hidden_basic_auth(self):
        """Test /hidden-basic-auth endpoint."""
        response = requestx.get("https://httpbin.org/hidden-basic-auth/user/pass", 
                               auth=("user", "pass"))
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertEqual(json_data["authenticated"], True)

    def test_query_parameter_handling(self):
        """Test comprehensive query parameter handling."""
        params = {
            "param1": "value1",
            "param2": "value2",
            "param3": ["item1", "item2"]
        }
        response = requestx.get("https://httpbin.org/get", params=params)
        self.assertEqual(response.status_code, 200)
        json_data = response.json()
        self.assertEqual(json_data["args"]["param1"], "value1")
        self.assertEqual(json_data["args"]["param2"], "value2")
        self.assertEqual(json_data["args"]["param3"], ["item1", "item2"])


class TestImageEndpoints(unittest.TestCase):
    """Test cases for image endpoints."""

    def test_image_png(self):
        """Test /image/png endpoint."""
        response = requestx.get("https://httpbin.org/image/png")
        self.assertEqual(response.status_code, 200)
        self.assertEqual(response.headers.get("Content-Type"), "image/png")

    def test_image_jpeg(self):
        """Test /image/jpeg endpoint."""
        response = requestx.get("https://httpbin.org/image/jpeg")
        self.assertEqual(response.status_code, 200)
        self.assertEqual(response.headers.get("Content-Type"), "image/jpeg")

    def test_image_webp(self):
        """Test /image/webp endpoint."""
        response = requestx.get("https://httpbin.org/image/webp")
        self.assertEqual(response.status_code, 200)
        self.assertEqual(response.headers.get("Content-Type"), "image/webp")

    def test_image_svg(self):
        """Test /image/svg endpoint."""
        response = requestx.get("https://httpbin.org/image/svg")
        self.assertEqual(response.status_code, 200)
        self.assertEqual(response.headers.get("Content-Type"), "image/svg+xml")

    def test_links_endpoints(self):
        """Test /links/{n}/{offset} endpoint."""
        response = requestx.get("https://httpbin.org/links/10/5")
        self.assertEqual(response.status_code, 200)
        self.assertIn("text/html", response.headers.get("Content-Type"))

    def test_stream_endpoint(self):
        """Test /stream/{n} endpoint."""
        n = 5
        response = requestx.get(f"https://httpbin.org/stream/{n}")
        self.assertEqual(response.status_code, 200)
        
        # Parse streaming response
        lines = response.text.strip().split('\n')
        # Should have n JSON objects
        self.assertGreaterEqual(len(lines), n)

    def test_stream_bytes_endpoint(self):
        """Test /stream-bytes/{n} endpoint."""
        n = 100
        response = requestx.get(f"https://httpbin.org/stream-bytes/{n}")
        self.assertEqual(response.status_code, 200)
        # Total content should be n bytes
        self.assertEqual(len(response.content), n)

    def test_drip_endpoint(self):
        """Test /drip endpoint."""
        duration = 1
        numbytes = 10
        response = requestx.get(f"https://httpbin.org/drip?duration={duration}&numbytes={numbytes}")
        self.assertEqual(response.status_code, 200)
        self.assertEqual(len(response.content), numbytes)

    def test_robots_txt(self):
        """Test /robots.txt endpoint."""
        response = requestx.get("https://httpbin.org/robots.txt")
        self.assertEqual(response.status_code, 200)
        self.assertIn("text/plain", response.headers.get("Content-Type"))


class TestErrorHandling(unittest.TestCase):
    """Test cases for error handling in the HTTP client."""

    def test_invalid_url(self):
        """Test that invalid URLs raise appropriate errors."""
        with self.assertRaises(Exception):  # noqa: B017 - Expected exception for test
            requestx.get("not-a-valid-url")

    def test_invalid_method_error(self):
        """Test handling of invalid HTTP methods."""
        with self.assertRaises(Exception):  # noqa: B017 - Expected exception for test
            requestx.request("INVALID_METHOD", "https://httpbin.org/get")

    def test_network_error_handling(self):
        """Test handling of network errors."""
        # Test connection to non-existent domain
        with self.assertRaises(Exception):  # noqa: B017 - Expected exception for test
            requestx.get("https://this-domain-does-not-exist-12345.com")


if __name__ == "__main__":
    # Run the tests
    unittest.main(verbosity=2)
