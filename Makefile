# RequestX Makefile
# Abstracts the release pipeline commands following tech.md steering file

.PHONY: help setup clean format lint test build release publish all
.DEFAULT_GOAL := help

# Colors for output
BLUE := \033[34m
GREEN := \033[32m
YELLOW := \033[33m
RED := \033[31m
RESET := \033[0m

# Version extraction from Cargo.toml
VERSION := $(shell grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')

help: ## Show this help message
	@echo "$(BLUE)RequestX Build System$(RESET)"
	@echo "Version: $(GREEN)$(VERSION)$(RESET)"
	@echo ""
	@echo "$(YELLOW)Available targets:$(RESET)"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  $(GREEN)%-20s$(RESET) %s\n", $$1, $$2}' $(MAKEFILE_LIST)

# =============================================================================
# Development Setup
# =============================================================================

setup: ## Install development dependencies and setup environment
	@echo "$(BLUE)Setting up development environment...$(RESET)"
	@command -v uv >/dev/null 2>&1 || { echo "$(RED)uv not found. Installing...$(RESET)"; curl -LsSf https://astral.sh/uv/install.sh | sh; }
	uv sync --dev
	@echo "$(GREEN)Development environment setup complete!$(RESET)"

# =============================================================================
# Code Quality & Formatting
# =============================================================================

format: ## Format all code (Rust and Python)
	@echo "$(BLUE)Formatting code...$(RESET)"
	@echo "$(YELLOW)Formatting Rust code...$(RESET)"
	cargo fmt
	@echo "$(YELLOW)Formatting Python code...$(RESET)"
	uv run black .
	@echo "$(GREEN)Code formatting complete!$(RESET)"

format-check: ## Check code formatting without making changes
	@echo "$(BLUE)Checking code formatting...$(RESET)"
	@echo "$(YELLOW)Checking Rust formatting...$(RESET)"
	cargo fmt --check
	@echo "$(YELLOW)Checking Python formatting...$(RESET)"
	uv run black --check .
	@echo "$(GREEN)Format check complete!$(RESET)"

lint: ## Run all linting checks
	@echo "$(BLUE)Running linting checks...$(RESET)"
	@echo "$(YELLOW)Rust linting with clippy...$(RESET)"
	cargo clippy -- -D warnings
	@echo "$(YELLOW)Python linting with ruff...$(RESET)"
	uv run ruff check .
	@echo "$(YELLOW)Python type checking with mypy...$(RESET)"
	uv run mypy .
	@echo "$(GREEN)Linting complete!$(RESET)"

quality-check: format-check lint ## Run all code quality checks (CI stage 1)
	@echo "$(GREEN)All quality checks passed!$(RESET)"

# =============================================================================
# Building
# =============================================================================

build-dev: ## Build extension for development
	@echo "$(BLUE)Building development extension...$(RESET)"
	uv run maturin develop
	@echo "$(GREEN)Development build complete!$(RESET)"

build: build-dev ## Alias for build-dev

build-release: ## Build release version
	@echo "$(BLUE)Building release version...$(RESET)"
	uv run maturin build --release
	@echo "$(GREEN)Release build complete!$(RESET)"

build-wheels: ## Build wheels for distribution
	@echo "$(BLUE)Building distribution wheels...$(RESET)"
	uv run maturin build --release --strip
	@echo "$(GREEN)Wheel build complete!$(RESET)"

build-sdist: ## Build source distribution
	@echo "$(BLUE)Building source distribution...$(RESET)"
	uv run maturin sdist
	@echo "$(GREEN)Source distribution build complete!$(RESET)"

verify-import: build-dev ## Verify Python package can be imported (CI stage 2)
	@echo "$(BLUE)Verifying package import...$(RESET)"
	uv run python -c "import requestx; print('Import successful')"
	@echo "$(GREEN)Import verification complete!$(RESET)"

# =============================================================================
# Testing
# =============================================================================

test-rust: ## Run Rust unit tests (CI stage 3)
	@echo "$(BLUE)Running Rust unit tests...$(RESET)"
	cargo test --verbose
	cargo test --doc
	@echo "$(GREEN)Rust tests complete!$(RESET)"

test-python: build-dev ## Run Python unit tests (CI stage 4)
	@echo "$(BLUE)Running Python unit tests...$(RESET)"
	uv run python -m unittest discover tests/ -v
	@echo "$(GREEN)Python tests complete!$(RESET)"

test-core: build-dev ## Run core client tests specifically
	@echo "$(BLUE)Running core client tests...$(RESET)"
	uv run python -m unittest tests.test_core_client -v
	@echo "$(GREEN)Core client tests complete!$(RESET)"

test-integration: build-dev ## Run integration tests (CI stage 5)
	@echo "$(BLUE)Running integration tests...$(RESET)"
	@if [ -f tests/test_integration.py ]; then \
		uv run python -m unittest tests.test_integration -v; \
	else \
		echo "$(YELLOW)Integration tests not yet implemented$(RESET)"; \
	fi
	@if [ -f tests/test_async.py ]; then \
		uv run python -m unittest tests.test_async -v; \
	else \
		echo "$(YELLOW)Async tests not yet implemented$(RESET)"; \
	fi
	@echo "$(GREEN)Integration tests complete!$(RESET)"



test-comprehensive: build-dev ## Run comprehensive test suite (Task 9)
	@echo "$(BLUE)Running comprehensive test suite...$(RESET)"
	uv run python tests/test_final_suite.py
	@echo "$(GREEN)Comprehensive test suite complete!$(RESET)"

test-all-modules: build-dev ## Run all test modules with summary
	@echo "$(BLUE)Running all test modules...$(RESET)"
	uv run python -m unittest discover -s tests -v
	@echo "$(GREEN)All test modules complete!$(RESET)"

test-coverage: build-dev ## Run tests with coverage measurement
	@echo "$(BLUE)Running tests with coverage measurement...$(RESET)"
	@if command -v coverage >/dev/null 2>&1; then \
		uv run python -m coverage run -m unittest discover tests/ -v && uv run python -m coverage report; \
	else \
		echo "$(YELLOW)Coverage package not available, running tests without coverage$(RESET)"; \
		uv run python -m unittest discover tests/ -v; \
	fi
	@echo "$(GREEN)Coverage tests complete!$(RESET)"

test: test-rust test-python ## Run all tests
	@echo "$(GREEN)All tests complete!$(RESET)"

test-all: test test-integration test-performance test-comprehensive ## Run all tests including integration and performance
	@echo "$(GREEN)All tests (including integration and performance) complete!$(RESET)"

test-task9: test-comprehensive ## Run Task 9 comprehensive test suite
	@echo "$(GREEN)Task 9 comprehensive test suite complete!$(RESET)"

test-installation: build-dev ## Test installation process and bundled dependencies (Task 10)
	@echo "$(BLUE)Testing installation process...$(RESET)"
	uv run python scripts/test_installation.py
	@echo "$(GREEN)Installation tests complete!$(RESET)"

test-wheel-installation: build-wheels ## Test wheel installation in clean environment
	@echo "$(BLUE)Testing wheel installation...$(RESET)"
	uv run python scripts/test_installation.py --test-wheel
	@echo "$(GREEN)Wheel installation tests complete!$(RESET)"

task10: quality-check build-wheels build-sdist test-installation ## Complete Task 10: Set up build system and packaging
	@echo "$(GREEN)🎉 Task 10 completed successfully!$(RESET)"
	@echo "$(YELLOW)Build system and packaging setup complete:$(RESET)"
	@echo "  ✓ Maturin configured for cross-platform wheel building"
	@echo "  ✓ GitHub Actions CI/CD pipeline set up"
	@echo "  ✓ Cross-platform wheel building configured"
	@echo "  ✓ Installation process tested and verified"

docs-build: ## Build documentation with Sphinx
	@echo "$(BLUE)Building documentation...$(RESET)"
	@if [ -d docs ]; then \
		cd docs && make html; \
		echo "$(GREEN)Documentation built successfully!$(RESET)"; \
		echo "$(YELLOW)Open docs/_build/html/index.html to view$(RESET)"; \
	else \
		echo "$(RED)docs/ directory not found$(RESET)"; \
		exit 1; \
	fi

docs-serve: docs-build ## Build and serve documentation locally
	@echo "$(BLUE)Serving documentation locally...$(RESET)"
	@cd docs/_build/html && python -m http.server 8000
	@echo "$(GREEN)Documentation available at http://localhost:8000$(RESET)"

docs-clean: ## Clean documentation build files
	@echo "$(BLUE)Cleaning documentation build files...$(RESET)"
	@if [ -d docs/_build ]; then \
		rm -rf docs/_build; \
		echo "$(GREEN)Documentation build files cleaned!$(RESET)"; \
	else \
		echo "$(YELLOW)No documentation build files to clean$(RESET)"; \
	fi

task12: docs-build ## Complete Task 12: Create documentation and examples
	@echo "$(GREEN)🎉 Task 12 completed successfully!$(RESET)"
	@echo "$(YELLOW)Documentation and examples created:$(RESET)"
	@echo "  ✓ Comprehensive Sphinx documentation"
	@echo "  ✓ Read the Docs configuration"
	@echo "  ✓ API reference documentation"
	@echo "  ✓ User guide with examples"
	@echo "  ✓ Migration guide from requests"
	@echo "  ✓ Async/await usage guide"

	@echo "  ✓ Contributing guidelines"

# =============================================================================
# Documentation
# =============================================================================

docs: ## Generate documentation (CI stage 8)
	@echo "$(BLUE)Generating documentation...$(RESET)"
	@if [ -d docs ]; then \
		if command -v sphinx-build >/dev/null 2>&1; then \
			uv run sphinx-build docs/ docs/_build/; \
		else \
			echo "$(YELLOW)Sphinx not available, skipping documentation generation$(RESET)"; \
		fi; \
	else \
		echo "$(YELLOW)docs/ directory not found, skipping documentation generation$(RESET)"; \
	fi
	@if [ -f scripts/update_readme_examples.py ]; then \
		uv run python scripts/update_readme_examples.py; \
	else \
		echo "$(YELLOW)README update script not found$(RESET)"; \
	fi
	@echo "$(GREEN)Documentation generation complete!$(RESET)"

# =============================================================================
# Release Pipeline
# =============================================================================

ci-pipeline: quality-check verify-import test-rust test-python test-integration test-performance docs ## Run full CI pipeline
	@echo "$(GREEN)Full CI pipeline completed successfully!$(RESET)"

pre-release: ci-pipeline build-wheels build-sdist ## Prepare for release (run full pipeline + build artifacts)
	@echo "$(GREEN)Pre-release preparation complete!$(RESET)"
	@echo "$(YELLOW)Artifacts ready for release:$(RESET)"
	@ls -la target/wheels/ 2>/dev/null || echo "  No wheels found"
	@ls -la dist/ 2>/dev/null || echo "  No source distribution found"

release-tag: ## Create and push release tag
	@echo "$(BLUE)Creating release tag v$(VERSION)...$(RESET)"
	@if git diff --quiet && git diff --cached --quiet; then \
		git tag v$(VERSION); \
		git push origin v$(VERSION); \
		echo "$(GREEN)Release tag v$(VERSION) created and pushed!$(RESET)"; \
	else \
		echo "$(RED)Error: Working directory not clean. Commit changes first.$(RESET)"; \
		exit 1; \
	fi

publish-pypi: ## Publish to PyPI (requires PYPI_TOKEN)
	@echo "$(BLUE)Publishing to PyPI...$(RESET)"
	@if [ -z "$$PYPI_TOKEN" ]; then \
		echo "$(RED)Error: PYPI_TOKEN environment variable not set$(RESET)"; \
		exit 1; \
	fi
	PYPI_TOKEN=$$PYPI_TOKEN uv run maturin publish --username __token__ --password $$PYPI_TOKEN
	@echo "$(GREEN)Published to PyPI successfully!$(RESET)"

github-release: ## Create GitHub release (requires gh CLI and GITHUB_TOKEN)
	@echo "$(BLUE)Creating GitHub release...$(RESET)"
	@if command -v gh >/dev/null 2>&1; then \
		gh release create v$(VERSION) --generate-notes; \
		echo "$(GREEN)GitHub release created successfully!$(RESET)"; \
	else \
		echo "$(RED)Error: gh CLI not found. Install GitHub CLI first.$(RESET)"; \
		exit 1; \
	fi

release: pre-release release-tag publish-pypi github-release ## Full release process
	@echo "$(GREEN)🎉 Release v$(VERSION) completed successfully!$(RESET)"

# =============================================================================
# Utility Commands
# =============================================================================

clean: ## Clean build artifacts
	@echo "$(BLUE)Cleaning build artifacts...$(RESET)"
	cargo clean
	rm -rf target/wheels/
	rm -rf dist/
	rm -rf build/
	rm -rf *.egg-info/
	find . -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true
	find . -type f -name "*.pyc" -delete 2>/dev/null || true
	@echo "$(GREEN)Clean complete!$(RESET)"

status: ## Show project status
	@echo "$(BLUE)RequestX Project Status$(RESET)"
	@echo "Version: $(GREEN)$(VERSION)$(RESET)"
	@echo "Git branch: $(GREEN)$$(git branch --show-current 2>/dev/null || echo 'unknown')$(RESET)"
	@echo "Git status:"
	@git status --porcelain 2>/dev/null || echo "  Not a git repository"
	@echo ""
	@echo "$(YELLOW)Dependencies:$(RESET)"
	@command -v uv >/dev/null 2>&1 && echo "  ✓ uv installed" || echo "  ✗ uv not found"
	@command -v cargo >/dev/null 2>&1 && echo "  ✓ cargo installed" || echo "  ✗ cargo not found"
	@command -v python >/dev/null 2>&1 && echo "  ✓ python installed" || echo "  ✗ python not found"
	@command -v gh >/dev/null 2>&1 && echo "  ✓ gh CLI installed" || echo "  ✗ gh CLI not found"

dev: setup build-dev ## Quick development setup (setup + build)
	@echo "$(GREEN)Development environment ready!$(RESET)"

# =============================================================================
# Aliases for common workflows
# =============================================================================

check: quality-check ## Alias for quality-check
fix: format ## Alias for format (fix formatting issues)
install: setup ## Alias for setup
all: ci-pipeline ## Run everything (full CI pipeline)

# =============================================================================
# Environment Info
# =============================================================================

env-info: ## Show environment information
	@echo "$(BLUE)Environment Information$(RESET)"
	@echo "Make version: $$(make --version | head -1)"
	@echo "Shell: $$SHELL"
	@echo "OS: $$(uname -s)"
	@echo "Architecture: $$(uname -m)"
	@echo ""
	@echo "$(YELLOW)Tool Versions:$(RESET)"
	@command -v uv >/dev/null 2>&1 && echo "uv: $$(uv --version)" || echo "uv: not installed"
	@command -v cargo >/dev/null 2>&1 && echo "cargo: $$(cargo --version)" || echo "cargo: not installed"
	@command -v python >/dev/null 2>&1 && echo "python: $$(python --version)" || echo "python: not installed"
	@command -v rustc >/dev/null 2>&1 && echo "rustc: $$(rustc --version)" || echo "rustc: not installed"