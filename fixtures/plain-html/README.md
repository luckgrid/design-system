# Plain HTML consumer fixture

Classification: **internal**.

This fixture is the first consumer of the supported Design System CSS. It loads:

1. the declared `core.css` export at its public path, `/packages/styles/index.css`,
   which brings its own token and classless base imports;
2. its own `consumer.css`, which declares the consumer layer `app` after `ds`.

It uses no framework, Tailwind, network fetch, script, private repository path,
unpublished alias, or Rust runtime in the browser. The browser suite serves it
from a static server that answers only declared exports and fixture files, so an
undeclared path fails the run instead of loading silently.

`consumer.css` belongs to the fixture. It is consumer-owned CSS, not a Design
System export, token contract, or theme contract. It reads public semantic roles
such as `--ds-color-canvas` and never a `--ds-ref-*` reference value;
`ds-check tokens` rejects a consumer stylesheet that assigns an internal token.

The page is also the classless base fixture. It holds every subject that
`base.tsv` owns: text, a sentence-level link, lists, code, an image and an
inline SVG, a table, a labelled form with each control type, disclosure, a
popover, and an open dialog. It also has content stress: a long word, a long
fragment URL, and a right-to-left paragraph. `ds-check base` fails if an owned
element or attribute is missing. The only non-stylesheet asset is the local
`figure.svg`. Its in-page links are same-document fragments, which load
nothing.
