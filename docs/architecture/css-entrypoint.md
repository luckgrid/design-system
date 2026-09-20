# CSS entrypoint and cascade contract

Status: **public-preview**. The project is experimental and has no supported
release yet. This page describes the first CSS surface consumers may use, and
what may still change.

## Entrypoint

| Export | Path | Class |
|---|---|---|
| `core.css` | `packages/styles/index.css` | `public-preview` |

`exports.tsv` declares the export, and `bootstrap-surfaces.tsv` classifies the
same path exactly once. `ds-check` rejects duplicate or aliased inventory paths,
overlapping directory rows, and rows that resolve to the same file;
`ds-check layers` fails if export and inventory classes disagree, or if a
public stylesheet is not exported.

The authored source file *is* the artifact. There is no compiled or generated
copy, and no Tailwind, framework, JavaScript, or Rust is needed to use it. A
future compiled form, if one is ever added, is a reproducible projection of this
source rather than a second authority.

Load it with a plain stylesheet link at its public path:

```html
<link rel="stylesheet" href="/packages/styles/index.css">
<link rel="stylesheet" href="./app.css">
```

## Layer order

The entrypoint publishes this order before any Design System layer holds
rules:

```css
@layer ds.tokens, ds.base, ds.layouts, ds.primitives, ds.components, ds.utilities;
```

| Name | Class | Consumer use |
|---|---|---|
| `ds` | `public-preview` | The one targetable name. Declare consumer layers after it. |
| order of `ds.tokens` … `ds.utilities` | `public-preview` | Documented precedence: a later child outranks an earlier one. Do not author rules into these layers. |
| anything nested below a `ds.*` child | `internal` | No compatibility promise. |

The entrypoint imports the token authority, `tokens.css`, into `ds.tokens`; see
[`tokens.md`](tokens.md). `tokens.css` also imports the theme contract into the
internal sub-layer `ds.tokens.theme`; see [`theme.md`](theme.md). The entrypoint
then imports the classless base, `base.css`, into `ds.base`; see
[`base.md`](base.md). It then imports the layout primitives, `layouts.css`, into
`ds.layouts`; see [`layouts.md`](layouts.md). It then imports the primitives,
`primitives.css`, into `ds.primitives`; see [`primitives.md`](primitives.md).
The entrypoint itself holds only the order statement and those four imports. It adds no `ds.reset` layer; reset behavior belongs to the base. Adding a new
`ds.*` child later is a reviewed public-preview change.

## Overriding

Declare consumer layers after loading the entrypoint:

```css
@layer app;

@layer app {
  main {
    max-inline-size: 48rem;
  }
}
```

Any layer declared after `ds` outranks every Design System layer through layer
order alone. A plain element selector in `app` beats a more specific selector in
any `ds.*` layer, so normal extension never needs higher specificity or
`!important`. The browser suite checks this in Chromium, Firefox, and WebKit.

Do not name `ds.*` child layers from consumer stylesheets. Besides writing into
layers the consumer does not own, it is not load-order safe: Chromium has been
observed to keep a later stylesheet's `ds.*` child order when that stylesheet
finishes loading before the entrypoint's imports. Layers declared after `ds`,
such as `app`, are not affected.

Unlayered consumer CSS also outranks every layer. That is supported, but a named
consumer layer is the recommended extension path because it keeps the
consumer's own precedence explicit.

## Authoring rules for Design System source

`ds-check layers` enforces these rules on every stylesheet reached from an
export:

- each stylesheet enters the cascade exactly once, and every tracked
  stylesheet under `packages/styles/` must be reached from an export;
- an export begins with its `@layer` order statement;
- imports are quoted relative `./` paths inside `packages/styles/`, with an
  optional named `layer(...)`, and never `url()`, a network URL, or an
  anonymous layer;
- when an import supplies `layer(P)`, the imported stylesheet must not declare
  `P` or `P.*`. It orders its own sub-layers with unqualified names, such as
  `@layer base, variants;`, which nest as `P.base` and `P.variants`;
- a source-owned public layer has exactly one owning stylesheet;
- every rule sits inside a named layer, and anonymous layers are rejected;
- at a stylesheet's own top level, every layer it names sits under `ds`
  (`ds` or `ds.*`), so Design System source cannot create a sibling layer that
  outranks or undercuts the documented order;
- no declaration uses a `!` priority such as `!important`: consumer layers win
  by order alone, and an important Design System declaration would invert that;
- Tailwind directives such as `@tailwind`, `@theme`, `@apply`, and `@utility` are
  rejected, because the core must be correct as plain browser CSS;
- the scanner tokenizes like a browser: a string may not contain an unescaped
  newline, an escaped code point such as `\{` is never structure, and at-rule
  names must be written without escapes. Otherwise a rule the scanner saw inside
  a layer could land outside it in the browser.

The checked-in cases under `fixtures/layer-ownership/` cover these rules. The
browser suite also shows why the prefix rule exists: a prefix-qualified
statement under an import-owned parent nests as `ds.components.ds.components.*`
and leaves the intended child order to first appearance.
