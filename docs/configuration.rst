RequestX Configuration System Implementation
===========================================

This document describes the implementation of the externalized configuration system for RequestX performance settings.

Overview
--------

The configuration system has been successfully implemented to externalize "magic numbers" from the core RequestX modules into a configurable TOML file. This allows users to customize performance settings without modifying the source code.

Files Modified/Created
----------------------

1. ``config.toml`` - Configuration File
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
Contains all performance-related settings with sensible defaults::

    [http_client]
    pool_idle_timeout_secs = 90
    pool_max_idle_per_host = 50
    http2_only = false
    http2_initial_stream_window_size = 65536
    http2_initial_connection_window_size = 1048576

    [runtime]
    worker_threads = 8
    max_blocking_threads = 512
    thread_name = "requestx-worker"
    thread_stack_size = 524288

    [session]
    pool_idle_timeout_secs = 90
    pool_max_idle_per_host = 512
    http2_only = false
    http2_keep_alive_interval_secs = 30
    http2_keep_alive_timeout_secs = 10
    http2_initial_stream_window_size = 65536
    http2_initial_connection_window_size = 1048576
2. ``src/config.rs`` - Configuration Module
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
Defines configuration structures and loading logic:

- ``HttpClientConfig`` - HTTP client performance settings
- ``RuntimeConfig`` - Tokio runtime configuration
- ``SessionConfig`` - Session-specific settings
- Configuration loading with fallback to defaults
- Thread-safe global configuration access

3. Updated Core Modules
~~~~~~~~~~~~~~~~~~~~~~~

``src/core/client.rs``
^^^^^^^^^^^^^^^^^^^^^^
- Replaced hardcoded values in ``get_global_client()``
- Updated ``create_custom_client()`` to use configuration
- Added import for ``get_http_client_config``

``src/core/runtime.rs``
^^^^^^^^^^^^^^^^^^^^^^^
- Replaced hardcoded runtime settings
- Updated ``get_global_runtime()`` to use ``get_runtime_config()``
- Added import for ``get_runtime_config``

``src/session.rs``
^^^^^^^^^^^^^^^^^^
- Updated ``Session::new()`` to use session configuration
- Replaced hardcoded HTTP/2 and connection pool settings
- Added import for ``get_session_config``

``src/lib.rs``
^^^^^^^^^^^^^^
- Added ``mod config;`` to expose the configuration module

``Cargo.toml``
^^^^^^^^^^^^^^
- Added ``toml = "0.8"`` dependency for TOML parsing

Configuration Structure
-----------------------

The configuration system consists of two main sections:

1. Client Configuration (``[client]``)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
- ``pool_idle_timeout_secs``: Connection pool idle timeout (default: 90 seconds)
- ``pool_max_idle_per_host``: Maximum idle connections per host (default: 512)
- ``http2_only``: Force HTTP/2 only connections (default: false)
- ``http2_keep_alive_interval_secs``: HTTP/2 keep-alive interval (default: 30 seconds)
- ``http2_keep_alive_timeout_secs``: HTTP/2 keep-alive timeout (default: 10 seconds)
- ``http2_initial_stream_window_size``: HTTP/2 stream window size (default: 65536)
- ``http2_initial_connection_window_size``: HTTP/2 connection window size (default: 1048576)

2. Runtime Configuration (``[runtime]``)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
- ``worker_threads``: Number of worker threads (0 = auto-detect, default: 0)
- ``max_blocking_threads``: Maximum blocking threads (default: 512)
- ``thread_name``: Thread name prefix (default: "requestx-worker")
- ``thread_stack_size``: Thread stack size in bytes (default: 524288)

Key Features
------------

1. **Fallback to Defaults**: If ``config.toml`` is missing or invalid, the system uses sensible defaults
2. **Thread-Safe**: Configuration is loaded once and cached globally
3. **Type Safety**: All configuration values are properly typed
4. **Performance Optimized**: Configuration loading happens once at startup
5. **Backward Compatible**: Existing code continues to work without changes

Usage
-----

Users can now customize RequestX performance by:

1. Creating/modifying ``config.toml`` in the project root
2. Adjusting values based on their specific use case:
   - High-performance scenarios: Increase connection pools and worker threads
   - Low-resource environments: Reduce thread counts and connection limits
   - HTTP/2 optimization: Adjust window sizes and keep-alive settings

Benefits
--------

1. **Maintainability**: No more scattered "magic numbers" in the codebase
2. **Flexibility**: Easy performance tuning without code changes
3. **Documentation**: All settings are clearly documented in the TOML file
4. **Testing**: Different configurations can be tested easily
5. **Deployment**: Different environments can use different configurations

Verification
------------

The implementation has been verified through:

- Successful compilation with ``cargo check``
- All hardcoded values replaced with configuration calls
- Proper error handling for missing/invalid configuration files
- Thread-safe global configuration access

The configuration system is now ready for production use and provides a clean, maintainable way to manage RequestX performance settings.