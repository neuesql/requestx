# AGENTS.md - python/requestx/

## OVERVIEW
Python wrapper for the Rust-based RequestX core, providing a requests-compatible API with integrated performance profiling.

## WHERE TO LOOK
| Component | File | Responsibility |
|-----------|------|----------------|
| **Public API** | `__init__.py` | Entry point, requests-compatible interface, method wrapping, exception hierarchy. |

## CONVENTIONS
- **Requests Compatibility**: Maintain strict parity with the `requests` library's API and response object structure.
- **Exception Hierarchy Mapping**: Always map internal PyO3/Rust errors to the Python-defined `RequestException` tree in `_map_exception`.
- **PyO3 Method Wrapping**: Use `_wrap_request_function` to inject exception mapping and telemetry into raw Rust bindings.
- **Monkey-patching Extensions**: Enhance Rust `Response` objects (e.g., `json()`, `raise_for_status()`) via Python-side wrapping.
- **Context-Aware Execution**: Support both sync and async callers with automatic detection where possible.


## ANTI-PATTERNS
- **Direct Binding Exposure**: Never expose `_requestx` members directly to users; always through the `__init__.py` facade.
- **Raw Builtin Exceptions**: Avoid letting `ValueError` or `RuntimeError` escape; translate them to library-specific exceptions.
- **Implicit Async/Sync Mixing**: Do not assume the execution context; verify if the caller is in an event loop before returning awaitables.
- **Placeholder Error Messages**: Exception mapping must preserve or clarify the original error context from the Rust core.
