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

Do not copy Luna CSS here wholesale.
