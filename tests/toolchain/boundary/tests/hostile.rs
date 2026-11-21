//! The hostile entry contract: its exact name and its argument token.
//!
//! The entry point is inert without a token, so these tests are the only thing
//! that exercises the parser under `make test`. No hostile behaviour runs.

use rstest::rstest;

use super::super::hostile::{self, Behaviour, Request, TOKEN_PREFIX};
use crate::scratch::{check, check_eq};

/// The entry point's exact libtest path is pinned.
#[test]
fn the_hostile_entry_name_is_pinned() -> Result<(), String> {
    check_eq!(
        hostile::entry_name(),
        "boundary::hostile::entry".to_owned(),
        "a module move must be noticed here rather than in a silent probe"
    );
    Ok(())
}

/// An argument list without a token is inert.
#[test]
fn an_absent_token_is_inert() -> Result<(), String> {
    let args = vec![
        "toolchain".to_owned(),
        "--exact".to_owned(),
        "--test-threads=1".to_owned(),
    ];
    check_eq!(
        hostile::request(&args),
        Ok(None),
        "no token must mean no behaviour"
    );
    Ok(())
}

/// One well-formed token parses.
#[test]
fn a_token_selects_one_behaviour() -> Result<(), String> {
    let args = vec![format!("{TOKEN_PREFIX}offline:run-7")];
    check_eq!(
        hostile::request(&args),
        Ok(Some(Request {
            behaviour: Behaviour::Offline,
            nonce: "run-7".to_owned(),
        })),
        "the token must select the offline behaviour with its nonce"
    );
    Ok(())
}

/// Two tokens are a probe defect rather than a silent choice.
#[test]
fn a_duplicate_token_is_rejected() -> Result<(), String> {
    let args = vec![
        format!("{TOKEN_PREFIX}offline:run-7"),
        format!("{TOKEN_PREFIX}memory:run-8"),
    ];
    match hostile::request(&args) {
        Err(reason) => check!(
            reason.contains("more than one"),
            "the reason must say the token is duplicated, got {reason:?}"
        ),
        other => return Err(format!("two tokens must be refused, got {other:?}")),
    }
    Ok(())
}

/// Malformed tokens are refused.
#[rstest]
#[case::no_nonce("offline")]
#[case::unknown_behaviour("bogus:run-1")]
#[case::empty_nonce("offline:")]
#[case::bad_nonce("offline:bad/nonce")]
#[case::long_nonce(
    "offline:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
)]
fn a_malformed_token_is_rejected(#[case] token: &str) -> Result<(), String> {
    let args = vec![format!("{TOKEN_PREFIX}{token}")];
    match hostile::request(&args) {
        Err(_reason) => Ok(()),
        other => Err(format!(
            "the malformed token {token:?} must be refused, got {other:?}"
        )),
    }
}

/// Every behaviour name appears in the diagnostic list.
#[test]
fn the_behaviour_names_are_complete() -> Result<(), String> {
    let names = hostile::names();
    for behaviour in Behaviour::ALL {
        check!(
            names.contains(behaviour.as_str()),
            "the name list {names:?} must contain {}",
            behaviour.as_str()
        );
        check_eq!(
            Behaviour::parse(behaviour.as_str()),
            Some(behaviour),
            "every behaviour must parse from its own name"
        );
    }
    Ok(())
}
