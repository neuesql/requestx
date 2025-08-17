# RequestX Documentation

This directory contains the complete documentation for RequestX, built with Sphinx and hosted on Read the Docs.

## Building Documentation

### Prerequisites

Install documentation dependencies:

```bash
pip install -r requirements.txt
```

### Build HTML Documentation

```bash
# From the docs directory
make html

# Or from the project root
make docs-build
```

The built documentation will be available in `_build/html/index.html`.

### Serve Documentation Locally

```bash
# From the project root
make docs-serve
```

This will build the documentation and serve it at http://localhost:8000.

## Documentation Structure

```
docs/
├── index.rst              # Main documentation index
├── quickstart.rst          # Quick start guide
├── installation.rst        # Installation instructions
├── migration.rst           # Migration guide from requests
├── async-guide.rst         # Async/await usage guide
├── performance.rst         # Performance guide and benchmarks
├── changelog.rst           # Project changelog
├── contributing.rst        # Contributing guidelines
├── benchmarks.rst          # Detailed benchmark results
├── user-guide/            # Comprehensive user guide
│   ├── index.rst
│   └── ...
├── api/                   # API reference
│   ├── index.rst
│   ├── functions.rst
│   ├── response.rst
│   ├── session.rst
│   └── exceptions.rst
├── examples/              # Code examples
│   ├── index.rst
│   ├── basic-usage.rst
│   ├── async-usage.rst
│   ├── sessions.rst
│   └── advanced.rst
├── _static/               # Static files (CSS, images)
│   └── custom.css
├── _templates/            # Custom Sphinx templates
├── conf.py               # Sphinx configuration
├── requirements.txt      # Documentation dependencies
├── Makefile             # Sphinx build commands
└── README.md            # This file
```

## Writing Documentation

### Style Guidelines

- Use clear, concise language
- Include practical code examples
- Test all code examples to ensure they work
- Use proper reStructuredText formatting
- Include type hints in Python examples

### Code Examples

All code examples should be complete and runnable:

```python
import requestx

# Good: Complete example
response = requestx.get('https://httpbin.org/json')
data = response.json()
print(f"Status: {response.status_code}")

# Include error handling when relevant
try:
    response = requestx.get('https://api.example.com', timeout=10)
    response.raise_for_status()
    return response.json()
except requestx.RequestException as e:
    print(f"Request failed: {e}")
```

### reStructuredText Tips

- Use `.. code-block:: python` for Python code
- Use `.. note::` for important notes
- Use `.. warning::` for warnings
- Use `:doc:` for internal links
- Use proper section headers (=, -, ~, ^)

## Read the Docs Integration

The documentation is automatically built and deployed to Read the Docs when changes are pushed to the main branch.

### Configuration

- `.readthedocs.yaml` - Read the Docs configuration
- `conf.py` - Sphinx configuration with theme and extensions
- `requirements.txt` - Documentation build dependencies

### Theme

We use the [Furo](https://pradyunsg.me/furo/) theme for a modern, clean appearance.

## Contributing to Documentation

1. Make your changes to the relevant `.rst` files
2. Build the documentation locally to test: `make html`
3. Check that all links work and examples are correct
4. Submit a pull request with your changes

### Common Tasks

**Adding a new page:**
1. Create a new `.rst` file
2. Add it to the appropriate `toctree` directive
3. Build and test locally

**Adding code examples:**
1. Write complete, runnable examples
2. Test them manually
3. Include proper error handling
4. Add explanatory comments

**Updating API documentation:**
1. Update docstrings in the source code
2. Rebuild documentation to reflect changes
3. Verify all links and references work

## Troubleshooting

**Build errors:**
- Check that all dependencies are installed
- Verify reStructuredText syntax
- Check for broken internal links

**Missing content:**
- Ensure files are added to toctree directives
- Check file paths and names
- Verify Sphinx can find all referenced files

**Styling issues:**
- Check custom CSS in `_static/custom.css`
- Verify theme configuration in `conf.py`
- Test in different browsers

For more help, see the [Sphinx documentation](https://www.sphinx-doc.org/) or ask in our GitHub discussions.