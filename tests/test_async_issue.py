#!/usr/bin/env python3
"""
Test to reproduce the async RequestX issue seen in performance tests.
"""

import asyncio
import time
import traceback

try:
    import requestx

    HAS_REQUESTX = True
except ImportError:
    HAS_REQUESTX = False
    print("Warning: requestx not available")

try:
    import aiohttp

    HAS_AIOHTTP = True
except ImportError:
    HAS_AIOHTTP = False
    print("Warning: aiohttp not available")

try:
    import httpx

    HAS_HTTPX = True
except ImportError:
    HAS_HTTPX = False
    print("Warning: httpx not available")


async def test_requestx_async():
    """Test RequestX async functionality"""
    print("\n=== Testing RequestX Async ===")

    test_url = "http://127.0.0.1:8000/get"

    if not HAS_REQUESTX:
        print("RequestX not available")
        return

    # Check if RequestX has async methods
    print("RequestX module attributes:")
    requestx_attrs = [attr for attr in dir(requestx) if not attr.startswith("_")]
    print(f"  Available: {requestx_attrs}")

    # Try to find async methods
    async_methods = [attr for attr in requestx_attrs if "async" in attr.lower()]
    print(f"  Async methods: {async_methods}")

    # Check if there's a Session class with async methods
    if hasattr(requestx, "Session"):
        session = requestx.Session()
        session_attrs = [attr for attr in dir(session) if not attr.startswith("_")]
        print(f"  Session attributes: {session_attrs}")

        session_async_methods = [
            attr for attr in session_attrs if "async" in attr.lower()
        ]
        print(f"  Session async methods: {session_async_methods}")

        # Try to use session async methods if they exist
        if session_async_methods:
            for method_name in session_async_methods:
                try:
                    method = getattr(session, method_name)
                    print(f"  Trying {method_name}...")

                    if method_name.endswith("_async") or "get" in method_name:
                        result = await method(test_url)
                        print(f"    Success: Status {result.status_code}")
                    else:
                        print(f"    Skipping {method_name} (not a GET method)")

                except Exception as e:
                    print(f"    Error with {method_name}: {e}")
                    traceback.print_exc()

    # Try direct async calls if they exist
    if hasattr(requestx, "get_async"):
        try:
            print("  Trying requestx.get_async...")
            result = await requestx.get_async(test_url)
            print(f"    Success: Status {result.status_code}")
        except Exception as e:
            print(f"    Error with get_async: {e}")
            traceback.print_exc()

    # Try using regular requestx.get in async context (this might be the issue)
    try:
        print("  Trying requestx.get in async context...")
        result = requestx.get(test_url)
        print(f"    Success: Status {result.status_code}")
    except Exception as e:
        print(f"    Error with sync get in async context: {e}")
        traceback.print_exc()


async def test_aiohttp_async():
    """Test aiohttp for comparison"""
    print("\n=== Testing aiohttp Async ===")

    if not HAS_AIOHTTP:
        print("aiohttp not available")
        return

    test_url = "http://127.0.0.1:8000/get"

    try:
        async with aiohttp.ClientSession() as session:
            async with session.get(test_url) as response:
                print(f"  aiohttp Success: Status {response.status}")
                text = await response.text()
                print(f"  Response length: {len(text)} chars")
    except Exception as e:
        print(f"  aiohttp Error: {e}")
        traceback.print_exc()


async def test_httpx_async():
    """Test httpx async for comparison"""
    print("\n=== Testing httpx Async ===")

    if not HAS_HTTPX:
        print("httpx not available")
        return

    test_url = "http://127.0.0.1:8000/get"

    try:
        async with httpx.AsyncClient() as client:
            response = await client.get(test_url)
            print(f"  httpx Success: Status {response.status_code}")
            print(f"  Response length: {len(response.text)} chars")
    except Exception as e:
        print(f"  httpx Error: {e}")
        traceback.print_exc()


async def test_concurrent_async_requests():
    """Test concurrent async requests"""
    print("\n=== Testing Concurrent Async Requests ===")

    test_url = "http://127.0.0.1:8000/get"
    num_requests = 5

    # Test with aiohttp
    if HAS_AIOHTTP:
        print("\nTesting aiohttp concurrent requests:")

        async def aiohttp_request(session, request_id):
            try:
                start_time = time.time()
                async with session.get(test_url) as response:
                    end_time = time.time()
                    return {
                        "request_id": request_id,
                        "status": response.status,
                        "time": (end_time - start_time) * 1000,
                        "success": True,
                    }
            except Exception as e:
                return {"request_id": request_id, "error": str(e), "success": False}

        try:
            async with aiohttp.ClientSession() as session:
                tasks = [aiohttp_request(session, i) for i in range(num_requests)]
                results = await asyncio.gather(*tasks)

                successful = [r for r in results if r["success"]]
                failed = [r for r in results if not r["success"]]

                print(
                    f"  aiohttp Results: {len(successful)} successful, {len(failed)} failed"
                )
                for result in results:
                    if result["success"]:
                        print(
                            f"    Request {result['request_id']}: Status {result['status']}, Time: {result['time']:.1f}ms"
                        )
                    else:
                        print(
                            f"    Request {result['request_id']}: ERROR - {result['error']}"
                        )
        except Exception as e:
            print(f"  aiohttp concurrent test failed: {e}")
            traceback.print_exc()

    # Test with httpx
    if HAS_HTTPX:
        print("\nTesting httpx concurrent requests:")

        async def httpx_request(client, request_id):
            try:
                start_time = time.time()
                response = await client.get(test_url)
                end_time = time.time()
                return {
                    "request_id": request_id,
                    "status": response.status_code,
                    "time": (end_time - start_time) * 1000,
                    "success": True,
                }
            except Exception as e:
                return {"request_id": request_id, "error": str(e), "success": False}

        try:
            async with httpx.AsyncClient() as client:
                tasks = [httpx_request(client, i) for i in range(num_requests)]
                results = await asyncio.gather(*tasks)

                successful = [r for r in results if r["success"]]
                failed = [r for r in results if not r["success"]]

                print(
                    f"  httpx Results: {len(successful)} successful, {len(failed)} failed"
                )
                for result in results:
                    if result["success"]:
                        print(
                            f"    Request {result['request_id']}: Status {result['status']}, Time: {result['time']:.1f}ms"
                        )
                    else:
                        print(
                            f"    Request {result['request_id']}: ERROR - {result['error']}"
                        )
        except Exception as e:
            print(f"  httpx concurrent test failed: {e}")
            traceback.print_exc()


async def main():
    """Run all async tests"""
    print("Starting RequestX Async Issue Detection...")
    print("=" * 60)

    await test_requestx_async()
    await test_aiohttp_async()
    await test_httpx_async()
    await test_concurrent_async_requests()

    print("\n" + "=" * 60)
    print("Async issue detection completed!")


if __name__ == "__main__":
    asyncio.run(main())
