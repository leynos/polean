//! Bounded sweeps over generated fence shapes.
//!
//! Each sweep states its own bounds and claims nothing past them: the loader
//! accepts any fence run of three or more characters, so a bounded table is
//! regression coverage rather than a proof over the parser's input domain.

use crate::documentation_examples::{
    TestedExample,
    parse_document,
    sweep::{
        BODY_VARIANTS,
        IDENTIFIER_RUN_PAIRS,
        MAXIMUM_SWEPT_INDENT,
        ROUND_TRIP_RUN_PAIRS,
        SHORT_CLOSER_RUN_PAIRS,
        VALID_IDENTIFIERS,
        fence_shapes,
    },
};

/// A marked example round-trips exactly across the bounded fence shapes.
///
/// This is bounded regression coverage, not a proof about the parser's whole
/// input domain. `opening_fence` accepts any run of three or more fence
/// characters, so runs longer than the sweep's largest are accepted and
/// unswept, and the same holds for any body text outside the variants below.
///
/// What the sweep does enumerate is the union of the dimensions the rules turn
/// on: both fence characters, closers equal to and longer than their opener,
/// opening and closing indentation varied independently across the supported
/// range, and five body variants including empty, multiline, non-Latin, and
/// ragged text. Each valid case asserts the identifier, language, and body
/// exactly, and names its generated shape when it fails.
#[test]
fn marked_examples_round_trip_across_bounded_fence_shapes() {
    let shapes = fence_shapes(ROUND_TRIP_RUN_PAIRS);
    assert!(
        !shapes.is_empty() && !BODY_VARIANTS.is_empty(),
        "the sweep must generate cases; an empty dimension would pass this test vacuously",
    );
    for shape in shapes {
        for variant in BODY_VARIANTS {
            let described = format!("{} variant={}", shape.describe(), variant.name());
            let document = shape.marked_example("sweep", &variant.placed(shape.opening_indent));
            let examples = parse_document("sweep.md", &document)
                .unwrap_or_else(|error| panic!("{described}: {error}"));
            assert_eq!(
                examples,
                vec![TestedExample {
                    id: "sweep".to_owned(),
                    language: "bash".to_owned(),
                    body: variant.expected(),
                }],
                "{described}",
            );
        }
    }
}

/// A closing run one character shorter leaves the fence unterminated.
///
/// The other half of the length rule: a closer must be at least as long as its
/// opener. Each case here pairs an opening run with a closing run *exactly one
/// character shorter* — four against three, five against four, and so on — so
/// the sweep exercises the immediately failing length, not every shorter run.
/// A closer several characters short is a strictly larger violation of the same
/// rule and is not enumerated here.
#[test]
fn a_closing_run_one_character_short_does_not_close_the_fence() {
    let shapes = fence_shapes(SHORT_CLOSER_RUN_PAIRS);
    assert!(
        !shapes.is_empty(),
        "the sweep must generate cases; an empty table would pass it vacuously",
    );
    for shape in shapes {
        let document = shape.marked_example("short", &["make lint".to_owned()]);
        let error = parse_document("sweep.md", &document)
            .expect_err("a shorter closing run should not close the fence");
        assert!(
            error.to_string().contains("fence is not terminated"),
            "{}: expected an unterminated rejection in '{error}'",
            shape.describe(),
        );
    }
}

/// Valid identifiers round-trip exactly across representative fence shapes.
///
/// Identifiers are read from the marker, so the marker is what varies: a
/// hyphenated slug, a dotted slug, and a digit-tailed slug, each asserted back
/// verbatim. The shapes are the two ends of the run range, which is where a
/// loader that trimmed or truncated its marker text would diverge.
#[test]
fn valid_identifiers_round_trip_across_bounded_fence_shapes() {
    let shapes = fence_shapes(IDENTIFIER_RUN_PAIRS);
    assert!(
        !shapes.is_empty() && !VALID_IDENTIFIERS.is_empty(),
        "the sweep must generate cases; an empty dimension would pass this test vacuously",
    );
    for shape in shapes {
        for identifier in VALID_IDENTIFIERS {
            let document = shape.marked_example(identifier, &["make lint".to_owned()]);
            let examples = parse_document("sweep.md", &document)
                .unwrap_or_else(|error| panic!("{} id={identifier}: {error}", shape.describe()));
            assert_eq!(
                examples,
                vec![TestedExample {
                    id: (*identifier).to_owned(),
                    language: "bash".to_owned(),
                    body: "make lint\n".to_owned(),
                }],
                "{} id={identifier}",
                shape.describe(),
            );
        }
    }
}

/// An empty or repeated identifier is rejected in a generated shape.
///
/// The empty marker and the duplicate identifier are already pinned by named
/// regressions; this states the same two rules against fences the sweep builds,
/// so a rejection that depended on the fence's shape rather than on the marker
/// would not pass unnoticed. The repeated case uses non-adjacent markers: a
/// second example is read between the two that collide.
#[test]
fn invalid_identifiers_are_rejected_across_bounded_fence_shapes() {
    let shapes = fence_shapes(IDENTIFIER_RUN_PAIRS);
    assert!(
        !shapes.is_empty(),
        "the sweep must generate cases; an empty table would pass it vacuously",
    );
    for shape in shapes {
        let described = shape.describe();
        let empty = shape.marked_example("", &[]);
        let empty_error =
            parse_document("sweep.md", &empty).expect_err("an empty identifier should be rejected");
        assert!(
            empty_error
                .to_string()
                .contains("tested-example identifier must not be empty"),
            "{described}: expected an empty-identifier rejection in '{empty_error}'",
        );

        let first = shape.marked_example("repeated", &[]);
        let between = shape.marked_example("between", &["make lint".to_owned()]);
        let repeated = shape.marked_example("repeated", &["make test".to_owned()]);
        let duplicate_error = parse_document("sweep.md", &format!("{first}{between}{repeated}"))
            .expect_err("a repeated identifier should be rejected");
        assert!(
            duplicate_error
                .to_string()
                .contains("duplicate tested-example identifier 'repeated'"),
            "{described}: expected a duplicate-identifier rejection in '{duplicate_error}'",
        );
    }
}

/// A marked fence indented past the supported range is rejected, not skipped.
///
/// The boundary is at four spaces, the point where `CommonMark` stops reading a
/// line as a fence. A marker at column zero followed by a fence that far
/// indented is therefore a marked example whose fence the loader cannot see,
/// and it must say so rather than quietly returning nothing. The deeper
/// indentation of a marker *itself* is the separate, documented container gap
/// recorded in the module comment, which is why the marker here stays at column
/// zero: this test pins the fail-closed direction.
#[test]
fn a_marked_fence_indented_too_deeply_is_rejected() {
    for character in ['`', '~'] {
        let fence = character.to_string().repeat(3);
        for indent in [4, 5, 6] {
            let spaces = " ".repeat(indent);
            let marked =
                format!("<!-- tested-example: deep -->\n{spaces}{fence}bash\n{spaces}make lint\n");
            let Err(error) = parse_document("sweep.md", &marked) else {
                panic!(
                    "char={character} indent={indent}: a fence indented past \
                     {MAXIMUM_SWEPT_INDENT} spaces should be reported, not skipped"
                );
            };
            assert!(
                error
                    .to_string()
                    .contains("expected an opening fence after marker"),
                "char={character} indent={indent}: expected the deep fence to be reported, not \
                 skipped, in '{error}'",
            );
        }
    }
}
