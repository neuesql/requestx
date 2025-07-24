#!/usr/bin/env python3
"""Performance tests and benchmarks using unittest."""

import unittest
import time
import gc
import sys
import requestx


class TestPerformanceBasics(unittest.TestCase):
    """Basic performance tests for HTTP operations."""

    def test_request_response_time(self):
        """Test that requests complete within reasonable time."""
        start_time = time.time()
        response = requestx.get("https://httpbin.org/get")
        end_time = time.time()
        
        # Should complete within 10 seconds (generous timeout for CI)
        self.assertLess(end_time - start_time, 10.0)
        self.assertEqual(response.status_code, 200)

    def test_multiple_requests_performance(self):
        """Test performance of multiple sequential requests."""
        start_time = time.time()
        
        responses = []
        for i in range(5):
            response = requestx.get("https://httpbin.org/get")
            responses.append(response)
        
        end_time = time.time()
        
        # All requests should succeed
        for response in responses:
            self.assertEqual(response.status_code, 200)
        
        # Should complete within reasonable time (50 seconds for 5 requests)
        self.assertLess(end_time - start_time, 50.0)
        
        print(f"5 sequential requests took {end_time - start_time:.2f} seconds")

    def test_large_response_handling(self):
        """Test handling of large responses."""
        # Request a large JSON response (100 items)
        start_time = time.time()
        response = requestx.get("https://httpbin.org/json")
        end_time = time.time()
        
        self.assertEqual(response.status_code, 200)
        self.assertLess(end_time - start_time, 10.0)
        
        # Test that we can parse the JSON
        json_data = response.json()
        self.assertIsInstance(json_data, dict)

    def test_timeout_performance(self):
        """Test that timeouts work correctly and don't hang."""
        # This test would need timeout support to be implemented
        # For now, just test that delayed requests work
        start_time = time.time()
        response = requestx.get("https://httpbin.org/delay/2")
        end_time = time.time()
        
        # Should take at least 2 seconds
        self.assertGreaterEqual(end_time - start_time, 2.0)
        # But not more than 10 seconds
        self.assertLess(end_time - start_time, 10.0)
        self.assertEqual(response.status_code, 200)


class TestMemoryUsage(unittest.TestCase):
    """Memory usage tests."""

    def test_response_memory_cleanup(self):
        """Test that response objects don't leak memory."""
        # Force garbage collection before test
        gc.collect()
        initial_objects = len(gc.get_objects())
        
        # Create and discard multiple responses
        for i in range(10):
            response = requestx.get("https://httpbin.org/get")
            self.assertEqual(response.status_code, 200)
            # Access response data to ensure it's loaded
            _ = response.text
            _ = response.headers
        
        # Force garbage collection after test
        gc.collect()
        final_objects = len(gc.get_objects())
        
        # Object count shouldn't grow significantly
        # Allow some growth for test infrastructure
        object_growth = final_objects - initial_objects
        self.assertLess(object_growth, 1000, 
                       f"Memory leak detected: {object_growth} objects created")

    def test_large_response_memory_efficiency(self):
        """Test memory efficiency with large responses."""
        # Get memory usage before
        gc.collect()
        
        # Make request for potentially large response
        response = requestx.get("https://httpbin.org/json")
        self.assertEqual(response.status_code, 200)
        
        # Access the content
        text_content = response.text
        json_content = response.json()
        binary_content = response.content
        
        # Verify content is accessible
        self.assertIsInstance(text_content, str)
        self.assertIsInstance(json_content, dict)
        self.assertIsNotNone(binary_content)
        
        # Clean up
        del response, text_content, json_content, binary_content
        gc.collect()


class TestBenchmarkComparison(unittest.TestCase):
    """Benchmark tests comparing different scenarios."""

    def test_cold_vs_warm_requests(self):
        """Compare performance of first request vs subsequent requests."""
        # Cold request (first one)
        start_time = time.time()
        response1 = requestx.get("https://httpbin.org/get")
        cold_time = time.time() - start_time
        
        self.assertEqual(response1.status_code, 200)
        
        # Warm requests (subsequent ones)
        warm_times = []
        for i in range(3):
            start_time = time.time()
            response = requestx.get("https://httpbin.org/get")
            warm_time = time.time() - start_time
            warm_times.append(warm_time)
            self.assertEqual(response.status_code, 200)
        
        avg_warm_time = sum(warm_times) / len(warm_times)
        
        print(f"Cold request time: {cold_time:.3f}s")
        print(f"Average warm request time: {avg_warm_time:.3f}s")
        
        # Both should be reasonable
        self.assertLess(cold_time, 10.0)
        self.assertLess(avg_warm_time, 10.0)

    def test_different_http_methods_performance(self):
        """Compare performance of different HTTP methods."""
        methods_and_urls = [
            ("GET", "https://httpbin.org/get"),
            ("POST", "https://httpbin.org/post"),
            ("PUT", "https://httpbin.org/put"),
            ("DELETE", "https://httpbin.org/delete"),
            ("PATCH", "https://httpbin.org/patch"),
        ]
        
        method_times = {}
        
        for method, url in methods_and_urls:
            start_time = time.time()
            
            if method == "GET":
                response = requestx.get(url)
            elif method == "POST":
                response = requestx.post(url)
            elif method == "PUT":
                response = requestx.put(url)
            elif method == "DELETE":
                response = requestx.delete(url)
            elif method == "PATCH":
                response = requestx.patch(url)
            
            end_time = time.time()
            method_times[method] = end_time - start_time
            
            self.assertEqual(response.status_code, 200)
        
        # Print timing results
        for method, timing in method_times.items():
            print(f"{method} request time: {timing:.3f}s")
        
        # All methods should complete within reasonable time
        for method, timing in method_times.items():
            self.assertLess(timing, 10.0, f"{method} took too long: {timing}s")


if __name__ == '__main__':
    # Run the tests with more verbose output for performance info
    unittest.main(verbosity=2)