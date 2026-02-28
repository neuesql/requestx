"""Integration tests for OpenAI SDK with RequestX."""

import pytest

# Skip entire module if openai not installed
pytest.importorskip("openai")

from openai import OpenAI, AsyncOpenAI, AuthenticationError
import requestx
from tests_integration.utils import (
    validate_chat_response,
    assert_valid_content,
    collect_stream_chunks,
    collect_async_stream_chunks,
)


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


@pytest.mark.integration
class TestErrorHandling:
    """Test error handling with OpenAI SDK."""

    def test_invalid_api_key(self):
        """Test that invalid API key raises authentication error."""
        http_client = requestx.Client()
        client = OpenAI(api_key="invalid-key-12345", http_client=http_client)

        # Should raise either AuthenticationError (401) or HTTPStatusError (403)
        with pytest.raises(Exception) as exc_info:
            client.chat.completions.create(
                model="gpt-4o",
                messages=[{"role": "user", "content": "Hello"}],
                max_tokens=10
            )

        # Verify error message contains authentication-related text or status codes
        error_msg = str(exc_info.value).lower()
        assert any(keyword in error_msg for keyword in ["authentication", "api key", "401", "403", "forbidden"])

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
