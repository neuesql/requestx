HTTP Functions
==============

RequestX provides top-level functions for making HTTP requests, matching the ``requests`` API.

.. function:: get(url, params=None, **kwargs)

   Sends a GET request.
   :param url: URL for the new :class:`Request` object.
   :param params: (optional) Dictionary, list of tuples or bytes to send in the query string for the :class:`Request`.
   :param \*\*kwargs: Optional arguments that ``request`` takes.
   :return: :class:`Response <Response>` object.
   :rtype: requestx.Response

.. function:: post(url, data=None, json=None, **kwargs)

   Sends a POST request.
   :param url: URL for the new :class:`Request` object.
   :param data: (optional) Dictionary, list of tuples, bytes, or file-like object to send in the body of the :class:`Request`.
   :param json: (optional) A JSON serializable Python object to send in the body of the :class:`Request`.
   :param \*\*kwargs: Optional arguments that ``request`` takes.
   :return: :class:`Response <Response>` object.
   :rtype: requestx.Response

.. function:: put(url, data=None, **kwargs)

   Sends a PUT request.
   :param url: URL for the new :class:`Request` object.
   :param data: (optional) Dictionary, list of tuples, bytes, or file-like object to send in the body of the :class:`Request`.
   :param \*\*kwargs: Optional arguments that ``request`` takes.
   :return: :class:`Response <Response>` object.
   :rtype: requestx.Response

.. function:: delete(url, **kwargs)

   Sends a DELETE request.
   :param url: URL for the new :class:`Request` object.
   :param \*\*kwargs: Optional arguments that ``request`` takes.
   :return: :class:`Response <Response>` object.
   :rtype: requestx.Response

.. function:: head(url, **kwargs)

   Sends a HEAD request.
   :param url: URL for the new :class:`Request` object.
   :param \*\*kwargs: Optional arguments that ``request`` takes.
   :return: :class:`Response <Response>` object.
   :rtype: requestx.Response

.. function:: options(url, **kwargs)

   Sends an OPTIONS request.
   :param url: URL for the new :class:`Request` object.
   :param \*\*kwargs: Optional arguments that ``request`` takes.
   :return: :class:`Response <Response>` object.
   :rtype: requestx.Response

.. function:: patch(url, data=None, **kwargs)

   Sends a PATCH request.
   :param url: URL for the new :class:`Request` object.
   :param data: (optional) Dictionary, list of tuples, bytes, or file-like object to send in the body of the :class:`Request`.
   :param \*\*kwargs: Optional arguments that ``request`` takes.
   :return: :class:`Response <Response>` object.
   :rtype: requestx.Response
