"""Integration tests for Anthropic SDK with RequestX."""

import json

import pytest

# Skip entire module if anthropic not installed
pytest.importorskip("anthropic")

from anthropic import Anthropic, AsyncAnthropic, AuthenticationError
import requestx

MODEL = "claude-sonnet-4-5-20250929"


@pytest.mark.integration
class TestBasicChatCompletion:
    """Test basic chat completion with Anthropic SDK."""

    def test_simple_chat_completion(self, anthropic_api_key):
        """Test simple message with requestx.Client."""
        http_client = requestx.Client()
        client = Anthropic(api_key=anthropic_api_key, http_client=http_client)

        response = client.messages.create(
            model=MODEL,
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
            model=MODEL,
            system="You are a helpful assistant.",
            messages=[{"role": "user", "content": "Say hello in one word"}],
            max_tokens=10
        )

        assert hasattr(response, "content"), "Response missing 'content'"
        assert len(response.content) > 0, "Response has no content"
        assert response.content[0].text, "Content has no text"


@pytest.mark.integration
class TestMultiTurnConversation:
    """Test multi-turn conversation with Anthropic SDK."""

    def test_multi_turn_conversation(self, anthropic_api_key):
        """Test multi-turn message array with assistant prefill."""
        http_client = requestx.Client()
        client = Anthropic(api_key=anthropic_api_key, http_client=http_client)

        response = client.messages.create(
            model=MODEL,
            messages=[
                {"role": "user", "content": "My name is Alice."},
                {"role": "assistant", "content": "Nice to meet you, Alice!"},
                {"role": "user", "content": "What's my name?"},
            ],
            max_tokens=50
        )

        assert len(response.content) > 0, "Response has no content"
        assert "alice" in response.content[0].text.lower(), \
            "Response should contain the name from turn 1"


@pytest.mark.integration
class TestResponseModelProperties:
    """Test response model properties parsed through requestx."""

    def test_usage_property(self, anthropic_api_key):
        """Test that usage tokens are correctly parsed."""
        http_client = requestx.Client()
        client = Anthropic(api_key=anthropic_api_key, http_client=http_client)

        response = client.messages.create(
            model=MODEL,
            messages=[{"role": "user", "content": "Hi"}],
            max_tokens=10
        )

        assert response.usage.input_tokens > 0, "input_tokens should be positive"
        assert response.usage.output_tokens > 0, "output_tokens should be positive"
        assert isinstance(response.usage.input_tokens, int)
        assert isinstance(response.usage.output_tokens, int)

    def test_request_id_property(self, anthropic_api_key):
        """Test that request ID is passed through from response header."""
        http_client = requestx.Client()
        client = Anthropic(api_key=anthropic_api_key, http_client=http_client)

        response = client.messages.create(
            model=MODEL,
            messages=[{"role": "user", "content": "Hi"}],
            max_tokens=10
        )

        request_id = response._request_id
        assert isinstance(request_id, str), "request_id should be a string"
        assert len(request_id) > 0, "request_id should not be empty"
        assert request_id.startswith("req_"), \
            f"request_id should start with 'req_', got: {request_id}"

    def test_model_and_stop_reason(self, anthropic_api_key):
        """Test model name and stop reason fields."""
        http_client = requestx.Client()
        client = Anthropic(api_key=anthropic_api_key, http_client=http_client)

        response = client.messages.create(
            model=MODEL,
            messages=[{"role": "user", "content": "Hi"}],
            max_tokens=10
        )

        assert "claude" in response.model, \
            f"model should contain 'claude', got: {response.model}"
        assert response.stop_reason in ("end_turn", "max_tokens", "stop_sequence", "tool_use"), \
            f"Unexpected stop_reason: {response.stop_reason}"
        assert response.role == "assistant"

    def test_serialization_methods(self, anthropic_api_key):
        """Test response serialization to JSON and dict."""
        http_client = requestx.Client()
        client = Anthropic(api_key=anthropic_api_key, http_client=http_client)

        response = client.messages.create(
            model=MODEL,
            messages=[{"role": "user", "content": "Hi"}],
            max_tokens=10
        )

        # to_json() should return valid JSON string
        json_str = response.to_json()
        parsed = json.loads(json_str)
        assert isinstance(parsed, dict)

        # to_dict() should return a dict with matching fields
        d = response.to_dict()
        assert isinstance(d, dict)
        assert d["id"] == response.id
        assert d["model"] == response.model


@pytest.mark.integration
class TestTokenCounting:
    """Test token counting endpoint."""

    def test_count_tokens(self, anthropic_api_key):
        """Test basic token counting."""
        http_client = requestx.Client()
        client = Anthropic(api_key=anthropic_api_key, http_client=http_client)

        result = client.messages.count_tokens(
            model=MODEL,
            messages=[{"role": "user", "content": "Hello, world!"}],
        )

        assert isinstance(result.input_tokens, int)
        assert result.input_tokens > 0, "Token count should be positive"

    def test_count_tokens_with_system(self, anthropic_api_key):
        """Test that system prompt increases token count."""
        http_client = requestx.Client()
        client = Anthropic(api_key=anthropic_api_key, http_client=http_client)

        messages = [{"role": "user", "content": "Hello"}]

        without_system = client.messages.count_tokens(
            model=MODEL,
            messages=messages,
        )

        with_system = client.messages.count_tokens(
            model=MODEL,
            messages=messages,
            system="You are an extremely detailed and verbose assistant.",
        )

        assert with_system.input_tokens > without_system.input_tokens, \
            "Token count with system prompt should be higher"


@pytest.mark.integration
class TestStreamingResponses:
    """Test streaming responses with Anthropic SDK."""

    def test_streaming_chat_completion(self, anthropic_api_key):
        """Test streaming message."""
        http_client = requestx.Client()
        client = Anthropic(api_key=anthropic_api_key, http_client=http_client)

        chunks = []
        with client.messages.stream(
            model=MODEL,
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
            model=MODEL,
            messages=[{"role": "user", "content": "Count to three"}],
            max_tokens=10
        ) as stream:
            for text in stream.text_stream:
                chunks.append(text)

        full_content = "".join(chunks)
        assert len(full_content) > 0, "Accumulated content must not be empty"
        assert len(chunks) >= 1, "Should receive chunks"


@pytest.mark.integration
class TestRawStreamingEvents:
    """Test raw SSE streaming events."""

    def test_raw_sse_streaming(self, anthropic_api_key):
        """Test raw SSE event types from stream=True."""
        http_client = requestx.Client()
        client = Anthropic(api_key=anthropic_api_key, http_client=http_client)

        event_types = []
        with client.messages.create(
            model=MODEL,
            messages=[{"role": "user", "content": "Hi"}],
            max_tokens=10,
            stream=True,
        ) as stream:
            for event in stream:
                event_types.append(event.type)

        assert "message_start" in event_types, \
            f"Expected 'message_start' in events, got: {event_types}"
        assert "content_block_delta" in event_types, \
            f"Expected 'content_block_delta' in events, got: {event_types}"
        assert "message_stop" in event_types, \
            f"Expected 'message_stop' in events, got: {event_types}"


@pytest.mark.integration
class TestStreamingHelpersFinalMessage:
    """Test streaming helper get_final_message."""

    def test_get_final_message(self, anthropic_api_key):
        """Test get_final_message from streaming helper."""
        http_client = requestx.Client()
        client = Anthropic(api_key=anthropic_api_key, http_client=http_client)

        with client.messages.stream(
            model=MODEL,
            messages=[{"role": "user", "content": "Say hello"}],
            max_tokens=20,
        ) as stream:
            # Consume text_stream to drive the stream to completion
            for _ in stream.text_stream:
                pass
            final = stream.get_final_message()

        assert hasattr(final, "id"), "Final message missing 'id'"
        assert len(final.content) > 0, "Final message has no content"
        assert final.usage.output_tokens > 0, "Final message should have output tokens"

    @pytest.mark.asyncio
    async def test_async_get_final_message(self, anthropic_api_key):
        """Test async get_final_message from streaming helper."""
        http_client = requestx.AsyncClient()
        client = AsyncAnthropic(api_key=anthropic_api_key, http_client=http_client)

        try:
            async with client.messages.stream(
                model=MODEL,
                messages=[{"role": "user", "content": "Say hello"}],
                max_tokens=20,
            ) as stream:
                async for _ in stream.text_stream:
                    pass
                final = await stream.get_final_message()

            assert hasattr(final, "id"), "Final message missing 'id'"
            assert len(final.content) > 0, "Final message has no content"
            assert final.usage.output_tokens > 0, "Final message should have output tokens"
        finally:
            await http_client.aclose()


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
                model=MODEL,
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
                model=MODEL,
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
@pytest.mark.asyncio
class TestAsyncRawStreaming:
    """Test async raw SSE streaming."""

    async def test_async_raw_sse_streaming(self, anthropic_api_key):
        """Test async raw SSE event types with stream=True."""
        http_client = requestx.AsyncClient()
        client = AsyncAnthropic(api_key=anthropic_api_key, http_client=http_client)

        try:
            event_types = []
            async with await client.messages.create(
                model=MODEL,
                messages=[{"role": "user", "content": "Hi"}],
                max_tokens=10,
                stream=True,
            ) as stream:
                async for event in stream:
                    event_types.append(event.type)

            assert "message_start" in event_types, \
                f"Expected 'message_start' in events, got: {event_types}"
            assert "content_block_delta" in event_types, \
                f"Expected 'content_block_delta' in events, got: {event_types}"
            assert "message_stop" in event_types, \
                f"Expected 'message_stop' in events, got: {event_types}"
        finally:
            await http_client.aclose()


@pytest.mark.integration
class TestToolUse:
    """Test tool use with Anthropic SDK."""

    def test_tool_use_single_turn(self, anthropic_api_key):
        """Test single-turn tool use with tool_choice=any."""
        http_client = requestx.Client()
        client = Anthropic(api_key=anthropic_api_key, http_client=http_client)

        tools = [
            {
                "name": "get_weather",
                "description": "Get the weather for a location.",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "City name",
                        }
                    },
                    "required": ["location"],
                },
            }
        ]

        response = client.messages.create(
            model=MODEL,
            messages=[{"role": "user", "content": "What's the weather in Paris?"}],
            tools=tools,
            tool_choice={"type": "any"},
            max_tokens=200,
        )

        assert response.stop_reason == "tool_use", \
            f"Expected stop_reason 'tool_use', got: {response.stop_reason}"

        # Find the tool_use content block
        tool_use_blocks = [b for b in response.content if b.type == "tool_use"]
        assert len(tool_use_blocks) > 0, "Expected at least one tool_use block"

        tool_block = tool_use_blocks[0]
        assert tool_block.name == "get_weather"
        assert "location" in tool_block.input, \
            f"Tool input should contain 'location', got: {tool_block.input}"

    def test_tool_use_multi_turn(self, anthropic_api_key):
        """Test multi-turn tool use with tool_result follow-up."""
        http_client = requestx.Client()
        client = Anthropic(api_key=anthropic_api_key, http_client=http_client)

        tools = [
            {
                "name": "get_weather",
                "description": "Get the weather for a location.",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "location": {"type": "string", "description": "City name"}
                    },
                    "required": ["location"],
                },
            }
        ]

        # First turn: model calls the tool
        first_response = client.messages.create(
            model=MODEL,
            messages=[{"role": "user", "content": "What's the weather in Tokyo?"}],
            tools=tools,
            tool_choice={"type": "any"},
            max_tokens=200,
        )

        tool_use_block = next(b for b in first_response.content if b.type == "tool_use")

        # Second turn: send tool_result back
        second_response = client.messages.create(
            model=MODEL,
            messages=[
                {"role": "user", "content": "What's the weather in Tokyo?"},
                {"role": "assistant", "content": first_response.content},
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "tool_result",
                            "tool_use_id": tool_use_block.id,
                            "content": "Sunny, 25°C",
                        }
                    ],
                },
            ],
            tools=tools,
            max_tokens=200,
        )

        assert second_response.stop_reason == "end_turn", \
            f"Expected 'end_turn', got: {second_response.stop_reason}"
        text_blocks = [b for b in second_response.content if b.type == "text"]
        assert len(text_blocks) > 0, "Expected a text content block in final response"


@pytest.mark.integration
class TestContextManager:
    """Test client context manager patterns."""

    def test_sync_context_manager(self, anthropic_api_key):
        """Test sync client as context manager."""
        with Anthropic(
            api_key=anthropic_api_key,
            http_client=requestx.Client(),
        ) as client:
            response = client.messages.create(
                model=MODEL,
                messages=[{"role": "user", "content": "Hi"}],
                max_tokens=10,
            )

        assert len(response.content) > 0
        assert response.content[0].text

    @pytest.mark.asyncio
    async def test_async_context_manager(self, anthropic_api_key):
        """Test async client as context manager."""
        async with AsyncAnthropic(
            api_key=anthropic_api_key,
            http_client=requestx.AsyncClient(),
        ) as client:
            response = await client.messages.create(
                model=MODEL,
                messages=[{"role": "user", "content": "Hi"}],
                max_tokens=10,
            )

        assert len(response.content) > 0
        assert response.content[0].text


@pytest.mark.integration
class TestRawResponseAccess:
    """Test raw response access for headers and status."""

    def test_raw_response_headers(self, anthropic_api_key):
        """Test with_raw_response for status code and headers."""
        http_client = requestx.Client()
        client = Anthropic(api_key=anthropic_api_key, http_client=http_client)

        response = client.messages.with_raw_response.create(
            model=MODEL,
            messages=[{"role": "user", "content": "Hi"}],
            max_tokens=10,
        )

        assert response.status_code == 200, \
            f"Expected 200, got: {response.status_code}"
        assert response.headers is not None, "Headers should be accessible"
        assert isinstance(response.request_id, str)
        assert len(response.request_id) > 0

    def test_raw_response_parse(self, anthropic_api_key):
        """Test parsing raw response into a message object."""
        http_client = requestx.Client()
        client = Anthropic(api_key=anthropic_api_key, http_client=http_client)

        raw = client.messages.with_raw_response.create(
            model=MODEL,
            messages=[{"role": "user", "content": "Hi"}],
            max_tokens=10,
        )

        message = raw.parse()
        assert hasattr(message, "id"), "Parsed message missing 'id'"
        assert len(message.content) > 0, "Parsed message has no content"
        assert message.content[0].text, "Parsed message text is empty"


@pytest.mark.integration
class TestDefaultHeaders:
    """Test custom default headers."""

    def test_custom_default_headers(self, anthropic_api_key):
        """Test that custom default headers don't break requests."""
        http_client = requestx.Client()
        client = Anthropic(
            api_key=anthropic_api_key,
            http_client=http_client,
            default_headers={"X-Custom-Test": "requestx"},
        )

        response = client.messages.create(
            model=MODEL,
            messages=[{"role": "user", "content": "Hi"}],
            max_tokens=10,
        )

        assert len(response.content) > 0
        assert response.content[0].text


@pytest.mark.integration
class TestTimeoutConfiguration:
    """Test timeout configuration patterns."""

    def test_granular_timeout_succeeds(self, anthropic_api_key):
        """Test that requestx.Timeout object works with Anthropic SDK."""
        http_client = requestx.Client()
        client = Anthropic(
            api_key=anthropic_api_key,
            http_client=http_client,
            timeout=requestx.Timeout(60.0, read=30.0, connect=10.0),
        )

        response = client.messages.create(
            model=MODEL,
            messages=[{"role": "user", "content": "Hi"}],
            max_tokens=10,
        )

        assert len(response.content) > 0
        assert response.content[0].text

    def test_with_options_timeout_override(self, anthropic_api_key):
        """Test per-request timeout override via with_options."""
        http_client = requestx.Client()
        client = Anthropic(
            api_key=anthropic_api_key,
            http_client=http_client,
        )

        with pytest.raises(Exception) as exc_info:
            client.with_options(timeout=0.001).messages.create(
                model=MODEL,
                messages=[{"role": "user", "content": "Hi"}],
                max_tokens=10,
            )

        error_msg = str(exc_info.value).lower()
        assert "timeout" in error_msg or "timed out" in error_msg or "connection" in error_msg, \
            f"Expected timeout-related error, got: {error_msg}"


@pytest.mark.integration
class TestRetries:
    """Test retry behavior with Anthropic SDK."""

    def test_no_retries_on_auth_error(self):
        """Test that auth errors surface correctly with no retries."""
        http_client = requestx.Client()
        client = Anthropic(
            api_key="invalid-key-12345",
            http_client=http_client,
            max_retries=0,
        )

        with pytest.raises(AuthenticationError):
            client.messages.create(
                model=MODEL,
                messages=[{"role": "user", "content": "Hi"}],
                max_tokens=10,
            )


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
                model=MODEL,
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
                model=MODEL,
                messages=[{"role": "user", "content": "Hello"}],
                max_tokens=10
            )

        error_msg = str(exc_info.value).lower()
        assert "timeout" in error_msg or "timed out" in error_msg or "connection" in error_msg
