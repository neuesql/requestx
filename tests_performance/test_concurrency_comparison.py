"""Comprehensive benchmark comparing requestx vs httpx vs aiohttp across concurrency levels."""

import pytest
from http_benchmark.benchmark import BenchmarkConfiguration, BenchmarkRunner

TEST_URL = "http://localhost:80/get"
CONCURRENCY_LEVELS = [1, 2, 4, 6, 8, 10]


def run_benchmark(
    client_library: str, concurrency: int, is_async: bool = False
) -> dict:
    """Run a benchmark for a specific client library and concurrency level."""
    config = BenchmarkConfiguration(
        target_url=TEST_URL,
        http_method="GET",
        concurrency=concurrency,
        total_requests=100,
        client_library=client_library,
        is_async=is_async,
        timeout=30,
        verify_ssl=True,
        name=f"{client_library}_c{concurrency}",
    )
    runner = BenchmarkRunner(config)
    result = runner.run()
    return result.to_dict()


def print_sync_table(results: dict) -> None:
    """Print sync comparison table."""
    print("\n" + "=" * 100)
    print("SYNC CLIENT COMPARISON (Requests Per Second)")
    print("=" * 100)
    print(
        f"{'Concurrency':<12} {'requestx':>12} {'httpx':>12} {'requests':>12} {'urllib3':>12} {'rx/httpx':>10}"
    )
    print("-" * 100)

    for c in CONCURRENCY_LEVELS:
        rx = results.get(("requestx", c), {}).get("rps", 0)
        hx = results.get(("httpx", c), {}).get("rps", 0)
        req = results.get(("requests", c), {}).get("rps", 0)
        ul3 = results.get(("urllib3", c), {}).get("rps", 0)
        ratio = rx / hx if hx > 0 else 0
        print(
            f"{c:<12} {rx:>12.1f} {hx:>12.1f} {req:>12.1f} {ul3:>12.1f} {ratio:>9.2f}x"
        )

    print("=" * 100)


def print_async_table(results: dict) -> None:
    """Print async comparison table."""
    print("\n" + "=" * 80)
    print("ASYNC CLIENT COMPARISON (Requests Per Second)")
    print("=" * 80)
    print(
        f"{'Concurrency':<12} {'requestx':>12} {'httpx':>12} {'aiohttp':>12} {'rx/httpx':>10} {'rx/aiohttp':>12}"
    )
    print("-" * 80)

    for c in CONCURRENCY_LEVELS:
        rx = results.get(("requestx", c), {}).get("rps", 0)
        hx = results.get(("httpx", c), {}).get("rps", 0)
        aio = results.get(("aiohttp", c), {}).get("rps", 0)
        ratio_hx = rx / hx if hx > 0 else 0
        ratio_aio = rx / aio if aio > 0 else 0
        print(
            f"{c:<12} {rx:>12.1f} {hx:>12.1f} {aio:>12.1f} {ratio_hx:>9.2f}x {ratio_aio:>11.1%}"
        )

    print("=" * 80)


def print_latency_table(results: dict, is_async: bool) -> None:
    """Print latency comparison table (P99)."""
    mode = "ASYNC" if is_async else "SYNC"
    clients = (
        ["requestx", "httpx", "aiohttp"]
        if is_async
        else ["requestx", "httpx", "requests", "urllib3"]
    )

    print(f"\n{mode} CLIENT P99 LATENCY (ms)")
    print("-" * (12 + 12 * len(clients)))
    header = f"{'Concurrency':<12}" + "".join(f"{c:>12}" for c in clients)
    print(header)
    print("-" * (12 + 12 * len(clients)))

    for c in CONCURRENCY_LEVELS:
        row = f"{c:<12}"
        for client in clients:
            p99 = results.get((client, c), {}).get("p99", 0) * 1000
            row += f"{p99:>12.2f}"
        print(row)


@pytest.mark.network
def test_sync_concurrency_comparison():
    """Run sync benchmarks across all concurrency levels."""
    clients = ["requestx", "httpx", "requests", "urllib3"]
    results = {}

    for c in CONCURRENCY_LEVELS:
        print(f"\n--- Concurrency {c} ---")
        for client in clients:
            print(f"  Benchmarking {client}...")
            try:
                result = run_benchmark(client, c, is_async=False)
                results[(client, c)] = {
                    "rps": result["requests_per_second"],
                    "avg": result["avg_response_time"],
                    "p95": result["p95_response_time"],
                    "p99": result["p99_response_time"],
                    "errors": result["error_count"],
                }
            except Exception as e:
                print(f"    Error: {e}")
                results[(client, c)] = {
                    "rps": 0,
                    "avg": 0,
                    "p95": 0,
                    "p99": 0,
                    "errors": -1,
                }

    print_sync_table(results)
    print_latency_table(results, is_async=False)


@pytest.mark.network
def test_async_concurrency_comparison():
    """Run async benchmarks across all concurrency levels."""
    clients = ["requestx", "httpx", "aiohttp"]
    results = {}

    for c in CONCURRENCY_LEVELS:
        print(f"\n--- Concurrency {c} ---")
        for client in clients:
            print(f"  Benchmarking {client}...")
            try:
                result = run_benchmark(client, c, is_async=True)
                results[(client, c)] = {
                    "rps": result["requests_per_second"],
                    "avg": result["avg_response_time"],
                    "p95": result["p95_response_time"],
                    "p99": result["p99_response_time"],
                    "errors": result["error_count"],
                }
            except Exception as e:
                print(f"    Error: {e}")
                results[(client, c)] = {
                    "rps": 0,
                    "avg": 0,
                    "p95": 0,
                    "p99": 0,
                    "errors": -1,
                }

    print_async_table(results)
    print_latency_table(results, is_async=True)


@pytest.mark.network
def test_full_concurrency_comparison():
    """Run both sync and async benchmarks and print comprehensive comparison."""
    sync_clients = ["requestx", "httpx", "requests", "urllib3"]
    async_clients = ["requestx", "httpx", "aiohttp"]
    sync_results = {}
    async_results = {}

    # Run sync benchmarks
    print("\n" + "=" * 50)
    print("RUNNING SYNC BENCHMARKS")
    print("=" * 50)
    for c in CONCURRENCY_LEVELS:
        print(f"\n--- Concurrency {c} ---")
        for client in sync_clients:
            print(f"  Benchmarking {client}...")
            try:
                result = run_benchmark(client, c, is_async=False)
                sync_results[(client, c)] = {
                    "rps": result["requests_per_second"],
                    "avg": result["avg_response_time"],
                    "p95": result["p95_response_time"],
                    "p99": result["p99_response_time"],
                    "errors": result["error_count"],
                }
            except Exception as e:
                print(f"    Error: {e}")
                sync_results[(client, c)] = {
                    "rps": 0,
                    "avg": 0,
                    "p95": 0,
                    "p99": 0,
                    "errors": -1,
                }

    # Run async benchmarks
    print("\n" + "=" * 50)
    print("RUNNING ASYNC BENCHMARKS")
    print("=" * 50)
    for c in CONCURRENCY_LEVELS:
        print(f"\n--- Concurrency {c} ---")
        for client in async_clients:
            print(f"  Benchmarking {client}...")
            try:
                result = run_benchmark(client, c, is_async=True)
                async_results[(client, c)] = {
                    "rps": result["requests_per_second"],
                    "avg": result["avg_response_time"],
                    "p95": result["p95_response_time"],
                    "p99": result["p99_response_time"],
                    "errors": result["error_count"],
                }
            except Exception as e:
                print(f"    Error: {e}")
                async_results[(client, c)] = {
                    "rps": 0,
                    "avg": 0,
                    "p95": 0,
                    "p99": 0,
                    "errors": -1,
                }

    # Print results
    print_sync_table(sync_results)
    print_async_table(async_results)

    # Print summary
    print("\n" + "=" * 60)
    print("SUMMARY: requestx vs httpx speedup by concurrency")
    print("=" * 60)
    print(f"{'Concurrency':<12} {'Sync Speedup':>15} {'Async Speedup':>15}")
    print("-" * 60)
    for c in CONCURRENCY_LEVELS:
        sync_rx = sync_results.get(("requestx", c), {}).get("rps", 0)
        sync_hx = sync_results.get(("httpx", c), {}).get("rps", 0)
        async_rx = async_results.get(("requestx", c), {}).get("rps", 0)
        async_hx = async_results.get(("httpx", c), {}).get("rps", 0)
        sync_ratio = sync_rx / sync_hx if sync_hx > 0 else 0
        async_ratio = async_rx / async_hx if async_hx > 0 else 0
        print(f"{c:<12} {sync_ratio:>14.2f}x {async_ratio:>14.2f}x")
    print("=" * 60)
