# Bootstrap architecture

Status: experimental, internal bootstrap.

The repository is a Rust-first contributor workspace whose frontend product
remains CSS-first and semantic-HTML-first. Rust validates source and release
boundaries; it is not a runtime requirement for CSS consumers.

## Current bootstrap surfaces

The compatibility inventory is `bootstrap-surfaces.tsv`. At T3 every listed
surface is `internal`. The bootstrap intentionally exposes no
`public-stable` surface and no supported portable CSS entrypoint.

The first frontend source seam is `packages/styles/`. DS-E01.S2 will
define the first supported CSS contract; this directory's existence does not do
so early.

## Contributor check

Run:

```sh
cargo run --locked -p design-system-check -- check \
  bootstrap-surfaces.tsv fixtures/plain-html
```

The checker validates root-bounded inventory paths, explicit bootstrap
classification, supported-input privacy markers, and the self-contained plain
HTML fixture.

## Release premise

There is no supported release at bootstrap. The workspace version is `0.0.0`
and product maturity is experimental. A later E01 candidate may use an immutable
source tag plus GitHub release/archive after the portable frontend contract and
assurance work are accepted. Registry publication remains evidence-driven.

The repository is intentionally unlicensed until a license is selected before
the first supported release candidate.
