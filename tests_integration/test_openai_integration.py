"""Integration tests for OpenAI SDK with RequestX."""

import json

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

MODEL = "gpt-4o-mini"


@pytest.mark.integration
class TestBasicChatCompletion:
    """Test basic chat completion with OpenAI SDK."""

    def test_simple_chat_completion(self, openai_api_key):
        """Test simple chat completion with requestx.Client."""
        http_client = requestx.Client()
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        response = client.chat.completions.create(
            model=MODEL,
            messages=[{"role": "user", "content": "Say hello in one word"}],
            max_tokens=10,
        )

        validate_chat_response(response, MODEL)
        content = response.choices[0].message.content
        assert_valid_content(content)

    def test_chat_with_system_message(self, openai_api_key):
        """Test chat with system message."""
        http_client = requestx.Client()
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        response = client.chat.completions.create(
            model=MODEL,
            messages=[
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "Say hello in one word"},
            ],
            max_tokens=10,
        )

        validate_chat_response(response, MODEL)
        content = response.choices[0].message.content
        assert_valid_content(content)


@pytest.mark.integration
class TestMultiTurnConversation:
    """Test multi-turn conversation with OpenAI SDK."""

    def test_multi_turn_conversation(self, openai_api_key):
        """Test multi-turn message array with assistant history."""
        http_client = requestx.Client()
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        response = client.chat.completions.create(
            model=MODEL,
            messages=[
                {"role": "user", "content": "My name is Alice."},
                {"role": "assistant", "content": "Nice to meet you, Alice!"},
                {"role": "user", "content": "What's my name?"},
            ],
            max_tokens=50,
        )

        validate_chat_response(response, MODEL)
        content = response.choices[0].message.content
        assert_valid_content(content)
        assert (
            "alice" in content.lower()
        ), "Response should contain the name from turn 1"


@pytest.mark.integration
class TestResponseModelProperties:
    """Test response model properties parsed through requestx."""

    def test_usage_property(self, openai_api_key):
        """Test that usage tokens are correctly parsed."""
        http_client = requestx.Client()
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        response = client.chat.completions.create(
            model=MODEL, messages=[{"role": "user", "content": "Hi"}], max_tokens=10
        )

        assert response.usage.prompt_tokens > 0, "prompt_tokens should be positive"
        assert (
            response.usage.completion_tokens > 0
        ), "completion_tokens should be positive"
        assert response.usage.total_tokens > 0, "total_tokens should be positive"
        assert isinstance(response.usage.prompt_tokens, int)
        assert isinstance(response.usage.completion_tokens, int)
        assert response.usage.total_tokens == (
            response.usage.prompt_tokens + response.usage.completion_tokens
        ), "total_tokens should equal prompt + completion"

    def test_response_id_property(self, openai_api_key):
        """Test that response ID is passed through correctly."""
        http_client = requestx.Client()
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        response = client.chat.completions.create(
            model=MODEL, messages=[{"role": "user", "content": "Hi"}], max_tokens=10
        )

        assert isinstance(response.id, str), "id should be a string"
        assert len(response.id) > 0, "id should not be empty"
        assert response.id.startswith(
            "chatcmpl-"
        ), f"id should start with 'chatcmpl-', got: {response.id}"

    def test_model_and_finish_reason(self, openai_api_key):
        """Test model name and finish_reason fields."""
        http_client = requestx.Client()
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        response = client.chat.completions.create(
            model=MODEL, messages=[{"role": "user", "content": "Hi"}], max_tokens=10
        )

        assert (
            "gpt" in response.model
        ), f"model should contain 'gpt', got: {response.model}"
        assert response.choices[0].finish_reason in (
            "stop",
            "length",
        ), f"Unexpected finish_reason: {response.choices[0].finish_reason}"

    def test_serialization_model_dump(self, openai_api_key):
        """Test response serialization to JSON and dict."""
        http_client = requestx.Client()
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        response = client.chat.completions.create(
            model=MODEL, messages=[{"role": "user", "content": "Hi"}], max_tokens=10
        )

        # model_dump_json() should return valid JSON string
        json_str = response.model_dump_json()
        parsed = json.loads(json_str)
        assert isinstance(parsed, dict)
        assert "id" in parsed
        assert "model" in parsed

        # model_dump() should return a dict with matching fields
        d = response.model_dump()
        assert isinstance(d, dict)
        assert d["id"] == response.id
        assert d["model"] == response.model


@pytest.mark.integration
class TestToolCalling:
    """Test function/tool calling with OpenAI SDK."""

    def test_tool_call_single_turn(self, openai_api_key):
        """Test single-turn tool call with tool_choice."""
        http_client = requestx.Client()
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        tools = [
            {
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Get the weather for a location.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "location": {
                                "type": "string",
                                "description": "City name",
                            }
                        },
                        "required": ["location"],
                    },
                },
            }
        ]

        response = client.chat.completions.create(
            model=MODEL,
            messages=[{"role": "user", "content": "What's the weather in Paris?"}],
            tools=tools,
            tool_choice={"type": "function", "function": {"name": "get_weather"}},
            max_tokens=100,
        )

        assert (
            response.choices[0].finish_reason == "stop"
        ), f"Expected finish_reason 'stop', got: {response.choices[0].finish_reason}"

        tool_calls = response.choices[0].message.tool_calls
        assert tool_calls is not None, "Expected tool_calls in response"
        assert len(tool_calls) > 0, "Expected at least one tool call"

        tool_call = tool_calls[0]
        assert tool_call.function.name == "get_weather"
        args = json.loads(tool_call.function.arguments)
        assert (
            "location" in args
        ), f"Tool arguments should contain 'location', got: {args}"

    def test_tool_call_multi_turn(self, openai_api_key):
        """Test multi-turn tool call with tool result follow-up."""
        http_client = requestx.Client()
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        tools = [
            {
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Get the weather for a location.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "location": {"type": "string", "description": "City name"}
                        },
                        "required": ["location"],
                    },
                },
            }
        ]

        # First turn: model calls the tool
        first_response = client.chat.completions.create(
            model=MODEL,
            messages=[{"role": "user", "content": "What's the weather in Tokyo?"}],
            tools=tools,
            tool_choice={"type": "function", "function": {"name": "get_weather"}},
            max_tokens=100,
        )

        tool_call = first_response.choices[0].message.tool_calls[0]

        # Second turn: send tool result back
        second_response = client.chat.completions.create(
            model=MODEL,
            messages=[
                {"role": "user", "content": "What's the weather in Tokyo?"},
                first_response.choices[0].message,
                {
                    "role": "tool",
                    "tool_call_id": tool_call.id,
                    "content": "Sunny, 25°C",
                },
            ],
            tools=tools,
            max_tokens=100,
        )

        assert (
            second_response.choices[0].finish_reason == "stop"
        ), f"Expected 'stop', got: {second_response.choices[0].finish_reason}"
        content = second_response.choices[0].message.content
        assert (
            content is not None and len(content) > 0
        ), "Expected text content in final response"


@pytest.mark.integration
class TestJSONMode:
    """Test JSON mode response format."""

    def test_json_mode_response(self, openai_api_key):
        """Test response_format={'type': 'json_object'}."""
        http_client = requestx.Client()
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        response = client.chat.completions.create(
            model=MODEL,
            messages=[
                {"role": "system", "content": "You output JSON."},
                {"role": "user", "content": 'Return {"greeting": "hello"} as JSON.'},
            ],
            response_format={"type": "json_object"},
            max_tokens=50,
        )

        content = response.choices[0].message.content
        assert_valid_content(content)

        parsed = json.loads(content)
        assert isinstance(parsed, dict), "JSON mode should return a JSON object"


@pytest.mark.integration
class TestEmbeddings:
    """Test embeddings API with OpenAI SDK."""

    def test_single_embedding(self, openai_api_key):
        """Test single text embedding."""
        http_client = requestx.Client()
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        response = client.embeddings.create(
            model="text-embedding-3-small",
            input="Hello world",
        )

        assert len(response.data) == 1, "Should have one embedding"
        embedding = response.data[0].embedding
        assert isinstance(embedding, list), "Embedding should be a list"
        assert len(embedding) > 0, "Embedding should not be empty"
        assert all(
            isinstance(x, float) for x in embedding[:10]
        ), "Embedding values should be floats"

    def test_batch_embeddings(self, openai_api_key):
        """Test batch text embeddings."""
        http_client = requestx.Client()
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        response = client.embeddings.create(
            model="text-embedding-3-small",
            input=["Hello", "World", "Test"],
        )

        assert len(response.data) == 3, "Should have three embeddings"
        for i, item in enumerate(response.data):
            assert item.index == i, f"Index mismatch at position {i}"
            assert len(item.embedding) > 0, f"Embedding {i} should not be empty"


@pytest.mark.integration
class TestModelsListing:
    """Test models API with OpenAI SDK."""

    def test_list_models(self, openai_api_key):
        """Test listing available models."""
        http_client = requestx.Client()
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        models = client.models.list()

        model_list = list(models)
        assert len(model_list) > 0, "Should have at least one model"
        # Verify model objects have expected fields
        first = model_list[0]
        assert hasattr(first, "id"), "Model should have 'id'"
        assert hasattr(first, "created"), "Model should have 'created'"

    def test_retrieve_model(self, openai_api_key):
        """Test retrieving a specific model."""
        http_client = requestx.Client()
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        model = client.models.retrieve(MODEL)

        assert model.id == MODEL, f"Expected model id '{MODEL}', got: {model.id}"
        assert hasattr(model, "created"), "Model should have 'created'"


@pytest.mark.integration
class TestStreamingResponses:
    """Test streaming responses with OpenAI SDK."""

    def test_streaming_chat_completion(self, openai_api_key):
        """Test streaming chat completion."""
        http_client = requestx.Client()
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        stream = client.chat.completions.create(
            model=MODEL,
            messages=[{"role": "user", "content": "Say hello in one word"}],
            max_tokens=10,
            stream=True,
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
            model=MODEL,
            messages=[{"role": "user", "content": "Count to three"}],
            max_tokens=10,
            stream=True,
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
                model=MODEL,
                messages=[{"role": "user", "content": "Say hello in one word"}],
                max_tokens=10,
            )

            validate_chat_response(response, MODEL)
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
                model=MODEL,
                messages=[{"role": "user", "content": "Say hello in one word"}],
                max_tokens=10,
                stream=True,
            )

            chunks = await collect_async_stream_chunks(stream)

            assert len(chunks) > 0, "Should receive at least one chunk"
            full_content = "".join(chunks)
            assert_valid_content(full_content)
        finally:
            await http_client.aclose()


@pytest.mark.integration
class TestContextManager:
    """Test client context manager patterns."""

    def test_sync_context_manager(self, openai_api_key):
        """Test sync client as context manager."""
        with OpenAI(
            api_key=openai_api_key,
            http_client=requestx.Client(),
        ) as client:
            response = client.chat.completions.create(
                model=MODEL,
                messages=[{"role": "user", "content": "Hi"}],
                max_tokens=10,
            )

        validate_chat_response(response, MODEL)
        assert_valid_content(response.choices[0].message.content)

    @pytest.mark.asyncio
    async def test_async_context_manager(self, openai_api_key):
        """Test async client as context manager."""
        async with AsyncOpenAI(
            api_key=openai_api_key,
            http_client=requestx.AsyncClient(),
        ) as client:
            response = await client.chat.completions.create(
                model=MODEL,
                messages=[{"role": "user", "content": "Hi"}],
                max_tokens=10,
            )

        validate_chat_response(response, MODEL)
        assert_valid_content(response.choices[0].message.content)


@pytest.mark.integration
class TestRawResponseAccess:
    """Test raw response access for headers and status."""

    def test_raw_response_headers(self, openai_api_key):
        """Test with_raw_response for status code and headers."""
        http_client = requestx.Client()
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        response = client.chat.completions.with_raw_response.create(
            model=MODEL,
            messages=[{"role": "user", "content": "Hi"}],
            max_tokens=10,
        )

        assert response.status_code == 200, f"Expected 200, got: {response.status_code}"
        assert response.headers is not None, "Headers should be accessible"
        # OpenAI returns request-id header
        request_id = response.headers.get("x-request-id")
        assert request_id is not None, "x-request-id header should be present"
        assert len(request_id) > 0

    def test_raw_response_parse(self, openai_api_key):
        """Test parsing raw response into a completion object."""
        http_client = requestx.Client()
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        raw = client.chat.completions.with_raw_response.create(
            model=MODEL,
            messages=[{"role": "user", "content": "Hi"}],
            max_tokens=10,
        )

        completion = raw.parse()
        assert hasattr(completion, "id"), "Parsed completion missing 'id'"
        assert len(completion.choices) > 0, "Parsed completion has no choices"
        assert_valid_content(completion.choices[0].message.content)


@pytest.mark.integration
class TestDefaultHeaders:
    """Test custom default headers."""

    def test_custom_default_headers(self, openai_api_key):
        """Test that custom default headers don't break requests."""
        http_client = requestx.Client()
        client = OpenAI(
            api_key=openai_api_key,
            http_client=http_client,
            default_headers={"X-Custom-Test": "requestx"},
        )

        response = client.chat.completions.create(
            model=MODEL,
            messages=[{"role": "user", "content": "Hi"}],
            max_tokens=10,
        )

        validate_chat_response(response, MODEL)
        assert_valid_content(response.choices[0].message.content)


@pytest.mark.integration
class TestTimeoutConfiguration:
    """Test timeout configuration patterns."""

    def test_granular_timeout_succeeds(self, openai_api_key):
        """Test that requestx.Timeout object works with OpenAI SDK."""
        http_client = requestx.Client()
        client = OpenAI(
            api_key=openai_api_key,
            http_client=http_client,
            timeout=requestx.Timeout(60.0, read=30.0, connect=10.0),
        )

        response = client.chat.completions.create(
            model=MODEL,
            messages=[{"role": "user", "content": "Hi"}],
            max_tokens=10,
        )

        validate_chat_response(response, MODEL)
        assert_valid_content(response.choices[0].message.content)

    def test_with_options_timeout_override(self, openai_api_key):
        """Test per-request timeout override via with_options."""
        http_client = requestx.Client()
        client = OpenAI(
            api_key=openai_api_key,
            http_client=http_client,
        )

        with pytest.raises(Exception) as exc_info:
            client.with_options(timeout=0.001).chat.completions.create(
                model=MODEL,
                messages=[{"role": "user", "content": "Hi"}],
                max_tokens=10,
            )

        error_msg = str(exc_info.value).lower()
        assert (
            "timeout" in error_msg
            or "timed out" in error_msg
            or "connection" in error_msg
        ), f"Expected timeout-related error, got: {error_msg}"


@pytest.mark.integration
class TestRetries:
    """Test retry behavior with OpenAI SDK."""

    def test_no_retries_on_auth_error(self):
        """Test that auth errors surface correctly with no retries."""
        http_client = requestx.Client()
        client = OpenAI(
            api_key="invalid-key-12345",
            http_client=http_client,
            max_retries=0,
        )

        with pytest.raises(AuthenticationError):
            client.chat.completions.create(
                model=MODEL,
                messages=[{"role": "user", "content": "Hi"}],
                max_tokens=10,
            )


@pytest.mark.integration
class TestParameterVariations:
    """Test various API parameter combinations."""

    def test_temperature_zero(self, openai_api_key):
        """Test temperature=0 for deterministic output."""
        http_client = requestx.Client()
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        response = client.chat.completions.create(
            model=MODEL,
            messages=[
                {"role": "user", "content": "What is 2+2? Reply with just the number."}
            ],
            max_tokens=10,
            temperature=0,
        )

        validate_chat_response(response, MODEL)
        content = response.choices[0].message.content
        assert_valid_content(content)
        assert "4" in content, f"Expected '4' in response, got: {content}"

    def test_max_tokens_truncation(self, openai_api_key):
        """Test that max_tokens=1 truncates output."""
        http_client = requestx.Client()
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        response = client.chat.completions.create(
            model=MODEL,
            messages=[
                {
                    "role": "user",
                    "content": "Write a long essay about the history of computing.",
                }
            ],
            max_tokens=1,
        )

        assert (
            response.choices[0].finish_reason == "length"
        ), f"Expected finish_reason 'length', got: {response.choices[0].finish_reason}"

    def test_multiple_choices(self, openai_api_key):
        """Test n=2 for multiple choices."""
        http_client = requestx.Client()
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        response = client.chat.completions.create(
            model=MODEL,
            messages=[{"role": "user", "content": "Say a random word"}],
            max_tokens=10,
            n=2,
        )

        assert (
            len(response.choices) == 2
        ), f"Expected 2 choices, got: {len(response.choices)}"
        for choice in response.choices:
            assert_valid_content(choice.message.content)


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
                model=MODEL,
                messages=[{"role": "user", "content": "Hello"}],
                max_tokens=10,
            )

        # Verify error message contains authentication-related text or status codes
        error_msg = str(exc_info.value).lower()
        assert any(
            keyword in error_msg
            for keyword in ["authentication", "api key", "401", "403", "forbidden"]
        )

    def test_timeout_handling(self, openai_api_key):
        """Test timeout handling."""
        # Create client with extremely short timeout
        http_client = requestx.Client(timeout=0.001)  # 1ms - impossible
        client = OpenAI(api_key=openai_api_key, http_client=http_client)

        # Should raise timeout exception
        with pytest.raises(Exception) as exc_info:
            client.chat.completions.create(
                model=MODEL,
                messages=[{"role": "user", "content": "Hello"}],
                max_tokens=10,
            )

        # Verify it's a timeout-related error
        error_msg = str(exc_info.value).lower()
        assert (
            "timeout" in error_msg
            or "timed out" in error_msg
            or "connection" in error_msg
        )
