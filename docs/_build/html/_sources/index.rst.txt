RequestX Documentation
=====================

.. image:: https://img.shields.io/pypi/v/requestx.svg
   :target: https://pypi.org/project/requestx/
   :alt: PyPI version

.. image:: https://img.shields.io/pypi/pyversions/requestx.svg
   :target: https://pypi.org/project/requestx/
   :alt: Python versions

.. image:: https://github.com/neuesql/requestx/workflows/Test%20and%20Build/badge.svg
   :target: https://github.com/neuesql/requestx/actions
   :alt: Build status

.. image:: https://img.shields.io/badge/code%20style-black-000000.svg
   :target: https://github.com/psf/black
   :alt: Code style: black

RequestX is a high-performance HTTP client library for Python that provides a **drop-in replacement** for the popular ``requests`` library. Built with Rust for speed and memory safety, it offers both synchronous and asynchronous APIs while maintaining full compatibility with the familiar requests interface.

🚀 **Key Features**
------------------

* **Drop-in replacement** for requests library with identical API
* **High performance** leveraging Rust's speed and memory safety  
* **Dual API support** - both sync and async/await patterns
* **Cross-platform** compatibility (Windows, macOS, Linux)
* **Requests compatibility** for easy migration from existing codebases
* **Native async/await** support with automatic context detection
* **Session management** with persistent connections and cookies
* **Comprehensive error handling** with requests-compatible exceptions

⚡ **Performance**
-----------------

RequestX delivers significant performance improvements over traditional Python HTTP libraries:

* **2-5x faster** than requests for synchronous operations
* **3-10x faster** than aiohttp for asynchronous operations  
* **Lower memory usage** due to Rust's efficient memory management
* **Better connection pooling** with hyper's advanced HTTP/2 support

📦 **Quick Installation**
------------------------

.. code-block:: bash

   pip install requestx

🔥 **Quick Start**
-----------------

RequestX works exactly like requests - just change the import:

.. code-block:: python

   # Before (requests)
   import requests
   
   response = requests.get('https://httpbin.org/json')
   print(response.json())

   # After (requestx) - same API!
   import requestx
   
   response = requestx.get('https://httpbin.org/json')
   print(response.json())

**Async/await support:**

.. code-block:: python

   import asyncio
   import requestx

   async def main():
       # Same API, but now works in async context!
       response = await requestx.get('https://httpbin.org/json')
       print(response.json())

   asyncio.run(main())

📚 **Documentation Contents**
----------------------------

.. toctree::
   :maxdepth: 2
   :caption: User Guide

   quickstart
   installation
   user-guide/index
   migration
   async-guide
   performance

.. toctree::
   :maxdepth: 2
   :caption: API Reference

   api/index
   api/functions
   api/response
   api/session
   api/exceptions

.. toctree::
   :maxdepth: 2
   :caption: Examples

   examples/index
   examples/basic-usage
   examples/async-usage
   examples/sessions
   examples/advanced

.. toctree::
   :maxdepth: 2
   :caption: Development

   contributing
   changelog
   benchmarks

🤝 **Community & Support**
--------------------------

* **GitHub**: https://github.com/neuesql/requestx
* **Issues**: https://github.com/neuesql/requestx/issues
* **Discussions**: https://github.com/neuesql/requestx/discussions

📄 **License**
-------------

RequestX is released under the MIT License. See the `LICENSE <https://github.com/neuesql/requestx/blob/main/LICENSE>`_ file for details.

Indices and tables
==================

* :ref:`genindex`
* :ref:`modindex`
* :ref:`search`