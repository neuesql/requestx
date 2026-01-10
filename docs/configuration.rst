Configuration System
====================

RequestX features a flexible configuration system that allows you to fine-tune performance and runtime behavior without modifying your code.

Overview
--------

The configuration system externalizes "magic numbers" from the core RequestX modules into a configurable TOML file. By default, RequestX looks for a ``config.toml`` file in your project root. If the file is missing or invalid, sensible defaults are used.

Configuration File (``config.toml``)
------------------------------------

The ``config.toml`` file is divided into two main sections: ``[client]`` and ``[runtime]``.

.. code-block:: toml

    [client]
    # HTTP Connection Pool Settings
    pool_idle_timeout_secs = 90
    pool_max_idle_per_host = 1024
    
    # HTTP/2 Protocol Settings
    http2_only = false
    http2_keep_alive_interval_secs = 30
    http2_keep_alive_timeout_secs = 10
    http2_initial_stream_window_size = 262144
    http2_initial_connection_window_size = 2097152

    [runtime]
    # Tokio Async Runtime Settings
    worker_threads = 32
    max_blocking_threads = 512
    thread_name = "requestx-worker"
    thread_stack_size = 1048576

Client Configuration (``[client]``)
-----------------------------------

*   **pool_idle_timeout_secs**: How long (in seconds) to keep idle connections alive before closing them. Longer timeouts reduce connection overhead but use more memory. (Default: 90)
*   **pool_max_idle_per_host**: Maximum number of idle connections to keep per host. Increasing this significantly improves performance for applications that make many concurrent requests to the same service. (Default: 1024)
*   **http2_only**: Force all connections to use HTTP/2 protocol only. (Default: false)
*   **http2_keep_alive_interval_secs**: Interval between HTTP/2 ping frames to keep connections alive. (Default: 30)
*   **http2_keep_alive_timeout_secs**: Timeout waiting for a ping response before considering the connection dead. (Default: 10)
*   **http2_initial_stream_window_size**: Initial flow control window size per HTTP/2 stream. Larger values improve throughput for high-bandwidth connections. (Default: 262144)
*   **http2_initial_connection_window_size**: Initial flow control window size per HTTP/2 connection. (Default: 2097152)

Runtime Configuration (``[runtime]``)
-------------------------------------

*   **worker_threads**: Number of worker threads for the asynchronous runtime. Set to ``0`` for auto-detection (typically CPU cores * 2). For high-concurrency workloads, manual tuning may be beneficial. (Default: 32)
*   **max_blocking_threads**: Maximum threads for blocking operations like file I/O or DNS resolution. (Default: 512)
*   **thread_name**: Prefix for threads created by RequestX. (Default: "requestx-worker")
*   **thread_stack_size**: Stack size for each worker thread in bytes. (Default: 1048576)

Performance Tuning
------------------

For optimal performance in different environments, consider the following adjustments:

High-Concurrency API Clients
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

If you are building a service that makes thousands of requests to a few backend APIs:

*   Increase ``pool_max_idle_per_host`` to 1024 or higher.
*   Ensure ``worker_threads`` matches your available CPU resources.
*   Use ``requestx.Session()`` to take advantage of connection pooling.

Low-Memory Environments
~~~~~~~~~~~~~~~~~~~~~~~

In resource-constrained environments (like small containers):

*   Decrease ``pool_max_idle_per_host`` to 64 or 128.
*   Reduce ``worker_threads`` to match the number of allocated CPU cores.
*   Lower ``thread_stack_size`` to 524288 (512KB).

High-Latency Networks
~~~~~~~~~~~~~~~~~~~~~

If your requests travel over the public internet or high-latency links:

*   Increase ``http2_initial_stream_window_size`` and ``http2_initial_connection_window_size`` to allow more data in flight.
*   Ensure ``pool_idle_timeout_secs`` is high enough to keep connections alive during periods of inactivity.
