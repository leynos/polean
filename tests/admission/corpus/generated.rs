//! Sources whose bytes a `.rego` file cannot carry safely.
//!
//! Three of these are evidence that the frontend refuses a byte sequence a
//! committed file cannot round-trip through ordinary tooling: a leading BOM, a
//! bare carriage return, and a line over the frontend's 1,024-column limit.
//! The NUL fixture is the recognizer's boundary rather than the frontend's: the
//! frontend reads a NUL inside a comment as the end of its input, so it parses
//! and admits the prefix while the recognizer still sees the submitted bytes
//! and refuses the NUL. The fifth, CRLF line endings, is admitted, so it also
//! shows that permitted trivia does not change the outcome.
//!
//! Each function is named in `examples/admission/manifest.json` by name, so the
//! corpus can list them without a file path. The functions derive every
//! variant from one baseline, which keeps the single deliberate difference
//! visible.

/// Every generated fixture name, in manifest order.
pub const NAMES: [&str; 5] = [
    "leading_bom",
    "bare_carriage_return",
    "nul_in_comment",
    "over_long_line",
    "crlf",
];

/// The baseline the committed fixtures also use, without a leading comment.
pub const BASELINE: &str = r#"package authz

import rego.v1

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
}
"#;

/// The text of the generated fixture called `name`, if there is one.
#[must_use]
pub fn source(name: &str) -> Option<String> {
    match name {
        "leading_bom" => Some(format!("\u{feff}{BASELINE}")),
        "bare_carriage_return" => Some(BASELINE.replace("\nimport rego.v1", "\rimport rego.v1")),
        // The NUL sits inside the trailing comment, and the rule after it is
        // syntax the frontend never reads. The prefix is admissible, so a
        // refusal can only come from the recognizer seeing the raw bytes.
        "nul_in_comment" => Some(format!(
            "{BASELINE}# an embedded nul hides the trailing rule \u{0}\nallow if {{\n    \
             input.subject.role == \"root\"\n}}\n"
        )),
        "over_long_line" => Some(BASELINE.replace(
            "    input.action == \"write\"",
            &format!("    \"{}\" == \"x\"", "x".repeat(1_100)),
        )),
        "crlf" => Some(BASELINE.replace('\n', "\r\n")),
        _ => None,
    }
}
