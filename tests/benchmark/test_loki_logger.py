import json
import logging
import os
import unittest

import logging_loki
from dotenv import load_dotenv

load_dotenv()


class TestLokiLogger(unittest.TestCase):
    def setUp(self):
        loki_url = os.getenv("LOKI_URL")
        loki_user = os.getenv("LOKI_USER")
        loki_password = os.getenv("LOKI_PASSWORD")
        if not loki_url or not loki_user or not loki_password:
            self.skipTest("LOKI_URL, LOKI_USER, and LOKI_PASSWORD must be set in .env file")
        self.handler = logging_loki.LokiHandler(
            url=loki_url,
            tags={
                "source": "requestx",
                "application": "benchmark-benchmark",
                "type": "cloud-logs",
                "environment": "test"
            },
            auth=(loki_user, loki_password),
            version="1"
        )

        self.logger = logging.getLogger("test-logger")
        self.logger.addHandler(self.handler)

    def test_send_log_to_loki(self):
        self.logger.error(
            "Something happened in Unit Test",
            extra={"tags": {"service": "my-service"}},
        )

    def test_send_row_to_loki(self):
        # Simulate sending a benchmark result
        row = {
            "library": "requestx",
            "method": "GET",
            "concurrency": 10,
            "total_requests": 1000,
            "successful_requests": 995,
            "failed_requests": 5,
            "requests_per_second": 125.5,
            "average_response_time_ms": 79.8,
            "median_response_time_ms": 75.2,
            "p95_response_time_ms": 120.3,
            "p99_response_time_ms": 180.7,
            "error_rate": 0.5,
            "cpu_usage_percent": 45.2,
            "memory_usage_mb": 128.5,
            "timestamp": 60.0,
        }
        self.logger.info(json.dumps(row))


if __name__ == "__main__":
    unittest.main()
