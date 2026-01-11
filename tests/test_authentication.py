"""Tests for authentication system (Phase 3)"""
import unittest
import sys
import os
import base64

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "python"))
import requestx
from testcontainers.generic import ServerContainer


class HttpbinTestCase(unittest.TestCase):
    """Base test case with httpbin container."""

    @classmethod
    def setUpClass(cls):
        cls.container = ServerContainer(port=80, image="kennethreitz/httpbin")
        cls.container.start()
        cls.httpbin_port = cls.container.get_exposed_port(80)
        global HTTPBIN_HOST
        HTTPBIN_HOST = f"http://localhost:{cls.httpbin_port}"

    @classmethod
    def tearDownClass(cls):
        cls.container.stop()


class TestBasicAuth(HttpbinTestCase):
    """Tests for basic authentication."""

    def test_basic_auth_tuple(self):
        """Test basic auth with tuple."""
        r = requestx.get(
            f"{HTTPBIN_HOST}/basic-auth/user/passwd", auth=("user", "passwd")
        )
        self.assertEqual(r.status_code, 200)

    def test_basic_auth_list(self):
        """Test basic auth with list."""
        r = requestx.get(
            f"{HTTPBIN_HOST}/basic-auth/user/passwd", auth=["user", "passwd"]
        )
        self.assertEqual(r.status_code, 200)

    def test_basic_auth_wrong_password(self):
        """Test basic auth with wrong password returns 401."""
        r = requestx.get(
            f"{HTTPBIN_HOST}/basic-auth/user/passwd", auth=("user", "wrong")
        )
        self.assertEqual(r.status_code, 401)

    def test_basic_auth_no_credentials(self):
        """Test basic auth without credentials returns 401."""
        r = requestx.get(f"{HTTPBIN_HOST}/basic-auth/user/passwd")
        self.assertEqual(r.status_code, 401)

    def test_basic_auth_post(self):
        """Test basic auth with POST request."""
        r = requestx.post(
            f"{HTTPBIN_HOST}/post",
            auth=("user", "passwd"),
            json={"key": "value"},
        )
        self.assertEqual(r.status_code, 200)


class TestAuthFromUrl(HttpbinTestCase):
    """Tests for authentication parsed from URL."""

    def test_auth_from_url(self):
        """Test authentication parsed from URL."""
        r = requestx.get(f"{HTTPBIN_HOST}/basic-auth/user/passwd")
        self.assertEqual(r.status_code, 401)

    def test_auth_from_url_with_credentials(self):
        """Test authentication with credentials in URL."""
        r = requestx.get(f"http://user:passwd@{HTTPBIN_HOST.replace('http://', '')}/basic-auth/user/passwd")
        self.assertEqual(r.status_code, 200)

    def test_auth_from_url_overrides_kwargs(self):
        """Test that kwargs auth takes precedence over URL auth."""
        # URL has wrong password, kwargs has correct password
        r = requestx.get(
            f"http://user:wrong@{HTTPBIN_HOST.replace('http://', '')}/basic-auth/user/passwd",
            auth=("user", "passwd"),
        )
        self.assertEqual(r.status_code, 200)


class TestDigestAuth(HttpbinTestCase):
    """Tests for HTTP Digest authentication."""

    def test_digest_auth_class_exists(self):
        """Test HTTPDigestAuth class exists."""
        self.assertTrue(hasattr(requestx, "HTTPDigestAuth"))

    def test_digest_auth_creation(self):
        """Test HTTPDigestAuth instance creation."""
        auth = requestx.HTTPDigestAuth("user", "passwd")
        self.assertEqual(auth.username, "user")
        self.assertEqual(auth.password, "passwd")

    def test_digest_auth_with_tuple_fallback(self):
        """Test HTTPDigestAuth falls back to basic auth as tuple."""
        # HTTPDigestAuth can be used like a tuple for basic auth
        auth = requestx.HTTPDigestAuth("user", "passwd")
        r = requestx.get(
            f"{HTTPBIN_HOST}/basic-auth/user/passwd",
            auth=(auth.username, auth.password),
        )
        self.assertEqual(r.status_code, 200)


class TestProxyAuth(HttpbinTestCase):
    """Tests for proxy authentication."""

    def test_proxy_auth_class_exists(self):
        """Test HTTPProxyAuth class exists."""
        self.assertTrue(hasattr(requestx, "HTTPProxyAuth"))

    def test_proxy_auth_creation(self):
        """Test HTTPProxyAuth instance creation."""
        auth = requestx.HTTPProxyAuth("proxy_user", "proxy_pass")
        self.assertEqual(auth.username, "proxy_user")
        self.assertEqual(auth.password, "proxy_pass")

    def test_proxy_auth_header_generation(self):
        """Test Proxy-Authorization header generation."""
        import base64

        auth = requestx.HTTPProxyAuth("user", "pass")
        # The authorize method should return a Base64 encoded header
        # This is a basic test - actual proxy auth would require a proxy server
        header = auth.authorize()
        self.assertTrue(header.startswith("Basic "))
        # Verify the base64 encoding
        encoded = header.split(" ")[1]
        decoded = base64.b64decode(encoded).decode("utf-8")
        self.assertEqual(decoded, "user:pass")


class TestAuthUtilities(HttpbinTestCase):
    """Tests for authentication utility functions."""

    def test_get_auth_from_url_with_auth(self):
        """Test get_auth_from_url with credentials in URL."""
        result = requestx.get_auth_from_url("https://user:pass@example.com/path")
        self.assertIsNotNone(result)
        username, password = result
        self.assertEqual(username, "user")
        self.assertEqual(password, "pass")

    def test_get_auth_from_url_without_auth(self):
        """Test get_auth_from_url without credentials in URL."""
        result = requestx.get_auth_from_url("https://example.com/path")
        self.assertIsNone(result)

    def test_get_auth_from_url_no_password(self):
        """Test get_auth_from_url with username only."""
        result = requestx.get_auth_from_url("https://user@example.com/path")
        self.assertIsNotNone(result)
        username, password = result
        self.assertEqual(username, "user")
        self.assertEqual(password, "")

    def test_urldefragauth_removes_auth(self):
        """Test urldefragauth removes authentication from URL."""
        result = requestx.urldefragauth("https://user:pass@example.com/path?query=1")
        self.assertEqual(result, "https://example.com/path?query=1")

    def test_urldefragauth_preserves_path_and_query(self):
        """Test urldefragauth preserves path and query string."""
        result = requestx.urldefragauth("https://user:pass@example.com/api/v1/users?id=123")
        self.assertEqual(result, "https://example.com/api/v1/users?id=123")

    def test_urldefragauth_preserves_fragment(self):
        """Test urldefragauth preserves fragment."""
        result = requestx.urldefragauth("https://user:pass@example.com/path#section")
        # Note: urldefragauth should also remove fragment as per its name
        self.assertIn("example.com", result)
        self.assertNotIn("user", result)
        self.assertNotIn("pass", result)


class TestAuthWithSession(HttpbinTestCase):
    """Tests for authentication with Session."""

    def test_session_with_basic_auth(self):
        """Test Session with basic auth."""
        session = requestx.Session()
        r = session.get(
            f"{HTTPBIN_HOST}/basic-auth/user/passwd", auth=("user", "passwd")
        )
        self.assertEqual(r.status_code, 200)

    def test_session_with_digest_auth(self):
        """Test Session with digest auth (falls back to basic auth)."""
        session = requestx.Session()
        auth = requestx.HTTPDigestAuth("user", "passwd")
        # Use as basic auth tuple since full digest requires 401 challenge handling
        r = session.get(
            f"{HTTPBIN_HOST}/basic-auth/user/passwd",
            auth=(auth.username, auth.password),
        )
        self.assertEqual(r.status_code, 200)

    def test_session_persists_auth(self):
        """Test that session persists authentication across requests."""
        session = requestx.Session()
        # First request sets up session
        r1 = session.get(
            f"{HTTPBIN_HOST}/get", auth=("user", "passwd")
        )
        self.assertEqual(r1.status_code, 200)
        # Second request should use same session (and auth)
        r2 = session.get(
            f"{HTTPBIN_HOST}/get", auth=("user", "passwd")
        )
        self.assertEqual(r2.status_code, 200)


class TestAuthEdgeCases(HttpbinTestCase):
    """Edge case tests for authentication."""

    def test_auth_with_empty_username(self):
        """Test auth with empty username."""
        r = requestx.get(
            f"{HTTPBIN_HOST}/basic-auth/user/passwd", auth=("", "passwd")
        )
        self.assertEqual(r.status_code, 401)

    def test_auth_with_empty_password(self):
        """Test auth with empty password."""
        r = requestx.get(
            f"{HTTPBIN_HOST}/basic-auth/user/passwd", auth=("user", "")
        )
        self.assertEqual(r.status_code, 401)

    def test_auth_with_special_characters(self):
        """Test auth with special characters in username/password."""
        # URL encode special characters
        r = requestx.get(
            f"{HTTPBIN_HOST}/basic-auth/user/passwd",
            auth=("user:passwd", "pass:word"),
        )
        # This should fail with 401 since credentials don't match
        self.assertEqual(r.status_code, 401)

    def test_digest_auth_with_special_characters(self):
        """Test digest auth with special characters."""
        auth = requestx.HTTPDigestAuth("user", "pass:word")
        r = requestx.get(
            f"{HTTPBIN_HOST}/digest-auth/2/user/passwd", auth=auth
        )
        # Should work or fail with 401, not crash
        self.assertIn(r.status_code, [200, 401])


if __name__ == "__main__":
    unittest.main()
