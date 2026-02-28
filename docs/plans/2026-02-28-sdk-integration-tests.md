# SDK Integration Tests Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add comprehensive integration tests that make real API calls to OpenAI and Anthropic services using requestx clients, verifying end-to-end SDK compatibility.

**Architecture:** Create `tests_integration/` directory with separate test files for OpenAI and Anthropic. Shared fixtures in conftest.py handle API key validation and test skipping. Real API calls with minimal tokens (~$0.01 per run).

**Tech Stack:** pytest, openai>=1.0.0, anthropic>=0.18.0, requestx

---

## Phase 1: Infrastructure Setup

### Task 1: Create integration tests directory structure

**Files:**
- Create: `tests_integration/__init__.py`
- Create: `tests_integration/conftest.py`
- Create: `tests_integration/utils.py`

**Step 1: Create empty package**

Create `tests_integration/__init__.py`:
```python
"""Integration tests for RequestX SDK compatibility.

These tests make real API calls to verify requestx works with OpenAI and Anthropic SDKs.
Requires OPENAI_API_KEY and/or ANTHROPIC_API_KEY environment variables.
"""
```

**Step 2: Create conftest with API key fixtures**

Create `tests_integration/conftest.py`:
```python
"""Pytest configuration for integration tests."""

import os
import pytest


@pytest.fixture(scope="session")
def openai_api_key():
    """Get OpenAI API key from environment or skip tests."""
    key = os.getenv("OPENAI_API_KEY")
    if not key:
        pytest.skip("OPENAI_API_KEY not set - skipping OpenAI integration tests")
    return key


@pytest.fixture(scope="session")
def anthropic_api_key():
    """Get Anthropic API key from environment or skip tests."""
    key = os.getenv("ANTHROPIC_API_KEY")
    if not key:
        pytest.skip("ANTHROPIC_API_KEY not set - skipping Anthropic integration tests")
    return key


# Configure pytest
def pytest_configure(config):
    """Register custom markers."""
    config.addinivalue_line(
        "markers", "integration: marks tests as integration tests (deselect with '-m \"not integration\"')"
    )
```

**Step 3: Create shared utilities**

Create `tests_integration/utils.py`:
```python
"""Shared utility functions for integration tests."""

from typing import Any, List


def validate_chat_response(response: Any, expected_model: str) -> None:
    """Validate chat completion response structure.

    Args:
        response: Chat completion response object
        expected_model: Model name to verify

    Raises:
        AssertionError: If response structure is invalid
    """
    assert hasattr(response, "id"), "Response missing 'id' field"
    assert hasattr(response, "model"), "Response missing 'model' field"
    assert hasattr(response, "choices"), "Response missing 'choices' field"
    assert len(response.choices) > 0, "Response has no choices"

    # Verify model (may have suffixes like -0125)
    assert response.model.startswith(expected_model.split("-")[0]), \
        f"Expected model {expected_model}, got {response.model}"


def collect_stream_chunks(stream) -> List[str]:
    """Collect streaming chunks into a list of content strings.

    Args:
        stream: Streaming response iterator

    Returns:
        List of content strings from chunks
    """
    chunks = []
    for chunk in stream:
        if hasattr(chunk, "choices") and len(chunk.choices) > 0:
            delta = chunk.choices[0].delta
            if hasattr(delta, "content") and delta.content:
                chunks.append(delta.content)
    return chunks


async def collect_async_stream_chunks(stream) -> List[str]:
    """Collect async streaming chunks into a list of content strings.

    Args:
        stream: Async streaming response iterator

    Returns:
        List of content strings from chunks
    """
    chunks = []
    async for chunk in stream:
        if hasattr(chunk, "choices") and len(chunk.choices) > 0:
            delta = chunk.choices[0].delta
            if hasattr(delta, "content") and delta.content:
                chunks.append(delta.content)
    return chunks


def assert_valid_content(content: str) -> None:
    """Verify content is a non-empty string.

    Args:
        content: Content to validate

    Raises:
        AssertionError: If content is invalid
    """
    assert isinstance(content, str), f"Content must be string, got {type(content)}"
    assert len(content) > 0, "Content must not be empty"
```

**Step 4: Commit infrastructure**

```bash
git add tests_integration/
git commit -m "feat: add integration tests infrastructure (conftest, utils)"
```

---

## Phase 2: OpenAI Integration Tests

### Task 2: OpenAI basic chat completion tests

**Files:**
- Create: `tests_integration/test_openai_integration.py`

**Step 1: Create test file with basic chat test**

Create `tests_integration/test_openai_integration.py`:
```python
"""Integration tests for OpenAI SDK with RequestX."""

import pytest

# Skip entire module if openai not installed
pytest.importorskip("openai")

from openai import OpenAI
import requestx
from tests_integration.utils import validate_chat_response, assert_valid_content


@pytest.mark.integration
class TestBasicChatCompletion:
    """Test basic chat completion with OpenAI SDK."""

    def test_simple_chat_completion(self, openai_api_key):
        """Test simple chat completion with requestx.Client."""
        # Create OpenAI client with requestx
        http_client = requestx.Client()
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        # Make simple request
        response = client.chat.completions.create(
            model="gpt-4o",
            messages=[{"role": "user", "content": "Say hello in one word"}],
            max_tokens=10
        )

        # Validate response
        validate_chat_response(response, "gpt-4o")
        content = response.choices[0].message.content
        assert_valid_content(content)

    def test_chat_with_system_message(self, openai_api_key):
        """Test chat with system message."""
        http_client = requestx.Client()
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        response = client.chat.completions.create(
            model="gpt-4o",
            messages=[
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "Say hello in one word"}
            ],
            max_tokens=10
        )

        validate_chat_response(response, "gpt-4o")
        content = response.choices[0].message.content
        assert_valid_content(content)
```

**Step 2: Run tests (should pass if API key available)**

Run: `pytest tests_integration/test_openai_integration.py::TestBasicChatCompletion -v`
Expected: PASS (if OPENAI_API_KEY set) or SKIP (if not set)

**Step 3: Commit basic chat tests**

```bash
git add tests_integration/test_openai_integration.py
git commit -m "test: add OpenAI basic chat completion tests"
```

### Task 3: OpenAI streaming tests

**Files:**
- Modify: `tests_integration/test_openai_integration.py`

**Step 1: Add streaming test class**

Add to `tests_integration/test_openai_integration.py`:
```python
from tests_integration.utils import collect_stream_chunks


@pytest.mark.integration
class TestStreamingResponses:
    """Test streaming responses with OpenAI SDK."""

    def test_streaming_chat_completion(self, openai_api_key):
        """Test streaming chat completion."""
        http_client = requestx.Client()
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        stream = client.chat.completions.create(
            model="gpt-4o",
            messages=[{"role": "user", "content": "Say hello in one word"}],
            max_tokens=10,
            stream=True
        )

        chunks = collect_stream_chunks(stream)

        # Verify we got chunks
        assert len(chunks) > 0, "Should receive at least one chunk"

        # Verify content is valid
        full_content = "".join(chunks)
        assert_valid_content(full_content)

    def test_streaming_accumulation(self, openai_api_key):
        """Test that streaming chunks accumulate to complete message."""
        http_client = requestx.Client()
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        stream = client.chat.completions.create(
            model="gpt-4o",
            messages=[{"role": "user", "content": "Count to three"}],
            max_tokens=10,
            stream=True
        )

        chunks = collect_stream_chunks(stream)
        full_content = "".join(chunks)

        # Verify accumulated content is coherent
        assert_valid_content(full_content)
        assert len(chunks) >= 1, "Should receive multiple chunks for counting"
```

**Step 2: Run streaming tests**

Run: `pytest tests_integration/test_openai_integration.py::TestStreamingResponses -v`
Expected: PASS

**Step 3: Commit streaming tests**

```bash
git add tests_integration/test_openai_integration.py
git commit -m "test: add OpenAI streaming response tests"
```

### Task 4: OpenAI async tests

**Files:**
- Modify: `tests_integration/test_openai_integration.py`

**Step 1: Add async test class**

Add to `tests_integration/test_openai_integration.py`:
```python
from openai import AsyncOpenAI
from tests_integration.utils import collect_async_stream_chunks


@pytest.mark.integration
@pytest.mark.asyncio
class TestAsyncOperations:
    """Test async operations with OpenAI SDK."""

    async def test_async_chat_completion(self, openai_api_key):
        """Test async chat completion."""
        http_client = requestx.AsyncClient()
        client = AsyncOpenAI(api_key=openai_api_key, http_client=http_client)

        try:
            response = await client.chat.completions.create(
                model="gpt-4o",
                messages=[{"role": "user", "content": "Say hello in one word"}],
                max_tokens=10
            )

            validate_chat_response(response, "gpt-4o")
            content = response.choices[0].message.content
            assert_valid_content(content)
        finally:
            await http_client.aclose()

    async def test_async_streaming(self, openai_api_key):
        """Test async streaming."""
        http_client = requestx.AsyncClient()
        client = AsyncOpenAI(api_key=openai_api_key, http_client=http_client)

        try:
            stream = await client.chat.completions.create(
                model="gpt-4o",
                messages=[{"role": "user", "content": "Say hello in one word"}],
                max_tokens=10,
                stream=True
            )

            chunks = await collect_async_stream_chunks(stream)

            assert len(chunks) > 0, "Should receive at least one chunk"
            full_content = "".join(chunks)
            assert_valid_content(full_content)
        finally:
            await http_client.aclose()
```

**Step 2: Run async tests**

Run: `pytest tests_integration/test_openai_integration.py::TestAsyncOperations -v`
Expected: PASS

**Step 3: Commit async tests**

```bash
git add tests_integration/test_openai_integration.py
git commit -m "test: add OpenAI async operation tests"
```

### Task 5: OpenAI error handling tests

**Files:**
- Modify: `tests_integration/test_openai_integration.py`

**Step 1: Add error handling test class**

Add to `tests_integration/test_openai_integration.py`:
```python
from openai import AuthenticationError
import requestx


@pytest.mark.integration
class TestErrorHandling:
    """Test error handling with OpenAI SDK."""

    def test_invalid_api_key(self):
        """Test that invalid API key raises authentication error."""
        http_client = requestx.Client()
        client = OpenAI(api_key="invalid-key-12345", http_client=http_client)

        with pytest.raises(AuthenticationError) as exc_info:
            client.chat.completions.create(
                model="gpt-4o",
                messages=[{"role": "user", "content": "Hello"}],
                max_tokens=10
            )

        # Verify error message contains authentication-related text
        error_msg = str(exc_info.value).lower()
        assert "authentication" in error_msg or "api key" in error_msg or "401" in error_msg

    def test_timeout_handling(self, openai_api_key):
        """Test timeout handling."""
        # Create client with extremely short timeout
        http_client = requestx.Client(timeout=0.001)  # 1ms - impossible
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        # Should raise timeout exception
        with pytest.raises(Exception) as exc_info:
            client.chat.completions.create(
                model="gpt-4o",
                messages=[{"role": "user", "content": "Hello"}],
                max_tokens=10
            )

        # Verify it's a timeout-related error
        error_msg = str(exc_info.value).lower()
        assert "timeout" in error_msg or "timed out" in error_msg
```

**Step 2: Run error handling tests**

Run: `pytest tests_integration/test_openai_integration.py::TestErrorHandling -v`
Expected: PASS

**Step 3: Commit error handling tests**

```bash
git add tests_integration/test_openai_integration.py
git commit -m "test: add OpenAI error handling tests"
```

---

## Phase 3: Anthropic Integration Tests

### Task 6: Anthropic basic chat completion tests

**Files:**
- Create: `tests_integration/test_anthropic_integration.py`

**Step 1: Create test file with basic tests**

Create `tests_integration/test_anthropic_integration.py`:
```python
"""Integration tests for Anthropic SDK with RequestX."""

import pytest

# Skip entire module if anthropic not installed
pytest.importorskip("anthropic")

from anthropic import Anthropic
import requestx


@pytest.mark.integration
class TestBasicChatCompletion:
    """Test basic chat completion with Anthropic SDK."""

    def test_simple_chat_completion(self, anthropic_api_key):
        """Test simple message with requestx.Client."""
        http_client = requestx.Client()
        client = Anthropic(api_key=anthropic_api_key, http_client=http_client)

        response = client.messages.create(
            model="claude-3-5-sonnet-20241022",
            messages=[{"role": "user", "content": "Say hello in one word"}],
            max_tokens=10
        )

        # Validate response structure
        assert hasattr(response, "id"), "Response missing 'id'"
        assert hasattr(response, "model"), "Response missing 'model'"
        assert hasattr(response, "content"), "Response missing 'content'"
        assert len(response.content) > 0, "Response has no content"
        assert response.content[0].text, "First content block has no text"
        assert len(response.content[0].text) > 0, "Content text is empty"

    def test_chat_with_system_message(self, anthropic_api_key):
        """Test message with system prompt."""
        http_client = requestx.Client()
        client = Anthropic(api_key=anthropic_api_key, http_client=http_client)

        response = client.messages.create(
            model="claude-3-5-sonnet-20241022",
            system="You are a helpful assistant.",
            messages=[{"role": "user", "content": "Say hello in one word"}],
            max_tokens=10
        )

        assert hasattr(response, "content"), "Response missing 'content'"
        assert len(response.content) > 0, "Response has no content"
        assert response.content[0].text, "Content has no text"
```

**Step 2: Run basic tests**

Run: `pytest tests_integration/test_anthropic_integration.py::TestBasicChatCompletion -v`
Expected: PASS (if ANTHROPIC_API_KEY set) or SKIP (if not set)

**Step 3: Commit basic Anthropic tests**

```bash
git add tests_integration/test_anthropic_integration.py
git commit -m "test: add Anthropic basic chat completion tests"
```

### Task 7: Anthropic streaming tests

**Files:**
- Modify: `tests_integration/test_anthropic_integration.py`

**Step 1: Add streaming test class**

Add to `tests_integration/test_anthropic_integration.py`:
```python
@pytest.mark.integration
class TestStreamingResponses:
    """Test streaming responses with Anthropic SDK."""

    def test_streaming_chat_completion(self, anthropic_api_key):
        """Test streaming message."""
        http_client = requestx.Client()
        client = Anthropic(api_key=anthropic_api_key, http_client=http_client)

        chunks = []
        with client.messages.stream(
            model="claude-3-5-sonnet-20241022",
            messages=[{"role": "user", "content": "Say hello in one word"}],
            max_tokens=10
        ) as stream:
            for text in stream.text_stream:
                chunks.append(text)

        # Verify we got chunks
        assert len(chunks) > 0, "Should receive at least one chunk"

        # Verify content is valid
        full_content = "".join(chunks)
        assert isinstance(full_content, str), "Content must be string"
        assert len(full_content) > 0, "Content must not be empty"

    def test_streaming_accumulation(self, anthropic_api_key):
        """Test that streaming chunks accumulate to complete message."""
        http_client = requestx.Client()
        client = Anthropic(api_key=anthropic_api_key, http_client=http_client)

        chunks = []
        with client.messages.stream(
            model="claude-3-5-sonnet-20241022",
            messages=[{"role": "user", "content": "Count to three"}],
            max_tokens=10
        ) as stream:
            for text in stream.text_stream:
                chunks.append(text)

        full_content = "".join(chunks)
        assert len(full_content) > 0, "Accumulated content must not be empty"
        assert len(chunks) >= 1, "Should receive chunks"
```

**Step 2: Run streaming tests**

Run: `pytest tests_integration/test_anthropic_integration.py::TestStreamingResponses -v`
Expected: PASS

**Step 3: Commit streaming tests**

```bash
git add tests_integration/test_anthropic_integration.py
git commit -m "test: add Anthropic streaming response tests"
```

### Task 8: Anthropic async tests

**Files:**
- Modify: `tests_integration/test_anthropic_integration.py`

**Step 1: Add async test class**

Add to `tests_integration/test_anthropic_integration.py`:
```python
from anthropic import AsyncAnthropic


@pytest.mark.integration
@pytest.mark.asyncio
class TestAsyncOperations:
    """Test async operations with Anthropic SDK."""

    async def test_async_chat_completion(self, anthropic_api_key):
        """Test async message."""
        http_client = requestx.AsyncClient()
        client = AsyncAnthropic(api_key=anthropic_api_key, http_client=http_client)

        try:
            response = await client.messages.create(
                model="claude-3-5-sonnet-20241022",
                messages=[{"role": "user", "content": "Say hello in one word"}],
                max_tokens=10
            )

            assert hasattr(response, "content"), "Response missing 'content'"
            assert len(response.content) > 0, "Response has no content"
            assert response.content[0].text, "Content has no text"
        finally:
            await http_client.aclose()

    async def test_async_streaming(self, anthropic_api_key):
        """Test async streaming."""
        http_client = requestx.AsyncClient()
        client = AsyncAnthropic(api_key=anthropic_api_key, http_client=http_client)

        try:
            chunks = []
            async with client.messages.stream(
                model="claude-3-5-sonnet-20241022",
                messages=[{"role": "user", "content": "Say hello in one word"}],
                max_tokens=10
            ) as stream:
                async for text in stream.text_stream:
                    chunks.append(text)

            assert len(chunks) > 0, "Should receive at least one chunk"
            full_content = "".join(chunks)
            assert len(full_content) > 0, "Content must not be empty"
        finally:
            await http_client.aclose()
```

**Step 2: Run async tests**

Run: `pytest tests_integration/test_anthropic_integration.py::TestAsyncOperations -v`
Expected: PASS

**Step 3: Commit async tests**

```bash
git add tests_integration/test_anthropic_integration.py
git commit -m "test: add Anthropic async operation tests"
```

### Task 9: Anthropic error handling tests

**Files:**
- Modify: `tests_integration/test_anthropic_integration.py`

**Step 1: Add error handling test class**

Add to `tests_integration/test_anthropic_integration.py`:
```python
from anthropic import AuthenticationError


@pytest.mark.integration
class TestErrorHandling:
    """Test error handling with Anthropic SDK."""

    def test_invalid_api_key(self):
        """Test that invalid API key raises authentication error."""
        http_client = requestx.Client()
        client = Anthropic(api_key="invalid-key-12345", http_client=http_client)

        with pytest.raises(AuthenticationError) as exc_info:
            client.messages.create(
                model="claude-3-5-sonnet-20241022",
                messages=[{"role": "user", "content": "Hello"}],
                max_tokens=10
            )

        # Verify error message
        error_msg = str(exc_info.value).lower()
        assert "authentication" in error_msg or "api key" in error_msg or "401" in error_msg

    def test_timeout_handling(self, anthropic_api_key):
        """Test timeout handling."""
        # Create client with extremely short timeout
        http_client = requestx.Client(timeout=0.001)  # 1ms
        client = Anthropic(api_key=anthropic_api_key, http_client=http_client)

        with pytest.raises(Exception) as exc_info:
            client.messages.create(
                model="claude-3-5-sonnet-20241022",
                messages=[{"role": "user", "content": "Hello"}],
                max_tokens=10
            )

        error_msg = str(exc_info.value).lower()
        assert "timeout" in error_msg or "timed out" in error_msg
```

**Step 2: Run error handling tests**

Run: `pytest tests_integration/test_anthropic_integration.py::TestErrorHandling -v`
Expected: PASS

**Step 3: Commit error handling tests**

```bash
git add tests_integration/test_anthropic_integration.py
git commit -m "test: add Anthropic error handling tests"
```

---

## Phase 4: Documentation & Dependencies

### Task 10: Add optional dependencies

**Files:**
- Modify: `pyproject.toml`

**Step 1: Add integration dependencies to pyproject.toml**

Find the `[project.optional-dependencies]` section and add:
```toml
[project.optional-dependencies]
integration = [
    "openai>=1.0.0",
    "anthropic>=0.18.0",
]
```

If the section doesn't exist, add it after the `[project]` section.

**Step 2: Test installation**

Run: `pip install -e ".[integration]"`
Expected: Successfully installs openai and anthropic packages

**Step 3: Commit dependency changes**

```bash
git add pyproject.toml
git commit -m "build: add optional integration test dependencies"
```

### Task 11: Update README with integration tests section

**Files:**
- Modify: `README.md`

**Step 1: Add integration tests section to README**

Find an appropriate location (after "Quick Commands" or testing section) and add:

```markdown
## Integration Tests

RequestX includes integration tests that verify compatibility with real AI SDK APIs (OpenAI and Anthropic). These tests make actual API calls and require API keys.

### Setup

1. Install integration dependencies:
```bash
pip install -e ".[integration]"
```

2. Set environment variables:
```bash
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
```

### Running Integration Tests

```bash
# Run all integration tests
pytest tests_integration/ -v

# Run only OpenAI tests
pytest tests_integration/test_openai_integration.py -v

# Run only Anthropic tests
pytest tests_integration/test_anthropic_integration.py -v
```

### Important Notes

- **Cost**: Tests make real API calls and incur costs (~$0.01 per full run)
- **API Keys**: Tests skip gracefully if API keys are not set
- **CI/CD**: These tests should NOT run in regular CI (require secrets, cost money)
- Tests use minimal tokens (max_tokens=10) to minimize costs
```

**Step 2: Commit README changes**

```bash
git add README.md
git commit -m "docs: add integration tests section to README"
```

---

## Verification & Final Steps

### Task 12: Run full test suite

**Step 1: Run all integration tests with API keys**

```bash
# Set both API keys
export OPENAI_API_KEY="your-key"
export ANTHROPIC_API_KEY="your-key"

# Run all integration tests
pytest tests_integration/ -v
```

Expected: All tests PASS

**Step 2: Run without API keys to verify skipping**

```bash
# Unset keys
unset OPENAI_API_KEY
unset ANTHROPIC_API_KEY

# Run tests
pytest tests_integration/ -v
```

Expected: All tests SKIPPED with clear messages

**Step 3: Run regular tests to ensure nothing broke**

```bash
pytest tests_requestx/ -v
```

Expected: 1413 passed, 1 skipped

### Task 13: Final commit and summary

**Step 1: Review all changes**

```bash
git log --oneline -10
git diff feature/v6-ai-client-compatiblity...HEAD --stat
```

**Step 2: Create summary commit if needed**

If you made any fixes during verification, commit them:
```bash
git add .
git commit -m "test: finalize SDK integration tests"
```

**Step 3: Document completion**

Create completion message summarizing:
- Number of tests added (16 total: 8 OpenAI + 8 Anthropic)
- Test coverage (basic, streaming, async, error handling)
- Instructions for running tests
- Estimated cost per run (~$0.01)

---

## Success Criteria

✅ Integration tests directory created with proper structure
✅ 8 OpenAI integration tests (basic, streaming, async, errors)
✅ 8 Anthropic integration tests (basic, streaming, async, errors)
✅ Shared utilities and fixtures implemented
✅ Tests skip gracefully when API keys missing
✅ Optional dependencies configured in pyproject.toml
✅ README documentation added
✅ All tests pass with real API keys
✅ Regular test suite still passes (1413 tests)
✅ Cost per run under $0.01
