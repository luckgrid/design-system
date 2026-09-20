# Classless semantic base

Status: **public-preview**. The base gives ordinary semantic HTML coherent
defaults, so a page that loads `core.css` and nothing else reads well: text,
links, lists, code, tables, forms, disclosure, and dialogs. It needs no class,
wrapper, script, or framework.

The base ships inside the core export. The entrypoint imports
`packages/styles/base.css` into `ds.base`. That file imports one module per
group into internal sub-layers, in this order: `ds.base.document`,
`ds.base.content`, `ds.base.forms`, `ds.base.interactive`. The T1 layer order
is unchanged.

## Rules every base selector and value follows

- **Zero specificity.** Every selector is one `:where(...)`, sometimes followed
  by `::placeholder` or `::file-selector-button`. Any consumer rule, even a bare
  element selector in the same layer, therefore outranks a base rule.
- **No product hooks.** Selectors use element names, native pseudo-classes
  (`:any-link`, `:hover`, `:visited`, `:focus-visible`, `:disabled`,
  `:user-invalid`, and the logical `:not()`, `:is()`, `:where()`), and the
  native `type`, `popover`, `multiple`, and `size` attributes. They use no
  class, id, `data-*` attribute, or `role`, and they make no assumption about
  the page's wrappers.
- **Values come from the token authority.** Colors, fonts, sizes, spacing,
  radius, and focus geometry come from the public `--ds-*` roles in
  [`tokens.md`](tokens.md). Only font-relative offsets use `em`, `ch`, `lh`, or
  `%`. The base declares no custom property and reads no `--ds-ref-*` value.
- **Native behavior stays native.** The base sets no transition or animation.
  It never removes an outline, and it keeps native control appearance.

Because every rule reads the semantic roles, the base follows the theme: the
default preference and the `data-ds-scheme` hook in [`theme.md`](theme.md)
switch it with no extra rule. A brand that maps its values into the roles
restyles the base in the same way.

## Owned defaults

`base.tsv` classifies each subject below as `public-preview`.

### document

| Subject | Default |
|---|---|
| `html` | canvas and text colors, body font family, body line height, `overflow-wrap: break-word` so long words and URLs wrap instead of widening the page |
| `body` | body text size and weight. The size is set on `<body>`, not the root, so `rem` keeps its initial meaning |
| `:focus-visible` | one focus indicator for every focusable element: an outline of `--ds-focus-width` in `--ds-color-focus`, offset by `--ds-focus-offset`. An outline survives forced-colors mode, where a shadow would not |

### content

| Subject | Default |
|---|---|
| `h1`, `h2`, `h3`, `h4`, `h5`, `h6` | heading font, weight, and line height; one heading text role per level; balanced wrapping |
| `p`, `ul`, `ol`, `dl`, `blockquote`, `pre`, `figure`, `table` | one trailing block gap from `--ds-space-flow` |
| `a` | links (`a:any-link`) use the accent color **and an underline in every state**. The underline is the non-colour cue: link text differs from body text by colour alone at 3.89:1 in light and 2.22:1 in dark. Hover thickens the underline |
| `strong`, `b`, `dt` | the strong weight |
| `small` | the small text role |
| `mark` | the highlight role |
| `blockquote` | an inline-start rule in the border role, no UA indent |
| `hr` | a rule in the border role |
| `code`, `kbd`, `samp` | the code font on the surface role, at `0.875em` |
| `pre` | the code font on the surface role. Long lines scroll inside the block |
| `img`, `svg` | never wider than their container; images keep their aspect ratio |
| `figcaption` | the small text role in the muted color |
| `caption`, `th`, `td` | start-aligned cells with control padding and a border-role rule; `th` uses the strong weight |

### forms

| Subject | Default |
|---|---|
| `input`, `select`, `textarea` | inherit the document font and color. Text-like fields get the canvas role, a border-role outline box, the control radius and padding, and at least `--ds-size-target-min` block size |
| `button` | a **neutral** button on the surface role with a border, with at least the minimum target size. `input` buttons and the file-selector button match it. A primary or branded button is a product role and is not in the base |
| `fieldset`, `legend` | a border-role group box; the legend uses the strong weight |

Checkboxes and radios keep native rendering and take `accent-color` from the
accent role. Placeholders use the muted role. Disabled controls use the muted
role and a `not-allowed` cursor. `:user-invalid` fields take the critical border.
`:user-invalid` waits for user interaction, so an untouched required field is
not flagged.

WebKit draws a single-line `<select>` natively and ignores its padding and
`min-block-size`, so it would measure 23 to 31 CSS pixels. Single-line selects
therefore get an explicit block size: one line (`1lh`) plus the control padding
and border, never below `--ds-size-target-min`. That keeps native appearance
and gives every engine at least its natural height. `lh` is supported at the
accepted floor (Chromium 109, Firefox 120, Safari 16.4).

### interactive

| Subject | Default |
|---|---|
| `details`, `summary` | a block gap for `details`; a pointer cursor on `summary`. The disclosure marker and toggling stay native |
| `dialog` | the canvas and text roles with a border, radius, and control padding. Opening, focus movement, Escape, and the `::backdrop` scrim stay native |
| `[popover]` | the same surface as `dialog`, and nothing else. See below |

**Popover is progressive.** The Popover API is not available at the accepted
browser floor (Firefox 121). The base styles only the surface: no position,
`display`, or animation. Where Popover is unsupported, the element is ordinary
visible content in the page and still reads correctly. Do not put content that
users must reach behind a popover alone.

## Exclusions

These elements get no base styling beyond the user agent's. Styling them would
need a layout, primitive, or component decision that the global base cannot
make for every page. `base.tsv` lists each one as `excluded` with its owner:
a program task, or `consumer`.

| Excluded | Why | Owner |
|---|---|---|
| `header`, `nav`, `main`, `aside`, `footer`, `section`, `article`, `search`, `menu` | landmarks and sectioning are page layout, so no Design System rule selects them. A consumer may still put a layout hook on one; see [`layouts.md`](layouts.md) | consumer |
| `progress`, `meter`, `video`, `audio`, `iframe`, `canvas` | native widgets and embedded media whose styling is a component or pattern choice | native-pattern-refinement |

The base also deliberately does not do these things:

- **No custom control appearance.** It does not set `appearance`, so it does
  not customize select pickers, range, color, date, or file inputs.
- **No responsive table wrapper.** Changing a table's `display` can remove its
  table semantics, so a wide table does not scroll on its own. Wrap wide data in
  a consumer-owned scroll container.
- **No ARIA-role widgets.** Tabs, menus, comboboxes, and similar widgets are
  component contracts.
- **No popover positioning or animation.**
- **No custom backdrop.** No token models a scrim yet.
- **No page layout.** It sets no body margin or content measure. The opt-in
  layouts in [`layouts.md`](layouts.md) arrange content; the page shell stays
  with the consumer.

## Overriding

Declare consumer layers after the core export, as described in
[`css-entrypoint.md`](css-entrypoint.md). A consumer `@layer app` rule wins over
every base rule through layer order alone. Because base selectors have zero
specificity, unlayered consumer CSS and same-specificity rules win too. For
example:

```css
@layer app;

@layer app {
  a {
    text-decoration-thickness: 2px;
  }
}
```

If a consumer removes the link underline, it takes over the non-colour cue and
must provide another one, such as a 3:1 contrast difference plus a focus and
hover cue.

## Accessibility notes

- **Keyboard.** The base changes no native keyboard behavior. The browser suite
  tabs through the fixture in all three engines and checks that each stop shows
  the focus outline in DOM order.
- **Reflow.** Long words and URLs wrap, `pre` scrolls inside itself, and images
  and SVG shrink to fit. The fixture has no horizontal page scroll at 320 CSS
  pixels.
- **Forced colors.** Focus uses an outline, and colors come from roles the
  browser replaces in forced-colors mode. The full forced-colors review is part
  of the release review.
- **Reduced motion.** The base adds no motion.
- **Right-to-left.** Every spacing and border rule uses logical properties.

## Classification and checks

`base.tsv` lists each owned subject as `public-preview` and each exclusion as
`excluded`. Adding, removing, or changing an owned default is a reviewed
public-preview change. `ds-check base` fails when:

- `base.css` does anything other than order and import the four group modules,
  or a module is missing or unexpected;
- a module holds an at-rule, including a conditional group;
- a selector is not one literal `:where(...)`, or uses a class, id, `*`,
  `data-*`, `role`, another attribute, or a non-native pseudo-class, in any
  spelling (escaped, spaced, or with a comment);
- a selector styles an excluded element, or a subject `base.tsv` does not own,
  or an owned subject has no rule;
- a value uses a color literal or named color, a unit other than `em`, `ch`,
  `lh`, or `%`, a `--ds-ref-*` reference, a non-Design-System `var()`, a `var()`
  fallback, a string, or a function other than `var()`, `calc()`, `min()`, and
  `max()`;
- a rule declares a custom property, `color-scheme`, `appearance`, a
  transition, or an animation, or removes the outline;
- this document's owned tables or exclusions do not match `base.tsv`, or the
  plain fixture lacks an owned element or attribute or carries a `class`
  attribute.

The browser suite (`tests/browser/specs/base.spec.mjs`) checks the base in
Chromium, Firefox, and WebKit:

- layer placement and token binding in both schemes;
- the link underline and contrast;
- keyboard focus order and the focus outline;
- keyboard operation of the native controls, disclosure, dialog, and popover;
- target size and reflow;
- consumer precedence;
- the absence of motion;
- an axe-core scan of the fixture in both schemes.
