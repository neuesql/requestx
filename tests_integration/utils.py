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
