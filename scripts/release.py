#!/usr/bin/env python3
"""
Release management script for requestx.

This script helps manage version bumps, changelog generation, and release preparation.
"""

import argparse
import re
import subprocess
import sys
from pathlib import Path
from typing import List, Optional, Tuple


def run_command(cmd: List[str], cwd: Optional[Path] = None) -> Tuple[int, str, str]:
    """Run a command and return exit code, stdout, stderr."""
    result = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
    return result.returncode, result.stdout.strip(), result.stderr.strip()


def get_current_version() -> str:
    """Get current version from pyproject.toml."""
    pyproject_path = Path("pyproject.toml")
    if not pyproject_path.exists():
        raise FileNotFoundError("pyproject.toml not found")

    content = pyproject_path.read_text()
    match = re.search(r'^version = "([^"]+)"', content, re.MULTILINE)
    if not match:
        raise ValueError("Version not found in pyproject.toml")

    return match.group(1)


def update_version(new_version: str) -> None:
    """Update version in both pyproject.toml and Cargo.toml."""
    # Update pyproject.toml
    pyproject_path = Path("pyproject.toml")
    content = pyproject_path.read_text()
    content = re.sub(
        r'^version = "[^"]+"', f'version = "{new_version}"', content, flags=re.MULTILINE
    )
    pyproject_path.write_text(content)
    print(f"✅ Updated pyproject.toml to version {new_version}")

    # Update Cargo.toml
    cargo_path = Path("Cargo.toml")
    content = cargo_path.read_text()
    content = re.sub(
        r'^version = "[^"]+"',
        f'version = "{new_version}"',
        content,
        flags=re.MULTILINE,
        count=1,  # Only update the first occurrence (package version)
    )
    cargo_path.write_text(content)
    print(f"✅ Updated Cargo.toml to version {new_version}")


def bump_version(current: str, bump_type: str) -> str:
    """Bump version according to semantic versioning."""
    parts = current.split(".")
    if len(parts) != 3:
        raise ValueError(f"Invalid version format: {current}")

    major, minor, patch = map(int, parts)

    if bump_type == "major":
        return f"{major + 1}.0.0"
    elif bump_type == "minor":
        return f"{major}.{minor + 1}.0"
    elif bump_type == "patch":
        return f"{major}.{minor}.{patch + 1}"
    else:
        raise ValueError(f"Invalid bump type: {bump_type}")


def get_git_commits_since_tag(tag: Optional[str] = None) -> List[str]:
    """Get git commits since the specified tag (or all commits if no tag)."""
    if tag:
        cmd = ["git", "log", f"{tag}..HEAD", "--oneline"]
    else:
        cmd = ["git", "log", "--oneline"]

    exit_code, stdout, stderr = run_command(cmd)
    if exit_code != 0:
        print(f"Warning: Failed to get git commits: {stderr}")
        return []

    return stdout.split("\n") if stdout else []


def get_latest_tag() -> Optional[str]:
    """Get the latest git tag."""
    exit_code, stdout, stderr = run_command(["git", "describe", "--tags", "--abbrev=0"])
    if exit_code != 0:
        return None
    return stdout


def generate_changelog(version: str) -> str:
    """Generate changelog for the new version."""
    latest_tag = get_latest_tag()
    commits = get_git_commits_since_tag(latest_tag)

    changelog = f"## Version {version}\n\n"

    if latest_tag:
        changelog += f"### Changes since {latest_tag}\n\n"
    else:
        changelog += "### Initial Release\n\n"

    if commits:
        for commit in commits:
            if commit.strip():
                # Extract commit hash and message
                parts = commit.split(" ", 1)
                if len(parts) == 2:
                    hash_part, message = parts
                    changelog += f"- {message} ({hash_part})\n"
    else:
        changelog += "- No changes recorded\n"

    changelog += f"\n### Installation\n\n"
    changelog += f"```bash\n"
    changelog += f"pip install requestx=={version}\n"
    changelog += f"```\n"

    return changelog


def check_git_status() -> bool:
    """Check if git working directory is clean."""
    exit_code, stdout, stderr = run_command(["git", "status", "--porcelain"])
    return exit_code == 0 and not stdout


def create_git_tag(version: str, message: str) -> bool:
    """Create and push a git tag."""
    tag_name = f"v{version}"

    # Create tag
    exit_code, stdout, stderr = run_command(
        ["git", "tag", "-a", tag_name, "-m", message]
    )
    if exit_code != 0:
        print(f"❌ Failed to create tag: {stderr}")
        return False

    print(f"✅ Created tag {tag_name}")

    # Push tag
    exit_code, stdout, stderr = run_command(["git", "push", "origin", tag_name])
    if exit_code != 0:
        print(f"❌ Failed to push tag: {stderr}")
        return False

    print(f"✅ Pushed tag {tag_name}")
    return True


def main():
    parser = argparse.ArgumentParser(description="Release management for requestx")
    subparsers = parser.add_subparsers(dest="command", help="Available commands")

    # Version command
    version_parser = subparsers.add_parser("version", help="Show current version")

    # Bump command
    bump_parser = subparsers.add_parser("bump", help="Bump version")
    bump_parser.add_argument(
        "type", choices=["major", "minor", "patch"], help="Type of version bump"
    )
    bump_parser.add_argument(
        "--no-commit", action="store_true", help="Don't commit the version changes"
    )

    # Release command
    release_parser = subparsers.add_parser("release", help="Create a release")
    release_parser.add_argument("version", help="Version to release (e.g., 1.0.0)")
    release_parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Show what would be done without doing it",
    )

    # Changelog command
    changelog_parser = subparsers.add_parser("changelog", help="Generate changelog")
    changelog_parser.add_argument("version", help="Version for changelog")

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        return

    try:
        if args.command == "version":
            current = get_current_version()
            print(f"Current version: {current}")

        elif args.command == "bump":
            current = get_current_version()
            new_version = bump_version(current, args.type)

            print(f"Bumping version from {current} to {new_version}")

            if not check_git_status():
                print(
                    "❌ Git working directory is not clean. Please commit or stash changes."
                )
                sys.exit(1)

            update_version(new_version)

            if not args.no_commit:
                # Commit version changes
                run_command(["git", "add", "pyproject.toml", "Cargo.toml"])
                run_command(["git", "commit", "-m", f"Bump version to {new_version}"])
                print(f"✅ Committed version bump to {new_version}")

        elif args.command == "changelog":
            changelog = generate_changelog(args.version)
            print(changelog)

        elif args.command == "release":
            if not check_git_status():
                print(
                    "❌ Git working directory is not clean. Please commit or stash changes."
                )
                sys.exit(1)

            current = get_current_version()
            if current != args.version:
                print(
                    f"❌ Current version ({current}) doesn't match release version ({args.version})"
                )
                print(
                    "Run 'python scripts/release.py bump' first or update version manually"
                )
                sys.exit(1)

            changelog = generate_changelog(args.version)

            if args.dry_run:
                print("🔍 Dry run - would create release with:")
                print(f"Version: {args.version}")
                print("Changelog:")
                print(changelog)
                return

            print(f"Creating release for version {args.version}")
            print("Changelog:")
            print(changelog)

            confirm = input("\nProceed with release? (y/N): ")
            if confirm.lower() != "y":
                print("❌ Release cancelled")
                return

            # Create and push tag
            if create_git_tag(args.version, f"Release v{args.version}"):
                print(f"🎉 Release v{args.version} created successfully!")
                print(
                    "The GitHub Actions workflow will now build and publish the release."
                )
            else:
                print("❌ Failed to create release")
                sys.exit(1)

    except Exception as e:
        print(f"❌ Error: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
