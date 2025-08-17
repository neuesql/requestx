Basic Usage Examples
===================

This page contains examples of basic RequestX usage patterns that work exactly like the ``requests`` library.

Simple GET Request
-----------------

The most basic HTTP request:

.. code-block:: python

   import requestx
   
   # Make a simple GET request
   response = requestx.get('https://httpbin.org/json')
   
   print(f"Status Code: {response.status_code}")
   print(f"Content-Type: {response.headers['content-type']}")
   print(f"Response: {response.json()}")

Output:

.. code-block:: text

   Status Code: 200
   Content-Type: application/json
   Response: {'slideshow': {'author': 'Yours Truly', 'date': 'date of publication', ...}}

GET with Parameters
------------------

Adding URL parameters to your requests:

.. code-block:: python

   import requestx
   
   # URL parameters as a dictionary
   params = {
       'q': 'python requests',
       'sort': 'stars',
       'order': 'desc'
   }
   
   response = requestx.get('https://httpbin.org/get', params=params)
   data = response.json()
   
   print(f"Final URL: {response.url}")
   print(f"Parameters received: {data['args']}")

Output:

.. code-block:: text

   Final URL: https://httpbin.org/get?q=python+requests&sort=stars&order=desc
   Parameters received: {'q': 'python requests', 'sort': 'stars', 'order': 'desc'}

POST with Form Data
------------------

Sending form data with POST requests:

.. code-block:: python

   import requestx
   
   # Form data
   form_data = {
       'username': 'john_doe',
       'email': 'john@example.com',
       'age': '30'
   }
   
   response = requestx.post('https://httpbin.org/post', data=form_data)
   result = response.json()
   
   print(f"Status: {response.status_code}")
   print(f"Form data received: {result['form']}")

POST with JSON Data
------------------

Sending JSON data in POST requests:

.. code-block:: python

   import requestx
   
   # JSON data
   user_data = {
       'name': 'Alice Smith',
       'email': 'alice@example.com',
       'preferences': {
           'theme': 'dark',
           'notifications': True
       }
   }
   
   response = requestx.post('https://httpbin.org/post', json=user_data)
   result = response.json()
   
   print(f"Status: {response.status_code}")
   print(f"JSON data received: {result['json']}")
   print(f"Content-Type sent: {result['headers']['Content-Type']}")

Custom Headers
-------------

Adding custom headers to requests:

.. code-block:: python

   import requestx
   
   # Custom headers
   headers = {
       'User-Agent': 'RequestX-Example/1.0',
       'Authorization': 'Bearer your-api-token',
       'Accept': 'application/json',
       'X-Custom-Header': 'custom-value'
   }
   
   response = requestx.get('https://httpbin.org/headers', headers=headers)
   received_headers = response.json()['headers']
   
   print("Headers sent:")
   for key, value in headers.items():
       if key in received_headers:
           print(f"  {key}: {received_headers[key]}")

Working with Response Data
-------------------------

Different ways to access response data:

.. code-block:: python

   import requestx
   
   response = requestx.get('https://httpbin.org/json')
   
   # Status code
   print(f"Status Code: {response.status_code}")
   print(f"Status OK: {response.status_code == 200}")
   
   # Headers
   print(f"Content-Type: {response.headers['content-type']}")
   print(f"Content-Length: {response.headers.get('content-length', 'Not specified')}")
   
   # Response body as text
   print(f"Response text (first 100 chars): {response.text[:100]}...")
   
   # Response body as bytes
   print(f"Response size: {len(response.content)} bytes")
   
   # Parse JSON
   if 'application/json' in response.headers.get('content-type', ''):
       data = response.json()
       print(f"JSON keys: {list(data.keys())}")

HTTP Methods
-----------

Examples of different HTTP methods:

.. code-block:: python

   import requestx
   
   base_url = 'https://httpbin.org'
   
   # GET request
   get_response = requestx.get(f'{base_url}/get')
   print(f"GET: {get_response.status_code}")
   
   # POST request
   post_data = {'key': 'value'}
   post_response = requestx.post(f'{base_url}/post', json=post_data)
   print(f"POST: {post_response.status_code}")
   
   # PUT request
   put_response = requestx.put(f'{base_url}/put', json={'update': 'data'})
   print(f"PUT: {put_response.status_code}")
   
   # PATCH request
   patch_response = requestx.patch(f'{base_url}/patch', json={'patch': 'data'})
   print(f"PATCH: {patch_response.status_code}")
   
   # DELETE request
   delete_response = requestx.delete(f'{base_url}/delete')
   print(f"DELETE: {delete_response.status_code}")
   
   # HEAD request (no response body)
   head_response = requestx.head(f'{base_url}/get')
   print(f"HEAD: {head_response.status_code}, Content-Length: {head_response.headers.get('content-length')}")
   
   # OPTIONS request
   options_response = requestx.options(f'{base_url}/get')
   print(f"OPTIONS: {options_response.status_code}")

Basic Authentication
-------------------

Using HTTP Basic Authentication:

.. code-block:: python

   import requestx
   
   # Basic authentication
   username = 'user'
   password = 'pass'
   
   response = requestx.get('https://httpbin.org/basic-auth/user/pass', 
                          auth=(username, password))
   
   if response.status_code == 200:
       result = response.json()
       print(f"Authenticated: {result['authenticated']}")
       print(f"User: {result['user']}")
   else:
       print(f"Authentication failed: {response.status_code}")

Handling Cookies
---------------

Working with cookies in requests:

.. code-block:: python

   import requestx
   
   # Send cookies with request
   cookies = {
       'session_id': 'abc123',
       'user_preference': 'dark_theme'
   }
   
   response = requestx.get('https://httpbin.org/cookies', cookies=cookies)
   received_cookies = response.json()['cookies']
   
   print("Cookies sent:")
   for name, value in received_cookies.items():
       print(f"  {name}: {value}")
   
   # Access cookies from response
   # (httpbin.org doesn't set cookies, but here's how you would access them)
   if response.cookies:
       print("\nCookies received:")
       for cookie in response.cookies:
           print(f"  {cookie.name}: {cookie.value}")

Timeout Configuration
--------------------

Setting request timeouts:

.. code-block:: python

   import requestx
   from requestx import Timeout
   
   try:
       # Timeout after 5 seconds
       response = requestx.get('https://httpbin.org/delay/3', timeout=5)
       print(f"Request completed: {response.status_code}")
   except Timeout:
       print("Request timed out!")
   
   try:
       # This will timeout (delay is longer than timeout)
       response = requestx.get('https://httpbin.org/delay/10', timeout=5)
       print(f"Request completed: {response.status_code}")
   except Timeout:
       print("Request timed out as expected!")

Error Handling
-------------

Proper error handling for HTTP requests:

.. code-block:: python

   import requestx
   from requestx import RequestException, HTTPError, ConnectionError, Timeout
   
   def safe_request(url, **kwargs):
       """Make a request with comprehensive error handling"""
       try:
           response = requestx.get(url, **kwargs)
           
           # Check for HTTP errors (4xx, 5xx status codes)
           response.raise_for_status()
           
           return response.json()
           
       except HTTPError as e:
           print(f"HTTP Error {e.response.status_code}: {e}")
           return None
       except ConnectionError as e:
           print(f"Connection Error: {e}")
           return None
       except Timeout as e:
           print(f"Timeout Error: {e}")
           return None
       except RequestException as e:
           print(f"Request Error: {e}")
           return None
   
   # Test with different scenarios
   print("Testing successful request:")
   data = safe_request('https://httpbin.org/json')
   if data:
       print(f"Success: {list(data.keys())}")
   
   print("\nTesting 404 error:")
   data = safe_request('https://httpbin.org/status/404')
   
   print("\nTesting timeout:")
   data = safe_request('https://httpbin.org/delay/10', timeout=2)

File Upload
----------

Uploading files with POST requests:

.. code-block:: python

   import requestx
   import io
   
   # Create a sample file in memory
   file_content = "This is a test file content.\nLine 2 of the file."
   file_obj = io.StringIO(file_content)
   
   # Upload file
   files = {
       'file': ('test.txt', file_obj, 'text/plain')
   }
   
   response = requestx.post('https://httpbin.org/post', files=files)
   result = response.json()
   
   print(f"Status: {response.status_code}")
   print(f"Files received: {list(result['files'].keys())}")
   print(f"File content: {result['files']['file']}")

Query String Building
--------------------

Different ways to build query strings:

.. code-block:: python

   import requestx
   
   # Method 1: Dictionary
   params1 = {'q': 'python', 'sort': 'stars'}
   response1 = requestx.get('https://httpbin.org/get', params=params1)
   
   # Method 2: List of tuples (allows duplicate keys)
   params2 = [('tag', 'python'), ('tag', 'web'), ('sort', 'date')]
   response2 = requestx.get('https://httpbin.org/get', params=params2)
   
   # Method 3: Pre-built query string
   url_with_params = 'https://httpbin.org/get?q=python&sort=stars'
   response3 = requestx.get(url_with_params)
   
   print(f"Method 1 URL: {response1.url}")
   print(f"Method 2 URL: {response2.url}")
   print(f"Method 3 URL: {response3.url}")

Response Status Checking
-----------------------

Different ways to check response status:

.. code-block:: python

   import requestx
   
   def check_response_status(url):
       response = requestx.get(url)
       
       print(f"URL: {url}")
       print(f"Status Code: {response.status_code}")
       
       # Check specific status codes
       if response.status_code == 200:
           print("✓ Success")
       elif response.status_code == 404:
           print("✗ Not Found")
       elif response.status_code >= 400:
           print("✗ Client/Server Error")
       
       # Check status ranges
       if 200 <= response.status_code < 300:
           print("✓ Success range")
       elif 300 <= response.status_code < 400:
           print("→ Redirect range")
       elif 400 <= response.status_code < 500:
           print("✗ Client error range")
       elif response.status_code >= 500:
           print("✗ Server error range")
       
       # Use raise_for_status() for automatic error handling
       try:
           response.raise_for_status()
           print("✓ No HTTP errors")
       except requestx.HTTPError as e:
           print(f"✗ HTTP Error: {e}")
       
       print("-" * 40)
   
   # Test different status codes
   check_response_status('https://httpbin.org/status/200')
   check_response_status('https://httpbin.org/status/404')
   check_response_status('https://httpbin.org/status/500')

Working with Different Content Types
-----------------------------------

Handling various response content types:

.. code-block:: python

   import requestx
   import json
   
   def handle_response_content(url, expected_type=None):
       response = requestx.get(url)
       content_type = response.headers.get('content-type', '').lower()
       
       print(f"URL: {url}")
       print(f"Content-Type: {content_type}")
       
       if 'application/json' in content_type:
           try:
               data = response.json()
               print(f"JSON data: {type(data)} with {len(data)} items")
           except json.JSONDecodeError:
               print("Failed to parse JSON")
       
       elif 'text/html' in content_type:
           print(f"HTML content: {len(response.text)} characters")
           print(f"Title found: {'<title>' in response.text}")
       
       elif 'text/plain' in content_type:
           print(f"Plain text: {len(response.text)} characters")
           print(f"First line: {response.text.split('\\n')[0]}")
       
       elif 'image/' in content_type:
           print(f"Image data: {len(response.content)} bytes")
       
       else:
           print(f"Other content type: {len(response.content)} bytes")
       
       print("-" * 40)
   
   # Test different content types
   handle_response_content('https://httpbin.org/json')  # JSON
   handle_response_content('https://httpbin.org/html')  # HTML
   handle_response_content('https://httpbin.org/robots.txt')  # Plain text

This covers the fundamental usage patterns of RequestX. These examples work exactly the same as they would with the ``requests`` library, but with better performance thanks to RequestX's Rust implementation.