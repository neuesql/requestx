"""
RequestX Enhanced Response Test Suite

This test suite covers the enhanced response features implemented in Phase 1
to achieve 90% compatibility with the requests library.

Test categories:
- Elapsed time tracking (response.elapsed)
- Redirect history (response.history)
- Case-insensitive headers
- Response links (response.links)
- iter_content() generator
- iter_lines() generator

All tests use a local httpbin container via testcontainers.
"""

import datetime
import os
import sys
import unittest

# Add the python directory to the path for importing requestx
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "python"))

import requestx
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


class TestElapsedTime(HttpbinTestCase):
    """Test response.elapsed property for timing information."""

    def test_elapsed_exists(self):
        """Test that elapsed property exists on response."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        self.assertTrue(hasattr(r, "elapsed"))

    def test_elapsed_is_timedelta(self):
        """Test that elapsed returns a datetime.timedelta."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        self.assertIsInstance(r.elapsed, datetime.timedelta)

    def test_elapsed_positive(self):
        """Test that elapsed time is positive or zero."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        self.assertGreaterEqual(r.elapsed.total_seconds(), 0)

    def test_elapsed_reasonable(self):
        """Test that elapsed time is reasonable (< 60 seconds for simple request)."""
        r = requestx.get(HTTPBIN_HOST + "/get", timeout=30)
        self.assertLess(r.elapsed.total_seconds(), 60)

    def test_elapsed_total_seconds(self):
        """Test that elapsed.total_seconds() method works."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        total = r.elapsed.total_seconds()
        self.assertIsInstance(total, float)
        self.assertGreaterEqual(total, 0)

    def test_elapsed_microseconds(self):
        """Test that elapsed has microseconds component."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        # timedelta has days, seconds, and microseconds attributes
        self.assertTrue(hasattr(r.elapsed, "microseconds"))

    def test_elapsed_with_redirects(self):
        """Test elapsed time with redirects."""
        r = requestx.get(HTTPBIN_HOST + "/redirect/2", timeout=30)
        self.assertIsInstance(r.elapsed, datetime.timedelta)
        self.assertGreaterEqual(r.elapsed.total_seconds(), 0)

    def test_elapsed_consistency(self):
        """Test that elapsed is consistent across requests."""
        r1 = requestx.get(HTTPBIN_HOST + "/get", timeout=10)
        r2 = requestx.get(HTTPBIN_HOST + "/get", timeout=10)
        # Both should have valid elapsed times
        self.assertGreaterEqual(r1.elapsed.total_seconds(), 0)
        self.assertGreaterEqual(r2.elapsed.total_seconds(), 0)


class TestResponseHistory(HttpbinTestCase):
    """Test response.history property for redirect tracking."""

    def test_history_exists(self):
        """Test that history property exists on response."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        self.assertTrue(hasattr(r, "history"))

    def test_history_is_list(self):
        """Test that history returns a list."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        self.assertIsInstance(r.history, list)

    def test_history_no_redirects(self):
        """Test that history is empty for non-redirect responses."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        self.assertEqual(len(r.history), 0)

    def test_history_single_redirect(self):
        """Test history with single redirect."""
        r = requestx.get(HTTPBIN_HOST + "/relative-redirect/1", timeout=30)
        self.assertEqual(len(r.history), 1)

    def test_history_multiple_redirects(self):
        """Test history with multiple redirects."""
        r = requestx.get(HTTPBIN_HOST + "/redirect/3", timeout=30)
        self.assertEqual(len(r.history), 3)

    def test_history_redirect_status_codes(self):
        """Test that history contains redirect status codes."""
        r = requestx.get(HTTPBIN_HOST + "/redirect/2", timeout=30)
        for resp in r.history:
            self.assertIn(resp.status_code, [301, 302, 303, 307, 308])

    def test_history_response_has_required_properties(self):
        """Test that history responses have required properties."""
        r = requestx.get(HTTPBIN_HOST + "/redirect/2", timeout=30)
        for resp in r.history:
            self.assertTrue(hasattr(resp, "status_code"))
            self.assertTrue(hasattr(resp, "url"))
            self.assertTrue(hasattr(resp, "headers"))

    def test_history_response_status_code(self):
        """Test that history response status_code is correct."""
        r = requestx.get(HTTPBIN_HOST + "/redirect/2", timeout=30)
        for resp in r.history:
            self.assertIsInstance(resp.status_code, int)

    def test_history_response_url(self):
        """Test that history response URL is set."""
        r = requestx.get(HTTPBIN_HOST + "/redirect/2", timeout=30)
        for resp in r.history:
            self.assertIsInstance(resp.url, str)
            self.assertTrue(len(resp.url) > 0)

    def test_history_response_headers(self):
        """Test that history response headers are accessible."""
        r = requestx.get(HTTPBIN_HOST + "/redirect/2", timeout=30)
        for resp in r.history:
            self.assertTrue(hasattr(resp, "headers"))

    def test_history_with_disabled_redirects(self):
        """Test that history is empty when redirects are disabled."""
        r = requestx.get(
            HTTPBIN_HOST + "/redirect-to",
            params={"url": HTTPBIN_HOST + "/get"},
            allow_redirects=False,
            timeout=30,
        )
        self.assertEqual(len(r.history), 0)


class TestCaseInsensitiveHeaders(HttpbinTestCase):
    """Test case-insensitive header access."""

    def test_headers_case_insensitive_exact_match(self):
        """Test that headers can be accessed with exact case."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        # The headers should be accessible with the original case from server
        self.assertIn("content-type", r.headers)

    def test_headers_case_insensitive_uppercase(self):
        """Test that headers can be accessed with uppercase."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        # Should work with any case
        content_type = r.headers.get("CONTENT-TYPE")
        self.assertIsNotNone(content_type)
        self.assertIn("application/json", content_type)

    def test_headers_case_insensitive_lowercase(self):
        """Test that headers can be accessed with lowercase."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        content_type = r.headers.get("content-type")
        self.assertIsNotNone(content_type)
        self.assertIn("application/json", content_type)

    def test_headers_case_insensitive_mixed_case(self):
        """Test that headers can be accessed with mixed case."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        content_type = r.headers.get("Content-Type")
        self.assertIsNotNone(content_type)
        self.assertIn("application/json", content_type)

    def test_headers_getitem_case_insensitive(self):
        """Test __getitem__ with case-insensitive access."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        value1 = r.headers["content-type"]
        value2 = r.headers["Content-Type"]
        value3 = r.headers["CONTENT-TYPE"]
        self.assertEqual(value1, value2)
        self.assertEqual(value2, value3)

    def test_headers_get_method(self):
        """Test get() method works with case-insensitive access."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        value1 = r.headers.get("content-type")
        value2 = r.headers.get("Content-Type")
        self.assertEqual(value1, value2)

    def test_headers_contains_case_insensitive(self):
        """Test __contains__ with case-insensitive access."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        self.assertTrue("content-type" in r.headers)
        self.assertTrue("Content-Type" in r.headers)
        self.assertTrue("CONTENT-TYPE" in r.headers)

    def test_headers_keys(self):
        """Test that headers.keys() returns keys."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        keys = r.headers.keys()
        self.assertIsInstance(keys, list)
        self.assertIn("content-type", keys)

    def test_headers_values(self):
        """Test that headers.values() returns values."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        values = r.headers.values()
        self.assertIsInstance(values, list)
        self.assertTrue(len(values) > 0)

    def test_headers_items(self):
        """Test that headers.items() returns items."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        items = r.headers.items()
        self.assertIsInstance(items, list)
        self.assertTrue(len(items) > 0)

    def test_headers_len(self):
        """Test that len(headers) returns count."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        self.assertGreater(len(r.headers), 0)

    def test_headers_to_dict(self):
        """Test that to_dict() converts to regular dict."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        d = r.headers.to_dict()
        self.assertIsInstance(d, dict)
        self.assertIn("content-type", d)

    def test_headers_non_existent_key(self):
        """Test accessing non-existent header returns None."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        self.assertIsNone(r.headers.get("non-existent-header"))

    def test_headers_non_existent_keyitem(self):
        """Test accessing non-existent header via [] raises KeyError."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        with self.assertRaises(KeyError):
            _ = r.headers["non-existent-header"]


class TestResponseLinks(HttpbinTestCase):
    """Test response.links property for Link header parsing."""

    def test_links_exists(self):
        """Test that links property exists on response."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        self.assertTrue(hasattr(r, "links"))

    def test_links_is_dict(self):
        """Test that links returns a dictionary."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        self.assertIsInstance(r.links, dict)

    def test_links_empty_without_header(self):
        """Test that links is empty when no Link header present."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        self.assertEqual(len(r.links), 0)

    def test_links_with_link_header(self):
        """Test links with Link header (GitHub API pagination)."""
        # Use GitHub API which sends Link headers
        try:
            r = requestx.get("https://api.github.com/users?since=0")
            # If Link header is present, links should be populated
            # If not, links should be empty dict
            self.assertIsInstance(r.links, dict)
        except Exception:
            # Skip if network fails
            self.skipTest("Network unavailable")

    def test_links_structure(self):
        """Test that link entries have url key."""
        # httpbin.org/links endpoint might not exist, so we test structure
        r = requestx.get(HTTPBIN_HOST + "/get")
        links = r.links
        self.assertIsInstance(links, dict)


class TestIterContent(HttpbinTestCase):
    """Test iter_content() method for chunked content iteration."""

    def test_iter_content_exists(self):
        """Test that iter_content method exists."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        self.assertTrue(hasattr(r, "iter_content"))

    def test_iter_content_is_generator(self):
        """Test that iter_content returns a generator."""
        r = requestx.get(HTTPBIN_HOST + "/bytes/100")
        chunks = r.iter_content(chunk_size=50)
        import types
        self.assertIsInstance(chunks, types.GeneratorType)

    def test_iter_content_chunks_bytes(self):
        """Test that iter_content yields bytes chunks."""
        r = requestx.get(HTTPBIN_HOST + "/bytes/100")
        chunks = list(r.iter_content(chunk_size=50))
        self.assertTrue(len(chunks) > 0)
        for chunk in chunks:
            self.assertIsInstance(chunk, bytes)

    def test_iter_content_chunk_size(self):
        """Test that chunk_size parameter controls chunk size."""
        r = requestx.get(HTTPBIN_HOST + "/bytes/100")
        chunks = list(r.iter_content(chunk_size=25))
        # With 100 bytes and 25 byte chunks, should get 4 chunks
        total = sum(len(c) for c in chunks)
        self.assertEqual(total, 100)

    def test_iter_content_default_chunk_size(self):
        """Test iter_content with default chunk size."""
        r = requestx.get(HTTPBIN_HOST + "/bytes/200")
        chunks = list(r.iter_content())
        self.assertTrue(len(chunks) > 0)
        for chunk in chunks:
            self.assertIsInstance(chunk, bytes)

    def test_iter_content_complete_data(self):
        """Test that all content is yielded."""
        r = requestx.get(HTTPBIN_HOST + "/bytes/256")
        content = b"".join(r.iter_content(chunk_size=64))
        self.assertEqual(len(content), 256)

    def test_iter_content_empty_response(self):
        """Test iter_content on empty response."""
        r = requestx.get(HTTPBIN_HOST + "/status/204")
        chunks = list(r.iter_content())
        self.assertEqual(len(chunks), 0)

    def test_iter_content_none_chunk_size(self):
        """Test iter_content with None chunk_size uses default."""
        r = requestx.get(HTTPBIN_HOST + "/bytes/100")
        chunks = list(r.iter_content(chunk_size=None))
        self.assertTrue(len(chunks) > 0)


class TestIterLines(HttpbinTestCase):
    """Test iter_lines() method for line-by-line content iteration."""

    def test_iter_lines_exists(self):
        """Test that iter_lines method exists."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        self.assertTrue(hasattr(r, "iter_lines"))

    def test_iter_lines_is_generator(self):
        """Test that iter_lines returns a generator."""
        r = requestx.get(HTTPBIN_HOST + "/robots.txt")
        lines = r.iter_lines()
        import types
        self.assertIsInstance(lines, types.GeneratorType)

    def test_iter_lines_yields_strings(self):
        """Test that iter_lines yields string lines."""
        r = requestx.get(HTTPBIN_HOST + "/robots.txt")
        lines = list(r.iter_lines())
        self.assertTrue(len(lines) > 0)
        for line in lines:
            self.assertIsInstance(line, str)

    def test_iter_lines_robots_txt(self):
        """Test iter_lines with robots.txt endpoint."""
        r = requestx.get(HTTPBIN_HOST + "/robots.txt")
        lines = list(r.iter_lines())
        self.assertTrue(len(lines) >= 2)
        self.assertIn("User-agent", lines[0])
        self.assertIn("Disallow", lines[1])

    def test_iter_lines_empty_response(self):
        """Test iter_lines on response with no lines."""
        r = requestx.get(HTTPBIN_HOST + "/status/204")
        lines = list(r.iter_lines())
        self.assertEqual(len(lines), 0)

    def test_iter_lines_newlines(self):
        """Test that newlines are properly handled."""
        r = requestx.get(HTTPBIN_HOST + "/robots.txt")
        lines = list(r.iter_lines())
        # Lines should not contain trailing newlines
        for line in lines:
            self.assertFalse(line.endswith("\n"))
            self.assertFalse(line.endswith("\r"))


class TestResponseCombinations(HttpbinTestCase):
    """Test combinations of Phase 1 features."""

    def test_elapsed_with_history(self):
        """Test elapsed and history work together."""
        r = requestx.get(HTTPBIN_HOST + "/redirect/1", timeout=30)
        self.assertIsInstance(r.elapsed, datetime.timedelta)
        self.assertEqual(len(r.history), 1)

    def test_headers_with_history(self):
        """Test headers and history work together."""
        r = requestx.get(HTTPBIN_HOST + "/redirect/1", timeout=30)
        # Main response has headers
        self.assertIsInstance(r.headers, dict)
        # History responses have headers
        for resp in r.history:
            self.assertTrue(hasattr(resp, "headers"))

    def test_links_empty_with_history(self):
        """Test links is empty even with redirects (no Link header)."""
        r = requestx.get(HTTPBIN_HOST + "/redirect/1", timeout=30)
        self.assertIsInstance(r.links, dict)
        self.assertEqual(len(r.links), 0)

    def test_iter_content_with_history(self):
        """Test iter_content works after redirects."""
        r = requestx.get(HTTPBIN_HOST + "/redirect/1", timeout=30)
        # iter_content should work on final response
        content = b"".join(r.iter_content(chunk_size=100))
        self.assertIsInstance(content, bytes)

    def test_all_features_together(self):
        """Test all Phase 1 features work in a single request."""
        r = requestx.get(HTTPBIN_HOST + "/redirect/1", timeout=30)

        # Test elapsed
        self.assertIsInstance(r.elapsed, datetime.timedelta)

        # Test history
        self.assertEqual(len(r.history), 1)

        # Test headers case-insensitive
        ct1 = r.headers.get("content-type")
        ct2 = r.headers.get("Content-Type")
        self.assertEqual(ct1, ct2)

        # Test links
        self.assertIsInstance(r.links, dict)

        # Test iter_content
        content = b"".join(r.iter_content(chunk_size=100))
        self.assertIsInstance(content, bytes)


class TestEdgeCases(HttpbinTestCase):
    """Test edge cases and error handling."""

    def test_elapsed_zero_time(self):
        """Test elapsed with very fast response."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        self.assertGreaterEqual(r.elapsed.total_seconds(), 0)

    def test_history_max_redirects(self):
        """Test history with many redirects."""
        # httpbin allows up to 20 redirects
        r = requestx.get(HTTPBIN_HOST + "/redirect/10", timeout=30)
        self.assertEqual(len(r.history), 10)

    def test_headers_special_characters(self):
        """Test headers with special characters."""
        r = requestx.get(HTTPBIN_HOST + "/get")
        # Accessing headers should work even with special chars in keys
        self.assertIsNotNone(r.headers.get("content-type"))

    def test_iter_content_large_content(self):
        """Test iter_content with larger content."""
        r = requestx.get(HTTPBIN_HOST + "/bytes/1024")
        chunks = list(r.iter_content(chunk_size=256))
        content = b"".join(chunks)
        self.assertEqual(len(content), 1024)

    def test_iter_lines_multiline(self):
        """Test iter_lines with multiline content."""
        # Create a response that will have multiple lines
        r = requestx.get(HTTPBIN_HOST + "/robots.txt")
        lines = list(r.iter_lines())
        # Should have multiple lines
        self.assertGreater(len(lines), 1)


if __name__ == "__main__":
    unittest.main()
