Migration from Requests
======================

RequestX is designed as a **drop-in replacement** for the popular ``requests`` library. This guide will help you migrate your existing code with minimal changes while taking advantage of RequestX's performance improvements.

Why Migrate?
-----------

**Performance Benefits**
   * 2-5x faster for synchronous requests
   * 3-10x faster for asynchronous operations
   * Lower memory usage
   * Better connection pooling

**Modern Features**
   * Native async/await support
   * HTTP/2 support out of the box
   * Automatic context detection
   * Rust-powered reliability

**Compatibility**
   * Same API as requests
   * Same exceptions and error handling
   * Same response objects and methods
   * Drop-in replacement in most cases

Simple Migration
---------------

The easiest way to migrate is to simply change your import statement:

**Before (requests):**

.. code-block:: python

   import requests
   
   response = requests.get('https://api.example.com/data')
   print(response.json())

**After (requestx):**

.. code-block:: python

   import requestx as requests  # Drop-in replacement
   
   response = requests.get('https://api.example.com/data')
   print(response.json())

Or use RequestX directly:

.. code-block:: python

   import requestx
   
   response = requestx.get('https://api.example.com/data')
   print(response.json())

API Compatibility
----------------

RequestX maintains 100% API compatibility with requests for all common use cases:

HTTP Methods
~~~~~~~~~~~

.. code-block:: python

   # All these work exactly the same
   import requestx as requests
   
   requests.get(url, **kwargs)
   requests.post(url, data=None, json=None, **kwargs)
   requests.put(url, data=None, **kwargs)
   requests.patch(url, data=None, **kwargs)
   requests.delete(url, **kwargs)
   requests.head(url, **kwargs)
   requests.options(url, **kwargs)

Parameters and Data
~~~~~~~~~~~~~~~~~~

.. code-block:: python

   import requestx as requests
   
   # URL parameters
   requests.get('https://api.example.com', params={'key': 'value'})
   
   # Form data
   requests.post('https://api.example.com', data={'key': 'value'})
   
   # JSON data
   requests.post('https://api.example.com', json={'key': 'value'})
   
   # Files
   with open('file.txt', 'rb') as f:
       requests.post('https://api.example.com', files={'file': f})

Headers and Authentication
~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: python

   import requestx as requests
   
   # Custom headers
   headers = {'Authorization': 'Bearer token'}
   requests.get('https://api.example.com', headers=headers)
   
   # Basic auth
   requests.get('https://api.example.com', auth=('user', 'pass'))

Response Handling
~~~~~~~~~~~~~~~~

.. code-block:: python

   import requestx as requests
   
   response = requests.get('https://api.example.com')
   
   # All these work the same
   print(response.status_code)
   print(response.headers)
   print(response.text)
   print(response.content)
   print(response.json())
   print(response.url)
   print(response.cookies)
   
   # Error handling
   response.raise_for_status()

Sessions
~~~~~~~

.. code-block:: python

   import requestx as requests
   
   # Sessions work identically
   session = requests.Session()
   session.headers.update({'User-Agent': 'My App'})
   
   response = session.get('https://api.example.com')

Exception Handling
~~~~~~~~~~~~~~~~~

.. code-block:: python

   import requestx as requests
   from requestx import RequestException, HTTPError, ConnectionError, Timeout
   
   try:
       response = requests.get('https://api.example.com', timeout=5)
       response.raise_for_status()
   except HTTPError as e:
       print(f"HTTP Error: {e}")
   except ConnectionError as e:
       print(f"Connection Error: {e}")
   except Timeout as e:
       print(f"Timeout: {e}")
   except RequestException as e:
       print(f"Request failed: {e}")

New Features in RequestX
-----------------------

While maintaining full compatibility, RequestX adds powerful new features:

Automatic Async/Await Support
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

The biggest advantage of RequestX is native async/await support with the same API:

.. code-block:: python

   import asyncio
   import requestx
   
   # Synchronous (same as requests)
   response = requestx.get('https://api.example.com')
   
   # Asynchronous (new in RequestX!)
   async def fetch_data():
       response = await requestx.get('https://api.example.com')
       return response.json()
   
   data = asyncio.run(fetch_data())

Context Detection
~~~~~~~~~~~~~~~~

RequestX automatically detects whether you're in a sync or async context:

.. code-block:: python

   import requestx
   
   def sync_function():
       # Automatically runs synchronously
       return requestx.get('https://api.example.com')
   
   async def async_function():
       # Automatically runs asynchronously
       return await requestx.get('https://api.example.com')

Better Performance
~~~~~~~~~~~~~~~~~

Same code, better performance:

.. code-block:: python

   import requestx
   import time
   
   # Make 100 requests - much faster than requests!
   start = time.time()
   session = requestx.Session()
   
   for i in range(100):
       response = session.get(f'https://httpbin.org/get?id={i}')
   
   print(f"Completed in {time.time() - start:.2f} seconds")

Migration Checklist
------------------

Use this checklist to ensure a smooth migration:

**✅ Basic Migration**
   - [ ] Replace ``import requests`` with ``import requestx as requests``
   - [ ] Test basic GET/POST requests
   - [ ] Verify response handling works
   - [ ] Check error handling

**✅ Advanced Features**
   - [ ] Test session usage
   - [ ] Verify authentication methods
   - [ ] Check file upload functionality
   - [ ] Test timeout and retry logic

**✅ Performance Testing**
   - [ ] Benchmark critical request paths
   - [ ] Test with your typical request volumes
   - [ ] Verify memory usage improvements
   - [ ] Check connection pooling behavior

**✅ Async Migration (Optional)**
   - [ ] Identify I/O-bound request code
   - [ ] Convert to async/await where beneficial
   - [ ] Test concurrent request patterns
   - [ ] Measure async performance improvements

Common Migration Issues
----------------------

Here are solutions to common issues you might encounter:

Import Conflicts
~~~~~~~~~~~~~~~

If you have both ``requests`` and ``requestx`` installed:

.. code-block:: python

   # Option 1: Use alias
   import requestx as requests
   
   # Option 2: Import specific functions
   from requestx import get, post, Session
   
   # Option 3: Use full module name
   import requestx
   response = requestx.get(url)

Third-Party Library Compatibility
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Some libraries expect the ``requests`` module specifically:

.. code-block:: python

   # If a library does: import requests
   # You can monkey-patch it:
   import sys
   import requestx
   sys.modules['requests'] = requestx
   
   # Now the library will use RequestX instead

Custom Adapters
~~~~~~~~~~~~~~

If you use custom ``requests`` adapters, you'll need to rewrite them for RequestX's architecture. Contact us for migration assistance.

Testing Your Migration
---------------------

Create a simple test to verify your migration:

.. code-block:: python

   import requestx as requests
   
   def test_migration():
       """Test that RequestX works as a drop-in replacement"""
       
       # Test basic GET
       response = requests.get('https://httpbin.org/get')
       assert response.status_code == 200
       assert 'args' in response.json()
       
       # Test POST with JSON
       data = {'test': 'data'}
       response = requests.post('https://httpbin.org/post', json=data)
       assert response.status_code == 200
       assert response.json()['json'] == data
       
       # Test headers
       headers = {'Custom-Header': 'test-value'}
       response = requests.get('https://httpbin.org/headers', headers=headers)
       assert 'Custom-Header' in response.json()['headers']
       
       # Test session
       session = requests.Session()
       session.headers.update({'Session-Header': 'session-value'})
       response = session.get('https://httpbin.org/headers')
       assert 'Session-Header' in response.json()['headers']
       
       print("✅ Migration test passed!")
   
   if __name__ == '__main__':
       test_migration()

Performance Comparison
--------------------

Here's a simple script to compare performance:

.. code-block:: python

   import time
   import requests as old_requests
   import requestx as new_requests
   
   def benchmark_library(lib, name, url, count=10):
       """Benchmark a requests library"""
       start = time.time()
       session = lib.Session()
       
       for i in range(count):
           response = session.get(f'{url}?id={i}')
           assert response.status_code == 200
       
       duration = time.time() - start
       print(f"{name}: {duration:.2f}s ({count/duration:.1f} req/s)")
       return duration
   
   # Compare performance
   url = 'https://httpbin.org/get'
   count = 50
   
   old_time = benchmark_library(old_requests, "requests", url, count)
   new_time = benchmark_library(new_requests, "requestx", url, count)
   
   improvement = (old_time - new_time) / old_time * 100
   print(f"\nRequestX is {improvement:.1f}% faster!")

Gradual Migration Strategy
-------------------------

For large codebases, consider a gradual migration:

**Phase 1: Install and Test**
   1. Install RequestX alongside requests
   2. Create a test module using RequestX
   3. Verify compatibility with your use cases

**Phase 2: Module-by-Module**
   1. Start with non-critical modules
   2. Replace imports one module at a time
   3. Test thoroughly after each change

**Phase 3: Performance-Critical Code**
   1. Identify bottlenecks in your HTTP code
   2. Migrate these areas first for immediate benefits
   3. Consider adding async/await for I/O-bound operations

**Phase 4: Complete Migration**
   1. Migrate remaining modules
   2. Remove requests dependency
   3. Optimize for RequestX-specific features

Getting Help
-----------

If you encounter issues during migration:

* **GitHub Issues**: https://github.com/neuesql/requestx/issues
* **Migration Guide**: This document
* **API Reference**: :doc:`api/index`
* **Examples**: :doc:`examples/index`

We're committed to making migration as smooth as possible. If you find compatibility issues, please report them so we can address them quickly.