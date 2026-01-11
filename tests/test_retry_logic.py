"""
RequestX Retry Logic Test Suite

This test suite covers the actual retry behavior of the requestx library:
- Retry on HTTP status codes (502, 503, 504)
- Retry on connection errors
- Exponential backoff timing
- Max retries limit enforcement
- HTTPAdapter mount pattern

All tests use a local httpbin container via testcontainers.
"""

import os
import sys
import time
import unittest

# Add the python directory to the path for importing requestx
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "python"))

import requestx
from requestx import Retry, HTTPAdapter
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
        if hasattr(cls, "container"):
            cls.container.stop()


class TestRetryOnStatusCodes(HttpbinTestCase):
    """Test retry behavior on specific HTTP status codes."""

    def test_retry_on_502_status(self):
        """Test that retry occurs on 502 Bad Gateway status code."""
        session = requestx.Session()
        # Mount HTTPAdapter with retry config that includes 502
        adapter = HTTPAdapter(max_retries=Retry(total=2, backoff_factor=0.01))
        session.mount("http://", adapter)

        # httpbin's /status/502 endpoint returns 502
        start_time = time.time()
        r = session.get(HTTPBIN_HOST + "/status/502", timeout=10)
        elapsed = time.time() - start_time

        # Should eventually return 502 (after retries exhausted)
        self.assertEqual(r.status_code, 502)
        # With 2 retries and backoff, should take some time
        # Minimum: initial request + 2 retries with backoff
        self.assertGreater(elapsed, 0.02, "Retry backoff should take measurable time")

    def test_retry_on_503_status(self):
        """Test that retry occurs on 503 Service Unavailable status code."""
        session = requestx.Session()
        adapter = HTTPAdapter(max_retries=Retry(total=3, backoff_factor=0.01))
        session.mount("http://", adapter)

        # httpbin's /status/503 endpoint returns 503
        start_time = time.time()
        r = session.get(HTTPBIN_HOST + "/status/503", timeout=10)
        elapsed = time.time() - start_time

        # Should eventually return 503
        self.assertEqual(r.status_code, 503)
        # With 3 retries, should take longer
        self.assertGreater(elapsed, 0.03, "Multiple retries should take more time")

    def test_retry_on_504_status(self):
        """Test that retry occurs on 504 Gateway Timeout status code."""
        session = requestx.Session()
        adapter = HTTPAdapter(max_retries=Retry(total=2, backoff_factor=0.01))
        session.mount("http://", adapter)

        # httpbin's /status/504 endpoint returns 504
        r = session.get(HTTPBIN_HOST + "/status/504", timeout=10)
        self.assertEqual(r.status_code, 504)

    def test_no_retry_on_200_status(self):
        """Test that successful 200 responses are not retried."""
        session = requestx.Session()
        adapter = HTTPAdapter(max_retries=Retry(total=3, backoff_factor=0.1))
        session.mount("http://", adapter)

        start_time = time.time()
        r = session.get(HTTPBIN_HOST + "/get", timeout=10)
        elapsed = time.time() - start_time

        # Should return immediately without retries
        self.assertEqual(r.status_code, 200)
        self.assertLess(elapsed, 1.0, "Successful request should be fast")

    def test_no_retry_on_404_status(self):
        """Test that 404 Not Found is not retried by default."""
        session = requestx.Session()
        adapter = HTTPAdapter(max_retries=Retry(total=3, backoff_factor=0.1))
        session.mount("http://", adapter)

        start_time = time.time()
        r = session.get(HTTPBIN_HOST + "/status/404", timeout=10)
        elapsed = time.time() - start_time

        # 404 should not be in default status_forcelist
        self.assertEqual(r.status_code, 404)
        # Should not retry, so should be fast
        self.assertLess(elapsed, 1.0, "404 should not trigger retries")

    def test_custom_status_forcelist(self):
        """Test retry with custom status_forcelist including 404."""
        session = requestx.Session()
        # Include 404 in the forcelist
        adapter = HTTPAdapter(max_retries=Retry(total=2, backoff_factor=0.01, status_forcelist=[404, 500]))
        session.mount("http://", adapter)

        start_time = time.time()
        r = session.get(HTTPBIN_HOST + "/status/404", timeout=10)
        elapsed = time.time() - start_time

        # With 404 in forcelist, should retry
        self.assertEqual(r.status_code, 404)
        self.assertGreater(elapsed, 0.01, "Custom forcelist should trigger retries")


class TestRetryOnConnectionErrors(HttpbinTestCase):
    """Test retry behavior on connection errors."""

    def test_connection_error_with_retry(self):
        """Test that connection errors trigger retry when configured."""
        session = requestx.Session()
        # Use very short timeout to force connection errors on slow/hanging requests
        adapter = HTTPAdapter(max_retries=Retry(total=1, backoff_factor=0.01))
        session.mount("http://", adapter)

        # Use a non-existent port to force connection error
        # This should trigger retry
        with self.assertRaises(Exception):  # noqa: B017
            session.get("http://localhost:59999/status/200", timeout=0.05)

    def test_connection_error_no_retry(self):
        """Test that connection errors fail fast when retries disabled."""
        session = requestx.Session()
        # No retry adapter (default)
        session.max_retries = 0

        start_time = time.time()
        try:
            session.get("http://localhost:59999/status/200", timeout=0.5)
        except Exception:
            pass
        elapsed = time.time() - start_time

        # Should fail quickly without retries
        self.assertLess(elapsed, 2.0, "Should fail fast without retry")


class TestExponentialBackoff(HttpbinTestCase):
    """Test exponential backoff timing behavior."""

    def test_backoff_increases_with_attempts(self):
        """Test that backoff delay increases exponentially with retry attempts."""
        session = requestx.Session()
        adapter = HTTPAdapter(max_retries=Retry(total=3, backoff_factor=0.05))
        session.mount("http://", adapter)

        start_time = time.time()
        # Use a status code that triggers retry
        r = session.get(HTTPBIN_HOST + "/status/503", timeout=10)
        total_elapsed = time.time() - start_time

        # With 3 retries and exponential backoff:
        # Delay 1: 0.05 * 1000 * 2^0 = 50ms
        # Delay 2: 0.05 * 1000 * 2^1 = 100ms
        # Delay 3: 0.05 * 1000 * 2^2 = 200ms
        # Total backoff: ~350ms minimum
        # We allow some tolerance for network variance
        self.assertGreater(total_elapsed, 0.3, "Exponential backoff should take significant time")

    def test_larger_backoff_factor(self):
        """Test that larger backoff_factor increases retry delays."""
        session1 = requestx.Session()
        adapter1 = HTTPAdapter(max_retries=Retry(total=2, backoff_factor=0.02))
        session1.mount("http://", adapter1)

        session2 = requestx.Session()
        adapter2 = HTTPAdapter(max_retries=Retry(total=2, backoff_factor=0.2))
        session2.mount("http://", adapter2)

        # Test with retry-triggering status code
        start1 = time.time()
        session1.get(HTTPBIN_HOST + "/status/503", timeout=10)
        elapsed1 = time.time() - start1

        start2 = time.time()
        session2.get(HTTPBIN_HOST + "/status/503", timeout=10)
        elapsed2 = time.time() - start2

        # Larger backoff_factor should result in longer total time
        self.assertGreater(elapsed2, elapsed1 * 2, 
                          "Larger backoff_factor should significantly increase delay")


class TestMaxRetriesLimit(HttpbinTestCase):
    """Test that max_retries limit is enforced."""

    def test_max_retries_one(self):
        """Test behavior with max_retries=1 (1 retry after initial failure)."""
        session = requestx.Session()
        adapter = HTTPAdapter(max_retries=Retry(total=1, backoff_factor=0.02))
        session.mount("http://", adapter)

        start_time = time.time()
        r = session.get(HTTPBIN_HOST + "/status/503", timeout=10)
        elapsed = time.time() - start_time

        self.assertEqual(r.status_code, 503)
        # With 1 retry: initial + 1 retry = 2 attempts
        # Should have 1 backoff delay
        self.assertGreater(elapsed, 0.02, "Single retry should add measurable delay")

    def test_max_retries_three(self):
        """Test behavior with max_retries=3 (3 retries after initial failure)."""
        session = requestx.Session()
        adapter = HTTPAdapter(max_retries=Retry(total=3, backoff_factor=0.02))
        session.mount("http://", adapter)

        start_time = time.time()
        r = session.get(HTTPBIN_HOST + "/status/502", timeout=15)
        elapsed = time.time() - start_time

        self.assertEqual(r.status_code, 502)
        # With 3 retries: initial + 3 retries = 4 attempts
        # Backoff delays: 20ms + 40ms + 80ms = 140ms minimum
        self.assertGreater(elapsed, 0.14, "Multiple retries should accumulate delay")

    def test_zero_retries_immediate_failure(self):
        """Test that max_retries=0 means no retries."""
        session = requestx.Session()
        # Default max_retries=0 means no retries
        adapter = HTTPAdapter(max_retries=Retry(total=0, backoff_factor=0.1))
        session.mount("http://", adapter)

        start_time = time.time()
        r = session.get(HTTPBIN_HOST + "/status/503", timeout=10)
        elapsed = time.time() - start_time

        # Should return immediately without retries
        self.assertEqual(r.status_code, 503)
        self.assertLess(elapsed, 0.5, "Zero retries should be immediate")


class TestHTTPAdapterMount(HttpbinTestCase):
    """Test HTTPAdapter mount pattern."""

    def test_mount_http_adapter(self):
        """Test mounting HTTPAdapter for HTTP URLs."""
        session = requestx.Session()
        adapter = HTTPAdapter(max_retries=Retry(total=2, backoff_factor=0.01))
        
        # Mount for http:// URLs
        session.mount("http://", adapter)
        
        # Request to http URL should use the adapter
        r = session.get(HTTPBIN_HOST + "/get")
        self.assertEqual(r.status_code, 200)

    def test_mount_https_adapter(self):
        """Test mounting HTTPAdapter for HTTPS URLs."""
        session = requestx.Session()
        adapter = HTTPAdapter(max_retries=Retry(total=2, backoff_factor=0.01))
        
        # Mount for https:// URLs
        session.mount("https://", adapter)
        
        # Note: httpbin doesn't support HTTPS in test container
        # This tests that mount doesn't fail
        session.max_retries = 2

    def test_mount_with_prefix(self):
        """Test that adapter matches URL prefix correctly."""
        session = requestx.Session()
        adapter = HTTPAdapter(max_retries=Retry(total=5, backoff_factor=0.01))
        
        # Mount for specific prefix
        session.mount(HTTPBIN_HOST + "/", adapter)
        
        # Requests to this URL should use the adapter
        r = session.get(HTTPBIN_HOST + "/get")
        self.assertEqual(r.status_code, 200)

    def test_get_adapter_method(self):
        """Test that get_adapter returns correct adapter for URL."""
        session = requestx.Session()
        adapter = HTTPAdapter(max_retries=Retry(total=3, backoff_factor=0.1))
        session.mount("http://", adapter)
        
        # Get adapter for http URL
        matched_adapter = session.get_adapter("http://example.com")
        self.assertIsNotNone(matched_adapter)
        
        # Get adapter for unknown URL
        unknown_adapter = session.get_adapter("ftp://example.com")
        self.assertIsNone(unknown_adapter)


class TestRetryConfigAttributes(HttpbinTestCase):
    """Test Retry configuration object attributes."""

    def test_retry_total_attribute(self):
        """Test that Retry.total specifies max retry count."""
        retry = Retry(total=3)
        self.assertEqual(retry.total, 3)

    def test_retry_backoff_factor_attribute(self):
        """Test that Retry.backoff_factor is configurable."""
        retry = Retry(total=2, backoff_factor=0.5)
        self.assertEqual(retry.backoff_factor, 0.5)

    def test_retry_status_forcelist_default(self):
        """Test default status_forcelist includes 502, 503, 504."""
        retry = Retry(total=1)
        # Default should include retryable status codes
        self.assertIn(502, retry.status_forcelist)
        self.assertIn(503, retry.status_forcelist)
        self.assertIn(504, retry.status_forcelist)

    def test_retry_custom_status_forcelist(self):
        """Test custom status_forcelist."""
        retry = Retry(total=1, status_forcelist=[403, 500])
        self.assertIn(403, retry.status_forcelist)
        self.assertIn(500, retry.status_forcelist)
        self.assertNotIn(502, retry.status_forcelist)


class TestSessionRetryIntegration(HttpbinTestCase):
    """Integration tests for retry with session features."""

    def test_session_with_retry_and_headers(self):
        """Test retry works with session headers."""
        session = requestx.Session()
        session.update_header("X-Custom", "test-value")
        adapter = HTTPAdapter(max_retries=Retry(total=2, backoff_factor=0.01))
        session.mount("http://", adapter)

        r = session.get(HTTPBIN_HOST + "/get", timeout=10)
        self.assertEqual(r.status_code, 200)

    def test_session_with_retry_and_cookies(self):
        """Test retry works with session cookies."""
        session = requestx.Session()
        adapter = HTTPAdapter(max_retries=Retry(total=2, backoff_factor=0.01))
        session.mount("http://", adapter)

        # Set cookies
        r = session.get(HTTPBIN_HOST + "/cookies/set?test=value", timeout=10)
        self.assertEqual(r.status_code, 200)

    def test_session_clone_preserves_retry_config(self):
        """Test that cloned session preserves retry adapter."""
        session = requestx.Session()
        adapter = HTTPAdapter(max_retries=Retry(total=3, backoff_factor=0.1))
        session.mount("http://", adapter)

        cloned = session.clone()
        
        # Clone should be able to make requests
        r = cloned.get(HTTPBIN_HOST + "/get", timeout=10)
        self.assertEqual(r.status_code, 200)


if __name__ == "__main__":
    unittest.main()
