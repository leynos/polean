//! Fence primitives for the tested-example loader.
//!
//! A fence is a run of three or more backticks or tildes, optionally indented,
//! optionally followed by a language tag, and closed by a run of the same
//! character at least as long. This module holds the recognition and
//! de-indentation rules; the parser in the parent module decides what a
//! recognized fence means for the contract.
//!
//! Both `CommonMark` fence characters are supported, a backtick fence whose
//! info string contains a backtick is rejected as `CommonMark` requires, and a
//! closer may carry trailing spaces or tabs.

/// Shortest fence run that opens a fence.
const MINIMUM_FENCE: usize = 3;

/// Most leading spaces a fence, its marker, or a marked body line may carry.
///
/// `CommonMark` treats a backtick run indented further than this as an indented
/// code block, which is prose showing a fence rather than a fence.
pub(super) const MAXIMUM_INDENT: usize = 3;

/// A fence character, one of the two `CommonMark` recognizes.
///
/// Both are contracted. Recognizing only backticks would let a tilde fence
/// escape the marker check silently, which is the fail-open direction the
/// style guide's "every fenced example" rule forbids.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FenceCharacter {
    /// Backtick, the character the style guide's own examples use.
    Backtick,
    /// Tilde, the other `CommonMark` fence character.
    Tilde,
}

impl FenceCharacter {
    /// The fence character a line begins with, if it begins with either.
    pub(super) const fn of(character: char) -> Option<Self> {
        match character {
            '`' => Some(Self::Backtick),
            '~' => Some(Self::Tilde),
            _ => None,
        }
    }

    /// The character a closing run must repeat.
    pub(super) const fn as_char(self) -> char {
        match self {
            Self::Backtick => '`',
            Self::Tilde => '~',
        }
    }
}

/// An opening fence: its character, run length, indentation, and language tag.
#[derive(Clone, Copy)]
pub(super) struct Fence<'a> {
    /// Character the opening run is made of.
    pub(super) character: FenceCharacter,
    /// Number of fence characters in the opening run.
    pub(super) run: usize,
    /// Leading spaces before the opening run.
    pub(super) indent: usize,
    /// Language tag, which may be empty when the author omitted one.
    pub(super) language: &'a str,
}

/// Parse an opening fence: a run of three or more fence characters, then a
/// language.
///
/// Both `CommonMark` fence characters are recognized. A backtick fence whose
/// info string contains a backtick is not a valid fence, so such a line reports
/// `None` rather than an odd language tag; a tilde fence has no such
/// restriction.
pub(super) fn opening_fence(line: &str) -> Option<Fence<'_>> {
    let indented = strip_indent(line);
    let character = FenceCharacter::of(indented.chars().next()?)?;
    let remainder = indented.trim_start_matches(character.as_char());
    let run = indented.len().checked_sub(remainder.len())?;
    if run < MINIMUM_FENCE {
        return None;
    }
    // `remainder` begins at the run, so a later backtick is part of the info
    // string, which CommonMark forbids for a backtick fence.
    let language = remainder.trim();
    if character == FenceCharacter::Backtick && language.contains('`') {
        return None;
    }
    Some(Fence {
        character,
        run,
        indent: line.len() - indented.len(),
        language,
    })
}

/// Report whether a line closes the fence opened by `fence`.
///
/// A closer is a run of the same character as the opener, at least as long,
/// indented by no more than [`MAXIMUM_INDENT`] spaces, followed only by spaces
/// or tabs. `CommonMark` permits that trailing whitespace, and the shared
/// loader accepts it, so a document that closes a fence with a trailing space
/// is contracted rather than silently escaping the check.
pub(super) fn is_closing_fence(line: &str, fence: Fence<'_>) -> bool {
    let run = strip_indent(line).trim_end_matches([' ', '\t']);
    let remaining = run.trim_start_matches(fence.character.as_char());
    remaining.is_empty() && run.len() >= fence.run
}

/// Remove up to [`MAXIMUM_INDENT`] leading spaces, leaving further indentation.
pub(super) fn strip_indent(line: &str) -> &str {
    let spaces = line
        .len()
        .saturating_sub(line.trim_start_matches(' ').len());
    line.get(spaces.min(MAXIMUM_INDENT)..).unwrap_or_default()
}

/// Remove up to `indent` leading spaces from a marked example's body line.
///
/// The body is returned as Rust source or shell commands for execution, in
/// which leading indentation is significant rather than Markdown layout, so
/// each line is de-indented by the opening fence's own indentation. Only
/// leading spaces are removed, and no more of them than the line actually
/// carries: a line indented less than its fence keeps its remaining text
/// instead of losing content to a byte offset that assumed more indentation,
/// and a line beginning with a multibyte character is never split.
pub(super) fn strip_indent_of(line: &str, indent: usize) -> &str {
    let spaces = line
        .len()
        .saturating_sub(line.trim_start_matches(' ').len());
    line.get(spaces.min(indent)..).unwrap_or_default()
}
