# Design System

This directory is the local slot reserved for Luckgrid Design System work under
`Build/src/workspaces/`. It is currently **empty of source by design**.

It exists so the eventual source checkout has an unambiguous, documented home,
and so anyone arriving here can reach the real authority without guessing.

## What this is not

- **Not** the public Design System repository. That repository does not exist yet.
- **Not** a Cargo workspace, package, or build target. No `Cargo.toml`,
  `rust-toolchain.toml`, license, MSRV, or member list is declared here.
- **Not** an authority for any Design System contract. No file in this directory
  may be cited as a Design System decision, API, token, or policy.

Planning and specification authority lives in `lg-workstreams` under
`Build/bin/design-system/`. This directory is untracked local staging.

## Authority map

| Concern | Authority |
|---|---|
| Durable product intent | `Plan/bin/systems/frontend-systems/design-system/` |
| Build implementation wiki | `Build/bin/design-system/README.md` |
| Live process position | `Build/bin/design-system/_process/STATE.md` |
| Public source/consumer contract | `Build/bin/design-system/foundation/source-and-consumer-contract.md` |
| Rust workspace architecture | `Build/bin/design-system/architecture/rust-workspace-architecture.md` |
| Bootstrap contract for the future repository | `Build/bin/design-system/architecture/public-repository-bootstrap.md` |
| First epic | `Build/bin/design-system/program/ds-e01/README.md` |
| First story | `Build/bin/design-system/program/ds-e01/s1-inventory-compatibility-and-public-source-bootstrap/README.md` |
| Gating task | `Build/bin/design-system/program/ds-e01/s1-inventory-compatibility-and-public-source-bootstrap/t3-public-repository-decision-and-bootstrap.md` |

Paths are relative to the `lg-workstreams` repository root.

## Branch context

```text
durable working branch   lg/design-system-e01
PR #133 planning branch  lg/design-system-e01-task-specs
```

Both names refer to branches of `luckgrid/lg-workstreams`. They are **not** source
branches, because no Design System source repository exists. `lg/design-system-e01`
is the durable name to reuse for E01 Workstreams-side work.

## Gate

Restated from `Build/bin/design-system/_process/STATE.md` gate rules 2 and 3,
without reinterpretation:

1. Do not create or migrate Design System source before `DS-E01.S1.T1` and
   `DS-E01.S1.T2` provide the inventory and compatibility evidence that
   `DS-E01.S1.T3` requires.
2. `DS-E01.S1.T3` is not documentation-only. After T1/T2 evidence it selects the
   exact repository identity, license, Cargo members, MSRV, source boundary,
   release premise, and Luna transition, and creates the real public repository
   with bootstrap evidence.

The preferred repository-name candidate is `luckgrid/design-system`. It is a
candidate, not accepted authority, until T3 verifies availability and records the
decision.

## When source arrives

At `DS-E01.S1.T3`, the public repository is cloned **into** this directory as its
own git repository, following the pattern of `Build/src/workspaces/uwiki` and
`Build/src/workspaces/wfos`. At that point:

- the source repository carries its own `WORKSTREAMS.md` provenance pin, which
  becomes the authoritative source → Build pointer;
- this staging `README.md` and `WORKSTREAMS.md` are superseded or reduced to a
  pointer;
- `design-system.descriptor.toml` is replaced by a real workspace descriptor with
  the toolchain and entrypoints that T3 selects.

## Where evidence goes

T1/T2 inventory, compatibility, and behavioral evidence is recorded in
`lg-workstreams` under `Build/bin/design-system/`. It is never recorded here.
This directory is ignored by the parent repository and nothing in it is durable.
