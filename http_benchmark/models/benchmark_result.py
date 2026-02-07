"""Benchmark result model for the HTTP benchmark framework."""

import uuid
from datetime import datetime
from typing import Dict, Any, Optional
from .base import BaseModel


class BenchmarkResult(BaseModel):
    """Contains performance metrics from a single benchmark run."""

    def __init__(
        self,
        name: str,
        client_library: str,
        client_type: str,
        http_method: str,
        url: str,
        start_time: datetime,
        end_time: datetime,
        duration: float,
        requests_count: int,
        requests_per_second: float,
        avg_response_time: float,
        min_response_time: float,
        max_response_time: float,
        p95_response_time: float,
        p99_response_time: float,
        cpu_usage_avg: float,
        memory_usage_avg: float,
        network_io: Dict[str, int],
        error_count: int,
        error_rate: float,
        concurrency_level: int,
        config_snapshot: Dict[str, Any],
        id: Optional[str] = None,
    ):
        self.id = id or str(uuid.uuid4())
        self.name = name
        self.client_library = client_library
        self.client_type = client_type
        self.http_method = http_method
        self.url = url
        self.start_time = start_time
        self.end_time = end_time
        self.duration = duration
        self.requests_count = requests_count
        self.requests_per_second = requests_per_second
        self.avg_response_time = avg_response_time
        self.min_response_time = min_response_time
        self.max_response_time = max_response_time
        self.p95_response_time = p95_response_time
        self.p99_response_time = p99_response_time
        self.cpu_usage_avg = cpu_usage_avg
        self.memory_usage_avg = memory_usage_avg
        self.network_io = network_io
        self.error_count = error_count
        self.error_rate = error_rate
        self.concurrency_level = concurrency_level
        self.config_snapshot = config_snapshot
