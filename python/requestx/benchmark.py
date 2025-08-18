"""RequestX benchmarking module.

This module provides core benchmarking functionality for comparing RequestX
performance against other HTTP libraries.
"""

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
    average_response_time: float
    median_response_time: float
    p95_response_time: float
    p99_response_time: float
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
        
        start_time = time.time()
        start_cpu = psutil.cpu_percent()
        start_memory = psutil.Process().memory_info().rss / 1024 / 1024
        
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
        end_cpu = psutil.cpu_percent()
        end_memory = psutil.Process().memory_info().rss / 1024 / 1024
        
        total_time = end_time - start_time
        requests_per_second = num_requests / total_time if total_time > 0 else 0
        error_rate = (failed_requests / num_requests) * 100 if num_requests > 0 else 0
        
        self.teardown()
        
        return BenchmarkResult(
            library=self.library_name,
            concurrency=concurrent_requests,
            method=method,
            requests_per_second=requests_per_second,
            average_response_time=statistics.mean(response_times) if response_times else 0,
            median_response_time=statistics.median(response_times) if response_times else 0,
            p95_response_time=self._percentile(response_times, 95) if response_times else 0,
            p99_response_time=self._percentile(response_times, 99) if response_times else 0,
            error_rate=error_rate,
            total_requests=num_requests,
            successful_requests=successful_requests,
            failed_requests=failed_requests,
            cpu_usage_percent=end_cpu - start_cpu,
            memory_usage_mb=end_memory - start_memory,
            timestamp=time.time()
        )
    
    async def benchmark_async(self, url: str, method: str, num_requests: int,
                             concurrent_requests: int, timeout: float) -> BenchmarkResult:
        """Run asynchronous benchmark."""
        self.setup()
        
        start_time = time.time()
        start_cpu = psutil.cpu_percent()
        start_memory = psutil.Process().memory_info().rss / 1024 / 1024
        
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
        end_cpu = psutil.cpu_percent()
        end_memory = psutil.Process().memory_info().rss / 1024 / 1024
        
        total_time = end_time - start_time
        requests_per_second = num_requests / total_time if total_time > 0 else 0
        error_rate = (failed_requests / num_requests) * 100 if num_requests > 0 else 0
        
        self.teardown()
        
        return BenchmarkResult(
            library=self.library_name,
            concurrency=concurrent_requests,
            method=method,
            requests_per_second=requests_per_second,
            average_response_time=statistics.mean(response_times) if response_times else 0,
            median_response_time=statistics.median(response_times) if response_times else 0,
            p95_response_time=self._percentile(response_times, 95) if response_times else 0,
            p99_response_time=self._percentile(response_times, 99) if response_times else 0,
            error_rate=error_rate,
            total_requests=num_requests,
            successful_requests=successful_requests,
            failed_requests=failed_requests,
            cpu_usage_percent=end_cpu - start_cpu,
            memory_usage_mb=end_memory - start_memory,
            timestamp=time.time()
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
    
    def make_request(self, url: str, method: str = 'GET', **kwargs) -> bool:
        """Default sync implementation that runs async method synchronously."""
        import asyncio
        try:
            loop = asyncio.get_event_loop()
            return loop.run_until_complete(self.make_async_request(url, method, **kwargs))
        except RuntimeError:
            # If no event loop is running, create a new one
            return asyncio.run(self.make_async_request(url, method, **kwargs))
    
    def benchmark_sync(self, url: str, method: str, num_requests: int, 
                      concurrent_requests: int, timeout: float) -> BenchmarkResult:
        """Run synchronous benchmark."""
        self.setup()
        
        start_time = time.time()
        start_cpu = psutil.cpu_percent()
        start_memory = psutil.Process().memory_info().rss / 1024 / 1024
        
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
        end_cpu = psutil.cpu_percent()
        end_memory = psutil.Process().memory_info().rss / 1024 / 1024
        
        total_time = end_time - start_time
        requests_per_second = num_requests / total_time if total_time > 0 else 0
        error_rate = (failed_requests / num_requests) * 100 if num_requests > 0 else 0
        
        self.teardown()
        
        return BenchmarkResult(
            library=self.library_name,
            concurrency=concurrent_requests,
            method=method,
            requests_per_second=requests_per_second,
            average_response_time=statistics.mean(response_times) if response_times else 0,
            median_response_time=statistics.median(response_times) if response_times else 0,
            p95_response_time=self._percentile(response_times, 95) if response_times else 0,
            p99_response_time=self._percentile(response_times, 99) if response_times else 0,
            error_rate=error_rate,
            total_requests=num_requests,
            successful_requests=successful_requests,
            failed_requests=failed_requests,
            cpu_usage_percent=end_cpu - start_cpu,
            memory_usage_mb=end_memory - start_memory,
            timestamp=time.time()
        )
    
    async def benchmark_async(self, url: str, method: str, num_requests: int,
                             concurrent_requests: int, timeout: float) -> BenchmarkResult:
        """Run asynchronous benchmark."""
        self.setup()
        
        start_time = time.time()
        start_cpu = psutil.cpu_percent()
        start_memory = psutil.Process().memory_info().rss / 1024 / 1024
        
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
        end_cpu = psutil.cpu_percent()
        end_memory = psutil.Process().memory_info().rss / 1024 / 1024
        
        total_time = end_time - start_time
        requests_per_second = num_requests / total_time if total_time > 0 else 0
        error_rate = (failed_requests / num_requests) * 100 if num_requests > 0 else 0
        
        self.teardown()
        
        return BenchmarkResult(
            library=self.library_name,
            concurrency=concurrent_requests,
            method=method,
            requests_per_second=requests_per_second,
            average_response_time=statistics.mean(response_times) if response_times else 0,
            median_response_time=statistics.median(response_times) if response_times else 0,
            p95_response_time=self._percentile(response_times, 95) if response_times else 0,
            p99_response_time=self._percentile(response_times, 99) if response_times else 0,
            error_rate=error_rate,
            total_requests=num_requests,
            successful_requests=successful_requests,
            failed_requests=failed_requests,
            cpu_usage_percent=end_cpu - start_cpu,
            memory_usage_mb=end_memory - start_memory,
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


class RequestXBenchmarker(BenchmarkerSync):
    """Benchmarker for RequestX library."""
    
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
            response = self.session.request(method, url, **kwargs)
            return 200 <= response.status_code < 400
        except Exception:
            return False
    
    async def make_async_request(self, url: str, method: str = 'GET', **kwargs) -> bool:
        # RequestX doesn't have async support yet, so we'll use sync in thread
        import asyncio
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, self.make_request, url, method, **kwargs)


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
        # Warmup
        if self.config.warmup_requests > 0:
            benchmarker.setup()
            for _ in range(self.config.warmup_requests):
                try:
                    await benchmarker.make_async_request(url, method, timeout=self.config.timeout)
                except Exception:
                    pass
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