"""Integration tests for Anthropic SDK with RequestX."""

import pytest

# Skip entire module if anthropic not installed
pytest.importorskip("anthropic")

from anthropic import Anthropic, AsyncAnthropic, AuthenticationError
import requestx


@pytest.mark.integration
class TestBasicChatCompletion:
    """Test basic chat completion with Anthropic SDK."""

    def test_simple_chat_completion(self, anthropic_api_key):
        """Test simple message with requestx.Client."""
        http_client = requestx.Client()
        client = Anthropic(api_key=anthropic_api_key, http_client=http_client)

        response = client.messages.create(
            model="claude-sonnet-4-5-20250929",
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
            model="claude-sonnet-4-5-20250929",
            system="You are a helpful assistant.",
            messages=[{"role": "user", "content": "Say hello in one word"}],
            max_tokens=10
        )

        assert hasattr(response, "content"), "Response missing 'content'"
        assert len(response.content) > 0, "Response has no content"
        assert response.content[0].text, "Content has no text"


@pytest.mark.integration
class TestStreamingResponses:
    """Test streaming responses with Anthropic SDK."""

    def test_streaming_chat_completion(self, anthropic_api_key):
        """Test streaming message."""
        http_client = requestx.Client()
        client = Anthropic(api_key=anthropic_api_key, http_client=http_client)

        chunks = []
        with client.messages.stream(
            model="claude-sonnet-4-5-20250929",
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
            model="claude-sonnet-4-5-20250929",
            messages=[{"role": "user", "content": "Count to three"}],
            max_tokens=10
        ) as stream:
            for text in stream.text_stream:
                chunks.append(text)

        full_content = "".join(chunks)
        assert len(full_content) > 0, "Accumulated content must not be empty"
        assert len(chunks) >= 1, "Should receive chunks"


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
                model="claude-sonnet-4-5-20250929",
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
                model="claude-sonnet-4-5-20250929",
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


@pytest.mark.integration
class TestErrorHandling:
    """Test error handling with Anthropic SDK."""

    def test_invalid_api_key(self):
        """Test that invalid API key raises authentication error."""
        http_client = requestx.Client()
        client = Anthropic(api_key="invalid-key-12345", http_client=http_client)

        # Should raise either AuthenticationError (401) or HTTPStatusError (403)
        with pytest.raises(Exception) as exc_info:
            client.messages.create(
                model="claude-sonnet-4-5-20250929",
                messages=[{"role": "user", "content": "Hello"}],
                max_tokens=10
            )

        # Verify error message contains authentication-related text or status codes
        error_msg = str(exc_info.value).lower()
        assert any(keyword in error_msg for keyword in ["authentication", "api key", "401", "403", "forbidden"])

    def test_timeout_handling(self, anthropic_api_key):
        """Test timeout handling."""
        # Create client with extremely short timeout
        http_client = requestx.Client(timeout=0.001)  # 1ms
        client = Anthropic(api_key=anthropic_api_key, http_client=http_client)

        with pytest.raises(Exception) as exc_info:
            client.messages.create(
                model="claude-sonnet-4-5-20250929",
                messages=[{"role": "user", "content": "Hello"}],
                max_tokens=10
            )

        error_msg = str(exc_info.value).lower()
        assert "timeout" in error_msg or "timed out" in error_msg or "connection" in error_msg
