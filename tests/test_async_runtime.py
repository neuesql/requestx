#!/usr/bin/env python3
"""
Test async context detection and runtime management functionality.

This module tests the enhanced async context detection using pyo3-asyncio,
runtime management for both sync and async execution, tokio runtime integration
with Python asyncio, event loop detection, and coroutine creation.

Requirements tested: 2.1, 2.2, 2.3, 2.4, 7.3
"""

import asyncio
import unittest
import threading
import time
import sys
import os

# Add the parent directory to the path to import requestx
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

try:
    import requestx
except ImportError as e:
    print(f"Failed to import requestx: {e}")
    print("Make sure to build the extension with: uv run maturin develop")
    sys.exit(1)


class TestAsyncContextDetection(unittest.TestCase):
    """Test async context detection functionality."""

    def setUp(self):
        """Set up test fixtures."""
        self.test_url = "https://httpbin.org/get"
        self.timeout_url = "https://httpbin.org/delay/1"

    def test_sync_context_detection(self):
        """Test that sync context is properly detected."""
        # In sync context, should return Response object directly
        response = requestx.get(self.test_url)

        # Should not be a coroutine
        self.assertFalse(asyncio.iscoroutine(response))
        self.assertFalse(asyncio.isfuture(response))

        # Should be a Response object
        self.assertTrue(hasattr(response, "status_code"))
        self.assertTrue(hasattr(response, "text"))
        self.assertTrue(hasattr(response, "json"))
        self.assertEqual(response.status_code, 200)

    def test_sync_context_all_methods(self):
        """Test sync context detection for all HTTP methods."""
        methods_and_urls = [
            (requestx.get, "https://httpbin.org/get"),
            (requestx.post, "https://httpbin.org/post"),
            (requestx.put, "https://httpbin.org/put"),
            (requestx.delete, "https://httpbin.org/delete"),
            (requestx.head, "https://httpbin.org/get"),
            (requestx.options, "https://httpbin.org/get"),
            (requestx.patch, "https://httpbin.org/patch"),
        ]

        for method_func, url in methods_and_urls:
            with self.subTest(method=method_func.__name__, url=url):
                response = method_func(url)

                # Should not be a coroutine in sync context
                self.assertFalse(asyncio.iscoroutine(response))
                self.assertFalse(asyncio.isfuture(response))

                # Should be a Response object
                self.assertTrue(hasattr(response, "status_code"))
                self.assertIn(
                    response.status_code, [200, 204]
                )  # Some methods return 204

    async def test_async_context_detection(self):
        """Test that async context is properly detected."""
        # In async context, should return a coroutine or future
        result = requestx.get(self.test_url)

        # Should be awaitable (coroutine or future)
        self.assertTrue(asyncio.iscoroutine(result) or asyncio.isfuture(result))

        # Should be able to await it
        response = await result

        # After awaiting, should be a Response object
        self.assertTrue(hasattr(response, "status_code"))
        self.assertTrue(hasattr(response, "text"))
        self.assertTrue(hasattr(response, "json"))
        self.assertEqual(response.status_code, 200)

    async def test_async_context_all_methods(self):
        """Test async context detection for all HTTP methods."""
        methods_and_urls = [
            (requestx.get, "https://httpbin.org/get"),
            (requestx.post, "https://httpbin.org/post"),
            (requestx.put, "https://httpbin.org/put"),
            (requestx.delete, "https://httpbin.org/delete"),
            (requestx.head, "https://httpbin.org/get"),
            (requestx.options, "https://httpbin.org/get"),
            (requestx.patch, "https://httpbin.org/patch"),
        ]

        for method_func, url in methods_and_urls:
            with self.subTest(method=method_func.__name__, url=url):
                result = method_func(url)

                # Should be awaitable in async context
                self.assertTrue(asyncio.iscoroutine(result) or asyncio.isfuture(result))

                # Should be able to await it
                response = await result

                # Should be a Response object
                self.assertTrue(hasattr(response, "status_code"))
                self.assertIn(response.status_code, [200, 204])

    async def test_async_generic_request_method(self):
        """Test async context detection for generic request method."""
        methods = ["GET", "POST", "PUT", "DELETE", "HEAD", "OPTIONS", "PATCH"]

        for method in methods:
            with self.subTest(method=method):
                url = f"https://httpbin.org/{method.lower()}"
                if method == "HEAD" or method == "OPTIONS":
                    url = "https://httpbin.org/get"

                result = requestx.request(method, url)

                # Should be awaitable in async context
                self.assertTrue(asyncio.iscoroutine(result) or asyncio.isfuture(result))

                # Should be able to await it
                response = await result

                # Should be a Response object
                self.assertTrue(hasattr(response, "status_code"))
                self.assertIn(response.status_code, [200, 204])


class TestRuntimeManagement(unittest.TestCase):
    """Test runtime management for both sync and async execution."""

    def setUp(self):
        """Set up test fixtures."""
        self.test_url = "https://httpbin.org/get"

    def test_sync_execution_performance(self):
        """Test sync execution performance and resource management."""
        start_time = time.time()

        # Make multiple sync requests
        responses = []
        for _ in range(5):
            response = requestx.get(self.test_url)
            responses.append(response)

        end_time = time.time()

        # All should be successful
        for response in responses:
            self.assertEqual(response.status_code, 200)

        # Should complete in reasonable time (less than 10 seconds for 5 requests)
        self.assertLess(end_time - start_time, 10.0)

    async def test_async_execution_performance(self):
        """Test async execution performance and resource management."""
        start_time = time.time()

        # Make multiple async requests concurrently
        tasks = [requestx.get(self.test_url) for _ in range(5)]
        responses = await asyncio.gather(*tasks)

        end_time = time.time()

        # All should be successful
        for response in responses:
            self.assertEqual(response.status_code, 200)

        # Should complete faster than sync (less than 5 seconds for concurrent requests)
        self.assertLess(end_time - start_time, 5.0)

    def test_mixed_sync_async_execution(self):
        """Test mixed sync and async execution in different threads."""
        results = {}

        def sync_worker():
            """Worker function for sync requests."""
            try:
                response = requestx.get(self.test_url)
                results["sync"] = response.status_code
            except Exception as e:
                results["sync_error"] = str(e)

        async def async_worker():
            """Worker function for async requests."""
            try:
                response = await requestx.get(self.test_url)
                results["async"] = response.status_code
            except Exception as e:
                results["async_error"] = str(e)

        # Start sync worker in a thread
        sync_thread = threading.Thread(target=sync_worker)
        sync_thread.start()

        # Run async worker in current thread
        asyncio.run(async_worker())

        # Wait for sync thread to complete
        sync_thread.join(timeout=10)

        # Both should succeed
        self.assertEqual(results.get("sync"), 200)
        self.assertEqual(results.get("async"), 200)
        self.assertNotIn("sync_error", results)
        self.assertNotIn("async_error", results)


class TestEventLoopIntegration(unittest.TestCase):
    """Test event loop detection and coroutine creation."""

    def setUp(self):
        """Set up test fixtures."""
        self.test_url = "https://httpbin.org/get"

    def test_no_event_loop_sync_execution(self):
        """Test execution when no event loop is running."""
        # Ensure no event loop is running
        try:
            loop = asyncio.get_running_loop()
            self.fail("Event loop should not be running in sync test")
        except RuntimeError:
            pass  # Expected - no event loop running

        # Should execute synchronously
        response = requestx.get(self.test_url)
        self.assertFalse(asyncio.iscoroutine(response))
        self.assertEqual(response.status_code, 200)

    async def test_event_loop_async_execution(self):
        """Test execution when event loop is running."""
        # Ensure event loop is running
        loop = asyncio.get_running_loop()
        self.assertIsNotNone(loop)

        # Should return awaitable
        result = requestx.get(self.test_url)
        self.assertTrue(asyncio.iscoroutine(result) or asyncio.isfuture(result))

        # Should be able to await
        response = await result
        self.assertEqual(response.status_code, 200)

    def test_nested_event_loop_handling(self):
        """Test handling of nested event loop scenarios."""

        def sync_with_nested_async():
            """Function that tries to run async code in sync context."""
            # This should work - sync execution
            response = requestx.get(self.test_url)
            self.assertEqual(response.status_code, 200)

            # Try to create a new event loop and run async code
            async def nested_async():
                result = requestx.get(self.test_url)
                self.assertTrue(asyncio.iscoroutine(result) or asyncio.isfuture(result))
                response = await result
                return response.status_code

            # Run in new event loop
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            try:
                status_code = loop.run_until_complete(nested_async())
                self.assertEqual(status_code, 200)
            finally:
                loop.close()
                asyncio.set_event_loop(None)

        sync_with_nested_async()

    async def test_concurrent_requests_in_async_context(self):
        """Test concurrent requests in async context."""
        # Create multiple concurrent requests
        urls = [
            "https://httpbin.org/get",
            "https://httpbin.org/get?param1=value1",
            "https://httpbin.org/get?param2=value2",
        ]

        # All should return awaitables
        tasks = [requestx.get(url) for url in urls]
        for task in tasks:
            self.assertTrue(asyncio.iscoroutine(task) or asyncio.isfuture(task))

        # Should be able to await all concurrently
        responses = await asyncio.gather(*tasks)

        # All should succeed
        for response in responses:
            self.assertEqual(response.status_code, 200)


class TestRuntimeBehavior(unittest.TestCase):
    """Test runtime behavior and resource management."""

    def setUp(self):
        """Set up test fixtures."""
        self.test_url = "https://httpbin.org/get"

    def test_runtime_resource_cleanup(self):
        """Test that runtime resources are properly managed."""
        # Make multiple requests to test resource management
        for i in range(10):
            response = requestx.get(self.test_url)
            self.assertEqual(response.status_code, 200)

            # Verify response can be used
            self.assertIsInstance(response.text, str)
            self.assertGreater(len(response.text), 0)

    async def test_async_runtime_resource_cleanup(self):
        """Test async runtime resource cleanup."""
        # Make multiple async requests
        for i in range(10):
            response = await requestx.get(self.test_url)
            self.assertEqual(response.status_code, 200)

            # Verify response can be used
            self.assertIsInstance(response.text, str)
            self.assertGreater(len(response.text), 0)

    def test_error_handling_in_sync_context(self):
        """Test error handling in sync context."""
        with self.assertRaises(ValueError):
            requestx.get("not-a-valid-url")

    async def test_error_handling_in_async_context(self):
        """Test error handling in async context."""
        with self.assertRaises(ValueError):
            await requestx.get("not-a-valid-url")

    def test_timeout_handling_sync(self):
        """Test timeout handling in sync context."""
        with self.assertRaises(Exception):  # Should raise timeout error
            requestx.get("https://httpbin.org/delay/10", timeout=1)

    async def test_timeout_handling_async(self):
        """Test timeout handling in async context."""
        with self.assertRaises(Exception):  # Should raise timeout error
            await requestx.get("https://httpbin.org/delay/10", timeout=1)


def run_async_tests():
    """Run async tests using asyncio."""

    async def run_all_async_tests():
        # Create test instances and set them up
        context_test = TestAsyncContextDetection()
        context_test.setUp()

        runtime_test = TestRuntimeManagement()
        runtime_test.setUp()

        event_loop_test = TestEventLoopIntegration()
        event_loop_test.setUp()

        behavior_test = TestRuntimeBehavior()
        behavior_test.setUp()

        # Define async test methods with their names
        async_tests = [
            (
                "test_async_context_detection",
                context_test.test_async_context_detection(),
            ),
            (
                "test_async_context_all_methods",
                context_test.test_async_context_all_methods(),
            ),
            (
                "test_async_generic_request_method",
                context_test.test_async_generic_request_method(),
            ),
            (
                "test_async_execution_performance",
                runtime_test.test_async_execution_performance(),
            ),
            (
                "test_event_loop_async_execution",
                event_loop_test.test_event_loop_async_execution(),
            ),
            (
                "test_concurrent_requests_in_async_context",
                event_loop_test.test_concurrent_requests_in_async_context(),
            ),
            (
                "test_async_runtime_resource_cleanup",
                behavior_test.test_async_runtime_resource_cleanup(),
            ),
            (
                "test_error_handling_in_async_context",
                behavior_test.test_error_handling_in_async_context(),
            ),
            (
                "test_timeout_handling_async",
                behavior_test.test_timeout_handling_async(),
            ),
        ]

        print("Running async tests...")
        for i, (test_name, test_coro) in enumerate(async_tests):
            try:
                await test_coro
                print(f"  ✓ {test_name} passed")
            except Exception as e:
                print(f"  ✗ {test_name} failed: {e}")
                raise

        print("All async tests passed!")

    # Run async tests
    asyncio.run(run_all_async_tests())


if __name__ == "__main__":
    print("Testing async context detection and runtime management...")

    # Run sync tests
    print("\nRunning sync tests...")
    suite = unittest.TestSuite()

    # Add sync test methods
    suite.addTest(TestAsyncContextDetection("test_sync_context_detection"))
    suite.addTest(TestAsyncContextDetection("test_sync_context_all_methods"))
    suite.addTest(TestRuntimeManagement("test_sync_execution_performance"))
    suite.addTest(TestRuntimeManagement("test_mixed_sync_async_execution"))
    suite.addTest(TestEventLoopIntegration("test_no_event_loop_sync_execution"))
    suite.addTest(TestEventLoopIntegration("test_nested_event_loop_handling"))
    suite.addTest(TestRuntimeBehavior("test_runtime_resource_cleanup"))
    suite.addTest(TestRuntimeBehavior("test_error_handling_in_sync_context"))
    suite.addTest(TestRuntimeBehavior("test_timeout_handling_sync"))

    runner = unittest.TextTestRunner(verbosity=2)
    sync_result = runner.run(suite)

    # Run async tests
    print("\nRunning async tests...")
    try:
        run_async_tests()
        async_success = True
    except Exception as e:
        print(f"Async tests failed: {e}")
        async_success = False

    # Summary
    print(f"\nTest Summary:")
    print(f"Sync tests: {'PASSED' if sync_result.wasSuccessful() else 'FAILED'}")
    print(f"Async tests: {'PASSED' if async_success else 'FAILED'}")

    if sync_result.wasSuccessful() and async_success:
        print("All async context detection and runtime management tests passed!")
        sys.exit(0)
    else:
        print("Some tests failed!")
        sys.exit(1)
