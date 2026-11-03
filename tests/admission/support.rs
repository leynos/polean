//! Corpus helpers shared by the test modules.
//!
//! Each helper panics with the underlying read or parse failure, so a broken
//! fixture names itself rather than surfacing as an unrelated assertion in the
//! test that happened to read it first.

use crate::corpus::{self, Entry, Profile};

/// The parsed manifest, or a panic naming the first parse failure.
pub fn manifest() -> Vec<Entry> {
    match corpus::entries() {
        Ok(entries) => entries,
        Err(error) => panic!("examples/admission/manifest.json must parse: {error}"),
    }
}

/// The profile contract, or a panic naming the parse failure.
pub fn profile() -> Profile {
    match corpus::profile_contract() {
        Ok(profile) => profile,
        Err(error) => panic!("the profile must parse: {error}"),
    }
}

/// The source of one entry, or a panic naming the read failure.
pub fn source(entry: &Entry) -> String {
    match corpus::source(entry) {
        Ok(source) => source,
        Err(error) => panic!("{error}"),
    }
}
