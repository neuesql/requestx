"""Sync GET benchmark comparing requestx vs httpx vs requests."""

import pytest
from http_benchmark.benchmark import BenchmarkConfiguration, BenchmarkRunner

# Test URL - using localhost for faster benchmarks
TEST_URL = "http://localhost/get"


def run_benchmark(client_library: str) -> dict:
    """Run a sync benchmark for a specific client library."""
    config = BenchmarkConfiguration(
        target_url=TEST_URL,
        http_method="GET",
        concurrency=1,
        total_requests=100,
        client_library=client_library,
        is_async=False,
        timeout=30,
        verify_ssl=True,
        name=f"{client_library}_sync_get",
    )
    runner = BenchmarkRunner(config)
    result = runner.run()
    return result.to_dict()


def print_comparison(results: list[dict]) -> None:
    """Print a comparison table of benchmark results."""
    print("\n" + "=" * 80)
    print("SYNC GET BENCHMARK COMPARISON")
    print("=" * 80)
    print(
        f"{'Client':<15} {'RPS':>10} {'Avg (ms)':>12} {'P95 (ms)':>12} {'P99 (ms)':>12} {'Errors':>8}"
    )
    print("-" * 80)

    for r in sorted(results, key=lambda x: x["requests_per_second"], reverse=True):
        print(
            f"{r['client_library']:<15} "
            f"{r['requests_per_second']:>10.2f} "
            f"{r['avg_response_time'] * 1000:>12.2f} "
            f"{r['p95_response_time'] * 1000:>12.2f} "
            f"{r['p99_response_time'] * 1000:>12.2f} "
            f"{r['error_count']:>8}"
        )

    print("=" * 80)

    # Find the fastest
    fastest = max(results, key=lambda x: x["requests_per_second"])
    print(
        f"\nFastest: {fastest['client_library']} ({fastest['requests_per_second']:.2f} RPS)"
    )


@pytest.mark.network
def test_sync_get_requestx():
    """Benchmark requestx sync GET performance."""
    result = run_benchmark("requestx")
    assert result["error_count"] == 0, f"Errors occurred: {result['error_count']}"
    assert result["requests_per_second"] > 0
    print(
        f"\nrequestx sync: {result['requests_per_second']:.2f} RPS, avg {result['avg_response_time']*1000:.2f}ms"
    )


@pytest.mark.network
def test_sync_get_httpx():
    """Benchmark httpx sync GET performance."""
    result = run_benchmark("httpx")
    assert result["error_count"] == 0, f"Errors occurred: {result['error_count']}"
    assert result["requests_per_second"] > 0
    print(
        f"\nhttpx sync: {result['requests_per_second']:.2f} RPS, avg {result['avg_response_time']*1000:.2f}ms"
    )


@pytest.mark.network
def test_sync_get_requests():
    """Benchmark requests sync GET performance."""
    result = run_benchmark("requests")
    assert result["error_count"] == 0, f"Errors occurred: {result['error_count']}"
    assert result["requests_per_second"] > 0
    print(
        f"\nrequests sync: {result['requests_per_second']:.2f} RPS, avg {result['avg_response_time']*1000:.2f}ms"
    )


@pytest.mark.network
def test_sync_get_urllib3():
    """Benchmark urllib3 sync GET performance."""
    result = run_benchmark("urllib3")
    assert result["error_count"] == 0, f"Errors occurred: {result['error_count']}"
    assert result["requests_per_second"] > 0
    print(
        f"\nurllib3 sync: {result['requests_per_second']:.2f} RPS, avg {result['avg_response_time']*1000:.2f}ms"
    )


@pytest.mark.network
def test_sync_get_comparison():
    """Run full sync comparison benchmark across all sync-capable clients."""
    clients = ["requestx", "httpx", "requests", "urllib3"]
    results = []

    for client in clients:
        print(f"\nBenchmarking {client}...")
        result = run_benchmark(client)
        results.append(result)

    print_comparison(results)

    # Verify requestx is competitive (within 50% of the fastest)
    requestx_result = next(r for r in results if r["client_library"] == "requestx")
    fastest_rps = max(r["requests_per_second"] for r in results)

    assert requestx_result["requests_per_second"] >= fastest_rps * 0.5, (
        f"requestx ({requestx_result['requests_per_second']:.2f} RPS) "
        f"is more than 50% slower than fastest ({fastest_rps:.2f} RPS)"
    )
