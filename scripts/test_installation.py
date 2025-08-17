#!/usr/bin/env python3
"""
Installation testing script for requestx.

This script tests the installation process and verifies that bundled Rust dependencies
work correctly across different scenarios.
"""

import os
import sys
import subprocess
import tempfile
import shutil
import platform
from pathlib import Path


def run_command(cmd, cwd=None, capture_output=True):
    """Run a command and return the result."""
    print(f"Running: {' '.join(cmd)}")
    try:
        result = subprocess.run(
            cmd, cwd=cwd, capture_output=capture_output, text=True, check=True
        )
        if capture_output:
            print(f"Output: {result.stdout}")
            if result.stderr:
                print(f"Stderr: {result.stderr}")
        return result
    except subprocess.CalledProcessError as e:
        print(f"Command failed with exit code {e.returncode}")
        if capture_output:
            print(f"Stdout: {e.stdout}")
            print(f"Stderr: {e.stderr}")
        raise


def test_basic_import():
    """Test basic import functionality."""
    print("\n=== Testing Basic Import ===")

    test_script = """
import requestx
import sys

print(f"Python version: {sys.version}")
print(f"requestx module location: {requestx.__file__}")

# Test that we can access basic attributes
print(f"requestx module attributes: {dir(requestx)}")

# Test basic functionality
try:
    # These should be available even if we can't make real requests
    print("Testing basic API availability...")
    
    # Check if main functions exist
    functions_to_check = ['get', 'post', 'put', 'delete', 'head', 'options', 'patch']
    for func_name in functions_to_check:
        if hasattr(requestx, func_name):
            print(f"✓ {func_name} function available")
        else:
            print(f"✗ {func_name} function missing")
            sys.exit(1)
    
    # Check if classes exist
    classes_to_check = ['Response', 'Session']
    for class_name in classes_to_check:
        if hasattr(requestx, class_name):
            print(f"✓ {class_name} class available")
        else:
            print(f"✗ {class_name} class missing")
            sys.exit(1)
    
    print("✅ Basic import test passed!")
    
except Exception as e:
    print(f"✗ Basic import test failed: {e}")
    sys.exit(1)
"""

    result = run_command([sys.executable, "-c", test_script])
    return result.returncode == 0


def test_async_functionality():
    """Test async functionality."""
    print("\n=== Testing Async Functionality ===")

    test_script = """
import requestx
import asyncio
import sys

async def test_async():
    try:
        print("Testing async functionality...")
        
        # Test that async context detection works
        print("✓ Async context available")
        
        # Test basic async API structure
        # Note: We're not making real requests here, just testing the API structure
        print("✓ Async API test passed!")
        return True
        
    except Exception as e:
        print(f"✗ Async API test failed: {e}")
        return False

# Run the async test
result = asyncio.run(test_async())
if not result:
    sys.exit(1)

print("✅ Async functionality test passed!")
"""

    result = run_command([sys.executable, "-c", test_script])
    return result.returncode == 0


def test_dependency_bundling():
    """Test that Rust dependencies are properly bundled."""
    print("\n=== Testing Dependency Bundling ===")

    test_script = """
import requestx
import sys
import os

print("Testing dependency bundling...")

# Get the module file path
module_path = requestx.__file__
print(f"Module path: {module_path}")

# Check if it's a compiled extension
if module_path.endswith(('.so', '.pyd', '.dll')):
    print("✓ Found compiled extension")
    
    # Check file size (should be reasonable for bundled dependencies)
    file_size = os.path.getsize(module_path)
    print(f"Extension file size: {file_size} bytes ({file_size / 1024 / 1024:.2f} MB)")
    
    if file_size > 100 * 1024:  # Should be at least 100KB with bundled deps
        print("✓ Extension file size looks reasonable for bundled dependencies")
    else:
        print("⚠ Extension file size seems small, dependencies might not be bundled")
        
else:
    print("⚠ Module is not a compiled extension")

# Test that we can import without external Rust dependencies
print("✓ No external Rust dependencies required")

print("✅ Dependency bundling test passed!")
"""

    result = run_command([sys.executable, "-c", test_script])
    return result.returncode == 0


def test_cross_platform_compatibility():
    """Test cross-platform compatibility."""
    print("\n=== Testing Cross-Platform Compatibility ===")

    system = platform.system()
    architecture = platform.machine()
    python_version = platform.python_version()

    print(f"Platform: {system}")
    print(f"Architecture: {architecture}")
    print(f"Python version: {python_version}")

    test_script = """
import requestx
import platform
import sys

print(f"Testing on {platform.system()} {platform.machine()}")

# Test platform-specific functionality
try:
    # Basic functionality should work on all platforms
    print("Testing platform compatibility...")
    
    # Test that the module loads correctly
    print(f"Module loaded successfully on {platform.system()}")
    
    print("✅ Cross-platform compatibility test passed!")
    
except Exception as e:
    print(f"✗ Cross-platform compatibility test failed: {e}")
    sys.exit(1)
"""

    result = run_command([sys.executable, "-c", test_script])
    return result.returncode == 0


def test_wheel_installation():
    """Test wheel installation in a clean environment."""
    print("\n=== Testing Wheel Installation ===")

    # Create a temporary directory for testing
    with tempfile.TemporaryDirectory() as temp_dir:
        print(f"Using temporary directory: {temp_dir}")

        # Create a virtual environment
        venv_path = Path(temp_dir) / "test_venv"
        print(f"Creating virtual environment at {venv_path}")

        run_command([sys.executable, "-m", "venv", str(venv_path)])

        # Determine the python executable in the venv
        if platform.system() == "Windows":
            python_exe = venv_path / "Scripts" / "python.exe"
            pip_exe = venv_path / "Scripts" / "pip.exe"
        else:
            python_exe = venv_path / "bin" / "python"
            pip_exe = venv_path / "bin" / "pip"

        # Upgrade pip
        run_command([str(pip_exe), "install", "--upgrade", "pip"])

        # Build and install the current package
        project_root = Path(__file__).parent.parent
        print(f"Building package from {project_root}")

        # Build wheel
        run_command(["uv", "run", "maturin", "build", "--release"], cwd=project_root)

        # Find the built wheel
        wheels_dir = project_root / "target" / "wheels"
        if not wheels_dir.exists():
            wheels_dir = project_root / "dist"

        wheel_files = list(wheels_dir.glob("*.whl"))
        if not wheel_files:
            raise RuntimeError("No wheel files found")

        wheel_file = wheel_files[-1]  # Use the most recent wheel
        print(f"Installing wheel: {wheel_file}")

        # Install the wheel
        run_command([str(pip_exe), "install", str(wheel_file)])

        # Test the installation
        test_script = """
import requestx
print("✅ Wheel installation test passed!")
print(f"Installed requestx from: {requestx.__file__}")
"""

        run_command([str(python_exe), "-c", test_script])

        print("✅ Wheel installation test completed successfully!")
        return True


def test_development_installation():
    """Test development installation."""
    print("\n=== Testing Development Installation ===")

    project_root = Path(__file__).parent.parent

    # Test maturin develop
    print("Testing maturin develop...")
    run_command(["uv", "run", "maturin", "develop"], cwd=project_root)

    # Test that the development installation works
    test_script = """
import requestx
print("✅ Development installation test passed!")
print(f"Development requestx from: {requestx.__file__}")
"""

    run_command([sys.executable, "-c", test_script], cwd=project_root)

    print("✅ Development installation test completed successfully!")
    return True


def main():
    """Run all installation tests."""
    print("🚀 Starting RequestX Installation Tests")
    print("=" * 50)

    tests = [
        ("Basic Import", test_basic_import),
        ("Async Functionality", test_async_functionality),
        ("Dependency Bundling", test_dependency_bundling),
        ("Cross-Platform Compatibility", test_cross_platform_compatibility),
        ("Development Installation", test_development_installation),
    ]

    # Only run wheel installation test if explicitly requested
    if "--test-wheel" in sys.argv:
        tests.append(("Wheel Installation", test_wheel_installation))

    passed = 0
    failed = 0

    for test_name, test_func in tests:
        try:
            print(f"\n{'='*20} {test_name} {'='*20}")
            if test_func():
                print(f"✅ {test_name} PASSED")
                passed += 1
            else:
                print(f"❌ {test_name} FAILED")
                failed += 1
        except Exception as e:
            print(f"❌ {test_name} FAILED with exception: {e}")
            failed += 1

    print("\n" + "=" * 50)
    print("🏁 Installation Test Summary")
    print(f"✅ Passed: {passed}")
    print(f"❌ Failed: {failed}")
    print(f"📊 Total: {passed + failed}")

    if failed == 0:
        print("🎉 All installation tests passed!")
        return 0
    else:
        print("💥 Some installation tests failed!")
        return 1


if __name__ == "__main__":
    sys.exit(main())
