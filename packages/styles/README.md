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

Do not copy Luna CSS here wholesale.
