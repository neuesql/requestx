"""Tests for synchronous HTTP client functionality."""

import pytest
import requestx
from requestx import Client, Headers, Cookies, Timeout, Proxy, Auth


class TestModuleLevelFunctions:
    """Test module-level convenience functions."""

    def test_get_request(self):
        """Test basic GET request."""
        response = requestx.get("https://httpbin.org/get")
        assert response.status_code == 200
        assert response.is_success
        data = response.json()
        assert "url" in data

    def test_post_json(self):
        """Test POST request with JSON body."""
        response = requestx.post(
            "https://httpbin.org/post",
            json={"key": "value", "number": 42}
        )
        assert response.status_code == 200
        data = response.json()
        assert data["json"] == {"key": "value", "number": 42}

    def test_post_form_data(self):
        """Test POST request with form data."""
        response = requestx.post(
            "https://httpbin.org/post",
            data={"field1": "value1", "field2": "value2"}
        )
        assert response.status_code == 200
        data = response.json()
        assert data["form"]["field1"] == "value1"

    def test_custom_headers(self):
        """Test request with custom headers."""
        response = requestx.get(
            "https://httpbin.org/headers",
            headers={"X-Custom-Header": "test-value"}
        )
        assert response.status_code == 200
        data = response.json()
        assert data["headers"]["X-Custom-Header"] == "test-value"

    def test_query_params(self):
        """Test request with query parameters."""
        response = requestx.get(
            "https://httpbin.org/get",
            params={"foo": "bar", "baz": "qux"}
        )
        assert response.status_code == 200
        data = response.json()
        assert data["args"]["foo"] == "bar"
        assert data["args"]["baz"] == "qux"

    def test_put_request(self):
        """Test PUT request."""
        response = requestx.put(
            "https://httpbin.org/put",
            json={"updated": True}
        )
        assert response.status_code == 200
        data = response.json()
        assert data["json"]["updated"] is True

    def test_patch_request(self):
        """Test PATCH request."""
        response = requestx.patch(
            "https://httpbin.org/patch",
            json={"patched": True}
        )
        assert response.status_code == 200

    def test_delete_request(self):
        """Test DELETE request."""
        response = requestx.delete("https://httpbin.org/delete")
        assert response.status_code == 200

    def test_head_request(self):
        """Test HEAD request."""
        response = requestx.head("https://httpbin.org/get")
        assert response.status_code == 200
        # HEAD should not have a body
        assert len(response.content) == 0

    def test_options_request(self):
        """Test OPTIONS request."""
        response = requestx.options("https://httpbin.org/get")
        assert response.status_code == 200


class TestClient:
    """Test Client class."""

    def test_client_context_manager(self):
        """Test client as context manager."""
        with Client() as client:
            response = client.get("https://httpbin.org/get")
            assert response.status_code == 200

    def test_client_base_url(self):
        """Test client with base URL."""
        with Client(base_url="https://httpbin.org") as client:
            response = client.get("/get")
            assert response.status_code == 200

    def test_client_default_headers(self):
        """Test client with default headers."""
        with Client(headers={"X-Default": "header-value"}) as client:
            response = client.get("https://httpbin.org/headers")
            data = response.json()
            assert data["headers"]["X-Default"] == "header-value"

    def test_client_multiple_requests(self):
        """Test multiple requests with same client."""
        with Client() as client:
            r1 = client.get("https://httpbin.org/get")
            r2 = client.post("https://httpbin.org/post", json={"test": 1})
            r3 = client.get("https://httpbin.org/get")

            assert r1.status_code == 200
            assert r2.status_code == 200
            assert r3.status_code == 200


class TestResponse:
    """Test Response class."""

    def test_response_attributes(self):
        """Test response attributes."""
        response = requestx.get("https://httpbin.org/get")

        assert isinstance(response.status_code, int)
        assert isinstance(response.url, str)
        assert isinstance(response.headers, Headers)
        assert hasattr(response, "content")
        assert hasattr(response, "text")
        assert hasattr(response, "elapsed")

    def test_response_json(self):
        """Test JSON response parsing."""
        response = requestx.get("https://httpbin.org/json")
        data = response.json()
        assert isinstance(data, dict)

    def test_response_text(self):
        """Test text response."""
        response = requestx.get("https://httpbin.org/html")
        text = response.text
        assert isinstance(text, str)
        assert "html" in text.lower()

    def test_response_status_checks(self):
        """Test response status check methods."""
        response = requestx.get("https://httpbin.org/get")
        assert response.is_success
        assert not response.is_redirect
        assert not response.is_client_error
        assert not response.is_server_error
        assert not response.is_error

    def test_response_404(self):
        """Test 404 response."""
        response = requestx.get("https://httpbin.org/status/404")
        assert response.status_code == 404
        assert response.is_client_error
        assert response.is_error
        assert not response.is_success

    def test_raise_for_status(self):
        """Test raise_for_status method."""
        response = requestx.get("https://httpbin.org/status/500")
        with pytest.raises(Exception):
            response.raise_for_status()

    def test_response_bool(self):
        """Test response boolean conversion."""
        success = requestx.get("https://httpbin.org/get")
        error = requestx.get("https://httpbin.org/status/404")

        assert bool(success) is True
        assert bool(error) is False


class TestHeaders:
    """Test Headers class."""

    def test_headers_creation(self):
        """Test Headers creation."""
        headers = Headers({"Content-Type": "application/json"})
        assert headers.get("content-type") == "application/json"

    def test_headers_case_insensitive(self):
        """Test headers are case-insensitive."""
        headers = Headers({"Content-Type": "application/json"})
        assert headers.get("content-type") == "application/json"
        assert headers.get("CONTENT-TYPE") == "application/json"

    def test_headers_set_get(self):
        """Test setting and getting headers."""
        headers = Headers()
        headers.set("X-Custom", "value")
        assert headers.get("x-custom") == "value"


class TestCookies:
    """Test Cookies class."""

    def test_cookies_creation(self):
        """Test Cookies creation."""
        cookies = Cookies({"session": "abc123"})
        assert cookies.get("session") == "abc123"

    def test_cookies_set_get(self):
        """Test setting and getting cookies."""
        cookies = Cookies()
        cookies.set("token", "xyz")
        assert cookies.get("token") == "xyz"


class TestTimeout:
    """Test Timeout class."""

    def test_timeout_creation(self):
        """Test Timeout creation."""
        timeout = Timeout(timeout=30.0, connect=5.0)
        assert timeout.total_timeout == 30.0
        assert timeout.connect_timeout == 5.0


class TestAuth:
    """Test Auth class."""

    def test_basic_auth(self):
        """Test basic authentication."""
        response = requestx.get(
            "https://httpbin.org/basic-auth/user/pass",
            auth=Auth.basic("user", "pass")
        )
        assert response.status_code == 200

    def test_bearer_auth(self):
        """Test bearer token authentication."""
        response = requestx.get(
            "https://httpbin.org/bearer",
            auth=Auth.bearer("test-token")
        )
        assert response.status_code == 200


class TestRedirects:
    """Test redirect handling."""

    def test_follow_redirects(self):
        """Test that redirects are followed by default."""
        response = requestx.get("https://httpbin.org/redirect/2")
        assert response.status_code == 200
        assert "httpbin.org/get" in response.url

    def test_no_follow_redirects(self):
        """Test disabling redirect following."""
        response = requestx.get(
            "https://httpbin.org/redirect/1",
            follow_redirects=False
        )
        assert response.status_code == 302


class TestProxy:
    """Test proxy configuration."""

    def test_proxy_creation(self):
        """Test Proxy creation."""
        proxy = Proxy(url="http://proxy.example.com:8080")
        assert proxy.http_proxy == "http://proxy.example.com:8080"
        assert proxy.https_proxy == "http://proxy.example.com:8080"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
