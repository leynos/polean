//! The canonical token sequence of an admitted rule list.
//!
//! The renderer turns a rule list and the profile's atom table into the exact
//! tokens a source must spell, plus the one property of the gaps that is not
//! visible in the tokens: a body literal other than the first must begin after
//! a line feed (D10). The tokens are rendered from `contracts/profile.json`,
//! never from a table of the renderer's own, so an escaped spelling of a
//! registered atom cannot match (D13).
//!
//! The renderer never produces a source and never parses one. It is the only
//! place that knows which gaps must contain a line feed, which keeps that rule
//! out of the verified kernel and inside a function a property test can
//! exercise.

use crate::corpus::Profile;

/// The canonical tokens of one admitted module.
#[derive(Debug)]
pub struct Tokens<'a> {
    /// Each token's exact text, in canonical order.
    pub text: Vec<&'a str>,
    /// Whether the gap before the token must contain a line feed.
    pub needs_newline: Vec<bool>,
}

impl<'a> Tokens<'a> {
    /// The tokens as the byte slices the kernel takes.
    #[must_use]
    pub fn bytes(&self) -> Vec<&'a [u8]> {
        self.text.iter().map(|token| token.as_bytes()).collect()
    }
}

/// Accumulates tokens and their gap rules.
#[derive(Default)]
struct Builder<'a> {
    /// The tokens rendered so far.
    text: Vec<&'a str>,
    /// The gap rule for each token rendered so far.
    needs_newline: Vec<bool>,
}

impl<'a> Builder<'a> {
    /// Append one token and its gap rule.
    fn push(&mut self, token: &'a str, needs_newline: bool) {
        self.text.push(token);
        self.needs_newline.push(needs_newline);
    }

    /// Append the tokens of one atom comparison.
    ///
    /// Only the first token of a literal that is not the body's first carries
    /// the line-feed rule; the rest of the literal stays on that line.
    fn literal(&mut self, canonical: &'a str, first: bool) -> Result<(), String> {
        for (index, part) in split(canonical)?.into_iter().enumerate() {
            self.push(part, !first && index == 0);
        }
        Ok(())
    }

    /// Finish the token list.
    fn finish(self) -> Tokens<'a> {
        Tokens {
            text: self.text,
            needs_newline: self.needs_newline,
        }
    }
}

/// Render the canonical tokens of `ir` under `profile`.
///
/// # Errors
///
/// Returns a description of the first atom the rule list names that the profile
/// does not register, or of an atom text that cannot be split into tokens. The
/// walk rejects both first, so this fails closed rather than silently rendering
/// a different module.
pub fn canonical_tokens<'a>(
    profile: &'a Profile,
    ir: &[Vec<String>],
) -> Result<Tokens<'a>, String> {
    let mut builder = Builder::default();
    builder.push("package", false);
    builder.push(&profile.package, false);
    builder.push("import", false);
    builder.push(&profile.import, false);
    builder.push("default", false);
    builder.push(&profile.rule_name, false);
    builder.push(":=", false);
    builder.push(if profile.default { "true" } else { "false" }, false);
    for rule in ir {
        builder.push(&profile.rule_name, false);
        builder.push("if", false);
        builder.push("{", false);
        for (index, atom) in rule.iter().enumerate() {
            let canonical = profile
                .atom_text(atom)
                .ok_or_else(|| format!("the rule list names the unregistered atom `{atom}`"))?;
            builder.literal(canonical, index == 0)?;
        }
        builder.push("}", false);
    }
    Ok(builder.finish())
}

/// Split one atom's canonical text at its separating spaces, keeping a quoted
/// string whole so that a value containing a space stays one token.
fn split(canonical: &str) -> Result<Vec<&str>, String> {
    let bytes = canonical.as_bytes();
    let mut parts = Vec::new();
    let mut at = 0;
    while let Some(&byte) = bytes.get(at) {
        if is_space(byte) {
            at += 1;
            continue;
        }
        let start = at;
        at = if byte == b'"' {
            quoted_end(canonical, at)?
        } else {
            bare_end(bytes, at)
        };
        let part = canonical
            .get(start..at)
            .ok_or_else(|| format!("the canonical text `{canonical}` splits off a character"))?;
        parts.push(part);
    }
    if parts.is_empty() {
        return Err(format!("the canonical text `{canonical}` has no tokens"));
    }
    Ok(parts)
}

/// Whether a byte separates tokens in a canonical atom text.
const fn is_space(byte: u8) -> bool { byte == b' ' || byte == b'\t' }

/// The offset just past the bare token that starts at `at`.
fn bare_end(bytes: &[u8], mut at: usize) -> usize {
    while let Some(&byte) = bytes.get(at) {
        if is_space(byte) {
            break;
        }
        at += 1;
    }
    at
}

/// The offset just past the quoted string that starts at `at`.
///
/// A backslash escapes the next byte, so `"wr\"ite"` stays one token and an
/// unterminated string is a contract fault rather than a silent truncation.
fn quoted_end(canonical: &str, mut at: usize) -> Result<usize, String> {
    let bytes = canonical.as_bytes();
    at += 1;
    while let Some(&byte) = bytes.get(at) {
        match byte {
            b'"' => return Ok(at + 1),
            b'\\' => at += 2,
            _ => at += 1,
        }
    }
    Err(format!(
        "the canonical text `{canonical}` has an unterminated string"
    ))
}

#[cfg(test)]
mod tests {
    use super::split;

    #[test]
    fn a_quoted_string_with_a_space_stays_one_token() {
        let parts = match split(r#"input.action == "write now""#) {
            Ok(parts) => parts,
            Err(error) => panic!("{error}"),
        };
        assert_eq!(
            parts,
            ["input.action", "==", "\"write now\""],
            "a quoted value must stay one token"
        );
    }

    #[test]
    fn an_unterminated_string_is_a_contract_fault() {
        assert!(split("input.action == \"write").is_err());
    }
}
