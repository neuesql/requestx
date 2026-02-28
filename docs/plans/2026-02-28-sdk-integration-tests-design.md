# SDK Integration Tests Design

**Date:** 2026-02-28
**Status:** Approved
**Author:** Design Session

## Overview

Add comprehensive integration tests that make real API calls to OpenAI and Anthropic services using requestx clients, verifying end-to-end compatibility with both SDKs. These tests complement the existing `test_sdk_compatibility.py` which only verifies isinstance checks.

## Goals

1. Verify requestx clients work with real OpenAI and Anthropic API calls
2. Test synchronous and asynchronous operations
3. Validate streaming responses work correctly
4. Ensure error handling propagates properly through the SDK layer
5. Provide confidence that requestx is a true drop-in replacement for httpx in AI SDK usage

## Non-Goals

- Testing SDK functionality (that's the SDK's responsibility)
- Comprehensive API coverage (focus on core operations that prove compatibility)
- Performance benchmarking (separate concern)
- Running in regular CI (too expensive, requires secrets)

## Architecture & File Structure

### Directory Structure

```
tests_integration/
├── __init__.py                      # Empty, marks as package
├── conftest.py                      # Shared pytest fixtures & config
├── utils.py                         # Shared validation helpers
├── test_openai_integration.py       # OpenAI SDK integration tests
└── test_anthropic_integration.py    # Anthropic SDK integration tests
```

### Environment Variables

Each SDK requires its own API key:
- `OPENAI_API_KEY` - Required for OpenAI tests
- `ANTHROPIC_API_KEY` - Required for Anthropic tests

Tests skip gracefully if their respective key is missing, allowing partial test runs (e.g., only OpenAI if ANTHROPIC_API_KEY is not set).

### Test Execution

```bash
# Run all integration tests
pytest tests_integration/ -v

# Run only OpenAI tests
pytest tests_integration/test_openai_integration.py -v

# Run only Anthropic tests
pytest tests_integration/test_anthropic_integration.py -v
```

### Conftest.py Responsibilities

- Check environment variables and create pytest fixtures for API keys
- Provide skip markers when keys are missing
- Configure pytest settings (timeouts, markers)
- Session-scoped fixtures to check env vars once per test session

## Test Coverage & Components

### Test Classes Structure

Each SDK test file will have four test classes:

#### 1. TestBasicChatCompletion
- `test_simple_chat_completion` - Basic request/response with requestx.Client
- `test_chat_with_system_message` - Multi-message conversation
- **Validates:** Response structure, content returned, status codes

#### 2. TestStreamingResponses
- `test_streaming_chat_completion` - Stream response chunks with requestx.Client
- `test_streaming_accumulation` - Verify complete message assembled from chunks
- **Validates:** Chunks received, final content matches, no data loss

#### 3. TestAsyncOperations
- `test_async_chat_completion` - Basic async request with requestx.AsyncClient
- `test_async_streaming` - Async streaming response
- **Validates:** Async/await patterns work, concurrent requests possible

#### 4. TestErrorHandling
- `test_invalid_api_key` - Should raise authentication error with clear message
- `test_timeout_handling` - Configure short timeout, verify timeout exception raised
- **Validates:** Proper error propagation, exception types match SDK expectations

### Shared Utilities (utils.py)

Helper functions both test files will use:
- `validate_chat_response(response, expected_model)` - Check response structure
- `collect_stream_chunks(stream)` - Gather streaming chunks into list
- `assert_valid_content(content)` - Verify content is non-empty string

### Test Characteristics

- Each test is independent (no shared state between tests)
- Tests use minimal tokens (simple prompts like "Say hello in one word")
- All tests have reasonable timeouts (30s default, 5s for timeout tests)
- Tests verify both requestx functionality AND SDK compatibility

## Implementation Details

### Conftest.py Implementation

**Fixtures:**
```python
@pytest.fixture(scope="session")
def openai_api_key():
    """Get OpenAI API key from environment or skip tests."""
    key = os.getenv("OPENAI_API_KEY")
    if not key:
        pytest.skip("OPENAI_API_KEY not set")
    return key

@pytest.fixture(scope="session")
def anthropic_api_key():
    """Get Anthropic API key from environment or skip tests."""
    key = os.getenv("ANTHROPIC_API_KEY")
    if not key:
        pytest.skip("ANTHROPIC_API_KEY not set")
    return key
```

**Configuration:**
- Default timeout: 30 seconds per test
- Pytest marker: `@pytest.mark.integration` (optional, for selective runs)
- Session-scoped fixtures (check env vars once, not per test)

### Model Configuration

**OpenAI:**
- Model: `gpt-4o`
- Simple prompts: `"Say hello in one word"`
- Max tokens: 10 (minimize cost)

**Anthropic:**
- Model: `claude-3-5-sonnet-20241022`
- Simple prompts: `"Say hello in one word"`
- Max tokens: 10 (minimize cost)

### Error Handling Test Implementation

**Invalid API Key Test:**
- Create client with `api_key="invalid-key-12345"`
- Attempt chat completion
- Verify exception type (AuthenticationError or similar)
- Check error message contains "authentication" or "API key"

**Timeout Test:**
- Create client with `timeout=0.001` (1ms - impossible to complete)
- Attempt chat completion
- Verify timeout exception raised
- Exception should be from requestx (proving requestx timeout handling works)

### Streaming Validation

Both SDKs have different streaming formats:
- **OpenAI**: Yields `ChatCompletionChunk` objects with `.choices[0].delta.content`
- **Anthropic**: Yields various event types, need to filter for `content_block_delta` events

Tests will validate:
- At least one chunk received
- Chunks can be accumulated into final message
- Final message is non-empty string

## Dependencies & Documentation

### Dependencies

Add to `pyproject.toml`:
```toml
[project.optional-dependencies]
integration = [
    "openai>=1.0.0",
    "anthropic>=0.18.0",
]
```

These are optional dependencies - don't force all users to install them. Users who want to run integration tests install with:
```bash
pip install -e ".[integration]"
```

### README.md Documentation

Add a new section titled **"Integration Tests"** with:

**Setup:**
- Export API keys as environment variables
- Install integration dependencies
- Run tests with pytest

**Example:**
```bash
# Set API keys
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."

# Install dependencies
pip install -e ".[integration]"

# Run integration tests
pytest tests_integration/ -v
```

**Notes:**
- Tests make real API calls and incur costs (minimal, ~$0.01 per full run)
- Tests skip gracefully if API keys not set
- Can run individual SDK tests separately

## Important Considerations

### Cost Control
- All prompts use minimal tokens (10 max_tokens)
- Estimated cost per full test run: < $0.01
- Tests designed to be run frequently without budget concerns

### CI/CD
- These tests should NOT run in regular CI (require secrets, cost money)
- Suitable for manual testing or nightly scheduled runs with secrets
- Regular CI continues to run `tests_requestx/` which are free and fast

### Test Stability
- Real API calls can be flaky (network, rate limits)
- Tests include retries where appropriate (not in timeout tests)
- Timeout tests use extremely short timeouts (0.001s) to guarantee failure

## Implementation Phases

1. **Phase 1: Infrastructure**
   - Create `tests_integration/` directory
   - Implement `conftest.py` with fixtures
   - Implement `utils.py` with shared helpers

2. **Phase 2: OpenAI Tests**
   - Implement all four test classes for OpenAI
   - Verify tests pass with real API key

3. **Phase 3: Anthropic Tests**
   - Implement all four test classes for Anthropic
   - Verify tests pass with real API key

4. **Phase 4: Documentation**
   - Update README.md with integration test section
   - Add optional dependencies to pyproject.toml

## Success Criteria

- All integration tests pass when both API keys are provided
- Tests skip gracefully when API keys are missing
- Cost per test run remains under $0.01
- Tests complete in under 60 seconds total
- Clear error messages when tests fail
- Documentation enables any contributor to run tests easily
