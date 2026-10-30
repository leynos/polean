//! O4: an independent checker for a recognizer witness.
//!
//! `recognize` returns offsets, and a wrong offset list is only visible when
//! something other than the kernel reads it. The witness slices the source at
//! the reported offsets and validates each gap on its own, so it shares the
//! kernel's rules but none of its control flow: a mistake in the kernel's loop
//! cannot also be a mistake in the witness's slicing.
//!
//! The witness is the oracle for every `Ok` the corpus and the properties see,
//! and it is what catches the seeded kernel faults in the tests: a kernel with
//! one rule removed returns offsets the witness refuses.

/// Confirm that `offsets` decompose `src` into `toks`.
///
/// # Errors
///
/// Returns a description of the first rule the witness finds broken.
pub fn confirm(
    src: &[u8],
    toks: &[&[u8]],
    needs_newline: &[bool],
    offsets: &[usize],
) -> Result<(), String> {
    if toks.len() != offsets.len() || toks.len() != needs_newline.len() {
        return Err(format!(
            "the witness has {} offsets and {} flags for {} tokens",
            offsets.len(),
            needs_newline.len(),
            toks.len()
        ));
    }
    let mut cursor = 0_usize;
    for (index, (token, &offset)) in toks.iter().zip(offsets).enumerate() {
        if token.is_empty() {
            return Err(format!("token {index} is empty"));
        }
        if offset < cursor {
            return Err(format!(
                "token {index} starts at {offset}, before the previous token ends at {cursor}"
            ));
        }
        if index > 0 && offset == cursor {
            return Err(format!("the gap before token {index} is empty"));
        }
        check_gap(src, cursor, offset, false)
            .map_err(|problem| format!("the gap before token {index}: {problem}"))?;
        if needs_newline.get(index).copied().unwrap_or(false)
            && !contains_newline(src, cursor, offset)
        {
            return Err(format!("the gap before token {index} has no line feed"));
        }
        if src.get(offset..offset.saturating_add(token.len())) != Some(token) {
            return Err(format!("token {index} is not the bytes at offset {offset}"));
        }
        cursor = offset.saturating_add(token.len());
    }
    check_gap(src, cursor, src.len(), true)
        .map_err(|problem| format!("the trailing gap: {problem}"))
}

/// Whether the range `from..to` holds a line feed.
fn contains_newline(src: &[u8], from: usize, to: usize) -> bool {
    src.get(from..to).unwrap_or_default().contains(&b'\n')
}

/// Validate one gap of trivia.
///
/// `last` says no token follows, which permits a comment to end at the end of
/// the source; otherwise a comment must be followed by its line feed inside the
/// gap, because a token on a comment's line belongs to the comment.
fn check_gap(src: &[u8], from: usize, to: usize, last: bool) -> Result<(), String> {
    if from > to || to > src.len() {
        return Err(format!("the gap {from}..{to} is not a source range"));
    }
    let gap = src
        .get(from..to)
        .ok_or_else(|| format!("the gap {from}..{to} is not a source range"))?;
    let mut at = 0;
    while let Some(&byte) = gap.get(at) {
        match byte {
            b' ' | b'\t' | b'\n' => at += 1,
            b'\r' if gap.get(at.saturating_add(1)) == Some(&b'\n') => at += 1,
            b'#' => {
                let end = comment_end(gap, at)?;
                if end == gap.len() && !last {
                    return Err("a comment is not followed by a line feed".to_owned());
                }
                at = end;
            }
            _ => {
                return Err(format!(
                    "byte {byte:#04x} at offset {} is not trivia",
                    from.saturating_add(at)
                ));
            }
        }
    }
    Ok(())
}

/// Find the end of the comment that starts at `hash`.
fn comment_end(gap: &[u8], hash: usize) -> Result<usize, String> {
    let start = hash.saturating_add(1);
    let mut end = start;
    while let Some(&byte) = gap.get(end) {
        if is_line_end(gap, end) {
            break;
        }
        if !matches!(byte, b'\t' | 0x20..=0x7e) {
            return Err(format!(
                "the comment holds the byte {byte:#04x} at gap offset {end}"
            ));
        }
        end += 1;
    }
    let body = gap.get(start..end).unwrap_or_default();
    if super::has_metadata(body) {
        return Err("the comment spells METADATA".to_owned());
    }
    Ok(end)
}

/// Whether the byte at `at` ends a line: a line feed, or a carriage return
/// that precedes one.
fn is_line_end(gap: &[u8], at: usize) -> bool {
    match gap.get(at) {
        Some(&b'\n') => true,
        Some(&b'\r') => gap.get(at.saturating_add(1)) == Some(&b'\n'),
        Some(_) | None => false,
    }
}
