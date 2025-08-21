#!/usr/bin/env python3
"""
Main benchmark runner script for RequestX performance testing.

This script runs comprehensive benchmarks comparing RequestX against other
HTTP libraries and exports results in multiple formats including CSV and JSON.
"""

import argparse
import json
import logging
import os
import sqlite3
import sys
from dataclasses import asdict
from datetime import datetime
from typing import List, Dict, Any, Optional

import logging_loki
from dotenv import load_dotenv

load_dotenv(".env")


# Add the parent directory to sys.path to import requestx
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', '..', 'python'))

from requestx.benchmark import (
    BenchmarkRunner, 
    BenchmarkConfig, 
    BenchmarkResult,
    RequestXSyncBenchmarker,
    RequestXAsyncBenchmarker,
    HttpxSyncBenchmarker,
    HttpxAsyncBenchmarker,
    RequestsBenchmarker,
    AiohttpBenchmarker
)

# Cloud export constants and functionality
DEFAULT_LOKI_TAGS = {
    "service": "requestx",
    "type": "benchmark",
    "environment": "production"
}

# Global Loki logger instance
_loki_logger = None


def _get_loki_logger(
    source_name: str = "requestx-benchmark",
    tags: Optional[Dict[str, str]] = None
) -> Optional[logging.Logger]:
    """Get or create a Loki logger instance.
    
    Args:
        source_name: Identifier for the log source
        tags: Additional tags to include with logs
        
    Returns:
        Logger instance or None if configuration is invalid
    """
    global _loki_logger

    # Get configuration from environment variables
    loki_url = os.getenv("LOKI_URL")
    loki_user = os.getenv("LOKI_USER")
    loki_password = os.getenv("LOKI_PASSWORD")
    
    if not loki_url or not loki_user or not loki_password:
        print("Error: LOKI_URL, LOKI_USER, and LOKI_PASSWORD must be set in environment variables")
        return None
    
    # Create logger if it doesn't exist or needs updating
    if _loki_logger is None:
        # Default tags for filtering in Grafana
        default_tags = DEFAULT_LOKI_TAGS.copy()
        default_tags["application"] = source_name
        
        if tags:
            default_tags.update(tags)
        
        try:
            # Create Loki handler
            handler = logging_loki.LokiHandler(
                url=loki_url,
                tags=default_tags,
                auth=(loki_user, loki_password),
                version="1"
            )
            
            # Create logger
            _loki_logger = logging.getLogger("requestx-benchmark")
            _loki_logger.setLevel(logging.INFO)
            
            # Remove existing handlers to avoid duplicates
            for h in _loki_logger.handlers[:]:
                _loki_logger.removeHandler(h)
            
            _loki_logger.addHandler(handler)
            
        except Exception as e:
            print(f"Error creating Loki logger: {e}")
            return None
    
    return _loki_logger


def export_to_cloud(
    results: List[Dict[str, Any]],
    source_name: str = "requestx-benchmark",
    tags: Optional[Dict[str, str]] = None
) -> bool:
    if not results:
        print("No results to export")
        return False
    
    # Get Loki logger
    logger = _get_loki_logger(source_name, tags)
    if logger is None:
        return False
    
    try:
        # Send each result as JSON log entry
        exported_count = 0
        for result in results:
            # Convert result to dict if needed
            if hasattr(result, '__dict__'):
                log_data = asdict(result)
            elif hasattr(result, 'to_dict'):
                log_data = result.to_dict()
            else:
                log_data = dict(result)
            
            # Send benchmark data as JSON (following test pattern)
            logger.info(json.dumps(log_data))
            exported_count += 1
        
        print(f"Successfully exported {exported_count} benchmark results to Grafana Cloud Logs")
        return True
        
    except Exception as e:
        print(f"Error exporting to Grafana Cloud: {e}")
        return False


def save_to_database(results: List[BenchmarkResult], db_path: str) -> bool:
    """Save benchmark results to SQLite database.
    
    Args:
        results: List of BenchmarkResult objects to save
        db_path: Path to the SQLite database file
        
    Returns:
        True if successful, False otherwise
    """
    if not results:
        print("No results to save to database")
        return False
    
    try:
        # Connect to database
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        
        # Create table if it doesn't exist (with schema matching performance.db)
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS benchmark (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                test_time DATETIME DEFAULT CURRENT_TIMESTAMP,
                
                library TEXT NOT NULL,
                concurrency INTEGER NOT NULL,
                method TEXT NOT NULL,
                requests_per_second REAL NOT NULL,
                average_response_time_ms REAL NOT NULL,
                median_response_time_ms REAL NOT NULL,
                p95_response_time_ms REAL NOT NULL,
                p99_response_time_ms REAL NOT NULL,
                error_rate REAL NOT NULL,
                total_requests INTEGER NOT NULL,
                successful_requests INTEGER NOT NULL,
                failed_requests INTEGER NOT NULL,
                cpu_usage_percent REAL NOT NULL,
                memory_usage_mb REAL NOT NULL,
                timestamp REAL NOT NULL
            )
        """)
        
        # Create indexes if they don't exist
        cursor.execute("CREATE INDEX IF NOT EXISTS idx_library ON benchmark(library)")
        cursor.execute("CREATE INDEX IF NOT EXISTS idx_method ON benchmark(method)")
        cursor.execute("CREATE INDEX IF NOT EXISTS idx_concurrency ON benchmark(concurrency)")
        cursor.execute("CREATE INDEX IF NOT EXISTS idx_test_time ON benchmark(test_time)")
        cursor.execute("CREATE INDEX IF NOT EXISTS idx_created_at ON benchmark(test_time)")
        
        # Insert results (excluding id and test_time as they are auto-generated)
        saved_count = 0
        for result in results:
            cursor.execute("""
                INSERT INTO benchmark (
                    library, concurrency, method, requests_per_second,
                    average_response_time_ms, median_response_time_ms,
                    p95_response_time_ms, p99_response_time_ms, error_rate,
                    total_requests, successful_requests, failed_requests,
                    cpu_usage_percent, memory_usage_mb, timestamp
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """, (
                result.library,
                result.concurrency,
                result.method,
                result.requests_per_second,
                result.average_response_time_ms,
                result.median_response_time_ms,
                result.p95_response_time_ms,
                result.p99_response_time_ms,
                result.error_rate,
                result.total_requests,
                result.successful_requests,
                result.failed_requests,
                result.cpu_usage_percent,
                result.memory_usage_mb,
                result.timestamp
            ))
            saved_count += 1
        
        # Commit changes
        conn.commit()
        conn.close()
        return True
        
    except Exception as e:
        print(f"Error saving to database: {e}")
        return False


def parse_arguments():
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(
        description="Run RequestX performance benchmarks",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Basic usage - run quick benchmarks with default settings
  ./requestx-benchmark --quick
  
  # Full benchmarks with custom concurrency and request count
  ./requestx-benchmark --concurrency 1,10,100,1000 --requests 500
  
  # Test specific HTTP methods only
  ./requestx-benchmark --methods GET,POST --sizes small,medium
  
  # Use custom test server (default is https://httpbin.org)
  ./requestx-benchmark --host http://localhost:8080 --quick
  ./requestx-benchmark --host https://api.example.com --methods GET
  
  # Compare specific libraries
  ./requestx-benchmark --libraries requestx,requests --quick
  ./requestx-benchmark --libraries requestx --verbose
  
  # Custom output and reporting
  ./requestx-benchmark --output-dir ./my_results --verbose
  ./requestx-benchmark --no-csv --output-dir ./json_only
  
  # Performance testing scenarios
  ./requestx-benchmark --concurrency 1,50,100,500 --requests 1000 --timeout 60
  ./requestx-benchmark --methods GET,POST,PUT,DELETE --sizes small,large --warmup 10
  
  # Development and debugging
  ./requestx-benchmark --quick --verbose --libraries requestx
  ./requestx-benchmark --host http://localhost:3000 --methods GET --requests 10 --verbose
        """
    )
    
    # Show help if no arguments provided
    if len(sys.argv) == 1:
        parser.print_help()
        sys.exit(0)
    
    # Test configuration
    parser.add_argument(
        '--concurrency', 
        type=str, 
        default='1,10,100',
        help='Comma-separated concurrency levels (default: 1,10,100)'
    )
    
    parser.add_argument(
        '--requests', 
        type=int, 
        default=100,
        help='Number of requests per test (default: 100)'
    )
    
    parser.add_argument(
        '--methods', 
        type=str, 
        default='GET,POST,PUT,DELETE,HEAD,OPTIONS,PATCH',
        help='Comma-separated HTTP methods to test (default: all)'
    )
    
    parser.add_argument(
        '--sizes', 
        type=str, 
        default='small,medium,large',
        help='Comma-separated request sizes to test (default: small,medium,large)'
    )
    
    parser.add_argument(
        '--timeout', 
        type=float, 
        default=300.0,
        help='Request timeout in seconds (default: 300.0)'
    )
    
    parser.add_argument(
        '--warmup', 
        type=int, 
        default=5,
        help='Number of warmup requests (default: 5)'
    )
    
    # Quick test mode
    parser.add_argument(
        '--quick', 
        action='store_true',
        help='Run quick benchmarks (fewer requests and concurrency levels)'
    )
    
    # Output configuration
    parser.add_argument(
        '--output-dir', 
        type=str, 
        default='.',
        help='Output directory for results (default: current directory)'
    )
    
    parser.add_argument(
        '--no-csv', 
        action='store_true',
        help='Disable CSV output'
    )
    
    parser.add_argument(
        '--no-json', 
        action='store_true',
        help='Disable JSON output'
    )
    
    parser.add_argument(
        '--no-cloud', 
        action='store_true',
        help='Disable cloud export to Grafana Loki'
    )
    
    parser.add_argument(
        '--no-db', 
        action='store_true',
        help='Disable database persistence to SQLite'
    )
    
    parser.add_argument(
        '--db-path', 
        type=str, 
        default='performance.db',
        help='Path to SQLite database file (default: performance.db)'
    )
    
    # Library selection
    parser.add_argument(
        '--libraries', 
        type=str, 
        default='requestx-sync,requestx-async,httpx-sync,httpx-async,requests,aiohttp',
        help='Comma-separated libraries to test (default: all available)'
    )
    
    # Host configuration
    parser.add_argument(
        '--host',
        type=str,
        default='http://localhost:8080',
        help='Base URL for the test server (default: http://localhost:8080)'
    )
    
    # Verbose output
    parser.add_argument(
        '--verbose', '-v', 
        action='store_true',
        help='Enable verbose output'
    )
    
    return parser.parse_args()


def create_config_from_args(args) -> BenchmarkConfig:
    """Create BenchmarkConfig from command line arguments."""
    
    # Parse libraries
    libraries = [x.strip().lower() for x in args.libraries.split(',')]
    
    # Parse HTTP methods to endpoints
    endpoints = []
    for method in args.methods.split(','):
        method = method.strip().upper()
        if method == 'GET':
            endpoints.append('/get')
        elif method == 'POST':
            endpoints.append('/post')
        elif method == 'PUT':
            endpoints.append('/put')
        elif method == 'DELETE':
            endpoints.append('/delete')
        else:
            endpoints.append(f'/{method.lower()}')
    
    # Quick mode adjustments
    if args.quick:
        concurrent_requests = 10
        num_requests = 20
        warmup_requests = 2
    else:
        concurrent_requests = int(args.concurrency.split(',')[0])  # Use first concurrency level
        num_requests = args.requests
        warmup_requests = args.warmup
    
    return BenchmarkConfig(
        num_requests=num_requests,
        concurrent_requests=concurrent_requests,
        timeout=args.timeout,
        warmup_requests=warmup_requests,
        libraries=libraries,
        endpoints=endpoints
    )


def filter_libraries(runner: BenchmarkRunner, libraries: str) -> None:
    """Filter benchmarkers based on requested libraries."""
    requested = [lib.strip().lower() for lib in libraries.split(',')]
    
    # Remove unwanted benchmarkers
    to_remove = []
    for lib_name in runner.benchmarkers.keys():
        if lib_name not in requested:
            to_remove.append(lib_name)
    
    for lib_name in to_remove:
        del runner.benchmarkers[lib_name]


def generate_report(results: List[BenchmarkResult], output_dir: str) -> None:
    """Generate a comprehensive benchmark report."""
    
    if not results:
        return
    
    print("\n" + "="*50)
    print("BENCHMARK REPORT")
    print("="*50)
    
    # Group by library
    library_stats = {}
    for result in results:
        lib = result.library
        if lib not in library_stats:
            library_stats[lib] = {
                'total_tests': 0,
                'avg_rps': 0,
                'avg_response_time': 0,
                'avg_memory': 0,
                'total_errors': 0,
                'total_requests': 0
            }
        
        stats = library_stats[lib]
        stats['total_tests'] += 1
        stats['avg_rps'] += result.requests_per_second
        stats['avg_response_time'] += result.average_response_time_ms
        stats['avg_memory'] += result.memory_usage_mb
        stats['total_errors'] += result.failed_requests
        stats['total_requests'] += result.total_requests
    
    # Calculate averages
    for lib, stats in library_stats.items():
        if stats['total_tests'] > 0:
            stats['avg_rps'] /= stats['total_tests']
            stats['avg_response_time'] /= stats['total_tests']
            stats['avg_memory'] /= stats['total_tests']
    
    # Print summary
    print("\nSUMMARY:")
    print(f"{'Library':<15} {'Avg RPS':<10} {'Avg RT (ms)':<12} {'Memory (MB)':<12} {'Error Rate':<10}")
    print("-" * 70)
    
    for lib, stats in sorted(library_stats.items()):
        error_rate = (stats['total_errors'] / stats['total_requests'] * 100) if stats['total_requests'] > 0 else 0
        print(f"{lib:<15} {stats['avg_rps']:<10.2f} {stats['avg_response_time']*1000:<12.2f} {stats['avg_memory']:<12.2f} {error_rate:<10.2f}%")
    
    # Best performers
    print("\nBEST PERFORMERS:")
    
    best_rps = max(results, key=lambda r: r.requests_per_second)
    print(f"Highest RPS: {best_rps.library} - {best_rps.requests_per_second:.2f} RPS")
    
    best_latency = min(results, key=lambda r: r.average_response_time_ms)
    print(f"Lowest Latency: {best_latency.library} - {best_latency.average_response_time_ms * 1000:.2f}ms")
    
    if hasattr(results[0], 'memory_usage_mb'):
        best_memory = min(results, key=lambda r: r.memory_usage_mb)
        print(f"Lowest Memory: {best_memory.library} - {best_memory.memory_usage_mb:.2f}MB")
    
    report_file = os.path.join(output_dir, f"benchmark_report_{datetime.now().strftime('%Y%m%d_%H%M%S')}.md")
    
    with open(report_file, 'w') as f:
        f.write("# RequestX Benchmark Report\n\n")
        f.write(f"Generated: {datetime.now().isoformat()}\n\n")
        
        # Summary statistics
        f.write("## Summary\n\n")
        
        # Group by library
        library_stats = {}
        for result in results:
            lib = result.library
            if lib not in library_stats:
                library_stats[lib] = {
                    'total_tests': 0,
                    'avg_rps': 0,
                    'avg_response_time': 0,
                    'avg_memory': 0,
                    'total_errors': 0,
                    'total_requests': 0
                }
            
            stats = library_stats[lib]
            stats['total_tests'] += 1
            stats['avg_rps'] += result.requests_per_second
            stats['avg_response_time'] += result.average_response_time_ms
            stats['avg_memory'] += result.memory_usage_mb
            stats['total_errors'] += result.failed_requests
            stats['total_requests'] += result.total_requests
        
        # Calculate averages
        for lib, stats in library_stats.items():
            if stats['total_tests'] > 0:
                stats['avg_rps'] /= stats['total_tests']
                stats['avg_response_time'] /= stats['total_tests']
                stats['avg_memory'] /= stats['total_tests']
        
        # Write summary table
        f.write("| Library | Avg RPS | Avg Response Time (ms) | Avg Memory (MB) | Error Rate (%) |\n")
        f.write("|---------|---------|------------------------|-----------------|----------------|\n")
        
        for lib, stats in sorted(library_stats.items()):
            error_rate = (stats['total_errors'] / stats['total_requests'] * 100) if stats['total_requests'] > 0 else 0
            f.write(f"| {lib} | {stats['avg_rps']:.2f} | {stats['avg_response_time']*1000:.2f} | {stats['avg_memory']:.2f} | {error_rate:.2f} |\n")
        
        # Best performers
        f.write("\n## Best Performers\n\n")
        
        best_rps = max(results, key=lambda r: r.requests_per_second)
        f.write(f"**Highest RPS:** {best_rps.library} - {best_rps.requests_per_second:.2f} RPS\n")
        
        best_latency = min(results, key=lambda r: r.average_response_time_ms)
        f.write(f"**Lowest Latency:** {best_latency.library} - {best_latency.average_response_time_ms * 1000:.2f}ms\n")
        
        best_memory = min(results, key=lambda r: r.memory_usage_mb)
        f.write(f"**Lowest Memory:** {best_memory.library} - {best_memory.memory_usage_mb:.2f}MB\n")
        
        # Detailed results
        f.write("\n## Detailed Results\n\n")
        f.write("| Library | Method | Concurrency | RPS | Response Time (ms) | Memory (MB) | Success Rate (%) |\n")
        f.write("|---------|--------|-------------|-----|-------------------|-------------|------------------|\n")
        
        for result in sorted(results, key=lambda r: (r.library, r.method, r.concurrency)):
            success_rate = (result.successful_requests / result.total_requests * 100) if result.total_requests > 0 else 0
            f.write(f"| {result.library} | {result.method} | {result.concurrency} | "
                   f"{result.requests_per_second:.2f} | {result.average_response_time_ms * 1000:.2f} | "
                   f"{result.memory_usage_mb:.2f} | {success_rate:.2f} |\n")
    
    print(f"Detailed report saved to: {report_file}")


def main():
    """Main function to run benchmarks."""
    args = parse_arguments()
    
    # Parse concurrency levels
    if args.quick:
        concurrency_levels = [10]
    else:
        concurrency_levels = [int(x.strip()) for x in args.concurrency.split(',')]
    
    # Create base configuration
    config = create_config_from_args(args)
    
    if args.verbose:
        print(f"Configuration:")
        print(f"  Concurrent requests: {', '.join(map(str, concurrency_levels))}")
        print(f"  Requests per test: {config.num_requests}")
        print(f"  Libraries: {', '.join(config.libraries)}")
        print(f"  Endpoints: {', '.join(config.endpoints)}")
        print(f"  Timeout: {config.timeout}s")
        print(f"  Warmup requests: {config.warmup_requests}")
        print()
    
    # Initialize benchmark runner
    runner = BenchmarkRunner(config)
    
    # Base URL for testing
    base_url = args.host.rstrip('/')
    
    # Create benchmarkers for each library
    benchmarkers = {}
    for library in config.libraries:
        if library == 'requestx-sync':
            benchmarkers[library] = RequestXSyncBenchmarker()
        elif library == 'requestx-async':
            benchmarkers[library] = RequestXAsyncBenchmarker()
        elif library == 'httpx-sync':
            try:
                import httpx
                benchmarkers[library] = HttpxSyncBenchmarker()
            except ImportError:
                print(f"Warning: Library 'httpx' not installed. Skipping {library}.")
                continue
        elif library == 'httpx-async':
            try:
                import httpx
                benchmarkers[library] = HttpxAsyncBenchmarker()
            except ImportError:
                print(f"Warning: Library 'httpx' not installed. Skipping {library}.")
                continue
        elif library == 'requests':
            try:
                import requests
                benchmarkers[library] = RequestsBenchmarker()
            except ImportError:
                print(f"Warning: Library '{library}' not installed. Skipping.")
                continue
        elif library == 'aiohttp':
            try:
                import aiohttp
                benchmarkers[library] = AiohttpBenchmarker()
            except ImportError:
                print(f"Warning: Library '{library}' not installed. Skipping.")
                continue
        else:
            print(f"Warning: Unknown library '{library}'. Skipping.")
            continue
    
    if not benchmarkers:
        print("Error: No supported libraries found.")
        return
    
    # Run benchmarks
    print("Running benchmarks...")
    all_results = []
    
    for library, benchmarker in benchmarkers.items():
        print(f"\nBenchmarking {library}...")
        
        for endpoint in config.endpoints:
            url = f"{base_url}{endpoint}"
            method = endpoint.upper().replace('/', '') if endpoint != '/' else 'GET'
            
            for concurrency in concurrency_levels:
                try:
                    print(f"  Testing {method} {endpoint} (concurrency: {concurrency})...")
                    
                    # Create config for this specific concurrency level
                    current_config = BenchmarkConfig(
                        num_requests=config.num_requests,
                        concurrent_requests=concurrency,
                        timeout=config.timeout,
                        warmup_requests=config.warmup_requests,
                        libraries=config.libraries,
                        endpoints=config.endpoints
                    )
                    
                    # Create runner with current config
                    current_runner = BenchmarkRunner(current_config)
                    
                    # Run benchmark directly without profiler override
                    from requestx.benchmark import BenchmarkerAsync
                    if isinstance(benchmarker, BenchmarkerAsync):
                        import asyncio
                        result = asyncio.run(current_runner.run_async_benchmark(benchmarker, url, method))
                    else:
                        result = current_runner.run_benchmark(benchmarker, url, method)
                    
                    # No need to override - the benchmark methods already calculate CPU/memory correctly
                    all_results.append(result)
                    
                    if args.verbose:
                        print(f"    RPS: {result.requests_per_second:.2f}")
                        print(f"    Avg Response Time: {result.average_response_time_ms:.2f}ms")
                        print(f"    Error Rate: {result.error_rate:.2f}%")
                        print(f"    CPU Usage: {result.cpu_usage_percent:.2f}%")
                        print(f"    Memory Usage: {result.memory_usage_mb:.2f}MB")
                    
                except Exception as e:
                    print(f"    Error: {e}")
                    continue
    
    # Save results
    if all_results:
        # Generate timestamp for filenames
        timestamp = datetime.now().strftime("%Y-%m-%d-%H:%M:%S")
        
        # Save to JSON
        if not args.no_json:
            json_file = os.path.join(args.output_dir, f'benchmark_results_{timestamp}.json')
            os.makedirs(args.output_dir, exist_ok=True)
            
            with open(json_file, 'w') as f:
                json.dump([result.to_dict() for result in all_results], f, indent=2)
            print(f"\nResults saved to {json_file}")
        
        # Save to CSV
        if not args.no_csv:
            csv_file = os.path.join(args.output_dir, f'benchmark_results_{timestamp}.csv')
            os.makedirs(args.output_dir, exist_ok=True)
            
            import csv
            with open(csv_file, 'w', newline='') as f:
                if all_results:
                    writer = csv.DictWriter(f, fieldnames=all_results[0].to_dict().keys())
                    writer.writeheader()
                    for result in all_results:
                        writer.writerow(result.to_dict())
            print(f"Results saved to {csv_file}")
        
        # Export to cloud if not disabled
        if not args.no_cloud:
            print("\nExporting results to Grafana Cloud Logs...")
            # Convert results to dictionaries for cloud export
            results_dicts = [result.to_dict() for result in all_results]
            
            # Add timestamp and additional metadata
            for result_dict in results_dicts:
                result_dict['timestamp'] = timestamp
                result_dict['benchmark_version'] = '1.0'
            
            # Export to cloud with additional tags
            cloud_tags = {
                'timestamp': timestamp,
                'total_results': str(len(all_results))
            }
            
            success = export_to_cloud(results_dicts, tags=cloud_tags)
            if not success:
                print("Warning: Failed to export results to cloud")
        
        # Save to database if not disabled
        if not args.no_db:
            print("\nSaving results to database...")
            try:
                save_to_database(all_results, args.db_path)
                print(f"Successfully saved {len(all_results)} results to database: {args.db_path}")
            except Exception as e:
                print(f"Warning: Failed to save results to database: {e}")
        
        # Generate report
        if args.verbose:
            print("\n" + "="*50)
            print("BENCHMARK REPORT")
            print("="*50)
            generate_report(all_results, args.output_dir)
    
    else:
        print("No benchmark results to save.")


if __name__ == "__main__":
    sys.exit(main())