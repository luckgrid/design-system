# Layer-ownership fixtures

Classification: **internal**.

Each directory is a small stylesheet graph with an `index.css` entry. Cargo tests
run the `ds-check` layer-ownership validator against every case, and the browser
suite loads the behavioral cases to observe the cascade that actually results.

| Case | Expected result |
|---|---|
| `accepted-import-owned` | passes; the import supplies `layer(ds.components)` and the child orders unqualified local sub-layers |
| `accepted-source-owned` | passes; the child file owns `ds.tokens` itself and is imported without a layer wrapper |
| `rejected-exact-duplicate` | fails; an import-owned child redeclares its parent `ds.components` |
| `rejected-prefix-qualified` | fails; an import-owned child declares `ds.components.*`, which nests as `ds.components.ds.components.*` in a browser |
| `rejected-double-import` | fails; one stylesheet enters the cascade twice |
| `rejected-unlayered-rule` | fails; a Design System rule sits outside every named layer |
| `rejected-tailwind-directive` | fails; the core requires a Tailwind directive |
| `rejected-non-relative-import` | fails; the import is not a quoted relative path |
