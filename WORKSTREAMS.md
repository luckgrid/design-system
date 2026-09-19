# Workstreams planning provenance

This file is a **source-side provenance pointer** for the Luckgrid Workstreams
planning process. It is process metadata for humans and automation working on
this repository; it is not a runtime, CLI, schema, configuration, or publication
contract, and it must not be loaded by product code.

The Design System is a standalone product. Product intent is authored in the
Build wiki; this file only makes the exact source → Build relationship
reconstructible.

## Current pin

```text
planning_repository:       luckgrid/lg-workstreams
planning_revision:         2cfd87e1ead31a006d7442c4b344d7d0544d24e4
planning_root:             Build/bin/design-system
live_state_path:           Build/bin/design-system/_process/STATE.md
source_repository:         luckgrid/design-system
source_baseline:           efc11b705b35b8adc40db821cf52bf64d43b5692
```

`2cfd87e1ead31a006d7442c4b344d7d0544d24e4` is the `lg-workstreams` `main` commit
that merged PR #133, "design-system: materialize DS-E01 task specs and CSS
refinement plan". It is the first revision at which the full DS-E01 task plan
exists as durable planning authority, and it is the authority this repository was
bootstrapped against.

`source_baseline` is this repository's first commit on `main`.

## Sequence note

This repository was created ahead of its place in the program sequence. Creation
belongs to `DS-E01.S1.T3`, which requires accepted `DS-E01.S1.T1`/`DS-E01.S1.T2`
evidence as input; at creation time `DS-E01.S1.T1` was implemented but still
review-pending and `DS-E01.S1.T2` had not started. The repository was created by
owner decision regardless.

That is recorded on the Build side, not papered over:

```text
session:   Build/bin/design-system/_process/sessions/
             2026-09-17-s019-public-shell-and-license-clarification.md
decision:  Build/bin/design-system/_process/decisions/
             2026-09-17-out-of-sequence-public-repository-creation.md
```

Both records land on the planning repository through `lg-workstreams#134`. They
therefore do **not** resolve at the `planning_revision` pinned above, which is an
earlier `main` commit. The pin advances to the post-merge `main` revision once
that pull request merges, at which point both paths resolve.

The override settled exactly two things — this repository's identity and its
public visibility. Cargo members, crate and package names, MSRV, release topology,
artifact topology, API classification, and the Luna `packages/ds` end-state all
remain unmade `DS-E01.S1.T3` decisions.

The project license is deliberately **not** one of them. It is deferred past that
step to the first supported release candidate, so this repository stays public and
intentionally unlicensed in the meantime.

## Authority and update rule

- `planning_revision` is an exact Git commit on the planning repository, never a
  moving branch name or PR number. For paired maintenance, pin the **post-merge**
  revision that lands on `main`, not a pre-merge review-branch HEAD.
- `planning_root` identifies the authored Build wiki that specifies Design System
  work.
- `live_state_path` identifies the Build-owned process-position authority. This
  repository does not duplicate that live state.
- The pin changes only when a source change is intentionally paired with newer
  Build authority or a session record. The updating PR must state the old and new
  Build revisions and link the paired Workstreams evidence.
- A source-only refactor or maintenance change that does not alter planning
  authority must not silently advance this pin.

## Note for outside readers

`luckgrid/lg-workstreams` is a private planning repository. The paths and commit
SHAs above will not resolve for you. They are recorded so that the project's own
decisions remain auditable and reconstructible internally, not to gate
participation here. Anything required to build, test, or consume this repository
must be present in this repository.
