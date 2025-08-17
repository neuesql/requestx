Examples
========

This section contains practical examples of using RequestX for common tasks.

.. toctree::
   :maxdepth: 2

   basic-usage
   async-usage
   sessions
   advanced
   migration-examples
   performance-examples

Overview
--------

These examples demonstrate real-world usage patterns for RequestX, from simple GET requests to advanced async patterns and performance optimization techniques.

**Basic Examples**
   Simple synchronous usage patterns that work exactly like the ``requests`` library.

**Async Examples**
   Asynchronous usage with ``async/await`` for high-performance applications.

**Session Examples**
   Using sessions for connection reuse, authentication, and state management.

**Advanced Examples**
   Complex scenarios including error handling, retries, streaming, and file uploads.

**Migration Examples**
   Side-by-side comparisons showing how to migrate from ``requests`` to RequestX.

**Performance Examples**
   Optimization techniques and benchmarking code.

Quick Examples
-------------

**Simple GET Request**

.. code-block:: python

   import requestx
   
   response = requestx.get('https://api.github.com/users/octocat')
   user_data = response.json()
   print(f"User: {user_data['name']}")

**POST with JSON**

.. code-block:: python

   import requestx
   
   data = {'name': 'John Doe', 'email': 'john@example.com'}
   response = requestx.post('https://api.example.com/users', json=data)
   
   if response.status_code == 201:
       print("User created successfully!")

**Async Concurrent Requests**

.. code-block:: python

   import asyncio
   import requestx
   
   async def fetch_multiple():
       urls = [
           'https://api.github.com/users/octocat',
           'https://api.github.com/users/defunkt',
           'https://api.github.com/users/pjhyett'
       ]
       
       tasks = [requestx.get(url) for url in urls]
       responses = await asyncio.gather(*tasks)
       
       return [r.json() for r in responses]
   
   users = asyncio.run(fetch_multiple())

**Session with Authentication**

.. code-block:: python

   import requestx
   
   session = requestx.Session()
   session.headers.update({'Authorization': 'Bearer your-token'})
   
   # All requests in this session will include the auth header
   response = session.get('https://api.example.com/protected')
   data = response.json()

**Error Handling**

.. code-block:: python

   import requestx
   from requestx import HTTPError, ConnectionError, Timeout
   
   try:
       response = requestx.get('https://api.example.com/data', timeout=10)
       response.raise_for_status()
       return response.json()
   except HTTPError as e:
       print(f"HTTP error {e.response.status_code}: {e}")
   except ConnectionError:
       print("Failed to connect to the server")
   except Timeout:
       print("Request timed out")

Use Case Categories
------------------

**Web Scraping**
   Examples for scraping websites efficiently with RequestX's performance benefits.

**API Integration**
   Patterns for integrating with REST APIs, handling authentication, and managing rate limits.

**Microservices Communication**
   Service-to-service communication patterns using async RequestX.

**Data Processing**
   Fetching and processing large datasets with concurrent requests.

**Testing and Mocking**
   Examples of testing HTTP clients and mocking responses.

**File Operations**
   Uploading and downloading files with progress tracking.

Getting Started
--------------

If you're new to RequestX:

1. Start with :doc:`basic-usage` for fundamental patterns
2. Move to :doc:`async-usage` for performance-critical applications
3. Check :doc:`sessions` for advanced connection management
4. Explore :doc:`advanced` for complex scenarios

If you're migrating from ``requests``:

1. Review :doc:`migration-examples` for side-by-side comparisons
2. Check :doc:`performance-examples` to see the benefits
3. Use :doc:`../migration` for a complete migration guide

Code Organization
----------------

All examples follow these conventions:

* **Imports**: Clear import statements at the top
* **Error Handling**: Proper exception handling where appropriate
* **Comments**: Explanatory comments for complex logic
* **Type Hints**: Type annotations for better code clarity
* **Best Practices**: Following RequestX and Python best practices

Running Examples
---------------

All examples can be run directly:

.. code-block:: bash

   # Save example to a file
   python example.py
   
   # Or run interactively
   python -i example.py

Most examples use public APIs like httpbin.org for demonstration, so they work out of the box without requiring API keys or setup.

Contributing Examples
--------------------

Have a useful RequestX example? We'd love to include it! See our `contributing guide <https://github.com/neuesql/requestx/blob/main/CONTRIBUTING.md>`_ for details on submitting examples.

Good examples should:

* Solve a real-world problem
* Include error handling
* Be well-commented
* Follow Python best practices
* Work with public APIs when possible