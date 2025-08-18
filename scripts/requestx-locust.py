#!/usr/bin/env python3
"""
RequestX Locust Benchmark Script

This script uses Locust to benchmark different HTTP clients against a target host
with GET requests only.

Usage:
    # Run with specific client
    locust -f scripts/requestx-locust.py RequestXSyncUser --host http://localhost:8080
    
    # Run with web UI
    locust -f scripts/requestx-locust.py --host http://localhost:8080
    
    # Run headless
    locust -f scripts/requestx-locust.py RequestXSyncUser --host http://localhost:8080 --users 10 --spawn-rate 2 --run-time 60s --headless
"""

import asyncio
import time
from concurrent.futures import ThreadPoolExecutor

import aiohttp
import httpx
import requests
from locust import HttpUser, between, task, events
from locust.exception import StopUser

try:
    import requestx
    REQUESTX_AVAILABLE = True
except ImportError:
    REQUESTX_AVAILABLE = False
    print("Warning: requestx not available. RequestX benchmarks will be skipped.")


class BaseHttpUser(HttpUser):
    """Base class for HTTP client users"""
    wait_time = between(1, 3)
    host = "http://localhost:8080"
    
    def fire_request_event(self, request_type, name, start_time, response=None, exception=None):
        """Fire Locust request event for metrics collection"""
        response_time = int((time.time() - start_time) * 1000)
        
        if exception:
            # Fire failure event
            events.request.fire(
                request_type=request_type,
                name=name,
                response_time=response_time,
                response_length=0,
                exception=exception
            )
        elif response and hasattr(response, 'status_code') and response.status_code == 200:
            # Fire success event
            response_length = len(response.content) if hasattr(response, 'content') else 0
            events.request.fire(
                request_type=request_type,
                name=name,
                response_time=response_time,
                response_length=response_length,
            )
        elif response and hasattr(response, 'status_code'):
            # Fire failure event for non-200 status codes
            events.request.fire(
                request_type=request_type,
                name=name,
                response_time=response_time,
                response_length=0,
                exception=f"HTTP {response.status_code}"
            )
        elif response and hasattr(response, 'status') and response.status == 200:
            # For aiohttp responses
            events.request.fire(
                request_type=request_type,
                name=name,
                response_time=response_time,
                response_length=0,  # aiohttp doesn't easily provide content length
            )
        elif response and hasattr(response, 'status'):
            # Fire failure event for aiohttp non-200 status
            events.request.fire(
                request_type=request_type,
                name=name,
                response_time=response_time,
                response_length=0,
                exception=f"HTTP {response.status}"
            )


class RequestXSyncUser(BaseHttpUser):
    """RequestX synchronous client user"""
    
    def on_start(self):
        if not REQUESTX_AVAILABLE:
            raise StopUser("RequestX not available")

    @task
    def get_request(self):
        start_time = time.time()
        try:
            response = requestx.get(f"{self.host}/get")
            self.fire_request_event("GET", "/get", start_time, response=response)
        except Exception as e:
            self.fire_request_event("GET", "/get", start_time, exception=e)


class RequestXAsyncUser(BaseHttpUser):
    """RequestX asynchronous client user"""
    
    def on_start(self):
        if not REQUESTX_AVAILABLE:
            raise StopUser("RequestX not available")
        self.executor = ThreadPoolExecutor(max_workers=1)

    @task
    def get_request(self):
        def async_get():
            async def _get():
                start_time = time.time()
                try:
                    response = await requestx.get(f"{self.host}/get")
                    self.fire_request_event("GET", "/get", start_time, response=response)
                except Exception as e:
                    self.fire_request_event("GET", "/get", start_time, exception=e)
            
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            try:
                loop.run_until_complete(_get())
            finally:
                loop.close()
        
        self.executor.submit(async_get)


class HttpxSyncUser(BaseHttpUser):
    """HTTPX synchronous client user"""
    
    def on_start(self):
        self.client = httpx.Client()
    
    def on_stop(self):
        self.client.close()

    @task
    def get_request(self):
        start_time = time.time()
        try:
            response = self.client.get(f"{self.host}/get")
            self.fire_request_event("GET", "/get", start_time, response=response)
        except Exception as e:
            self.fire_request_event("GET", "/get", start_time, exception=e)


class HttpxAsyncUser(BaseHttpUser):
    """HTTPX asynchronous client user"""
    
    def on_start(self):
        self.executor = ThreadPoolExecutor(max_workers=1)
    
    def on_stop(self):
        self.executor.shutdown(wait=True)

    @task
    def get_request(self):
        def async_get():
            async def _get():
                start_time = time.time()
                async with httpx.AsyncClient() as client:
                    try:
                        response = await client.get(f"{self.host}/get")
                        self.fire_request_event("GET", "/get", start_time, response=response)
                    except Exception as e:
                        self.fire_request_event("GET", "/get", start_time, exception=e)
            
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            try:
                loop.run_until_complete(_get())
            finally:
                loop.close()
        
        self.executor.submit(async_get)


class RequestsUser(BaseHttpUser):
    """Requests client user"""
    
    def on_start(self):
        self.session = requests.Session()

    @task
    def get_request(self):
        start_time = time.time()
        try:
            response = self.session.get(f"{self.host}/get")
            self.fire_request_event("GET", "/get", start_time, response=response)
        except Exception as e:
            self.fire_request_event("GET", "/get", start_time, exception=e)


class AiohttpUser(BaseHttpUser):
    """Aiohttp client user"""

    @task
    def get_request(self):
        def async_get():
            async def _get():
                start_time = time.time()
                async with aiohttp.ClientSession() as session:
                    try:
                        async with session.get(f"{self.host}/get") as response:
                            self.fire_request_event("GET", "/get", start_time, response=response)
                    except Exception as e:
                        self.fire_request_event("GET", "/get", start_time, exception=e)
            
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            try:
                loop.run_until_complete(_get())
            finally:
                loop.close()
        
        import threading
        thread = threading.Thread(target=async_get)
        thread.start()
        thread.join()


if __name__ == "__main__":
    print("RequestX Locust Benchmark Script")
    print("Available HttpUser classes:")
    print("- RequestXSyncUser")
    print("- RequestXAsyncUser") 
    print("- HttpxSyncUser")
    print("- HttpxAsyncUser")
    print("- RequestsUser")
    print("- AiohttpUser")
    print("\nUsage examples:")
    print("locust -f scripts/requestx-locust.py RequestXSyncUser --host http://localhost:8080")
    print("locust -f scripts/requestx-locust.py --host http://localhost:8080")
    print("locust -f scripts/requestx-locust.py RequestXSyncUser --host http://localhost:8080 --users 10 --spawn-rate 2 --run-time 60s --headless")