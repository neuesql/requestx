#!/bin/bash
# Version Bump Script for Requestx
# Updates version in all 3 files: Cargo.toml, pyproject.toml, python/requestx/__init__.py
#
# Usage:
#   ./bump.sh 1.2.3        # Set specific version
#   ./bump.sh patch        # Bump patch (1.0.0 -> 1.0.1)
#   ./bump.sh minor        # Bump minor (1.0.0 -> 1.1.0)
#   ./bump.sh major        # Bump major (1.0.0 -> 2.0.0)
#   ./bump.sh              # Show current version

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# File paths (script is in project root)
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CARGO_TOML="$PROJECT_ROOT/Cargo.toml"
PYPROJECT_TOML="$PROJECT_ROOT/pyproject.toml"
INIT_PY="$PROJECT_ROOT/python/requestx/__init__.py"

# Get current version from pyproject.toml
get_current_version() {
    grep '^version = ' "$PYPROJECT_TOML" | head -1 | sed 's/version = "\(.*\)"/\1/'
}

# Bump version
bump_version() {
    local version=$1
    local bump_type=$2

    IFS='.' read -r major minor patch <<< "$version"

    case $bump_type in
        major)
            echo "$((major + 1)).0.0"
            ;;
        minor)
            echo "$major.$((minor + 1)).0"
            ;;
        patch)
            echo "$major.$minor.$((patch + 1))"
            ;;
        *)
            echo "$version"
            ;;
    esac
}

# Update version in a file
update_file() {
    local file=$1
    local old_version=$2
    local new_version=$3
    local pattern=$4

    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        sed -i '' "s/${pattern}\"${old_version}\"/${pattern}\"${new_version}\"/" "$file"
    else
        # Linux
        sed -i "s/${pattern}\"${old_version}\"/${pattern}\"${new_version}\"/" "$file"
    fi
}

# Verify all versions match
verify_versions() {
    local cargo_ver=$(grep '^version = ' "$CARGO_TOML" | head -1 | sed 's/version = "\(.*\)"/\1/')
    local pyproject_ver=$(grep '^version = ' "$PYPROJECT_TOML" | head -1 | sed 's/version = "\(.*\)"/\1/')
    local init_ver=$(grep '__version__ = ' "$INIT_PY" | sed 's/__version__ = "\(.*\)"/\1/')

    echo -e "${BLUE}Current versions:${NC}"
    echo "  Cargo.toml:     $cargo_ver"
    echo "  pyproject.toml: $pyproject_ver"
    echo "  __init__.py:    $init_ver"

    if [[ "$cargo_ver" == "$pyproject_ver" && "$cargo_ver" == "$init_ver" ]]; then
        echo -e "${GREEN}All versions in sync${NC}"
        return 0
    else
        echo -e "${RED}Version mismatch detected!${NC}"
        return 1
    fi
}

# Main
main() {
    local input=$1
    local current_version=$(get_current_version)

    # No argument - show current versions
    if [[ -z "$input" ]]; then
        verify_versions
        exit 0
    fi

    # Determine new version
    local new_version
    case $input in
        patch|minor|major)
            new_version=$(bump_version "$current_version" "$input")
            echo -e "${YELLOW}Bumping $input version: $current_version -> $new_version${NC}"
            ;;
        *)
            # Validate version format (x.y.z)
            if [[ ! "$input" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
                echo -e "${RED}Error: Invalid version format. Use x.y.z (e.g., 1.2.3)${NC}"
                exit 1
            fi
            new_version=$input
            echo -e "${YELLOW}Setting version: $current_version -> $new_version${NC}"
            ;;
    esac

    # Update all files
    echo -e "${BLUE}Updating files...${NC}"

    # Update Cargo.toml
    update_file "$CARGO_TOML" "$current_version" "$new_version" "version = "
    echo "  Updated Cargo.toml"

    # Update pyproject.toml
    update_file "$PYPROJECT_TOML" "$current_version" "$new_version" "version = "
    echo "  Updated pyproject.toml"

    # Update __init__.py
    update_file "$INIT_PY" "$current_version" "$new_version" "__version__ = "
    echo "  Updated python/requestx/__init__.py"

    # Verify
    echo ""
    verify_versions

    echo ""
    echo -e "${GREEN}Version updated to $new_version${NC}"
    echo ""
    echo "Next steps:"
    echo "  git add Cargo.toml pyproject.toml python/requestx/__init__.py"
    echo "  git commit -m \"chore: bump version to $new_version\""
    echo "  git tag v$new_version"
    echo "  git push origin main --tags"
}

main "$@"
