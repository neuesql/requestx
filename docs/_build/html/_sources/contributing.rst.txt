Contributing to RequestX
=======================

We welcome contributions to RequestX! This guide will help you get started with contributing to the project.

Ways to Contribute
------------------

There are many ways to contribute to RequestX:

**Code Contributions**
- Bug fixes
- New features
- Performance improvements
- Documentation improvements
- Test coverage improvements

**Non-Code Contributions**
- Bug reports
- Feature requests
- Documentation feedback
- Performance benchmarks
- Usage examples
- Community support

**Community Contributions**
- Answering questions in discussions
- Writing blog posts or tutorials
- Speaking about RequestX at conferences
- Helping with translations

Getting Started
--------------

Development Setup
~~~~~~~~~~~~~~~~

1. **Fork and Clone**

   .. code-block:: bash

      git clone https://github.com/yourusername/requestx.git
      cd requestx

2. **Install Development Dependencies**

   .. code-block:: bash

      # Install uv for dependency management
      curl -LsSf https://astral.sh/uv/install.sh | sh
      
      # Install development dependencies
      uv sync --dev

3. **Install Rust**

   .. code-block:: bash

      # Install Rust if you haven't already
      curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
      source ~/.cargo/env

4. **Build the Extension**

   .. code-block:: bash

      # Build the Rust extension for development
      uv run maturin develop

5. **Run Tests**

   .. code-block:: bash

      # Run Rust tests
      cargo test
      
      # Run Python tests
      uv run python -m unittest discover tests/ -v

Development Workflow
~~~~~~~~~~~~~~~~~~~

1. **Create a Branch**

   .. code-block:: bash

      git checkout -b feature/your-feature-name
      # or
      git checkout -b fix/your-bug-fix

2. **Make Changes**
   - Write your code
   - Add tests for new functionality
   - Update documentation if needed

3. **Test Your Changes**

   .. code-block:: bash

      # Format code
      make format
      
      # Run linting
      make lint
      
      # Run all tests
      make test

4. **Commit and Push**

   .. code-block:: bash

      git add .
      git commit -m "Add your descriptive commit message"
      git push origin your-branch-name

5. **Create Pull Request**
   - Go to GitHub and create a pull request
   - Fill out the pull request template
   - Wait for review and address feedback

Code Style and Standards
-----------------------

Rust Code
~~~~~~~~

- Follow standard Rust formatting with ``cargo fmt``
- Use ``cargo clippy`` for linting
- Write comprehensive tests for new functionality
- Document public APIs with doc comments
- Follow Rust naming conventions

.. code-block:: rust

   /// Makes an HTTP GET request to the specified URL.
   /// 
   /// # Arguments
   /// 
   /// * `url` - The URL to request
   /// * `config` - Request configuration options
   /// 
   /// # Returns
   /// 
   /// Returns a `Result` containing the response data or an error.
   pub async fn get_async(url: &str, config: RequestConfig) -> Result<ResponseData, RequestxError> {
       // Implementation here
   }

Python Code
~~~~~~~~~~

- Follow PEP 8 style guidelines
- Use ``black`` for code formatting
- Use ``ruff`` for linting
- Use ``mypy`` for type checking
- Write docstrings for all public functions

.. code-block:: python

   def example_function(param: str, optional_param: Optional[int] = None) -> Dict[str, Any]:
       """
       Example function with proper type hints and docstring.
       
       Args:
           param: Description of the parameter
           optional_param: Optional parameter description
           
       Returns:
           Dictionary containing the result
           
       Raises:
           ValueError: If param is invalid
       """
       pass

Testing Guidelines
-----------------

Test Structure
~~~~~~~~~~~~~

- **Unit Tests**: Test individual functions and methods
- **Integration Tests**: Test complete request/response cycles
- **Performance Tests**: Benchmark critical paths
- **Compatibility Tests**: Ensure requests compatibility

Writing Tests
~~~~~~~~~~~~

.. code-block:: python

   import unittest
   import requestx
   
   class TestBasicFunctionality(unittest.TestCase):
       def setUp(self):
           """Set up test fixtures before each test method."""
           self.session = requestx.Session()
       
       def tearDown(self):
           """Clean up after each test method."""
           # Clean up resources if needed
           pass
       
       def test_get_request(self):
           """Test basic GET request functionality."""
           response = requestx.get('https://httpbin.org/get')
           self.assertEqual(response.status_code, 200)
           self.assertIn('args', response.json())
       
       def test_async_get_request(self):
           """Test async GET request functionality."""
           import asyncio
           
           async def async_test():
               response = await requestx.get('https://httpbin.org/get')
               self.assertEqual(response.status_code, 200)
               return response.json()
           
           result = asyncio.run(async_test())
           self.assertIn('args', result)

Running Tests
~~~~~~~~~~~~

.. code-block:: bash

   # Run all tests
   make test
   
   # Run specific test file
   uv run python -m unittest tests.test_core_client -v
   
   # Run with coverage
   make test-coverage
   
   # Run performance tests
   make test-performance

Documentation Guidelines
-----------------------

Documentation Structure
~~~~~~~~~~~~~~~~~~~~~~

- **API Reference**: Complete function/class documentation
- **User Guide**: How-to guides and tutorials
- **Examples**: Practical code examples
- **Migration Guide**: Help users migrate from other libraries

Writing Documentation
~~~~~~~~~~~~~~~~~~~~

- Use clear, concise language
- Include code examples for all features
- Test all code examples to ensure they work
- Use proper reStructuredText formatting
- Include type hints in examples

.. code-block:: rst

   Making Requests
   ===============
   
   RequestX provides simple functions for making HTTP requests.
   
   Basic GET Request
   ----------------
   
   .. code-block:: python
   
      import requestx
      
      response = requestx.get('https://api.example.com/data')
      print(response.json())

Building Documentation
~~~~~~~~~~~~~~~~~~~~~

.. code-block:: bash

   # Install documentation dependencies
   pip install -r docs/requirements.txt
   
   # Build documentation
   cd docs
   make html
   
   # View documentation
   open _build/html/index.html

Pull Request Guidelines
----------------------

Before Submitting
~~~~~~~~~~~~~~~~~

- [ ] Code follows project style guidelines
- [ ] Tests pass locally
- [ ] Documentation is updated if needed
- [ ] Commit messages are descriptive
- [ ] Branch is up to date with main

Pull Request Template
~~~~~~~~~~~~~~~~~~~~

When creating a pull request, please include:

**Description**
- What does this PR do?
- Why is this change needed?
- How does it work?

**Type of Change**
- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update

**Testing**
- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] New and existing unit tests pass locally with my changes
- [ ] I have tested this change manually

**Checklist**
- [ ] My code follows the style guidelines of this project
- [ ] I have performed a self-review of my own code
- [ ] I have commented my code, particularly in hard-to-understand areas
- [ ] I have made corresponding changes to the documentation
- [ ] My changes generate no new warnings

Review Process
~~~~~~~~~~~~~

1. **Automated Checks**: CI will run tests and checks
2. **Code Review**: Maintainers will review your code
3. **Feedback**: Address any feedback or requested changes
4. **Approval**: Once approved, your PR will be merged

Bug Reports
----------

When reporting bugs, please include:

**Bug Report Template**

.. code-block:: text

   **Describe the bug**
   A clear and concise description of what the bug is.
   
   **To Reproduce**
   Steps to reproduce the behavior:
   1. Go to '...'
   2. Click on '....'
   3. Scroll down to '....'
   4. See error
   
   **Expected behavior**
   A clear and concise description of what you expected to happen.
   
   **Code Example**
   ```python
   import requestx
   # Your code that demonstrates the bug
   ```
   
   **Environment**
   - OS: [e.g. Windows 10, macOS 12, Ubuntu 20.04]
   - Python version: [e.g. 3.9.7]
   - RequestX version: [e.g. 0.1.0]
   
   **Additional context**
   Add any other context about the problem here.

Feature Requests
---------------

When requesting features, please include:

**Feature Request Template**

.. code-block:: text

   **Is your feature request related to a problem? Please describe.**
   A clear and concise description of what the problem is.
   
   **Describe the solution you'd like**
   A clear and concise description of what you want to happen.
   
   **Describe alternatives you've considered**
   A clear and concise description of any alternative solutions or features you've considered.
   
   **Use Case**
   Describe how this feature would be used and why it's valuable.
   
   **Additional context**
   Add any other context or screenshots about the feature request here.

Release Process
--------------

For maintainers, the release process is:

1. **Update Version Numbers**
   - Update version in ``Cargo.toml``
   - Update version in ``pyproject.toml``
   - Update version in ``docs/conf.py``

2. **Update Changelog**
   - Add new version section to ``CHANGELOG.md``
   - List all changes since last release

3. **Create Release**
   - Create and push git tag: ``git tag v0.1.0 && git push origin v0.1.0``
   - GitHub Actions will automatically build and publish to PyPI

4. **Post-Release**
   - Update documentation
   - Announce release on social media
   - Update any dependent projects

Community Guidelines
-------------------

**Code of Conduct**
We follow the `Contributor Covenant Code of Conduct <https://www.contributor-covenant.org/version/2/1/code_of_conduct/>`_. Please read it before participating.

**Communication**
- Be respectful and constructive
- Help others learn and grow
- Focus on the technical merits of ideas
- Assume good intentions

**Getting Help**
- Check existing issues and discussions first
- Provide minimal reproducible examples
- Be patient and respectful when asking for help
- Help others when you can

Recognition
----------

Contributors are recognized in several ways:

- Listed in the project's ``CONTRIBUTORS.md`` file
- Mentioned in release notes for significant contributions
- Given credit in documentation for major features
- Invited to join the maintainer team for sustained contributions

**Hall of Fame**
We maintain a list of significant contributors and their contributions to the project.

Thank You!
---------

Thank you for your interest in contributing to RequestX! Every contribution, no matter how small, helps make RequestX better for everyone.

Questions about contributing? Feel free to ask in our `GitHub Discussions <https://github.com/neuesql/requestx/discussions>`_ or open an issue.