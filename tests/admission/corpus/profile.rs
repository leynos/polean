//! The parts of `contracts/profile.json` that the admission spike enforces.
//!
//! The contract is a machine-readable input, so it is read strictly: a missing
//! key, a mistyped value, or an atom that is not a non-empty string is an error
//! rather than something quietly defaulted. Only the sections EP-M2 needs are
//! decoded here, so this reader neither claims nor widens the contract's other
//! sections.
//!
//! The atoms are keyed by their canonical source text, which is exactly the
//! string the profile walk renders from an admitted comparison. Keeping the
//! canonical text in the contract, rather than duplicating it in the spike,
//! means EP-M3's renderer and the recognizer share one source of truth (D13).

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value as Json};

/// The subset of the profile the walk enforces.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Profile {
    /// The only package the module may declare.
    pub package: String,
    /// The only import the module may declare, written dot-separated.
    pub import: String,
    /// The rule the profile's entrypoint names, without its package path.
    pub rule_name: String,
    /// The only value the default rule may assign.
    pub default: bool,
    /// The fewest non-default rules a module may declare.
    pub rule_min: usize,
    /// The most non-default rules a module may declare.
    pub rule_max: usize,
    /// The fewest literals one rule body may declare.
    pub body_min: usize,
    /// The most literals one rule body may declare.
    pub body_max: usize,
    /// Every registered atom, keyed by its canonical source text.
    pub atoms: BTreeMap<String, String>,
}

impl Profile {
    /// The registered atom names, without their canonical texts.
    #[must_use]
    pub fn atom_names(&self) -> BTreeSet<String> { self.atoms.values().cloned().collect() }

    /// The atom name registered for `canonical`, if there is one.
    #[must_use]
    pub fn atom_name(&self, canonical: &str) -> Option<&str> {
        self.atoms.get(canonical).map(String::as_str)
    }

    /// The canonical source text registered for `name`, if there is one.
    ///
    /// The contract keys the table by canonical text, so this reverses it. The
    /// recognizer renders tokens from the contract rather than from a table of
    /// its own (D13), which is why the lookup lives here.
    #[must_use]
    pub fn atom_text(&self, name: &str) -> Option<&str> {
        self.atoms
            .iter()
            .find(|(_, registered)| registered.as_str() == name)
            .map(|(canonical, _)| canonical.as_str())
    }
}

/// Read the contract's enforceable subset.
///
/// # Errors
///
/// Returns a description of the first missing, mistyped, or contradictory key.
pub fn profile(text: &str) -> Result<Profile, String> {
    let value: Json =
        serde_json::from_str(text).map_err(|error| format!("the profile is not JSON: {error}"))?;
    let root = object(&value, "the profile")?;
    let rules = object(required(root, "rules")?, "`rules`")?;
    let package = string(root, "package")?;
    Ok(Profile {
        package: package.to_owned(),
        import: string(root, "import")?.to_owned(),
        rule_name: rule_name(string(root, "entrypoint")?, package)?,
        default: boolean(root, "default")?,
        rule_min: count(rules, "min")?,
        rule_max: count(rules, "max")?,
        body_min: count(rules, "body_min")?,
        body_max: count(rules, "body_max")?,
        atoms: atoms(root)?,
    })
}

/// Read the rule name from the entrypoint's `data.<package>.<rule>` path.
fn rule_name(entrypoint: &str, package: &str) -> Result<String, String> {
    let path = entrypoint
        .strip_prefix("data.")
        .ok_or_else(|| format!("the entrypoint `{entrypoint}` is not a `data.` path"))?;
    let suffix = path
        .strip_prefix(package)
        .and_then(|rest| rest.strip_prefix('.'))
        .ok_or_else(|| format!("the entrypoint `{entrypoint}` is not under `data.{package}`"))?;
    if suffix.is_empty() || suffix.contains('.') {
        return Err(format!(
            "the entrypoint `{entrypoint}` does not name one rule in `{package}`"
        ));
    }
    Ok(suffix.to_owned())
}

/// Read the atom table, rejecting an empty text or name.
fn atoms(root: &Map<String, Json>) -> Result<BTreeMap<String, String>, String> {
    let table = object(required(root, "atoms")?, "`atoms`")?;
    let mut atoms = BTreeMap::new();
    for (name, value) in table {
        let canonical = value
            .as_str()
            .ok_or_else(|| format!("atom `{name}` is not a string"))?;
        if name.is_empty() || canonical.is_empty() {
            return Err("an atom name or text is empty".to_owned());
        }
        if let Some(previous) = atoms.insert(canonical.to_owned(), name.clone()) {
            return Err(format!(
                "atoms `{previous}` and `{name}` share the canonical text `{canonical}`"
            ));
        }
    }
    Ok(atoms)
}

/// Read a JSON object.
fn object<'a>(value: &'a Json, what: &str) -> Result<&'a Map<String, Json>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{what} is not a JSON object"))
}

/// Read a required member.
fn required<'a>(map: &'a Map<String, Json>, key: &str) -> Result<&'a Json, String> {
    map.get(key)
        .ok_or_else(|| format!("the profile has no `{key}` key"))
}

/// Read a required string member.
fn string<'a>(map: &'a Map<String, Json>, key: &str) -> Result<&'a str, String> {
    required(map, key)?
        .as_str()
        .ok_or_else(|| format!("`{key}` is not a string"))
}

/// Read a required boolean member.
fn boolean(map: &Map<String, Json>, key: &str) -> Result<bool, String> {
    required(map, key)?
        .as_bool()
        .ok_or_else(|| format!("`{key}` is not a boolean"))
}

/// Read a required non-negative integer member as a platform count.
fn count(map: &Map<String, Json>, key: &str) -> Result<usize, String> {
    let value = required(map, key)?
        .as_u64()
        .ok_or_else(|| format!("`{key}` is not a non-negative integer"))?;
    usize::try_from(value).map_err(|error| format!("`{key}` is too large: {error}"))
}
