"""
Performance profiling decorator for RequestX benchmarking.

This module provides a decorator-based method to measure performance metrics
including CPU usage, memory usage, request timing, and error rates.
"""

import time
import psutil
import threading
import functools
import asyncio
from typing import Dict, Any, Optional, Callable, Union
from dataclasses import dataclass, field
from contextlib import contextmanager
import tracemalloc
import gc


@dataclass
class PerformanceMetrics:
    """Container for performance measurement results."""
    
    # Timing metrics
    total_time: float = 0.0
    average_response_time: float = 0.0
    min_response_time: float = float('inf')
    max_response_time: float = 0.0
    connection_time: float = 0.0
    
    # Throughput metrics
    requests_per_second: float = 0.0
    total_requests: int = 0
    successful_requests: int = 0
    failed_requests: int = 0
    
    # Resource usage metrics
    cpu_usage_percent: float = 0.0
    memory_usage_mb: float = 0.0
    peak_memory_mb: float = 0.0
    memory_growth_mb: float = 0.0
    
    # Error metrics
    error_rate: float = 0.0
    errors: Dict[str, int] = field(default_factory=dict)
    
    # Additional metrics
    response_times: list = field(default_factory=list)
    metadata: Dict[str, Any] = field(default_factory=dict)


class ResourceMonitor:
    """Monitor system resources during benchmark execution."""
    
    def __init__(self):
        self.process = psutil.Process()
        self.monitoring = False
        self.cpu_samples = []
        self.memory_samples = []
        self.peak_memory = 0.0
        self.initial_memory = 0.0
        
    def start_monitoring(self):
        """Start resource monitoring in background thread."""
        self.monitoring = True
        self.initial_memory = self.process.memory_info().rss / 1024 / 1024
        self.peak_memory = self.initial_memory
        
        def monitor():
            while self.monitoring:
                try:
                    cpu = self.process.cpu_percent()
                    memory_mb = self.process.memory_info().rss / 1024 / 1024
                    
                    self.cpu_samples.append(cpu)
                    self.memory_samples.append(memory_mb)
                    self.peak_memory = max(self.peak_memory, memory_mb)
                    
                    time.sleep(0.1)  # Sample every 100ms
                except (psutil.NoSuchProcess, psutil.AccessDenied):
                    break
        
        self.monitor_thread = threading.Thread(target=monitor, daemon=True)
        self.monitor_thread.start()
    
    def stop_monitoring(self) -> Dict[str, float]:
        """Stop monitoring and return collected metrics."""
        self.monitoring = False
        if hasattr(self, 'monitor_thread'):
            self.monitor_thread.join(timeout=1.0)
        
        return {
            'cpu_usage_percent': sum(self.cpu_samples) / len(self.cpu_samples) if self.cpu_samples else 0.0,
            'memory_usage_mb': sum(self.memory_samples) / len(self.memory_samples) if self.memory_samples else 0.0,
            'peak_memory_mb': self.peak_memory,
            'memory_growth_mb': self.peak_memory - self.initial_memory
        }


class Profile:
    """
    Decorator for measuring performance metrics during function execution.
    
    Usage:
        @Profile(cpu=True, memory=True, request=True)
        def my_function():
            # Function implementation
            pass
    
    Args:
        cpu: Enable CPU usage monitoring
        memory: Enable memory usage monitoring  
        request: Enable request timing and throughput metrics
        connection: Enable connection timing measurement
        errors: Enable error tracking
        detailed: Enable detailed response time tracking
    """
    
    def __init__(
        self,
        cpu: bool = True,
        memory: bool = True,
        request: bool = True,
        connection: bool = False,
        errors: bool = True,
        detailed: bool = True
    ):
        self.cpu = cpu
        self.memory = memory
        self.request = request
        self.connection = connection
        self.errors = errors
        self.detailed = detailed
        
    def __call__(self, func: Callable) -> Callable:
        """Apply profiling to the decorated function."""
        
        @functools.wraps(func)
        def sync_wrapper(*args, **kwargs):
            return self._profile_sync(func, *args, **kwargs)
        
        @functools.wraps(func)
        async def async_wrapper(*args, **kwargs):
            return await self._profile_async(func, *args, **kwargs)
        
        # Return appropriate wrapper based on function type
        if asyncio.iscoroutinefunction(func):
            async_wrapper._profiler = self
            return async_wrapper
        else:
            sync_wrapper._profiler = self
            return sync_wrapper
    
    def _profile_sync(self, func: Callable, *args, **kwargs) -> Any:
        """Profile synchronous function execution."""
        metrics = PerformanceMetrics()
        monitor = ResourceMonitor() if (self.cpu or self.memory) else None
        
        # Start memory tracing if detailed memory tracking is enabled
        if self.memory and self.detailed:
            tracemalloc.start()
        
        # Start resource monitoring
        if monitor:
            monitor.start_monitoring()
        
        start_time = time.perf_counter()
        
        try:
            # Execute function
            result = func(*args, **kwargs)
            
            # Record successful execution
            metrics.successful_requests = 1
            metrics.total_requests = 1
            
        except Exception as e:
            # Record failed execution
            metrics.failed_requests = 1
            metrics.total_requests = 1
            metrics.errors[type(e).__name__] = metrics.errors.get(type(e).__name__, 0) + 1
            
            if not self.errors:
                raise
            result = None
        
        end_time = time.perf_counter()
        execution_time = end_time - start_time
        
        # Calculate timing metrics
        if self.request:
            metrics.total_time = execution_time
            metrics.average_response_time = execution_time
            metrics.min_response_time = execution_time
            metrics.max_response_time = execution_time
            metrics.requests_per_second = 1.0 / execution_time if execution_time > 0 else 0.0
            
            if self.detailed:
                metrics.response_times = [execution_time]
        
        # Stop resource monitoring and collect metrics
        if monitor:
            resource_metrics = monitor.stop_monitoring()
            if self.cpu:
                metrics.cpu_usage_percent = resource_metrics['cpu_usage_percent']
            if self.memory:
                metrics.memory_usage_mb = resource_metrics['memory_usage_mb']
                metrics.peak_memory_mb = resource_metrics['peak_memory_mb']
                metrics.memory_growth_mb = resource_metrics['memory_growth_mb']
        
        # Stop memory tracing
        if self.memory and self.detailed and tracemalloc.is_tracing():
            current, peak = tracemalloc.get_traced_memory()
            tracemalloc.stop()
            metrics.metadata['traced_memory_current'] = current / 1024 / 1024  # MB
            metrics.metadata['traced_memory_peak'] = peak / 1024 / 1024  # MB
        
        # Calculate error rate
        if self.errors:
            metrics.error_rate = metrics.failed_requests / metrics.total_requests if metrics.total_requests > 0 else 0.0
        
        # Force garbage collection for consistent memory measurements
        if self.memory:
            gc.collect()
        
        # Store metrics in function result if possible
        try:
            if hasattr(result, '__dict__') and not isinstance(result, (str, int, float, bool)):
                result._performance_metrics = metrics
            elif isinstance(result, dict):
                result['_performance_metrics'] = metrics
            elif isinstance(result, list):
                # For lists, we can't attach attributes, so we store in a global registry
                if not hasattr(self, '_metrics_registry'):
                    self._metrics_registry = {}
                self._metrics_registry[id(result)] = metrics
        except (AttributeError, TypeError):
            # Can't attach metrics to immutable types like strings, numbers
            pass
        
        # Always store the last metrics for retrieval
        self._last_metrics = metrics
        
        return result
    
    async def _profile_async(self, func: Callable, *args, **kwargs) -> Any:
        """Profile asynchronous function execution."""
        metrics = PerformanceMetrics()
        monitor = ResourceMonitor() if (self.cpu or self.memory) else None
        
        # Start memory tracing if detailed memory tracking is enabled
        if self.memory and self.detailed:
            tracemalloc.start()
        
        # Start resource monitoring
        if monitor:
            monitor.start_monitoring()
        
        start_time = time.perf_counter()
        
        try:
            # Execute async function
            result = await func(*args, **kwargs)
            
            # Record successful execution
            metrics.successful_requests = 1
            metrics.total_requests = 1
            
        except Exception as e:
            # Record failed execution
            metrics.failed_requests = 1
            metrics.total_requests = 1
            metrics.errors[type(e).__name__] = metrics.errors.get(type(e).__name__, 0) + 1
            
            if not self.errors:
                raise
            result = None
        
        end_time = time.perf_counter()
        execution_time = end_time - start_time
        
        # Calculate timing metrics
        if self.request:
            metrics.total_time = execution_time
            metrics.average_response_time = execution_time
            metrics.min_response_time = execution_time
            metrics.max_response_time = execution_time
            metrics.requests_per_second = 1.0 / execution_time if execution_time > 0 else 0.0
            
            if self.detailed:
                metrics.response_times = [execution_time]
        
        # Stop resource monitoring and collect metrics
        if monitor:
            resource_metrics = monitor.stop_monitoring()
            if self.cpu:
                metrics.cpu_usage_percent = resource_metrics['cpu_usage_percent']
            if self.memory:
                metrics.memory_usage_mb = resource_metrics['memory_usage_mb']
                metrics.peak_memory_mb = resource_metrics['peak_memory_mb']
                metrics.memory_growth_mb = resource_metrics['memory_growth_mb']
        
        # Stop memory tracing
        if self.memory and self.detailed and tracemalloc.is_tracing():
            current, peak = tracemalloc.get_traced_memory()
            tracemalloc.stop()
            metrics.metadata['traced_memory_current'] = current / 1024 / 1024  # MB
            metrics.metadata['traced_memory_peak'] = peak / 1024 / 1024  # MB
        
        # Calculate error rate
        if self.errors:
            metrics.error_rate = metrics.failed_requests / metrics.total_requests if metrics.total_requests > 0 else 0.0
        
        # Force garbage collection for consistent memory measurements
        if self.memory:
            gc.collect()
        
        # Store metrics in function result if possible
        try:
            if hasattr(result, '__dict__') and not isinstance(result, (str, int, float, bool)):
                result._performance_metrics = metrics
            elif isinstance(result, dict):
                result['_performance_metrics'] = metrics
            elif isinstance(result, list):
                # For lists, we can't attach attributes, so we store in a global registry
                if not hasattr(self, '_metrics_registry'):
                    self._metrics_registry = {}
                self._metrics_registry[id(result)] = metrics
        except (AttributeError, TypeError):
            # Can't attach metrics to immutable types like strings, numbers
            pass
        
        # Always store the last metrics for retrieval
        self._last_metrics = metrics
        
        return result


@contextmanager
def profile_context(
    cpu: bool = True,
    memory: bool = True,
    request: bool = True,
    connection: bool = False,
    errors: bool = True,
    detailed: bool = True
):
    """
    Context manager for profiling code blocks.
    
    Usage:
        with profile_context(cpu=True, memory=True) as metrics:
            # Code to profile
            pass
        
        print(f"CPU usage: {metrics.cpu_usage_percent}%")
    """
    metrics = PerformanceMetrics()
    monitor = ResourceMonitor() if (cpu or memory) else None
    
    # Start memory tracing if detailed memory tracking is enabled
    if memory and detailed:
        tracemalloc.start()
    
    # Start resource monitoring
    if monitor:
        monitor.start_monitoring()
    
    start_time = time.perf_counter()
    
    try:
        yield metrics
        
        # Record successful execution
        metrics.successful_requests = 1
        metrics.total_requests = 1
        
    except Exception as e:
        # Record failed execution
        metrics.failed_requests = 1
        metrics.total_requests = 1
        metrics.errors[type(e).__name__] = metrics.errors.get(type(e).__name__, 0) + 1
        
        if not errors:
            raise
    
    finally:
        end_time = time.perf_counter()
        execution_time = end_time - start_time
        
        # Calculate timing metrics
        if request:
            metrics.total_time = execution_time
            metrics.average_response_time = execution_time
            metrics.min_response_time = execution_time
            metrics.max_response_time = execution_time
            metrics.requests_per_second = 1.0 / execution_time if execution_time > 0 else 0.0
            
            if detailed:
                metrics.response_times = [execution_time]
        
        # Stop resource monitoring and collect metrics
        if monitor:
            resource_metrics = monitor.stop_monitoring()
            if cpu:
                metrics.cpu_usage_percent = resource_metrics['cpu_usage_percent']
            if memory:
                metrics.memory_usage_mb = resource_metrics['memory_usage_mb']
                metrics.peak_memory_mb = resource_metrics['peak_memory_mb']
                metrics.memory_growth_mb = resource_metrics['memory_growth_mb']
        
        # Stop memory tracing
        if memory and detailed and tracemalloc.is_tracing():
            current, peak = tracemalloc.get_traced_memory()
            tracemalloc.stop()
            metrics.metadata['traced_memory_current'] = current / 1024 / 1024  # MB
            metrics.metadata['traced_memory_peak'] = peak / 1024 / 1024  # MB
        
        # Calculate error rate
        if errors:
            metrics.error_rate = metrics.failed_requests / metrics.total_requests if metrics.total_requests > 0 else 0.0
        
        # Force garbage collection for consistent memory measurements
        if memory:
            gc.collect()


def aggregate_metrics(metrics_list: list) -> PerformanceMetrics:
    """
    Aggregate multiple PerformanceMetrics instances into a single summary.
    
    Args:
        metrics_list: List of PerformanceMetrics instances to aggregate
        
    Returns:
        Aggregated PerformanceMetrics instance
    """
    if not metrics_list:
        return PerformanceMetrics()
    
    aggregated = PerformanceMetrics()
    
    # Aggregate timing metrics
    total_times = [m.total_time for m in metrics_list if m.total_time > 0]
    response_times = []
    for m in metrics_list:
        response_times.extend(m.response_times)
    
    if total_times:
        aggregated.total_time = sum(total_times)
        aggregated.average_response_time = sum(total_times) / len(total_times)
        aggregated.min_response_time = min(total_times)
        aggregated.max_response_time = max(total_times)
    
    # Aggregate request metrics
    aggregated.total_requests = sum(m.total_requests for m in metrics_list)
    aggregated.successful_requests = sum(m.successful_requests for m in metrics_list)
    aggregated.failed_requests = sum(m.failed_requests for m in metrics_list)
    
    if aggregated.total_time > 0:
        aggregated.requests_per_second = aggregated.total_requests / aggregated.total_time
    
    # Aggregate resource metrics (averages)
    cpu_values = [m.cpu_usage_percent for m in metrics_list if m.cpu_usage_percent > 0]
    memory_values = [m.memory_usage_mb for m in metrics_list if m.memory_usage_mb > 0]
    peak_memory_values = [m.peak_memory_mb for m in metrics_list if m.peak_memory_mb > 0]
    
    if cpu_values:
        aggregated.cpu_usage_percent = sum(cpu_values) / len(cpu_values)
    if memory_values:
        aggregated.memory_usage_mb = sum(memory_values) / len(memory_values)
    if peak_memory_values:
        aggregated.peak_memory_mb = max(peak_memory_values)
    
    # Aggregate errors
    for m in metrics_list:
        for error_type, count in m.errors.items():
            aggregated.errors[error_type] = aggregated.errors.get(error_type, 0) + count
    
    # Calculate error rate
    if aggregated.total_requests > 0:
        aggregated.error_rate = aggregated.failed_requests / aggregated.total_requests
    
    # Store all response times
    aggregated.response_times = response_times
    
    return aggregated


def get_last_metrics(profiled_function) -> Optional[PerformanceMetrics]:
    """
    Get the last performance metrics from a profiled function.
    
    Args:
        profiled_function: Function decorated with @Profile
        
    Returns:
        Last PerformanceMetrics instance or None if not available
    """
    if hasattr(profiled_function, '_profiler') and hasattr(profiled_function._profiler, '_last_metrics'):
        return profiled_function._profiler._last_metrics
    return None