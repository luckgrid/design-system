# Bootstrap architecture

Status: experimental, internal bootstrap.

The repository is a Rust-first contributor workspace whose frontend product
remains CSS-first and semantic-HTML-first. Rust validates source and release
boundaries; it is not a runtime requirement for CSS consumers.

## Current bootstrap surfaces

The compatibility inventory is `bootstrap-surfaces.tsv`. At T3 every listed
surface is `internal`. The bootstrap intentionally exposes no
`public-stable` surface and no supported portable CSS entrypoint.

Every tracked path in this repository is classified there. Classification states
a compatibility position; it does not claim a path is a build input.

The first frontend source seam is `packages/styles/`. DS-E01.S2 will
define the first supported CSS contract; this directory's existence does not do
so early.

## Contributor check

Run:

```sh
cargo run --locked -p design-system-check -- check \
  bootstrap-surfaces.tsv fixtures/plain-html
```

The checker validates:

- every inventory row carries an explicit `internal`/`public-preview` class, and
  rejects `public-stable` while no compatibility contract owns it;
- every listed path is relative, inside a permitted bootstrap root, and exists;
- listed files contain no private planning/cross-repository path markers.
  Directory surfaces are walked, so this reaches every file beneath them;
- every Cargo workspace member declares `[lints] workspace = true`, so the root
  lint policy cannot silently stop applying;
- the plain fixture stays local, framework-free, Tailwind-free, network-free, and
  free of any browser-runtime Rust requirement.

A surface is exempted from the privacy content scan only through an explicit
`scan-exempt:<reason>` field in the inventory. That exists for the files which
must legitimately contain such literals — the checker's own marker constants,
the negative boundary fixtures, and the deliberate planning-provenance pointers
(`WORKSTREAMS.md`, `design-system.descriptor.toml`). The command reports how
many files were scanned and how many were exempt, so its output never implies
coverage it did not perform.

Pass/fail behavior is pinned by checked-in inputs under
`fixtures/bootstrap-boundary/`, covering an accepted inventory, a walked
directory surface, and rejected missing, out-of-root, private-path,
private-marker-in-file, malformed-exemption, and non-lint-inheriting cases.

## Release premise

There is no supported release at bootstrap. The workspace version is `0.0.0`
and product maturity is experimental. A later E01 candidate may use an immutable
source tag plus GitHub release/archive after the portable frontend contract and
assurance work are accepted. Registry publication remains evidence-driven.

The repository is intentionally unlicensed until a license is selected before
the first supported release candidate.
