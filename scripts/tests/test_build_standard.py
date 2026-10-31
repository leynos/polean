"""Contract tests for the development build standard (``rust-build-defaults``).

The Concordat rule needs the parallel frontend and the Cranelift backend as
Cargo defaults, the ``mold`` linker as a Linux-only default, the fast flags
restated wherever a gate recipe assigns ``RUSTFLAGS``, and the Whitaker route
isolated from all of it. Each property lives in a file an unrelated edit could
quietly break, so each is held here.
"""

from __future__ import annotations

import re
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MOLD = "-Clink-arg=-fuse-ld=mold"
THREADS = "-Zthreads=8"
LINUX_TABLE = 'cfg(target_os = "linux")'
SETUP_RUST_STEP = re.compile(r"setup-rust@\w{40}\n\s+with:\n(?P<inputs>(?:\s+\S.*\n)+)")


def _config() -> dict:
    """Parse the repository's Cargo configuration."""
    return tomllib.loads((ROOT / ".cargo" / "config.toml").read_text("utf-8"))


def _recipe(target: str) -> str:
    """Return the text of one Makefile target's recipe."""
    makefile = (ROOT / "Makefile").read_text("utf-8")
    pattern = rf"^{re.escape(target)}:[^\n]*\n((?:\t[^\n]*\n)*)"
    match = re.search(pattern, makefile, re.MULTILINE)
    assert match, f"the Makefile must define {target}"
    return match.group(1)


def test_cargo_defaults_select_the_fast_build() -> None:
    """Every rustflags source repeats the frontend flag; Cranelift is the dev backend."""
    config = _config()
    assert THREADS in config["build"]["rustflags"], "[build] must carry -Zthreads=8"
    linux = config["target"][LINUX_TABLE]["rustflags"]
    assert {THREADS, MOLD} <= set(linux), "the Linux table must repeat -Zthreads=8"
    assert config["unstable"]["codegen-backend"] is True, "the backend key needs its gate"
    assert config["profile"]["dev"]["codegen-backend"] == "cranelift", (
        "Cranelift must be the development default"
    )


def test_mold_is_linux_only() -> None:
    """mold ships for Linux alone, so no platform-wide source names it."""
    assert MOLD not in _config()["build"]["rustflags"], "[build] must not name mold"


def test_the_toolchain_ships_the_cranelift_component() -> None:
    """The pin lists the component the dev backend needs."""
    pin = tomllib.loads((ROOT / "rust-toolchain.toml").read_text("utf-8"))
    assert "rustc-codegen-cranelift-preview" in pin["toolchain"]["components"]


def test_typecheck_restates_the_fast_flags() -> None:
    """An assigned RUSTFLAGS replaces the configuration, so typecheck adds them."""
    makefile = (ROOT / "Makefile").read_text("utf-8")
    assert f"-Zthreads=8$(if $(filter Linux,$(BUILD_HOST_OS)), {MOLD})" in makefile
    assert "$(STANDARD_RUSTFLAGS)" in _recipe("typecheck"), "typecheck must restate them"


def test_the_whitaker_route_is_isolated() -> None:
    """The route clears inherited flags and backend overrides, and takes none."""
    makefile = (ROOT / "Makefile").read_text("utf-8")
    clean = re.search(r"^WHITAKER_CLEAN_ENV = (?:[^\n]*\\\n)*[^\n]*", makefile, re.MULTILINE)
    assert clean, "the Makefile must define WHITAKER_CLEAN_ENV"
    for variable in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CARGO_PROFILE_DEV_CODEGEN_BACKEND"):
        assert f"-u {variable}" in clean.group(0), f"the Whitaker route must clear {variable}"
    recipe = _recipe("lint-whitaker")
    assert "$(WHITAKER_CLEAN_ENV)" in recipe, "lint-whitaker must run in the clean environment"
    assert "STANDARD_RUSTFLAGS" not in recipe, "the Whitaker route must not inherit mold"


def test_ci_installs_mold_and_the_whitaker_cranelift_component() -> None:
    """Hosted jobs that build with the Linux table need mold; Whitaker needs Cranelift."""
    for workflow in ("ci.yml", "toolchain.yml"):
        text = (ROOT / ".github" / "workflows" / workflow).read_text("utf-8")
        step = SETUP_RUST_STEP.search(text)
        assert step, f"{workflow} must configure setup-rust"
        assert "install-mold: 'true'" in step.group("inputs"), f"{workflow} must install mold"
    ci = (ROOT / ".github" / "workflows" / "ci.yml").read_text("utf-8")
    assert re.search(r"install-whitaker@\w{40}\n\s+with:\n\s+cranelift: 'true'", ci), (
        "ci.yml must install the Cranelift component for the Whitaker toolchain"
    )
