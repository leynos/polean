MDTABLEFIX ?= mdtablefix
MDTABLEFIX_SELECT = --git --include-untracked
MDTABLEFIX_RULES = --wrap --renumber --breaks --ellipsis --fences
MDLINT ?= markdownlint-cli2
CARGO ?= cargo
PYTHON ?= python3

.PHONY: help all build clean test typecheck lint fmt check-fmt markdownlint design-check design-properties audit

SHELL := bash

all: check-fmt typecheck lint test design-check design-properties ## Run every local commit gate

build: ## Build the Rust scaffold
	$(CARGO) build --all-targets --all-features

clean: ## Remove Rust and Python build artefacts
	$(CARGO) clean
	find . -type d -name __pycache__ -prune -exec rm -rf {} +

lint: ## Build documentation and run Clippy with warnings denied
	RUSTDOCFLAGS="-D warnings" $(CARGO) doc --no-deps --all-features
	$(CARGO) clippy --all-targets --all-features -- -D warnings

test: ## Run Rust unit, integration, and documentation tests
	$(CARGO) test --all-targets --all-features
	RUSTDOCFLAGS="-D warnings" $(CARGO) test --doc --all-features

typecheck: ## Type-check every Rust target with warnings denied
	RUSTFLAGS="-D warnings" $(CARGO) check --all-targets --all-features

fmt: ## Format Rust and Markdown sources
	$(CARGO) fmt --all
	$(MDTABLEFIX) --in-place $(MDTABLEFIX_SELECT) $(MDTABLEFIX_RULES)
	$(MDLINT) --fix "**/*.md"

check-fmt: ## Verify Rust and Markdown table formatting
	$(CARGO) fmt --all -- --check
	$(MDTABLEFIX) --check $(MDTABLEFIX_SELECT) $(MDTABLEFIX_RULES)

markdownlint: ## Lint Markdown sources
	$(MDLINT) "**/*.md"

design-check: ## Validate schemas, policy fixtures, abstractions, and witnesses
	$(PYTHON) validation/validate_design.py

design-properties: ## Search the admitted domain for model-side counterexamples
	$(PYTHON) validation/property_fixtures.py

audit: ## Audit Rust dependencies for known vulnerabilities
	$(CARGO) audit

help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*?##' $(MAKEFILE_LIST) | \
	awk 'BEGIN {FS=":"; printf "Available targets:\n"} {printf "  %-20s %s\n", $$1, $$2}'
