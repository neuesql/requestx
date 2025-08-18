#!/usr/bin/env python3
"""
Unit tests for the RequestX profiler and benchmarking system.

This module contains unit tests for the @Profile decorator, benchmark
suite, and performance measurement functionality.
"""

import asyncio
import sys
import os
import unittest
from unittest.mock import patch, MagicMock
import time

# Add the parent directory to sys.path to import requestx
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', '..', 'python'))

try:
    import requestx
    from requestx import Profile, profile_context, PerformanceMetrics
    PROFILER_AVAILABLE = True
except ImportError:
    PROFILER_AVAILABLE = False


class TestProfileDecorator(unittest.TestCase):
    """Test cases for the @Profile decorator."""
    
    def setUp(self):
        """Set up test fixtures."""
        if not PROFILER_AVAILABLE:
            self.skipTest("RequestX profiler not available. Build RequestX first with: make build-dev")
    
    def test_profile_decorator_basic_functionality(self):
        """Test that @Profile decorator works with basic function."""
        @Profile(cpu=True, memory=True, request=True)
        def simulate_work():
            """Simulate some CPU and memory intensive work."""
            total = 0
            for i in range(1000):  # Reduced for faster testing
                total += i * i
            
            # Simulate memory allocation
            data = [i for i in range(100)]  # Reduced for faster testing
            
            return {"result": total, "data_size": len(data)}
        
        # Execute function with profiling
        result = simulate_work()
        
        # Verify function executed correctly
        self.assertIsInstance(result, dict)
        self.assertIn("result", result)
        self.assertIn("data_size", result)
        self.assertEqual(result["data_size"], 100)
        self.assertGreater(result["result"], 0)
    
    def test_profile_decorator_with_sleep(self):
        """Test @Profile decorator with I/O simulation."""
        @Profile(cpu=True, memory=True, request=True)
        def simulate_io_work():
            """Simulate I/O work."""
            time.sleep(0.01)  # Very short sleep for testing
            return {"status": "completed"}
        
        result = simulate_io_work()
        self.assertEqual(result["status"], "completed")
    
    def test_profile_decorator_error_handling_enabled(self):
        """Test @Profile decorator with error handling enabled."""
        @Profile(cpu=True, memory=True, request=True, errors=True)
        def function_with_error():
            """Function that raises an error."""
            time.sleep(0.001)  # Some work before error
            raise ValueError("Intentional error for testing")
        
        # Should not raise exception due to errors=True
        result = function_with_error()
        self.assertIsNone(result)
    
    def test_profile_decorator_error_handling_disabled(self):
        """Test @Profile decorator with error handling disabled."""
        @Profile(cpu=True, memory=True, request=True, errors=False)
        def function_with_error_strict():
            """Function that raises an error (strict mode)."""
            raise RuntimeError("This should be raised")
        
        # Should raise exception due to errors=False
        with self.assertRaises(RuntimeError):
            function_with_error_strict()


class TestProfileContextManager(unittest.TestCase):
    """Test cases for the profile_context context manager."""
    
    def setUp(self):
        """Set up test fixtures."""
        if not PROFILER_AVAILABLE:
            self.skipTest("RequestX profiler not available. Build RequestX first with: make build-dev")
    
    def test_profile_context_basic_usage(self):
        """Test basic usage of profile_context."""
        with profile_context(cpu=True, memory=True, request=True) as metrics:
            # CPU intensive task
            result = sum(i * i for i in range(1000))  # Reduced for faster testing
            
            # Memory allocation
            temp_data = list(range(100))  # Reduced for faster testing
        
        # Verify metrics object exists and has expected attributes
        self.assertIsNotNone(metrics)
        self.assertTrue(hasattr(metrics, 'total_time'))
        self.assertGreaterEqual(metrics.total_time, 0)
        
        # Verify computation was correct
        expected_result = sum(i * i for i in range(1000))
        self.assertEqual(result, expected_result)
        self.assertEqual(len(temp_data), 100)


class TestAsyncProfiling(unittest.TestCase):
    """Test cases for async function profiling."""
    
    def setUp(self):
        """Set up test fixtures."""
        if not PROFILER_AVAILABLE:
            self.skipTest("RequestX profiler not available. Build RequestX first with: make build-dev")
    
    def test_async_profile_decorator(self):
        """Test @Profile decorator with async functions."""
        @Profile(cpu=True, memory=True, request=True)
        async def async_work():
            """Simulate async work."""
            # Simulate async I/O
            await asyncio.sleep(0.01)  # Very short sleep for testing
            
            # CPU work
            result = sum(i for i in range(100))  # Reduced for faster testing
            
            # Memory allocation
            data = [i * 2 for i in range(50)]  # Reduced for faster testing
            
            return {"async_result": result, "data_length": len(data)}
        
        # Execute async function
        result = asyncio.run(async_work())
        
        self.assertIsInstance(result, dict)
        self.assertIn("async_result", result)
        self.assertIn("data_length", result)
        self.assertEqual(result["data_length"], 50)
        expected_result = sum(i for i in range(100))
        self.assertEqual(result["async_result"], expected_result)


class TestHttpProfiling(unittest.TestCase):
    """Test cases for HTTP request profiling."""
    
    def setUp(self):
        """Set up test fixtures."""
        if not PROFILER_AVAILABLE:
            self.skipTest("RequestX profiler not available. Build RequestX first with: make build-dev")
    
    @patch('requests.get')
    def test_http_request_profiling_success(self, mock_get):
        """Test HTTP request profiling with mocked successful response."""
        # Mock successful response
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.content = b"test response content"
        mock_get.return_value = mock_response
        
        @Profile(cpu=True, memory=True, request=True)
        def make_http_request():
            """Make an HTTP request with profiling."""
            try:
                import requests
                response = requests.get("https://httpbin.org/get", timeout=10)
                return {
                    "status_code": response.status_code,
                    "response_size": len(response.content)
                }
            except Exception as e:
                return {"error": str(e)}
        
        result = make_http_request()
        
        self.assertIsInstance(result, dict)
        self.assertIn("status_code", result)
        self.assertIn("response_size", result)
        self.assertEqual(result["status_code"], 200)
        self.assertEqual(result["response_size"], 21)  # Length of "test response content"
    
    @patch('requests.get')
    def test_http_request_profiling_error(self, mock_get):
        """Test HTTP request profiling with mocked error response."""
        # Mock exception
        mock_get.side_effect = Exception("Connection error")
        
        @Profile(cpu=True, memory=True, request=True)
        def make_http_request():
            """Make an HTTP request with profiling."""
            try:
                import requests
                response = requests.get("https://httpbin.org/get", timeout=10)
                return {
                    "status_code": response.status_code,
                    "response_size": len(response.content)
                }
            except Exception as e:
                return {"error": str(e)}
        
        result = make_http_request()
        
        self.assertIsInstance(result, dict)
        self.assertIn("error", result)
        self.assertEqual(result["error"], "Connection error")


class TestBenchmarkIntegration(unittest.TestCase):
    """Test cases for benchmark suite integration."""
    
    def test_benchmark_config_creation(self):
        """Test creation of BenchmarkConfig."""
        try:
            from requestx import BenchmarkConfig
            
            config = BenchmarkConfig(
                num_requests=5,
                concurrent_requests=2,
                timeout=10.0,
                warmup_requests=1,
                libraries=['requestx'],
                endpoints=['/get']
            )
            
            self.assertIsNotNone(config)
            self.assertEqual(config.num_requests, 5)
            self.assertEqual(config.concurrent_requests, 2)
            self.assertEqual(config.timeout, 10.0)
            self.assertEqual(config.warmup_requests, 1)
            self.assertEqual(config.libraries, ['requestx'])
            self.assertEqual(config.endpoints, ['/get'])
            
        except ImportError:
            self.skipTest("Benchmark suite not available")
    
    def test_benchmark_runner_creation(self):
        """Test creation of BenchmarkRunner."""
        try:
            from requestx import BenchmarkConfig, BenchmarkRunner
            
            config = BenchmarkConfig(
                num_requests=1,
                concurrent_requests=1,
                timeout=5.0,
                warmup_requests=0,
                libraries=['requestx'],
                endpoints=['/get']
            )
            
            runner = BenchmarkRunner(config)
            self.assertIsNotNone(runner)
            
        except ImportError:
            self.skipTest("Benchmark suite not available")


class TestPerformanceMetrics(unittest.TestCase):
    """Test cases for PerformanceMetrics functionality."""
    
    def setUp(self):
        """Set up test fixtures."""
        if not PROFILER_AVAILABLE:
            self.skipTest("RequestX profiler not available. Build RequestX first with: make build-dev")
    
    def test_performance_metrics_attributes(self):
        """Test that PerformanceMetrics has expected attributes."""
        # This test verifies the PerformanceMetrics class can be imported
        # and has the expected interface
        self.assertTrue(hasattr(requestx, 'PerformanceMetrics'))


class TestModuleImports(unittest.TestCase):
    """Test cases for module imports and availability."""
    
    def test_requestx_import(self):
        """Test that requestx module can be imported."""
        if PROFILER_AVAILABLE:
            self.assertTrue(hasattr(requestx, 'Profile'))
            self.assertTrue(hasattr(requestx, 'profile_context'))
            self.assertTrue(hasattr(requestx, 'PerformanceMetrics'))
        else:
            self.skipTest("RequestX profiler not available")
    
    def test_benchmark_imports(self):
        """Test that benchmark-related imports work."""
        try:
            from requestx import BenchmarkConfig, BenchmarkRunner
            self.assertIsNotNone(BenchmarkConfig)
            self.assertIsNotNone(BenchmarkRunner)
        except ImportError:
            self.skipTest("Benchmark suite not available")


if __name__ == "__main__":
    # Configure test runner
    unittest.main(verbosity=2, buffer=True)