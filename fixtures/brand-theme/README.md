# Brand theme consumer fixture

Classification: **internal**.

This fixture shows one consumer-owned brand, the fictional "Tidewater", mapped
onto the Design System through public surfaces only. It loads:

1. the declared `core.css` export at `/packages/styles/index.css`;
2. its own `brand.css`, which declares the consumer layer `app` after `ds`.

`brand.css` keeps the brand's palette and font in its own `--tidewater-*`
custom properties and assigns them to public-preview semantic roles such as
`--ds-color-accent`. It overrides no `--ds-ref-*` reference value, uses no
`!important`, and edits no Design System file. `ds-check tokens` rejects a
consumer stylesheet that assigns an internal token, and `ds-check theme` rejects
one that sets the root `color-scheme` or uses a theme hook other than
`data-ds-scheme`.

The fixture sets no scheme itself and ships no script. The page follows the
user's preference by default. The browser suite sets `data-ds-scheme` on
`<html>` to prove explicit light and dark selection, as a consumer's own toggle
would. Switching, persistence, and the toggle's UI stay consumer-owned.

The fixture is one brand. It proves the mapping seam, not that several
materially different real brands can share one release; that proof belongs to
DS-E02.
