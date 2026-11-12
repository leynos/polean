MDTABLEFIX ?= mdtablefix
MDTABLEFIX_SELECT = --git --include-untracked
MDTABLEFIX_RULES = --wrap --renumber --breaks --ellipsis --fences
MDLINT ?= markdownlint-cli2
CARGO ?= cargo
WHITAKER ?= whitaker
PYTHON ?= python3

.PHONY: help all build clean test typecheck lint interrogate lint-whitaker fmt check-fmt markdownlint design-check design-properties design-boundary audit tools test-scripts toolchain-smoke install-verus verus verus-proof verus-controls

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

# Interrogate scans each target recursively, subject to its default exclusions
# (check a new file against the verbose report). Add a new Python directory to
# the targets. Python 3.14 is required: interrogate parses the source with the
# interpreter that runs it, and the scripts import `compression.zstd`.
INTERROGATE_VERSION ?= 1.7.0
INTERROGATE_TARGETS ?= scripts validation
# Level 2 prints a MISSED row per undocumented definition, which the
# `interrogate` target fails on: the rounded percentage alone can pass a lone
# gap.
INTERROGATE_FLAGS ?= -vv --no-color --fail-under 100
INTERROGATE ?= uv tool run --python 3.14 --from 'interrogate==$(INTERROGATE_VERSION)' interrogate

# `grep` reads the whole report, so an early exit cannot break the pipe.
interrogate: ## Enforce complete Python docstring coverage
	@set -euo pipefail; \
	  status=0; \
	  report="$$($(INTERROGATE) $(INTERROGATE_FLAGS) $(INTERROGATE_TARGETS))" || status=$$?; \
	  printf '%s\n' "$$report"; \
	  test "$$status" -eq 0 || exit "$$status"; \
	  found=0; \
	  printf '%s\n' "$$report" | grep '| *MISSED *|' >/dev/null || found=$$?; \
	  if test "$$found" -ne 1; then \
	    printf '%s\n' 'interrogate: undocumented definitions remain (or the MISSED check failed); see above' >&2; \
	    exit 1; \
	  fi

lint: interrogate lint-whitaker ## Check docstring coverage, build documentation, run Clippy and the Whitaker suite with warnings denied
	RUSTDOCFLAGS="-D warnings" $(CARGO) doc --no-deps --all-features
	$(CARGO) clippy --all-targets --all-features -- -D warnings

# Whitaker builds on its installer-managed toolchain, which must not inherit the
# repository's development flags. The route clears the encoded flags and the
# target settings, assigns the warning policy as its own `RUSTFLAGS` (an unset
# value would leave Cargo reading the configuration's rustflags), and selects
# LLVM for the development profile, whose Cranelift default the configuration
# would otherwise apply (CARGO_UNSTABLE_CODEGEN_BACKEND opts in the temporary
# Cargo project where Dylint builds its driver, which cannot discover the root
# `.cargo/config.toml` that enables the unstable profile key). Dylint reads the warning policy from DYLINT_RUSTFLAGS.
lint-whitaker: ## Run the Whitaker Dylint suite with clean driver inputs and warnings denied
	env -u CARGO_ENCODED_RUSTFLAGS -u CARGO_BUILD_TARGET -u CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER -u CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS -u CFLAGS -u LDFLAGS RUSTFLAGS="-D warnings" CARGO_PROFILE_DEV_CODEGEN_BACKEND=llvm CARGO_UNSTABLE_CODEGEN_BACKEND=true DYLINT_RUSTFLAGS="-D warnings" $(WHITAKER) --all -- --all-targets --all-features

test: ## Run Rust unit, integration, and documentation tests
	$(CARGO) fetch --locked
	$(CARGO) nextest run --all-targets --all-features
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

# The Verus pins are read through $(shell ...), so the path and the pins are
# fixed with `override :=` and validated before either can be interpolated.
override PROVER_TOOLS_REF_FILE := tools/rust-prover-tools/REF
override PROVER_TOOLS_REF := $(shell awk '/^ref:/ && length($$2) == 40 && $$2 !~ /[^0-9a-f]/ { print $$2; exit }' $(PROVER_TOOLS_REF_FILE))
ifeq ($(PROVER_TOOLS_REF),)
$(error PROVER_TOOLS_REF missing or malformed; expected "ref: <40-hex>" in $(PROVER_TOOLS_REF_FILE))
endif
override PROVER_TOOLS_SOURCE := git+https://github.com/leynos/rust-prover-tools.git@$(PROVER_TOOLS_REF)
override PROVER_TOOLS := uv tool run --from "$(PROVER_TOOLS_SOURCE)" prover-tools
VERUS_PROOF_FILE ?= verus/admission_recognizer.rs
VERUS_FLAGS ?=
VERUS_Z3_PATH := $(CURDIR)/.verus/z3/z3
override VERUS_VERSION := $(shell awk 'NR == 1 && /^[0-9A-Za-z.]+$$/ { print; exit }' tools/verus/VERSION 2>/dev/null)
ifeq ($(VERUS_VERSION),)
$(error VERUS_VERSION missing or malformed in tools/verus/VERSION; expected a release such as 0.2026.09.06.8dea4a2)
endif
# Both installers write under `.verus`, where `VERUS_BIN` and `VERUS_Z3_PATH`
# find them. The shared tool owns the downloads, digests and the Z3 release; this
# file only chooses where they land.
override VERUS_INSTALL_DIR := .verus/$(VERUS_VERSION)
override VERUS_BIN := $(CURDIR)/$(VERUS_INSTALL_DIR)/verus/verus

install-verus: ## Install the pinned Verus release and its pinned Z3 solver
	$(PROVER_TOOLS) verus install --repo-root . --install-dir "$(VERUS_INSTALL_DIR)"
	$(PROVER_TOOLS) z3 install --repo-root . --install-dir .verus/z3 --cache-dir .verus/z3-archive

verus: verus-proof verus-controls ## Run the Verus proof and its negative controls

verus-proof: ## Run the Verus proof entry point
	@test -x "$(VERUS_BIN)" || { \
		echo "Verus is not installed at $(VERUS_BIN); run 'make install-verus' first" >&2; \
		exit 1; \
	}
	VERUS_Z3_PATH="$(VERUS_Z3_PATH)" $(PROVER_TOOLS) verus run --repo-root . \
		--verus-bin "$(VERUS_BIN)" --no-install-missing \
		--proof-file "$(VERUS_PROOF_FILE)" $(addprefix --extra-arg=,$(VERUS_FLAGS))

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
