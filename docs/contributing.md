# Contributing Guide

Thank you for your interest in contributing to RequestX! This guide will help you get started.

## Development Setup

### Prerequisites

- Python 3.12 or higher
- Rust toolchain (rustc, cargo)
- uv (recommended) or pip

### Clone the Repository

```bash
git clone https://github.com/neuesql/requestx.git
cd requestx
```

### Setup Development Environment

Using the Makefile:

```bash
make 1-setup
```

Or manually:

```bash
# Install uv if you haven't
curl -LsSf https://astral.sh/uv/install.sh | sh

# Create virtual environment and install dependencies
uv sync --all-extras
```

### Build the Project

```bash
make 5-build
```

Or directly:

```bash
uv run maturin develop
```

## Development Workflow

### 1. Format Code

```bash
make 2-format
```

This formats both Rust and Python code:
- Rust: `cargo fmt`
- Python: `black`

### 2. Check Formatting

```bash
make 2-format-check
```

### 3. Run Linters

```bash
make 3-lint
```

This runs:
- Rust: `cargo clippy`
- Python: `ruff`

### 4. Run Quality Checks

```bash
make 4-quality-check
```

Combines format check and linting.

### 5. Build

```bash
make 5-build
```

### 6. Run Tests

```bash
# All tests
make 6-test-all

# Rust tests only
make 6-test-rust

# Python tests only
make 6-test-python
```

## Project Structure

```
requestx/
├── src/                    # Rust source code
│   ├── lib.rs             # PyO3 module definition
│   ├── client.rs          # Client implementations
│   ├── response.rs        # Response type
│   ├── error.rs           # Error types
│   ├── types.rs           # Configuration types
│   ├── request.rs         # Module-level functions
│   └── streaming.rs       # Streaming responses
├── python/requestx/       # Python package
│   └── __init__.py        # Re-exports
├── tests/                 # Python tests
├── docs/                  # Documentation
├── Cargo.toml             # Rust dependencies
├── pyproject.toml         # Python config
└── Makefile               # Development commands
```

## Making Changes

### Adding a New Feature

1. Create a feature branch:
   ```bash
   git checkout -b feature/my-feature
   ```

2. Make your changes in the appropriate files

3. Add tests for new functionality

4. Run the full test suite:
   ```bash
   make 6-test-all
   ```

5. Update documentation if needed

6. Submit a pull request

### Adding a New Client Option

1. Add field to `ClientConfig` in `src/client.rs`
2. Update `Client::new()` and `AsyncClient::new()` signatures
3. Apply the config in `build_reqwest_client()` / `build_blocking_client()`
4. Export from `python/requestx/__init__.py` if it's a new type
5. Add tests in `tests/test_sync.py` and `tests/test_async.py`
6. Update documentation

### Adding a New Exception Type

1. Define in `src/error.rs` using `create_exception!` macro
2. Add variant to `ErrorKind` enum
3. Add constructor method to `Error` impl
4. Map in `From<Error> for PyErr` impl
5. Register in `lib.rs` module init
6. Export from `python/requestx/__init__.py`

## Code Style

### Rust

- Follow standard Rust style guidelines
- Use `cargo fmt` for formatting
- Address all `clippy` warnings
- Write documentation comments for public APIs

### Python

- Follow PEP 8 guidelines
- Use `black` for formatting
- Use type hints where appropriate
- Write docstrings for public functions

## Testing

### Writing Tests

- Place Python tests in `tests/` directory
- Use `pytest` for Python tests
- Use `cargo test` for Rust tests

### Test Coverage

Ensure your changes have adequate test coverage:

```bash
# Run Python tests with coverage
uv run pytest --cov=requestx tests/
```

## Documentation

### Building Docs

```bash
make 7-doc-build
```

### Documentation Guidelines

- Update docs when adding new features
- Include code examples
- Keep explanations clear and concise

## Pull Request Process

1. **Fork the repository** and create your branch from `main`

2. **Make your changes** following the guidelines above

3. **Add tests** for any new functionality

4. **Run the full test suite** to ensure nothing is broken

5. **Update documentation** as needed

6. **Create a pull request** with a clear description of changes

### PR Checklist

- [ ] Code follows project style guidelines
- [ ] Tests pass locally
- [ ] Documentation is updated
- [ ] Commit messages are clear and descriptive

## Getting Help

- **Issues**: [GitHub Issues](https://github.com/neuesql/requestx/issues)
- **Discussions**: [GitHub Discussions](https://github.com/neuesql/requestx/discussions)

## License

By contributing to RequestX, you agree that your contributions will be licensed under the MIT License.
