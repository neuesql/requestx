# CORE HTTP ENGINE (src/core/)

## OVERVIEW
High-performance HTTP engine leveraging `hyper` and `tokio` with optimized connection pooling.

## WHERE TO LOOK
| File | Responsibility | Key Components |
|------|----------------|----------------|
| `client.rs` | HTTP implementation | `RequestxClient`, `GLOBAL_CLIENT`, `execute_request_async` |
| `runtime.rs` | Runtime execution | `RuntimeManager`, `GLOBAL_RUNTIME`, context detection |
| `mod.rs` | Module exports | Public API surface for the core engine |

## CONVENTIONS
- **Singleton Clients**: Use `OnceLock<Client>` (`GLOBAL_CLIENT`, `NOVERIFY_CLIENT`) to persist connection pools and TLS connectors.
- **TLS Caching**: Always prefer `get_noverify_client()` for `verify=False` to avoid expensive TLS handshake re-initialization.
- **Zero-Copy & Pre-allocation**: 
    - Use `Body::from(owned_data)` to move data instead of cloning.
    - Pre-allocate `String::with_capacity` for form encoding based on estimated size.
    - Utilize `CONTENT_TYPE_JSON` and `CONTENT_TYPE_FORM` constants to reduce heap allocations.
- **HTTP/2 Optimization**: Configured via `http2_only`, `initial_stream_window_size`, and `keep_alive_interval` for persistent high-throughput streams.
- **Context Awareness**: `RuntimeManager` detects Python's `asyncio` loop to automatically decide between `future_into_py` and `runtime.block_on`.

## ANTI-PATTERNS
- **Client Instantiation**: Never create a new `hyper::Client` or `HttpsConnector` inside a request loop.
- **Data Duplication**: Avoid `.clone()` on `RequestConfig.data` or `ResponseData.body` unless strictly required for ownership.
- **Parameter Mixing**: Never allow both `data` and `json` in a single `RequestConfig` (enforced in `execute_request_async`).
- **GIL Blocking**: Do not run `runtime.block_on` without `py.allow_threads()` in synchronous contexts.
- **Manual Redirects**: Do not implement redirects in Python; use the optimized `while` loop in `execute_request_async`.
