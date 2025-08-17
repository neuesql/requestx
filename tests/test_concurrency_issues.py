#!/usr/bin/env python3
"""
Focused concurrency issue detection for RequestX.
"""

import time
import threading
import concurrent.futures
from concurrent.futures import ThreadPoolExecutor, as_completed
import statistics
import traceback
import sys

try:
    import requestx

    HAS_REQUESTX = True
except ImportError:
    HAS_REQUESTX = False
    print("Warning: requestx not available")

try:
    import requests

    HAS_REQUESTS = True
except ImportError:
    HAS_REQUESTS = False
    print("Warning: requests not available")


def test_single_request_reliability():
    """Test if single requests work reliably"""
    print("\n=== Testing Single Request Reliability ===")

    test_url = "http://127.0.0.1:8000/get"

    # Test RequestX
    if HAS_REQUESTX:
        print("\nTesting RequestX single requests:")
        for i in range(10):
            try:
                start_time = time.time()
                response = requestx.get(test_url)
                end_time = time.time()
                print(
                    f"  Request {i+1}: Status {response.status_code}, Time: {(end_time-start_time)*1000:.1f}ms"
                )
            except Exception as e:
                print(f"  Request {i+1}: ERROR - {e}")
                traceback.print_exc()

    # Test Requests
    if HAS_REQUESTS:
        print("\nTesting Requests single requests:")
        for i in range(10):
            try:
                start_time = time.time()
                response = requests.get(test_url)
                end_time = time.time()
                print(
                    f"  Request {i+1}: Status {response.status_code}, Time: {(end_time-start_time)*1000:.1f}ms"
                )
            except Exception as e:
                print(f"  Request {i+1}: ERROR - {e}")


def test_thread_safety():
    """Test thread safety issues"""
    print("\n=== Testing Thread Safety ===")

    test_url = "http://127.0.0.1:8000/get"
    num_threads = 5
    requests_per_thread = 3

    def worker_function(http_func, thread_id, results_list, errors_list):
        """Worker function for thread safety test"""
        for i in range(requests_per_thread):
            try:
                start_time = time.time()
                response = http_func(test_url)
                end_time = time.time()

                result = {
                    "thread_id": thread_id,
                    "request_id": i,
                    "status_code": response.status_code,
                    "response_time": (end_time - start_time) * 1000,
                    "success": True,
                }
                results_list.append(result)
                print(
                    f"    Thread {thread_id}, Request {i+1}: Status {response.status_code}, Time: {result['response_time']:.1f}ms"
                )

            except Exception as e:
                error = {
                    "thread_id": thread_id,
                    "request_id": i,
                    "error": str(e),
                    "error_type": type(e).__name__,
                }
                errors_list.append(error)
                print(f"    Thread {thread_id}, Request {i+1}: ERROR - {e}")

    # Test RequestX thread safety
    if HAS_REQUESTX:
        print("\nTesting RequestX thread safety:")
        requestx_results = []
        requestx_errors = []

        threads = []
        for thread_id in range(num_threads):
            thread = threading.Thread(
                target=worker_function,
                args=(requestx.get, thread_id, requestx_results, requestx_errors),
            )
            threads.append(thread)
            thread.start()

        for thread in threads:
            thread.join()

        print(
            f"  RequestX Results: {len(requestx_results)} successful, {len(requestx_errors)} errors"
        )
        if requestx_errors:
            print("  RequestX Errors:")
            for error in requestx_errors:
                print(
                    f"    Thread {error['thread_id']}: {error['error_type']} - {error['error']}"
                )

    # Test Requests thread safety
    if HAS_REQUESTS:
        print("\nTesting Requests thread safety:")
        requests_results = []
        requests_errors = []

        threads = []
        for thread_id in range(num_threads):
            thread = threading.Thread(
                target=worker_function,
                args=(requests.get, thread_id, requests_results, requests_errors),
            )
            threads.append(thread)
            thread.start()

        for thread in threads:
            thread.join()

        print(
            f"  Requests Results: {len(requests_results)} successful, {len(requests_errors)} errors"
        )
        if requests_errors:
            print("  Requests Errors:")
            for error in requests_errors:
                print(
                    f"    Thread {error['thread_id']}: {error['error_type']} - {error['error']}"
                )


def test_threadpool_executor_issues():
    """Test issues with ThreadPoolExecutor"""
    print("\n=== Testing ThreadPoolExecutor Issues ===")

    test_url = "http://127.0.0.1:8000/get"

    def make_request(http_func, request_id):
        """Make a single request and return result"""
        try:
            start_time = time.time()
            response = http_func(test_url)
            end_time = time.time()

            return {
                "request_id": request_id,
                "status_code": response.status_code,
                "response_time": (end_time - start_time) * 1000,
                "success": True,
                "error": None,
            }
        except Exception as e:
            return {
                "request_id": request_id,
                "status_code": None,
                "response_time": None,
                "success": False,
                "error": str(e),
                "error_type": type(e).__name__,
            }

    # Test RequestX with ThreadPoolExecutor
    if HAS_REQUESTX:
        print("\nTesting RequestX with ThreadPoolExecutor:")

        with ThreadPoolExecutor(max_workers=10) as executor:
            futures = []
            for i in range(20):
                future = executor.submit(make_request, requestx.get, i)
                futures.append(future)

            results = []
            for future in as_completed(futures, timeout=30):
                try:
                    result = future.result()
                    results.append(result)
                    if result["success"]:
                        print(
                            f"  Request {result['request_id']}: Status {result['status_code']}, Time: {result['response_time']:.1f}ms"
                        )
                    else:
                        print(
                            f"  Request {result['request_id']}: ERROR - {result['error']}"
                        )
                except Exception as e:
                    print(f"  Future exception: {e}")

            successful = [r for r in results if r["success"]]
            failed = [r for r in results if not r["success"]]

            print(
                f"  RequestX ThreadPool Results: {len(successful)} successful, {len(failed)} failed"
            )

            if failed:
                print("  RequestX ThreadPool Errors:")
                for result in failed:
                    print(
                        f"    Request {result['request_id']}: {result.get('error_type', 'Unknown')} - {result['error']}"
                    )

    # Test Requests with ThreadPoolExecutor
    if HAS_REQUESTS:
        print("\nTesting Requests with ThreadPoolExecutor:")

        with ThreadPoolExecutor(max_workers=10) as executor:
            futures = []
            for i in range(20):
                future = executor.submit(make_request, requests.get, i)
                futures.append(future)

            results = []
            for future in as_completed(futures, timeout=30):
                try:
                    result = future.result()
                    results.append(result)
                    if result["success"]:
                        print(
                            f"  Request {result['request_id']}: Status {result['status_code']}, Time: {result['response_time']:.1f}ms"
                        )
                    else:
                        print(
                            f"  Request {result['request_id']}: ERROR - {result['error']}"
                        )
                except Exception as e:
                    print(f"  Future exception: {e}")

            successful = [r for r in results if r["success"]]
            failed = [r for r in results if not r["success"]]

            print(
                f"  Requests ThreadPool Results: {len(successful)} successful, {len(failed)} failed"
            )

            if failed:
                print("  Requests ThreadPool Errors:")
                for result in failed:
                    print(
                        f"    Request {result['request_id']}: {result.get('error_type', 'Unknown')} - {result['error']}"
                    )


def test_rapid_sequential_requests():
    """Test rapid sequential requests for race conditions"""
    print("\n=== Testing Rapid Sequential Requests ===")

    test_url = "http://127.0.0.1:8000/get"
    num_requests = 50

    # Test RequestX
    if HAS_REQUESTX:
        print("\nTesting RequestX rapid sequential requests:")

        start_time = time.time()
        successful = 0
        errors = 0
        response_times = []

        for i in range(num_requests):
            try:
                req_start = time.time()
                response = requestx.get(test_url)
                req_end = time.time()

                response_times.append((req_end - req_start) * 1000)
                successful += 1

                if i % 10 == 0:
                    print(f"  Completed {i+1}/{num_requests} requests")

            except Exception as e:
                errors += 1
                print(f"  Request {i+1}: ERROR - {e}")

        end_time = time.time()
        total_time = end_time - start_time

        print(f"  RequestX Sequential Results:")
        print(f"    Total time: {total_time:.2f}s")
        print(f"    Successful: {successful}/{num_requests}")
        print(f"    Errors: {errors}")
        print(f"    RPS: {successful/total_time:.1f}")
        if response_times:
            print(f"    Avg response time: {statistics.mean(response_times):.1f}ms")
            print(
                f"    Min/Max response time: {min(response_times):.1f}ms / {max(response_times):.1f}ms"
            )

    # Test Requests
    if HAS_REQUESTS:
        print("\nTesting Requests rapid sequential requests:")

        start_time = time.time()
        successful = 0
        errors = 0
        response_times = []

        for i in range(num_requests):
            try:
                req_start = time.time()
                response = requests.get(test_url)
                req_end = time.time()

                response_times.append((req_end - req_start) * 1000)
                successful += 1

                if i % 10 == 0:
                    print(f"  Completed {i+1}/{num_requests} requests")

            except Exception as e:
                errors += 1
                print(f"  Request {i+1}: ERROR - {e}")

        end_time = time.time()
        total_time = end_time - start_time

        print(f"  Requests Sequential Results:")
        print(f"    Total time: {total_time:.2f}s")
        print(f"    Successful: {successful}/{num_requests}")
        print(f"    Errors: {errors}")
        print(f"    RPS: {successful/total_time:.1f}")
        if response_times:
            print(f"    Avg response time: {statistics.mean(response_times):.1f}ms")
            print(
                f"    Min/Max response time: {min(response_times):.1f}ms / {max(response_times):.1f}ms"
            )


def main():
    """Run all concurrency issue tests"""
    print("Starting RequestX Concurrency Issue Detection...")
    print("=" * 60)

    # Run all tests
    test_single_request_reliability()
    test_thread_safety()
    test_threadpool_executor_issues()
    test_rapid_sequential_requests()

    print("\n" + "=" * 60)
    print("Concurrency issue detection completed!")


if __name__ == "__main__":
    main()
