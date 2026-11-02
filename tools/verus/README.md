# Pinned Verus prover

`VERSION` and `SHA256SUMS` pin the Verus release used to verify the admission
recognizer kernel. `make install-verus` downloads that release through
`rust-prover-tools`, checks it against `SHA256SUMS`, and unpacks it to
`.verus/<version>/verus`. `tools/rust-prover-tools/REF` pins the installer
itself.

Verus is not part of the Cargo workspace and not part of the E0 toolchain tuple
in `toolchain-pins.json`. `make verus` is an on-demand target: it is not part of
`make all` or the pull-request gate, and nothing in `src/` depends on it. It
runs `verus-proof` and then `verus-controls`, so the proof and the evidence for
its sensitivity to defects are produced by one command.

## What the proof covers

`verus/admission_recognizer.rs` is the proof root. It pulls in
`tests/admission/recognizer/kernel/mod.rs` with `#[path]`, so the same source
is compiled by Cargo as test code and verified by Verus. The proof establishes
that an `Ok` from the kernel decomposes the source into the rendered tokens
with permitted trivia between them, and that the accepted source honours the
flag list's line-feed demands.

Two exclusions are proved only under conditions on the token list, because the
generic kernel's contract says nothing about token bytes beyond their being
non-empty:

- T1 (`lemma_no_hidden_bytes`): **if** every token byte is printable ASCII,
  every byte of an accepted source is a tab, printable ASCII, a line feed, or a
  carriage return that starts a line ending.
- T2 (`lemma_no_metadata_outside_tokens`): **if** no token byte is `#`, no `#`
  in an accepted source begins a comment that spells `METADATA` on its own line.

Neither exclusion holds for an arbitrary token list. A token containing a NUL,
a non-ASCII byte, or a `#` would be accepted by the kernel and would defeat the
corresponding conclusion. The committed renderer supplies only printable tokens
that contain no `#`, so both conditions hold for the corpus.

`lemma_baseline_accepts` is the concrete accepting instance: it proves
`decomposes_seq` for the canonical baseline's source, token bytes, offsets, and
the line feed before the second body literal. The tokens are stated as byte
sequences because Verus gives a byte-literal slice opaque contents, so a
decomposition over slice tokens could not be checked concretely.
`lemma_decomposes_seq_to_slices` closes that gap: it turns the sequence-token
relation into the kernel's own `decomposes_at` for any slice whose view is the
inspected sequence. The executable test
`the_concrete_baseline_is_recognised_at_the_proved_offsets` runs the same
source through the real kernel and checks the same tokens, flags, and offsets.
Neither establishes that Rego assigns the same meaning to those tokens; that
claim stays with the OPA and Regorus agreement work.

## Negative controls

A passing run is evidence about the obligations the proof states, not about its
sensitivity to a defect. `scripts/check_verus_controls.py`, run by
`make verus-controls` and by `make verus`, edits a scratch copy of the proof
sources, requires the unmodified copy to verify with zero errors first, and
then requires the edited copy to fail at the named obligation:

| Kind          | Change                                       | Required failure                    |
| :------------ | :------------------------------------------- | :---------------------------------- |
| body          | drop the `METADATA` scan from `skip_comment` | `skip_comment`'s postcondition      |
| body          | drop the non-empty gap rule from `recognize` | `recognize`'s loop invariant        |
| body          | drop the line-feed rule from `recognize`     | `recognize`'s line-feed assertion   |
| specification | let `body_byte` admit NUL                    | T1's `lemma_comment_body_permitted` |
| specification | drop `comment_ok`'s metadata exclusion       | T2's `lemma_comment_no_metadata`    |

The three body controls mutate the executable kernel and leave the
specification unchanged; the two specification controls mutate a definition and
leave the kernel unchanged. Every edit's anchor must occur exactly once, so a
control fails closed when the source it describes has drifted rather than
silently replacing nothing. The script writes only to a temporary directory and
runs no Cargo or repository gate.

These controls do not replace the proof and do not widen its foundations: the
result still rests on Verus, its encoding, `vstd`, Z3, and the compilation
machinery, and the controls exercise only the kernel's soundness contract.

## Bumping the pin

1. Choose a Verus release and read its `x86-linux.zip` digest from the release
   assets.
2. Update `VERSION` and the matching row of `SHA256SUMS`. Keep the archive name
   in the checksum row: the installer looks the digest up by that name.
3. If the release pins a new `vstd`, update the `vstd` dev-dependency in
   `Cargo.toml` to the matching version and rebuild the lock file.
4. Run `make install-verus` and then `make verus`, and record the result.
