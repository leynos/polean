//! O5: generated canonical sources are admitted as exactly their rule list.
//!
//! The corpus proves that the fixtures a person wrote behave as stated. This
//! module proves the same for modules a property generates: every rule list of
//! zero to sixteen rules of one to sixteen registered atoms, written with every
//! class of permitted trivia, must come back from admission unchanged, and must
//! satisfy the witness (O4) and the lexer cross-check (O7).
//!
//! The boundary shapes are pinned because a generator reaches them unreliably:
//! the empty rule list, the sixteen-by-sixteen maximum, seventeen rules, and
//! seventeen literals in one body. The last two must be rejected by the profile
//! rather than quietly accepted.

mod case;

#[cfg(test)]
mod tests;
