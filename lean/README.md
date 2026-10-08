# Lean package boundary

The technical design assigns formal syntax, semantics, abstraction lemmas,
registered claims, finite-checker soundness, witness soundness, and theorem
audits to a pinned Lean package.

No Lean toolchain or package is committed yet. Experiment E0 must identify and
record a compatible Lean version, package graph, allowed axiom policy, and
clean checking command before executable modules appear here.

Do not place generated Rust, arbitrary submitted Lean, Lake configuration, or
compiled proof artefacts in this directory. The proposed verifier will generate
fixed data modules in isolated job directories and check them against trusted
package code.
