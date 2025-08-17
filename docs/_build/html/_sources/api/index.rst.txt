API Reference
=============

This section contains the complete API reference for RequestX.

.. toctree::
   :maxdepth: 2

   functions
   response
   session
   exceptions

Overview
--------

RequestX provides a simple, intuitive API that's fully compatible with the ``requests`` library. The API is organized into several main components:

**HTTP Functions**
   Top-level functions for making HTTP requests: ``get()``, ``post()``, ``put()``, etc.

**Response Objects**
   The ``Response`` class that represents HTTP responses with methods like ``json()``, ``text``, etc.

**Session Objects**
   The ``Session`` class for persistent connections, cookies, and configuration.

**Exceptions**
   Exception classes for handling various error conditions.

Quick Reference
--------------

**Making Requests**

.. code-block:: python

   import requestx
   
   # Basic requests
   response = requestx.get(url, **kwargs)
   response = requestx.post(url, data=None, json=None, **kwargs)
   response = requestx.put(url, data=None, **kwargs)
   response = requestx.patch(url, data=None, **kwargs)
   response = requestx.delete(url, **kwargs)
   response = requestx.head(url, **kwargs)
   response = requestx.options(url, **kwargs)

**Common Parameters**

.. code-block:: python

   requestx.get(
       url,
       params=None,          # URL parameters
       headers=None,         # HTTP headers
       cookies=None,         # Cookies
       auth=None,           # Authentication
       timeout=None,        # Request timeout
       allow_redirects=True, # Follow redirects
       proxies=None,        # Proxy configuration
       verify=True,         # SSL verification
       stream=False,        # Stream response
       cert=None           # Client certificate
   )

**Response Properties**

.. code-block:: python

   response.status_code     # HTTP status code
   response.headers         # Response headers
   response.text           # Response text
   response.content        # Response bytes
   response.json()         # Parse JSON response
   response.url            # Final URL
   response.cookies        # Response cookies
   response.history        # Redirect history

**Session Usage**

.. code-block:: python

   session = requestx.Session()
   session.headers.update({'Authorization': 'Bearer token'})
   response = session.get(url)

**Error Handling**

.. code-block:: python

   from requestx import RequestException, HTTPError, ConnectionError, Timeout
   
   try:
       response = requestx.get(url, timeout=10)
       response.raise_for_status()
   except HTTPError as e:
       # Handle HTTP errors (4xx, 5xx)
       pass
   except ConnectionError as e:
       # Handle connection errors
       pass
   except Timeout as e:
       # Handle timeout errors
       pass
   except RequestException as e:
       # Handle all request errors
       pass

Async/Await Support
------------------

All functions and methods support async/await with automatic context detection:

.. code-block:: python

   import asyncio
   import requestx
   
   # Synchronous usage
   response = requestx.get('https://api.example.com')
   
   # Asynchronous usage (same API!)
   async def fetch_data():
       response = await requestx.get('https://api.example.com')
       return response.json()
   
   data = asyncio.run(fetch_data())

Type Hints
---------

RequestX includes comprehensive type hints for better IDE support:

.. code-block:: python

   from typing import Optional, Dict, Any
   import requestx
   
   def fetch_json(url: str, headers: Optional[Dict[str, str]] = None) -> Dict[str, Any]:
       response: requestx.Response = requestx.get(url, headers=headers)
       response.raise_for_status()
       return response.json()

Compatibility
------------

RequestX maintains 100% API compatibility with the ``requests`` library for all documented features. This means:

* All function signatures are identical
* All response properties and methods work the same way
* All exception types and hierarchies are preserved
* All session functionality is compatible

The only difference is improved performance and native async/await support.

Module Structure
---------------

.. code-block:: python

   requestx/
   ├── __init__.py          # Main module with public API
   ├── _requestx            # Compiled Rust extension
   ├── exceptions.py        # Exception classes
   └── models.py           # Response and Session classes

For detailed information about each component, see the individual API reference pages.