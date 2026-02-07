"""Core benchmarking functionality for the HTTP benchmark framework."""

import asyncio
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime
from typing import Dict, Any

from .clients.aiohttp_adapter import AiohttpAdapter
from .clients.httpx_adapter import HttpxAdapter
from .clients.pycurl_adapter import PycurlAdapter
from .clients.requests_adapter import RequestsAdapter
from .clients.requestx_adapter import RequestXAdapter
from .clients.urllib3_adapter import Urllib3Adapter
from .models.benchmark_configuration import BenchmarkConfiguration
from .models.benchmark_result import BenchmarkResult
from .models.http_request import HTTPRequest
from .utils.logging import app_logger
from .utils.resource_monitor import resource_monitor


class BenchmarkRunner:
    """Core benchmarking functionality for HTTP client performance testing."""

    def __init__(self, config: BenchmarkConfiguration):
        self.config = config
        self.adapter_classes = {
            "requests": RequestsAdapter,
            "requestx": RequestXAdapter,
            "httpx": HttpxAdapter,
            "aiohttp": AiohttpAdapter,
            "urllib3": Urllib3Adapter,
            "pycurl": PycurlAdapter,
        }
        self.results = []
        self.resource_metrics = []

    def run(self) -> BenchmarkResult:
        """Run the benchmark with the given configuration."""
        app_logger.info(f"Starting benchmark for {self.config.target_url} using {self.config.client_library}")

        start_time = datetime.now()
        perf_start = time.perf_counter()

        if self.config.client_library not in self.adapter_classes:
            raise ValueError(f"Unsupported client library: {self.config.client_library}")

        adapter_class = self.adapter_classes[self.config.client_library]

        http_request = HTTPRequest(
            method=self.config.http_method,
            url=self.config.target_url,
            headers=self.config.headers,
            body=self.config.body,
            timeout=self.config.timeout,
            verify_ssl=self.config.verify_ssl,
        )

        # Start continuous monitoring
        resource_monitor.start_monitoring()

        if self.config.is_async:
            result = asyncio.run(self._run_async_benchmark(adapter_class, http_request))
        else:
            result = self._run_sync_benchmark(adapter_class, http_request)

        # Stop monitoring and get aggregated metrics
        metrics = resource_monitor.stop_monitoring()
        network_io = resource_monitor.get_network_io_delta()

        end_time = datetime.now()
        perf_end = time.perf_counter()
        duration = perf_end - perf_start

        # Use aggregated metrics instead of 2-point average
        cpu_usage_avg = metrics["cpu_avg"]
        memory_usage_avg = metrics["memory_avg"]

        benchmark_result = BenchmarkResult(
            name=self.config.name,
            client_library=self.config.client_library,
            client_type="async" if self.config.is_async else "sync",
            http_method=self.config.http_method,
            url=self.config.target_url,
            start_time=start_time,
            end_time=end_time,
            duration=duration,
            requests_count=result["requests_count"],
            requests_per_second=result["requests_per_second"],
            avg_response_time=result["avg_response_time"],
            min_response_time=result["min_response_time"],
            max_response_time=result["max_response_time"],
            p95_response_time=result["p95_response_time"],
            p99_response_time=result["p99_response_time"],
            cpu_usage_avg=cpu_usage_avg,
            memory_usage_avg=memory_usage_avg,
            network_io=network_io,  # Now contains delta, not cumulative
            error_count=result["error_count"],
            error_rate=result["error_rate"],
            concurrency_level=self.config.concurrency,
            config_snapshot=self.config.to_dict(),
        )

        app_logger.info(f"Benchmark completed: {benchmark_result.requests_per_second} RPS")
        return benchmark_result

    def _run_sync_benchmark(self, adapter_class, http_request: HTTPRequest) -> Dict[str, Any]:
        """Run a synchronous benchmark."""
        app_logger.info("Running synchronous benchmark")

        adapter = adapter_class()
        adapter.verify_ssl = http_request.verify_ssl
        with adapter:
            return self._execute_sync_benchmark(adapter, http_request)

    def _execute_sync_benchmark(self, adapter, http_request: HTTPRequest) -> Dict[str, Any]:

        response_times = []
        error_count = 0
        start_time = time.perf_counter()
        end_time = start_time + self.config.duration_seconds

        # Execute requests concurrently using ThreadPoolExecutor for the specified duration
        with ThreadPoolExecutor(max_workers=self.config.concurrency) as executor:
            # Submit initial batch of requests
            futures = set()
            for _ in range(self.config.concurrency):
                futures.add(executor.submit(adapter.make_request, http_request))

            # Continue making requests for the specified duration
            while time.perf_counter() < end_time:
                completed_futures = []
                try:
                    for future in as_completed(futures, timeout=1):  # Use timeout to check duration periodically
                        result = future.result()
                        if result["success"]:
                            response_times.append(result["response_time"])
                        else:
                            error_count += 1
                            if error_count <= 5:  # Limit error logging
                                app_logger.error(f"Request failed: {result.get('error', 'Unknown error')}")

                        # Submit a new request to keep the concurrency level
                        if time.perf_counter() < end_time:
                            futures.add(executor.submit(adapter.make_request, http_request))

                        completed_futures.append(future)
                except TimeoutError:
                    # Timeout reached, loop will continue and check time
                    pass
                except Exception as e:
                    # Handle other potential errors during execution
                    # app_logger.error(f"Error in future processing: {str(e)}")
                    print(e)

                # Remove completed futures
                for future in completed_futures:
                    futures.discard(future)

                # If all futures completed before duration, submit more
                while len(futures) < self.config.concurrency and time.perf_counter() < end_time:
                    futures.add(executor.submit(adapter.make_request, http_request))

        # Wait for any remaining requests to complete
        for future in as_completed(futures):
            result = future.result()
            if result["success"]:
                response_times.append(result["response_time"])
            else:
                error_count += 1

        # Calculate metrics
        if response_times:
            avg_response_time = sum(response_times) / len(response_times)
            min_response_time = min(response_times) if response_times else 0
            max_response_time = max(response_times) if response_times else 0

            # Calculate percentiles using linear interpolation method
            sorted_times = sorted(response_times)

            def calculate_percentile(data, percentile):
                if not data:
                    return 0
                n = len(data)
                # Using the standard percentile formula: P = (percentile * (n - 1)) + 1
                # Then interpolate between values if needed
                rank = percentile * (n - 1)
                lower_idx = int(rank)
                upper_idx = min(lower_idx + 1, n - 1)

                # Interpolate between the two values
                fraction = rank - lower_idx
                if lower_idx == upper_idx:
                    return data[lower_idx]
                else:
                    lower_val = data[lower_idx]
                    upper_val = data[upper_idx]
                    return lower_val + fraction * (upper_val - lower_val)

            p95_response_time = calculate_percentile(sorted_times, 0.95)
            p99_response_time = calculate_percentile(sorted_times, 0.99)
        else:
            avg_response_time = 0
            min_response_time = 0
            max_response_time = 0
            p95_response_time = 0
            p99_response_time = 0

        total_completed_requests = len(response_times) + error_count
        actual_duration = time.perf_counter() - start_time
        requests_per_second = total_completed_requests / actual_duration if actual_duration > 0 else 0
        error_rate = (error_count / total_completed_requests) * 100 if total_completed_requests > 0 else 0

        return {
            "requests_count": total_completed_requests,
            "requests_per_second": requests_per_second,
            "avg_response_time": avg_response_time,
            "min_response_time": min_response_time,
            "max_response_time": max_response_time,
            "p95_response_time": p95_response_time,
            "p99_response_time": p99_response_time,
            "error_count": error_count,
            "error_rate": error_rate,
        }

    async def _run_async_benchmark(self, adapter_class, http_request: HTTPRequest) -> Dict[str, Any]:
        """Run an asynchronous benchmark."""
        app_logger.info("Running asynchronous benchmark")

        adapter = adapter_class()
        adapter.verify_ssl = http_request.verify_ssl
        async with adapter:
            return await self._execute_async_benchmark(adapter, http_request)

    async def _execute_async_benchmark(self, adapter, http_request: HTTPRequest) -> Dict[str, Any]:

        response_times = []
        error_count = 0
        start_time = time.perf_counter()
        end_time = start_time + self.config.duration_seconds

        # Create initial tasks for concurrent execution
        tasks = set()
        for _ in range(self.config.concurrency):
            task = adapter.make_request_async(http_request)
            tasks.add(asyncio.create_task(task))

        # Continue making requests for the specified duration
        while time.perf_counter() < end_time:
            if not tasks:
                break

            # Wait for at least one task to complete
            done, pending = await asyncio.wait(tasks, return_when=asyncio.FIRST_COMPLETED, timeout=1.0)

            for task in done:
                try:
                    result = await task
                    if result["success"]:
                        response_times.append(result["response_time"])
                    else:
                        error_count += 1
                        if error_count <= 5:  # Limit error logging
                            app_logger.error(f"Request failed: {result.get('error', 'Unknown error')}")
                except Exception:
                    error_count += 1

            # Update tasks to include remaining pending tasks
            tasks = pending

            # If all tasks completed before duration, submit more
            while len(tasks) < self.config.concurrency and time.perf_counter() < end_time:
                new_task = adapter.make_request_async(http_request)
                tasks.add(asyncio.create_task(new_task))

        # Wait for any remaining tasks to complete
        if tasks:
            for task in asyncio.as_completed(tasks):
                try:
                    result = await task
                    if result["success"]:
                        response_times.append(result["response_time"])
                    else:
                        error_count += 1
                except Exception:
                    error_count += 1

        # Calculate metrics
        if response_times:
            avg_response_time = sum(response_times) / len(response_times)
            min_response_time = min(response_times) if response_times else 0
            max_response_time = max(response_times) if response_times else 0

            # Calculate percentiles using linear interpolation method
            sorted_times = sorted(response_times)

            def calculate_percentile(data, percentile):
                if not data:
                    return 0
                n = len(data)
                # Using the standard percentile formula: P = (percentile * (n - 1)) + 1
                # Then interpolate between values if needed
                rank = percentile * (n - 1)
                lower_idx = int(rank)
                upper_idx = min(lower_idx + 1, n - 1)

                # Interpolate between the two values
                fraction = rank - lower_idx
                if lower_idx == upper_idx:
                    return data[lower_idx]
                else:
                    lower_val = data[lower_idx]
                    upper_val = data[upper_idx]
                    return lower_val + fraction * (upper_val - lower_val)

            p95_response_time = calculate_percentile(sorted_times, 0.95)
            p99_response_time = calculate_percentile(sorted_times, 0.99)
        else:
            avg_response_time = 0
            min_response_time = 0
            max_response_time = 0
            p95_response_time = 0
            p99_response_time = 0

        total_completed_requests = len(response_times) + error_count
        actual_duration = time.perf_counter() - start_time
        requests_per_second = total_completed_requests / actual_duration if actual_duration > 0 else 0
        error_rate = (error_count / total_completed_requests) * 100 if total_completed_requests > 0 else 0
        return {
            "requests_count": total_completed_requests,
            "requests_per_second": requests_per_second,
            "avg_response_time": avg_response_time,
            "min_response_time": min_response_time,
            "max_response_time": max_response_time,
            "p95_response_time": p95_response_time,
            "p99_response_time": p99_response_time,
            "error_count": error_count,
            "error_rate": error_rate,
        }
