# Technology Stack & Build System

## Core Technologies
- **Rust** - Core HTTP implementation using hyper and tokio
- **Python** - Target language with PyO3 bindings
- **PyO3** - Python-Rust interop with abi3 stable ABI support
- **Hyper** - High-performance HTTP implementation
- **Tokio** - Async runtime for Rust
- **Maturin** - Build tool for Python extensions written in Rust

## Key Dependencies

### Rust Dependencies
- `pyo3` - Python bindings with extension-module and abi3-py38 features
- `pyo3-asyncio` - Async support with tokio-runtime
- `hyper` - HTTP client with full features
- `hyper-tls` - TLS support for HTTPS
- `tokio` - Async runtime with full features
- `serde/serde_json` - JSON serialization
- `thiserror` - Error handling

### Python Dependencies
- Development tools: `black`, `ruff`, `mypy`
- Testing: Python built-in `unittest` framework (no external dependencies)

## Build System
- **Primary**: `maturin` for building Python extensions
- **Package manager**: `uv` for Python dependency management
- **Python source**: Located in `python/` directory
- **Module name**: `requestx._requestx` (internal Rust module)

## Common Commands

### Development Setup
```bash
# Install uv for dependency management
curl -LsSf https://astral.sh/uv/install.sh | sh

# Install development dependencies
uv sync --dev

# Build extension for development
uv run maturin develop
```

### Testing
```bash
# Run Python tests using unittest
uv run python -m unittest discover tests/ -v

# Run specific test file
uv run python -m unittest tests.test_core_client -v

# Run Rust tests
cargo test
```

### Code Quality
```bash
# Format Python code
uv run black .

# Lint Python code
uv run ruff check .

# Type checking
uv run mypy .

# Format Rust code
cargo fmt
```

### Building
```bash
# Development build
uv run maturin develop

# Release build
uv run maturin build --release

# Build wheel
uv run maturin build
```

## CI/CD Pipeline

### Pipeline Stages
The CI/CD pipeline should follow this sequence for comprehensive testing and release:

1. **Code Quality & Linting**
   ```bash
   # Rust formatting and linting
   cargo fmt --check
   cargo clippy -- -D warnings
   
   # Python formatting and linting
   uv run black --check .
   uv run ruff check .
   uv run mypy .
   ```

2. **Build Stage**
   ```bash
   # Build Rust extension
   uv run maturin develop
   
   # Verify Python package can be imported
   python -c "import requestx; print('Import successful')"
   ```

3. **Rust Unit Tests**
   ```bash
   # Run Rust tests with coverage
   cargo test --verbose
   cargo test --doc
   ```

4. **Python Unit Tests**
   ```bash
   # Run Python tests using unittest
   uv run python -m unittest discover tests/ -v
   
   # Run specific test modules
   uv run python -m unittest tests.test_core_client -v
   ```

5. **Integration Tests**
   ```bash
   # Test requests compatibility
   uv run python -m unittest tests.test_integration -v
   
   # Test async/sync behavior
   uv run python -m unittest tests.test_async -v
   ```



7. **Cross-Platform Testing**
   - Test on Linux (Ubuntu latest)
   - Test on macOS (latest)
   - Test on Windows (latest)
   - Test Python versions: 3.8, 3.9, 3.10, 3.11, 3.12

8. **Documentation Generation**
   ```bash
   # Generate API documentation
   uv run sphinx-build docs/ docs/_build/
   
   # Update README examples
   uv run python scripts/update_readme_examples.py
   ```

9. **Release Build**
   ```bash
   # Build wheels for all platforms
   uv run maturin build --release --strip
   
   # Build source distribution
   uv run maturin sdist
   ```

10. **Release Process**
    ```bash
    # Tag release
    git tag v${VERSION}
    git push origin v${VERSION}
    
    # Publish to PyPI
    uv run maturin publish
    
    # Create GitHub release with artifacts
    gh release create v${VERSION} --generate-notes
    ```

### GitHub Actions Workflow Structure
```yaml
# Suggested workflow stages:
- name: quality-check    # Linting, formatting
- name: build           # Compile extension
- name: test-rust       # Rust unit tests
- name: test-python     # Python unit tests  
- name: test-integration # Compatibility tests
- name: test-performance # Benchmarks
- name: build-wheels    # Cross-platform builds
- name: publish         # Release to PyPI (on tags)
```

### Environment Variables
- `PYPI_TOKEN` - PyPI authentication
- `GITHUB_TOKEN` - GitHub releases
- `CODECOV_TOKEN` - Code coverage reporting

### Artifacts to Publish
- Python wheels for all platforms (Linux, macOS, Windows)
- Source distribution (sdist)
- Documentation site
- Performance benchmark results
- GitHub release with changelog

## Configuration Files
- `Cargo.toml` - Rust package configuration with cdylib crate type
- `pyproject.toml` - Python package and tool configuration
- `.rustfmt.toml` - Rust formatting rules
- `uv.lock` - Python dependency lock file
- `.github/workflows/` - CI/CD pipeline definitions