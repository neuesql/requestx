Session Object
==============

.. class:: Session

   A RequestX session. Provides cookie persistence, connection-pooling, and configuration.

   .. method:: get(url, **kwargs)
      Sends a GET request. Returns :class:`Response` object.

   .. method:: post(url, data=None, json=None, **kwargs)
      Sends a POST request. Returns :class:`Response` object.

   .. method:: put(url, data=None, **kwargs)
      Sends a PUT request. Returns :class:`Response` object.

   .. method:: patch(url, data=None, **kwargs)
      Sends a PATCH request. Returns :class:`Response` object.

   .. method:: delete(url, **kwargs)
      Sends a DELETE request. Returns :class:`Response` object.

   .. method:: head(url, **kwargs)
      Sends a HEAD request. Returns :class:`Response` object.

   .. method:: options(url, **kwargs)
      Sends an OPTIONS request. Returns :class:`Response` object.

   .. method:: request(method, url, **kwargs)
      Constructs a :class:`Request <Request>`, prepares it and sends it. Returns :class:`Response <Response>` object.

   .. method:: close()
      Closes all adapters and as such the session.
