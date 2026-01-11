Async/Await Guide
=================

One of RequestX's most powerful features is native async/await support with automatic context detection. This guide covers everything you need to know about using RequestX in asynchronous applications.

Why Use Async?
-------------

**Performance Benefits**
   * Handle thousands of concurrent requests
   * Non-blocking I/O operations
   * Better resource utilization
   * Ideal for I/O-bound applications

**Use Cases**
   * Web scraping at scale
   * API aggregation services
   * Microservices communication
   * Real-time data processing

Automatic Context Detection
--------------------------

RequestX automatically detects whether you're in a synchronous or asynchronous context:

.. code-block:: python

   import requestx
   import asyncio
   
   # Synchronous context - runs immediately
   def sync_function():
       response = requestx.get('https://httpbin.org/json')
       return response.json()
   
   # Asynchronous context - returns awaitable
   async def async_function():
       response = await requestx.get('https://httpbin.org/json')
       return response.json()
   
   # Usage
   sync_data = sync_function()  # Immediate result
   async_data = asyncio.run(async_function())  # Awaitable result

Basic Async Usage
----------------

Simple Async Request
~~~~~~~~~~~~~~~~~~~

.. code-block:: python

   import asyncio
   import requestx
   
   async def fetch_data():
       response = await requestx.get('https://httpbin.org/json')
       return response.json()
   
   # Run the async function
   data = asyncio.run(fetch_data())
   print(data)

Multiple Concurrent Requests
~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: python

   import asyncio
   import requestx
   
   async def fetch_url(url):
       response = await requestx.get(url)
       return response.json()
   
   async def fetch_multiple():
       urls = [
           'https://httpbin.org/json',
           'https://httpbin.org/uuid',
           'https://httpbin.org/ip',
       ]
       
       # Run requests concurrently
       tasks = [fetch_url(url) for url in urls]
       results = await asyncio.gather(*tasks)
       return results
   
   # Execute
   results = asyncio.run(fetch_multiple())
   for i, result in enumerate(results):
       print(f"Result {i}: {result}")

Async Sessions
-------------

Sessions provide connection pooling and state persistence in async contexts:

Basic Async Session
~~~~~~~~~~~~~~~~~~

.. code-block:: python

   import asyncio
   import requestx
   
   async def use_session():
       session = requestx.Session()
       
       # Set default headers
       session.headers.update({'User-Agent': 'RequestX-Async/1.0'})
       
       # Make multiple requests with connection reuse
       response1 = await session.get('https://httpbin.org/headers')
       response2 = await session.get('https://httpbin.org/json')
       response3 = await session.post('https://httpbin.org/post', json={'async': True})
       
       return [response1.json(), response2.json(), response3.json()]
   
   results = asyncio.run(use_session())

Session Context Manager
~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: python

   import asyncio
   import requestx
   
   async def with_session_context():
       async with requestx.Session() as session:
           session.headers.update({'Authorization': 'Bearer token'})
           
           # Session automatically closed when exiting context
           response = await session.get('https://api.example.com/data')
           return response.json()
   
   data = asyncio.run(with_session_context())

Advanced Async Patterns
----------------------

Rate Limiting with Semaphore
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Control concurrency to avoid overwhelming servers:

.. code-block:: python

   import asyncio
   import requestx
   
   async def fetch_with_limit(session, url, semaphore):
       async with semaphore:  # Limit concurrent requests
           response = await session.get(url)
           return response.json()
   
   async def fetch_many_with_limit():
       # Allow only 5 concurrent requests
       semaphore = asyncio.Semaphore(5)
       session = requestx.Session()
       
       urls = [f'https://httpbin.org/delay/1?id={i}' for i in range(20)]
       
       tasks = [fetch_with_limit(session, url, semaphore) for url in urls]
       results = await asyncio.gather(*tasks)
       return results
   
   results = asyncio.run(fetch_many_with_limit())

Timeout and Error Handling
~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: python

   import asyncio
   import requestx
   from requestx import RequestException, Timeout, HTTPError
   
   async def robust_fetch(url, timeout=10, retries=3):
       session = requestx.Session()
       
       for attempt in range(retries):
           try:
               response = await session.get(url, timeout=timeout)
               response.raise_for_status()
               return response.json()
           except Timeout:
               print(f"Timeout on attempt {attempt + 1}")
               if attempt == retries - 1:
                   raise
           except HTTPError as e:
               print(f"HTTP error {e.response.status_code} on attempt {attempt + 1}")
               if attempt == retries - 1:
                   raise
           except RequestException as e:
               print(f"Request failed on attempt {attempt + 1}: {e}")
               if attempt == retries - 1:
                   raise
           
           # Wait before retry
           await asyncio.sleep(2 ** attempt)  # Exponential backoff
   
   # Usage
   async def main():
       try:
           data = await robust_fetch('https://httpbin.org/status/500')
           print(data)
       except RequestException as e:
           print(f"All retries failed: {e}")
   
   asyncio.run(main())

Progress Tracking
~~~~~~~~~~~~~~~~

Track progress of multiple async requests:

.. code-block:: python

   import asyncio
   import requestx
   from typing import List, Dict, Any
   
   async def fetch_with_progress(session: requestx.Session, url: str, progress: Dict[str, Any]):
       try:
           response = await session.get(url)
           result = response.json()
           progress['completed'] += 1
           progress['results'].append(result)
           print(f"Progress: {progress['completed']}/{progress['total']}")
           return result
       except Exception as e:
           progress['errors'] += 1
           progress['failed'].append({'url': url, 'error': str(e)})
           raise
   
   async def fetch_with_progress_tracking(urls: List[str]):
       progress = {
           'total': len(urls),
           'completed': 0,
           'errors': 0,
           'results': [],
           'failed': []
       }
       
       session = requestx.Session()
       tasks = [fetch_with_progress(session, url, progress) for url in urls]
       
       # Use return_exceptions=True to handle errors gracefully
       results = await asyncio.gather(*tasks, return_exceptions=True)
       
       print(f"Completed: {progress['completed']}, Errors: {progress['errors']}")
       return progress
   
   # Usage
   urls = [f'https://httpbin.org/delay/1?id={i}' for i in range(10)]
   progress = asyncio.run(fetch_with_progress_tracking(urls))

Streaming Responses
------------------

Handle large responses efficiently:

.. code-block:: python

   import asyncio
   import requestx
   
   async def stream_large_file():
       session = requestx.Session()
       
       # Note: Streaming is handled automatically by RequestX
       # Large responses are processed efficiently
       response = await session.get('https://httpbin.org/stream/1000')
       
       # Process response data
       data = response.json()  # Efficiently handles large JSON
       return data
   
   # For very large files, you might want to process in chunks
   async def process_large_response():
       session = requestx.Session()
       response = await session.get('https://httpbin.org/bytes/1048576')  # 1MB
       
       # Content is available as bytes
       content = response.content
       print(f"Downloaded {len(content)} bytes")
       return content

Integration with Web Frameworks
------------------------------

FastAPI Integration
~~~~~~~~~~~~~~~~~~

.. code-block:: python

   from fastapi import FastAPI
   import requestx
   
   app = FastAPI()
   
   # Create a global session for reuse
   http_session = requestx.Session()
   
   @app.get("/proxy/{path:path}")
   async def proxy_request(path: str):
       # RequestX automatically works in FastAPI's async context
       response = await http_session.get(f"https://api.example.com/{path}")
       return response.json()
   
   @app.post("/aggregate")
   async def aggregate_data():
       # Make multiple concurrent requests
       tasks = [
           http_session.get("https://api.service1.com/data"),
           http_session.get("https://api.service2.com/data"),
           http_session.get("https://api.service3.com/data"),
       ]
       
       responses = await asyncio.gather(*tasks)
       return {
           "service1": responses[0].json(),
           "service2": responses[1].json(),
           "service3": responses[2].json(),
       }

Aiohttp Integration
~~~~~~~~~~~~~~~~~~

.. code-block:: python

   from aiohttp import web
   import requestx
   
   # Global session
   http_session = requestx.Session()
   
   async def handle_request(request):
       # RequestX works seamlessly with aiohttp
       response = await http_session.get('https://api.example.com/data')
       return web.json_response(response.json())
   
   app = web.Application()
   app.router.add_get('/data', handle_request)

Performance Optimization
-----------------------

Connection Pooling
~~~~~~~~~~~~~~~~~

.. code-block:: python

   import asyncio
   import requestx
   
   async def optimized_requests():
       # Reuse session for connection pooling
       session = requestx.Session()
       
       # Configure session for optimal performance
       session.headers.update({
           'Connection': 'keep-alive',
           'User-Agent': 'RequestX-Optimized/1.0'
       })
       
       # Make many requests to the same host
       tasks = []
       for i in range(100):
           task = session.get(f'https://httpbin.org/get?id={i}')
           tasks.append(task)
       
       # Execute concurrently with connection reuse
       responses = await asyncio.gather(*tasks)
       return [r.json() for r in responses]

Batch Processing
~~~~~~~~~~~~~~~

.. code-block:: python

   import asyncio
   import requestx
   from typing import List, Any
   
   async def process_batch(session: requestx.Session, batch: List[str]) -> List[Any]:
       """Process a batch of URLs concurrently"""
       tasks = [session.get(url) for url in batch]
       responses = await asyncio.gather(*tasks, return_exceptions=True)
       
       results = []
       for response in responses:
           if isinstance(response, Exception):
               results.append({'error': str(response)})
           else:
               results.append(response.json())
       
       return results
   
   async def process_urls_in_batches(urls: List[str], batch_size: int = 10):
       """Process URLs in batches to control memory usage"""
       session = requestx.Session()
       all_results = []
       
       for i in range(0, len(urls), batch_size):
           batch = urls[i:i + batch_size]
           print(f"Processing batch {i//batch_size + 1}")
           
           batch_results = await process_batch(session, batch)
           all_results.extend(batch_results)
           
           # Optional: Add delay between batches
           await asyncio.sleep(0.1)
       
       return all_results
   
   # Usage
   urls = [f'https://httpbin.org/get?id={i}' for i in range(100)]
   results = asyncio.run(process_urls_in_batches(urls, batch_size=20))

Testing Async Code
-----------------

Unit Testing
~~~~~~~~~~~

.. code-block:: python

   import asyncio
   import unittest
   import requestx
   
   class TestAsyncRequests(unittest.TestCase):
       def setUp(self):
           self.session = requestx.Session()
       
       def test_async_get(self):
           async def async_test():
               response = await self.session.get('https://httpbin.org/json')
               self.assertEqual(response.status_code, 200)
               data = response.json()
               self.assertIn('slideshow', data)
           
           asyncio.run(async_test())
       
       def test_concurrent_requests(self):
           async def async_test():
               urls = ['https://httpbin.org/json', 'https://httpbin.org/uuid']
               tasks = [self.session.get(url) for url in urls]
               responses = await asyncio.gather(*tasks)
               
               self.assertEqual(len(responses), 2)
               for response in responses:
                   self.assertEqual(response.status_code, 200)
           
           asyncio.run(async_test())

Pytest with Async
~~~~~~~~~~~~~~~~~

.. code-block:: python

   import pytest
   import requestx
   
   @pytest.mark.asyncio
   async def test_async_request():
       session = requestx.Session()
       response = await session.get('https://httpbin.org/json')
       assert response.status_code == 200
       assert 'slideshow' in response.json()
   
   @pytest.mark.asyncio
   async def test_concurrent_requests():
       session = requestx.Session()
       urls = ['https://httpbin.org/json', 'https://httpbin.org/uuid']
       
       tasks = [session.get(url) for url in urls]
       responses = await asyncio.gather(*tasks)
       
       assert len(responses) == 2
       for response in responses:
           assert response.status_code == 200

Best Practices
-------------

1. **Reuse Sessions**: Always reuse session objects for connection pooling
2. **Control Concurrency**: Use semaphores to limit concurrent requests
3. **Handle Errors**: Implement proper error handling and retries
4. **Use Context Managers**: Use ``async with`` for automatic cleanup
5. **Monitor Performance**: Track request times and success rates
6. **Respect Rate Limits**: Implement backoff strategies for APIs
7. **Process in Batches**: For large datasets, process in manageable batches

Common Pitfalls
--------------

**Creating Too Many Sessions**
   .. code-block:: python
   
      # Bad: Creates new session for each request
      async def bad_pattern():
          for url in urls:
              session = requestx.Session()  # Don't do this!
              response = await session.get(url)
      
      # Good: Reuse session
      async def good_pattern():
          session = requestx.Session()
          for url in urls:
              response = await session.get(url)

**Not Handling Exceptions**
   .. code-block:: python
   
      # Bad: No error handling
      async def bad_error_handling():
          response = await requestx.get('https://might-fail.com')
          return response.json()  # Might raise exception
      
      # Good: Proper error handling
      async def good_error_handling():
          try:
              response = await requestx.get('https://might-fail.com', timeout=10)
              response.raise_for_status()
              return response.json()
          except requestx.RequestException as e:
              print(f"Request failed: {e}")
              return None

**Blocking the Event Loop**
   .. code-block:: python
   
      # Bad: Blocking operations in async function
      async def bad_blocking():
          response = await requestx.get('https://api.example.com')
          time.sleep(1)  # Blocks the event loop!
          return response.json()
      
      # Good: Use async sleep
      async def good_async():
          response = await requestx.get('https://api.example.com')
          await asyncio.sleep(1)  # Non-blocking
          return response.json()

Next Steps
---------

* Learn about :doc:`user-guide/sessions-and-cookies` for advanced session management
* Check out :doc:`examples/async-usage` for more practical examples
* Read about :doc:`performance` to optimize your async applications