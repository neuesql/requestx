# Project Structure & Organization

## Root Directory Layout
```
requestx/
├── .kiro/                  # Kiro IDE configuration and steering
├── python/                 # Python package source
│   └── requestx/          # Main Python module
├── src/                   # Rust source code
├── tests/                 # Python tests
├── target/                # Rust build artifacts (gitignored)
├── .venv/                 # Python virtual environment (gitignored)
├── Cargo.toml             # Rust package configuration
├── pyproject.toml         # Python package configuration
└── README.md              # Project documentation
```

## Rust Source Structure (`src/`)
- `lib.rs` - Main library entry point, PyO3 module definition
- `error.rs` - Error types and Python exception conversion
- `response.rs` - Response object implementation
- `session.rs` - Session object for connection reuse
- `core/` - Core HTTP functionality
  - `mod.rs` - Module exports
  - `client.rs` - HTTP client implementation using hyper
  - `runtime.rs` - Async runtime management

## Python Package Structure (`python/`)
- `requestx/` - Main Python package
  - `__init__.py` - Public API exports
  - `_requestx.abi3.so` - Compiled Rust extension (build artifact)

## Architecture Patterns

### Rust Layer
- **Core HTTP Client** (`RequestxClient`) - Handles actual HTTP operations
- **Configuration Structs** (`RequestConfig`, `RequestData`) - Type-safe request configuration
- **Error Handling** - Custom error types with Python exception conversion
- **Async/Sync Bridge** - Runtime management for blocking on async operations

### Python Layer
- **Module Functions** - Top-level HTTP method functions (get, post, etc.)
- **Classes** - Response and Session objects with requests-compatible APIs
- **Error Mapping** - Rust errors converted to appropriate Python exceptions

### Key Design Principles
- **Requests Compatibility** - Maintain identical API surface to requests library
- **Performance First** - Leverage Rust's performance while providing Python ergonomics
- **Type Safety** - Use Rust's type system for reliable HTTP operations
- **Async Support** - Native async/await support alongside synchronous API

## File Naming Conventions
- Rust files: `snake_case.rs`
- Python files: `snake_case.py`
- Test files: `test_*.py`
- Configuration files: Standard names (`Cargo.toml`, `pyproject.toml`)

## Module Organization
- Keep HTTP method implementations in `src/lib.rs` for PyO3 bindings
- Core logic in `src/core/` modules for separation of concerns
- Python-specific objects (`Response`, `Session`) in dedicated files
- Error handling centralized in `src/error.rs`