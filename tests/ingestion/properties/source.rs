//! Ingestion preserves the bytes whatever the reader's chunking.
//!
//! INV-SRC-EXACT says an accepted source is the input bytes; INV-SRC-CHUNKING
//! says the result does not depend on how the reader splits the stream. Both
//! quantify over inputs a table cannot enumerate, so they are properties. The
//! oracle is [`PolicySource::from_bytes`], which takes the same bytes without a
//! reader between them.
//!
//! The reader the chunking property uses reports `Interrupted` before every
//! successful read, so no run can pass without retrying.

use polean::{
    profile::SOURCE_BYTES_MAX,
    source::{PolicySource, SourceError},
};
use proptest::prelude::*;

use crate::support::ChunkedInterrupting;

/// A short valid UTF-8 document.
fn text() -> impl Strategy<Value = String> { "[ -~\n]{0,64}" }

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    /// A source read in arbitrary chunks, with an interruption before every
    /// read, is the source read whole.
    #[test]
    fn chunking_does_not_change_the_source(
        text in text(),
        chunks in proptest::collection::vec(1_usize..5, 1..8),
    ) {
        let bytes = text.into_bytes();
        let expected = PolicySource::from_bytes(bytes.clone())
            .expect("a short UTF-8 source fits the profile");

        for chunk in chunks {
            let reader = ChunkedInterrupting::new(bytes.clone(), chunk);
            let source = PolicySource::read_from(reader).expect("the source fits");
            prop_assert_eq!(source.as_bytes(), expected.as_bytes());
        }
    }

    /// An accepted source is its input bytes, and both entry points refuse the
    /// same bytes with the same fault.
    #[test]
    fn from_bytes_agrees_with_read_from(bytes in proptest::collection::vec(any::<u8>(), 0..1024)) {
        let direct = PolicySource::from_bytes(bytes.clone());
        let streamed = PolicySource::read_from(bytes.as_slice());

        match (&direct, &streamed) {
            (Ok(bytes_read), Ok(text_read)) => {
                prop_assert_eq!(bytes_read.as_bytes(), bytes.as_slice());
                prop_assert_eq!(bytes_read.as_bytes(), text_read.as_bytes());
            }
            (Err(byte_fault), Err(text_fault)) => {
                prop_assert_eq!(fault(byte_fault), fault(text_fault));
            }
            _ => {
                prop_assert!(false, "the entry points disagree: {direct:?} and {streamed:?}");
            }
        }
    }
}

/// A comparable description of an ingestion fault.
///
/// `SourceError` holds an `io::Error` in one variant and so cannot derive
/// equality; the bytes-only entry points never produce that variant, and the
/// description makes the disagreement readable when one does.
fn fault(error: &SourceError) -> String {
    match error {
        SourceError::TooLarge { limit } => format!("too large: {limit}"),
        SourceError::InvalidUtf8 { valid_up_to } => format!("invalid UTF-8 at {valid_up_to}"),
        SourceError::Nul { offset } => format!("NUL at {offset}"),
        SourceError::Read(_) => "read failure".to_owned(),
    }
}

/// The cap is exact for every valid filler ASCII byte, not only for the sizes
/// the table names.
#[test]
fn the_cap_is_exact_for_any_filler() {
    // NUL is excluded because ingestion refuses it on its own account; the
    // cap and the NUL rule are separate, and this test is about the cap.
    for filler in [b'a', b'\n', 0x7f, b' '] {
        let mut fitting = vec![filler; SOURCE_BYTES_MAX];
        if let Some(last) = fitting.last_mut() {
            *last = b'\n';
        }
        assert!(
            PolicySource::from_bytes(fitting.clone()).is_ok(),
            "a source at the cap is accepted"
        );

        fitting.push(b'a');
        assert!(
            matches!(
                PolicySource::from_bytes(fitting),
                Err(SourceError::TooLarge { .. })
            ),
            "one byte over the cap is refused"
        );
    }
}
