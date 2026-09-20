# CSS authoring and processing toolchain

Status: **internal architecture guidance**. Readable CSS under
`packages/styles/` remains the conceptual and review authority.

## Operation ownership

| Operation | Owner | DS-E01.S3.T6 disposition |
|---|---|---|
| Parse and source-boundary validation | `ds-check` | Required; dependency-free Rust validation remains the contributor gate. |
| Import and layer ownership | `ds-check` | Required; the accepted layered source graph is checked fail-closed. |
| Before/after source measurement | `ds-check audit` | Required; emits deterministic file, byte, line, import, at-rule, and custom-property counts. |
| Browser-target transforms | None | Not needed while the package publishes authored CSS and has no target-transform artifact. |
| Minification and source maps | None | Not needed while no compiled artifact is released; adding a minifier would create output policy without a consumer need. |
| Framework/provider utilities | Optional adapter | Not part of core; Tailwind remains outside the portable source authority. |

## Lightning CSS evaluation

Lightning CSS was evaluated as the first low-level candidate. The Rust crate
provides the relevant parser, bundler, transform, minifier, and source-map
operations, but the current package has no compiled CSS artifact, browser
target matrix, or source-map release contract to exercise. The available crate
line is also alpha quality. **DS-E01.S3.T6 — Stylesheet Simplification and
Toolchain** therefore does not add it as an unused root dependency. If a release
artifact or target transform becomes an accepted
operation, Lightning CSS should be re-evaluated against that concrete fixture
before a framework tool becomes authoritative.

This keeps the plain export consumable without Node, TypeScript, Tailwind, a
JavaScript framework, or Rust at browser runtime.

## Simplification boundary

The three layout modules intentionally repeat the direct-child reset. The
layout checker requires one module and one exact selector contract per promoted
layout; moving the reset into an aggregate or shared selector would weaken that
ownership boundary. The repetition is therefore retained as semantic module
clarity rather than compressed for line count.
