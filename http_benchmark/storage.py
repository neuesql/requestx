"""Storage module for the HTTP benchmark framework."""

import sqlite3
import json
from datetime import datetime
from typing import List, Dict, Any, Optional
from .models.benchmark_result import BenchmarkResult


class ResultStorage:
    """Handle storage and retrieval of benchmark results using SQLite."""

    def __init__(self, db_path: str = "benchmark_results.db"):
        self.db_path = db_path
        self.init_db()

    def init_db(self) -> None:
        """Initialize the SQLite database with required tables."""
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        # Create benchmark_results table
        cursor.execute(
            """
            CREATE TABLE IF NOT EXISTS benchmark_results (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                client_library TEXT NOT NULL,
                client_type TEXT NOT NULL,
                http_method TEXT NOT NULL,
                url TEXT NOT NULL,
                start_time TEXT NOT NULL,
                end_time TEXT NOT NULL,
                duration REAL NOT NULL,
                requests_count INTEGER NOT NULL,
                requests_per_second REAL NOT NULL,
                avg_response_time REAL NOT NULL,
                min_response_time REAL NOT NULL,
                max_response_time REAL NOT NULL,
                p95_response_time REAL NOT NULL,
                p99_response_time REAL NOT NULL,
                cpu_usage_avg REAL NOT NULL,
                memory_usage_avg REAL NOT NULL,
                network_io TEXT NOT NULL,
                error_count INTEGER NOT NULL,
                error_rate REAL NOT NULL,
                concurrency_level INTEGER NOT NULL,
                config_snapshot TEXT NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            )
        """
        )

        conn.commit()
        conn.close()

    def save_result(self, result: BenchmarkResult) -> None:
        """Save a benchmark result to the database."""
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        cursor.execute(
            """
            INSERT INTO benchmark_results (
                id, name, client_library, client_type, http_method, url, start_time, end_time,
                duration, requests_count, requests_per_second, avg_response_time,
                min_response_time, max_response_time, p95_response_time, p99_response_time,
                cpu_usage_avg, memory_usage_avg, network_io, error_count, error_rate,
                concurrency_level, config_snapshot
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """,
            (
                result.id,
                result.name,
                result.client_library,
                result.client_type,
                result.http_method,
                result.url,
                result.start_time.isoformat(),
                result.end_time.isoformat(),
                result.duration,
                result.requests_count,
                result.requests_per_second,
                result.avg_response_time,
                result.min_response_time,
                result.max_response_time,
                result.p95_response_time,
                result.p99_response_time,
                result.cpu_usage_avg,
                result.memory_usage_avg,
                json.dumps(result.network_io),
                result.error_count,
                result.error_rate,
                result.concurrency_level,
                json.dumps(result.config_snapshot),
            ),
        )

        conn.commit()
        conn.close()

    def get_result_by_id(self, result_id: str) -> Optional[BenchmarkResult]:
        """Retrieve a benchmark result by its ID."""
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        cursor.execute(
            """
            SELECT * FROM benchmark_results WHERE id = ?
        """,
            (result_id,),
        )

        row = cursor.fetchone()
        conn.close()

        if row:
            return self._row_to_benchmark_result(row)
        return None

    def get_results_by_name(self, name: str) -> List[BenchmarkResult]:
        """Retrieve benchmark results by name."""
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        cursor.execute(
            """
            SELECT * FROM benchmark_results WHERE name = ?
        """,
            (name,),
        )

        rows = cursor.fetchall()
        conn.close()

        return [self._row_to_benchmark_result(row) for row in rows]

    def get_all_results(self) -> List[BenchmarkResult]:
        """Retrieve all benchmark results."""
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        cursor.execute(
            """
            SELECT * FROM benchmark_results ORDER BY created_at DESC
        """
        )

        rows = cursor.fetchall()
        conn.close()

        return [self._row_to_benchmark_result(row) for row in rows]

    def compare_results(self, result_ids: List[str]) -> List[Dict[str, Any]]:
        """Compare multiple benchmark results."""
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        placeholders = ",".join("?" * len(result_ids))
        cursor.execute(
            f"""
            SELECT * FROM benchmark_results WHERE id IN ({placeholders})
        """,
            result_ids,
        )

        rows = cursor.fetchall()
        conn.close()

        results = [self._row_to_benchmark_result(row) for row in rows]

        comparison = []
        for result in results:
            comparison.append(
                {
                    "id": result.id,
                    "name": result.name,
                    "client_library": result.client_library,
                    "requests_per_second": result.requests_per_second,
                    "avg_response_time": result.avg_response_time,
                    "error_rate": result.error_rate,
                    "cpu_usage_avg": result.cpu_usage_avg,
                    "memory_usage_avg": result.memory_usage_avg,
                }
            )

        return comparison

    def _row_to_benchmark_result(self, row: tuple) -> BenchmarkResult:
        """Convert a database row to a BenchmarkResult object."""
        return BenchmarkResult(
            id=row[0],
            name=row[1],
            client_library=row[2],
            client_type=row[3],
            http_method=row[4],
            url=row[5],
            start_time=datetime.fromisoformat(row[6]),
            end_time=datetime.fromisoformat(row[7]),
            duration=row[8],
            requests_count=row[9],
            requests_per_second=row[10],
            avg_response_time=row[11],
            min_response_time=row[12],
            max_response_time=row[13],
            p95_response_time=row[14],
            p99_response_time=row[15],
            cpu_usage_avg=row[16],
            memory_usage_avg=row[17],
            network_io=json.loads(row[18]),
            error_count=row[19],
            error_rate=row[20],
            concurrency_level=row[21],
            config_snapshot=json.loads(row[22]),
        )
