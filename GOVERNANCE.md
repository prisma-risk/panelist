<!-- SPDX-License-Identifier: Apache-2.0 -->
# Governance

Panelist is an Apache-2.0 project maintained by Prisma Risk.

During incubation, Sebastian Thiebaud (`@sebastianthiebaud`) is the lead
maintainer and final decision maker for design, merge, release, security, and
community matters. Additional maintainers may be added after sustained,
substantive contribution and demonstrated stewardship of the public API and
Grafana compatibility contract.

Day-to-day changes use lazy consensus in issues and pull requests. Breaking API
changes, new serialization formats, and substantial DSL additions should be
discussed in an issue before implementation. When consensus cannot be reached,
the lead maintainer records the decision and rationale in the relevant public
thread.

Releases will be automated from reviewed release pull requests after the crate
is approved for crates.io publication. Until that public-release gate is met,
the workspace remains `publish = false` and no release credentials are needed.
