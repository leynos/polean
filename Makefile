MDTABLEFIX ?= mdtablefix
MDTABLEFIX_SELECT = --git --include-untracked
MDTABLEFIX_RULES = --wrap --renumber --breaks --ellipsis --fences
MDLINT ?= markdownlint-cli2
CARGO ?= cargo
WHITAKER ?= whitaker
PYTHON ?= python3

.PHONY: help all build clean test typecheck lint lint-whitaker fmt check-fmt markdownlint design-check design-properties design-boundary audit tools test-scripts toolchain-smoke

SHELL := bash

# The development build standard (concordat rule `rust-build-defaults`): the
# parallel rustc frontend and, on Linux, the mold linker. An assigned RUSTFLAGS
# replaces every `rustflags` table in .cargo/config.toml, so each gate recipe
# that sets it composes these onto any inherited value (CI's setup-rust may
# export one). The Whitaker route takes neither, because its installer-managed
# toolchain must not inherit repository development flags.
BUILD_HOST_OS := $(shell uname -s)
STANDARD_RUSTFLAGS := -Zthreads=8$(if $(filter Linux,$(BUILD_HOST_OS)), -Clink-arg=-fuse-ld=mold)

all: check-fmt markdownlint typecheck lint test design-check design-properties design-boundary ## Run every local commit gate

build: ## Build the Rust scaffold
	$(CARGO) build --all-targets --all-features

clean: ## Remove Rust and Python build artefacts
	$(CARGO) clean
	find . -type d -name __pycache__ -prune -exec rm -rf {} +

lint: lint-whitaker ## Build documentation, run Clippy and the Whitaker suite with warnings denied
	RUSTDOCFLAGS="-D warnings" $(CARGO) doc --no-deps --all-features
	$(CARGO) clippy --all-targets --all-features -- -D warnings

WHITAKER_CLEAN_ENV = env -u RUSTFLAGS -u CARGO_ENCODED_RUSTFLAGS \
	-u CARGO_PROFILE_DEV_CODEGEN_BACKEND -u CARGO_BUILD_TARGET \
	-u CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER \
	-u CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS -u CFLAGS -u LDFLAGS

lint-whitaker: ## Run the Whitaker Dylint suite with clean driver inputs and warnings denied
	$(WHITAKER_CLEAN_ENV) DYLINT_RUSTFLAGS="-D warnings" $(WHITAKER) --all -- --all-targets --all-features

test: ## Run Rust unit, integration, and documentation tests
	$(CARGO) test --all-targets --all-features
	RUSTDOCFLAGS="-D warnings" $(CARGO) test --doc --all-features

typecheck: ## Type-check every Rust target with warnings denied
	RUSTFLAGS="$${RUSTFLAGS:+$$RUSTFLAGS }-D warnings $(STANDARD_RUSTFLAGS)" $(CARGO) check --all-targets --all-features

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

design-boundary: ## Test the harness's orchestration and storage boundaries
	$(PYTHON) validation/test_boundary.py

audit: ## Audit Rust dependencies for known vulnerabilities
	$(CARGO) audit

tools: ## Provision the pinned OPA and Lean tools into .polean/tools
	$(PYTHON) scripts/provision_tools.py

test-scripts: ## Test the provisioning script with pytest
	$(PYTHON) -m pytest scripts/tests

toolchain-smoke: tools ## Provision the pinned tools, then run the ignored smoke tests
	$(CARGO) test --locked --all-features --test toolchain -- --ignored smoke::

help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*?##' $(MAKEFILE_LIST) | \
	awk 'BEGIN {FS=":"; printf "Available targets:\n"} {printf "  %-20s %s\n", $$1, $$2}'
