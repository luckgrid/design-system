# Design System

A portable, multi-brand, web-platform-first design system: CSS-first,
semantic-HTML-first, framework-agnostic, and independently versioned.

> **Status: experimental bootstrap.** The canonical source workspace is being
> established. There is no released CSS, no supported public CSS API, and no
> `public-stable` surface yet.

## What this is

The Design System is a standalone product intended to become open source and to
be consumed through published artifacts rather than source reach-through.

Its first implementation horizon is frontend-focused:

- portable CSS with an explicit cascade/layer contract;
- semantic and classless base styling before specialized hooks;
- reference and semantic tokens with consumer-owned theme overrides;
- base primitives and semantic UI primitives kept distinct;
- bounded fluid spacing/type and container-first reusable-module responsiveness;
- native HTML/CSS interaction before shared JavaScript where compatibility and
  accessibility evidence permits it.

Rust and Cargo are the contributor-workspace and validation substrate.
**Consuming released frontend CSS will not require Rust or Cargo.**

## Current bootstrap

The `DS-E01.S1.T3` bootstrap establishes:

- a Rust 2024 / resolver-3 Cargo workspace pinned to Rust 1.98.1;
- one contributor tool, `ds-check`, for bootstrap/source-boundary validation;
- the future frontend source root at `packages/styles/`;
- a self-contained plain HTML fixture;
- explicit bootstrap compatibility classification in
  `bootstrap-surfaces.tsv`;
- least-privilege GitHub Actions quality checks.

Every listed bootstrap surface is currently **`internal`**. The frontend
directory contains no supported CSS entrypoint yet. DS-E01.S2 owns the first
portable CSS/token/theme/cascade contract.

## Contributor checks

From the repository root:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo +1.98.1 check --workspace --all-targets --locked
cargo run --locked -p design-system-check -- check \
  bootstrap-surfaces.tsv fixtures/plain-html
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the contributor workflow and
[SECURITY.md](SECURITY.md) for vulnerability reporting.

## Stability

Repository paths are not API by default.

| Class | Meaning |
|---|---|
| `public-stable` | Supported compatibility surface. None exists yet. |
| `public-preview` | Deliberately exposed but still moving. None is required by `DS-E01.S1.T3`. |
| `internal` | Not API; may change or disappear. All current bootstrap surfaces use this class. |

Product maturity and compatibility classification are separate. Experimental
project maturity does not make an internal path public.

## What this is not

- Not a component framework for React, Solid, Vue, or another UI runtime.
- Not a Tailwind plugin. Tailwind may later exist as an optional adapter.
- Not a backend/service framework.
- Not a general-purpose UI kit copied from one consumer.
- Not yet a supported CSS package or release artifact.

## License

**This repository is currently unlicensed.** It is public and readable, but
default copyright applies: public visibility is not permission to copy, modify,
or redistribute the source.

Open-source licensing is intended but deliberately deferred until before the
first supported release candidate. No `LICENSE` file is added by the `DS-E01.S1.T3`
bootstrap.

## Provenance

Planning/specification provenance is described in
[WORKSTREAMS.md](WORKSTREAMS.md). That file is human/process metadata only and
is not required to build, test, or eventually consume this repository.
