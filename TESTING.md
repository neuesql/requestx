# RequestX Testing Guide

This document describes the testing structure for RequestX, organized according to the CI/CD pipeline defined in `tech.md`.

## Test Organization

The tests have been reorganized from a single `test_core_client.py` file into a structured test suite that follows the CI/CD pipeline stages:

```
tests/
├── __init__.py              # Test package documentation
├── test_unit.py             # Stage 4: Python Unit Tests
├── test_integration.py      # Stage 5: Integration Tests  
├── test_performance.py      # Stage 6: Performance Tests
└── README.md               # Detailed test documentation
```

## CI/CD Pipeline Stages

### Stage 4: Python Unit Tests
**File:** `tests/test_unit.py`

Core functionality tests using Python's built-in unittest framework:

- **Module Import Tests**: Verify all functions and classes are available
- **HTTP Method Tests**: Test all HTTP methods (GET, POST, PUT, DELETE, HEAD, OPTIONS, PATCH)
- **Response Object Tests**: Verify Response object properties and methods
- **Error Handling Tests**: Test exception handling and error conversion

**Command:**
```bash
uv run python -m unittest tests.test_unit -v
```

### Stage 5: Integration Tests
**File:** `tests/test_integration.py`

Requests library compatibility and real-world usage tests:

- **API Compatibility**: Verify drop-in replacement capability
- **Response Compatibility**: Test Response object matches requests.Response
- **Session Compatibility**: Test Session object matches requests.Session
- **Async/Sync Behavior**: Verify synchronous execution patterns

**Command:**
```bash
uv run python -m unittest tests.test_integration -v
```



## Running Tests

### Individual Test Suites
```bash
# Unit tests only
uv run python -m unittest tests.test_unit -v

# Integration tests only  
uv run python -m unittest tests.test_integration -v

# Performance tests only
uv run python -m unittest tests.test_performance -v
```

### All Tests (Discovery)
```bash
# Run all tests using unittest discovery
uv run python -m unittest discover tests/ -v
```

### Pipeline Script
```bash
# Run complete CI/CD pipeline test sequence
bash scripts/test_pipeline.sh
```

## Test Framework

Following tech.md guidelines:
- **Framework**: Python's built-in `unittest` (no external dependencies)
- **Test Discovery**: `python -m unittest discover tests/ -v`
- **Specific Tests**: `python -m unittest tests.test_module -v`
- **Live Testing**: Uses httpbin.org for real HTTP requests
- **No pytest**: Explicitly using unittest as specified in tech.md

## Test Coverage

The test suite covers all implemented functionality from Task 2:

### ✅ Core HTTP Client Foundation
- [x] RequestxClient struct with hyper::Client and hyper-tls integration
- [x] Async HTTP method functions (get, post, put, delete, head, options, patch)
- [x] Error handling with RequestxError enum and Python exception conversion
- [x] Unit tests for core HTTP functionality

### ✅ Python API Integration
- [x] All HTTP method functions exposed to Python
- [x] Response object with requests-compatible API
- [x] Session object creation (placeholder for future tasks)
- [x] Error handling and exception conversion

### ✅ Requirements Validation
- [x] Requirements 1.1: HTTP method support
- [x] Requirements 3.1: Error handling
- [x] Requirements 6.4: Response object functionality
- [x] Requirements 7.2: Python integration

## Test Results

Current test status (as of Task 2 completion):
- **Unit Tests**: 16 tests - Core functionality ✅
- **Integration Tests**: 9 tests - Requests compatibility ✅
- **Performance Tests**: 8 tests - Performance validation ✅

**Total**: 33 tests covering all implemented functionality

## Network Dependencies

Some tests require internet connectivity:
- Uses httpbin.org for live HTTP testing
- May occasionally fail due to network issues (502 errors)
- Core functionality tests (imports, object creation) work offline

## Future Test Expansion

As new tasks are implemented, tests should be added to appropriate categories:
- **Unit Tests**: New core functionality
- **Integration Tests**: Enhanced requests compatibility
- **Performance Tests**: New performance features

The test structure is designed to scale with the project while maintaining the CI/CD pipeline organization.