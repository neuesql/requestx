Exceptions
==========

.. exception:: RequestException

   There was an ambiguous exception that occurred while handling your request.

.. exception:: HTTPError

   An HTTP error occurred.

.. exception:: ConnectionError

   A Connection error occurred.

.. exception:: ProxyError

   A proxy error occurred.

.. exception:: SSLError

   An SSL error occurred.

.. exception:: Timeout

   The request timed out.

.. exception:: ConnectTimeout

   The request timed out while trying to connect to the remote server.

.. exception:: ReadTimeout

   The server did not send any data in the allotted amount of time.

.. exception:: URLRequired

   A valid URL is required to make a request.

.. exception:: TooManyRedirects

   Too many redirects.

.. exception:: MissingSchema

   The URL schema (e.g. http or https) is missing.

.. exception:: InvalidSchema

   See defaults.py for valid schemas.

.. exception:: InvalidURL

   The URL provided was somehow invalid.

.. exception:: InvalidHeader

   The header value provided was somehow invalid.

.. exception:: ChunkedEncodingError

   The server declared chunked encoding but sent an invalid chunk.

.. exception:: ContentDecodingError

   Failed to decode response content.

.. exception:: StreamConsumedError

   The content for this response was already consumed.

.. exception:: RetryError

   Custom retries logic failed.

.. exception:: UnrewindableBodyError

   RequestX encountered an error when trying to rewind a body.
