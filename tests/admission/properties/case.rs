//! Generated modules and the canonical sources assembled from them.
//!
//! A case is a rule list plus a list of trivia classes, cycled across the
//! source's gaps. Generating the structure and the trivia separately keeps
//! shrinking syntax-valid: every rule always has at least one literal, and the
//! assembler never emits a gap class that a position cannot hold.
//!
//! The assembler reads its atoms from `contracts/profile.json` and restates the
//! line-feed rule itself. It never calls [`crate::recognizer::canonical_tokens`],
//! so a renderer fault cannot also be a fault in the source the renderer is
//! asked to recognise.

use proptest::{collection::vec as strategy_vec, prelude::Strategy, sample::select};

use crate::corpus::Profile;

/// The most rules a generated case may hold.
pub const MAX_RULES: usize = 16;

/// The most literals a generated rule may hold.
pub const MAX_ATOMS: usize = 16;

/// One class of permitted trivia.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Gap {
    /// A single space.
    Space,
    /// A single tab.
    Tab,
    /// One line feed.
    Newline,
    /// Two line feeds, making a blank line.
    BlankLine,
    /// A comment that ends at its line feed.
    Comment,
    /// A carriage-return line feed.
    Crlf,
}

impl Gap {
    /// Every class, in a fixed order.
    pub const ALL: [Self; 6] = [
        Self::Space,
        Self::Tab,
        Self::Newline,
        Self::BlankLine,
        Self::Comment,
        Self::Crlf,
    ];

    /// The name the property reports the class by.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Space => "space",
            Self::Tab => "tab",
            Self::Newline => "newline",
            Self::BlankLine => "blank-line",
            Self::Comment => "comment",
            Self::Crlf => "crlf",
        }
    }

    /// Whether the class holds a line feed.
    #[must_use]
    pub const fn has_newline(self) -> bool {
        matches!(
            self,
            Self::Newline | Self::BlankLine | Self::Comment | Self::Crlf
        )
    }

    /// The trivia the class renders as.
    #[must_use]
    pub const fn text(self) -> &'static str {
        match self {
            Self::Space => " ",
            Self::Tab => "\t",
            Self::Newline => "\n",
            Self::BlankLine => "\n\n",
            Self::Comment => " # generated comment\n",
            Self::Crlf => "\r\n",
        }
    }
}

/// One generated module: its rule list and the trivia cycled between tokens.
#[derive(Clone, Debug)]
pub struct Case {
    /// One atom name per literal, one vector per rule.
    pub ir: Vec<Vec<String>>,
    /// The trivia classes, cycled across the source's gaps.
    pub gaps: Vec<Gap>,
}

impl Case {
    /// A rectangular rule list with `atoms` literals in each of `rules` rules.
    ///
    /// The registered atoms are taken in the contract's order and cycled when
    /// the table runs out. The pinned boundary cases use this shape.
    #[must_use]
    pub fn rectangular(profile: &Profile, rules: usize, atoms: usize) -> Self {
        let names: Vec<String> = profile.atom_names().into_iter().collect();
        let ir = (0..rules)
            .map(|_| names.iter().cycle().take(atoms).cloned().collect())
            .collect();
        Self {
            ir,
            gaps: Gap::ALL.to_vec(),
        }
    }
}

/// The strategy for generated cases.
pub fn strategy(profile: &Profile) -> impl Strategy<Value = Case> {
    let names: Vec<String> = profile.atom_names().into_iter().collect();
    let rule = strategy_vec(select(names), 1..=MAX_ATOMS);
    let ir = strategy_vec(rule, 0..=MAX_RULES);
    let gaps = strategy_vec(select(Gap::ALL.to_vec()), 1..=8);
    (ir, gaps).prop_map(|(rules, trivia)| Case {
        ir: rules,
        gaps: trivia,
    })
}

/// The canonical source of `case`.
///
/// The atom texts come from the contract and are split at spaces here, so the
/// assembler shares no code with the renderer. Only a literal other than its
/// body's first begins a new line, which is the rule the renderer turns into
/// `needs_newline`.
#[must_use]
pub fn source(profile: &Profile, case: &Case) -> String {
    let mut tokens: Vec<(&str, bool)> = vec![
        ("package", false),
        (profile.package.as_str(), false),
        ("import", false),
        (profile.import.as_str(), false),
        ("default", false),
        (profile.rule_name.as_str(), false),
        (":=", false),
        (if profile.default { "true" } else { "false" }, false),
    ];
    for rule in &case.ir {
        tokens.push((profile.rule_name.as_str(), false));
        tokens.push(("if", false));
        tokens.push(("{", false));
        for (index, atom) in rule.iter().enumerate() {
            let canonical = profile.atom_text(atom).unwrap_or_else(|| {
                panic!("the generated case names the unregistered atom `{atom}`")
            });
            for (part_index, part) in canonical.split(' ').enumerate() {
                tokens.push((part, index > 0 && part_index == 0));
            }
        }
        tokens.push(("}", false));
    }
    join(case, &tokens)
}

/// Join the tokens with the cycled trivia.
fn join(case: &Case, tokens: &[(&str, bool)]) -> String {
    let mut source = String::new();
    let mut gaps = case.gaps.iter().cycle();
    for (index, &(token, must_newline)) in tokens.iter().enumerate() {
        if index > 0 {
            source.push_str(gap_text(gaps.next().copied(), must_newline));
        }
        source.push_str(token);
    }
    source
}

/// The trivia before a token, upgraded to a line feed when the position needs
/// one and the cycled class lacks it.
const fn gap_text(cycled: Option<Gap>, must_newline: bool) -> &'static str {
    match cycled {
        Some(gap) if must_newline && !gap.has_newline() => "\n",
        Some(gap) => gap.text(),
        None if must_newline => "\n",
        None => " ",
    }
}

#[cfg(test)]
mod tests {
    use super::Gap;

    #[test]
    fn every_trivia_class_has_its_documented_newline() {
        for gap in Gap::ALL {
            assert_eq!(
                gap.text().contains('\n'),
                gap.has_newline(),
                "the {} class must agree with its line-feed flag",
                gap.name()
            );
        }
    }
}
