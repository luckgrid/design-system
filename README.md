# Design System

A portable, multi-brand, web-platform-first design system: CSS-first,
semantic-HTML-first, framework-agnostic, and independently versioned.

> **Status: experimental.** There is no released CSS and no `public-stable`
> surface yet. The first CSS entrypoint exists as a `public-preview` surface.

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

## Current state

The repository currently provides:

- a Rust 2024 / resolver-3 Cargo workspace pinned to Rust 1.98.1;
- one contributor tool, `ds-check`, for source-boundary and CSS layer-contract
  validation;
- the first portable CSS entrypoint, `core.css` at `packages/styles/index.css`
  (`public-preview`), which publishes the shared cascade layer order; see
  [`docs/architecture/css-entrypoint.md`](docs/architecture/css-entrypoint.md);
- a plain HTML consumer fixture that loads only that export;
- browser contract tests in Chromium, Firefox, and WebKit under `tests/browser/`;
- explicit compatibility classification in `bootstrap-surfaces.tsv`;
- least-privilege GitHub Actions quality checks.

The entrypoint does not yet define tokens, a theme, or element styles. Later
DS-E01.S2 work adds those.

## Contributor checks

From the repository root:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo +1.98.1 check --workspace --all-targets --locked
cargo run --locked -p design-system-check -- check \
  bootstrap-surfaces.tsv fixtures/plain-html
cargo run --locked -p design-system-check -- layers \
  exports.tsv bootstrap-surfaces.tsv fixtures/plain-html
(cd tests/browser && npm ci && npx playwright install chromium firefox webkit && npx playwright test)
```

The browser tests are contributor tooling. Consuming the CSS never requires
Node, Rust, or Cargo.

See [CONTRIBUTING.md](CONTRIBUTING.md) for the contributor workflow and
[SECURITY.md](SECURITY.md) for vulnerability reporting.

## Stability

Repository paths are not API by default.

| Class | Meaning |
|---|---|
| `public-stable` | Supported compatibility surface. None exists yet. |
| `public-preview` | Deliberately exposed but still moving. Currently only the `core.css` entrypoint and its layer order. |
| `internal` | Not API; may change or disappear. Every other path uses this class. |

Product maturity and compatibility classification are separate. Experimental
project maturity does not make an internal path public.

## What this is not

- Not a component framework for React, Solid, Vue, or another UI runtime.
- Not a Tailwind plugin. Tailwind may later exist as an optional adapter.
- Not a backend/service framework.
- Not a general-purpose UI kit copied from one consumer.
- Not yet a released CSS package or release artifact.

## License

**This repository is currently unlicensed.** It is public and readable, but
default copyright applies: public visibility is not permission to copy, modify,
or redistribute the source.

Open-source licensing is intended but deliberately deferred until before the
first supported release candidate. No `LICENSE` file exists yet.

## Provenance

Planning/specification provenance is described in
[WORKSTREAMS.md](WORKSTREAMS.md). That file is human/process metadata only and
is not required to build, test, or eventually consume this repository.
