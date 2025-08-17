# RequestX Build System and Packaging

This document describes the build system and packaging setup for RequestX, a high-performance HTTP client library for Python built with Rust.

## Overview

RequestX uses a modern build system that combines:
- **Maturin** for building Python extensions from Rust code
- **uv** for Python dependency management
- **Cargo** for Rust dependency management
- **GitHub Actions** for CI/CD and automated releases

## Build System Components

### 1. Maturin Configuration

Maturin is configured in `pyproject.toml` for cross-platform wheel building:

```toml
[tool.maturin]
python-source = "python"
module-name = "requestx._requestx"
features = ["pyo3/extension-module"]
# Cross-platform wheel building configuration
strip = true
# Ensure compatibility with older glibc versions on Linux
compatibility = "linux"
# Build universal2 wheels on macOS for both x86_64 and arm64
universal2 = true
```

### 2. Cross-Platform Support

The build system supports the following platforms:

#### Linux
- **x86_64**: `x86_64-unknown-linux-gnu`
- **aarch64**: `aarch64-unknown-linux-gnu`
- **musl**: Compatible with Alpine Linux and other musl-based distributions

#### macOS
- **Universal2**: `universal2-apple-darwin` (supports both x86_64 and arm64)
- **Deployment Target**: macOS 11.0+ for optimal compatibility

#### Windows
- **x86_64**: `x86_64-pc-windows-msvc`
- **aarch64**: `aarch64-pc-windows-msvc`

### 3. Python Version Support

RequestX supports Python 3.8 through 3.12 using PyO3's abi3 stable ABI:

```toml
[dependencies]
pyo3 = { version = "0.20", features = ["extension-module", "abi3-py38"] }
```

This allows a single wheel to work across multiple Python versions.

## Build Commands

### Development Build
```bash
# Quick development setup
make dev

# Or manually:
uv sync --dev
uv run maturin develop
```

### Release Build
```bash
# Build release wheel
make build-wheels

# Or manually:
uv run maturin build --release --strip
```

### Source Distribution
```bash
# Build source distribution
make build-sdist

# Or manually:
uv run maturin sdist
```

### Cross-Platform Builds
```bash
# Build for specific target
uv run maturin build --release --strip --target x86_64-unknown-linux-gnu

# Build universal2 wheel for macOS
uv run maturin build --release --strip --target universal2-apple-darwin
```

## CI/CD Pipeline

### Workflow Structure

The CI/CD pipeline consists of multiple workflows:

1. **`ci.yml`** - Comprehensive CI pipeline with 10 stages
2. **`test.yml`** - Basic testing across platforms and Python versions
3. **`publish.yml`** - Release workflow for PyPI publishing
4. **`build-wheels.yml`** - Dedicated wheel building and testing

### CI Pipeline Stages

#### Stage 1: Code Quality & Linting
- Rust formatting (`cargo fmt --check`)
- Rust linting (`cargo clippy`)
- Python formatting (`black --check`)
- Python linting (`ruff check`)
- Type checking (`mypy`)

#### Stage 2: Build
- Build Rust extension (`maturin develop`)
- Verify Python import

#### Stage 3: Rust Unit Tests
- Run Rust tests (`cargo test --verbose`)
- Run documentation tests (`cargo test --doc`)

#### Stage 4: Python Unit Tests
- Cross-platform testing (Ubuntu, Windows, macOS)
- Multiple Python versions (3.8-3.12)
- unittest framework

#### Stage 5: Integration Tests
- Comprehensive integration tests
- Async/await functionality tests

#### Stage 6: Performance Tests
- Benchmark comparisons
- Memory usage tests

#### Stage 7: Cross-Platform Testing
- Platform-specific functionality verification

#### Stage 8: Documentation Generation
- API documentation (Sphinx)
- README examples update

#### Stage 9: Release Build
- Cross-platform wheel building
- Source distribution creation

#### Stage 10: Pipeline Summary
- Results aggregation and reporting

### Release Process

#### Automated Release (Recommended)
1. Create and push a release tag:
   ```bash
   make release-tag
   ```

2. GitHub Actions automatically:
   - Builds wheels for all platforms
   - Creates source distribution
   - Publishes to PyPI
   - Creates GitHub release

#### Manual Release
```bash
# Full release process
make release

# Individual steps:
make pre-release      # Run full CI + build artifacts
make release-tag      # Create and push git tag
make publish-pypi     # Publish to PyPI
make github-release   # Create GitHub release
```

## Wheel Building Details

### Wheel Naming Convention
Wheels follow the standard Python wheel naming convention:
```
requestx-{version}-cp{python_version}-abi3-{platform}.whl
```

Example:
```
requestx-0.2.0-cp38-abi3-macosx_11_0_arm64.whl
requestx-0.2.0-cp38-abi3-linux_x86_64.whl
requestx-0.2.0-cp38-abi3-win_amd64.whl
```

### Dependency Bundling

All Rust dependencies are statically linked into the wheel:
- **hyper** - HTTP client implementation
- **tokio** - Async runtime
- **hyper-tls** - TLS support
- **cookie_store** - Cookie management
- **serde/serde_json** - JSON serialization

This ensures:
- No external Rust dependencies required
- Single wheel installation
- Consistent behavior across environments

### Wheel Size Optimization

Wheels are optimized for size:
- **Strip symbols**: `--strip` flag removes debug symbols
- **Release build**: Optimized compilation
- **Static linking**: Reduces runtime dependencies

Typical wheel sizes:
- Linux x86_64: ~8-12 MB
- macOS universal2: ~15-20 MB
- Windows x86_64: ~8-12 MB

## Installation Testing

### Automated Testing
The build system includes comprehensive installation testing:

```bash
# Test installation process
make test-installation

# Test wheel installation in clean environment
make test-wheel-installation
```

### Test Coverage
Installation tests verify:
- ✅ Basic import functionality
- ✅ Async/await support
- ✅ Dependency bundling
- ✅ Cross-platform compatibility
- ✅ Development installation
- ✅ Wheel installation in clean environment

### Test Script
The installation test script (`scripts/test_installation.py`) provides:
- Automated testing across scenarios
- Clean environment testing
- Dependency verification
- Platform compatibility checks

## Environment Requirements

### Development Environment
- **Python**: 3.8+ (3.11+ recommended for development)
- **Rust**: Latest stable (1.70+)
- **uv**: Latest version for dependency management
- **maturin**: Installed via uv

### CI/CD Environment
- **GitHub Actions**: Ubuntu, Windows, macOS runners
- **Cross-compilation tools**: For aarch64 Linux builds
- **PyPI tokens**: For automated publishing

### Installation Requirements
- **End users**: Only Python 3.8+ required
- **No Rust toolchain**: Wheels are pre-compiled
- **No external dependencies**: Everything bundled

## Troubleshooting

### Common Build Issues

#### 1. Missing Rust Toolchain
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

#### 2. Missing uv
```bash
# Install uv
curl -LsSf https://astral.sh/uv/install.sh | sh
```

#### 3. Cross-compilation Issues
```bash
# Install cross-compilation tools (Linux)
sudo apt-get install gcc-aarch64-linux-gnu

# Add Rust target
rustup target add aarch64-unknown-linux-gnu
```

#### 4. Wheel Installation Issues
```bash
# Force rebuild
make clean
make build-wheels

# Test in clean environment
make test-wheel-installation
```

### Performance Optimization

#### Build Performance
- Use `--release` for optimized builds
- Use `--strip` to reduce wheel size
- Enable LTO for maximum optimization (if needed)

#### Runtime Performance
- Static linking reduces startup time
- abi3 stable ABI provides forward compatibility
- Optimized Rust code for maximum throughput

## Maintenance

### Regular Tasks
- Update dependencies monthly
- Test new Python versions when released
- Monitor wheel sizes and build times
- Update CI/CD workflows as needed

### Version Management
- Version is managed in `Cargo.toml`
- Automatically extracted by Makefile
- Used consistently across all build artifacts

### Security
- Dependabot for dependency updates
- Regular security audits
- Minimal dependency surface area
- Static linking reduces attack surface

## Future Enhancements

### Planned Improvements
- [ ] WASM target support
- [ ] Additional architecture support (RISC-V)
- [ ] Build caching optimization
- [ ] Reproducible builds
- [ ] Binary size optimization

### Monitoring
- Build time tracking
- Wheel size monitoring
- Download statistics
- Performance regression detection

This build system provides a robust, scalable foundation for RequestX development and distribution, ensuring high-quality releases across all supported platforms.