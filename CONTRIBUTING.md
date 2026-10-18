<!-- SPDX-License-Identifier: Apache-2.0 -->
# Contributing to Panelist

Thank you for helping make Grafana dashboards pleasant to author in Rust. By
participating, you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Before opening a change

Open an issue before implementing a breaking API change, a new serialization
format, or a large DSL extension. Small fixes, documentation improvements, and
well-scoped additions can go directly to a pull request.

Keep the central boundary intact:

```text
macro DSL / typed builders
            ↓
semantic authoring model
            ↓
validation + deterministic normalization/layout
            ↓
Grafana serialization model
            ↓
JSON
```

Do not place Grafana wire names in macro parsing code or expose
`serde_json::Value` as the normal authoring API. New escape hatches should be
explicit and preserve deterministic map ordering.

## Setup

The pinned toolchain is declared in [`rust-toolchain.toml`](rust-toolchain.toml).
Install [`cargo-deny`](https://github.com/EmbarkStudios/cargo-deny) to run the
complete local suite.

```sh
make check       # format check, Clippy, build, test, and rustdoc
make ci          # check + headers + package + cargo-deny
```

Useful focused commands:

```sh
cargo test -p panelist --test model
cargo test -p panelist --test macros
cargo test -p panelist --test ui   # TRYBUILD=overwrite to refresh macro diagnostics
cargo test -p panelist --test golden
cargo run -p panelist --example full_dashboard
make verify-grafana  # needs Docker: round-trips the goldens through a real Grafana
make demo            # needs Docker: serves every example on a live Grafana
```

Set `UPDATE_GOLDEN=1` when an intentional wire-format change requires updating the committed golden dashboards, then review the JSON diff before committing. Each `GrafanaTarget` has its own golden directory (`tests/golden` for the default target, `tests/golden/v11_5` for 11.5), and `UPDATE_GOLDEN=1` rewrites all of them.

A change that alters what an example renders should ship a refreshed
screenshot: `make render-examples` regenerates every image in
[`examples/images`](examples/images) from the examples themselves, and
`make demo-down` stops the stack afterwards.

## Tests and documentation

- Every behavior change needs a focused regression test.
- Serialization changes need a golden JSON assertion when they affect the
  stable output contract.
- Macro syntax should be compared with its equivalent builder model.
- Public types and methods need rustdoc. Important types should include a
  compiling example when that adds clarity.
- Examples must compile under `cargo clippy --all-targets --all-features`.

## Git and pull requests

- Use Conventional Commit prefixes such as `feat:`, `fix:`, `docs:`,
  `refactor:`, `test:`, and `chore:`.
- Rebase and squash onto current `main` before opening a pull request.
- After publication, add review-fix commits instead of rebasing so reviewers can
  see each delta; the repository squashes at merge.
- Explain wire-format compatibility and migration impact in changes to the
  Grafana serializer.

## Releases

The `Release` workflow accepts an exact reviewed source commit, a stable
version and public release notes from the configured release dispatcher.
The repository variable `RELEASE_DISPATCH_ACTOR` identifies that dispatcher;
maintainers manage its value outside the source tree. Ordinary pushes do not
publish a release.

The preparation job validates the source and packages the selected version
without publishing credentials. The publish job consumes that same run's
verified archive, reconciles its checksum against crates.io, and creates a
signed annotated `vX.Y.Z` tag and matching GitHub Release. Tag signing uses the
workflow's existing GitHub Actions OIDC identity. The `crates-io` environment
holds the crate publishing token and public release App credentials.

Release commits contain only the versioned workspace manifest and lockfile,
with their reviewed source commit recorded as provenance. The workflow never
pushes a version commit to `main`. Changelogs remain historical release records;
new release notes are reviewed in the release request.

Run `python3 -m unittest discover -s scripts -p 'test_release*.py'` after changing
release tooling, and `make release-dry-run` before changing crate metadata.
An existing version or tag is never replaced. After a partial failure, inspect
crates.io, the signed tag and GitHub Release before rerunning the same request.

Public release configuration must contain only public repository information,
public endpoints and generic input names. Keep private infrastructure,
repository references, dispatcher identities and credentials outside this tree.

## Developer Certificate of Origin

Panelist uses the [Developer Certificate of Origin 1.1](https://developercertificate.org/).
Sign off each commit with `git commit -s`. The sign-off certifies that you have
the right to submit the contribution under the project's Apache-2.0 license and
that the contribution record may be retained and redistributed.

## Comments

Comments should explain durable behavior and constraints. Keep issue numbers,
task markers, internal project names, and review labels in issues and pull
requests rather than source comments. External specifications and upstream
issues are welcome when they explain a lasting compatibility decision.
