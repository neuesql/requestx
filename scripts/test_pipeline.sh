#!/bin/bash
# CI/CD Pipeline Test Script following tech.md guidelines

set -e  # Exit on any error

echo "========================================"
echo "RequestX CI/CD Pipeline Test Execution"
echo "========================================"

# Stage 4: Python Unit Tests (following tech.md pipeline order)
echo ""
echo "Stage 4: Python Unit Tests"
echo "----------------------------------------"
echo "Running Python tests using unittest..."
uv run python -m unittest tests.test_unit -v

echo ""
echo "Running specific test modules..."
uv run python -m unittest tests.test_unit -v

# Stage 5: Integration Tests  
echo ""
echo "Stage 5: Integration Tests"
echo "----------------------------------------"
echo "Testing requests compatibility..."
uv run python -m unittest tests.test_integration -v

echo "Testing async/sync behavior..."
uv run python -m unittest tests.test_integration -v



# Run all tests using unittest discovery
echo ""
echo "Running Complete Test Suite"
echo "----------------------------------------"
cd tests && uv run python -m unittest discover -s . -v

echo ""
echo "========================================"
echo "Pipeline Execution Complete"
echo "========================================"