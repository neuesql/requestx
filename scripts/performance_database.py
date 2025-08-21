#!/usr/bin/env python3
import sqlite3
import os

def create_performance_database():
    """Create performance.db with benchmark table matching the JSON schema"""
    
    # Database file path
    db_path = 'performance.db'
    
    # Remove existing database if it exists
    if os.path.exists(db_path):
        os.remove(db_path)
        print(f"Removed existing {db_path}")
    
    # Connect to SQLite database (creates file if doesn't exist)
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()
    
    # Create benchmark table with schema matching the JSON structure
    create_table_sql = """
    CREATE TABLE benchmark (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        test_time DATETIME DEFAULT CURRENT_TIMESTAMP,
        
        library TEXT NOT NULL,
        concurrency INTEGER NOT NULL,
        method TEXT NOT NULL,
        requests_per_second REAL NOT NULL,
        average_response_time_ms REAL NOT NULL,
        median_response_time_ms REAL NOT NULL,
        p95_response_time_ms REAL NOT NULL,
        p99_response_time_ms REAL NOT NULL,
        error_rate REAL NOT NULL,
        total_requests INTEGER NOT NULL,
        successful_requests INTEGER NOT NULL,
        failed_requests INTEGER NOT NULL,
        cpu_usage_percent REAL NOT NULL,
        memory_usage_mb REAL NOT NULL,
        timestamp REAL NOT NULL
    )
    """
    
    cursor.execute(create_table_sql)
    
    # Create indexes for common query patterns
    indexes = [
        "CREATE INDEX idx_library ON benchmark(library)",
        "CREATE INDEX idx_method ON benchmark(method)",
        "CREATE INDEX idx_concurrency ON benchmark(concurrency)",
        "CREATE INDEX idx_test_time ON benchmark(test_time)"
    ]
    
    for index_sql in indexes:
        cursor.execute(index_sql)
    
    # Commit changes and close connection
    conn.commit()
    conn.close()
    
    print(f"Successfully created {db_path} with benchmark table")
    print("Table schema:")
    print("- id: INTEGER PRIMARY KEY AUTOINCREMENT")
    print("- test_time: DATETIME (auto-generated)")
    print("- library: TEXT (e.g., 'requestx-sync')")
    print("- concurrency: INTEGER")
    print("- method: TEXT (e.g., 'GET')")
    print("- requests_per_second: REAL")
    print("- average_response_time_ms: REAL")
    print("- median_response_time_ms: REAL")
    print("- p95_response_time_ms: REAL")
    print("- p99_response_time_ms: REAL")
    print("- error_rate: REAL")
    print("- total_requests: INTEGER")
    print("- successful_requests: INTEGER")
    print("- failed_requests: INTEGER")
    print("- cpu_usage_percent: REAL")
    print("- memory_usage_mb: REAL")
    print("- timestamp: REAL")
    
    
if __name__ == "__main__":
    create_performance_database()