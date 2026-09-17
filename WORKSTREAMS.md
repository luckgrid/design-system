# Workstreams planning provenance

This file is a **local staging provenance pointer** for the Luckgrid Design
System. It records which planning revision this workspace slot was set up
against. It is process metadata for humans and automation; it is not a runtime,
CLI, schema, configuration, or publication contract, and it carries no product
meaning.

The Design System source repository does not exist yet. Until `DS-E01.S1.T3`
creates it, this file has no source side to pin.

## Current pin

```text
planning_repository:       luckgrid/lg-workstreams
planning_revision:         2cfd87e1ead31a006d7442c4b344d7d0544d24e4
planning_root:             Build/bin/design-system
live_state_path:           Build/bin/design-system/_process/STATE.md
working_branch:            lg/design-system-e01
source_repository:         (not created — DS-E01.S1.T3)
source_baseline:           (none)
```

The pinned planning tree can be inspected directly at:

- <https://github.com/luckgrid/lg-workstreams/tree/2cfd87e1ead31a006d7442c4b344d7d0544d24e4/Build/bin/design-system>

`2cfd87e1ead31a006d7442c4b344d7d0544d24e4` is the `main` commit that merged
PR #133, "design-system: materialize DS-E01 task specs and CSS refinement plan".
It is the first revision at which the full E01 task plan exists as durable
authority.

Note: `Build/bin/design-system/_process/STATE.md` at version 1.8 still describes
PR #133 as awaiting final acceptance review, while `main` already contains its
merge. The pin above records the merged revision. Reconciling STATE is Build-wiki
work and is out of scope for this local workspace setup.

## Authority and update rule

- `planning_revision` is an exact Git commit on the planning repository, never a
  moving branch name or PR number.
- `planning_root` identifies the authored Build wiki that specifies Design System
  work.
- `live_state_path` identifies the Build-owned process-position authority. This
  workspace does not duplicate that live state.
- `working_branch` is the durable `lg-workstreams` branch name for E01 work. It
  is not a source branch.
- `source_repository` and `source_baseline` stay empty until `DS-E01.S1.T3`
  selects the repository identity and creates the public repository. Filling them
  in before that would assert a decision T3 owns.
- When the source repository is created and cloned into this directory, that
  repository's own `WORKSTREAMS.md` becomes the authoritative source → Build pin,
  following the `Build/src/workspaces/uwiki/WORKSTREAMS.md` convention, and this
  staging file is retired.
