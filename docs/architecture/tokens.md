# Design tokens

The core export `core.css` (`packages/styles/index.css`) imports the token
authority into the `ds.tokens` layer. The tokens are plain CSS custom properties
on `:root`. Loading them needs no Tailwind processing, build step, or Rust
runtime.

## Two tiers

| Tier | Prefix | Source | Class | Purpose |
|---|---|---|---|---|
| reference | `--ds-ref-*` | `packages/styles/tokens/reference.css` | `internal` | raw, scheme-neutral values: OKLCH colors, font stacks, bounded fluid scales |
| semantic | `--ds-*` | `packages/styles/tokens/semantic.css` | `public-preview` | brand-neutral roles that base, theme, and component CSS consume |

`tokens.tsv` classifies every declared property, one row each, with its value
type. `ds-check tokens` fails when the stylesheets and the inventory disagree.

Reference values are **not** an override seam. They may be renamed, re-derived,
or removed without a compatibility event, so do not override any `--ds-ref-*`
property. The same applies to any other property the inventory marks internal.
Public roles are the only supported mapping surface. They are `public-preview`,
so their names and meanings may still change through a reviewed preview
revision. No token is `public-stable` yet.

## Public semantic roles

### Color

Each color role, except the alias, is a light/dark pair written as
`light-dark(<light reference>, <dark reference>)`.

| Role | Meaning |
|---|---|
| `--ds-color-canvas` | page background |
| `--ds-color-surface` | slightly separated background, for example code or a raised region |
| `--ds-color-text` | default text on canvas or surface |
| `--ds-color-text-muted` | secondary text that still meets text contrast on canvas |
| `--ds-color-border` | control and separator borders, at least 3:1 against canvas and surface |
| `--ds-color-accent` | links and the primary interactive emphasis |
| `--ds-color-on-accent` | text and icons drawn on an accent fill |
| `--ds-color-critical` | errors and invalid state |
| `--ds-color-highlight` | marked or selected text background, used with `--ds-color-text` |
| `--ds-color-focus` | focus indicator; **an alias of `--ds-color-accent`** that follows it unless mapped separately |

The default values meet WCAG 2.2 AA contrast in both schemes:

| Pair | Contrast |
|---|---|
| text / canvas | ≥ 18:1 |
| muted text / canvas and surface | ≥ 6.9:1 |
| accent / canvas | ≥ 5.2:1 |
| on-accent / accent | ≥ 5.2:1 |
| critical / canvas | ≥ 6.1:1 |
| border / canvas and surface | ≥ 3.2:1 |

`light-dark()` picks its branch from the used `color-scheme` of the element that
**consumes** the role, not of `:root`. Custom properties carry the unresolved
function, so a subtree with a different `color-scheme` resolves the other branch.
The token stylesheets themselves do not set `color-scheme`. The theme stylesheet
does: by default the roles follow the user's preference, and
`data-ds-scheme="light"` or `"dark"` on `<html>` selects one explicitly. See the
[theme document](theme.md).

Forced-colors and high-contrast modes are a separate accessibility obligation.
The light/dark pairs do not address them.

### Typography

| Role | Type | Behavior |
|---|---|---|
| `--ds-font-body` | font-family | platform sans-serif stack |
| `--ds-font-heading` | font-family | **alias of `--ds-font-body`** |
| `--ds-font-code` | font-family | platform monospace stack |
| `--ds-text-small` | length | fluid 12px → 16px |
| `--ds-text-body` | length | fluid 16px → 22px |
| `--ds-text-lead` | length | fluid 18px → 24px |
| `--ds-text-heading-1` | length | fluid 36px → 44px |
| `--ds-text-heading-2` | length | fluid 32px → 40px |
| `--ds-text-heading-3` | length | fluid 28px → 36px |
| `--ds-text-heading-4` | length | fluid 24px → 32px |
| `--ds-text-heading-5` | length | fluid 20px → 26px |
| `--ds-text-heading-6` | length | fluid 18px → 24px |
| `--ds-leading-body` | number | fixed 1.5 |
| `--ds-leading-heading` | number | fixed 1.333 |
| `--ds-weight-body` | number | fixed 400 |
| `--ds-weight-strong` | number | fixed 700 |
| `--ds-weight-heading` | number | fixed 700 |

### Space and geometry

| Role | Type | Behavior |
|---|---|---|
| `--ds-space-control-block` | length | fluid 6px → 12px, block padding inside controls |
| `--ds-space-control-inline` | length | fluid 8px → 16px, inline padding inside controls |
| `--ds-space-flow` | length | fluid 16px → 32px, rhythm between flow content |
| `--ds-space-gutter` | length | fluid 24px → 96px, inline padding of page regions |
| `--ds-space-section` | length | fluid 48px → 144px, separation between major sections |
| `--ds-border-width` | length | fixed 1px |
| `--ds-radius-control` | length | fixed 0.375rem |
| `--ds-focus-width` | length | fixed 2px, the WCAG 2.2 focus-appearance minimum |
| `--ds-focus-offset` | length | fixed 2px |
| `--ds-size-target-min` | length | fixed 2.75rem minimum pointer target |
| `--ds-size-measure` | length | fixed 65ch comfortable line length |

## Fluid and fixed values

A fluid role grows linearly with the viewport width. It stays at its minimum at
480px and below, reaches its maximum at 2560px, and holds there above that. The
formula is `clamp(<min>rem, <intercept>rem + <slope>vw, <max>rem)`. Because it
is rem-based, it also follows the user's font size. Every fluid text maximum is
at most 2.5× its minimum, so zoomed text can still reach 200%.

Fluid interpolation is used only where continuous scaling is the design intent:
text sizes and content spacing. Values whose meaning is a threshold stay fixed:
border and focus geometry, minimum target size, content measure, control radius,
line height, and weight. Scaling those with the viewport would weaken an
interaction or accessibility guarantee.

## Mapping a brand

A consumer brand keeps its own palette and fonts in its own custom properties.
It then maps them into the semantic roles from a layer declared after `ds`:

```css
@layer app {
  :root {
    --brand-violet-40: oklch(45% 0.2 300);
    --brand-violet-80: oklch(80% 0.1 300);

    --ds-color-accent: light-dark(var(--brand-violet-40), var(--brand-violet-80));
    --ds-font-body: "Brand Sans", system-ui, sans-serif;
  }
}
```

The `app` layer outranks every `ds.*` layer, so no `!important` or extra
specificity is needed.

- An alias follows its target. Mapping `--ds-color-accent` also moves
  `--ds-color-focus`, and mapping `--ds-font-body` also moves
  `--ds-font-heading`, unless they are mapped separately.
- A mapping replaces a role's whole value, so the new value must keep the role's
  value type.
- A brand's own light/dark pairs are ordinary `light-dark()` assignments. Brand
  identity is not a `color-scheme`. The pairs follow the theme default and the
  `data-ds-scheme` hook with no extra work; see the [theme document](theme.md).

Map roles on `:root`. A role such as `--ds-color-focus` resolves its alias where
it is declared, so assigning `--ds-color-accent` only on a descendant does not
move the root-declared focus alias for that subtree.
