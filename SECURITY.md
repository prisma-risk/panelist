<!-- SPDX-License-Identifier: Apache-2.0 -->
# Security policy

Only the latest released minor version of Panelist receives security fixes.
During the pre-release incubation period, fixes land on `main`.

Do not open a public issue for a suspected vulnerability. Use a private draft
security advisory at:

<https://github.com/prisma-risk/panelist/security/advisories/new>

Include the affected version or commit, impact, reproduction, and any known
workaround. We aim to acknowledge reports within 24 hours, provide initial
triage within 72 hours, and coordinate disclosure within 30 days or when a fix
ships, whichever comes first.

Panelist is a synchronous JSON-construction library. The core security scope is
therefore unsafe serialization, injection of unexpected Grafana properties,
dependency/supply-chain compromise, and denial of service from untrusted
authoring input. Grafana server configuration, datasource authorization, and
the behavior of user-supplied raw escape-hatch values are outside the crate's
security boundary.
