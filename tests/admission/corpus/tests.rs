//! Tests that the corpus and its manifest are in bijection.
//!
//! These check the spike's own contract, not Regorus: every committed fixture
//! is described, every description resolves, the declared coverage is the whole
//! inventory, and the hand-written IR uses only registered atoms. The outcome
//! test that spends these entries lives in [`crate::corpus::outcomes`].

use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    path::Path,
};

use crate::{
    corpus::{self, Entry, Expected, Origin, ReasonCode, Stage},
    inventory::NodeKind,
};

/// The parsed manifest, or a panic naming the first parse failure.
fn manifest() -> Vec<Entry> {
    match corpus::entries() {
        Ok(entries) => entries,
        Err(error) => panic!("examples/admission/manifest.json must parse: {error}"),
    }
}

/// Every `*.rego` file under `root`, as repository-relative paths.
fn rego_files(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let entries = match std::fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) => panic!("cannot read {}: {error}", directory.display()),
        };
        for dir_entry in entries {
            let path = match dir_entry {
                Ok(resolved) => resolved.path(),
                Err(error) => panic!("cannot read {}: {error}", directory.display()),
            };
            if path.is_dir() {
                pending.push(path);
            } else if path.extension() == Some(OsStr::new("rego")) {
                found.push(relative(&path));
            }
        }
    }
    found.sort();
    found
}

/// The repository-relative form of an absolute path under the crate root.
fn relative(path: &Path) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    match path.strip_prefix(root) {
        Ok(relative) => relative.to_string_lossy().into_owned(),
        Err(error) => panic!("{} is outside the repository: {error}", path.display()),
    }
}

/// The labels of every entry with the given origin discriminant.
fn file_labels(entries: &[Entry], prefix: &str) -> BTreeSet<String> {
    entries
        .iter()
        .filter_map(|entry| match &entry.origin {
            Origin::File(path) if path.starts_with(prefix) => Some(path.clone()),
            Origin::File(_) | Origin::Generated(_) => None,
        })
        .collect()
}

#[test]
fn every_entry_has_a_distinct_label() {
    let entries = manifest();
    assert!(!entries.is_empty(), "the corpus must not be empty");
    let mut seen = BTreeSet::new();
    for entry in &entries {
        let label = entry.label();
        assert!(
            seen.insert(label.clone()),
            "duplicate manifest label `{label}`"
        );
    }
}

#[test]
fn every_committed_fixture_appears_exactly_once() {
    let entries = manifest();
    let declared = file_labels(&entries, "examples/admission/");
    let committed: BTreeSet<String> = rego_files(&corpus::repository_path("examples/admission"))
        .into_iter()
        .collect();
    let undeclared: Vec<&String> = committed.difference(&declared).collect();
    let missing: Vec<&String> = declared.difference(&committed).collect();
    assert!(
        undeclared.is_empty() && missing.is_empty(),
        "manifest and fixtures disagree: undeclared {undeclared:?}, missing {missing:?}"
    );
}

#[test]
fn every_tenant_write_source_is_referenced() {
    let entries = manifest();
    let declared = file_labels(&entries, "examples/tenant-write/");
    let committed: BTreeSet<String> = rego_files(&corpus::repository_path("examples/tenant-write"))
        .into_iter()
        .collect();
    assert_eq!(
        declared, committed,
        "the manifest must reference every tenant-write source and no other"
    );
}

#[test]
fn every_generated_name_resolves_to_a_source() {
    let entries = manifest();
    let declared: BTreeSet<String> = entries
        .iter()
        .filter_map(|entry| match &entry.origin {
            Origin::Generated(name) => Some(name.clone()),
            Origin::File(_) => None,
        })
        .collect();
    let named: BTreeSet<String> = corpus::GENERATED_NAMES
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    assert_eq!(declared, named, "generated names and functions must agree");
    for entry in entries {
        if matches!(entry.origin, Origin::Generated(_)) {
            assert!(
                corpus::source(&entry).is_ok(),
                "generated fixture `{}` must resolve",
                entry.label()
            );
        }
    }
}

#[test]
fn every_manifest_file_exists() {
    for entry in manifest() {
        if let Origin::File(path) = &entry.origin {
            assert!(
                corpus::repository_path(path).is_file(),
                "`{path}` does not exist"
            );
        }
        if let Expected::Admitted { ir } = &entry.expected {
            assert!(
                corpus::repository_path(ir).is_file(),
                "`{ir}` does not exist"
            );
        }
    }
}

#[test]
fn every_entry_lists_a_kind_once() {
    let mut duplicated = Vec::new();
    for entry in manifest() {
        let mut seen = BTreeSet::new();
        for kind in &entry.covers {
            if !seen.insert(*kind) {
                duplicated.push(format!("{}: {}", entry.label(), kind.name()));
            }
        }
    }
    assert!(
        duplicated.is_empty(),
        "an entry repeats a covered kind: {duplicated:?}"
    );
}

#[test]
fn the_declared_covers_are_the_whole_inventory() {
    let observed: BTreeSet<NodeKind> = manifest()
        .iter()
        .flat_map(|entry| entry.covers.iter().copied())
        .chain(NodeKind::BASELINE_KINDS)
        .collect();
    let expected: BTreeSet<NodeKind> = NodeKind::ALL.into_iter().collect();
    let missing: Vec<&str> = expected
        .difference(&observed)
        .map(|kind| kind.name())
        .collect();
    assert!(
        missing.is_empty(),
        "no fixture declares these kinds: {missing:?}"
    );
}

#[test]
fn every_corpus_reason_code_has_a_fixture() {
    let used: BTreeSet<ReasonCode> = manifest()
        .iter()
        .filter_map(|entry| match &entry.expected {
            Expected::Rejected { code, .. } => Some(*code),
            Expected::Admitted { .. } => None,
        })
        .collect();
    let mut expected: BTreeSet<ReasonCode> = ReasonCode::ALL.into_iter().collect();
    for code in ReasonCode::AST_CODES {
        assert!(expected.remove(&code), "`ast.*` codes must be in `ALL`");
    }
    let unused: Vec<&str> = expected
        .difference(&used)
        .map(|code| code.as_wire())
        .collect();
    let extra: Vec<&str> = used
        .difference(&expected)
        .map(|code| code.as_wire())
        .collect();
    assert!(
        unused.is_empty() && extra.is_empty(),
        "corpus code coverage: unused {unused:?}, unexpected {extra:?}"
    );
}

#[test]
fn the_corpus_never_expects_the_adapter_stages() {
    let mut stages = BTreeSet::new();
    for entry in manifest() {
        if let Expected::Rejected { stage, .. } = &entry.expected {
            stages.insert(*stage);
        }
    }
    let unexpected: Vec<&str> = stages
        .iter()
        .filter(|stage| !matches!(stage, Stage::Parse | Stage::Profile | Stage::Recognizer))
        .map(|stage| stage.as_wire())
        .collect();
    assert!(
        unexpected.is_empty(),
        "serialize and decode failures belong to the mutation sweep, not the corpus; found \
         {unexpected:?}"
    );
}

#[test]
fn every_admitted_ir_uses_registered_atoms() {
    let atoms = match corpus::profile_atoms() {
        Ok(atoms) => atoms,
        Err(error) => panic!("the profile must list its atoms: {error}"),
    };
    for entry in manifest() {
        let Expected::Admitted { ir } = &entry.expected else {
            continue;
        };
        let rules = match corpus::ir_rules(ir) {
            Ok(rules) => rules,
            Err(error) => panic!("{error}"),
        };
        for (index, rule) in rules.iter().enumerate() {
            assert!(
                !rule.is_empty(),
                "`{ir}` rule {index} is empty; the profile requires one atom"
            );
            for atom in rule {
                assert!(
                    atoms.contains(atom),
                    "`{ir}` rule {index} names the unregistered atom `{atom}`"
                );
            }
        }
    }
}

#[test]
fn every_fixture_names_its_difference_on_the_first_line() {
    for entry in manifest() {
        let Origin::File(path) = &entry.origin else {
            continue;
        };
        // The tenant-write sources have their own header convention, and the
        // empty module has no line to carry a comment.
        if !path.starts_with("examples/admission/")
            || path == "examples/admission/module/empty.rego"
        {
            continue;
        }
        let source = match corpus::source(&entry) {
            Ok(source) => source,
            Err(error) => panic!("{error}"),
        };
        let first = source.lines().next().unwrap_or_default();
        assert!(
            first.starts_with("# Admission fixture: ") && first.ends_with('.'),
            "`{path}` must name its difference on the first line, found `{first}`"
        );
    }
}

#[test]
fn every_fixture_is_reviewable() {
    let mut long = BTreeMap::new();
    for entry in manifest() {
        let Origin::File(path) = &entry.origin else {
            continue;
        };
        let source = match corpus::source(&entry) {
            Ok(source) => source,
            Err(error) => panic!("{error}"),
        };
        let lines = source.lines().count();
        if lines >= 90 {
            long.insert(path.clone(), lines);
        }
    }
    assert!(
        long.is_empty(),
        "fixtures must stay under 90 lines: {long:?}"
    );
}
