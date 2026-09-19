# Styles source

This directory holds the authored shared styles source for the Design System.

`index.css` is the one supported CSS entrypoint. It is declared as the
`core.css` export in `exports.tsv` and classified **`public-preview`**. See
[`docs/architecture/css-entrypoint.md`](/docs/architecture/css-entrypoint.md)
for the layer order, override pattern, and authoring rules.

Everything else here is `internal`. A stylesheet becomes a consumer surface only
when it is declared in `exports.tsv`, not because it appears in this public
directory. `ds-check layers` enforces the layer-ownership rules on every
stylesheet reached from an export.

`tokens.css` and `tokens/` hold the token authority. The stylesheet files are
`internal`; the custom properties they declare are classified one by one in
`tokens.tsv`, where semantic roles are `public-preview` and `--ds-ref-*`
reference values are `internal`. See
[`docs/architecture/tokens.md`](/docs/architecture/tokens.md).

`tokens/theme.css` is the theme contract. It sets only `color-scheme`: the
default follows the user's preference, and the `data-ds-scheme` hook on
`<html>` selects light or dark. Its default and hook are classified in
`theme.tsv`. See [`docs/architecture/theme.md`](/docs/architecture/theme.md).

`base.css` and `base/` are the classless semantic base in `ds.base`: one module
per group (document, content, forms, interactive), zero-specificity `:where()`
selectors, and values bound to the semantic roles. The owned subjects and the
exclusions are classified in `base.tsv`. See
[`docs/architecture/base.md`](/docs/architecture/base.md).

`layouts.css` and `layouts/` are the layout primitives in `ds.layouts`: one
module per layout (stack, cluster, grid), each opted into by a class hook
(`.ds-stack`, `.ds-cluster`, `.ds-grid`) under a zero-specificity `:where()`.
The hooks are classified in `layouts.tsv`, which also records the candidates
that stay consumer-owned, deferred, or rejected. See
[`docs/architecture/layouts.md`](/docs/architecture/layouts.md).

`primitives.css` and `primitives/` are the primitives in `ds.primitives`: the
`surface` base primitive (`.ds-surface`) and the `action` UI primitive
(`.ds-action` with its variants), each under a zero-specificity `:where()`.
The hooks, variants, and native states are classified in `primitives.tsv`,
which also records the candidates that stay consumer-owned, deferred, or
rejected. See
[`docs/architecture/primitives.md`](/docs/architecture/primitives.md).

Every rule that selects a hook styles only the element that carries it; a
layout also reaches its direct children. No module uses `@scope`, and no rule
tests a `data-*` attribute other than `data-ds-scheme`. See
[`docs/architecture/hooks.md`](/docs/architecture/hooks.md).

Do not copy Luna CSS here wholesale.
