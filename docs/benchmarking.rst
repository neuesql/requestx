# RequestX Performance Benchmarking Suite

This directory contains a comprehensive performance benchmarking system for RequestX, designed to measure and compare HTTP client performance across multiple libraries and scenarios.

## Overview

The benchmarking suite provides:

- **Decorator-based profiling** with `@Profile` for measuring CPU, memory, and request metrics
- **Comprehensive library comparison** between RequestX, requests, httpx, and aiohttp
- **Multiple test scenarios** covering different HTTP methods, concurrency levels, and request sizes
- **Structured result persistence** in CSV and JSON formats
- **OpenTelemetry integration** for exporting metrics to Grafana Cloud and other observability platforms
- **CLI commands** via Makefile for easy execution

## Quick Start

### Run Quick Benchmarks

```bash
# Run a quick benchmark suite (recommended for development)
make benchmark-quick

# Or directly with Python
./scripts/requestx-benchmark --quick
```

### Run Comprehensive Benchmarks

```bash
# Run full benchmark suite
make benchmark-full

# Run specific test categories
make benchmark-methods      # Test different HTTP methods
make benchmark-concurrency  # Test concurrency scaling
make benchmark-sizes        # Test different request sizes
```

## Architecture

### Core Components

1. **Profiler (`profiler.py`)** - Decorator and context manager for performance measurement
2. **Benchmark Suite (`benchmark_suite.py`)** - Main benchmarking framework
3. **OpenTelemetry Exporter (`otel_exporter.py`)** - Metrics export to observability platforms
4. **Test Runner (`requestx-benchmark`)** - CLI interface and orchestration

### Profiler Usage

The `@Profile` decorator can be used in RequestX source code for performance measurement:

```python
from requestx.profiler import Profile

@Profile(cpu=True, memory=True, request=True)
def my_http_function():
    # Function implementation
    pass

# Or use as context manager
from requestx.profiler import profile_context

with profile_context(cpu=True, memory=True) as metrics:
    # Code to profile
    pass

print(f"CPU usage: {metrics.cpu_usage_percent}%")
print(f"Memory usage: {metrics.memory_usage_mb}MB")
```

## Test Scenarios

### HTTP Methods Tested

- GET - Basic retrieval operations
- POST - Data submission with various payload sizes
- PUT - Data updates with various payload sizes  
- DELETE - Resource deletion
- HEAD - Header-only requests
- OPTIONS - Capability discovery
- PATCH - Partial updates with various payload sizes

### Concurrency Levels

- **1** - Sequential requests (baseline)
- **10** - Low concurrency
- **100** - Medium concurrency
- **1000** - High concurrency (stress testing)

### Request Sizes

- **Small (1KB)** - Typical API requests
- **Medium (10KB)** - Moderate payloads
- **Large (100KB)** - Large data transfers

### Libraries Compared

- **RequestX (sync)** - RequestX synchronous API
- **RequestX (async)** - RequestX asynchronous API
- **requests** - Popular Python HTTP library (sync only)
- **httpx (sync)** - Modern HTTP library synchronous API
- **httpx (async)** - Modern HTTP library asynchronous API
- **aiohttp** - Async-only HTTP library

## Metrics Collected

### Performance Metrics

- **Requests per second (RPS)** - Throughput measurement
- **Average response time** - Mean request latency
- **Min/Max response time** - Latency range
- **95th percentile response time** - Tail latency
- **Connection time** - Time to establish connections

### Resource Usage

- **CPU usage percentage** - Processor utilization
- **Memory usage (MB)** - RAM consumption
- **Peak memory (MB)** - Maximum memory usage
- **Memory growth (MB)** - Memory increase during test

### Success Metrics

- **Total requests** - Number of requests attempted
- **Successful requests** - Requests completed successfully
- **Failed requests** - Requests that failed
- **Error rate** - Percentage of failed requests
- **Error breakdown** - Categorized error types

## Output Formats

### CSV Results

Structured data suitable for analysis and visualization:

```csv
library,method,concurrency,request_size,requests_per_second,average_response_time,memory_usage_mb,error_rate
requestx_sync,GET,10,small,245.67,0.041,12.5,0.0
requests,GET,10,small,198.34,0.050,15.2,0.0
```

### JSON Results

Complete result data including error details and metadata:

```json
{
  "library": "requestx_sync",
  "method": "GET",
  "concurrency": 10,
  "requests_per_second": 245.67,
  "errors": {},
  "timestamp": "2024-01-15T10:30:00"
}
```

### Markdown Reports

Human-readable summary reports with performance comparisons and best performers.

## OpenTelemetry Integration

### Grafana Cloud Export

Export benchmark results to Grafana Cloud for visualization and monitoring:

```bash
# Set environment variables
export GRAFANA_INSTANCE_ID="your-instance-id"
export GRAFANA_API_KEY="your-api-key"

# Run benchmarks with Grafana export
make benchmark-grafana
```

### Custom OTLP Endpoints

Export to any OpenTelemetry-compatible system:

```bash
./scripts/requestx-benchmark \
  --otlp-endpoint "https://your-otlp-endpoint.com/v1/traces" \
  --otlp-headers "Authorization=Bearer your-token"
```

### Metrics Exported

- **http_requests_per_second** - RPS histogram
- **http_response_time** - Response time histogram  
- **memory_usage** - Memory usage histogram
- **cpu_usage** - CPU usage histogram
- **http_requests_total** - Request counter
- **http_errors_total** - Error counter

## CLI Commands

### Makefile Commands

```bash
# Quick development testing
make benchmark-quick

# Comprehensive benchmarks
make benchmark-full

# Specific test categories
make benchmark-methods
make benchmark-concurrency  
make benchmark-sizes

# Library comparisons
make benchmark-compare

# Grafana Cloud integration
make benchmark-grafana

# Test profiler functionality
make benchmark-profiler-test

# Complete Task 11
make task11
```

### Direct Python Execution

```bash
# Basic usage
./scripts/requestx-benchmark

# Custom parameters
./scripts/requestx-benchmark \
  --concurrency 1,10,100 \
  --requests 50 \
  --methods GET,POST \
  --sizes small,medium

# Quick test mode
./scripts/requestx-benchmark --quick

# Export to Grafana Cloud
./scripts/requestx-benchmark --grafana-cloud

# Custom output directory
./scripts/requestx-benchmark --output-dir ./my_results

# Verbose output
./scripts/requestx-benchmark --verbose
```

## Configuration

### Environment Variables

- `GRAFANA_INSTANCE_ID` - Grafana Cloud instance ID
- `GRAFANA_API_KEY` - Grafana Cloud API key
- `OTEL_EXPORTER_OTLP_ENDPOINT` - Custom OTLP endpoint
- `OTEL_EXPORTER_OTLP_HEADERS` - Custom OTLP headers

### Benchmark Configuration

Modify `BenchmarkConfig` in `benchmark_suite.py`:

```python
config = BenchmarkConfig(
    concurrency_levels=[1, 10, 100, 1000],
    request_sizes=['small', 'medium', 'large'],
    http_methods=['GET', 'POST', 'PUT', 'DELETE'],
    test_requests=100,
    timeout=30.0
)
```

## Dependencies

### Required

- `psutil` - System resource monitoring
- `asyncio` - Asynchronous execution support

### Optional

- `opentelemetry-api` - OpenTelemetry metrics export
- `opentelemetry-sdk` - OpenTelemetry SDK
- `opentelemetry-exporter-otlp` - OTLP exporter
- `requests` - For comparison benchmarks
- `httpx` - For comparison benchmarks
- `aiohttp` - For comparison benchmarks

### Installation

```bash
# Install required dependencies
pip install psutil

# Install OpenTelemetry (optional)
pip install opentelemetry-api opentelemetry-sdk opentelemetry-exporter-otlp

# Install comparison libraries (optional)
pip install requests httpx aiohttp
```

## Results Analysis

### Performance Comparison

The benchmark suite automatically identifies:

- **Highest RPS** - Best throughput performer
- **Lowest Latency** - Fastest response times
- **Lowest Memory** - Most memory-efficient
- **Best Error Rate** - Most reliable

### Trend Analysis

Results include timestamps for tracking performance over time:

- Performance regression detection
- Optimization impact measurement
- Release-to-release comparisons

### Statistical Analysis

Response time distributions and percentiles help identify:

- Tail latency issues
- Performance consistency
- Outlier detection

## Troubleshooting

### Common Issues

1. **Import Errors** - Ensure RequestX is built: `make build-dev`
2. **Network Timeouts** - Increase timeout: `--timeout 60`
3. **Memory Issues** - Reduce concurrency or request count
4. **Missing Libraries** - Install optional dependencies for comparisons

### Debug Mode

Enable verbose output for troubleshooting:

```bash
./scripts/requestx-benchmark --verbose
```

### Profiler Testing

Test the profiler independently:

```bash
make benchmark-profiler-test
```

## Contributing

When adding new benchmark scenarios:

1. Add test cases to `benchmark_suite.py`
2. Update CLI options in `requestx-benchmark`
3. Add Makefile commands for convenience
4. Update this README with new features
5. Ensure OpenTelemetry metrics are exported

## Performance Targets

RequestX aims to demonstrate:

- **Higher RPS** than requests and httpx
- **Lower memory usage** than comparable libraries
- **Consistent performance** across concurrency levels
- **Reliable error handling** with low error rates

The benchmark suite validates these performance characteristics and provides quantitative evidence of RequestX's advantages.