# OrthoConfig v0.10.0 documentation provenance

The two imported documents below are byte-for-byte copies from the immutable
commit peeled from the `v0.10.0` tag. The fetched tag resolution matched the
surveyed object and commit; no alternate tag resolution was observed.

| Field             | Value                                                             |
| ----------------- | ----------------------------------------------------------------- |
| Source repository | `https://github.com/leynos/ortho-config`                          |
| Tag               | `v0.10.0`                                                         |
| Tag object        | `996e5a1c9f22ace4f55bf5dcc196d2c0524262bf`                        |
| Peeled commit     | `38812c563e3953a490e3efc202609f37e21a0517`                        |
| Import method     | `git show 38812c563e3953a490e3efc202609f37e21a0517:<source-path>` |

| Upstream path                     | Imported path                                  | SHA-256                                                            |
| --------------------------------- | ---------------------------------------------- | ------------------------------------------------------------------ |
| `docs/users-guide.md`             | `docs/ortho-config-users-guide.md`             | `84d1c97b77024224dc9220a12d4b749205d9bb2db54849185da21c969dda5575` |
| `docs/v0-10-0-migration-guide.md` | `docs/ortho-config-v0-10-0-migration-guide.md` | `b5331aa6cfcc07b3dcd188c620db98d0a1a1f13b0c8ef96d88913d701bca25bf` |

## Upstream documentation notes

The imported files remain unchanged, including their relative links. These
links point to files in the source repository and do not resolve in this
checkout: the
[Ortho L10n README](https://github.com/leynos/ortho-config/blob/38812c563e3953a490e3efc202609f37e21a0517/ortho_l10n/README.md)
and the
[Hello World example](https://github.com/leynos/ortho-config/tree/38812c563e3953a490e3efc202609f37e21a0517/examples/hello_world/).

Other relative references assume the upstream `docs/` layout. The user's guide
links to `v-0-10-0-migration-guide.md`, while the migration guide links to
`users-guide.md` and its section anchors. In this checkout, the imported files
use the required `ortho-config-` names, and `docs/users-guide.md` belongs to
Polean. The source text is retained byte-for-byte; no compatibility aliases or
second published copy are part of this import. Follow these cross-references in
the pinned upstream
[user's guide](https://github.com/leynos/ortho-config/blob/38812c563e3953a490e3efc202609f37e21a0517/docs/users-guide.md)
and
[migration guide](https://github.com/leynos/ortho-config/blob/38812c563e3953a490e3efc202609f37e21a0517/docs/v0-10-0-migration-guide.md).
The upstream list formatting is likewise retained as part of the byte-exact
import.

The `tested-example` markers in the user's guide are consumed by the upstream
documentation-example tests. For example, `guide-first-cli` is registered in
the upstream
[documentation example tests](https://github.com/leynos/ortho-config/blob/38812c563e3953a490e3efc202609f37e21a0517/ortho_config/tests/documentation_examples_tests.rs)
and
[Rust-flow checks](https://github.com/leynos/ortho-config/blob/38812c563e3953a490e3efc202609f37e21a0517/ortho_config/tests/documentation_examples/rust_flows.rs).
Polean does not include or run that harness, so these markers record upstream
coverage and are not claims that Polean tests execute the imported examples.
