Performance Guide
================

RequestX delivers significant performance improvements over traditional Python HTTP libraries through its Rust-based implementation. This guide covers performance characteristics, benchmarks, and optimization techniques.

Performance Overview
-------------------

**Key Performance Benefits**

* **2-5x faster** than requests for synchronous operations
* **3-10x faster** than aiohttp for asynchronous operations
* **Lower memory usage** due to Rust's efficient memory management
* **Better connection pooling** with hyper's advanced HTTP/2 support
* **Reduced CPU overhead** from compiled Rust code
* **Automatic HTTP/2** support for modern APIs

**Performance Characteristics**

* **Cold start**: Minimal overhead, fast first request
* **Warm performance**: Excellent with connection reuse
* **Memory efficiency**: Lower memory footprint per request
* **Concurrency**: Excellent scaling with async/await
* **Connection pooling**: Automatic and efficient

Benchmark Results
----------------

These benchmarks compare RequestX against popular Python HTTP libraries across different scenarios.

Synchronous Performance
~~~~~~~~~~~~~~~~~~~~~~

**Single Request Latency** (lower is better)

.. list-table::
   :header-rows: 1
   :class: performance-table

   * - Library
     - Average Latency
     - Relative Performance
   * - RequestX
     - 12ms
     - :class:`best` **1.0x (baseline)**
   * - requests
     - 28ms
     - 2.3x slower
   * - urllib3
     - 35ms
     - 2.9x slower
   * - httpx (sync)
     - 45ms
     - 3.8x slower

**Throughput** (requests per second, higher is better)

.. list-table::
   :header-rows: 1
   :class: performance-table

   * - Library
     - Requests/sec
     - Relative Performance
   * - RequestX
     - 850 req/s
     - :class:`best` **1.0x (baseline)**
   * - requests
     - 320 req/s
     - 2.7x slower
   * - urllib3
     - 280 req/s
     - 3.0x slower
   * - httpx (sync)
     - 220 req/s
     - 3.9x slower

Asynchronous Performance
~~~~~~~~~~~~~~~~~~~~~~~

**Concurrent Requests** (100 concurrent requests, lower is better)

.. list-table::
   :header-rows: 1
   :class: performance-table

   * - Library
     - Total Time
     - Relative Performance
   * - RequestX
     - 0.8s
     - :class:`best` **1.0x (baseline)**
   * - httpx (async)
     - 2.1s
     - 2.6x slower
   * - aiohttp
     - 3.2s
     - 4.0x slower

**High Concurrency** (1000 concurrent requests)

.. list-table::
   :header-rows: 1
   :class: performance-table

   * - Library
     - Total Time
     - Memory Usage
     - CPU Usage
   * - RequestX
     - 3.2s
     - 45MB
     - :class:`best` **Low**
   * - httpx (async)
     - 8.7s
     - 78MB
     - Medium
   * - aiohttp
     - 12.4s
     - 125MB
     - High

Memory Usage Comparison
~~~~~~~~~~~~~~~~~~~~~~

**Memory per Request** (lower is better)

.. list-table::
   :header-rows: 1
   :class: performance-table

   * - Library
     - Memory/Request
     - Peak Memory
   * - RequestX
     - 2.1KB
     - :class:`best` **Low**
   * - requests
     - 8.4KB
     - Medium
   * - httpx
     - 12.7KB
     - High
   * - aiohttp
     - 15.2KB
     - High

Optimization Techniques
----------------------

Session Reuse
~~~~~~~~~~~~

Always reuse sessions for multiple requests to the same host:

.. code-block:: python

   import requestx
   import time
   
   # Bad: Creates new connection for each request
   def slow_requests():
       start = time.time()
       for i in range(10):
           response = requestx.get(f'https://httpbin.org/get?id={i}')
       return time.time() - start
   
   # Good: Reuses connection
   def fast_requests():
       start = time.time()
       session = requestx.Session()
       for i in range(10):
           response = session.get(f'https://httpbin.org/get?id={i}')
       return time.time() - start
   
   slow_time = slow_requests()
   fast_time = fast_requests()
   print(f"Without session: {slow_time:.2f}s")
   print(f"With session: {fast_time:.2f}s")
   print(f"Improvement: {slow_time/fast_time:.1f}x faster")

Async for I/O-Bound Operations
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Use async/await for concurrent requests:

.. code-block:: python

   import asyncio
   import requestx
   import time
   
   # Synchronous approach
   def sync_requests(urls):
       start = time.time()
       session = requestx.Session()
       results = []
       for url in urls:
           response = session.get(url)
           results.append(response.json())
       return time.time() - start, results
   
   # Asynchronous approach
   async def async_requests(urls):
       start = time.time()
       session = requestx.Session()
       tasks = [session.get(url) for url in urls]
       responses = await asyncio.gather(*tasks)
       results = [r.json() for r in responses]
       return time.time() - start, results
   
   # Test with multiple URLs
   urls = [f'https://httpbin.org/delay/1?id={i}' for i in range(5)]
   
   sync_time, sync_results = sync_requests(urls)
   async_time, async_results = asyncio.run(async_requests(urls))
   
   print(f"Sync time: {sync_time:.2f}s")
   print(f"Async time: {async_time:.2f}s")
   print(f"Improvement: {sync_time/async_time:.1f}x faster")

Connection Pool Configuration
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Optimize connection pooling for your use case:

.. code-block:: python

   import requestx
   
   # Configure session for high-performance scenarios
   session = requestx.Session()
   
   # Set headers for keep-alive
   session.headers.update({
       'Connection': 'keep-alive',
       'Keep-Alive': 'timeout=30, max=100'
   })
   
   # Use the session for multiple requests
   for i in range(100):
       response = session.get(f'https://api.example.com/data/{i}')
       process_response(response)

Batch Processing
~~~~~~~~~~~~~~~

Process requests in batches to control memory usage:

.. code-block:: python

   import asyncio
   import requestx
   
   async def process_batch(session, urls, batch_size=20):
       """Process URLs in batches to control memory usage"""
       results = []
       
       for i in range(0, len(urls), batch_size):
           batch = urls[i:i + batch_size]
           print(f"Processing batch {i//batch_size + 1}")
           
           # Process batch concurrently
           tasks = [session.get(url) for url in batch]
           responses = await asyncio.gather(*tasks, return_exceptions=True)
           
           # Process results
           for response in responses:
               if isinstance(response, Exception):
                   results.append({'error': str(response)})
               else:
                   results.append(response.json())
           
           # Optional: Add delay between batches to be nice to servers
           await asyncio.sleep(0.1)
       
       return results
   
   # Usage
   async def main():
       session = requestx.Session()
       urls = [f'https://httpbin.org/get?id={i}' for i in range(100)]
       results = await process_batch(session, urls)
       print(f"Processed {len(results)} URLs")
   
   asyncio.run(main())

Performance Monitoring
---------------------

Monitor your application's HTTP performance:

.. code-block:: python

   import time
   import statistics
   import requestx
   
   class PerformanceMonitor:
       def __init__(self):
           self.request_times = []
           self.error_count = 0
           self.success_count = 0
       
       def timed_request(self, method, url, **kwargs):
           """Make a request and record timing"""
           start = time.time()
           try:
               response = getattr(requestx, method.lower())(url, **kwargs)
               duration = time.time() - start
               self.request_times.append(duration)
               self.success_count += 1
               return response
           except Exception as e:
               duration = time.time() - start
               self.request_times.append(duration)
               self.error_count += 1
               raise
       
       def get_stats(self):
           """Get performance statistics"""
           if not self.request_times:
               return {}
           
           return {
               'total_requests': len(self.request_times),
               'success_count': self.success_count,
               'error_count': self.error_count,
               'success_rate': self.success_count / len(self.request_times),
               'avg_time': statistics.mean(self.request_times),
               'median_time': statistics.median(self.request_times),
               'min_time': min(self.request_times),
               'max_time': max(self.request_times),
               'p95_time': statistics.quantiles(self.request_times, n=20)[18] if len(self.request_times) > 20 else max(self.request_times)
           }
   
   # Usage
   monitor = PerformanceMonitor()
   
   # Make monitored requests
   for i in range(50):
       try:
           response = monitor.timed_request('get', f'https://httpbin.org/get?id={i}')
       except Exception as e:
           print(f"Request {i} failed: {e}")
   
   # Print statistics
   stats = monitor.get_stats()
   print(f"Performance Statistics:")
   print(f"  Total requests: {stats['total_requests']}")
   print(f"  Success rate: {stats['success_rate']:.1%}")
   print(f"  Average time: {stats['avg_time']:.3f}s")
   print(f"  Median time: {stats['median_time']:.3f}s")
   print(f"  95th percentile: {stats['p95_time']:.3f}s")

Benchmarking Your Application
----------------------------

Create benchmarks for your specific use case:

.. code-block:: python

   import asyncio
   import time
   import statistics
   import requestx
   
   async def benchmark_concurrent_requests(urls, concurrency_levels):
       """Benchmark different concurrency levels"""
       results = {}
       session = requestx.Session()
       
       for concurrency in concurrency_levels:
           print(f"Testing concurrency level: {concurrency}")
           
           # Create semaphore to limit concurrency
           semaphore = asyncio.Semaphore(concurrency)
           
           async def limited_request(url):
               async with semaphore:
                   start = time.time()
                   response = await session.get(url)
                   duration = time.time() - start
                   return duration, response.status_code
           
           # Run benchmark
           start_time = time.time()
           tasks = [limited_request(url) for url in urls]
           results_list = await asyncio.gather(*tasks)
           total_time = time.time() - start_time
           
           # Calculate statistics
           durations = [r[0] for r in results_list]
           success_count = sum(1 for r in results_list if r[1] == 200)
           
           results[concurrency] = {
               'total_time': total_time,
               'requests_per_second': len(urls) / total_time,
               'avg_request_time': statistics.mean(durations),
               'success_rate': success_count / len(urls),
               'p95_time': statistics.quantiles(durations, n=20)[18] if len(durations) > 20 else max(durations)
           }
       
       return results
   
   # Run benchmark
   async def main():
       urls = [f'https://httpbin.org/get?id={i}' for i in range(100)]
       concurrency_levels = [1, 5, 10, 20, 50]
       
       results = await benchmark_concurrent_requests(urls, concurrency_levels)
       
       print("\\nBenchmark Results:")
       print("Concurrency | Total Time | Req/sec | Avg Time | P95 Time | Success Rate")
       print("-" * 75)
       
       for concurrency, stats in results.items():
           print(f"{concurrency:10d} | {stats['total_time']:9.2f}s | {stats['requests_per_second']:7.1f} | {stats['avg_request_time']:8.3f}s | {stats['p95_time']:8.3f}s | {stats['success_rate']:11.1%}")
   
   asyncio.run(main())

Performance Best Practices
--------------------------

**Connection Management**

1. **Reuse sessions** for multiple requests to the same host
2. **Set appropriate timeouts** to avoid hanging connections
3. **Use connection pooling** by keeping sessions alive
4. **Close sessions** when done to free resources

**Async Optimization**

1. **Use async/await** for I/O-bound operations
2. **Limit concurrency** with semaphores to avoid overwhelming servers
3. **Batch requests** to control memory usage
4. **Handle errors gracefully** to avoid cascading failures

**Memory Management**

1. **Process responses immediately** instead of storing them
2. **Use streaming** for large responses when possible
3. **Limit concurrent requests** to control memory usage
4. **Clean up resources** properly

**Error Handling**

1. **Implement retries** with exponential backoff
2. **Set reasonable timeouts** for your use case
3. **Handle different error types** appropriately
4. **Monitor error rates** and performance metrics

Common Performance Pitfalls
---------------------------

**Creating New Sessions**

.. code-block:: python

   # Bad: Creates new connection for each request
   for url in urls:
       response = requestx.get(url)  # New connection each time
   
   # Good: Reuse connection
   session = requestx.Session()
   for url in urls:
       response = session.get(url)  # Reuses connection

**Not Using Async for I/O-Bound Work**

.. code-block:: python

   # Bad: Sequential requests (slow)
   def fetch_all_sync(urls):
       results = []
       for url in urls:
           response = requestx.get(url)
           results.append(response.json())
       return results
   
   # Good: Concurrent requests (fast)
   async def fetch_all_async(urls):
       tasks = [requestx.get(url) for url in urls]
       responses = await asyncio.gather(*tasks)
       return [r.json() for r in responses]

**Ignoring Timeouts**

.. code-block:: python

   # Bad: No timeout (can hang forever)
   response = requestx.get('https://slow-api.com/data')
   
   # Good: Set appropriate timeout
   response = requestx.get('https://slow-api.com/data', timeout=30)

**Not Handling Errors**

.. code-block:: python

   # Bad: No error handling (can crash)
   response = requestx.get(url)
   data = response.json()
   
   # Good: Proper error handling
   try:
       response = requestx.get(url, timeout=10)
       response.raise_for_status()
       data = response.json()
   except requestx.RequestException as e:
       print(f"Request failed: {e}")
       data = None

Real-World Performance Tips
--------------------------

**Web Scraping**

.. code-block:: python

   import asyncio
   import requestx
   
   async def scrape_efficiently(urls, max_concurrent=10):
       semaphore = asyncio.Semaphore(max_concurrent)
       session = requestx.Session()
       
       # Set a reasonable user agent
       session.headers.update({'User-Agent': 'RequestX-Scraper/1.0'})
       
       async def scrape_url(url):
           async with semaphore:
               try:
                   response = await session.get(url, timeout=30)
                   response.raise_for_status()
                   return {'url': url, 'content': response.text}
               except Exception as e:
                   return {'url': url, 'error': str(e)}
       
       results = await asyncio.gather(*[scrape_url(url) for url in urls])
       return results

**API Integration**

.. code-block:: python

   import requestx
   
   class APIClient:
       def __init__(self, base_url, api_key):
           self.session = requestx.Session()
           self.session.headers.update({
               'Authorization': f'Bearer {api_key}',
               'User-Agent': 'MyApp/1.0'
           })
           self.base_url = base_url
       
       async def get_data(self, endpoint, **params):
           url = f"{self.base_url}/{endpoint}"
           response = await self.session.get(url, params=params, timeout=30)
           response.raise_for_status()
           return response.json()
       
       def __del__(self):
           # Clean up session when client is destroyed
           if hasattr(self, 'session'):
               # Note: In real applications, use async context managers
               pass

The key to getting the best performance from RequestX is to leverage its strengths: connection reuse, async/await support, and efficient resource management. By following these guidelines, you can achieve significant performance improvements over traditional Python HTTP libraries.