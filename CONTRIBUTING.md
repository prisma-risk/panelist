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
cargo test -p panelist --test golden
cargo run -p panelist --example full_dashboard
make verify-grafana  # needs Docker: round-trips the goldens through a real Grafana
make demo            # needs Docker: serves every example on a live Grafana
```

Set `UPDATE_GOLDEN=1` when an intentional wire-format change requires updating
the committed golden dashboard, then review the JSON diff before committing.

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

Releases run through [release-plz](https://release-plz.dev/):

Do not edit `CHANGELOG.md` files by hand. The required changelog-ownership workflow permits only release PRs authored by `prismarisk-public-release[bot]` to change them.

1. Land release-worthy changes on `main` with a `feat:`, `fix:`, `perf:`, or `refactor:` Conventional Commit prefix.
2. Dispatch the `release-plz` workflow to open or update the release PR. It owns the workspace version and `crates/panelist/CHANGELOG.md` changes.
3. Review and merge the release PR. Its merge automatically creates a signed `vX.Y.Z` tag, publishes Panelist to crates.io, and creates the matching GitHub Release.
4. Verify the release workflow, crates.io publication, signed tag, and GitHub Release.

Release PR creation is manual. The publish phase runs automatically only when a push to `main` changes a release-plz-owned `CHANGELOG.md`; ordinary changes to `main` do not run it.

Before changing publish metadata, run `make release-dry-run` to validate the crate archive exactly as crates.io will receive it. The publish job is the only job that receives `CARGO_REGISTRY_TOKEN`.

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
