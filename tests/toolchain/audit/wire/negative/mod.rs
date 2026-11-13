//! The strict-decoding negatives: every malformed shape is a `tool_failure`.
//!
//! Each case is a mutation of a valid document that a lenient decoder would
//! accept. The point is not that the mutation is unusual, but that the adapter
//! refuses it *before* settlement, so a malformed report can never be read as
//! a rejection or an acceptance. `serde`'s derive with `deny_unknown_fields`
//! does the field work; the extra cases cover the bounds JSON Schema cannot
//! state and the presence checks a derive would otherwise default away.
//!
//! Report and request mutations are separate modules, so neither file reaches
//! the repository's module-length limit as cases accumulate.

mod report;
mod request;
