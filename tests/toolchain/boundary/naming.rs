//! The character rule shared by unit names, hostile nonces, and report tokens.
//!
//! A value that reaches a systemd unit name, a command-line token, or a report
//! line must be a bare name: ASCII letters, digits, `-`, and `_`. Keeping the
//! rule in one place means two callers cannot drift into accepting different
//! alphabets, and a test can exercise the alphabet without a job.

/// Whether `text` is a non-empty bare name.
#[must_use]
pub fn is_bare_name(text: &str) -> bool { !text.is_empty() && text.chars().all(is_name_character) }

/// Whether `character` may appear in a bare name.
#[must_use]
pub fn is_name_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '-' || character == '_'
}
