Response Object
===============

.. class:: Response

   The :class:`Response <Response>` object, which contains a server's response to an HTTP request.

   .. attribute:: status_code
      Integer Code of responded HTTP Status, e.g. 404 or 200.

   .. attribute:: reason
      Textual reason of responded HTTP Status, e.g. "Not Found" or "OK".

   .. attribute:: ok
      Returns True if :attr:`status_code` is less than 400, False if not.

   .. attribute:: headers
      Case-insensitive Dictionary of Response Headers.

   .. attribute:: text
      Content of the response, in unicode.

   .. attribute:: content
      Content of the response, in bytes.

   .. attribute:: url
      Final URL location of Response.

   .. attribute:: encoding
      Encoding to decode with when accessing :attr:`text`.

   .. attribute:: history
      A list of :class:`Response` objects from the history of the request. Any redirect responses will end up here. The list is sorted from the oldest to the most recent request.

   .. method:: json()
      Returns the json-encoded content of a response, if any.
      :raises requestx.exceptions.JSONDecodeError: If the response body does not contain valid json.

   .. method:: raise_for_status()
      Raises :class:`HTTPError`, if one occurred.
