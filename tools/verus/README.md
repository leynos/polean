# Pinned Verus prover

`VERSION` and `SHA256SUMS` pin the Verus release used to verify the admission
recognizer kernel and the audit settlement decision. `make install-verus`
downloads the release through `rust-prover-tools`, checks it against
`SHA256SUMS`, unpacks it to `.verus/<version>/verus`, and then runs
`prover-tools z3 install`, which installs the Z3 solver that revision pins to
`.verus/z3`. `make verus-proof` runs each proof entry point through
`prover-tools verus run`. `tools/rust-prover-tools/REF` pins the tool itself.

Verus is not part of the Cargo workspace and not part of the E0 toolchain tuple
in `toolchain-pins.json`. `make verus` is an on-demand target: it is not part of
`make all` or the pull-request gate, and nothing in `src/` depends on it. It
runs `verus-proof` and then `verus-controls`, so the proof and the evidence for
its sensitivity to defects are produced by one command.

## What the proofs cover

There are two proof roots, and `make verus` runs both before the controls.

`verus/admission_recognizer.rs` is the admission root. It pulls in
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

`verus/audit_settlement.rs` is the settlement root. It pulls in
`tests/toolchain/audit/settlement/decision/` with `#[path]`, so the same source
is compiled by Cargo and verified by Verus. It proves the settlement
obligations S0 to S7 over an abstract report: acceptance requires the pinned
toolchain, the requested module and roots, an exact statement, and only
approved foundations; a protocol breach outranks any finding; and a refusal is
distinguished from a rejection. Verus reports 61 verified obligations, with the
specification, lemmas, and witnesses in `audit_settlement_*.rs`.

The settlement proof establishes what acceptance means for a report it is
given. It does not establish that the Lean worker told the truth, that the wire
decoder admitted the report faithfully, or that a report corresponds to any
source policy. Those are separate claims, and only the first is inside the
proof.

## Negative controls

A passing run is evidence about the obligations the proof states, not about its
sensitivity to a defect. `scripts/check_verus_controls.py`, run by
`make verus-controls` and by `make verus`, edits a scratch copy of the proof
sources, requires each unmodified copy to verify with zero errors first, and
then requires the edited copy to fail at the named obligation.

The five admission controls are:

| Kind          | Change                                       | Required failure                    |
| :------------ | :------------------------------------------- | :---------------------------------- |
| body          | drop the `METADATA` scan from `skip_comment` | `skip_comment`'s postcondition      |
| body          | drop the non-empty gap rule from `recognize` | `recognize`'s loop invariant        |
| body          | drop the line-feed rule from `recognize`     | `recognize`'s line-feed assertion   |
| specification | let `body_byte` admit NUL                    | T1's `lemma_comment_body_permitted` |
| specification | drop `comment_ok`'s metadata exclusion       | T2's `lemma_comment_no_metadata`    |

_Table 1: The five admission negative controls and the failure each must cause._

The ten settlement controls are:

| Kind          | Change                                 | Required failure              |
| :------------ | :------------------------------------- | :---------------------------- |
| body          | let acceptance swallow a finding       | the acceptance postcondition  |
| body          | drop the accepted-replay check         | the replay postcondition      |
| body          | drop the statement comparison          | the finding assertion         |
| body          | drop the coverage name comparison      | the replay loop invariant     |
| body          | stop the root loop early               | the coverage postcondition    |
| body          | stop the axiom loop early              | the axiom postcondition       |
| body          | ignore the approved foundations        | the scan postcondition        |
| body          | report a breach as a rejection         | the settlement postcondition  |
| specification | admit `sorryAx` as a pinned foundation | the pinned-foundation lemma   |
| specification | weaken an admissible request           | the request witness assertion |

_Table 2: The ten settlement negative controls and the failure each must cause._

The body controls mutate executable decision code and leave the specification
unchanged; the specification controls mutate a definition and leave the code
unchanged. Every edit's anchor must occur exactly once, so a control fails
closed when the source it describes has drifted rather than silently replacing
nothing. The script writes only to a temporary directory and runs no Cargo or
repository gate.

These controls do not replace either proof and do not widen its foundations:
the results still rest on Verus, its encoding, `vstd`, Z3, and the compilation
machinery, and the controls exercise only the stated contracts.

## Bumping the pin

1. Choose a Verus release and read its `x86-linux.zip` digest from the release
   assets.
2. Update `VERSION` and the matching row of `SHA256SUMS`. Keep the archive name
   in the checksum row: the installer looks the digest up by that name.
3. Read the Z3 version the release's `source/tools/get-z3.sh` downloads and
   compare it with the one `prover-tools z3 install` installs. If they differ,
   move `tools/rust-prover-tools/REF` to a revision that pins the new one.
4. If the release pins a new `vstd`, update the `vstd` dev-dependency in
   `Cargo.toml` to the matching version and rebuild the lock file.
5. Run `make install-verus` and then `make verus`, and record the result.
