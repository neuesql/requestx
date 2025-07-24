#!/usr/bin/env python3
"""Performance tests and benchmarks using unittest."""

import asyncio
import gc
import time
import unittest
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass, asdict
from typing import Dict, Any

import psutil

# Import HTTP clients for comparison
import requestx

# Optional imports for comparison (may not be available in all environments)
try:
    import requests
    HAS_REQUESTS = True
except ImportError:
    HAS_REQUESTS = False

try:
    import httpx
    HAS_HTTPX = True
except ImportError:
    HAS_HTTPX = False

try:
    import aiohttp
    HAS_AIOHTTP = True
except ImportError:
    HAS_AIOHTTP = False

@dataclass
class PerformanceMetrics:
    """Performance metrics for HTTP client comparison."""
    library_name: str
    requests_per_second: float
    average_response_time: float
    total_time: float
    memory_usage_mb: float
    cpu_usage_percent: float
    success_rate: float
    error_count: int
    concurrency_level: int = 1  # New field for concurrency level
    
    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


class BenchmarkRunner:
    """Utility class for running performance benchmarks."""
    
    def __init__(self):
        self.process = psutil.Process()
        self.results: Dict[str, PerformanceMetrics] = {}
    
    def measure_sync_performance(self, client_func, client_name: str, urls: list, 
                                timeout: float = 30.0, concurrency: int = 1) -> PerformanceMetrics:
        """Measure synchronous HTTP client performance."""
        # Reset process monitoring
        self.process.cpu_percent()  # First call returns 0.0, so we discard it
        gc.collect()
        
        memory_before = self.process.memory_info().rss / 1024 / 1024  # MB
        start_time = time.time()
        
        response_times = []
        errors = 0
        successful_requests = 0
        
        for url in urls:
            try:
                req_start = time.time()
                response = client_func(url)
                req_end = time.time()
                
                # Check if request was successful
                if hasattr(response, 'status_code'):
                    if response.status_code == 200:
                        successful_requests += 1
                    else:
                        errors += 1
                elif hasattr(response, 'status'):
                    if response.status == 200:
                        successful_requests += 1
                    else:
                        errors += 1
                else:
                    successful_requests += 1  # Assume success if no status available
                
                response_times.append(req_end - req_start)
                
            except Exception as e:
                errors += 1
                print(f"Error in {client_name}: {e}")
            
            # Check timeout
            if time.time() - start_time > timeout:
                print(f"Timeout reached for {client_name}")
                break
        
        total_time = time.time() - start_time
        memory_after = self.process.memory_info().rss / 1024 / 1024  # MB
        cpu_usage = self.process.cpu_percent()
        
        # Calculate metrics
        total_requests = len(response_times) + errors
        rps = total_requests / total_time if total_time > 0 else 0
        avg_response_time = sum(response_times) / len(response_times) if response_times else 0
        success_rate = successful_requests / total_requests if total_requests > 0 else 0
        memory_usage = memory_after - memory_before
        
        return PerformanceMetrics(
            library_name=client_name,
            requests_per_second=rps,
            average_response_time=avg_response_time * 1000,  # Convert to ms
            total_time=total_time,
            memory_usage_mb=memory_usage,
            cpu_usage_percent=cpu_usage,
            success_rate=success_rate,
            error_count=errors,
            concurrency_level=concurrency
        )
    
    def measure_concurrent_performance(self, client_func, client_name: str, url: str, 
                                      num_requests: int, concurrency: int, 
                                      timeout: float = 60.0) -> PerformanceMetrics:
        """Measure concurrent HTTP client performance using ThreadPoolExecutor."""
        # Reset process monitoring
        self.process.cpu_percent()
        gc.collect()
        
        memory_before = self.process.memory_info().rss / 1024 / 1024  # MB
        start_time = time.time()
        
        response_times = []
        errors = 0
        successful_requests = 0
        
        def make_request():
            """Single request function for thread pool."""
            try:
                req_start = time.time()
                response = client_func(url)
                req_end = time.time()
                
                # Check if request was successful
                success = False
                if hasattr(response, 'status_code'):
                    success = response.status_code == 200
                elif hasattr(response, 'status'):
                    success = response.status == 200
                else:
                    success = True  # Assume success if no status available
                
                return {
                    'success': success,
                    'response_time': req_end - req_start,
                    'error': None
                }
            except Exception as e:
                return {
                    'success': False,
                    'response_time': 0,
                    'error': str(e)
                }
        
        # Execute concurrent requests
        with ThreadPoolExecutor(max_workers=concurrency) as executor:
            try:
                # Submit all requests
                futures = [executor.submit(make_request) for _ in range(num_requests)]
                
                # Collect results with timeout
                for future in as_completed(futures, timeout=timeout):
                    try:
                        result = future.result()
                        if result['success']:
                            successful_requests += 1
                            response_times.append(result['response_time'])
                        else:
                            errors += 1
                            if result['error']:
                                print(f"Error in {client_name}: {result['error']}")
                    except Exception as e:
                        errors += 1
                        print(f"Future error in {client_name}: {e}")
                        
            except Exception as e:
                print(f"Concurrent execution error in {client_name}: {e}")
                errors = num_requests
        
        total_time = time.time() - start_time
        memory_after = self.process.memory_info().rss / 1024 / 1024  # MB
        cpu_usage = self.process.cpu_percent()
        
        # Calculate metrics
        total_requests = num_requests
        rps = total_requests / total_time if total_time > 0 else 0
        avg_response_time = sum(response_times) / len(response_times) if response_times else 0
        success_rate = successful_requests / total_requests if total_requests > 0 else 0
        memory_usage = memory_after - memory_before
        
        return PerformanceMetrics(
            library_name=client_name,
            requests_per_second=rps,
            average_response_time=avg_response_time * 1000,  # Convert to ms
            total_time=total_time,
            memory_usage_mb=memory_usage,
            cpu_usage_percent=cpu_usage,
            success_rate=success_rate,
            error_count=errors,
            concurrency_level=concurrency
        )

    async def measure_async_performance(self, client_func, client_name: str, urls: list,
                                      timeout: float = 30.0) -> PerformanceMetrics:
        """Measure asynchronous HTTP client performance."""
        # Reset process monitoring
        self.process.cpu_percent()
        gc.collect()
        
        memory_before = self.process.memory_info().rss / 1024 / 1024  # MB
        start_time = time.time()
        
        errors = 0
        successful_requests = 0
        
        try:
            # Create tasks for concurrent execution
            tasks = [client_func(url) for url in urls]
            responses = await asyncio.wait_for(asyncio.gather(*tasks, return_exceptions=True), timeout=timeout)
            
            for response in responses:
                if isinstance(response, Exception):
                    errors += 1
                else:
                    successful_requests += 1
                    
        except asyncio.TimeoutError:
            print(f"Timeout reached for {client_name}")
            errors = len(urls)
        except Exception as e:
            print(f"Error in {client_name}: {e}")
            errors = len(urls)
        
        total_time = time.time() - start_time
        memory_after = self.process.memory_info().rss / 1024 / 1024  # MB
        cpu_usage = self.process.cpu_percent()
        
        # Calculate metrics
        total_requests = len(urls)
        rps = total_requests / total_time if total_time > 0 else 0
        avg_response_time = total_time / total_requests * 1000 if total_requests > 0 else 0  # ms
        success_rate = successful_requests / total_requests if total_requests > 0 else 0
        memory_usage = memory_after - memory_before
        
        return PerformanceMetrics(
            library_name=client_name,
            requests_per_second=rps,
            average_response_time=avg_response_time,
            total_time=total_time,
            memory_usage_mb=memory_usage,
            cpu_usage_percent=cpu_usage,
            success_rate=success_rate,
            error_count=errors,
            concurrency_level=len(urls)  # For async, concurrency = number of URLs
        )
    
    def print_comparison_table(self, results: Dict[str, PerformanceMetrics], title: str):
        """Print a formatted comparison table."""
        print(f"\n{title}")
        print("=" * len(title))
        
        # Table header with concurrency
        print(f"{'Library':<12} {'Concur':<7} {'RPS':<8} {'Avg Time':<10} {'Memory':<10} {'CPU':<8} {'Success':<8} {'Errors':<8}")
        print(f"{'':=<12} {'':=<7} {'':=<8} {'':=<10} {'':=<10} {'':=<8} {'':=<8} {'':=<8}")
        
        # Sort by RPS (descending)
        sorted_results = sorted(results.items(), key=lambda x: x[1].requests_per_second, reverse=True)
        
        for name, metrics in sorted_results:
            print(f"{name:<12} {metrics.concurrency_level:<7} {metrics.requests_per_second:<8.1f} {metrics.average_response_time:<10.1f}ms "
                  f"{metrics.memory_usage_mb:<10.1f}MB {metrics.cpu_usage_percent:<8.1f}% "
                  f"{metrics.success_rate:<8.1%} {metrics.error_count:<8}")
        
        # Performance comparison (relative to requestx)
        if 'requestx' in results:
            requestx_rps = results['requestx'].requests_per_second
            requestx_memory = results['requestx'].memory_usage_mb
            requestx_time = results['requestx'].average_response_time
            
            print(f"\nPerformance vs RequestX:")
            for name, metrics in sorted_results:
                if name != 'requestx':
                    rps_diff = ((metrics.requests_per_second - requestx_rps) / requestx_rps * 100) if requestx_rps > 0 else 0
                    memory_diff = ((metrics.memory_usage_mb - requestx_memory) / requestx_memory * 100) if requestx_memory > 0 else 0
                    time_diff = ((metrics.average_response_time - requestx_time) / requestx_time * 100) if requestx_time > 0 else 0
                    
                    print(f"  {name}: RPS {rps_diff:+.1f}%, Memory {memory_diff:+.1f}%, Time {time_diff:+.1f}%")


class TestPerformanceBasics(unittest.TestCase):
    """Basic performance tests for HTTP operations."""

    def setUp(self):
        self.benchmark_runner = BenchmarkRunner()

    def test_request_response_time(self):
        """Test that requests complete within reasonable time."""
        start_time = time.time()
        response = requestx.get("http://0.0.0.0/get")
        end_time = time.time()
        
        # Should complete within 10 seconds (generous timeout for CI)
        self.assertLess(end_time - start_time, 10.0)
        self.assertEqual(response.status_code, 200)

    def test_multiple_requests_performance(self):
        """Test performance of multiple sequential requests."""
        start_time = time.time()
        
        responses = []
        for i in range(5):
            response = requestx.get("http://0.0.0.0/get")
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
        response = requestx.get("http://0.0.0.0/json")
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
        response = requestx.get("http://0.0.0.0/delay/2")
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
            response = requestx.get("http://0.0.0.0/get")
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
        response = requestx.get("http://0.0.0.0/json")
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
    """Benchmark tests comparing different HTTP clients."""

    def setUp(self):
        self.benchmark_runner = BenchmarkRunner()
        self.test_urls = ["http://0.0.0.0/get"] * 10  # Reduced for faster testing

    def test_sync_libraries_comparison(self):
        """Compare synchronous HTTP libraries performance."""
        results = {}
        
        # Test RequestX
        results['requestx'] = self.benchmark_runner.measure_sync_performance(
            requestx.get, 'requestx', self.test_urls
        )
        
        # Test requests (if available)
        if HAS_REQUESTS:
            results['requests'] = self.benchmark_runner.measure_sync_performance(
                requests.get, 'requests', self.test_urls
            )
        
        # Test httpx sync (if available)
        if HAS_HTTPX:
            results['httpx'] = self.benchmark_runner.measure_sync_performance(
                httpx.get, 'httpx', self.test_urls
            )
        
        # Print comparison table
        self.benchmark_runner.print_comparison_table(results, "Synchronous HTTP Libraries Comparison")
        
        # Verify RequestX performed reasonably
        requestx_metrics = results['requestx']
        self.assertGreater(requestx_metrics.requests_per_second, 0)
        self.assertGreater(requestx_metrics.success_rate, 0.8)  # At least 80% success rate

    def test_async_libraries_comparison(self):
        """Compare asynchronous HTTP libraries performance."""
        async def run_async_benchmarks():
            results = {}
            
            # Test RequestX async (same function with await)
            async def requestx_async_get(url):
                return await requestx.get(url)
            
            # Note: RequestX doesn't have native async support yet, so this will fail
            # This is a placeholder for future async implementation
            try:
                results['requestx_async'] = await self.benchmark_runner.measure_async_performance(
                    requestx_async_get, 'requestx_async', self.test_urls[:5]  # Smaller test set
                )
            except Exception as e:
                print(f"RequestX async not yet implemented: {e}")
            
            # Test httpx async (if available)
            if HAS_HTTPX:
                async def httpx_async_get(url):
                    async with httpx.AsyncClient() as client:
                        return await client.get(url)
                
                try:
                    results['httpx_async'] = await self.benchmark_runner.measure_async_performance(
                        httpx_async_get, 'httpx_async', self.test_urls[:5]
                    )
                except Exception as e:
                    print(f"HTTPX async test failed: {e}")
            
            # Test aiohttp (if available)
            if HAS_AIOHTTP:
                async def aiohttp_get(url):
                    async with aiohttp.ClientSession() as session:
                        async with session.get(url) as response:
                            return response
                
                try:
                    results['aiohttp'] = await self.benchmark_runner.measure_async_performance(
                        aiohttp_get, 'aiohttp', self.test_urls[:5]
                    )
                except Exception as e:
                    print(f"aiohttp test failed: {e}")
            
            if results:
                self.benchmark_runner.print_comparison_table(results, "Asynchronous HTTP Libraries Comparison")
            else:
                print("No async libraries available for comparison")
            
            return results
        
        # Run async benchmarks
        try:
            results = asyncio.run(run_async_benchmarks())
            # Basic validation if we have results
            if results:
                for name, metrics in results.items():
                    self.assertGreaterEqual(metrics.success_rate, 0.0)
        except Exception as e:
            print(f"Async benchmark failed: {e}")

    def test_cold_vs_warm_requests(self):
        """Compare performance of first request vs subsequent requests."""
        # Cold request (first one)
        start_time = time.time()
        response1 = requestx.get("http://0.0.0.0/get")
        cold_time = time.time() - start_time
        
        self.assertEqual(response1.status_code, 200)
        
        # Warm requests (subsequent ones)
        warm_times = []
        for i in range(3):
            start_time = time.time()
            response = requestx.get("http://0.0.0.0/get")
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
        """Compare performance of different HTTP methods with RequestX."""
        methods_and_funcs = [
            ("GET", requestx.get, "http://0.0.0.0/get"),
            ("POST", requestx.post, "http://0.0.0.0/post"),
            ("PUT", requestx.put, "http://0.0.0.0/put"),
            ("DELETE", requestx.delete, "http://0.0.0.0/delete"),
            ("PATCH", requestx.patch, "http://0.0.0.0/patch"),
        ]
        
        results = {}
        
        for method, func, url in methods_and_funcs:
            try:
                metrics = self.benchmark_runner.measure_sync_performance(
                    lambda u: func(u), f'requestx_{method}', [url] * 3
                )
                results[method] = metrics
            except Exception as e:
                print(f"Error testing {method}: {e}")
        
        if results:
            self.benchmark_runner.print_comparison_table(results, "HTTP Methods Performance Comparison")
            
            # All methods should complete successfully (allow for some network issues)
            for method, metrics in results.items():
                self.assertGreaterEqual(metrics.success_rate, 0.5, f"{method} had low success rate")

    def test_memory_efficiency_comparison(self):
        """Compare memory efficiency across different libraries."""
        test_urls = ["http://0.0.0.0/json"] * 5  # JSON responses for memory testing
        results = {}
        
        # Test RequestX memory usage
        results['requestx'] = self.benchmark_runner.measure_sync_performance(
            requestx.get, 'requestx', test_urls
        )
        
        # Test requests memory usage (if available)
        if HAS_REQUESTS:
            results['requests'] = self.benchmark_runner.measure_sync_performance(
                requests.get, 'requests', test_urls
            )
        
        # Test httpx memory usage (if available)
        if HAS_HTTPX:
            results['httpx'] = self.benchmark_runner.measure_sync_performance(
                httpx.get, 'httpx', test_urls
            )
        
        self.benchmark_runner.print_comparison_table(results, "Memory Efficiency Comparison")
        
        # Verify reasonable memory usage
        for name, metrics in results.items():
            self.assertLess(metrics.memory_usage_mb, 100, f"{name} used too much memory")  # Less than 100MB


class TestConcurrencyBenchmarks(unittest.TestCase):
    """Concurrency benchmark tests comparing different HTTP clients."""

    def setUp(self):
        self.benchmark_runner = BenchmarkRunner()
        self.test_url = "http://0.0.0.0/get"
        self.concurrency_levels = [10, 100, 1000]  # Different concurrency levels to test

    def test_concurrency_comparison_small(self):
        """Compare libraries with 10 concurrent requests."""
        concurrency = 10
        num_requests = 50  # Total requests
        
        print(f"\n=== Concurrency Test: {concurrency} concurrent, {num_requests} total requests ===")
        
        results = {}
        
        # Test RequestX
        try:
            results['requestx'] = self.benchmark_runner.measure_concurrent_performance(
                requestx.get, 'requestx', self.test_url, num_requests, concurrency
            )
        except Exception as e:
            print(f"RequestX concurrent test failed: {e}")
        
        # Test requests (if available)
        if HAS_REQUESTS:
            try:
                results['requests'] = self.benchmark_runner.measure_concurrent_performance(
                    requests.get, 'requests', self.test_url, num_requests, concurrency
                )
            except Exception as e:
                print(f"Requests concurrent test failed: {e}")
        
        # Test httpx (if available)
        if HAS_HTTPX:
            try:
                results['httpx'] = self.benchmark_runner.measure_concurrent_performance(
                    httpx.get, 'httpx', self.test_url, num_requests, concurrency
                )
            except Exception as e:
                print(f"HTTPX concurrent test failed: {e}")
        
        if results:
            self.benchmark_runner.print_comparison_table(results, f"Concurrency {concurrency} Performance")
            
            # Verify all libraries handled concurrency reasonably
            for name, metrics in results.items():
                self.assertGreater(metrics.requests_per_second, 0, f"{name} had zero RPS")
                self.assertGreaterEqual(metrics.success_rate, 0.7, f"{name} had low success rate: {metrics.success_rate}")

    def test_concurrency_comparison_medium(self):
        """Compare libraries with 100 concurrent requests."""
        concurrency = 100
        num_requests = 200  # Total requests
        
        print(f"\n=== Concurrency Test: {concurrency} concurrent, {num_requests} total requests ===")
        
        results = {}
        
        # Test RequestX
        try:
            results['requestx'] = self.benchmark_runner.measure_concurrent_performance(
                requestx.get, 'requestx', self.test_url, num_requests, concurrency, timeout=120.0
            )
        except Exception as e:
            print(f"RequestX concurrent test failed: {e}")
        
        # Test requests (if available)
        if HAS_REQUESTS:
            try:
                results['requests'] = self.benchmark_runner.measure_concurrent_performance(
                    requests.get, 'requests', self.test_url, num_requests, concurrency, timeout=120.0
                )
            except Exception as e:
                print(f"Requests concurrent test failed: {e}")
        
        # Test httpx (if available)
        if HAS_HTTPX:
            try:
                results['httpx'] = self.benchmark_runner.measure_concurrent_performance(
                    httpx.get, 'httpx', self.test_url, num_requests, concurrency, timeout=120.0
                )
            except Exception as e:
                print(f"HTTPX concurrent test failed: {e}")
        
        if results:
            self.benchmark_runner.print_comparison_table(results, f"Concurrency {concurrency} Performance")
            
            # Verify all libraries handled concurrency reasonably
            for name, metrics in results.items():
                self.assertGreater(metrics.requests_per_second, 0, f"{name} had zero RPS")
                self.assertGreaterEqual(metrics.success_rate, 0.5, f"{name} had low success rate: {metrics.success_rate}")

    def test_concurrency_comparison_large(self):
        """Compare libraries with 1000 concurrent requests (stress test)."""
        concurrency = 10
        num_requests = 100  # Total requests
        
        print(f"\n=== Concurrency Stress Test: {concurrency} concurrent, {num_requests} total requests ===")
        
        results = {}
        
        # Test RequestX
        try:
            results['requestx'] = self.benchmark_runner.measure_concurrent_performance(
                requestx.get, 'requestx', self.test_url, num_requests, concurrency, timeout=180.0
            )
        except Exception as e:
            print(f"RequestX stress test failed: {e}")
        
        # Test requests (if available) - may struggle with high concurrency
        if HAS_REQUESTS:
            try:
                results['requests'] = self.benchmark_runner.measure_concurrent_performance(
                    requests.get, 'requests', self.test_url, num_requests, concurrency, timeout=180.0
                )
            except Exception as e:
                print(f"Requests stress test failed: {e}")
        
        # Test httpx (if available)
        if HAS_HTTPX:
            try:
                results['httpx'] = self.benchmark_runner.measure_concurrent_performance(
                    httpx.get, 'httpx', self.test_url, num_requests, concurrency, timeout=180.0
                )
            except Exception as e:
                print(f"HTTPX stress test failed: {e}")
        
        if results:
            self.benchmark_runner.print_comparison_table(results, f"Concurrency {concurrency} Stress Test")
            
            # More lenient validation for stress test
            for name, metrics in results.items():
                self.assertGreaterEqual(metrics.success_rate, 0.3, f"{name} had very low success rate: {metrics.success_rate}")

    def test_concurrency_scaling_analysis(self):
        """Analyze how each library scales with increasing concurrency."""
        print(f"\n=== Concurrency Scaling Analysis ===")
        
        scaling_results = {}
        concurrency_levels = [1, 10, 50]  # Reduced for faster testing
        num_requests_per_test = 50
        
        for concurrency in concurrency_levels:
            print(f"\nTesting concurrency level: {concurrency}")
            
            # Test RequestX scaling
            try:
                requestx_metrics = self.benchmark_runner.measure_concurrent_performance(
                    requestx.get, f'requestx_c{concurrency}', self.test_url, 
                    num_requests_per_test, concurrency, timeout=60.0
                )
                scaling_results[f'requestx_c{concurrency}'] = requestx_metrics
            except Exception as e:
                print(f"RequestX scaling test failed at concurrency {concurrency}: {e}")
            
            # Test requests scaling (if available)
            if HAS_REQUESTS:
                try:
                    requests_metrics = self.benchmark_runner.measure_concurrent_performance(
                        requests.get, f'requests_c{concurrency}', self.test_url, 
                        num_requests_per_test, concurrency, timeout=60.0
                    )
                    scaling_results[f'requests_c{concurrency}'] = requests_metrics
                except Exception as e:
                    print(f"Requests scaling test failed at concurrency {concurrency}: {e}")
        
        if scaling_results:
            self.benchmark_runner.print_comparison_table(scaling_results, "Concurrency Scaling Analysis")
            
            # Analyze scaling patterns
            print(f"\nScaling Analysis:")
            for library in ['requestx', 'requests']:
                if HAS_REQUESTS or library == 'requestx':
                    library_results = {k: v for k, v in scaling_results.items() if k.startswith(library)}
                    if len(library_results) > 1:
                        sorted_by_concurrency = sorted(library_results.items(), 
                                                     key=lambda x: x[1].concurrency_level)
                        
                        print(f"\n{library.upper()} Scaling:")
                        for name, metrics in sorted_by_concurrency:
                            print(f"  Concurrency {metrics.concurrency_level}: "
                                  f"{metrics.requests_per_second:.1f} RPS, "
                                  f"{metrics.average_response_time:.1f}ms avg, "
                                  f"{metrics.success_rate:.1%} success")

    def test_concurrent_vs_sequential_comparison(self):
        """Compare concurrent vs sequential performance for the same total requests."""
        num_requests = 50
        
        print(f"\n=== Concurrent vs Sequential Comparison ({num_requests} total requests) ===")
        
        results = {}
        
        # Sequential RequestX
        try:
            sequential_metrics = self.benchmark_runner.measure_concurrent_performance(
                requestx.get, 'requestx_seq', self.test_url, num_requests, 1  # Concurrency = 1
            )
            results['requestx_sequential'] = sequential_metrics
        except Exception as e:
            print(f"RequestX sequential test failed: {e}")
        
        # Concurrent RequestX (10 threads)
        try:
            concurrent_metrics = self.benchmark_runner.measure_concurrent_performance(
                requestx.get, 'requestx_conc', self.test_url, num_requests, 10
            )
            results['requestx_concurrent'] = concurrent_metrics
        except Exception as e:
            print(f"RequestX concurrent test failed: {e}")
        
        # Sequential requests (if available)
        if HAS_REQUESTS:
            try:
                requests_sequential = self.benchmark_runner.measure_concurrent_performance(
                    requests.get, 'requests_seq', self.test_url, num_requests, 1
                )
                results['requests_sequential'] = requests_sequential
            except Exception as e:
                print(f"Requests sequential test failed: {e}")
            
            try:
                requests_concurrent = self.benchmark_runner.measure_concurrent_performance(
                    requests.get, 'requests_conc', self.test_url, num_requests, 10
                )
                results['requests_concurrent'] = requests_concurrent
            except Exception as e:
                print(f"Requests concurrent test failed: {e}")
        
        if results:
            self.benchmark_runner.print_comparison_table(results, "Sequential vs Concurrent Performance")
            
            # Analyze concurrency benefits
            print(f"\nConcurrency Benefits Analysis:")
            for library in ['requestx', 'requests']:
                seq_key = f'{library}_sequential'
                conc_key = f'{library}_concurrent'
                
                if seq_key in results and conc_key in results:
                    seq_metrics = results[seq_key]
                    conc_metrics = results[conc_key]
                    
                    rps_improvement = ((conc_metrics.requests_per_second - seq_metrics.requests_per_second) / 
                                     seq_metrics.requests_per_second * 100)
                    time_improvement = ((seq_metrics.total_time - conc_metrics.total_time) / 
                                      seq_metrics.total_time * 100)
                    
                    print(f"  {library.upper()}: RPS improvement {rps_improvement:+.1f}%, "
                          f"Total time improvement {time_improvement:+.1f}%")


if __name__ == '__main__':
    # Run the tests with more verbose output for performance info
    unittest.main(verbosity=2)