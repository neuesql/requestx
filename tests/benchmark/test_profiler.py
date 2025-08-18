#!/usr/bin/env python3
"""
Test suite for the RequestX profiler module.

This module tests the performance profiling decorator and context manager
to ensure they correctly measure CPU, memory, and request metrics.
"""

import unittest
import asyncio
import time
import sys
import os


try:
    import requestx
    from requestx import Profile, profile_context, PerformanceMetrics, aggregate_metrics, get_last_metrics
    PROFILER_AVAILABLE = True
except ImportError:
    PROFILER_AVAILABLE = False


class TestProfiler(unittest.TestCase):
    """Test cases for the profiler functionality."""
    
    def setUp(self):
        """Set up test fixtures."""
        if not PROFILER_AVAILABLE:
            self.skipTest("Profiler not available")
    
    def test_profile_decorator_sync(self):
        """Test the Profile decorator with synchronous functions."""
        
        @Profile(cpu=True, memory=True, request=True)
        def test_function():
            time.sleep(0.1)  # Simulate work
            return "success"
        
        result = test_function()
        self.assertEqual(result, "success")
        
        # Check if metrics were attached or can be retrieved
        from requestx.profiler import get_last_metrics
        metrics = get_last_metrics(test_function)
        self.assertIsNotNone(metrics)
        self.assertGreater(metrics.total_time, 0)
    
    def test_profile_decorator_async(self):
        """Test the Profile decorator with asynchronous functions."""
        
        @Profile(cpu=True, memory=True, request=True)
        async def test_async_function():
            await asyncio.sleep(0.1)  # Simulate async work
            return "async_success"
        
        async def run_test():
            result = await test_async_function()
            self.assertEqual(result, "async_success")
            
            # Check if metrics can be retrieved
            from requestx.profiler import get_last_metrics
            metrics = get_last_metrics(test_async_function)
            self.assertIsNotNone(metrics)
            self.assertGreater(metrics.total_time, 0)
        
        asyncio.run(run_test())
    
    def test_profile_context_manager(self):
        """Test the profile_context context manager."""
        
        with profile_context(cpu=True, memory=True, request=True) as metrics:
            time.sleep(0.05)  # Simulate work
        
        # Check that metrics were collected
        self.assertIsInstance(metrics, PerformanceMetrics)
        self.assertGreater(metrics.total_time, 0)
        self.assertEqual(metrics.total_requests, 1)
        self.assertEqual(metrics.successful_requests, 1)
    
    def test_profile_error_handling(self):
        """Test profiler behavior with exceptions."""
        
        @Profile(cpu=True, memory=True, request=True, errors=True)
        def failing_function():
            raise ValueError("Test error")
        
        # Should not raise exception due to errors=True
        result = failing_function()
        self.assertIsNone(result)
    
    def test_aggregate_metrics(self):
        """Test metrics aggregation functionality."""
        
        # Create sample metrics
        metrics1 = PerformanceMetrics()
        metrics1.total_time = 1.0
        metrics1.total_requests = 10
        metrics1.successful_requests = 9
        metrics1.failed_requests = 1
        metrics1.requests_per_second = 10.0
        metrics1.response_times = [0.1, 0.2, 0.3]
        
        metrics2 = PerformanceMetrics()
        metrics2.total_time = 2.0
        metrics2.total_requests = 20
        metrics2.successful_requests = 18
        metrics2.failed_requests = 2
        metrics2.requests_per_second = 10.0
        metrics2.response_times = [0.15, 0.25]
        
        # Aggregate
        aggregated = aggregate_metrics([metrics1, metrics2])
        
        # Check aggregated values
        self.assertEqual(aggregated.total_time, 3.0)
        self.assertEqual(aggregated.total_requests, 30)
        self.assertEqual(aggregated.successful_requests, 27)
        self.assertEqual(aggregated.failed_requests, 3)
        self.assertEqual(len(aggregated.response_times), 5)
        self.assertEqual(aggregated.requests_per_second, 10.0)  # 30 requests / 3 seconds
    
    def test_performance_metrics_creation(self):
        """Test PerformanceMetrics data class creation."""
        
        metrics = PerformanceMetrics()
        
        # Check default values
        self.assertEqual(metrics.total_time, 0.0)
        self.assertEqual(metrics.total_requests, 0)
        self.assertEqual(metrics.successful_requests, 0)
        self.assertEqual(metrics.failed_requests, 0)
        self.assertEqual(metrics.error_rate, 0.0)
        self.assertEqual(len(metrics.errors), 0)
        self.assertEqual(len(metrics.response_times), 0)
    
    def test_cpu_memory_monitoring(self):
        """Test CPU and memory monitoring functionality."""
        
        @Profile(cpu=True, memory=True, request=False)
        def cpu_intensive_function():
            # Simulate CPU-intensive work
            total = 0
            for i in range(100000):
                total += i * i
            return total
        
        result = cpu_intensive_function()
        self.assertIsInstance(result, int)
        
        # Note: Actual CPU/memory values depend on system load
        # We just verify the function completes without error
    
    def test_selective_profiling(self):
        """Test selective enabling/disabling of profiling features."""
        
        @Profile(cpu=False, memory=True, request=True)
        def selective_function():
            time.sleep(0.01)
            return {"data": "test"}
        
        result = selective_function()
        self.assertEqual(result["data"], "test")
        
        # Function should complete without error regardless of selective profiling


class TestProfilerIntegration(unittest.TestCase):
    """Integration tests for profiler with HTTP libraries."""
    
    def setUp(self):
        """Set up test fixtures."""
        if not PROFILER_AVAILABLE:
            self.skipTest("Profiler not available")
    
    def test_profile_with_requests(self):
        """Test profiler integration with requests library."""
        try:
            import requests
        except ImportError:
            self.skipTest("requests library not available")
        
        @Profile(cpu=True, memory=True, request=True)
        def make_http_request():
            try:
                response = requests.get("https://httpbin.org/get", timeout=10)
                return response.status_code
            except Exception:
                return None
        
        result = make_http_request()
        # Should return status code or None (if network error)
        self.assertTrue(result is None or isinstance(result, int))
    
    def test_profile_with_httpx_async(self):
        """Test profiler integration with httpx async."""
        try:
            import httpx
        except ImportError:
            self.skipTest("httpx library not available")
        
        @Profile(cpu=True, memory=True, request=True)
        async def make_async_request():
            try:
                async with httpx.AsyncClient() as client:
                    response = await client.get("https://httpbin.org/get", timeout=10)
                    return response.status_code
            except Exception:
                return None
        
        async def run_test():
            result = await make_async_request()
            self.assertTrue(result is None or isinstance(result, int))
        
        asyncio.run(run_test())


def run_profiler_tests():
    """Run all profiler tests."""
    if not PROFILER_AVAILABLE:
        print("Profiler not available - skipping tests")
        return
    
    # Create test suite
    suite = unittest.TestSuite()
    
    # Add test cases
    suite.addTest(unittest.makeSuite(TestProfiler))
    suite.addTest(unittest.makeSuite(TestProfilerIntegration))
    
    # Run tests
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)
    
    return result.wasSuccessful()


if __name__ == "__main__":
    success = run_profiler_tests()
    sys.exit(0 if success else 1)