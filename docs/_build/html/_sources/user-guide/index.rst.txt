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