# RequestX - Simplified Build System
# Use numbered commands for clear sequencing: make 1-setup, make 2-format, etc.

.PHONY: help \
        1-setup 2-format 2-format-check \
        3-lint 4-quality-check \
        5-build 6-test-rust 6-test-python 6-test-all 6-test-coverage \
        7-doc-build 7-doc-serve \
        8-release-github 8-release-pypi \
        9-clean version-patch version-minor version-major version-show \
        benchmark-get-test benchmark-run benchmark-compare

.DEFAULT_GOAL := help

# Colors for output
BLUE  := \033[34m
GREEN := \033[32m
YELLOW:= \033[33m
RED   := \033[31m
RESET := \033[0m

# Version extraction from Cargo.toml
VERSION := $(shell grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')

# UV version
UV_VERSION := $(shell uv version --short 2>/dev/null || echo "not installed")

help: ## Show available commands
	@echo "$(BLUE)RequestX v$(VERSION)$(RESET) | UV: $(GREEN)$(UV_VERSION)$(RESET)"
	@echo ""
	@echo "$(YELLOW)Development:$(RESET)"
	@awk 'BEGIN {FS = ":.*?## "} /^[1-9]-.*:.*?## / {printf "  make %-18s %s\n", $$1, $$2}' $(MAKEFILE_LIST)
	@echo ""
	@echo "$(YELLOW)Benchmark:$(RESET)"
	@awk 'BEGIN {FS = ":.*?## "} /^benchmark-.*:.*?## / {printf "  make %-18s %s\n", $$1, $$2}' $(MAKEFILE_LIST)
	@echo ""
	@echo "$(YELLOW)Version bumping:$(RESET)"
	@awk 'BEGIN {FS = ":.*?## "} /^version-.*:.*?## / {printf "  make %-18s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

# =============================================================================
# 1. Setup
# =============================================================================

1-setup: ## Setup development environment
	@echo "$(BLUE)Setting up dev environment...$(RESET)"
	@command -v uv >/dev/null 2>&1 || { echo "$(RED)Installing uv...$(RESET)"; curl -LsSf https://astral.sh/uv/install.sh | sh; }
	uv sync --dev
	@echo "$(GREEN)✓ Setup complete$(RESET)"

# =============================================================================
# 2. Formatting
# =============================================================================

2-format: ## Format Rust + Python code
	@echo "$(BLUE)Formatting code...$(RESET)"
	cargo fmt
	uv run black .
	@echo "$(GREEN)✓ Formatted$(RESET)"

2-format-check: ## Check formatting (no changes)
	@echo "$(BLUE)Checking formatting...$(RESET)"
	cargo fmt --check
	uv run black --check .
	@echo "$(GREEN)✓ Format OK$(RESET)"

# =============================================================================
# 3. Linting
# =============================================================================

3-lint: ## Run linters (clippy + ruff)
	@echo "$(BLUE)Running linters...$(RESET)"
	cargo clippy --fix
	uv run ruff
	@echo "$(GREEN)✓ Linting passed$(RESET)"


3-lint-check: ## Run linters (clippy + ruff)
	@echo "$(BLUE)Running linters...$(RESET)"
	cargo clippy -- -D warnings -A clippy::too_many_arguments
	uv run ruff check .
	@echo "$(GREEN)✓ Linting passed$(RESET)"

# =============================================================================
# 4. Quality Check (CI Stage 1)
# =============================================================================

4-quality-check: 2-format-check 3-lint ## Combined format check + lint
	@echo "$(GREEN)✓ All quality checks passed$(RESET)"

# =============================================================================
# 5. Build
# =============================================================================

5-build: ## Build Rust/Python extension (dev)
	@echo "$(BLUE)Building...$(RESET)"
	uv run maturin develop
	@echo "$(GREEN)✓ Build complete$(RESET)"

# =============================================================================
# 6. Testing
# =============================================================================

6-test-rust: ## Run Rust tests
	@echo "$(BLUE)Running Rust tests...$(RESET)"
	cargo test --verbose
	cargo test --doc
	@echo "$(GREEN)✓ Rust tests passed$(RESET)"

6-test-python: 5-build ## Run Python tests (requires build)
	@echo "$(BLUE)Running Python tests...$(RESET)"
	uv run python -m pytest tests/ -v
	@echo "$(GREEN)✓ Python tests passed$(RESET)"

6-test-all: 6-test-rust 6-test-python ## Run all tests
	@echo "$(GREEN)✓ All tests passed$(RESET)"

6-test-coverage: 5-build ## Run tests with coverage
	@echo "$(BLUE)Running tests with coverage...$(RESET)"
	uv run python -m coverage run -m unittest discover tests/ -v
	uv run python -m coverage report
	uv run python -m coverage xml
	@echo "$(GREEN)✓ Coverage report generated$(RESET)"


# =============================================================================
# 7. Documentation (MkDocs)
# =============================================================================

7-doc-build: ## Build MkDocs documentation
	@echo "$(BLUE)Building docs...$(RESET)"
	uv run mkdocs build
	@echo "$(GREEN)✓ Docs built (site/index.html)$(RESET)"

7-doc-serve: ## Serve docs locally with hot reload
	@echo "$(BLUE)Serving docs at http://localhost:8000$(RESET)"
	uv run mkdocs serve

# =============================================================================
# 8. Release
# =============================================================================

8-release-github: ## Create GitHub release (requires GIT_TOKEN)
	@echo "$(BLUE)Creating GitHub release v$(VERSION)...$(RESET)"
	@if [ -z "$$GIT_TOKEN" ]; then \
		echo "$(RED)Error: GIT_TOKEN not set$(RESET)"; \
		exit 1; \
	fi
	@gh auth login --with-token $$GIT_TOKEN 2>/dev/null || true
	gh release create v$(VERSION) --generate-notes
	@echo "$(GREEN)✓ GitHub release created$(RESET)"


8-release-pypi: ## Publish to PyPI (requires PYPI_TOKEN)
	@echo "$(BLUE)Publishing to PyPI...$(RESET)"
	@if [ -z "$$PYPI_TOKEN" ]; then \
		echo "$(RED)Error: PYPI_TOKEN not set$(RESET)"; \
		exit 1; \
	fi
	uv run maturin publish --username __token__ --password $$PYPI_TOKEN
	@echo "$(GREEN)✓ Published to PyPI$(RESET)"

# =============================================================================
# Version Bumping (syncs Cargo.toml, pyproject.toml, __init__.py)
# =============================================================================

version-show: ## Show current version
	@./bump.sh

version-patch: ## Bump patch version (0.0.x)
	@./bump.sh patch

version-minor: ## Bump minor version (0.x.0)
	@./bump.sh minor

version-major: ## Bump major version (x.0.0)
	@./bump.sh major

# =============================================================================
# 9. Cleanup
# =============================================================================

9-clean: ## Clean all build artifacts + docs
	@echo "$(BLUE)Cleaning...$(RESET)"
	cargo clean
	rm -rf dist/ target/wheels/ build/ *.egg-info/
	rm -rf site/
	find . -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true
	find . -type f -name "*.pyc" -delete 2>/dev/null || true
	@echo "$(GREEN)✓ Clean complete$(RESET)"

# =============================================================================
# 10. Benchmark
# =============================================================================

benchmark-get-sync-test: ## Run GET benchmark (localhost, concurrency=1, duration=3s)
	@echo "$(BLUE)Running GET Sync benchmark...$(RESET)"
	.venv/bin/http-benchmark --url http://localhost/get --compare requestx requests httpx urllib3 pycurl --concurrency 1 --duration 3
	@echo "$(GREEN)✓ Benchmark complete$(RESET)"


benchmark-get-async-test: ## Run GET benchmark (localhost, concurrency=1, duration=3s)
	@echo "$(BLUE)Running GET Async benchmark...$(RESET)"
	.venv/bin/http-benchmark --url http://localhost/get --compare requestx httpx aiohttp --concurrency 1 --duration 3 --async
	@echo "$(GREEN)✓ Benchmark complete$(RESET)"
