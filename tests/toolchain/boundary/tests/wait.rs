//! The descriptor-sampling rule: an inherited descriptor survives every sample.
//!
//! The supervisor samples the job's descriptor table until the job stops and
//! keeps only the descriptors present in every sample. These cases pin that
//! rule without starting a job: an inherited descriptor persists for the job's
//! life and must be reported, while a descriptor the job opened for its own
//! work appears in only some samples and must not be mistaken for inherited.

use super::super::supervisor::retain_seen;
use crate::scratch::check_eq;

/// The first sample is the whole answer when no earlier evidence exists.
#[test]
fn the_first_sample_is_kept_whole() -> Result<(), String> {
    check_eq!(
        retain_seen(None, &[0, 1, 2]),
        vec![0, 1, 2],
        "the first sample must seed the evidence",
    );
    Ok(())
}

/// A descriptor present in every sample is inherited and stays reported.
#[test]
fn a_descriptor_present_in_every_sample_survives() -> Result<(), String> {
    let first = retain_seen(None, &[0, 1, 2, 3]);
    check_eq!(
        retain_seen(Some(first), &[0, 1, 2, 3]),
        vec![0, 1, 2, 3],
        "an inherited descriptor must survive every sample",
    );
    Ok(())
}

/// A descriptor the job opened after starting is dropped by the intersection.
#[test]
fn a_transient_descriptor_is_dropped() -> Result<(), String> {
    let first = retain_seen(None, &[0, 1, 2, 3]);
    check_eq!(
        retain_seen(Some(first), &[0, 1, 2]),
        vec![0, 1, 2],
        "a descriptor absent from a later sample must not be reported",
    );
    Ok(())
}

/// The intersection cannot gain a descriptor a later sample alone observed.
#[test]
fn a_later_sample_cannot_add_a_descriptor() -> Result<(), String> {
    let first = retain_seen(None, &[0, 1, 2]);
    check_eq!(
        retain_seen(Some(first), &[0, 1, 2, 3]),
        vec![0, 1, 2],
        "a sample taken later must not add a descriptor",
    );
    Ok(())
}

/// A job observed with no open descriptors stays empty.
#[test]
fn an_empty_sample_stays_empty() -> Result<(), String> {
    let first = retain_seen(None, &[]);
    check_eq!(
        retain_seen(Some(first), &[]),
        Vec::<i32>::new(),
        "an empty sample must stay empty",
    );
    Ok(())
}
