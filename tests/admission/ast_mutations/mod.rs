//! Structural mutations of the frontend's AST JSON (O2).
//!
//! The closed decoder is only closed if it rejects every shape the frontend
//! cannot emit. This module builds those shapes from a real AST: it walks the
//! JSON value and changes one position at a time, without consulting the
//! decoder's field and variant tables, so a decoder table that quietly drifts
//! from the frontend is not mirrored by a sweep that drifts with it.
//!
//! Each mutant names the reason code the decoder must report. The sweep in
//! [`tests`] applies every mutant to every fixture the frontend parses and
//! requires a `decode`-stage rejection with that code, so the mutation is
//! caught before any profile decision.
//!
//! Two shapes cannot be reached this way and are out of scope: duplicate keys,
//! which [`serde_json::Value`] cannot represent (X5), and a mutation of the
//! frontend's own output, which is why every mutant is applied to JSON the
//! pinned frontend produced.

mod generate;
mod tests;

pub use generate::{Class, Mutant, mutants};
