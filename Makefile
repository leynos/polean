MDTABLEFIX ?= mdtablefix
MDTABLEFIX_SELECT = --git --include-untracked
MDTABLEFIX_RULES = --wrap --renumber --breaks --ellipsis --fences
MDLINT ?= markdownlint-cli2
CARGO ?= cargo
WHITAKER ?= whitaker
PYTHON ?= python3

.PHONY: help all build clean test typecheck lint lint-whitaker fmt check-fmt markdownlint design-check design-properties design-boundary audit tools test-scripts toolchain-smoke install-verus verus verus-proof verus-controls

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

# The Verus pins are read through $(shell ...), so the path and the pin are
# fixed with `override :=` and validated before either can be interpolated.
override PROVER_TOOLS_REF_FILE := tools/rust-prover-tools/REF
override PROVER_TOOLS_REF := $(shell awk '/^ref:/ { print $$2 }' $(PROVER_TOOLS_REF_FILE))
ifeq ($(shell printf '%s' '$(PROVER_TOOLS_REF)' | grep -Ec '^[0-9a-f]{40}$$'),0)
$(error PROVER_TOOLS_REF missing or malformed; expected "ref: <40-hex>" in $(PROVER_TOOLS_REF_FILE))
endif
override PROVER_TOOLS_SOURCE := git+https://github.com/leynos/rust-prover-tools.git@$(PROVER_TOOLS_REF)
override PROVER_TOOLS := uv tool run --from "$(PROVER_TOOLS_SOURCE)" prover-tools
VERUS_PROOF_FILE ?= verus/admission_recognizer.rs
VERUS_FLAGS ?=
VERUS_Z3_PATH := $(CURDIR)/.verus/z3/z3
override VERUS_VERSION := $(shell cat tools/verus/VERSION 2>/dev/null)
# `prover-tools verus run` cannot read the pinned release's `--version` output,
# which indents its `Toolchain:` line, so the target runs the binary that
# `install-verus` has already installed and digest-checked.
override VERUS_BIN := $(CURDIR)/.verus/$(VERUS_VERSION)/verus/verus

install-verus: ## Install the pinned Verus release and its pinned Z3 solver
	$(PROVER_TOOLS) verus install --repo-root .
	$(PYTHON) scripts/install_verus_z3.py

verus: verus-proof verus-controls ## Run the Verus proof and its negative controls

verus-proof: ## Run the Verus proof entry point
	@test -x "$(VERUS_BIN)" || { \
		echo "Verus is not installed at $(VERUS_BIN); run 'make install-verus' first" >&2; \
		exit 1; \
	}
	VERUS_Z3_PATH="$(VERUS_Z3_PATH)" "$(VERUS_BIN)" "$(VERUS_PROOF_FILE)" $(VERUS_FLAGS)

verus-controls: ## Show the proof rejects mutated kernel bodies and weakened specs
	@test -x "$(VERUS_BIN)" || { \
		echo "Verus is not installed at $(VERUS_BIN); run 'make install-verus' first" >&2; \
		exit 1; \
	}
	$(PYTHON) scripts/check_verus_controls.py \
		--verus "$(VERUS_BIN)" --z3 "$(VERUS_Z3_PATH)"

help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*?##' $(MAKEFILE_LIST) | \
	awk 'BEGIN {FS=":"; printf "Available targets:\n"} {printf "  %-20s %s\n", $$1, $$2}'
