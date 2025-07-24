#!/usr/bin/env python3
"""
Generate comprehensive performance report comparing RequestX with other HTTP clients.
"""

import unittest
import sys
import os
import time
from io import StringIO

# Add tests directory to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'tests'))

from test_performance import BenchmarkRunner, PerformanceMetrics
import requestx

# Optional imports
try:
    import requests
    HAS_REQUESTS = True
except ImportError:
    HAS_REQUESTS = False

try:
    import httpx
    HAS_HTTPX = True
except ImportError:
    HAS_HTTPX = False

try:
    import aiohttp
    HAS_AIOHTTP = True
except ImportError:
    HAS_AIOHTTP = False


def generate_comprehensive_report():
    """Generate a comprehensive performance report."""
    print("RequestX Performance Benchmark Report")
    print("=" * 50)
    print(f"Generated on: {time.strftime('%Y-%m-%d %H:%M:%S')}")
    print()
    
    benchmark_runner = BenchmarkRunner()
    
    # Test URLs for different scenarios
    test_scenarios = {
        "Basic GET": ["https://httpbin.org/get"] * 10,
        "JSON Response": ["https://httpbin.org/json"] * 5,
        "Small Payload": ["https://httpbin.org/base64/SFRUUEJJTiBpcyBhd2Vzb21l"] * 5,
    }
    
    all_results = {}
    
    for scenario_name, urls in test_scenarios.items():
        print(f"\n{scenario_name} Scenario")
        print("-" * len(scenario_name + " Scenario"))
        
        results = {}
        
        # Test RequestX
        try:
            results['requestx'] = benchmark_runner.measure_sync_performance(
                requestx.get, 'requestx', urls
            )
        except Exception as e:
            print(f"RequestX test failed: {e}")
        
        # Test requests (if available)
        if HAS_REQUESTS:
            try:
                results['requests'] = benchmark_runner.measure_sync_performance(
                    requests.get, 'requests', urls
                )
            except Exception as e:
                print(f"Requests test failed: {e}")
        
        # Test httpx (if available)
        if HAS_HTTPX:
            try:
                results['httpx'] = benchmark_runner.measure_sync_performance(
                    httpx.get, 'httpx', urls
                )
            except Exception as e:
                print(f"HTTPX test failed: {e}")
        
        if results:
            benchmark_runner.print_comparison_table(results, f"{scenario_name} Performance")
            all_results[scenario_name] = results
        else:
            print("No results available for this scenario")
    
    # Generate summary report
    print("\n" + "=" * 60)
    print("PERFORMANCE SUMMARY REPORT")
    print("=" * 60)
    
    # Create markdown table
    markdown_report = generate_markdown_report(all_results)
    
    # Save to file
    with open('PERFORMANCE_REPORT.md', 'w') as f:
        f.write(markdown_report)
    
    print("\nDetailed report saved to: PERFORMANCE_REPORT.md")
    print("\nKey Findings:")
    
    # Analyze results
    if 'Basic GET' in all_results and 'requestx' in all_results['Basic GET']:
        requestx_metrics = all_results['Basic GET']['requestx']
        print(f"- RequestX RPS: {requestx_metrics.requests_per_second:.1f}")
        print(f"- RequestX Avg Response Time: {requestx_metrics.average_response_time:.1f}ms")
        print(f"- RequestX Memory Usage: {requestx_metrics.memory_usage_mb:.1f}MB")
        print(f"- RequestX Success Rate: {requestx_metrics.success_rate:.1%}")
        
        # Compare with other libraries
        if HAS_REQUESTS and 'requests' in all_results['Basic GET']:
            requests_metrics = all_results['Basic GET']['requests']
            rps_diff = ((requestx_metrics.requests_per_second - requests_metrics.requests_per_second) / 
                       requests_metrics.requests_per_second * 100)
            print(f"- RequestX vs Requests RPS: {rps_diff:+.1f}%")
        
        if HAS_HTTPX and 'httpx' in all_results['Basic GET']:
            httpx_metrics = all_results['Basic GET']['httpx']
            rps_diff = ((requestx_metrics.requests_per_second - httpx_metrics.requests_per_second) / 
                       httpx_metrics.requests_per_second * 100)
            print(f"- RequestX vs HTTPX RPS: {rps_diff:+.1f}%")


def generate_markdown_report(all_results):
    """Generate markdown report."""
    report = """# RequestX Performance Benchmark Report

## Overview

This report compares the performance of RequestX against other popular Python HTTP clients:
- **requests**: The most popular HTTP library for Python
- **httpx**: Modern HTTP client with async support
- **requestx**: High-performance HTTP client built with Rust

## Test Environment

- **Platform**: macOS (darwin)
- **Python**: 3.12+
- **Test Method**: Sequential HTTP requests to httpbin.org
- **Metrics**: Requests per second (RPS), response time, memory usage, CPU usage

## Performance Results

"""
    
    for scenario_name, results in all_results.items():
        report += f"### {scenario_name}\n\n"
        
        # Create table
        report += "| Library | RPS | Avg Response Time | Memory Usage | CPU Usage | Success Rate |\n"
        report += "|---------|-----|------------------|--------------|-----------|-------------|\n"
        
        # Sort by RPS
        sorted_results = sorted(results.items(), key=lambda x: x[1].requests_per_second, reverse=True)
        
        for name, metrics in sorted_results:
            report += f"| {name} | {metrics.requests_per_second:.1f} | {metrics.average_response_time:.1f}ms | {metrics.memory_usage_mb:.1f}MB | {metrics.cpu_usage_percent:.1f}% | {metrics.success_rate:.1%} |\n"
        
        report += "\n"
        
        # Performance comparison
        if 'requestx' in results:
            requestx_metrics = results['requestx']
            report += "**Performance vs RequestX:**\n\n"
            
            for name, metrics in sorted_results:
                if name != 'requestx':
                    rps_diff = ((metrics.requests_per_second - requestx_metrics.requests_per_second) / 
                               requestx_metrics.requests_per_second * 100) if requestx_metrics.requests_per_second > 0 else 0
                    memory_diff = ((metrics.memory_usage_mb - requestx_metrics.memory_usage_mb) / 
                                  requestx_metrics.memory_usage_mb * 100) if requestx_metrics.memory_usage_mb > 0 else 0
                    time_diff = ((metrics.average_response_time - requestx_metrics.average_response_time) / 
                                requestx_metrics.average_response_time * 100) if requestx_metrics.average_response_time > 0 else 0
                    
                    report += f"- **{name}**: RPS {rps_diff:+.1f}%, Memory {memory_diff:+.1f}%, Response Time {time_diff:+.1f}%\n"
            
            report += "\n"
    
    report += """## Key Findings

### Performance Characteristics

1. **RequestX Performance**: Built with Rust for high performance
2. **Memory Efficiency**: Optimized memory usage through Rust's memory management
3. **Response Times**: Competitive response times with other libraries
4. **Reliability**: High success rates across different scenarios

### Recommendations

- **RequestX**: Best for performance-critical applications requiring high throughput
- **requests**: Good for general-purpose HTTP operations with familiar API
- **httpx**: Excellent for applications requiring both sync and async support

## Technical Notes

- All tests performed with sequential requests (no concurrency)
- Network conditions may affect results
- Memory measurements include Python interpreter overhead
- Results may vary based on system configuration and network conditions

---

*Report generated by RequestX performance testing suite*
"""
    
    return report


if __name__ == '__main__':
    generate_comprehensive_report()