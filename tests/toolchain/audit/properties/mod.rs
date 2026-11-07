//! A1 and A2: generated keys and generated malformed documents.
//!
//! A1 is the claim that the byte keys settlement compares are injective: two
//! names or two statements share a key only when they are the same name or the
//! same statement. A2 is the claim that the decoder refuses every document
//! outside the contract, so a malformed report can never reach settlement as
//! either an acceptance or a rejection.
//!
//! Both are properties rather than examples because the interesting failures
//! are the ones a hand-written case misses: a length prefix that is dropped, a
//! tag that is reused, a required field that a derive silently defaults.

mod contract;
mod keys;
mod support;
