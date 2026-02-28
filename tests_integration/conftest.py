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
