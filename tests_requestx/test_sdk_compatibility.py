"""
SDK Compatibility Tests (TDD - These should FAIL until patch is implemented)

Tests that requestx.Client and requestx.AsyncClient can pass isinstance checks
for httpx.Client and httpx.AsyncClient, enabling compatibility with AI SDKs
like OpenAI and Anthropic.

Task 1: Write failing tests (this file)
Task 2: Implement the patch to make tests pass
"""

import pytest
import httpx
import requestx


class TestInstanceCheckCompatibility:
    """Test that requestx clients pass httpx isinstance checks."""

    def test_requestx_client_passes_httpx_isinstance_check(self):
        """requestx.Client should pass isinstance(client, httpx.Client) check."""
        client = requestx.Client()
        assert isinstance(
            client, httpx.Client
        ), "requestx.Client must pass isinstance check for httpx.Client"

    def test_requestx_async_client_passes_httpx_isinstance_check(self):
        """requestx.AsyncClient should pass isinstance(client, httpx.AsyncClient) check."""
        client = requestx.AsyncClient()
        assert isinstance(
            client, httpx.AsyncClient
        ), "requestx.AsyncClient must pass isinstance check for httpx.AsyncClient"

    def test_httpx_client_still_works(self):
        """Regression: real httpx.Client should still pass isinstance check."""
        client = httpx.Client()
        assert isinstance(
            client, httpx.Client
        ), "Real httpx.Client must still work after patching"


class TestOpenAISDKCompatibility:
    """Test compatibility with OpenAI SDK."""

    def test_openai_sdk_accepts_requestx_client(self):
        """OpenAI SDK should accept requestx.Client as http_client parameter."""
        pytest.importorskip("openai")
        from openai import OpenAI

        client = requestx.Client()

        # OpenAI SDK checks isinstance(http_client, httpx.Client)
        # This should not raise TypeError
        try:
            OpenAI(api_key="test-key", http_client=client)
        except TypeError as e:
            pytest.fail(f"OpenAI SDK rejected requestx.Client: {e}")

    def test_openai_async_sdk_accepts_requestx_async_client(self):
        """OpenAI AsyncOpenAI should accept requestx.AsyncClient as http_client parameter."""
        pytest.importorskip("openai")
        from openai import AsyncOpenAI

        client = requestx.AsyncClient()

        # OpenAI SDK checks isinstance(http_client, httpx.AsyncClient)
        # This should not raise TypeError
        try:
            AsyncOpenAI(api_key="test-key", http_client=client)
        except TypeError as e:
            pytest.fail(f"AsyncOpenAI SDK rejected requestx.AsyncClient: {e}")


class TestAnthropicSDKCompatibility:
    """Test compatibility with Anthropic SDK."""

    def test_anthropic_sdk_accepts_requestx_client(self):
        """Anthropic SDK should accept requestx.Client as http_client parameter."""
        pytest.importorskip("anthropic")
        from anthropic import Anthropic

        client = requestx.Client()

        # Anthropic SDK checks isinstance(http_client, httpx.Client)
        # This should not raise TypeError
        try:
            Anthropic(api_key="test-key", http_client=client)
        except TypeError as e:
            pytest.fail(f"Anthropic SDK rejected requestx.Client: {e}")

    def test_anthropic_async_sdk_accepts_requestx_async_client(self):
        """Anthropic AsyncAnthropic should accept requestx.AsyncClient as http_client parameter."""
        pytest.importorskip("anthropic")
        from anthropic import AsyncAnthropic

        client = requestx.AsyncClient()

        # Anthropic SDK checks isinstance(http_client, httpx.AsyncClient)
        # This should not raise TypeError
        try:
            AsyncAnthropic(api_key="test-key", http_client=client)
        except TypeError as e:
            pytest.fail(f"AsyncAnthropic SDK rejected requestx.AsyncClient: {e}")
