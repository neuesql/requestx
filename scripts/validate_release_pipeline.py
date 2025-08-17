#!/usr/bin/env python3
"""
Validate the complete release pipeline setup.

This script checks that all components of the release pipeline are properly configured.
"""

import json
import subprocess
import sys
from pathlib import Path
from typing import Dict, List, Optional


def check_file_exists(file_path: str, description: str) -> bool:
    """Check if a file exists."""
    path = Path(file_path)
    if path.exists():
        print(f"✅ {description}: {file_path}")
        return True
    else:
        print(f"❌ {description} missing: {file_path}")
        return False


def check_workflow_files() -> bool:
    """Check that all required workflow files exist."""
    print("🔍 Checking GitHub Actions workflows...")

    workflows = [
        (".github/workflows/ci.yml", "CI Pipeline"),
        (".github/workflows/publish.yml", "Release Pipeline"),
        (".github/workflows/build-wheels.yml", "Wheel Building"),
        (".github/workflows/test.yml", "Testing"),
        (".github/workflows/test-release.yml", "Release Testing"),
    ]

    all_exist = True
    for file_path, description in workflows:
        if not check_file_exists(file_path, description):
            all_exist = False

    return all_exist


def check_script_files() -> bool:
    """Check that all required script files exist and are executable."""
    print("\n🔍 Checking release management scripts...")

    scripts = [
        ("scripts/release.py", "Release Management Script"),
        ("scripts/test_release.py", "Release Testing Script"),
        ("scripts/validate_release_pipeline.py", "Pipeline Validation Script"),
    ]

    all_exist = True
    for file_path, description in scripts:
        if check_file_exists(file_path, description):
            # Check if executable
            path = Path(file_path)
            if path.stat().st_mode & 0o111:
                print(f"  ✅ {file_path} is executable")
            else:
                print(f"  ⚠️  {file_path} is not executable (run: chmod +x {file_path})")
        else:
            all_exist = False

    return all_exist


def check_configuration_files() -> bool:
    """Check that configuration files are properly set up."""
    print("\n🔍 Checking configuration files...")

    configs = [
        ("pyproject.toml", "Python Package Configuration"),
        ("Cargo.toml", "Rust Package Configuration"),
        (".github/PUBLISHING.md", "Publishing Documentation"),
    ]

    all_exist = True
    for file_path, description in configs:
        if not check_file_exists(file_path, description):
            all_exist = False

    return all_exist


def check_version_consistency() -> bool:
    """Check that versions are consistent across configuration files."""
    print("\n🔍 Checking version consistency...")

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

        if pyproject_version == cargo_version:
            print(f"✅ Version consistency: {pyproject_version}")
            return True
        else:
            print(
                f"❌ Version mismatch: pyproject.toml ({pyproject_version}) != Cargo.toml ({cargo_version})"
            )
            return False

    except Exception as e:
        print(f"❌ Error checking version consistency: {e}")
        return False


def check_dependencies() -> bool:
    """Check that required dependencies are available."""
    print("\n🔍 Checking dependencies...")

    dependencies = [
        ("uv", "UV package manager"),
        ("cargo", "Rust package manager"),
        ("git", "Git version control"),
    ]

    all_available = True
    for cmd, description in dependencies:
        try:
            result = subprocess.run([cmd, "--version"], capture_output=True, text=True)
            if result.returncode == 0:
                version = result.stdout.strip().split("\n")[0]
                print(f"✅ {description}: {version}")
            else:
                print(f"❌ {description} not available")
                all_available = False
        except FileNotFoundError:
            print(f"❌ {description} not found")
            all_available = False

    return all_available


def check_git_setup() -> bool:
    """Check git repository setup."""
    print("\n🔍 Checking git repository setup...")

    try:
        # Check if we're in a git repository
        result = subprocess.run(
            ["git", "rev-parse", "--git-dir"], capture_output=True, text=True
        )
        if result.returncode != 0:
            print("❌ Not in a git repository")
            return False

        print("✅ Git repository detected")

        # Check for remote origin
        result = subprocess.run(
            ["git", "remote", "get-url", "origin"], capture_output=True, text=True
        )
        if result.returncode == 0:
            origin = result.stdout.strip()
            print(f"✅ Git remote origin: {origin}")
        else:
            print("⚠️  No git remote origin configured")

        # Check current branch
        result = subprocess.run(
            ["git", "branch", "--show-current"], capture_output=True, text=True
        )
        if result.returncode == 0:
            branch = result.stdout.strip()
            print(f"✅ Current branch: {branch}")

        return True

    except Exception as e:
        print(f"❌ Error checking git setup: {e}")
        return False


def check_build_system() -> bool:
    """Check that the build system works."""
    print("\n🔍 Checking build system...")

    try:
        # Check Rust compilation
        print("  Testing Rust compilation...")
        result = subprocess.run(["cargo", "check"], capture_output=True, text=True)
        if result.returncode == 0:
            print("  ✅ Rust compilation successful")
        else:
            print(f"  ❌ Rust compilation failed: {result.stderr}")
            return False

        # Check if maturin is available
        print("  Checking maturin availability...")
        result = subprocess.run(
            ["uv", "run", "maturin", "--version"], capture_output=True, text=True
        )
        if result.returncode == 0:
            version = result.stdout.strip()
            print(f"  ✅ Maturin available: {version}")
        else:
            print("  ❌ Maturin not available")
            return False

        return True

    except Exception as e:
        print(f"❌ Error checking build system: {e}")
        return False


def generate_setup_instructions(failed_checks: List[str]) -> None:
    """Generate setup instructions for failed checks."""
    if not failed_checks:
        return

    print("\n" + "=" * 60)
    print("🔧 SETUP INSTRUCTIONS")
    print("=" * 60)

    instructions = {
        "workflows": """
GitHub Actions Workflows:
- Ensure all workflow files are present in .github/workflows/
- Check that workflow syntax is valid
- Verify environment variables and secrets are configured
""",
        "scripts": """
Release Scripts:
- Ensure all scripts are present in scripts/ directory
- Make scripts executable: chmod +x scripts/*.py
- Test scripts locally before using in CI
""",
        "configuration": """
Configuration Files:
- Ensure pyproject.toml has correct package metadata
- Ensure Cargo.toml has correct Rust package configuration
- Update documentation as needed
""",
        "version": """
Version Consistency:
- Update version in both pyproject.toml and Cargo.toml
- Use the release script: python scripts/release.py bump patch
""",
        "dependencies": """
Dependencies:
- Install uv: curl -LsSf https://astral.sh/uv/install.sh | sh
- Install Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
- Ensure git is installed and configured
""",
        "git": """
Git Setup:
- Initialize git repository: git init
- Add remote origin: git remote add origin <repository-url>
- Configure git user: git config user.name "Your Name"
""",
        "build": """
Build System:
- Install dependencies: uv sync --dev
- Install maturin: uv tool install maturin[patchelf]
- Test build: uv run maturin develop
""",
    }

    for check in failed_checks:
        if check in instructions:
            print(instructions[check])


def main():
    """Main validation function."""
    print("🚀 Validating Release Pipeline Setup")
    print("=" * 60)

    checks = [
        ("workflows", check_workflow_files),
        ("scripts", check_script_files),
        ("configuration", check_configuration_files),
        ("version", check_version_consistency),
        ("dependencies", check_dependencies),
        ("git", check_git_setup),
        ("build", check_build_system),
    ]

    results = {}
    failed_checks = []

    for check_name, check_func in checks:
        try:
            results[check_name] = check_func()
            if not results[check_name]:
                failed_checks.append(check_name)
        except Exception as e:
            print(f"❌ {check_name} check failed with exception: {e}")
            results[check_name] = False
            failed_checks.append(check_name)

    # Summary
    print("\n" + "=" * 60)
    print("📊 VALIDATION SUMMARY")
    print("=" * 60)

    passed = sum(results.values())
    total = len(results)

    for check_name, result in results.items():
        status = "✅ PASS" if result else "❌ FAIL"
        print(f"{check_name.title():.<20} {status}")

    print("=" * 60)
    print(f"Results: {passed}/{total} checks passed")

    if passed == total:
        print("\n🎉 All checks passed! Release pipeline is ready.")
        print("\nNext steps:")
        print("1. Test the pipeline: python scripts/test_release.py")
        print("2. Create a test release: python scripts/release.py release 0.2.0-test")
        print("3. Monitor the GitHub Actions workflow")
        sys.exit(0)
    else:
        print(
            f"\n❌ {len(failed_checks)} checks failed. Please fix issues before using the release pipeline."
        )
        generate_setup_instructions(failed_checks)
        sys.exit(1)


if __name__ == "__main__":
    main()
