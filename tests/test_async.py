"""Tests for asynchronous HTTP client functionality."""

import pytest
import asyncio
from requestx import AsyncClient, Headers, Auth


class TestAsyncClient:
    """Test AsyncClient class."""

    @pytest.mark.asyncio
    async def test_async_get(self):
        """Test async GET request."""
        async with AsyncClient() as client:
            response = await client.get("https://httpbin.org/get")
            assert response.status_code == 200
            data = response.json()
            assert "url" in data

    @pytest.mark.asyncio
    async def test_async_post_json(self):
        """Test async POST request with JSON."""
        async with AsyncClient() as client:
            response = await client.post(
                "https://httpbin.org/post", json={"key": "value"}
            )
            assert response.status_code == 200
            data = response.json()
            assert data["json"] == {"key": "value"}

    @pytest.mark.asyncio
    async def test_async_post_form(self):
        """Test async POST request with form data."""
        async with AsyncClient() as client:
            response = await client.post(
                "https://httpbin.org/post", data={"field": "value"}
            )
            assert response.status_code == 200
            data = response.json()
            assert data["form"]["field"] == "value"

    @pytest.mark.asyncio
    async def test_async_custom_headers(self):
        """Test async request with custom headers."""
        async with AsyncClient() as client:
            response = await client.get(
                "https://httpbin.org/headers", headers={"X-Test-Header": "test-value"}
            )
            assert response.status_code == 200
            data = response.json()
            assert data["headers"]["X-Test-Header"] == "test-value"

    @pytest.mark.asyncio
    async def test_async_query_params(self):
        """Test async request with query parameters."""
        async with AsyncClient() as client:
            response = await client.get(
                "https://httpbin.org/get", params={"key": "value"}
            )
            assert response.status_code == 200
            data = response.json()
            assert data["args"]["key"] == "value"

    @pytest.mark.asyncio
    async def test_async_base_url(self):
        """Test async client with base URL."""
        async with AsyncClient(base_url="https://httpbin.org") as client:
            response = await client.get("/get")
            assert response.status_code == 200

    @pytest.mark.asyncio
    async def test_async_multiple_concurrent_requests(self):
        """Test multiple concurrent async requests."""
        async with AsyncClient() as client:
            tasks = [
                client.get("https://httpbin.org/get"),
                client.get("https://httpbin.org/get"),
                client.get("https://httpbin.org/get"),
            ]
            responses = await asyncio.gather(*tasks)

            for response in responses:
                assert response.status_code == 200

    @pytest.mark.asyncio
    async def test_async_put(self):
        """Test async PUT request."""
        async with AsyncClient() as client:
            response = await client.put(
                "https://httpbin.org/put", json={"updated": True}
            )
            assert response.status_code == 200

    @pytest.mark.asyncio
    async def test_async_patch(self):
        """Test async PATCH request."""
        async with AsyncClient() as client:
            response = await client.patch(
                "https://httpbin.org/patch", json={"patched": True}
            )
            assert response.status_code == 200

    @pytest.mark.asyncio
    async def test_async_delete(self):
        """Test async DELETE request."""
        async with AsyncClient() as client:
            response = await client.delete("https://httpbin.org/delete")
            assert response.status_code == 200

    @pytest.mark.asyncio
    async def test_async_head(self):
        """Test async HEAD request."""
        async with AsyncClient() as client:
            response = await client.head("https://httpbin.org/get")
            assert response.status_code == 200

    @pytest.mark.asyncio
    async def test_async_options(self):
        """Test async OPTIONS request."""
        async with AsyncClient() as client:
            response = await client.options("https://httpbin.org/get")
            assert response.status_code == 200

    @pytest.mark.asyncio
    async def test_async_basic_auth(self):
        """Test async request with basic auth."""
        async with AsyncClient() as client:
            response = await client.get(
                "https://httpbin.org/basic-auth/user/pass",
                auth=Auth.basic("user", "pass"),
            )
            assert response.status_code == 200

    @pytest.mark.asyncio
    async def test_async_bearer_auth(self):
        """Test async request with bearer auth."""
        async with AsyncClient() as client:
            response = await client.get(
                "https://httpbin.org/bearer", auth=Auth.bearer("test-token")
            )
            assert response.status_code == 200

    @pytest.mark.asyncio
    async def test_async_default_headers(self):
        """Test async client with default headers."""
        async with AsyncClient(headers={"X-Default": "value"}) as client:
            response = await client.get("https://httpbin.org/headers")
            data = response.json()
            assert data["headers"]["X-Default"] == "value"

    @pytest.mark.asyncio
    async def test_async_timeout(self):
        """Test async request with timeout."""
        async with AsyncClient() as client:
            # Short timeout should work for fast requests
            response = await client.get("https://httpbin.org/get", timeout=30.0)
            assert response.status_code == 200

    @pytest.mark.asyncio
    async def test_async_response_attributes(self):
        """Test async response attributes."""
        async with AsyncClient() as client:
            response = await client.get("https://httpbin.org/get")

            assert isinstance(response.status_code, int)
            assert isinstance(response.url, str)
            assert isinstance(response.headers, Headers)
            assert response.is_success
            assert not response.is_error

    @pytest.mark.asyncio
    async def test_async_response_json(self):
        """Test async JSON response parsing."""
        async with AsyncClient() as client:
            response = await client.get("https://httpbin.org/json")
            data = response.json()
            assert isinstance(data, dict)

    @pytest.mark.asyncio
    async def test_async_response_text(self):
        """Test async text response."""
        async with AsyncClient() as client:
            response = await client.get("https://httpbin.org/html")
            text = response.text
            assert isinstance(text, str)
            assert "html" in text.lower()

    @pytest.mark.asyncio
    async def test_async_error_response(self):
        """Test async error response handling."""
        async with AsyncClient() as client:
            response = await client.get("https://httpbin.org/status/404")
            assert response.status_code == 404
            assert response.is_client_error
            assert not response.is_success


class TestAsyncClientPerformance:
    """Performance-related tests for AsyncClient."""

    @pytest.mark.asyncio
    async def test_many_concurrent_requests(self):
        """Test handling many concurrent requests."""
        async with AsyncClient() as client:
            # Create 10 concurrent requests
            tasks = [client.get(f"https://httpbin.org/get?id={i}") for i in range(10)]
            responses = await asyncio.gather(*tasks)

            assert len(responses) == 10
            for i, response in enumerate(responses):
                assert response.status_code == 200
                data = response.json()
                assert data["args"]["id"] == str(i)

    @pytest.mark.asyncio
    async def test_reuse_client(self):
        """Test that client can be reused for multiple requests."""
        async with AsyncClient() as client:
            for i in range(5):
                response = await client.get("https://httpbin.org/get")
                assert response.status_code == 200


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
