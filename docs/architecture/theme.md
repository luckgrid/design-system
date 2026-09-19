# Theme contract

The theme decides which branch every `light-dark()` semantic color role
resolves. It sets the CSS `color-scheme` property and nothing else. The
[token document](tokens.md) describes the roles themselves.

The theme ships inside the core export `core.css`. `tokens.css` imports it into
the internal sub-layer `ds.tokens.theme`, after the semantic roles, from
`packages/styles/tokens/theme.css`. No extra stylesheet, class, script, or build
step is needed.

## Default

```css
:root {
  color-scheme: light dark;
}
```

With no hook value, the page follows the user's OS or browser preference
(`prefers-color-scheme`). Every `light-dark()` role, the user-agent form
controls, scrollbars, and the default canvas follow the same preference.

## Explicit light or dark

The one public hook is the `data-ds-scheme` attribute on the root element
(`<html>`):

| Markup | `color-scheme` on the root | `light-dark()` roles resolve |
|---|---|---|
| no attribute | `light dark` | the user's preference |
| `data-ds-scheme="light"` | `light` | light, whatever the preference |
| `data-ds-scheme="dark"` | `dark` | dark, whatever the preference |
| any other value | `light dark` | the user's preference |

Because the hook sets `color-scheme` itself, the semantic roles and the native
controls always agree: a form control never shows a light widget on a dark
canvas.

A consumer switches schemes by setting or removing the attribute, from
server-rendered markup or from its own script:

```html
<html lang="en" data-ds-scheme="dark">
```

```js
document.documentElement.dataset.dsScheme = "light"; // explicit light
delete document.documentElement.dataset.dsScheme;    // back to the preference
```

The Design System ships no theme script. The toggle control, persistence
(for example `localStorage` or a cookie), and any server-side default belong to
the consumer.

The hook is read only on the root element. `data-ds-scheme` on another element
has no effect.

## Precedence with consumer CSS

A consumer layer such as `app`, declared after `ds`, outranks the whole theme.
If a consumer sets `color-scheme` on `:root` or `html` in that layer, it takes
over scheme selection and `data-ds-scheme` stops working. Consumers that use the
hook must leave the root `color-scheme` to the Design System. `ds-check theme`
rejects a root `color-scheme` in the repository's own consumer fixtures for
this reason.

A consumer can still style its own content under the hook, for example
`:root[data-ds-scheme="dark"] img { opacity: 0.9; }`, and can set
`color-scheme` on its own non-root elements.

## Brand themes

A brand theme is not a scheme. A consumer brand maps its own values into the
public semantic roles from its own layer, as the [token document](tokens.md)
describes. A brand's light/dark pairs are ordinary `light-dark()` assignments,
so they follow the default and `data-ds-scheme` with no extra hook. The
`fixtures/brand-theme` fixture proves this with one fictional brand.

The mapping seam is:

- public: the `--ds-*` semantic roles in `tokens.tsv` and the
  `data-ds-scheme` hook with its `light` and `dark` values in `theme.tsv`, all
  `public-preview`;
- consumer-owned: brand palettes, fonts, and other raw values, named in the
  consumer's own namespace; the scheme toggle and its persistence;
- internal: every `--ds-ref-*` reference value and the `ds.tokens.*`
  sub-layers. These are not an override seam.

One fixture brand shows the seam works. It does not show that several materially
different real brands can share one release. That proof is later work.

## Scoped themes

Scoped (subtree) themes are not part of this contract. No current consumer
needs them. `light-dark()` already follows a nested `color-scheme`, but the
root-declared aliases such as `--ds-color-focus` resolve where they are
declared. A supported subtree theme therefore needs its own design and
evidence. Until then, `data-ds-scheme` applies to the whole document only.

## Forced colors and reduced motion

The theme does not change forced-colors or reduced-motion behavior. In
forced-colors mode the browser replaces author colors with system colors
whatever the scheme. Accessibility review of both modes is part of the release
review, not this contract.

## Classification and checks

`theme.tsv` classifies the default and the hook as `public-preview`. Their
meaning may still change through a reviewed preview revision. `ds-check theme`
fails when:

- `theme.css` differs from `theme.tsv`, or places a rule inside a conditional
  group;
- `theme.css` assigns any custom property, which would make it a second token
  authority;
- any other Design System stylesheet sets `color-scheme`;
- any stylesheet uses a theme hook other than `data-ds-scheme`, such as a
  `.dark` class, in any spelling: escaped, spaced, namespaced, or inside
  `:is()`, `:where()`, `:not()`, or `:has()`. A selector the check cannot
  classify fails rather than passing;
- a consumer fixture sets `color-scheme` on a selector whose subject is the
  root, including `html:root` and `:where(:root)`;
- this document's `Default` block is not exactly the default rule, its behavior
  table does not give each state the scheme in `theme.tsv`, or it names a hook
  value, hook attribute, or alias that `theme.tsv` does not classify.

The browser suite (`tests/browser/specs/theme.spec.mjs`) checks the default,
explicit, invalid-value, runtime-switch, native-control, brand, and precedence
behavior in Chromium, Firefox, and WebKit.
