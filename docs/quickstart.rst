Quick Start Guide
================

This guide will get you up and running with RequestX in just a few minutes.

Installation
-----------

Install RequestX using pip:

.. code-block:: bash

   pip install requestx

That's it! RequestX comes with all dependencies bundled, so no additional setup is required.

Basic Usage
----------

RequestX provides the exact same API as the popular ``requests`` library. If you're familiar with requests, you already know how to use RequestX!

Making Your First Request
~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: python

   import requestx

   # Make a simple GET request
   response = requestx.get('https://httpbin.org/json')
   
   # Check the status
   print(f"Status: {response.status_code}")
   
   # Get JSON data
   data = response.json()
   print(f"Data: {data}")

Common HTTP Methods
~~~~~~~~~~~~~~~~~~

RequestX supports all standard HTTP methods:

.. code-block:: python

   import requestx

   # GET request
   response = requestx.get('https://httpbin.org/get')

   # POST request with JSON data
   response = requestx.post('https://httpbin.org/post', json={'key': 'value'})

   # PUT request with data
   response = requestx.put('https://httpbin.org/put', data='some data')

   # DELETE request
   response = requestx.delete('https://httpbin.org/delete')

   # HEAD request
   response = requestx.head('https://httpbin.org/get')

   # OPTIONS request
   response = requestx.options('https://httpbin.org/get')

   # PATCH request
   response = requestx.patch('https://httpbin.org/patch', json={'update': 'value'})

Working with Parameters
~~~~~~~~~~~~~~~~~~~~~~

Add URL parameters using the ``params`` argument:

.. code-block:: python

   import requestx

   # URL parameters
   params = {'key1': 'value1', 'key2': 'value2'}
   response = requestx.get('https://httpbin.org/get', params=params)
   
   # This makes a request to: https://httpbin.org/get?key1=value1&key2=value2
   print(response.url)

Sending Data
~~~~~~~~~~~

Send data in various formats:

.. code-block:: python

   import requestx

   # Send form data
   data = {'username': 'user', 'password': 'pass'}
   response = requestx.post('https://httpbin.org/post', data=data)

   # Send JSON data
   json_data = {'name': 'John', 'age': 30}
   response = requestx.post('https://httpbin.org/post', json=json_data)

   # Send raw data
   response = requestx.post('https://httpbin.org/post', data='raw string data')

Custom Headers
~~~~~~~~~~~~~

Add custom headers to your requests:

.. code-block:: python

   import requestx

   headers = {
       'User-Agent': 'RequestX/1.0',
       'Authorization': 'Bearer your-token-here',
       'Content-Type': 'application/json'
   }
   
   response = requestx.get('https://httpbin.org/headers', headers=headers)

Response Handling
~~~~~~~~~~~~~~~~

Work with response data:

.. code-block:: python

   import requestx

   response = requestx.get('https://httpbin.org/json')

   # Status code
   print(f"Status: {response.status_code}")

   # Response headers
   print(f"Content-Type: {response.headers['content-type']}")

   # Text content
   print(f"Text: {response.text}")

   # JSON content (if applicable)
   if response.headers.get('content-type', '').startswith('application/json'):
       data = response.json()
       print(f"JSON: {data}")

   # Raw bytes
   print(f"Content length: {len(response.content)} bytes")

Error Handling
~~~~~~~~~~~~~

Handle errors gracefully:

.. code-block:: python

   import requestx
   from requestx import RequestException, HTTPError, ConnectionError, Timeout

   try:
       response = requestx.get('https://httpbin.org/status/404', timeout=5)
       response.raise_for_status()  # Raises HTTPError for 4xx/5xx status codes
   except HTTPError as e:
       print(f"HTTP Error: {e}")
   except ConnectionError as e:
       print(f"Connection Error: {e}")
   except Timeout as e:
       print(f"Timeout Error: {e}")
   except RequestException as e:
       print(f"Request Error: {e}")

Async/Await Support
~~~~~~~~~~~~~~~~~~

RequestX automatically detects async contexts and works seamlessly with async/await:

.. code-block:: python

   import asyncio
   import requestx

   async def fetch_data():
       # Same API, but automatically works in async context!
       response = await requestx.get('https://httpbin.org/json')
       return response.json()

   async def main():
       data = await fetch_data()
       print(f"Received: {data}")

   # Run the async function
   asyncio.run(main())

Sessions for Connection Reuse
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Use sessions for better performance when making multiple requests:

.. code-block:: python

   import requestx

   # Create a session
   session = requestx.Session()

   # Set default headers for all requests in this session
   session.headers.update({'User-Agent': 'RequestX-Session/1.0'})

   # Make multiple requests - connections will be reused
   response1 = session.get('https://httpbin.org/get')
   response2 = session.get('https://httpbin.org/json')
   response3 = session.post('https://httpbin.org/post', json={'session': 'data'})

   # Sessions also work with async/await
   async def async_session_example():
       async with requestx.Session() as session:
           response = await session.get('https://httpbin.org/json')
           return response.json()

Migration from Requests
~~~~~~~~~~~~~~~~~~~~~~

If you're migrating from the ``requests`` library, it's as simple as changing the import:

.. code-block:: python

   # Before
   import requests
   
   response = requests.get('https://api.example.com/data')
   data = response.json()

   # After - just change the import!
   import requestx as requests  # Drop-in replacement
   
   response = requests.get('https://api.example.com/data')
   data = response.json()

Or use RequestX directly:

.. code-block:: python

   import requestx

   response = requestx.get('https://api.example.com/data')
   data = response.json()

Next Steps
---------

Now that you've learned the basics, explore more advanced features:

* :doc:`user-guide/index` - Comprehensive user guide
* :doc:`async-guide` - Deep dive into async/await usage
* :doc:`examples/index` - More code examples
* :doc:`api/index` - Complete API reference


Performance Tips
~~~~~~~~~~~~~~~

To get the best performance from RequestX:

1. **Use sessions** for multiple requests to the same host
2. **Enable connection pooling** by reusing session objects
3. **Use async/await** for I/O-bound operations
4. **Set appropriate timeouts** to avoid hanging requests
5. **Consider HTTP/2** for modern APIs (automatically handled by RequestX)

.. code-block:: python

   import requestx

   # Good: Reuse session for multiple requests
   session = requestx.Session()
   for i in range(10):
       response = session.get(f'https://api.example.com/item/{i}')
       process_response(response)

   # Even better: Use async for concurrent requests
   import asyncio

   async def fetch_all():
       session = requestx.Session()
       tasks = []
       for i in range(10):
           task = session.get(f'https://api.example.com/item/{i}')
           tasks.append(task)
       
       responses = await asyncio.gather(*tasks)
       return responses