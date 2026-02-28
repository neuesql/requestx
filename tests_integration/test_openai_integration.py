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
