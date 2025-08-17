# Publishing to PyPI

This repository is configured to automatically publish to PyPI using GitHub Actions and PyPI's trusted publishing feature.

## Setup Instructions

### 1. Configure PyPI API Token

1. Go to [PyPI](https://pypi.org) and log in to your account
2. Navigate to your account settings and select "API tokens"
3. Create a new API token with the scope limited to the `requestx` project
4. Copy the generated token (it starts with `pypi-`)

### 2. Configure GitHub Environment and Secrets

1. Go to your GitHub repository settings
2. Navigate to "Environments"
3. Create a new environment named `PROD`
4. In the `PROD` environment, add a new secret:
   - **Name**: `PYPI_TOKEN`
   - **Value**: The API token you copied from PyPI
5. Optionally, add protection rules (e.g., require reviewers for production releases)

## Publishing Process

### Automatic Publishing (Recommended)

The package will be automatically published to PyPI when you create a new release:

1. Go to your GitHub repository
2. Click on "Releases" → "Create a new release"
3. Create a new tag (e.g., `v0.1.1`)
4. Fill in the release title and description
5. Click "Publish release"

The GitHub Action will automatically:
- Build wheels for all supported platforms (Linux, macOS, Windows)
- Build wheels for all supported Python versions (3.8-3.12)
- Create a source distribution
- Publish everything to PyPI

### Manual Publishing

You can also trigger publishing manually:

1. Go to the "Actions" tab in your repository
2. Select the "Publish to PyPI" workflow
3. Click "Run workflow"
4. Choose whether to publish to Test PyPI or production PyPI

### Test PyPI

To test the publishing process without affecting the production package:

1. Set up trusted publishing for [Test PyPI](https://test.pypi.org) as well
2. Use the manual workflow trigger and check "Publish to Test PyPI"

## Package Requirements

The publishing workflow expects:
- A `pyproject.toml` file with proper package metadata
- Rust source code that can be compiled with `maturin`
- Python tests in the `tests/` directory

## Troubleshooting

### Common Issues

1. **Publishing fails with authentication error**
   - Ensure the PyPI API token is correctly set in the `PROD` environment
   - Check that the GitHub environment name matches exactly (`PROD`)
   - Verify the API token has the correct scope for the `requestx` project

2. **Wheel building fails**
   - Check that all Rust dependencies are properly specified
   - Ensure the code compiles on all target platforms

3. **Version conflicts**
   - Make sure to increment the version in `pyproject.toml` before creating a release
   - PyPI doesn't allow overwriting existing versions

### Getting Help

- Check the GitHub Actions logs for detailed error messages
- Review the [PyPI trusted publishing documentation](https://docs.pypi.org/trusted-publishers/)
- Check the [maturin documentation](https://maturin.rs/) for Rust-Python packaging issues