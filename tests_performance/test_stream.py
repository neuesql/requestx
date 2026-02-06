import unittest

from http_benchmark.clients.aiohttp_adapter import AiohttpAdapter
from http_benchmark.clients.requestx_adapter import RequestXAdapter
from http_benchmark.models.http_request import HTTPRequest


class TestAiohttpStreaming(unittest.IsolatedAsyncioTestCase):
    """Test async streaming functionality for AiohttpAdapter."""

    async def asyncSetUp(self):
        self.adapter = AiohttpAdapter()
        await self.adapter.__aenter__()
        self.request = HTTPRequest(
            method="GET",
            url="https://httpbin.org/stream/2",
            stream=True,
            timeout=30,
        )

    async def asyncTearDown(self):
        await self.adapter.__aexit__(None, None, None)

    async def test_aiohttp_stream_async_request(self):
        """Test async streaming request with aiohttp adapter."""
        result = await self.adapter.make_request_stream_async(self.request)

        self.assertTrue(result["success"])
        self.assertEqual(result["status_code"], 200)
        self.assertTrue(result["streamed"])
        self.assertIn("chunk_count", result)
        self.assertGreater(result["chunk_count"], 0)
        # httpbin.org/stream/2 returns newline-delimited JSON
        self.assertIn("id", result["content"])

class TestRequestXAsyncStreaming(unittest.IsolatedAsyncioTestCase):
    """Test async streaming functionality for RequestXAdapter."""

    async def asyncSetUp(self):
        self.adapter = RequestXAdapter()
        await self.adapter.__aenter__()
        self.request = HTTPRequest(
            method="GET",
            url="https://httpbin.org/stream/2",
            stream=True,
            timeout=30,
        )

    async def asyncTearDown(self):
        await self.adapter.__aexit__(None, None, None)

    async def test_requestx_stream_async_request(self):
        """Test async streaming request with requestx adapter."""
        result = await self.adapter.make_request_stream_async(self.request)

        self.assertTrue(result["success"])
        self.assertEqual(result["status_code"], 200)
        self.assertTrue(result["streamed"])
        self.assertIn("chunk_count", result)
        self.assertGreater(result["chunk_count"], 0)
        # httpbin.org/stream/2 returns newline-delimited JSON
        self.assertIn("id", result["content"])