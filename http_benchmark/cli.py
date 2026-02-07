"""Command-line interface for the HTTP benchmark framework."""

import argparse
import sys
from .benchmark import BenchmarkRunner
from .models.benchmark_configuration import BenchmarkConfiguration
from .storage import ResultStorage
from .utils.logging import app_logger


def main():
    """Main entry point for the CLI."""
    parser = argparse.ArgumentParser(description="HTTP Client Performance Benchmark Framework")
    parser.add_argument("--url", required=True, help="Target URL to benchmark")
    parser.add_argument(
        "--client",
        required=False,
        choices=["requests", "requestx", "httpx", "aiohttp", "urllib3", "pycurl"],
        help="HTTP client library to use (required unless --compare is used)",
    )
    parser.add_argument(
        "--method",
        default="GET",
        choices=["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS", "STREAM"],
        help="HTTP method to use",
    )
    parser.add_argument("--concurrency", type=int, default=10, help="Number of concurrent requests")
    parser.add_argument("--duration", type=int, default=30, help="Duration of benchmark in seconds")
    parser.add_argument("--headers", help="HTTP headers in JSON format")
    parser.add_argument("--body", help="Request body content")
    parser.add_argument("--async", dest="is_async", action="store_true", help="Use async requests")
    parser.add_argument("--output", help="Output file for results")
    parser.add_argument("--compare", nargs="+", help="Compare multiple client libraries")
    parser.add_argument(
        "--verify-ssl",
        dest="verify_ssl",
        action="store_true",
        default=False,
        help="Enable SSL verification (disabled by default)",
    )

    args = parser.parse_args()

    # Validate that either --client or --compare is provided
    if not args.client and not args.compare:
        parser.error("--client is required unless --compare is used")
    if args.client and args.compare:
        parser.error("--client and --compare cannot be used together")

    try:
        if args.compare:
            # Compare multiple client libraries
            compare_clients(args)
        else:
            # Run a single benchmark
            run_single_benchmark(args)
    except Exception as e:
        app_logger.error(f"Error running benchmark: {str(e)}")
        sys.exit(1)


def run_single_benchmark(args) -> None:
    """Run a single benchmark."""
    app_logger.info(f"Starting benchmark for {args.url} using {args.client}")

    # Parse headers if provided
    headers = {}
    if args.headers:
        import json

        try:
            headers = json.loads(args.headers)
        except json.JSONDecodeError:
            app_logger.error("Invalid JSON in headers argument")
            return

    # Create benchmark configuration
    config = BenchmarkConfiguration(
        target_url=args.url,
        http_method=args.method,
        headers=headers,
        body=args.body or "",
        concurrency=args.concurrency,
        duration_seconds=args.duration,
        client_library=args.client,
        is_async=args.is_async,
        verify_ssl=args.verify_ssl,
    )

    # Run the benchmark
    runner = BenchmarkRunner(config)
    result = runner.run()

    # Print results
    print("Benchmark Results:")
    print(f"  Client Library: {result.client_library}")
    print(f"  URL: {result.url}")
    print(f"  HTTP Method: {result.http_method}")
    print(f"  Duration: {result.duration:.2f}s")
    print(f"  Requests: {result.requests_count}")
    print(f"  RPS: {result.requests_per_second:.2f}")
    print(f"  Avg Response Time: {result.avg_response_time:.3f}s")
    print(f"  Min Response Time: {result.min_response_time:.3f}s")
    print(f"  Max Response Time: {result.max_response_time:.3f}s")
    print(f"  95th Percentile: {result.p95_response_time:.3f}s")
    print(f"  99th Percentile: {result.p99_response_time:.3f}s")
    print(f"  Error Rate: {result.error_rate:.2f}%")
    print(f"  CPU Usage (avg): {result.cpu_usage_avg:.2f}%")
    print(f"  Memory Usage (avg): {result.memory_usage_avg:.2f}MB")

    # Store results
    storage = ResultStorage()
    storage.save_result(result)
    app_logger.info(f"Benchmark result saved with ID: {result.id}")


def compare_clients(args) -> None:
    """Compare multiple client libraries."""
    app_logger.info(f"Comparing clients: {', '.join(args.compare)} for {args.url}")

    results = []

    for client in args.compare:
        app_logger.info(f"Running benchmark with {client}")

        # Create benchmark configuration
        config = BenchmarkConfiguration(
            target_url=args.url,
            http_method=args.method,
            concurrency=args.concurrency,
            duration_seconds=args.duration,
            client_library=client,
            is_async=args.is_async,
            verify_ssl=args.verify_ssl,
        )

        # Run the benchmark
        runner = BenchmarkRunner(config)
        result = runner.run()
        results.append(result)

        # Store result
        storage = ResultStorage()
        storage.save_result(result)
        app_logger.info(f"Result for {client} saved with ID: {result.id}")

    # Print comparison
    print(f"\nComparison Results for {args.url}:")
    print(f"{'Client':<12} {'RPS':<10} {'Avg Time':<12} {'Error Rate':<12} {'CPU %':<8} {'Memory MB':<10}")
    print("-" * 70)

    for result in results:
        print(
            f"{result.client_library:<12} {result.requests_per_second:<10.2f} "
            f"{result.avg_response_time:<12.3f} {result.error_rate:<12.2f} "
            f"{result.cpu_usage_avg:<8.2f} {result.memory_usage_avg:<10.2f}"
        )


if __name__ == "__main__":
    main()
