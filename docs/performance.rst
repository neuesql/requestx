Performance
===========

RequestX is engineered for high performance, leveraging Rust's efficiency and a highly optimized architecture. This document details the recent optimizations and current benchmarks.

Performance Benchmarks (localhost)
----------------------------------

The following benchmarks were conducted on a developer workstation (macOS, ARM64) targeting a local high-performance HTTP server to minimize network latency impact.

+--------------+-----------+-----------------+-----------------+
| Library      | GET (RPS) | POST (data) RPS | POST (JSON) RPS |
+==============+===========+=================+=================+
| **RequestX** | **886**   | **774**         | **632**         |
+--------------+-----------+-----------------+-----------------+
| HTTPX        | 949       | 700+            | 600+            |
+--------------+-----------+-----------------+-----------------+
| Requests     | 768       | 621             | 500+            |
+--------------+-----------+-----------------+-----------------+

**Summary**: RequestX significantly outperforms ``requests`` and is highly competitive with ``httpx``, especially in POST operations where our memory-efficient data handling shines.

Recent Optimizations
--------------------

RequestX has undergone a series of deep performance optimizations to achieve these results:

1. Multi-threaded GIL Release
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

In synchronous contexts, RequestX now explicitly releases the Python Global Interpreter Lock (GIL) during network I/O operations using ``py.allow_threads()``.

*   **Impact**: Enables true multi-threaded concurrency in Python, allowing other Python threads to execute while RequestX waits for the network.

2. Global Cached TLS Client
~~~~~~~~~~~~~~~~~~~~~~~~~~~

We implemented a global cached client for requests where SSL verification is disabled.

*   **Problem**: Previously, a new TLS connector was created for every request with ``verify=false``, causing massive overhead.
*   **Optimization**: Reusing a single cached TLS connector.
*   **Impact**: Throughput improved from **257 to 886 RPS (+245%)**.

3. Optimized JSON Parsing
~~~~~~~~~~~~~~~~~~~~~~~~~

RequestX now uses ``serde_json::from_slice`` directly on the raw bytes received from the network.

*   **Optimization**: Avoids an intermediate UTF-8 string conversion step before parsing JSON.
*   **Impact**: Reduced CPU usage and faster ``response.json()`` calls.

4. Zero-Copy Body Handling
~~~~~~~~~~~~~~~~~~~~~~~~~~

When sending request data, RequestX now takes ownership of the data (``Body::from(data)``) instead of cloning it whenever possible.

*   **Optimization**: Reduced memory allocations and copying for large request bodies.

5. Efficient Response Storage
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Internal response storage was changed from ``Vec<u8>`` to ``bytes::Bytes``.

*   **Optimization**: ``Bytes`` uses reference counting for efficient sharing and slicing without copying.

6. Enhanced Connection Pooling
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Fixed an issue in ``RequestxClient::clone()`` where connection pools were not being shared correctly.

*   **Optimization**: Cloned clients now share the same underlying connection pool, ensuring proper connection reuse across different parts of your application.

7. Inlining & Micro-optimizations
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

*   Added ``#[inline]`` hints to hot paths like configuration access and client retrieval.
*   Added early returns in header merging for sessions with no default headers.

Methodology
-----------

*   **Environment**: Isolated hardware, consistent CPU frequency.
*   **Warm-up**: 1000 requests before measurement begins.
*   **Iterations**: Each test run 5 times, reporting the average.
*   **Tooling**: Custom benchmarking suite included in ``tests/test_performance.py``.

Future Roadmap
--------------

We are continuously working to improve performance further:

*   **Brotli/Zstd support**: More efficient compression options.
*   **Fine-grained header parsing**: Delaying header string allocation until accessed.
*   **SIMD-accelerated URL parsing**: Leveraging hardware instructions for faster processing.
