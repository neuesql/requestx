"""RequestX benchmarking module.

This module provides core benchmarking functionality for comparing RequestX
performance against other HTTP libraries.
"""
import os
import time
import asyncio
import statistics
import psutil
import unittest
from abc import ABC, abstractmethod
from typing import Dict, Any, List, Optional, Union
from dataclasses import dataclass, asdict
from concurrent.futures import ThreadPoolExecutor, as_completed


@dataclass
class BenchmarkConfig:
    """Configuration for benchmark execution."""
    num_requests: int = 100
    concurrent_requests: int = 10
    timeout: float = 30.0
    warmup_requests: int = 10
    libraries: List[str] = None
    endpoints: List[str] = None

    def __post_init__(self):
        if self.libraries is None:
            self.libraries = ['requestx', 'requests', 'httpx', 'aiohttp']
        if self.endpoints is None:
            self.endpoints = ['/get', '/post', '/put', '/delete']


@dataclass
class BenchmarkResult:
    """Result of a single benchmark run."""
    library: str
    concurrency: int
    method: str
    requests_per_second: float
    average_response_time_ms: float
    median_response_time_ms: float
    p95_response_time_ms: float
    p99_response_time_ms: float
    error_rate: float
    total_requests: int
    successful_requests: int
    failed_requests: int
    cpu_usage_percent: float
    memory_usage_mb: float
    timestamp: float

    def to_dict(self) -> Dict[str, Any]:
        """Convert result to dictionary."""
        return asdict(self)


class Benchmarker(ABC):
    """Abstract base class for benchmarking HTTP libraries."""

    def __init__(self, library_name: str):
        self.library_name = library_name
        self.session = None

    def setup(self):
        """Setup the HTTP client session."""
        pass

    def teardown(self):
        """Cleanup the HTTP client session."""
        pass

    @abstractmethod
    def make_request(self, url: str, method: str = 'GET', **kwargs) -> bool:
        """Make a single HTTP request.
        
        Returns:
            True if request was successful, False otherwise
        """
        pass

    @abstractmethod
    async def make_async_request(self, url: str, method: str = 'GET', **kwargs) -> bool:
        """Make a single async HTTP request.
        
        Returns:
            True if request was successful, False otherwise
        """
        pass


class BenchmarkerSync(Benchmarker):
    """Synchronous benchmarker for HTTP libraries like requests, requestx-sync, httpx-sync."""

    async def make_async_request(self, url: str, method: str = 'GET', **kwargs) -> bool:
        """Default async implementation that runs sync method in executor."""
        import asyncio
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, self.make_request, url, method, **kwargs)

    def benchmark_sync(self, url: str, method: str, num_requests: int,
                      concurrent_requests: int, timeout: float) -> BenchmarkResult:
        """Run synchronous benchmark."""
        self.setup()

        # Initialize process and CPU measurement
        process = psutil.Process()
        process.cpu_percent()  # First call to initialize CPU measurement
        time.sleep(0.1)  # Small delay for accurate CPU measurement

        start_time = time.time()
        start_cpu = process.cpu_percent(interval=None)  # Get current CPU usage
        start_memory_bytes = process.memory_info().rss
        start_memory_mb = start_memory_bytes / 1024 / 1024
        
        # Print start resource usage
        print(f"Benchmark Start - CPU: {start_cpu:.2f}%, Memory: {start_memory_mb:.2f} MB ({start_memory_bytes:,} bytes)")

        successful_requests = 0
        failed_requests = 0
        response_times = []

        def make_single_request():
            request_start = time.time()
            try:
                success = self.make_request(url, method, timeout=timeout)
                response_time = time.time() - request_start
                response_times.append(response_time)
                return success
            except Exception:
                response_times.append(time.time() - request_start)
                return False

        with ThreadPoolExecutor(max_workers=concurrent_requests) as executor:
            futures = [executor.submit(make_single_request) for _ in range(num_requests)]

            for future in as_completed(futures):
                try:
                    if future.result():
                        successful_requests += 1
                    else:
                        failed_requests += 1
                except Exception:
                    failed_requests += 1

        end_time = time.time()
        end_cpu = process.cpu_percent(interval=None)  # Get current CPU usage
        end_memory_bytes = process.memory_info().rss
        end_memory_mb = end_memory_bytes / 1024 / 1024
        
        # Print end resource usage
        print(f"Benchmark End - CPU: {end_cpu:.2f}%, Memory: {end_memory_mb:.2f} MB ({end_memory_bytes:,} bytes)")
        
        # Calculate differences
        cpu_usage_diff = end_cpu - start_cpu
        memory_usage_diff_mb = end_memory_mb - start_memory_mb
        memory_usage_diff_bytes = end_memory_bytes - start_memory_bytes
        
        print(f"Resource Usage - CPU Change: {cpu_usage_diff:+.2f}%, Memory Change: {memory_usage_diff_mb:+.2f} MB ({memory_usage_diff_bytes:+,} bytes)")

        total_time = end_time - start_time
        requests_per_second = num_requests / total_time if total_time > 0 else 0
        error_rate = (failed_requests / num_requests) * 100 if num_requests > 0 else 0

        self.teardown()

        return BenchmarkResult(
            library=self.library_name,
            concurrency=concurrent_requests,
            method=method,
            requests_per_second=requests_per_second,
            average_response_time_ms=statistics.mean(response_times) if response_times else 0,
            median_response_time_ms=statistics.median(response_times) if response_times else 0,
            p95_response_time_ms=self._percentile(response_times, 95) if response_times else 0,
            p99_response_time_ms=self._percentile(response_times, 99) if response_times else 0,
            error_rate=error_rate,
            total_requests=num_requests,
            successful_requests=successful_requests,
            failed_requests=failed_requests,
            cpu_usage_percent=cpu_usage_diff,  # Use CPU usage difference
            memory_usage_mb=memory_usage_diff_mb,  # Use memory usage difference in MB
            timestamp=total_time
        )



    @staticmethod
    def _percentile(data: List[float], percentile: int) -> float:
        """Calculate percentile of response times."""
        if not data:
            return 0
        sorted_data = sorted(data)
        index = int((percentile / 100) * len(sorted_data))
        if index >= len(sorted_data):
            index = len(sorted_data) - 1
        return sorted_data[index]


class BenchmarkerAsync(Benchmarker, unittest.IsolatedAsyncioTestCase):
    """Asynchronous benchmarker for HTTP libraries like requestx-async, httpx-async, aiohttp."""

    def __init__(self, library_name: str):
        Benchmarker.__init__(self, library_name)
        unittest.IsolatedAsyncioTestCase.__init__(self)
        self._loop = None

    async def asyncSetUp(self):
        """Async setup method called before each test."""
        await self.async_setup()

    async def asyncTearDown(self):
        """Async teardown method called after each test."""
        await self.async_teardown()

    async def async_setup(self):
        """Override this method in subclasses for async setup."""
        pass

    async def async_teardown(self):
        """Override this method in subclasses for async teardown."""
        pass

    def make_request(self, url: str, method: str = 'GET', **kwargs) -> bool:
        """Sync method not supported in async benchmarker."""
        raise NotImplementedError("BenchmarkerAsync only supports async operations. Use make_async_request instead.")

    async def benchmark_async(self, url: str, method: str, num_requests: int,
                             concurrent_requests: int, timeout: float) -> BenchmarkResult:
        """Run asynchronous benchmark."""
        await self.async_setup()

        # Initialize process and CPU measurement
        process = psutil.Process()
        process.cpu_percent()  # First call to initialize CPU measurement
        await asyncio.sleep(0.1)  # Small delay for accurate CPU measurement

        start_time = time.time()
        start_cpu = process.cpu_percent(interval=None)  # Get current CPU usage
        start_memory_bytes = process.memory_info().rss
        start_memory_mb = start_memory_bytes / 1024 / 1024
        
        # Print start resource usage
        print(f"Async Benchmark Start - CPU: {start_cpu:.2f}%, Memory: {start_memory_mb:.2f} MB ({start_memory_bytes:,} bytes)")

        successful_requests = 0
        failed_requests = 0
        response_times = []

        async def make_single_request():
            request_start = time.time()
            try:
                success = await self.make_async_request(url, method, timeout=timeout)
                response_time = time.time() - request_start
                response_times.append(response_time)
                return success
            except Exception:
                response_times.append(time.time() - request_start)
                return False

        # Create semaphore to limit concurrent requests
        semaphore = asyncio.Semaphore(concurrent_requests)

        async def bounded_request():
            async with semaphore:
                return await make_single_request()

        tasks = [bounded_request() for _ in range(num_requests)]
        results = await asyncio.gather(*tasks, return_exceptions=True)

        for result in results:
            if isinstance(result, Exception):
                failed_requests += 1
            elif result:
                successful_requests += 1
            else:
                failed_requests += 1

        end_time = time.time()
        end_cpu = process.cpu_percent(interval=None)  # Get current CPU usage
        end_memory_bytes = process.memory_info().rss
        end_memory_mb = end_memory_bytes / 1024 / 1024
        
        # Print end resource usage
        print(f"Async Benchmark End - CPU: {end_cpu:.2f}%, Memory: {end_memory_mb:.2f} MB ({end_memory_bytes:,} bytes)")
        
        # Calculate differences
        cpu_usage_diff = end_cpu - start_cpu
        memory_usage_diff_mb = end_memory_mb - start_memory_mb
        memory_usage_diff_bytes = end_memory_bytes - start_memory_bytes
        
        print(f"Async Resource Usage - CPU Change: {cpu_usage_diff:+.2f}%, Memory Change: {memory_usage_diff_mb:+.2f} MB ({memory_usage_diff_bytes:+,} bytes)")

        total_time = end_time - start_time
        requests_per_second = num_requests / total_time if total_time > 0 else 0
        error_rate = (failed_requests / num_requests) * 100 if num_requests > 0 else 0

        await self.async_teardown()

        return BenchmarkResult(
            library=self.library_name,
            concurrency=concurrent_requests,
            method=method,
            requests_per_second=requests_per_second,
            average_response_time_ms=statistics.mean(response_times) if response_times else 0,
            median_response_time_ms=statistics.median(response_times) if response_times else 0,
            p95_response_time_ms=self._percentile(response_times, 95) if response_times else 0,
            p99_response_time_ms=self._percentile(response_times, 99) if response_times else 0,
            error_rate=error_rate,
            total_requests=num_requests,
            successful_requests=successful_requests,
            failed_requests=failed_requests,
            cpu_usage_percent=cpu_usage_diff,  # Use CPU usage difference
            memory_usage_mb=memory_usage_diff_mb,  # Use memory usage difference in MB
            timestamp=time.time()
        )

    @staticmethod
    def _percentile(data: List[float], percentile: float) -> float:
        """Calculate percentile of a list of values."""
        if not data:
            return 0.0
        sorted_data = sorted(data)
        index = (percentile / 100) * (len(sorted_data) - 1)
        if index.is_integer():
            return sorted_data[int(index)]
        else:
            lower = sorted_data[int(index)]
            upper = sorted_data[int(index) + 1]
            return lower + (upper - lower) * (index - int(index))


class HttpxAsyncBenchmarker(BenchmarkerAsync):
    """Benchmarker for httpx library (async variant)."""

    def __init__(self):
        super().__init__('httpx-async')
        self.session = None

    async def async_setup(self):
        """Async setup method for httpx."""
        import httpx
        self.session = httpx.AsyncClient()

    async def async_teardown(self):
        """Async teardown method for httpx."""
        if self.session:
            await self.session.aclose()

    async def make_async_request(self, url: str, method: str = 'GET', **kwargs) -> bool:
        """Make async request using httpx."""
        try:
            response = await self.session.request(method, url, **kwargs)
            return 200 <= response.status_code < 400
        except Exception:
            return False


class RequestXBenchmarker(BenchmarkerSync):
    """Benchmarker for RequestX library (legacy name for backward compatibility)."""

    def __init__(self):
        super().__init__('requestx')

    def setup(self):
        try:
            import requestx
            self.session = requestx.Session()
        except ImportError:
            raise ImportError("RequestX library not found")

    def teardown(self):
        if self.session:
            self.session.close()

    def make_request(self, url: str, method: str = 'GET', **kwargs) -> bool:
        try:
            if self.session is None:
                print("RequestX session is None!")
                return False
            response = self.session.request(method, url, **kwargs)
            print(f"RequestX response: {response.status_code}")
            return 200 <= response.status_code < 400
        except Exception as e:
            print(f"RequestX sync error: {e}")
            return False

    async def make_async_request(self, url: str, method: str = 'GET', **kwargs) -> bool:
        # RequestX doesn't have async support yet, so we'll use sync in thread
        import asyncio
        loop = asyncio.get_event_loop()
        try:
            result = await loop.run_in_executor(None, self.make_request, url, method, **kwargs)
            return result
        except Exception as e:
            print(f"RequestX async error: {e}")
            return False


class RequestXSyncBenchmarker(BenchmarkerSync):
    """Benchmarker for RequestX library (sync variant)."""

    def __init__(self):
        super().__init__('requestx-sync')

    def setup(self):
        try:
            import requestx
            self.session = requestx.Session()
        except ImportError:
            raise ImportError("RequestX library not found")

    def teardown(self):
        if self.session:
            self.session.close()

    def make_request(self, url: str, method: str = 'GET', **kwargs) -> bool:
        try:
            response = self.session.request(method, url, **kwargs)
            return 200 <= response.status_code < 400
        except Exception:
            return False

    async def make_async_request(self, url: str, method: str = 'GET', **kwargs) -> bool:
        # RequestX doesn't have async support yet, so we'll use sync in thread
        import asyncio
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, self.make_request, url, method, **kwargs)


class RequestsBenchmarker(BenchmarkerSync):
    """Benchmarker for requests library."""

    def __init__(self):
        super().__init__('requests')

    def setup(self):
        import requests
        self.session = requests.Session()

    def teardown(self):
        if self.session:
            self.session.close()

    def make_request(self, url: str, method: str = 'GET', **kwargs) -> bool:
        try:
            response = self.session.request(method, url, **kwargs)
            return 200 <= response.status_code < 400
        except Exception:
            return False


class HttpxBenchmarker(BenchmarkerSync):
    """Benchmarker for httpx library (legacy name for backward compatibility)."""

    def __init__(self):
        super().__init__('httpx')

    def setup(self):
        import httpx
        self.session = httpx.Client()

    def teardown(self):
        if self.session:
            self.session.close()

    def make_request(self, url: str, method: str = 'GET', **kwargs) -> bool:
        try:
            response = self.session.request(method, url, **kwargs)
            return 200 <= response.status_code < 400
        except Exception:
            return False


class HttpxSyncBenchmarker(BenchmarkerSync):
    """Benchmarker for httpx library (sync variant)."""

    def __init__(self):
        super().__init__('httpx-sync')

    def setup(self):
        import httpx
        self.session = httpx.Client()

    def teardown(self):
        if self.session:
            self.session.close()

    def make_request(self, url: str, method: str = 'GET', **kwargs) -> bool:
        try:
            response = self.session.request(method, url, **kwargs)
            return 200 <= response.status_code < 400
        except Exception:
            return False


class RequestXAsyncBenchmarker(BenchmarkerAsync):
    """Benchmarker for RequestX library (async variant)."""

    def __init__(self):
        super().__init__('requestx-async')
        self.session = None

    async def async_setup(self):
        """Async setup method for RequestX."""
        try:
            import requestx
            # RequestX doesn't have asyncsession yet, will use sync session
            self.session = requestx.Session()
            print(f"RequestX async_setup completed, session: {self.session}")
        except ImportError:
            print("RequestX library not found!")
            raise ImportError("RequestX library not found")

    async def async_teardown(self):
        """Async teardown method for RequestX."""
        if self.session:
            self.session.close()

    # Remove the make_request method - BenchmarkerAsync handles this with NotImplementedError

    async def make_async_request(self, url: str, method: str = 'GET', **kwargs) -> bool:
        # RequestX doesn't have async support yet, so we'll use sync in thread
        import asyncio
        loop = asyncio.get_event_loop()
        # Since we removed make_request, we need to implement the sync logic here
        def sync_request():
            try:
                response = self.session.request(method, url, **kwargs)
                return 200 <= response.status_code < 400
            except Exception:
                return False

        return await loop.run_in_executor(None, sync_request)


# Remove the duplicate HttpxAsyncBenchmarker class (lines 496-530)
# Keep only the clean version at line 293

class AiohttpBenchmarker(BenchmarkerAsync):
    """Benchmarker for aiohttp library."""

    def __init__(self):
        super().__init__('aiohttp')
        self.session = None

    async def async_setup(self):
        """Async setup method for aiohttp."""
        import aiohttp
        self.session = aiohttp.ClientSession()

    async def async_teardown(self):
        """Async teardown method for aiohttp."""
        if self.session and not self.session.closed:
            await self.session.close()



    async def make_async_request(self, url: str, method: str = 'GET', **kwargs) -> bool:
        """Make async request using aiohttp."""
        try:
            # Use the properly managed session from async_setup
            if self.session and not self.session.closed:
                async with self.session.request(method, url, **kwargs) as response:
                    return 200 <= response.status < 400
            else:
                # Fallback: create temporary session if none exists
                import aiohttp
                async with aiohttp.ClientSession() as session:
                    async with session.request(method, url, **kwargs) as response:
                        return 200 <= response.status < 400
        except Exception:
            return False


class BenchmarkRunner:
    """Main benchmark runner class."""

    def __init__(self, config: BenchmarkConfig):
        self.config = config
        self.results: List[BenchmarkResult] = []

    def run_benchmark(self, benchmarker: Benchmarker,
                     url: str, method: str = 'GET') -> BenchmarkResult:
        """Run a single benchmark."""
        # Warmup
        if self.config.warmup_requests > 0:
            benchmarker.setup()
            for _ in range(self.config.warmup_requests):
                try:
                    benchmarker.make_request(url, method, timeout=self.config.timeout)
                except Exception:
                    pass
            benchmarker.teardown()

        # Actual benchmark
        result = benchmarker.benchmark_sync(
            url, method,
            self.config.num_requests,
            self.config.concurrent_requests,
            self.config.timeout
        )

        self.results.append(result)
        return result

    async def run_async_benchmark(self, benchmarker: Benchmarker,
                                 url: str, method: str = 'GET') -> BenchmarkResult:
        """Run a single async benchmark."""
        # For async benchmarkers, setup/teardown is handled by IsolatedAsyncioTestCase
        # Warmup
        if self.config.warmup_requests > 0:
            # Only call setup/teardown for sync benchmarkers
            if hasattr(benchmarker, 'setup') and not isinstance(benchmarker, BenchmarkerAsync):
                benchmarker.setup()
            for _ in range(self.config.warmup_requests):
                try:
                    await benchmarker.make_async_request(url, method, timeout=self.config.timeout)
                except Exception:
                    pass
            if hasattr(benchmarker, 'teardown') and not isinstance(benchmarker, BenchmarkerAsync):
                benchmarker.teardown()

        # Actual benchmark
        result = await benchmarker.benchmark_async(
            url, method,
            self.config.num_requests,
            self.config.concurrent_requests,
            self.config.timeout
        )

        self.results.append(result)
        return result

    def get_results(self) -> List[BenchmarkResult]:
        """Get all benchmark results."""
        return self.results.copy()

    def clear_results(self):
        """Clear all stored results."""
        self.results.clear()

    def export_results(self, format: str = 'json') -> Union[str, Dict[str, Any]]:
        """Export results in specified format."""
        if format.lower() == 'json':
            import json
            return json.dumps([result.to_dict() for result in self.results], indent=2)
        elif format.lower() == 'dict':
            return {'results': [result.to_dict() for result in self.results]}
        else:
            raise ValueError(f"Unsupported export format: {format}")