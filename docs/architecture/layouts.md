# Layout primitives

Status: **public-preview**. The layouts arrange the direct children of one
element: a **stack** (one column), a **cluster** (a wrapping row), and a
**grid** (equal columns sized by the available space). An element opts in with
one class hook. The layouts carry no page, route, or product composition. The
page shell stays with the consumer.

The layouts ship inside the core export. The entrypoint imports
`packages/styles/layouts.css` into `ds.layouts`. That file imports one module
per layout into internal sub-layers, in this order: `ds.layouts.stack`,
`ds.layouts.cluster`, `ds.layouts.grid`. The layer order established by
**DS-E01.S3.T1 — Layout Primitives** is unchanged, so a layout rule outranks a
base rule and every later Design System layer and every consumer layer outranks
a layout rule.

## Rules every layout follows

- **One hook, zero specificity.** Each layout has one class hook. Its rules
  select only `:where(.ds-<layout>)` and that element's direct children,
  `:where(.ds-<layout>) > :where(*)`. Both carry zero specificity, so any
  consumer rule wins, even in an unlayered stylesheet.
- **Source order is visual order.** No layout uses `order`, a reversed
  direction, dense packing, explicit grid placement, `float`, or positioning.
  Children appear, and take keyboard focus, in DOM order. In right-to-left
  content, the inline direction flips with the text, as it should.
- **Intrinsic, not breakpoint-driven.** No layout uses a media query, container
  query, or `@supports` rule. The cluster wraps and the grid adds or removes
  columns from the space they actually have, in any container, at any viewport.
- **Values come from the token authority.** Gaps bind to the public spacing
  roles in [`tokens.md`](tokens.md). The grid's minimum column reads
  `--ds-size-measure`. Only track sizes use `fr` and `%`. No layout declares a
  custom property: a consumer changes a layout by setting the real property.
- **Logical properties only.** Every spacing and sizing rule is logical, so the
  layouts follow the writing direction.

## Contract

`layouts.tsv` classifies each hook below as `public-preview`. Each layout's
public contract is its hook, the arrangement of the direct children, the gap
role, and the rules above. The sub-layer names, the exact declarations, and the
grid's minimum-column expression are `internal`.

### stack

Hook: `.ds-stack`.

| Aspect | Contract |
|---|---|
| Children | any elements, in one column in source order; each stretches to the full inline size |
| Gap | `--ds-space-flow` between children |
| Child margins | the block margins of direct children are removed, so the gap is the only spacing between them |
| Overflow | children may shrink below their content width; long words wrap through the base's `overflow-wrap` |
| Needs | none; a stack works in any container and nests in any layout |

Use it for vertical rhythm in a region: a section's heading and paragraphs, a
card body, a form's field groups, a sidebar's blocks.

### cluster

Hook: `.ds-cluster`.

| Aspect | Contract |
|---|---|
| Children | any elements, in a row in source order that wraps onto new lines when the row is full |
| Gap | `--ds-space-control-block` between lines and `--ds-space-control-inline` between items |
| Alignment | items are centered on the cross axis, so mixed-height controls line up |
| Child margins | the block margins of direct children are removed |
| Overflow | children may shrink below their content width, so one long word wraps inside its item instead of widening the page |
| Needs | none |

Use it for small inline groups whose count and width vary: tags, metadata,
button groups, and navigation links. A `<ul>` or `<ol>` keeps its list
semantics; remove its markers in consumer CSS if they are not wanted.

### grid

Hook: `.ds-grid`.

| Aspect | Contract |
|---|---|
| Children | any elements, placed one per cell in source order, row by row |
| Columns | as many equal columns as fit. Each column is at least one third of `--ds-size-measure`, or the full width when less space is available, so a narrow container gets one column and never overflows |
| Gap | `--ds-space-flow` between rows and columns |
| Row height | each row is as tall as its tallest cell |
| Few items | empty tracks are kept (`auto-fill`), so one or two items keep their column width instead of stretching across the row |
| Child margins | the block margins of direct children are removed |
| Needs | none; the grid needs no breakpoint and no container query |

Use it for collections of like items: cards, galleries, and feature lists. It
is not a page grid. A 12-column page grid with named areas is page
composition and stays with the consumer.

## Selection rationale

A layout is promoted only when a reusable need exists beyond one page
arrangement.

| Layout | Reusable need |
|---|---|
| stack | vertical rhythm recurs in regions, cards, forms, navigation, and long-form content |
| cluster | wrapping inline groups recur for tags, metadata, actions, and navigation links |
| grid | intrinsic collections recur for cards, galleries, catalogs, and feature lists; page grids and named areas remain consumer composition |

## Browser support

The layouts need no feature beyond the accepted browser floor (Chromium 123,
Firefox 121, Safari 17.5):

| Feature | Disposition |
|---|---|
| flex `gap` | required; supported far below the floor |
| `repeat(auto-fill, minmax())` | required; supported far below the floor |
| `min()` and `calc()` in track sizes | required; supported far below the floor |
| logical properties | required, as for the base |
| size container queries | not used: the cluster and grid respond to their own space without a query |
| `@scope` | not used: a class hook under `:where()` needs no scoping, and `@scope` is only progressive at the floor |
| `reading-flow`, `order`, and other visual reordering | not used: source order is the reading order |

No layout has a progressive or fallback path, because none depends on a
feature outside the floor.

## Overriding

Declare consumer layers after the core export, as described in
[`css-entrypoint.md`](css-entrypoint.md). To change a layout, set the layout's
own property on the element in a consumer rule:

```css
@layer app;

@layer app {
  .card-list {
    gap: 0.5rem;
    grid-template-columns: repeat(2, 1fr);
  }
}
```

```html
<ul class="ds-grid card-list">…</ul>
```

Because the hooks have zero specificity, the override needs no `!important`, no
Design System class in the selector, and no knowledge of the sub-layers. The
same works in unlayered CSS. Do not reorder children visually with `order` or
grid placement; if the order must change, change the source.

Put one layout hook on an element. To combine layouts, nest them: a stack
inside a grid cell, or a cluster inside a stack. A layout may share its element
with the surface base primitive, which declares no layout property, but never
with an action, which owns its `display`. [`hooks.md`](hooks.md) states this
composition rule and the scoping boundary of every hook. The child rule
`:where(.ds-<layout>) > :where(*)` is internal: to change one child, put a
consumer class on it.

## Rejected and deferred

These candidates were evaluated and not promoted. `layouts.tsv` records each
one with its disposition and a reason code. None is a backlog commitment.

| Candidate | Disposition | Reason |
|---|---|---|
| `page-shell` | consumer | body and landmark shells, viewport filling, and sticky headers are page composition. No Design System rule selects a landmark; a consumer may put a layout hook on one |
| `article-layout` | consumer | named areas, sidebar panels, and sticky offsets are page arrangement |
| `column-grid` | consumer | the 12- and 16-column page grids with spans are a page-template framework, which this contract excludes |
| `scroll-timeline` | consumer | scroll-driven page effects are product behavior, not layout |
| `center` | deferred | a measure-bounded centered column; the only consumer hook for it is unused |
| `sidebar` | deferred | no consumer uses a reusable sidebar-and-content layout outside page shells |
| `switcher` | deferred | no consumer evidence |
| `cover` | deferred | no consumer evidence; full-height sections are page composition |
| `frame` | deferred | no consumer evidence for an aspect-ratio frame layout |
| `reel` | deferred | no consumer uses a horizontal scrolling reel; scroll snap is progressive |
| `container` | deferred | a hook that only establishes a size container has no consumer use; containment is a consumer decision until a promoted module needs it |
| `flex-switch` | rejected | cluster and grid already adapt to their available space |
| `footer-reorder` | rejected | moving footer navigation with `order` separates visual order from source and keyboard order |

## Accessibility notes

- **Order.** Visual order, reading order, and keyboard order follow the DOM in
  every layout. The browser suite checks this in all three engines, in
  left-to-right and right-to-left content.
- **Semantics.** A hook changes only layout. It adds no role, and lists keep
  their list semantics.
- **Reflow.** The fixture has no horizontal page scroll at 320 CSS pixels or at
  the width of a 1280-pixel window zoomed to 400%, including long unbroken
  words in stacks, clusters, and grid cells.
- **Reduced motion.** The layouts add no motion.

## Classification and checks

`layouts.tsv` lists each hook as `public-preview` and each other candidate as
`consumer`, `deferred`, or `rejected`. Adding, removing, or changing a hook or
its documented contract is a reviewed public-preview change. `ds-check layout`
fails when:

- `layouts.css` does anything other than declare the sub-layer order and import
  one module per promoted layout, in inventory order, or a module is missing or
  unexpected;
- a module holds an at-rule, including a media query, container query,
  `@supports`, or `@scope`;
- a selector is anything other than the module's `:where(.ds-<layout>)` or
  `:where(.ds-<layout>) > :where(*)`, in any spelling, or a rule appears twice,
  or the container rule is missing or sets no `display`;
- a declaration can reorder content (`order`, `float`, `flex-flow`,
  `grid-auto-flow`, grid placement or areas, `position`, or a `reverse` or
  `dense` value), is not one of the classified logical layout properties, or
  declares a custom property;
- a value uses a unit other than `fr` and `%`, a literal length, a
  `--ds-ref-*` reference, a non-Design-System `var()`, a `var()` fallback, or a
  function other than `var()`, `calc()`, `min()`, `repeat()`, and `minmax()`;
- this document's contract or candidate list does not match `layouts.tsv`;
- the layouts fixture has no `main`, misses a hook, uses an unpromoted `ds-`
  class, or puts two hooks on one element.

The browser suite (`tests/browser/specs/layouts.spec.mjs`) checks the layouts
in Chromium, Firefox, and WebKit:

- layer placement and zero specificity;
- the computed display model and token-bound gaps;
- visual order and keyboard order against DOM order, left to right and right to
  left;
- reflow and content stress from 320 CSS pixels to 1280;
- nesting and consumer overrides, layered and unlayered;
- an axe-core scan of the fixture in both schemes.
