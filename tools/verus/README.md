# Pinned Verus prover

`VERSION` and `SHA256SUMS` pin the Verus release used to verify the admission
recognizer kernel. `make install-verus` downloads that release through
`rust-prover-tools`, checks it against `SHA256SUMS`, and unpacks it to
`.verus/<version>/verus`. `tools/rust-prover-tools/REF` pins the installer
itself.

Verus is not part of the Cargo workspace and not part of the E0 toolchain tuple
in `toolchain-pins.json`. `make verus` is an on-demand target: it is not part of
`make all` or the pull-request gate, and nothing in `src/` depends on it.

## What the proof covers

`verus/admission_recognizer.rs` is the proof root. It pulls in
`tests/admission/recognizer/kernel.rs` with `#[path]`, so the same source is
compiled by Cargo as test code and verified by Verus. The proof establishes
that an `Ok` from the kernel decomposes the source into the rendered tokens
with permitted trivia between them, and that the accepted source holds no
`METADATA` annotation and no byte outside the permitted set. It does not
establish that Rego assigns the same meaning to those tokens; that claim stays
with the OPA and Regorus agreement work.

## Bumping the pin

1. Choose a Verus release and read its `x86-linux.zip` digest from the release
   assets.
2. Update `VERSION` and the matching row of `SHA256SUMS`. Keep the archive name
   in the checksum row: the installer looks the digest up by that name.
3. If the release pins a new `vstd`, update the `vstd` dev-dependency in
   `Cargo.toml` to the matching version and rebuild the lock file.
4. Run `make install-verus` and then `make verus`, and record the result.
