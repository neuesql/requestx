# Installation Guide

RequestX is designed to be easy to install and use across all major platforms.

## Requirements

- **Python**: 3.12 or higher
- **Operating System**: Windows, macOS, or Linux
- **Architecture**: x86_64, ARM64 (Apple Silicon, ARM64 Linux)

No additional dependencies or build tools are required - RequestX comes with all Rust dependencies pre-compiled and bundled.

## Standard Installation

Install RequestX using pip:

```bash
pip install requestx
```

This will install the latest stable version from PyPI with pre-built wheels for your platform.

## Development Installation

If you want to install the latest development version from GitHub:

```bash
pip install git+https://github.com/neuesql/requestx.git
```

## Virtual Environment Installation

It's recommended to install RequestX in a virtual environment:

```bash
# Create virtual environment
python -m venv requestx-env

# Activate virtual environment
# On Windows:
requestx-env\Scripts\activate
# On macOS/Linux:
source requestx-env/bin/activate

# Install RequestX
pip install requestx
```

## Using uv (Recommended)

For faster installation and better dependency management, use [uv](https://github.com/astral-sh/uv):

```bash
# Install uv if you haven't already
curl -LsSf https://astral.sh/uv/install.sh | sh

# Create project with RequestX
uv init my-project
cd my-project
uv add requestx

# Run your code
uv run python your_script.py
```

## Platform-Specific Notes

### Windows

RequestX works on all supported Windows versions:

- Windows 10 and 11 (x86_64 and ARM64)
- Windows Server 2019 and 2022

```cmd
pip install requestx
```

### macOS

RequestX provides universal wheels that work on both Intel and Apple Silicon Macs:

- macOS 11.0 (Big Sur) and later
- Both x86_64 (Intel) and ARM64 (Apple Silicon) architectures

```bash
pip install requestx
```

### Linux

RequestX supports all major Linux distributions:

- Ubuntu 20.04 LTS and later
- CentOS/RHEL 8 and later
- Debian 11 and later
- Both x86_64 and ARM64 architectures

```bash
pip install requestx
```

## Docker Installation

Use RequestX in Docker containers:

```dockerfile
FROM python:3.12-slim

# Install RequestX
RUN pip install requestx

# Copy your application
COPY . /app
WORKDIR /app

# Run your application
CMD ["python", "app.py"]
```

## Verification

Verify your installation by running:

```python
import requestx

# Make a test request
response = requestx.get("https://httpbin.org/json")
print(f"Status: {response.status_code}")
print("Installation successful!")
```

You should see output similar to:

```
Status: 200
Installation successful!
```

## Troubleshooting

### Installation Issues

If you encounter installation issues:

1. **Upgrade pip**: `pip install --upgrade pip`
2. **Clear pip cache**: `pip cache purge`
3. **Use --no-cache-dir**: `pip install --no-cache-dir requestx`
4. **Check Python version**: `python --version` (must be 3.12+)

### Import Issues

If you get import errors:

```python
import sys
print(sys.path)

try:
    import requestx
    print("RequestX imported successfully")
except ImportError as e:
    print(f"Import error: {e}")
```

### Getting Help

If you need help with installation:

- **GitHub Issues**: [https://github.com/neuesql/requestx/issues](https://github.com/neuesql/requestx/issues)
- **Discussions**: [https://github.com/neuesql/requestx/discussions](https://github.com/neuesql/requestx/discussions)

When reporting issues, please include:

- Your operating system and version
- Python version (`python --version`)
- RequestX version (`pip show requestx`)
- Full error message and traceback
- Steps to reproduce the issue

## Uninstallation

To uninstall RequestX:

```bash
pip uninstall requestx
```
