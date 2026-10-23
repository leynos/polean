//! Bounded sweep shapes for the tested-example contract tests.
//!
//! These build Markdown documents for the sweeps in
//! `tests/documentation_examples_tests.rs`. They live beside the loader rather
//! than inside its module because they are test scaffolding, not part of the
//! loader's contract.
//!
//! Every bound here is deliberately explicit. `opening_fence` accepts any run
//! of three or more fence characters and sets no upper limit, so no table in
//! this module enumerates the parser's whole input domain, and none of the
//! sweeps built from them should be described as if it did.

/// Largest indentation the bounded sweeps vary.
///
/// This is the loader's own bound: up to three leading spaces before a fence,
/// its marker, or a marked body line, with a deeper run treated as an indented
/// code block. The sweeps vary indentation across exactly that supported range
/// rather than a wider one.
pub const MAXIMUM_SWEPT_INDENT: usize = 3;

/// Opening runs paired with a closing run at least as long as the opener.
///
/// Bounded, not exhaustive. Runs longer than the largest pair here are accepted
/// by the loader and lie outside this sweep. The range above seven is included
/// because the loader's floor is the only bound on a run, which makes longer
/// runs the part a reader is most likely to assume is already covered.
pub const ROUND_TRIP_RUN_PAIRS: &[(usize, usize)] = &[
    (3, 3),
    (3, 5),
    (4, 4),
    (4, 6),
    (5, 5),
    (5, 7),
    (6, 6),
    (7, 7),
    (8, 8),
    (9, 10),
];

/// Opening runs paired with a closing run exactly one character shorter.
pub const SHORT_CLOSER_RUN_PAIRS: &[(usize, usize)] =
    &[(4, 3), (5, 4), (6, 5), (7, 6), (8, 7), (9, 8)];

/// Opening run pairs at each end of the range, for the identifier sweeps.
pub const IDENTIFIER_RUN_PAIRS: &[(usize, usize)] = &[(3, 3), (9, 10)];

/// Valid marker identifiers the identifier sweep round-trips exactly.
///
/// Real identifiers are slugs, so these vary in length, separators, and
/// trailing digits, and each is asserted back at every representative shape.
pub const VALID_IDENTIFIERS: &[&str] = &["sweep", "sweep-two", "sweep.three.v1", "readme-1-2-3"];

/// One fence shape in a bounded sweep.
#[derive(Clone, Copy)]
pub struct FenceShape {
    /// Fence character, either a backtick or a tilde.
    character: char,
    /// Number of fence characters in the opening run.
    opening_run: usize,
    /// Number of fence characters in the closing run.
    closing_run: usize,
    /// Leading spaces before the opening run, which is also the column the
    /// fence's own body lines sit at.
    pub opening_indent: usize,
    /// Leading spaces before the closing run.
    closing_indent: usize,
}

impl FenceShape {
    /// Describe this shape for a failure message.
    ///
    /// Every generated dimension is named, so a failing case states the
    /// configuration that produced it rather than only the document text.
    #[must_use]
    pub fn describe(&self) -> String {
        format!(
            "char={} opening_run={} closing_run={} opening_indent={} closing_indent={}",
            self.character,
            self.opening_run,
            self.closing_run,
            self.opening_indent,
            self.closing_indent,
        )
    }

    /// Emit `body` as a marked example fenced in this shape.
    ///
    /// The marker stays at column zero. A marker's own indentation is measured
    /// from column zero by this loader, so indenting it would exercise the
    /// documented container gap rather than the fence rules under test.
    #[must_use]
    pub fn marked_example(&self, identifier: &str, body: &[String]) -> String {
        let fence = |run: usize| self.character.to_string().repeat(run);
        let mut document = String::new();
        document.push_str("<!-- tested-example: ");
        document.push_str(identifier);
        document.push_str(" -->\n");
        document.push_str(&" ".repeat(self.opening_indent));
        document.push_str(&fence(self.opening_run));
        document.push_str("bash\n");
        for line in body {
            document.push_str(line);
            document.push('\n');
        }
        document.push_str(&" ".repeat(self.closing_indent));
        document.push_str(&fence(self.closing_run));
        document.push('\n');
        document
    }
}

/// A body variant the round-trip sweep places inside every fence shape.
#[derive(Clone, Copy)]
pub struct BodyVariant {
    /// Name used in failure messages.
    name: &'static str,
    /// Lines to place, each paired with the columns to subtract from the
    /// fence's own indentation.
    lines: &'static [(usize, &'static str)],
}

impl BodyVariant {
    /// Name this variant for a failure message.
    #[must_use]
    pub const fn name(&self) -> &'static str { self.name }

    /// Place the variant's lines inside a fence indented by `indent` columns.
    ///
    /// Each line is indented to the fence's own column, except where
    /// subtracting its reduction would take it closer to column zero, which is
    /// the deliberately ragged case. The loader removes only the spaces a line
    /// actually carries, so the text comes back unchanged either way.
    #[must_use]
    pub fn placed(&self, indent: usize) -> Vec<String> {
        self.lines
            .iter()
            .map(|&(reduction, text)| {
                format!("{}{text}", " ".repeat(indent.saturating_sub(reduction)))
            })
            .collect()
    }

    /// The exact body the loader should return for the placed lines.
    #[must_use]
    pub fn expected(&self) -> String {
        let mut body = String::new();
        for &(_, text) in self.lines {
            body.push_str(text);
            body.push('\n');
        }
        body
    }
}

/// Body variants the round-trip sweep places inside every fence shape.
///
/// These are the body shapes a contracted example takes: empty, a single line,
/// several lines with a blank one among them, text outside the Latin alphabet,
/// and lines placed closer to column zero than their fence.
pub const BODY_VARIANTS: &[BodyVariant] = &[
    BodyVariant {
        name: "empty",
        lines: &[],
    },
    BodyVariant {
        name: "single line",
        lines: &[(0, "make design-check")],
    },
    BodyVariant {
        name: "multiline",
        lines: &[(0, "make lint"), (0, ""), (0, "make test")],
    },
    BodyVariant {
        name: "multibyte",
        lines: &[(0, "日本語のコマンド")],
    },
    BodyVariant {
        name: "ragged",
        lines: &[(0, "make design-check"), (1, "make lint"), (3, "make test")],
    },
];

/// Every shape for one fence character and run pair, with both indentation
/// dimensions varied independently.
#[must_use]
pub fn indent_variants(character: char, run_pair: (usize, usize)) -> Vec<FenceShape> {
    let (opening_run, closing_run) = run_pair;
    let mut shapes = Vec::new();
    for opening_indent in 0..=MAXIMUM_SWEPT_INDENT {
        for closing_indent in 0..=MAXIMUM_SWEPT_INDENT {
            shapes.push(FenceShape {
                character,
                opening_run,
                closing_run,
                opening_indent,
                closing_indent,
            });
        }
    }
    shapes
}

/// Every shape in a run-pair table, for both fence characters.
#[must_use]
pub fn fence_shapes(run_pairs: &[(usize, usize)]) -> Vec<FenceShape> {
    let mut shapes = Vec::new();
    for character in ['`', '~'] {
        for &run_pair in run_pairs {
            shapes.extend(indent_variants(character, run_pair));
        }
    }
    shapes
}
