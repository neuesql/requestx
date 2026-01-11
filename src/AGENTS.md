# Rust Core Implementation (src/)

## OVERVIEW
High-performance Rust core leveraging `hyper` and `tokio` with `PyO3` bindings for Python interoperability.

## WHERE TO LOOK
- **`lib.rs`**: Entry point for the `_requestx` PyO3 module. Contains the Python-to-Rust bridge, argument parsing (`parse_kwargs`), and the main `get`/`post`/`request` function exports. It manages the conversion between Python types (like `PyDict`) and Rust structs.
- **`core/client.rs`**: The heart of the library. Implements `RequestxClient` and the critical `execute_request_async` function. Handles redirect logic, authentication header injection, and multi-part body construction (Text, Bytes, Form).
- **`core/runtime.rs`**: Manages the global `tokio` runtime via `GlobalRuntimeManager`. It is responsible for seamless sync/async execution detection, ensuring that `requestx` works correctly in both standard Python scripts and `asyncio` loops.
- **`error.rs`**: Centralized error management using the `thiserror` crate. Implements `From<RequestxError> for PyErr` to provide a requests-compatible exception hierarchy in Python (e.g., mapping `ConnectTimeout` to `PyTimeoutError`).
- **`session.rs`**: Implements the `Session` class for stateful requests. Handles cookie management and ensures connection pooling across multiple requests to the same host.
- **`response.rs`**: Defines the `Response` struct exposed to Python. It buffers the `hyper` body into `Bytes` and provides methods for decoding content into text or JSON.
- **`config.rs`**: Handles runtime configuration by reading `config.toml`. It tunes `hyper` performance parameters like `http2_initial_stream_window_size` and `pool_max_idle_per_host`.

## CONVENTIONS
- **Zero-Copy Transfers**: Move data into `hyper::Body` using `from()` whenever possible to avoid expensive cloning of request payloads during the Python-to-Rust transition.
- **Thread-Safe Singletons**: Use `std::sync::OnceLock` for shared global resources like `GLOBAL_CLIENT` and `GLOBAL_RUNTIME_MANAGER` to ensure they are initialized exactly once.
- **Boundary Error Handling**: Internal logic should strictly use `Result<T, RequestxError>`. Only convert to `PyResult<T>` at the `#[pyfunction]` or `#[pymethods]` boundary.
- **Modern PyO3 API**: Utilize `Bound<'_, T>` for all Python object interactions (introduced in PyO3 0.21) to benefit from improved safety and performance over the older `Py<T>` or `&PyAny` APIs.
- **Inlining Hot Paths**: Mark frequently called, small utility functions (like `get_global_client`) with `#[inline]` to allow the compiler to optimize across module boundaries.
- **Dependency Minimization**: Favor core `hyper` and `tokio` features. Use `serde` for all serialization needs and `thiserror` for clean, descriptive error enums.
- **Efficient String Building**: Use `String::with_capacity` when manual encoding is required (e.g., for `application/x-www-form-urlencoded` payloads) to reduce heap reallocations.

## ANTI-PATTERNS
- **Per-request Connectors**: Never instantiate a new `HttpsConnector` or `Client` inside a request function. This triggers expensive TLS handshakes and DNS resolutions. Always reuse the cached `GLOBAL_CLIENT`.
- **Mixing Body Parameters**: The API must strictly enforce that `data` and `json` parameters are mutually exclusive. Allowing both leads to ambiguous request states and potential server errors.
- **Blocking the Async Runtime**: Avoid using `std::thread::sleep` or blocking I/O calls inside `async` functions. Use `tokio::time::sleep` or offload heavy tasks to `spawn_blocking`.
- **Ad-hoc Serialization**: Do not implement custom JSON or form serializers. Leverage `serde_json` and Python's own `json.dumps` (via `pyo3::import("json")`) to ensure correctness and compatibility.
- **Unguarded Unwraps**: Avoid `.unwrap()` or `.expect()` in the request execution path. Any potential failure must be captured as a `RequestxError` and reported back to Python gracefully.
- **Ignoring Redirect Loops**: Never follow redirects without a limit. `core/client.rs` must enforce `MAX_REDIRECTS` (default 10) to protect against malicious or misconfigured servers.
