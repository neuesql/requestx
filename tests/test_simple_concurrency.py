#!/usr/bin/env python3
"""
Simple concurrency test to verify RequestX optimizations.
"""

import time
import threading
from concurrent.futures import ThreadPoolExecutor, as_completed
import statistics

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


def test_simple_performance():
    """Simple performance test with a reliable endpoint"""

    # Use a local endpoint for testing
    test_url = "http://127.0.0.1:8000/get"

    def test_library(http_func, library_name, concurrency, total_requests):
        """Test a specific library with given concurrency"""
        print(
            f"\nTesting {library_name} - Concurrency: {concurrency}, Total: {total_requests}"
        )

        response_times = []
        errors = 0
        successful = 0

        def make_request():
            nonlocal errors, successful
            start_time = time.time()
            try:
                response = http_func(test_url)
                end_time = time.time()

                # Check response status
                if (
                    hasattr(response, "status_code")
                    and 200 <= response.status_code < 300
                ):
                    successful += 1
                    response_times.append((end_time - start_time) * 1000)
                else:
                    errors += 1

            except Exception as e:
                end_time = time.time()
                errors += 1
                print(f"  Error in {library_name}: {e}")

        # Execute requests
        start_time = time.time()

        with ThreadPoolExecutor(max_workers=concurrency) as executor:
            futures = [executor.submit(make_request) for _ in range(total_requests)]

            for future in as_completed(futures, timeout=60):
                try:
                    future.result()
                except Exception as e:
                    errors += 1
                    print(f"  Future error in {library_name}: {e}")

        end_time = time.time()
        total_time = end_time - start_time

        # Calculate metrics
        rps = total_requests / total_time if total_time > 0 else 0
        avg_time = statistics.mean(response_times) if response_times else 0
        success_rate = (successful / total_requests) * 100

        print(
            f"  Results: {rps:.1f} RPS, {avg_time:.1f}ms avg, {success_rate:.1f}% success, {errors} errors"
        )

        return {
            "library": library_name,
            "concurrency": concurrency,
            "rps": rps,
            "avg_time": avg_time,
            "success_rate": success_rate,
            "errors": errors,
            "total_time": total_time,
        }

    # Test configurations
    test_configs = [
        (1, 20),  # Sequential
        (5, 20),  # Low concurrency
        (10, 30),  # Medium concurrency
        (20, 40),  # High concurrency
    ]

    results = []

    for concurrency, total_requests in test_configs:
        print(f"\n{'='*60}")
        print(
            f"Testing Concurrency Level: {concurrency} ({total_requests} total requests)"
        )
        print(f"{'='*60}")

        # Test RequestX
        if HAS_REQUESTX:
            try:
                result = test_library(
                    requestx.get, "RequestX", concurrency, total_requests
                )
                results.append(result)
            except Exception as e:
                print(f"RequestX test failed: {e}")

        # Test requests
        if HAS_REQUESTS:
            try:
                result = test_library(
                    requests.get, "Requests", concurrency, total_requests
                )
                results.append(result)
            except Exception as e:
                print(f"Requests test failed: {e}")

    # Print summary
    print(f"\n{'='*60}")
    print("PERFORMANCE SUMMARY")
    print(f"{'='*60}")

    print(
        f"{'Library':<10} {'Concurrency':<12} {'RPS':<8} {'Avg Time':<10} {'Success':<8} {'Errors':<8}"
    )
    print("-" * 60)

    for result in results:
        print(
            f"{result['library']:<10} {result['concurrency']:<12} {result['rps']:<8.1f} {result['avg_time']:<10.1f} {result['success_rate']:<8.1f}% {result['errors']:<8}"
        )

    # Analyze RequestX scaling
    requestx_results = [r for r in results if r["library"] == "RequestX"]
    if len(requestx_results) >= 2:
        print(f"\nRequestX Scaling Analysis:")
        print("-" * 30)

        base_result = requestx_results[0]
        for result in requestx_results[1:]:
            expected_rps = base_result["rps"] * (
                result["concurrency"] / base_result["concurrency"]
            )
            actual_rps = result["rps"]
            efficiency = (actual_rps / expected_rps) * 100 if expected_rps > 0 else 0

            print(
                f"Concurrency {result['concurrency']:2d}: {efficiency:5.1f}% scaling efficiency ({actual_rps:.1f} vs {expected_rps:.1f} expected RPS)"
            )

    return results


if __name__ == "__main__":
    print("Starting simple concurrency performance test...")
    results = test_simple_performance()
    print("\nTest completed!")
