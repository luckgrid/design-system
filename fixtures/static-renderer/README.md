# Static renderer consumer fixture

Classification: **internal**.

This fixture is a purpose-built assurance consumer. It answers one question:
can the promoted core be consumed by a materially unlike renderer, using only
declared public exports, with no Rust, Node, Tailwind, or JavaScript framework
in the consumer path? It is not a product demo and not a reusable Hugo adapter.

## Pieces

| Path | Role |
|---|---|
| `HUGO_VERSION` | The one exact renderer version. `build.sh` fails on any other. |
| `hugo.toml`, `layouts/`, `content/`, `static/` | Fixture-owned Hugo site. |
| `consumer/theme.css` | Consumer-owned semantic theme override in `@layer app`. |
| `consumer/site.css` | Consumer-owned page shell and normal cascade overrides. |
| `stage.sh` | Producer step: stages the declared exports into a release-shaped boundary. |
| `build.sh` | Consumer step: renders the site with the pinned Hugo. |
| `verify-toolchain-free.sh` | Runs both steps with no Rust, Node, or Tailwind command on `PATH`. |

## Staged public-export boundary

`stage.sh` reads `exports.tsv` and copies each declared export under its export
name, together with exactly the stylesheets it imports, laid out relative to the
export file. The result, `stage/design-system/`, contains `core.css` and its
import tree and no repository path. Hugo mounts it as `design-system/`, so pages
load `design-system/core.css` and nothing else from the Design System. The
staged tree and `stage/MANIFEST.tsv` (SHA-256 per file) are generated and
ignored. The release archive's `css/` directory has the same shape, and `DS_PACKAGED_CSS=<archive>/css`
makes `stage.sh` (and `verify-toolchain-free.sh`) stage from an unpacked archive instead of the
repository, so release verification renders this fixture from the packaged form.

## Commands

From the repository root:

```sh
sh fixtures/static-renderer/stage.sh
sh fixtures/static-renderer/build.sh
sh fixtures/static-renderer/verify-toolchain-free.sh
cargo run --locked -p design-system-check -- static-renderer \
  exports.tsv layouts.tsv primitives.tsv theme.tsv base.tsv fixtures/static-renderer
```

Staging and rendering need only Hugo and POSIX shell tools. `ds-check
static-renderer` is contributor tooling that verifies the result; a consumer
does not need it.

## Pages

| Page | Scheme | Exercises |
|---|---|---|
| Overview | default | `.ds-grid`, `.ds-surface`, `.ds-stack`, the default scheme |
| Hooks | `light` | layouts, surfaces, every action variant and state, consumer overrides |
| Article | `dark` | classless semantic base on ordinary HTML with no class attribute |
| Forms | default | form controls, breadcrumb navigation, actions in a surface |

## Independence axes

Each axis is recorded and checked separately rather than inferred from the
renderer name.

| Axis | Evidence |
|---|---|
| Renderer language and runtime | Hugo, a Go program; the source repository's contributor path is Rust and Node. |
| Static generation | `build.sh` writes plain files; no server or script runs in the page. |
| Tailwind-free | No Tailwind input, directive, or output in any consumer input. |
| Framework-free | No script element, package manifest, or framework in the site. |
| Public-export-only | Only `core.css` is linked from the Design System; internal modules are absent. |
| No Rust or Node to consume | `verify-toolchain-free.sh` stages and renders with neither on `PATH`. |
| Consumer-owned theme and override | `consumer/` maps `--ds-color-*` roles and overrides hooks in `app`. |

## Not proven here

Browser-floor, keyboard, and assistive-technology evidence belongs to the
release-candidate review, not this fixture. Renderer integration stays
fixture-owned; nothing here is promoted as a shared adapter. A later, second
unlike consumer could be a Go-templated or Python-based static generator; no
such proof is claimed.
