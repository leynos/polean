//! The cancellation arming rule, including a marker split across reads.
//!
//! The reader searches the accumulated kept bytes, not the latest chunk, so a
//! marker delivered in two pieces still arms the request. These cases split a
//! marker at every position and require both halves to be found once joined,
//! which a chunk-only search would miss.

use super::super::supervisor::contains_marker;
use crate::scratch::check;

/// A marker is found once its two pieces have both arrived.
#[test]
fn a_marker_split_at_every_position_is_found() -> Result<(), String> {
    let marker = b"polean-hostile attempt escapee started";
    for split in 0..=marker.len() {
        let mut kept = Vec::new();
        kept.extend_from_slice(marker.get(..split).unwrap_or_default());
        if split > 0 && split < marker.len() {
            check!(
                !contains_marker(&kept, marker),
                "a prefix shorter than the marker must not arm at split {split}"
            );
        }
        kept.extend_from_slice(marker.get(split..).unwrap_or_default());
        check!(
            contains_marker(&kept, marker),
            "the joined marker must arm at split {split}"
        );
    }
    Ok(())
}

/// An empty marker never arms, and unrelated bytes never match.
#[test]
fn a_marker_needs_all_of_its_bytes() -> Result<(), String> {
    check!(
        !contains_marker(b"prefix", b""),
        "an empty marker must not arm"
    );
    check!(
        !contains_marker(b"polean-hostile attempt", b"polean-hostile start"),
        "a partial line must not match"
    );
    Ok(())
}
