"""Resource metrics model for the HTTP benchmark framework."""

import uuid
from datetime import datetime
from typing import Optional
from .base import BaseModel


class ResourceMetrics(BaseModel):
    """Captures system resource usage during benchmark execution."""

    def __init__(
        self,
        benchmark_id: str,
        timestamp: datetime,
        cpu_percent: float,
        memory_mb: float,
        bytes_sent: int,
        bytes_received: int,
        disk_read_mb: float = 0.0,
        disk_write_mb: float = 0.0,
        id: Optional[str] = None,
    ):
        self.id = id or str(uuid.uuid4())
        self.benchmark_id = benchmark_id
        self.timestamp = timestamp
        self.cpu_percent = cpu_percent
        self.memory_mb = memory_mb
        self.bytes_sent = bytes_sent
        self.bytes_received = bytes_received
        self.disk_read_mb = disk_read_mb
        self.disk_write_mb = disk_write_mb
