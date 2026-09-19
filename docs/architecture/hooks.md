# Hooks and scoping

Status: **public-preview**. This page states the Design System's public hook
vocabulary and the scoping boundary of every module that has a hook: the
layouts in [`layouts.md`](layouts.md), the primitives in
[`primitives.md`](primitives.md), and the theme attribute in
[`theme.md`](theme.md).

A hook is the markup a consumer writes to opt an element into a Design System
contract. Everything else is internal: the sub-layers, the exact declarations,
and the rule a layout uses to reach its children.

## The scoping boundary

The hook is the boundary. Every rule that selects a hook is anchored at the
element that carries the hook:

- a layout, primitive, or variant rule selects the hooked element only, as one
  zero-specificity `:where()`;
- a state rule adds a native or ARIA state to that same element;
- a layout rule also reaches the hooked element's direct children, as
  `:where(.ds-<layout>) > :where(*)`, and nothing deeper.

No hooked rule depends on an ancestor, a sibling, or a descendant. It tests
only the hooked element's own state (`:hover`, `:disabled`, `:any-link`, an
attribute) and, for the theme, `:root`. None uses `:has()`, a structural or
`:focus-within` pseudo-class, or a shadow-tree selector. None negates a hook
or offers an alternative without it, and none hides a combinator inside
`:where()`, `:is()`, or `:not()`.
So a hook styles the same way wherever it sits. It cannot reach into
consumer markup it does not own. Consumer markup that only looks like a hook,
such as a variant class without its primitive's class, a `data-*` attribute, or
a class that merely contains `ds-`, gets no Design System styling.

No stylesheet uses `@scope`. A scope root would bound descendant selectors, but
no hooked rule has one. At the accepted browser floor (Chromium 123, Firefox
121, Safari 17.5), `@scope` is only progressive, so no rule that a hook needs
may depend on it. A later module may use `@scope` only as an enhancement over a
complete unscoped baseline, and only after its own reviewed decision. No build
step lowers `@scope`, and none is claimed.

## Public hooks

Each row is a hook, the kind of contract it opts into, the module or contract
that owns it, and its compatibility class. `ds-check hooks` derives this list
from `layouts.tsv`, `primitives.tsv`, and `theme.tsv`, and fails if this table
differs.

| Hook | Kind | Owner | Class | Meaning |
|---|---|---|---|---|
| `.ds-stack` | `layout` | `stack` | `public-preview` | children in one column with flow spacing |
| `.ds-cluster` | `layout` | `cluster` | `public-preview` | children in a wrapping row with control spacing |
| `.ds-grid` | `layout` | `grid` | `public-preview` | children in intrinsic equal-width tracks |
| `.ds-surface` | `base-primitive` | `surface` | `public-preview` | a region set off from the canvas |
| `.ds-action` | `ui-primitive` | `action` | `public-preview` | a `<button>` or `<a>` that reads as a control |
| `.ds-action-primary` | `variant` | `action` | `public-preview` | the one emphasized action in a group |
| `.ds-action-quiet` | `variant` | `action` | `public-preview` | an action with no fill or visible border at rest |
| `.ds-action-icon` | `variant` | `action` | `public-preview` | a square, icon-only action |
| `data-ds-scheme` | `theme-attribute` | `theme` | `public-preview` | on the root element, the explicit color scheme |

The action's states are not hooks. They are the native and ARIA states listed
in [`primitives.md`](primitives.md): `:hover`, `[aria-current]`, `:disabled`,
and an `<a>` with no `href`. Their selectors are `public-preview` with the
action.

## Vocabulary rules

- **A class names the contract.** A layout or primitive is a `ds-` class.
  A variant is a second class, `ds-<primitive>-<variant>`, that works only
  beside its primitive's class.
- **State is native.** A state is the element's own state or its ARIA state.
  No hook duplicates one, so there is no `data-state`, `data-disabled`, or
  `.is-active`.
- **One attribute hook.** `data-ds-scheme` on the root element is the only
  attribute a Design System rule tests. Every other `data-*` name is reserved:
  no Design System rule tests it, and consumers may use `data-*` freely.
- **Hooks are for contracts that exist.** A hook is added only with the module
  that needs it and its own reviewed decision. No hook exists for a future
  component.

The class hooks here are the clearest portable contract for these modules.
Each opts one element into one contract, and each variant refines its own
primitive. None of the modules has a slot, an internal descendant, or a state
the platform does not already express. The authoring order still starts with
semantic HTML and native state. A `data-*` hook becomes a candidate only when a
module needs one of those things.

## Composition

- Put **one layout hook** on an element. To combine layouts, nest them.
- A **base primitive composes with a layout on one element**. A surface
  declares no layout property, so `.ds-surface.ds-stack` is a stacked surface,
  and a card is exactly that.
- A **UI primitive never shares its element with a layout hook**. An action
  owns its `display`, so a layout on the same element gives a mixed result that
  no contract describes. Put the layout on the parent: a `.ds-cluster` of
  actions, for example.
- Put **one primitive** on an element. To combine them, nest them: an action
  inside a surface.

`ds-check primitive` and `ds-check hooks` reject a fixture that puts a layout
hook on an action or two primitives on one element.

## Reserved and rejected

| Form | Disposition | Why |
|---|---|---|
| `data-component`, `data-slot` | reserved | no selected module has a component root distinct from its class hook, or an internal slot. A future component that needs one names it `data-ds-*`, which needs its own decision and a narrower `ds-check theme` alias rule |
| `data-state`, `data-disabled`, `data-active` | rejected | they duplicate native and ARIA state |
| `data-variant`, `data-ds-layout` | rejected | a variant or layout choice is a class; a second spelling of the same contract doubles the public surface |
| unprefixed `data-*` or class hooks (`data-stack`, `.stack`) | rejected | they collide with consumer names |
| `data-theme`, `.dark` | rejected | theme aliases; `data-ds-scheme` is the one theme hook |
| component custom properties (`--ds-action-bg`) | rejected | a consumer overrides the real property; a second override seam adds a token class |
| `@scope` | not used | progressive at the browser floor, and no hooked rule has a descendant to bound |

## Overriding

A consumer overrides a hooked element by setting the real property on that
element, from a consumer layer or unlayered CSS:

```css
@layer app;

@layer app {
  .toolbar {
    gap: 0;
  }

  .toolbar > .step {
    margin-inline-start: auto;
  }
}
```

```html
<div class="ds-cluster toolbar">
  <a class="ds-action ds-action-quiet step" href="#next">Next</a>
</div>
```

Every Design System rule has zero specificity, so the override needs no
`!important`, no Design System class in the selector, and no knowledge of the
sub-layers. A consumer never needs to repeat a layout's child selector. The
layout's `> :where(*)` rule is internal, and a consumer's own class on the
child wins over it.

## Browser support

| Feature | Disposition |
|---|---|
| `:where()` | required; Chromium 88, Firefox 78, Safari 14 |
| `:not()` with a selector list | required for the current state; Chromium 88, Firefox 84, Safari 9 |
| the `i` attribute-selector flag | required for the current state; Chromium 49, Firefox 47, Safari 9 |
| `@scope` | not used; progressive at the floor |

## Classification and checks

Adding, removing, or changing a hook, its kind, or its documented meaning is a
reviewed public-preview change. `ds-check hooks` fails when:

- a Design System stylesheet reached from `exports.tsv` selects a class that is
  not a public hook, in any spelling;
- a rule tests a `data-*` attribute other than the theme hook;
- a hooked selector, read as written (the whitespace that ends an escape is
  not collapsed), does any of these:
  - puts a class or attribute hook anywhere but its first compound;
  - negates a hook inside `:not()`;
  - offers an alternative without the hook in `:is()` or `:where()`;
  - hides a combinator inside a pseudo-class argument;
  - uses a pseudo-class other than `:where()`, `:is()`, `:not()`, `:hover`,
    `:disabled`, `:any-link`, and `:root` (so no `:has()`, structural,
    `:focus-within`, or shadow-tree selector);
  - tests the theme attribute away from `:root`;
  - reaches past the hooked element other than a layout's `> :where(*)`;
- any stylesheet uses `@scope` or a nested rule, or spells an at-rule with an
  escape;
- this document's hook table does not list exactly the inventories' hooks, each
  with its kind, owner, and class;
- the scoping fixture has no `main`, misses a class hook, uses a `ds-` class
  that is not a hook, puts two primitives on one element, or puts a layout
  hook on a UI primitive.

The fixture check reads markup lexically. It does not decode a character
reference in a class name or inspect a hook inside `<template>`.

The browser suite (`tests/browser/specs/scoping.spec.mjs`) checks the boundary
in Chromium, Firefox, and WebKit on `fixtures/scoping`:

- hooks nested inside each other keep their own contracts;
- every published hooked rule is one anchored `:where()` (or a layout's child
  rule, or the theme attribute on `:root`), with no combinator or alternative
  inside it, and every variant sits with its primitive;
- lookalike markup gets no Design System styling;
- a layout does not reach its grandchildren;
- the current state follows the ARIA values;
- consumer overrides win without any Design System selector;
- no axe violation in either scheme.
