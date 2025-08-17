#!/usr/bin/env python3
"""
Test script for validating the complete release workflow.

This script tests the release pipeline end-to-end without actually publishing.
"""

import argparse
import json
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Dict, List, Optional, Tuple


def run_command(
    cmd: List[str], cwd: Optional[Path] = None, timeout: int = 300
) -> Tuple[int, str, str]:
    """Run a command and return exit code, stdout, stderr."""
    try:
        result = subprocess.run(
            cmd, cwd=cwd, capture_output=True, text=True, timeout=timeout
        )
        return result.returncode, result.stdout.strip(), result.stderr.strip()
    except subprocess.TimeoutExpired:
        return -1, "", f"Command timed out after {timeout} seconds"


def test_version_consistency() -> bool:
    """Test that versions are consistent across files."""
    print("🔍 Testing version consistency...")

    try:
        # Get version from pyproject.toml
        pyproject_path = Path("pyproject.toml")
        pyproject_content = pyproject_path.read_text()
        import re

        pyproject_match = re.search(
            r'^version = "([^"]+)"', pyproject_content, re.MULTILINE
        )
        if not pyproject_match:
            print("❌ Version not found in pyproject.toml")
            return False
        pyproject_version = pyproject_match.group(1)

        # Get version from Cargo.toml
        cargo_path = Path("Cargo.toml")
        cargo_content = cargo_path.read_text()
        cargo_match = re.search(r'^version = "([^"]+)"', cargo_content, re.MULTILINE)
        if not cargo_match:
            print("❌ Version not found in Cargo.toml")
            return False
        cargo_version = cargo_match.group(1)

        if pyproject_version != cargo_version:
            print(
                f"❌ Version mismatch: pyproject.toml ({pyproject_version}) != Cargo.toml ({cargo_version})"
            )
            return False

        print(f"✅ Version consistency check passed: {pyproject_version}")
        return True

    except Exception as e:
        print(f"❌ Version consistency check failed: {e}")
        return False


def test_build_system() -> bool:
    """Test that the build system works correctly."""
    print("🔧 Testing build system...")

    # Test Rust compilation
    print("  Testing Rust compilation...")
    exit_code, stdout, stderr = run_command(["cargo", "check"])
    if exit_code != 0:
        print(f"❌ Rust compilation failed: {stderr}")
        return False
    print("  ✅ Rust compilation successful")

    # Test Python extension build
    print("  Testing Python extension build...")
    exit_code, stdout, stderr = run_command(["uv", "run", "maturin", "develop"])
    if exit_code != 0:
        print(f"❌ Python extension build failed: {stderr}")
        return False
    print("  ✅ Python extension build successful")

    # Test import
    print("  Testing module import...")
    exit_code, stdout, stderr = run_command(
        [
            "uv",
            "run",
            "python",
            "-c",
            "import requestx; print(f'Import successful: {requestx.__file__}')",
        ]
    )
    if exit_code != 0:
        print(f"❌ Module import failed: {stderr}")
        return False
    print(f"  ✅ Module import successful")

    return True


def test_code_quality() -> bool:
    """Test code quality checks."""
    print("🧹 Testing code quality...")

    checks = [
        (["cargo", "fmt", "--check"], "Rust formatting"),
        (["cargo", "clippy", "--", "-D", "warnings"], "Rust linting"),
        (["uv", "run", "black", "--check", "."], "Python formatting"),
        (["uv", "run", "ruff", "check", "."], "Python linting"),
        (["uv", "run", "mypy", "."], "Python type checking"),
    ]

    for cmd, description in checks:
        print(f"  Testing {description}...")
        exit_code, stdout, stderr = run_command(cmd)
        if exit_code != 0:
            print(f"❌ {description} failed: {stderr}")
            return False
        print(f"  ✅ {description} passed")

    return True


def test_unit_tests() -> bool:
    """Test that unit tests pass."""
    print("🧪 Testing unit tests...")

    # Rust tests
    print("  Running Rust tests...")
    exit_code, stdout, stderr = run_command(["cargo", "test", "--verbose"])
    if exit_code != 0:
        print(f"❌ Rust tests failed: {stderr}")
        return False
    print("  ✅ Rust tests passed")

    # Python tests
    print("  Running Python tests...")
    exit_code, stdout, stderr = run_command(
        ["uv", "run", "python", "-m", "unittest", "discover", "tests/", "-v"]
    )
    if exit_code != 0:
        print(f"❌ Python tests failed: {stderr}")
        return False
    print("  ✅ Python tests passed")

    return True


def test_wheel_building() -> bool:
    """Test wheel building for current platform."""
    print("🎡 Testing wheel building...")

    with tempfile.TemporaryDirectory() as temp_dir:
        temp_path = Path(temp_dir)

        # Build wheel
        print("  Building wheel...")
        exit_code, stdout, stderr = run_command(
            [
                "uv",
                "run",
                "maturin",
                "build",
                "--release",
                "--strip",
                "--out",
                str(temp_path),
            ]
        )
        if exit_code != 0:
            print(f"❌ Wheel building failed: {stderr}")
            return False

        # Check wheel was created
        wheels = list(temp_path.glob("*.whl"))
        if not wheels:
            print("❌ No wheel file created")
            return False

        wheel_path = wheels[0]
        print(f"  ✅ Wheel built: {wheel_path.name}")

        # Test wheel installation in clean environment
        print("  Testing wheel installation...")
        with tempfile.TemporaryDirectory() as venv_dir:
            venv_path = Path(venv_dir)

            # Create virtual environment
            exit_code, stdout, stderr = run_command(
                ["python", "-m", "venv", str(venv_path / "test_env")]
            )
            if exit_code != 0:
                print(f"❌ Failed to create test environment: {stderr}")
                return False

            # Determine python executable path
            if sys.platform == "win32":
                python_exe = venv_path / "test_env" / "Scripts" / "python.exe"
            else:
                python_exe = venv_path / "test_env" / "bin" / "python"

            # Install wheel
            exit_code, stdout, stderr = run_command(
                [str(python_exe), "-m", "pip", "install", str(wheel_path)]
            )
            if exit_code != 0:
                print(f"❌ Wheel installation failed: {stderr}")
                return False

            # Test import in clean environment
            exit_code, stdout, stderr = run_command(
                [
                    str(python_exe),
                    "-c",
                    "import requestx; print('✅ Wheel installation test successful')",
                ]
            )
            if exit_code != 0:
                print(f"❌ Wheel import test failed: {stderr}")
                return False

            print("  ✅ Wheel installation test passed")

    return True


def test_sdist_building() -> bool:
    """Test source distribution building."""
    print("📦 Testing source distribution building...")

    with tempfile.TemporaryDirectory() as temp_dir:
        temp_path = Path(temp_dir)

        # Build sdist
        exit_code, stdout, stderr = run_command(
            ["uv", "run", "maturin", "sdist", "--out", str(temp_path)]
        )
        if exit_code != 0:
            print(f"❌ Source distribution building failed: {stderr}")
            return False

        # Check sdist was created
        sdists = list(temp_path.glob("*.tar.gz"))
        if not sdists:
            print("❌ No source distribution file created")
            return False

        sdist_path = sdists[0]
        print(f"  ✅ Source distribution built: {sdist_path.name}")

    return True


def test_github_actions_syntax() -> bool:
    """Test GitHub Actions workflow syntax."""
    print("⚙️  Testing GitHub Actions workflow syntax...")

    workflow_files = [
        ".github/workflows/ci.yml",
        ".github/workflows/publish.yml",
        ".github/workflows/build-wheels.yml",
        ".github/workflows/test.yml",
    ]

    for workflow_file in workflow_files:
        workflow_path = Path(workflow_file)
        if not workflow_path.exists():
            print(f"❌ Workflow file not found: {workflow_file}")
            return False

        # Basic YAML syntax check
        try:
            import yaml

            with open(workflow_path) as f:
                yaml.safe_load(f)
            print(f"  ✅ {workflow_file} syntax valid")
        except ImportError:
            print(
                f"  ⚠️  PyYAML not available, skipping syntax check for {workflow_file}"
            )
        except yaml.YAMLError as e:
            print(f"❌ {workflow_file} syntax error: {e}")
            return False

    return True


def test_release_script() -> bool:
    """Test the release management script."""
    print("📝 Testing release management script...")

    # Test version command
    exit_code, stdout, stderr = run_command(["python", "scripts/release.py", "version"])
    if exit_code != 0:
        print(f"❌ Release script version command failed: {stderr}")
        return False
    print(f"  ✅ Version command: {stdout}")

    # Test changelog generation
    exit_code, stdout, stderr = run_command(
        ["python", "scripts/release.py", "changelog", "1.0.0"]
    )
    if exit_code != 0:
        print(f"❌ Release script changelog command failed: {stderr}")
        return False
    print("  ✅ Changelog generation successful")

    return True


def run_all_tests() -> Dict[str, bool]:
    """Run all release tests."""
    tests = [
        ("Version Consistency", test_version_consistency),
        ("Build System", test_build_system),
        ("Code Quality", test_code_quality),
        ("Unit Tests", test_unit_tests),
        ("Wheel Building", test_wheel_building),
        ("Source Distribution", test_sdist_building),
        ("GitHub Actions Syntax", test_github_actions_syntax),
        ("Release Script", test_release_script),
    ]

    results = {}

    print("🚀 Starting release workflow validation...\n")

    for test_name, test_func in tests:
        print(f"{'='*60}")
        try:
            results[test_name] = test_func()
        except Exception as e:
            print(f"❌ {test_name} failed with exception: {e}")
            results[test_name] = False
        print()

    return results


def main():
    parser = argparse.ArgumentParser(description="Test release workflow")
    parser.add_argument(
        "--test",
        choices=[
            "version",
            "build",
            "quality",
            "tests",
            "wheel",
            "sdist",
            "actions",
            "script",
            "all",
        ],
        default="all",
        help="Specific test to run",
    )
    parser.add_argument(
        "--json", action="store_true", help="Output results in JSON format"
    )

    args = parser.parse_args()

    if args.test == "all":
        results = run_all_tests()
    else:
        test_map = {
            "version": test_version_consistency,
            "build": test_build_system,
            "quality": test_code_quality,
            "tests": test_unit_tests,
            "wheel": test_wheel_building,
            "sdist": test_sdist_building,
            "actions": test_github_actions_syntax,
            "script": test_release_script,
        }

        test_func = test_map[args.test]
        result = test_func()
        results = {args.test.title(): result}

    # Output results
    if args.json:
        print(json.dumps(results, indent=2))
    else:
        print("=" * 60)
        print("📊 RELEASE WORKFLOW TEST SUMMARY")
        print("=" * 60)

        passed = 0
        total = len(results)

        for test_name, result in results.items():
            status = "✅ PASS" if result else "❌ FAIL"
            print(f"{test_name:.<40} {status}")
            if result:
                passed += 1

        print("=" * 60)
        print(f"Results: {passed}/{total} tests passed")

        if passed == total:
            print("🎉 All tests passed! Release workflow is ready.")
            sys.exit(0)
        else:
            print("❌ Some tests failed. Please fix issues before releasing.")
            sys.exit(1)


if __name__ == "__main__":
    main()
