import unittest
import os
import sys
import psutil
import time
import asyncio
from typing import Dict, Any

class TestQuickBenchmark(unittest.TestCase):
    def setUp(self):
        """Set up test fixtures and environment."""
        # Ensure the python path includes the requestx package
        self.python_path_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', 'python'))
        sys.path.insert(0, self.python_path_dir)
        
        # Resource monitoring setup
        self.process = psutil.Process(os.getpid())
        self.initial_memory = self.process.memory_info().rss / 1024 / 1024  # MB
        
        # Resource limits for assertions
        self.max_memory_increase_mb = 50   # Maximum memory increase in MB
        self.max_cpu_usage_percent = 50    # Maximum CPU usage percentage
        self.max_execution_time_seconds = 30  # Maximum execution time

    def tearDown(self):
        """Clean up test environment."""
        # Remove added python path
        if self.python_path_dir in sys.path:
            sys.path.remove(self.python_path_dir)

    def test_requestx_sync_get(self):
        """Test requestx library with GET method using single concurrency via direct class usage."""
        try:
            # Import the benchmark classes
            from requestx.benchmark import RequestXBenchmarker, BenchmarkConfig
            
            # Create configuration for single concurrency GET test
            config = BenchmarkConfig(
                num_requests=10,
                concurrent_requests=1,
                timeout=30.0,
                warmup_requests=1,
                libraries=['requestx'],
                endpoints=['/get']
            )
            
            # Create RequestX benchmarker
            benchmarker = RequestXBenchmarker()
            
            # Test URL (using httpbin.org for real HTTP requests)
            test_url = "https://httpbin.org/get"
            method = "GET"
            
            # Start resource monitoring
            start_time = time.time()
            initial_cpu = self.process.cpu_percent()
            
            print(f"\nStarting RequestX benchmark:")
            print(f"URL: {test_url}")
            print(f"Method: {method}")
            print(f"Requests: {config.num_requests}")
            print(f"Concurrency: {config.concurrent_requests}")
            print(f"Timeout: {config.timeout}s")
            
            # Run the benchmark using the sync benchmarker
            result = benchmarker.benchmark_sync(
                url=test_url,
                method=method,
                num_requests=config.num_requests,
                concurrent_requests=config.concurrent_requests,
                timeout=config.timeout
            )
            
            execution_time = time.time() - start_time
            
            # Get final resource usage
            final_memory = self.process.memory_info().rss / 1024 / 1024  # MB
            memory_increase = final_memory - self.initial_memory
            final_cpu = self.process.cpu_percent()
            
            # Print resource usage for debugging
            print(f"\nResource Usage Summary:")
            print(f"Execution time: {execution_time:.2f} seconds")
            print(f"Memory increase: {memory_increase:.2f} MB")
            print(f"Initial memory: {self.initial_memory:.2f} MB")
            print(f"Final memory: {final_memory:.2f} MB")
            print(f"CPU usage: {final_cpu:.1f}%")
            
            # Print benchmark results
            print(f"\nBenchmark Results:")
            print(f"Library: {result.library}")
            print(f"Method: {result.method}")
            print(f"Concurrency: {result.concurrency}")
            print(f"Total requests: {result.total_requests}")
            print(f"Successful requests: {result.successful_requests}")
            print(f"Failed requests: {result.failed_requests}")
            print(f"Requests per second: {result.requests_per_second:.2f}")
            print(f"Average response time: {result.average_response_time_ms * 1000:.2f} ms")
            print(f"Median response time: {result.median_response_time_ms * 1000:.2f} ms")
            print(f"95th percentile: {result.p95_response_time_ms * 1000:.2f} ms")
            print(f"99th percentile: {result.p99_response_time_ms * 1000:.2f} ms")
            print(f"Error rate: {result.error_rate:.2f}%")
            print(f"CPU usage: {result.cpu_usage_percent:.1f}%")
            print(f"Memory usage: {result.memory_usage_mb:.2f} MB")
            
            # Basic functionality assertions
            self.assertEqual(result.library, "requestx", "Should test requestx library")
            self.assertEqual(result.method, method, "Should test GET method")
            self.assertEqual(result.concurrency, 1, "Should use 1 concurrent request")
            self.assertEqual(result.total_requests, config.num_requests, "Should make correct number of requests")
            
            # Performance assertions
            self.assertGreater(result.requests_per_second, 0, "RPS should be positive")
            self.assertGreater(result.average_response_time_ms, 0, "Response time should be positive")
            self.assertLessEqual(result.average_response_time_ms, 5.0, "Response time should be reasonable (<5s)")
            self.assertGreaterEqual(result.successful_requests, 1, "Should have at least 1 successful request")
            
            # Resource usage assertions
            self.assertLess(execution_time, self.max_execution_time_seconds,
                          f"Benchmark took too long: {execution_time:.2f}s > {self.max_execution_time_seconds}s")
            
            self.assertLess(memory_increase, self.max_memory_increase_mb,
                          f"Memory usage increased too much: {memory_increase:.2f}MB > {self.max_memory_increase_mb}MB")
            
            # CPU usage should be reasonable for single concurrency
            self.assertLess(result.cpu_usage_percent, self.max_cpu_usage_percent,
                          f"CPU usage too high: {result.cpu_usage_percent:.1f}% > {self.max_cpu_usage_percent}%")
            
            # Error rate should be low for simple GET requests
            self.assertLessEqual(result.error_rate, 10.0, "Error rate should be low (<10%) for simple requests")
            
            # Memory usage should be reported
            self.assertGreater(result.memory_usage_mb, 0, "Memory usage should be positive")
            
            # Verify performance is reasonable
            self.assertGreater(result.requests_per_second, 0.1, "Should achieve reasonable RPS")
            
            print(f"\n✅ All assertions passed!")
            
        except ImportError as e:
            self.fail(f"Failed to import benchmark classes: {e}")
        except Exception as e:
            self.fail(f"Benchmark test failed: {e}")

    def test_requestx_get_async(self):
        """Test requestx-async library with GET method using single concurrency via direct class usage."""
        async def run_async_benchmark():
            try:
                # Import the benchmark classes
                from requestx.benchmark import RequestXAsyncBenchmarker, BenchmarkConfig
                
                # Create configuration for single concurrency GET test with requestx-async
                config = BenchmarkConfig(
                    num_requests=10,
                    concurrent_requests=1,
                    timeout=30.0,
                    warmup_requests=1,
                    libraries=['requestx-async'],
                    endpoints=['/get']
                )
                
                # Create RequestX async benchmarker
                benchmarker = RequestXAsyncBenchmarker()
                
                # Test URL (using httpbin.org for real HTTP requests)
                test_url = "https://httpbin.org/get"
                method = "GET"
                
                # Start resource monitoring
                start_time = time.time()
                initial_cpu = self.process.cpu_percent()
                
                print(f"\nStarting RequestX Async benchmark:")
                print(f"URL: {test_url}")
                print(f"Method: {method}")
                print(f"Requests: {config.num_requests}")
                print(f"Concurrency: {config.concurrent_requests}")
                print(f"Timeout: {config.timeout}s")
                
                # Run the benchmark using the async benchmarker
                result = await benchmarker.benchmark_async(
                    url=test_url,
                    method=method,
                    num_requests=config.num_requests,
                    concurrent_requests=config.concurrent_requests,
                    timeout=config.timeout
                )
                
                execution_time = time.time() - start_time
                
                # Get final resource usage
                final_memory = self.process.memory_info().rss / 1024 / 1024  # MB
                memory_increase = final_memory - self.initial_memory
                final_cpu = self.process.cpu_percent()
                
                # Print resource usage for debugging
                print(f"\nAsync Resource Usage Summary:")
                print(f"Execution time: {execution_time:.2f} seconds")
                print(f"Memory increase: {memory_increase:.2f} MB")
                print(f"Initial memory: {self.initial_memory:.2f} MB")
                print(f"Final memory: {final_memory:.2f} MB")
                print(f"CPU usage: {final_cpu:.1f}%")
                
                # Print benchmark results
                print(f"\nAsync Benchmark Results:")
                print(f"Library: {result.library}")
                print(f"Method: {result.method}")
                print(f"Concurrency: {result.concurrency}")
                print(f"Total requests: {result.total_requests}")
                print(f"Successful requests: {result.successful_requests}")
                print(f"Failed requests: {result.failed_requests}")
                print(f"Requests per second: {result.requests_per_second:.2f}")
                print(f"Average response time: {result.average_response_time_ms * 1000:.2f} ms")
                print(f"Median response time: {result.median_response_time_ms * 1000:.2f} ms")
                print(f"95th percentile: {result.p95_response_time_ms * 1000:.2f} ms")
                print(f"99th percentile: {result.p99_response_time_ms * 1000:.2f} ms")
                print(f"Error rate: {result.error_rate:.2f}%")
                print(f"CPU usage: {result.cpu_usage_percent:.1f}%")
                print(f"Memory usage: {result.memory_usage_mb:.2f} MB")
                
                # Basic functionality assertions
                self.assertEqual(result.library, "requestx-async", "Should test requestx-async library")
                self.assertEqual(result.method, method, "Should test GET method")
                self.assertEqual(result.concurrency, 1, "Should use 1 concurrent request")
                self.assertEqual(result.total_requests, config.num_requests, "Should make correct number of requests")
                
                # Performance assertions
                self.assertGreater(result.requests_per_second, 0, "RPS should be positive")
                self.assertGreater(result.average_response_time_ms, 0, "Response time should be positive")
                self.assertLessEqual(result.average_response_time_ms, 5.0, "Response time should be reasonable (<5s)")
                self.assertGreaterEqual(result.successful_requests, 1, "Should have at least 1 successful request")
                
                # Resource usage assertions
                self.assertLess(execution_time, self.max_execution_time_seconds,
                              f"Async benchmark took too long: {execution_time:.2f}s > {self.max_execution_time_seconds}s")
                
                self.assertLess(memory_increase, self.max_memory_increase_mb,
                              f"Async memory usage increased too much: {memory_increase:.2f}MB > {self.max_memory_increase_mb}MB")
                
                # CPU usage should be reasonable for single concurrency
                self.assertLess(result.cpu_usage_percent, self.max_cpu_usage_percent,
                              f"Async CPU usage too high: {result.cpu_usage_percent:.1f}% > {self.max_cpu_usage_percent}%")
                
                # Error rate should be low for simple GET requests
                self.assertLessEqual(result.error_rate, 10.0, "Async error rate should be low (<10%) for simple requests")
                
                # Memory usage should be reported
                self.assertGreater(result.memory_usage_mb, 0, "Async memory usage should be positive")
                
                # Verify performance is reasonable
                self.assertGreater(result.requests_per_second, 0.1, "Should achieve reasonable async RPS")
                
                print(f"\n✅ All async assertions passed!")
                
            except ImportError as e:
                self.fail(f"Failed to import async benchmark classes: {e}")
            except Exception as e:
                self.fail(f"Async benchmark test failed: {e}")
        
        # Run the async test
        try:
            asyncio.run(run_async_benchmark())
        except Exception as e:
            self.fail(f"Failed to run async benchmark: {e}")

    def test_benchmark_config_validation(self):
        """Test that BenchmarkConfig works correctly."""
        try:
            from requestx.benchmark import BenchmarkConfig
            
            # Test default configuration
            default_config = BenchmarkConfig()
            self.assertEqual(default_config.num_requests, 100)
            self.assertEqual(default_config.concurrent_requests, 10)
            self.assertEqual(default_config.timeout, 30.0)
            self.assertEqual(default_config.warmup_requests, 10)
            self.assertEqual(default_config.libraries, ['requestx', 'requests', 'httpx', 'aiohttp'])
            self.assertEqual(default_config.endpoints, ['/get', '/post', '/put', '/delete'])
            
            # Test custom configuration
            custom_config = BenchmarkConfig(
                num_requests=5,
                concurrent_requests=1,
                timeout=15.0,
                warmup_requests=1,
                libraries=['requestx'],
                endpoints=['/get']
            )
            self.assertEqual(custom_config.num_requests, 5)
            self.assertEqual(custom_config.concurrent_requests, 1)
            self.assertEqual(custom_config.timeout, 15.0)
            self.assertEqual(custom_config.warmup_requests, 1)
            self.assertEqual(custom_config.libraries, ['requestx'])
            self.assertEqual(custom_config.endpoints, ['/get'])
            
            print("✅ BenchmarkConfig validation passed!")
            
        except ImportError as e:
            self.fail(f"Failed to import BenchmarkConfig: {e}")

if __name__ == '__main__':
    unittest.main(verbosity=2)