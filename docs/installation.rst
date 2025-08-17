Installation Guide
=================

RequestX is designed to be easy to install and use across all major platforms.

Requirements
-----------

* **Python**: 3.8 or higher
* **Operating System**: Windows, macOS, or Linux
* **Architecture**: x86_64, ARM64 (Apple Silicon, ARM64 Windows)

No additional dependencies or build tools are required - RequestX comes with all Rust dependencies pre-compiled and bundled.

Standard Installation
--------------------

Install RequestX using pip:

.. code-block:: bash

   pip install requestx

This will install the latest stable version from PyPI with pre-built wheels for your platform.

Development Installation
-----------------------

If you want to install the latest development version from GitHub:

.. code-block:: bash

   pip install git+https://github.com/neuesql/requestx.git

Virtual Environment Installation
-------------------------------

It's recommended to install RequestX in a virtual environment:

.. code-block:: bash

   # Create virtual environment
   python -m venv requestx-env
   
   # Activate virtual environment
   # On Windows:
   requestx-env\\Scripts\\activate
   # On macOS/Linux:
   source requestx-env/bin/activate
   
   # Install RequestX
   pip install requestx

Using uv (Recommended)
---------------------

For faster installation and better dependency management, use `uv <https://github.com/astral-sh/uv>`_:

.. code-block:: bash

   # Install uv if you haven't already
   curl -LsSf https://astral.sh/uv/install.sh | sh
   
   # Create project with RequestX
   uv init my-project
   cd my-project
   uv add requestx
   
   # Run your code
   uv run python your_script.py

Platform-Specific Notes
-----------------------

Windows
~~~~~~~

RequestX works on all supported Windows versions:

* Windows 10 and 11 (x86_64 and ARM64)
* Windows Server 2019 and 2022

.. code-block:: cmd

   # Standard installation
   pip install requestx
   
   # Or using conda
   conda install -c conda-forge requestx

macOS
~~~~~

RequestX provides universal wheels that work on both Intel and Apple Silicon Macs:

* macOS 11.0 (Big Sur) and later
* Both x86_64 (Intel) and ARM64 (Apple Silicon) architectures

.. code-block:: bash

   # Standard installation
   pip install requestx
   
   # Using Homebrew Python
   /opt/homebrew/bin/pip3 install requestx

Linux
~~~~~

RequestX supports all major Linux distributions:

* Ubuntu 20.04 LTS and later
* CentOS/RHEL 8 and later  
* Debian 11 and later
* Alpine Linux 3.15 and later
* Both x86_64 and ARM64 architectures

.. code-block:: bash

   # Standard installation
   pip install requestx
   
   # On older systems, you might need to upgrade pip first
   pip install --upgrade pip
   pip install requestx

Docker Installation
------------------

Use RequestX in Docker containers:

.. code-block:: dockerfile

   FROM python:3.11-slim

   # Install RequestX
   RUN pip install requestx

   # Copy your application
   COPY . /app
   WORKDIR /app

   # Run your application
   CMD ["python", "app.py"]

Or with a multi-stage build for smaller images:

.. code-block:: dockerfile

   FROM python:3.11-slim as builder

   # Install RequestX
   RUN pip install --user requestx

   FROM python:3.11-slim
   
   # Copy installed packages
   COPY --from=builder /root/.local /root/.local
   
   # Make sure scripts in .local are usable
   ENV PATH=/root/.local/bin:$PATH
   
   # Copy your application
   COPY . /app
   WORKDIR /app
   
   CMD ["python", "app.py"]

Verification
-----------

Verify your installation by running:

.. code-block:: python

   import requestx
   
   # Check version
   print(f"RequestX version: {requestx.__version__}")
   
   # Make a test request
   response = requestx.get('https://httpbin.org/json')
   print(f"Status: {response.status_code}")
   print("Installation successful!")

You should see output similar to:

.. code-block:: text

   RequestX version: 0.2.0
   Status: 200
   Installation successful!

Troubleshooting
--------------

Installation Issues
~~~~~~~~~~~~~~~~~~

If you encounter installation issues:

1. **Upgrade pip**: ``pip install --upgrade pip``
2. **Clear pip cache**: ``pip cache purge``
3. **Use --no-cache-dir**: ``pip install --no-cache-dir requestx``
4. **Check Python version**: ``python --version`` (must be 3.8+)

Import Issues
~~~~~~~~~~~~

If you get import errors:

.. code-block:: python

   # Check if RequestX is properly installed
   import sys
   print(sys.path)
   
   try:
       import requestx
       print("RequestX imported successfully")
   except ImportError as e:
       print(f"Import error: {e}")

Performance Issues
~~~~~~~~~~~~~~~~~

If RequestX seems slower than expected:

1. **Use sessions** for multiple requests
2. **Enable connection pooling** 
3. **Check your network connection**
4. **Use async/await** for concurrent requests

.. code-block:: python

   import requestx
   import time
   
   # Test performance
   start = time.time()
   response = requestx.get('https://httpbin.org/json')
   end = time.time()
   
   print(f"Request took: {end - start:.3f} seconds")
   print(f"Status: {response.status_code}")

Getting Help
-----------

If you need help with installation:

* **GitHub Issues**: https://github.com/neuesql/requestx/issues
* **Discussions**: https://github.com/neuesql/requestx/discussions
* **Documentation**: https://requestx.readthedocs.io

When reporting issues, please include:

* Your operating system and version
* Python version (``python --version``)
* RequestX version (``pip show requestx``)
* Full error message and traceback
* Steps to reproduce the issue

Uninstallation
-------------

To uninstall RequestX:

.. code-block:: bash

   pip uninstall requestx

This will remove RequestX and its bundled dependencies, but won't affect other packages in your environment.