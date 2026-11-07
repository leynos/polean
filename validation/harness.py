"""Shared diagnostics for the design harness and its contract checks.

Every check in this package reports a failure the same way: a named assertion
rather than a traceback. The audit contract checks live in
:mod:`audit_contracts`, so the helper they share with the rest of the harness
lives here rather than in either caller.
"""

from __future__ import annotations

from typing import Any

from jsonschema import Draft202012Validator
from jsonschema.exceptions import ValidationError

NEGATIVE_CONTROL_HELP = "a negative control did not detect its injected fault"


def require(condition: bool, message: str) -> None:
    """Fail the run with a named diagnostic when a design check does not hold."""
    if not condition:
        raise AssertionError(message)


def check_schema(validators: dict[str, Draft202012Validator], name: str,
                 value: Any, label: str) -> None:
    """Require a document to satisfy its contract, naming it on rejection.

    ``jsonschema`` raises a ``ValidationError`` whose message is a paragraph of
    schema internals. A rejected document is a finding about a fixture or a
    contract, not a defect in the harness, so it is reported in the same one
    shape as every other check rather than as a traceback.
    """
    try:
        validators[name].validate(value)
    except ValidationError as error:
        path = "/".join(str(part) for part in error.absolute_path) or "<root>"
        raise AssertionError(
            f"{label} does not satisfy the {name} contract at {path}: "
            f"{error.message}") from error
