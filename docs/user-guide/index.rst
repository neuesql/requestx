User Guide
==========

This comprehensive user guide covers all aspects of using RequestX effectively.

.. toctree::
   :maxdepth: 2

   making-requests
   handling-responses
   authentication
   sessions-and-cookies
   timeouts-and-retries
   ssl-and-certificates
   proxies
   streaming
   file-uploads
   error-handling

Overview
--------

RequestX is designed to be a drop-in replacement for the popular ``requests`` library while providing significant performance improvements through its Rust-based implementation. This guide will help you understand how to use RequestX effectively for all your HTTP client needs.

Key Concepts
-----------

**Synchronous and Asynchronous APIs**
   RequestX provides both sync and async APIs using the same functions. The library automatically detects the execution context and behaves appropriately.

**Session Management**
   Sessions allow you to persist certain parameters across requests, such as cookies, headers, and connection pooling for better performance.

**Error Handling**
   RequestX provides comprehensive error handling with exceptions that match the ``requests`` library for easy migration.

**Performance Optimization**
   Built with Rust, RequestX offers superior performance while maintaining full compatibility with the ``requests`` API.

Performance Best Practices
--------------------------

To get the most out of RequestX, follow these performance best practices:

**1. Reuse Session Objects**
   Always use a ``requestx.Session()`` when making multiple requests to the same host. This allows RequestX to reuse underlying TCP connections (and TLS handshakes), significantly reducing latency and CPU usage.

   .. code-block:: python

       import requestx
       
       # Efficient: Connection is reused
       with requestx.Session() as session:
           for i in range(100):
               session.get(f"https://api.example.com/items/{i}")

**2. Use Async for Concurrency**
   If you need to make many independent requests, use the asynchronous API with ``asyncio.gather``.

   .. code-block:: python

       import asyncio
       import requestx
       
       async def main():
           async with requestx.Session() as session:
               tasks = [session.get(f"https://api.example.com/items/{i}") for i in range(10)]
               responses = await asyncio.gather(*tasks)

**3. Configure Connection Pools**
   For high-load applications, adjust the ``pool_max_idle_per_host`` in your ``config.toml``. The default is 1024, which is suitable for most high-concurrency use cases.

**4. Stream Large Responses**
   When downloading large files, use ``stream=True`` to avoid loading the entire response into memory at once.

   .. code-block:: python

       response = requestx.get("https://example.com/large-file.zip", stream=True)
       for chunk in response.iter_content(chunk_size=8192):
           process_chunk(chunk)

Getting Started
--------------

If you're new to RequestX, start with the :doc:`../quickstart` guide. If you're migrating from ``requests``, check out the :doc:`../migration` guide.

For async/await usage patterns, see the :doc:`../async-guide`.

Common Patterns
--------------

Here are some common usage patterns you'll find throughout this guide:

**Basic Request**

.. code-block:: python

   import requestx
   
   response = requestx.get('https://api.example.com/data')
   data = response.json()

**With Session**

.. code-block:: python

   import requestx
   
   with requestx.Session() as session:
       session.headers.update({'Authorization': 'Bearer token'})
       response = session.get('https://api.example.com/data')

**Async Usage**

.. code-block:: python

   import asyncio
   import requestx
   
   async def fetch_data():
       response = await requestx.get('https://api.example.com/data')
       return response.json()
   
   data = asyncio.run(fetch_data())

**Error Handling**

.. code-block:: python

   import requestx
   
   try:
       response = requestx.get('https://api.example.com/data', timeout=10)
       response.raise_for_status()
       return response.json()
   except requestx.HTTPError as e:
       print(f"HTTP error: {e}")
   except requestx.RequestException as e:
       print(f"Request failed: {e}")

Next Steps
---------

Choose a topic from the table of contents above, or continue with :doc:`making-requests` to learn about the fundamentals of making HTTP requests with RequestX.