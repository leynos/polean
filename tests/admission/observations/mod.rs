//! What the pinned Regorus does with every corpus fixture.
//!
//! One line per fixture records three things the feasibility record needs: whether
//! `Engine::add_policy` accepted the source, whether `get_ast_as_json` then
//! succeeded, and the multiset of typed AST node kinds the module contains. It is
//! an observation of a frontend, not an admission decision: a fixture that parses
//! is not thereby admitted to the profile.
//!
//! The snapshot is a change detector. A Regorus bump that changes which sources
//! parse, or which node kinds they contain, changes the snapshot and must be
//! reviewed. EP-M2 and EP-M3 extend each line with the walk and recognizer
//! results; the observation fields stay as they are.

use std::fmt::Write as _;

use crate::{
    corpus::{self, Entry},
    inventory::{Observation, observe},
};

/// Render the observation snapshot: one sorted line per manifest entry.
pub fn render() -> Result<String, String> {
    let entries = corpus::entries()?;
    let mut lines = Vec::with_capacity(entries.len());
    for entry in &entries {
        lines.push((entry.label(), describe(entry)?));
    }
    lines.sort_by(|left, right| left.0.cmp(&right.0));
    let mut rendered = String::new();
    for (label, description) in lines {
        writeln!(rendered, "{label}  {description}")
            .map_err(|error| format!("cannot render the observation snapshot: {error}"))?;
    }
    Ok(rendered)
}

/// Describe one fixture's observation.
fn describe(entry: &Entry) -> Result<String, String> {
    let source = corpus::source(entry)?;
    Ok(match observe(&source) {
        Observation::Parsed(counts) => {
            let mut kinds = String::new();
            for (kind, count) in counts {
                write!(kinds, "{}={count} ", kind.name())
                    .map_err(|error| format!("cannot render node kinds: {error}"))?;
            }
            format!("regorus=parsed  kinds={}", kinds.trim_end())
        }
        Observation::SerializeFailed(message) => {
            format!("regorus=serialize-failed  {}", summarise(&message))
        }
        Observation::Rejected(message) => format!("regorus=rejected  {}", summarise(&message)),
    })
}

/// Reduce a frontend diagnostic to one reviewable clause.
///
/// The full message repeats the offending source line, which for the
/// over-long-line fixture is over a kilobyte, and for the embedded-NUL fixture
/// contains a NUL. Only the trailing `error:` clause carries the observation.
fn summarise(message: &str) -> String {
    let clause = message
        .rsplit_once("error: ")
        .map_or(message, |(_, tail)| tail);
    let first_line = clause.lines().next().unwrap_or(clause).trim();
    let mut trimmed: String = first_line.chars().take(100).collect();
    if trimmed.chars().count() < first_line.chars().count() {
        trimmed.push_str("...");
    }
    trimmed
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crate::{
        corpus,
        inventory::{NodeKind, Observation, observe},
    };

    #[test]
    fn corpus() {
        let rendered = match super::render() {
            Ok(rendered) => rendered,
            Err(error) => panic!("the observation snapshot must render: {error}"),
        };
        insta::with_settings!(
            {prepend_module_to_snapshot => false, snapshot_path => "../snapshots"},
            {
                insta::assert_snapshot!("admission__observations__corpus", rendered);
            }
        );
    }

    /// The corpus must exercise the whole inventory, not merely declare it.
    ///
    /// The declared-coverage test in `corpus::tests` says what the manifest
    /// intends. This one says what the pinned frontend actually produced, so a
    /// fixture that quietly stops producing a kind is a failure rather than a
    /// stale claim.
    #[test]
    fn the_corpus_observes_the_whole_inventory() {
        let entries = match corpus::entries() {
            Ok(entries) => entries,
            Err(error) => panic!("examples/admission/manifest.json must parse: {error}"),
        };
        let mut observed = BTreeSet::new();
        for entry in &entries {
            let source = match corpus::source(entry) {
                Ok(source) => source,
                Err(error) => panic!("{error}"),
            };
            if let Observation::Parsed(counts) = observe(&source) {
                observed.extend(counts.keys().copied());
            }
        }
        let missing: Vec<&str> = NodeKind::ALL
            .into_iter()
            .filter(|kind| !observed.contains(kind))
            .map(NodeKind::name)
            .collect();
        assert!(
            missing.is_empty(),
            "Regorus never produced these declared kinds: {missing:?}"
        );
    }
}
