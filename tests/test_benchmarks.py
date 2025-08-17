#!/usr/bin/env python3
"""
Performance benchmarking tests for RequestX.

This module provides comprehensive performance benchmarks comparing requestx
against other HTTP libraries (requests, httpx, aiohttp) across various metrics.

Requirements tested: 3.1, 3.2, 3.3, 3.4, 10.1, 10.2, 10.3, 10.4
"""

import unittest
import asyncio
import time
import threading
import statistics
import sys
import os
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
from typing import List, Dict, Any, Callable

# Add the parent directory to the path to import requestx
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'python'))

try:
    import requestx
except ImportError as e:
    print(f"Failed to import requestx: {e}")
    print("Make sure to build the extension with: uv run maturin develop")
    sys.exit(1)

# Try to import comparison libraries
try:
    import requests
    HAS_REQUESTS = True
except ImportError:
    HAS_REQUESTS = False
    print("Warning: requests library not available for comparison")

try:
    import httpx
    HAS_HTTPX = True
except ImportError:
    HAS_HTTPX = False
    print("Warning: httpx library not available for comparison")

try:
    import aiohttp
    HAS_AIOHTTP = True
except ImportError:
    HAS_AIOHTTP = False
    print("Warning: aiohttp library not available for comparison")

try:
    import psutil
    HAS_PSUTIL = True
except ImportError:
    HAS_PSUTIL = False
    print("Warning: psutil not available for memory monitoring")


@dataclass
class BenchmarkResult:
    """Container for benchmark results."""
    library_name: str
    requests_per_second: float
    average_response_time: float
    median_response_time: float
    p95_response_time: float
    total_time: float
    memory_usage_mb: float
    cpu_usage_percent: float
    success_rate: float
    error_count: int


class PerformanceBenchmark:
    """Performance benchmarking framework."""
    
    def __init__(self):
        self.base_url = "https://httpbin.org"
        self.timeout = 30
        self.results: List[BenchmarkResult] = []
    
    def measure_memory_usage(self) -> float:
        """Measure current memory usage in MB."""
        if not HAS_PSUTIL:
            return 0.0
        
        try:
            process = psutil.Process()
            return process.memory_info().rss / 1024 / 1024  # Convert to MB
        except Exception:
            return 0.0
    
    def measure_cpu_usage(self) -> float:
        """Measure current CPU usage percentage."""
        if not HAS_PSUTIL:
            return 0.0
        
        try:
            process = psutil.Process()
            return process.cpu_percent()
        except Exception:
            return 0.0
    
    def benchmark_sync_library(self, library_func: Callable, library_name: str, 
                              num_requests: int = 50) -> BenchmarkResult:
        """Benchmark a synchronous HTTP library."""
        print(f"Benchmarking {library_name} (sync) with {num_requests} requests...")
        
        # Warm up
        try:
            library_func(f"{self.base_url}/get")
        except Exception:
            pass
        
        # Measure initial state
        initial_memory = self.measure_memory_usage()
        initial_cpu = self.measure_cpu_usage()
        
        # Run benchmark
        response_times = []
        errors = 0
        successful_requests = 0
        
        start_time = time.time()
        
        for i in range(num_requests):
            request_start = time.time()
            try:
                response = library_func(f"{self.base_url}/get?id={i}")
                request_end = time.time()
                
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
                
                response_times.append(request_end - request_start)
                
            except Exception as e:
                request_end = time.time()
                errors += 1
                response_times.append(request_end - request_start)
        
        end_time = time.time()
        total_time = end_time - start_time
        
        # Measure final state
        final_memory = self.measure_memory_usage()
        final_cpu = self.measure_cpu_usage()
        
        # Calculate metrics
        if response_times:
            avg_response_time = statistics.mean(response_times)
            median_response_time = statistics.median(response_times)
            p95_response_time = statistics.quantiles(response_times, n=20)[18] if len(response_times) > 1 else avg_response_time
        else:
            avg_response_time = median_response_time = p95_response_time = 0.0
        
        requests_per_second = num_requests / total_time if total_time > 0 else 0.0
        success_rate = (successful_requests / num_requests) * 100 if num_requests > 0 else 0.0
        memory_usage = max(0, final_memory - initial_memory)
        cpu_usage = max(0, final_cpu - initial_cpu)
        
        result = BenchmarkResult(
            library_name=library_name,
            requests_per_second=requests_per_second,
            average_response_time=avg_response_time * 1000,  # Convert to ms
            median_response_time=median_response_time * 1000,
            p95_response_time=p95_response_time * 1000,
            total_time=total_time,
            memory_usage_mb=memory_usage,
            cpu_usage_percent=cpu_usage,
            success_rate=success_rate,
            error_count=errors
        )
        
        print(f"  {library_name}: {requests_per_second:.1f} req/s, "
              f"{avg_response_time*1000:.1f}ms avg, {success_rate:.1f}% success")
        
        return result
    
    async def benchmark_async_library(self, library_func: Callable, library_name: str,
                                    num_requests: int = 50) -> BenchmarkResult:
        """Benchmark an asynchronous HTTP library."""
        print(f"Benchmarking {library_name} (async) with {num_requests} requests...")
        
        # Warm up
        try:
            await library_func(f"{self.base_url}/get")
        except Exception:
            pass
        
        # Measure initial state
        initial_memory = self.measure_memory_usage()
        initial_cpu = self.measure_cpu_usage()
        
        # Run benchmark
        response_times = []
        errors = 0
        successful_requests = 0
        
        start_time = time.time()
        
        # Create tasks for concurrent execution
        async def make_request(request_id):
            request_start = time.time()
            try:
                response = await library_func(f"{self.base_url}/get?id={request_id}")
                request_end = time.time()
                
                # Check if request was successful
                if hasattr(response, 'status_code'):
                    success = response.status_code == 200
                elif hasattr(response, 'status'):
                    success = response.status == 200
                else:
                    success = True  # Assume success if no status available
                
                return request_end - request_start, success, None
                
            except Exception as e:
                request_end = time.time()
                return request_end - request_start, False, str(e)
        
        # Execute all requests concurrently
        tasks = [make_request(i) for i in range(num_requests)]
        results = await asyncio.gather(*tasks, return_exceptions=True)
        
        end_time = time.time()
        total_time = end_time - start_time
        
        # Process results
        for result in results:
            if isinstance(result, Exception):
                errors += 1
                response_times.append(0.1)  # Default time for failed requests
            else:
                response_time, success, error = result
                response_times.append(response_time)
                if success:
                    successful_requests += 1
                else:
                    errors += 1
        
        # Measure final state
        final_memory = self.measure_memory_usage()
        final_cpu = self.measure_cpu_usage()
        
        # Calculate metrics
        if response_times:
            avg_response_time = statistics.mean(response_times)
            median_response_time = statistics.median(response_times)
            p95_response_time = statistics.quantiles(response_times, n=20)[18] if len(response_times) > 1 else avg_response_time
        else:
            avg_response_time = median_response_time = p95_response_time = 0.0
        
        requests_per_second = num_requests / total_time if total_time > 0 else 0.0
        success_rate = (successful_requests / num_requests) * 100 if num_requests > 0 else 0.0
        memory_usage = max(0, final_memory - initial_memory)
        cpu_usage = max(0, final_cpu - initial_cpu)
        
        result = BenchmarkResult(
            library_name=f"{library_name} (async)",
            requests_per_second=requests_per_second,
            average_response_time=avg_response_time * 1000,  # Convert to ms
            median_response_time=median_response_time * 1000,
            p95_response_time=p95_response_time * 1000,
            total_time=total_time,
            memory_usage_mb=memory_usage,
            cpu_usage_percent=cpu_usage,
            success_rate=success_rate,
            error_count=errors
        )
        
        print(f"  {library_name} (async): {requests_per_second:.1f} req/s, "
              f"{avg_response_time*1000:.1f}ms avg, {success_rate:.1f}% success")
        
        return result
    
    def print_comparison_report(self):
        """Print detailed comparison report."""
        if not self.results:
            print("No benchmark results available")
            return
        
        print(f"\n{'='*80}")
        print("PERFORMANCE BENCHMARK RESULTS")
        print(f"{'='*80}")
        
        # Sort results by requests per second (descending)
        sorted_results = sorted(self.results, key=lambda x: x.requests_per_second, reverse=True)
        
        # Print table header
        print(f"{'Library':<20} {'RPS':<8} {'Avg(ms)':<8} {'Med(ms)':<8} {'P95(ms)':<8} "
              f"{'Mem(MB)':<8} {'CPU%':<6} {'Success%':<8} {'Errors':<6}")
        print("-" * 80)
        
        # Print results
        for result in sorted_results:
            print(f"{result.library_name:<20} "
                  f"{result.requests_per_second:<8.1f} "
                  f"{result.average_response_time:<8.1f} "
                  f"{result.median_response_time:<8.1f} "
                  f"{result.p95_response_time:<8.1f} "
                  f"{result.memory_usage_mb:<8.1f} "
                  f"{result.cpu_usage_percent:<6.1f} "
                  f"{result.success_rate:<8.1f} "
                  f"{result.error_count:<6}")
        
        # Print performance improvements
        if len(sorted_results) > 1:
            best = sorted_results[0]
            print(f"\n{'='*80}")
            print("PERFORMANCE IMPROVEMENTS")
            print(f"{'='*80}")
            
            for result in sorted_results[1:]:
                if result.requests_per_second > 0:
                    rps_improvement = ((best.requests_per_second - result.requests_per_second) / result.requests_per_second) * 100
                    time_improvement = ((result.average_response_time - best.average_response_time) / result.average_response_time) * 100
                    
                    print(f"{best.library_name} vs {result.library_name}:")
                    print(f"  Throughput: {rps_improvement:+.1f}% ({best.requests_per_second:.1f} vs {result.requests_per_second:.1f} req/s)")
                    print(f"  Response Time: {time_improvement:+.1f}% ({best.average_response_time:.1f} vs {result.average_response_time:.1f} ms)")
                    print()


class TestPerformanceBenchmarks(unittest.TestCase):
    """Test performance benchmarks."""
    
    def setUp(self):
        """Set up benchmark framework."""
        self.benchmark = PerformanceBenchmark()
        self.num_requests = 20  # Reduced for faster testing
    
    def test_requestx_sync_performance(self):
        """Test RequestX synchronous performance."""
        def requestx_get(url):
            return requestx.get(url, timeout=30)
        
        result = self.benchmark.benchmark_sync_library(requestx_get, "RequestX", self.num_requests)
        self.benchmark.results.append(result)
        
        # Basic performance assertions
        self.assertGreater(result.requests_per_second, 0)
        self.assertGreater(result.success_rate, 80)  # At least 80% success rate
        self.assertLess(result.average_response_time, 5000)  # Less than 5 seconds average
    
    def test_requestx_async_performance(self):
        """Test RequestX asynchronous performance."""
        async def requestx_get_async(url):
            return await requestx.get(url, timeout=30)
        
        async def run_async_benchmark():
            result = await self.benchmark.benchmark_async_library(requestx_get_async, "RequestX", self.num_requests)
            self.benchmark.results.append(result)
            
            # Basic performance assertions
            self.assertGreater(result.requests_per_second, 0)
            self.assertGreater(result.success_rate, 80)  # At least 80% success rate
            self.assertLess(result.average_response_time, 5000)  # Less than 5 seconds average
            
            return result
        
        asyncio.run(run_async_benchmark())
    
    @unittest.skipUnless(HAS_REQUESTS, "requests library not available")
    def test_requests_performance(self):
        """Test requests library performance for comparison."""
        def requests_get(url):
            return requests.get(url, timeout=30)
        
        result = self.benchmark.benchmark_sync_library(requests_get, "requests", self.num_requests)
        self.benchmark.results.append(result)
        
        # Basic assertions
        self.assertGreater(result.requests_per_second, 0)
        self.assertGreater(result.success_rate, 80)
    
    @unittest.skipUnless(HAS_HTTPX, "httpx library not available")
    def test_httpx_sync_performance(self):
        """Test httpx synchronous performance for comparison."""
        def httpx_get(url):
            return httpx.get(url, timeout=30)
        
        result = self.benchmark.benchmark_sync_library(httpx_get, "httpx", self.num_requests)
        self.benchmark.results.append(result)
        
        # Basic assertions
        self.assertGreater(result.requests_per_second, 0)
        self.assertGreater(result.success_rate, 80)
    
    @unittest.skipUnless(HAS_HTTPX, "httpx library not available")
    def test_httpx_async_performance(self):
        """Test httpx asynchronous performance for comparison."""
        async def httpx_get_async(url):
            async with httpx.AsyncClient() as client:
                return await client.get(url, timeout=30)
        
        async def run_async_benchmark():
            result = await self.benchmark.benchmark_async_library(httpx_get_async, "httpx", self.num_requests)
            self.benchmark.results.append(result)
            
            # Basic assertions
            self.assertGreater(result.requests_per_second, 0)
            self.assertGreater(result.success_rate, 80)
            
            return result
        
        asyncio.run(run_async_benchmark())
    
    @unittest.skipUnless(HAS_AIOHTTP, "aiohttp library not available")
    def test_aiohttp_performance(self):
        """Test aiohttp performance for comparison."""
        async def aiohttp_get(url):
            async with aiohttp.ClientSession() as session:
                async with session.get(url, timeout=aiohttp.ClientTimeout(total=30)) as response:
                    await response.text()
                    return response
        
        async def run_async_benchmark():
            result = await self.benchmark.benchmark_async_library(aiohttp_get, "aiohttp", self.num_requests)
            self.benchmark.results.append(result)
            
            # Basic assertions
            self.assertGreater(result.requests_per_second, 0)
            self.assertGreater(result.success_rate, 80)
            
            return result
        
        asyncio.run(run_async_benchmark())
    
    def test_concurrent_performance(self):
        """Test concurrent request performance."""
        def concurrent_requestx_test():
            """Test concurrent requests with RequestX."""
            start_time = time.time()
            
            def make_request(request_id):
                try:
                    response = requestx.get(f"{self.benchmark.base_url}/get?id={request_id}", timeout=30)
                    return response.status_code == 200
                except Exception:
                    return False
            
            # Use ThreadPoolExecutor for concurrent requests
            with ThreadPoolExecutor(max_workers=10) as executor:
                futures = [executor.submit(make_request, i) for i in range(self.num_requests)]
                results = [future.result() for future in as_completed(futures)]
            
            end_time = time.time()
            total_time = end_time - start_time
            success_count = sum(results)
            
            # Assertions
            self.assertGreater(success_count, self.num_requests * 0.8)  # At least 80% success
            self.assertLess(total_time, 30)  # Should complete within 30 seconds
            
            requests_per_second = self.num_requests / total_time
            print(f"Concurrent RequestX: {requests_per_second:.1f} req/s, "
                  f"{success_count}/{self.num_requests} successful")
            
            return requests_per_second
        
        rps = concurrent_requestx_test()
        self.assertGreater(rps, 0)
    
    def test_memory_efficiency(self):
        """Test memory efficiency during sustained load."""
        if not HAS_PSUTIL:
            self.skipTest("psutil not available for memory monitoring")
        
        initial_memory = self.benchmark.measure_memory_usage()
        
        # Make many requests to test memory usage
        for i in range(50):
            try:
                response = requestx.get(f"{self.benchmark.base_url}/get?id={i}", timeout=30)
                self.assertEqual(response.status_code, 200)
                
                # Access response content to ensure it's processed
                _ = response.text
                _ = response.json()
                
            except Exception as e:
                self.fail(f"Request {i} failed: {e}")
        
        final_memory = self.benchmark.measure_memory_usage()
        memory_increase = final_memory - initial_memory
        
        print(f"Memory usage: {initial_memory:.1f} MB -> {final_memory:.1f} MB "
              f"(+{memory_increase:.1f} MB)")
        
        # Memory increase should be reasonable (less than 100MB for 50 requests)
        self.assertLess(memory_increase, 100)
    
    def tearDown(self):
        """Clean up and print results."""
        if hasattr(self, 'benchmark') and self.benchmark.results:
            self.benchmark.print_comparison_report()


class TestLoadTesting(unittest.TestCase):
    """Test load handling capabilities."""
    
    def setUp(self):
        """Set up load testing."""
        self.base_url = "https://httpbin.org"
        self.timeout = 30
    
    def test_high_concurrency_load(self):
        """Test handling of high concurrency load."""
        num_concurrent = 20
        requests_per_thread = 5
        
        def worker_thread(thread_id):
            """Worker thread function."""
            results = []
            for i in range(requests_per_thread):
                try:
                    response = requestx.get(f"{self.base_url}/get?thread={thread_id}&req={i}", 
                                          timeout=self.timeout)
                    results.append(response.status_code == 200)
                except Exception:
                    results.append(False)
            return results
        
        # Start concurrent threads
        start_time = time.time()
        
        with ThreadPoolExecutor(max_workers=num_concurrent) as executor:
            futures = [executor.submit(worker_thread, i) for i in range(num_concurrent)]
            all_results = []
            for future in as_completed(futures):
                all_results.extend(future.result())
        
        end_time = time.time()
        total_time = end_time - start_time
        
        # Calculate metrics
        total_requests = num_concurrent * requests_per_thread
        successful_requests = sum(all_results)
        success_rate = (successful_requests / total_requests) * 100
        requests_per_second = total_requests / total_time
        
        print(f"High concurrency test: {num_concurrent} threads, {requests_per_thread} req/thread")
        print(f"Results: {successful_requests}/{total_requests} successful ({success_rate:.1f}%)")
        print(f"Throughput: {requests_per_second:.1f} req/s")
        print(f"Total time: {total_time:.2f} seconds")
        
        # Assertions
        self.assertGreater(success_rate, 80)  # At least 80% success rate
        self.assertGreater(requests_per_second, 1)  # At least 1 req/s
        self.assertLess(total_time, 60)  # Should complete within 60 seconds
    
    def test_sustained_load(self):
        """Test sustained load over time."""
        duration_seconds = 10
        target_rps = 5  # Target 5 requests per second
        
        start_time = time.time()
        end_time = start_time + duration_seconds
        
        request_count = 0
        success_count = 0
        
        while time.time() < end_time:
            try:
                response = requestx.get(f"{self.base_url}/get?sustained={request_count}", 
                                      timeout=self.timeout)
                if response.status_code == 200:
                    success_count += 1
                request_count += 1
                
                # Control request rate
                time.sleep(1.0 / target_rps)
                
            except Exception:
                request_count += 1
        
        actual_duration = time.time() - start_time
        actual_rps = request_count / actual_duration
        success_rate = (success_count / request_count) * 100 if request_count > 0 else 0
        
        print(f"Sustained load test: {duration_seconds}s duration")
        print(f"Results: {success_count}/{request_count} successful ({success_rate:.1f}%)")
        print(f"Actual RPS: {actual_rps:.1f} (target: {target_rps})")
        
        # Assertions
        self.assertGreater(success_rate, 80)  # At least 80% success rate
        self.assertGreater(request_count, duration_seconds * target_rps * 0.8)  # At least 80% of target


def run_full_benchmark_suite():
    """Run the full benchmark suite with detailed reporting."""
    print("Running Full Performance Benchmark Suite")
    print("=" * 60)
    
    benchmark = PerformanceBenchmark()
    num_requests = 30  # Reasonable number for comprehensive testing
    
    # Test RequestX sync
    def requestx_get(url):
        return requestx.get(url, timeout=30)
    
    result = benchmark.benchmark_sync_library(requestx_get, "RequestX", num_requests)
    benchmark.results.append(result)
    
    # Test RequestX async
    async def test_requestx_async():
        async def requestx_get_async(url):
            return await requestx.get(url, timeout=30)
        
        result = await benchmark.benchmark_async_library(requestx_get_async, "RequestX", num_requests)
        benchmark.results.append(result)
    
    asyncio.run(test_requestx_async())
    
    # Test comparison libraries if available
    if HAS_REQUESTS:
        def requests_get(url):
            return requests.get(url, timeout=30)
        
        result = benchmark.benchmark_sync_library(requests_get, "requests", num_requests)
        benchmark.results.append(result)
    
    if HAS_HTTPX:
        def httpx_get(url):
            return httpx.get(url, timeout=30)
        
        result = benchmark.benchmark_sync_library(httpx_get, "httpx", num_requests)
        benchmark.results.append(result)
        
        # Test httpx async
        async def test_httpx_async():
            async def httpx_get_async(url):
                async with httpx.AsyncClient() as client:
                    return await client.get(url, timeout=30)
            
            result = await benchmark.benchmark_async_library(httpx_get_async, "httpx", num_requests)
            benchmark.results.append(result)
        
        asyncio.run(test_httpx_async())
    
    if HAS_AIOHTTP:
        async def test_aiohttp():
            async def aiohttp_get(url):
                async with aiohttp.ClientSession() as session:
                    async with session.get(url, timeout=aiohttp.ClientTimeout(total=30)) as response:
                        await response.text()
                        return response
            
            result = await benchmark.benchmark_async_library(aiohttp_get, "aiohttp", num_requests)
            benchmark.results.append(result)
        
        asyncio.run(test_aiohttp())
    
    # Print comprehensive report
    benchmark.print_comparison_report()


if __name__ == '__main__':
    if len(sys.argv) > 1 and sys.argv[1] == '--full-benchmark':
        run_full_benchmark_suite()
    else:
        unittest.main(verbosity=2)