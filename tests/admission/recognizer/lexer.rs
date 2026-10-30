//! O7: cross-check the canonical tokens against the pinned frontend's lexer.
//!
//! Axiom X3 says a Rego lexer, given a source that the recognizer accepts,
//! produces exactly the canonical tokens. Within this task only Regorus can be
//! asked, through `regorus::unstable::Lexer`, and only on every source the
//! recognizer accepts. That is evidence for one implementation of the axiom,
//! not a proof, and it says nothing about OPA's lexer (2.3.2 owns that).
//!
//! Regorus splits a dotted reference into an identifier and a `.` per segment,
//! and excludes a string's quotes from its token span, so a canonical token may
//! span several lexer tokens or differ from one by its quotes. Matching
//! therefore normalises the canonical token first and accumulates lexer text
//! until it equals that form, which also catches a lexer that merges two
//! canonical tokens.

use regorus::unstable::{Lexer, Source, Token, TokenKind};

/// Check that Regorus lexes `source` into exactly `tokens`.
///
/// # Errors
///
/// Returns a description of the first disagreement.
pub fn cross_check(source: &str, tokens: &[&str]) -> Result<(), String> {
    let lexed = lexed_tokens(source)?;
    let mut at = 0;
    for token in tokens {
        let wanted = without_quotes(token);
        let mut accumulated = String::new();
        while accumulated.len() < wanted.len() {
            let Some(text) = lexed.get(at) else {
                return Err(format!(
                    "the lexer ended before the canonical token `{token}`"
                ));
            };
            accumulated.push_str(text);
            at += 1;
            if !wanted.starts_with(accumulated.as_str()) {
                return Err(format!(
                    "the lexer produced `{accumulated}` where the canonical tokens have `{token}`"
                ));
            }
        }
        if accumulated.as_str() != wanted {
            return Err(format!(
                "the lexer produced `{accumulated}` where the canonical tokens have `{token}`"
            ));
        }
    }
    if at == lexed.len() {
        Ok(())
    } else {
        Err(format!(
            "the lexer produced {} tokens after the canonical sequence",
            lexed.len() - at
        ))
    }
}

/// The form of a canonical token that Regorus's lexer reports.
///
/// The source must spell a string's quotes, so the canonical token keeps them,
/// but Regorus's token span excludes them: `"write"` is lexed as `write`.
fn without_quotes(token: &str) -> &str {
    token
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or(token)
}

/// The lexer's tokens for `source`, without the end-of-file marker.
fn lexed_tokens(source: &str) -> Result<Vec<String>, String> {
    let input = Source::from_contents("admission".to_owned(), source.to_owned())
        .map_err(|error| format!("the lexer could not read the source: {error}"))?;
    let mut lexer = Lexer::new(&input);
    let mut texts = Vec::new();
    loop {
        let Token(kind, span) = lexer
            .next_token()
            .map_err(|error| format!("the lexer refused the source: {error}"))?;
        if matches!(kind, TokenKind::Eof) {
            return Ok(texts);
        }
        texts.push(span.text().to_owned());
    }
}
