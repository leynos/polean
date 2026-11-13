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
    """Fail the run with a named diagnostic when a design check does not hold.

    Parameters
    ----------
    condition:
        The check's result. A true value passes; a false value fails the run.
    message:
        The diagnostic to report. It names the document, fixture, or contract
        that failed, so a reader does not have to reconstruct which check ran.

    Raises
    ------
    AssertionError
        If ``condition`` is false, carrying ``message`` unchanged.
    """
    if not condition:
        raise AssertionError(message)


def check_schema(validators: dict[str, Draft202012Validator], name: str,
                 value: Any, label: str) -> None:
    """Require a document to satisfy its contract, naming it on rejection.

    ``jsonschema`` raises a ``ValidationError`` whose message is a paragraph of
    schema internals. A rejected document is a finding about a fixture or a
    contract, not a defect in the harness, so it is reported in the same one
    shape as every other check rather than as a traceback.

    Parameters
    ----------
    validators:
        The compiled contract validators, keyed by contract name.
    name:
        The contract the document is held to. It must be a key of
        ``validators``.
    value:
        The decoded document to check.
    label:
        The document's path or description, used to name it in the diagnostic.

    Raises
    ------
    AssertionError
        If the document violates the contract. The message names ``label``, the
        contract, the offending JSON pointer, and the validator's reason.
    """
    try:
        validators[name].validate(value)
    except ValidationError as error:
        path = "/".join(str(part) for part in error.absolute_path) or "<root>"
        raise AssertionError(
            f"{label} does not satisfy the {name} contract at {path}: "
            f"{error.message}") from error
