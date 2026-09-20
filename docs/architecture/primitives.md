# Primitives

Status: **public-preview**. The primitives are two opt-in class hooks with
documented contracts. **Surface** is a base primitive: a bounded region. It
composes with any layout and owns no element or state. **Action** is a UI
primitive: a native `<button>`, or a link that reads as a control. Its state
comes from native HTML and ARIA.

The primitives ship inside the core export. The entrypoint imports
`packages/styles/primitives.css` into `ds.primitives`. That file imports one
module per primitive into internal sub-layers, in this order:
`ds.primitives.surface`, `ds.primitives.action`. The layer order is unchanged:
a primitive rule outranks a base rule and a layout rule. Every later Design
System layer, and every consumer layer, outranks a primitive rule.

## Taxonomy

The authoring hierarchy runs: classless base → layouts and base primitives →
UI primitives → components and patterns → utilities.

- A **base primitive** is a recurring presentation relationship that many
  interfaces can use without owning one element's meaning or interaction.
  It has one hook and one rule. It has no variant, no state, and no layout
  property, so a layout hook on the same element arranges its children with no
  conflict.
- A **UI primitive** is a reusable contract for one native element or role.
  The platform supplies its semantics, keyboard behavior, and state; the
  primitive styles them. It may have variants, and it styles only the native
  and ARIA states listed in `primitives.tsv`.
- A **component or pattern** combines primitives around a product meaning,
  such as a card, a breadcrumb trail, or a hero. None is promoted here.

A candidate is promoted only when both evidence consumers (Luna and
Luckgrid.net) repeat it, or when a DS-E01 fixture needs it, and its meaning is
product-neutral. Every other candidate is listed under
[Rejected and deferred](#rejected-and-deferred) with its reason.

## Rules every primitive follows

- **Hooks and selectors come from the inventory.** Each rule's selector is one
  zero-specificity `:where()` derived from `primitives.tsv`. For a primitive
  it is `:where(.ds-<name>)`. For a variant it is
  `:where(.ds-<name>.ds-<name>-<variant>)`. A state appends its native selector
  to the hook. Rules appear in inventory order: the primitive, then its
  variants, then its states. Any consumer rule wins, even an unlayered one.
- **State is native.** No primitive tests a `data-*` attribute. Disabled is
  the native `disabled` attribute; the current item is `aria-current`; an
  unavailable link is an `<a>` with no `href`. A primitive never sets
  `pointer-events`, `opacity`, or `visibility` to fake a state.
- **Focus stays with the base.** No primitive declares an `outline` property.
  The base's single `:focus-visible` outline applies to every primitive
  unchanged.
- **No query, no custom property.** No primitive uses a media query, container
  query, `@supports`, or `@scope`, and none declares a custom property.
  Spacing that should scale comes from the bounded fluid spacing roles, and
  the arrangement of children comes from the layouts, which are intrinsic. A
  consumer changes a primitive by setting the real property.
- **Values come from the token authority.** Every length and color binds to a
  public semantic role in [`tokens.md`](tokens.md). No primitive reads a
  `--ds-ref-*` value.

`ds-check primitive` enforces each rule above.

## Contract

`primitives.tsv` classifies each hook and state below as `public-preview`.
Each primitive's public contract is:

- its hook, its variant hooks, and the elements it may sit on;
- the native states it styles;
- the roles it reads;
- the rules above.

The sub-layer names and the exact declarations are `internal`.

### surface

Kind: `base-primitive`. Hook: `.ds-surface`.

| Aspect | Contract |
|---|---|
| Relationship | a region set off from the canvas: `--ds-color-surface` fill, `--ds-color-text` text, a `--ds-border-width` solid `--ds-color-border` border, `--ds-radius-control` corners, and `--ds-space-flow` padding on every side |
| Element | any element; the surface adds no meaning. Use the element the content calls for: an `<article>`, a `<section>` with a heading, or a `<div>` |
| State | none; a surface is not interactive. Put interaction inside it |
| Arrangement | none. Add `.ds-stack`, `.ds-cluster`, or `.ds-grid` on the same element, or inside it, to arrange its children. The surface declares no layout property, so the two never conflict |
| Responsiveness | none of its own. Its padding is the bounded fluid `--ds-space-flow` role, and the layout it composes with is intrinsic, so no breakpoint is needed |
| Override | set the real property (`background-color`, `padding-inline`, `border-radius`, and so on) from a consumer layer or rule |
| Compatibility | the hook and the relationship are `public-preview`; the exact declarations are `internal` |

A card is a surface plus a stack on one element. Card is not a separate
primitive.

### action

Kind: `ui-primitive`. Hooks: `.ds-action`, and the variants
`.ds-action-primary`, `.ds-action-quiet`, `.ds-action-icon`.

| Aspect | Contract |
|---|---|
| Roots | a `<button>`, with its `type` set by the consumer, or an `<a>`. A link keeps link semantics: it takes no `role="button"`. Choose `<button>` for an in-page command and `<a href>` for navigation |
| Default | an inline control with a `--ds-size-target-min` minimum block and inline size. It has control padding, the border, `--ds-radius-control` corners, a `--ds-color-surface` fill, `--ds-color-text` text, no underline, and a pointer cursor. Its content (a label and an optional icon) is centered with a `--ds-space-control-inline` gap |
| `.ds-action-primary` | the one emphasized action in a group: `--ds-color-accent` fill and border, `--ds-color-on-accent` text |
| `.ds-action-quiet` | no fill and a transparent border that keeps its width, so the box does not move on hover and forced-colors mode still draws it |
| `.ds-action-icon` | a square at the minimum target size with no padding. The consumer supplies the accessible name, for example with `aria-label` or visually hidden text |
| States | `:hover` shows an accent border. `[aria-current]` (any value except `"false"`) shows the highlight fill, an accent border, and strong weight; the weight is the non-colour cue. `:disabled` on a `<button>` shows muted text and a not-allowed cursor, and the browser blocks activation. `:not(:any-link)` on an `<a>` with no `href` looks the same: the element is not a link, is not focusable, and cannot be activated |
| Order | a state rule follows the variants, so a disabled primary action looks disabled and a current quiet action shows its fill |
| Keyboard and focus | native. A `<button>` activates on Enter and Space, a link on Enter. The base's `:focus-visible` outline is unchanged |
| Not provided | a pressed toggle (`aria-pressed`), a critical or destructive emphasis, loading or busy state, and `aria-disabled`. `aria-disabled` needs script to block activation, so a consumer that uses it owns that script and its styling |
| Responsiveness | none; an action sizes to its content and never falls below the target size. Group actions with `.ds-cluster` |
| Override | set the real property from a consumer layer or rule; a consumer rule wins over every action rule, including state rules |
| Compatibility | the hooks, the roots, and the four states are `public-preview`; the exact declarations are `internal` |

## Evidence

Both evidence consumers repeat an action control. Luna's `button.css` and
Luckgrid.net's `action.css` each style a filled default button and link
actions, an icon-only variant, a borderless ghost or text variant, an
`aria-current` current state, a hover state, and a disabled state. Both
consumers also use `role="button"` or a `data-*` hook where a native element or
attribute already carries the meaning; those parts are rejected below. Neither
consumer uses `aria-pressed`, so a pressed state is not promoted.

Luckgrid.net uses a bordered surface region (`[data-card]`) heavily; Luna uses
it once and more often composes an article card. Surface is promoted as the
smallest product-neutral part of that pattern, a bounded region. The card
arrangement is left to the layouts, and the card meaning to the consumer.

Both evidence consumers share an author. µWiki is the named candidate for a
materially unlike consumer; the release review re-checks these contracts
against it.

## Browser support

The primitives need no feature beyond the accepted browser floor (Chromium
123, Firefox 121, Safari 17.5):

| Feature | Disposition |
|---|---|
| `inline-flex` with `gap` | required; supported far below the floor |
| `aspect-ratio` | required for the icon variant; supported below the floor |
| `:any-link`, `:disabled`, `:hover`, attribute selectors | required; supported far below the floor |
| logical properties | required, as for the base |
| size container queries | not used: neither primitive changes with its own size |
| `@scope` | not used: a class hook under `:where()` needs no scoping, and `@scope` is only progressive at the floor |

No primitive has a progressive or fallback path.

## Overriding

Declare consumer layers after the core export, as described in
[`css-entrypoint.md`](css-entrypoint.md). To change a primitive, set its real
property on the element in a consumer rule:

```css
@layer app;

@layer app {
  .pill {
    border-radius: 999px;
  }

  .flat {
    border-color: transparent;
    background-color: var(--ds-color-canvas);
  }
}
```

```html
<button class="ds-action pill" type="button">Save</button>
<div class="ds-surface flat">…</div>
```

Because every primitive rule has zero specificity, the override needs no
`!important`, no Design System class in the selector, and no knowledge of the
sub-layers. The same works in unlayered CSS. A consumer rule also wins over a
state rule, so a consumer that restyles an action restyles its states too.

To give a brand its own colors, map them into the semantic roles as described
in [`tokens.md`](tokens.md); both primitives follow.

## Rejected and deferred

These candidates came from the Luna primitive and component inventory, the
Luckgrid.net templates and styles, and the E01 fixtures. Neither consumer
evidence nor these rows is a backlog commitment.

| Candidate | Disposition | Reason | Why |
|---|---|---|---|
| `breadcrumbs` | consumer | `product-composition` | a navigation pattern built from links and a list; it composes the action and cluster when a consumer wants it |
| `hero` | consumer | `product-composition` | page composition |
| `nav` | consumer | `product-composition` | landmarks stay consumer-owned (see [`base.md`](base.md)); a navigation list is a cluster of actions or links |
| `brand` | consumer | `product-identity` | product identity |
| `icon` | consumer | `product-identity` | icon sets and their assets are product choices; an icon-only control uses `.ds-action-icon` |
| `disclosure` | deferred | `native-pattern-refinement` | `<details>` open-state styling belongs to DS-E01.S3.T5 native-pattern refinement |
| `dialog` | deferred | `native-pattern-refinement` | Luna's dialog depends on script; native `<dialog>` refinement is DS-E01.S3.T5 |
| `popover` | deferred | `native-pattern-refinement` | Popover and anchor positioning are progressive features for DS-E01.S3.T5 |
| `tooltip` | deferred | `native-pattern-refinement` | depends on popover and anchor positioning; DS-E01.S3.T5 |
| `progress` | deferred | `native-pattern-refinement` | `progress` and `meter` are excluded from the base and owned by DS-E01.S3.T5 |
| `field` | deferred | `no-repeat-evidence` | a label, hint, and error grouping is used by one consumer only |
| `select` | deferred | `no-repeat-evidence` | used by one consumer only; the customizable select is a DS-E01.S3.T5 candidate |
| `table` | deferred | `no-repeat-evidence` | no consumer uses a table hook; the base styles tables |
| `tag` | deferred | `no-repeat-evidence` | too few uses in both consumers |
| `alert` | deferred | `no-repeat-evidence` | few uses in either consumer; alert semantics need an accessibility review first |
| `action-critical` | deferred | `no-repeat-evidence` | a destructive emphasis is used once, in one consumer |
| `action-pressed` | deferred | `no-repeat-evidence` | neither consumer uses `aria-pressed` |
| `card` | rejected | `covered-by-promoted` | a card is `.ds-surface` with `.ds-stack` on one element |
| `action-text` | rejected | `covered-by-promoted` | the quiet variant covers a borderless, unfilled action |
| `action-outline` | rejected | `covered-by-base` | the default action already has a border and a neutral fill |
| `link` | rejected | `covered-by-base` | the base styles every link and keeps its underline; a link that reads as a control uses `.ds-action` |
| `disabled-data-hook` | rejected | `duplicates-native-state` | `[data-disabled]` or a `.disabled` class duplicates the native `disabled` attribute or a missing `href` |
| `nav-active-data-hook` | rejected | `duplicates-native-state` | `[data-active]` duplicates `aria-current` |
| `link-role-button` | rejected | `conflicts-with-native-semantics` | `<a role="button">` announces a button that does not activate on Space; use a `<button>` |

`ds-check primitive` fails if the stylesheets, `primitives.tsv`, this document,
or `fixtures/primitives` disagree. In this table it checks every row's
disposition and reason code against the inventory.

## Accessibility notes

- **Semantics.** A hook changes only presentation. It adds no role. An action
  is a real `<button>` or a real link, so assistive technology announces what
  the element is.
- **Keyboard and focus.** Native. The base's focus outline is never restyled,
  and the browser suite checks it on every variant.
- **Target size.** Every action is at least `--ds-size-target-min` (2.75rem,
  44 CSS pixels at the default font size) in both dimensions, including the
  icon variant.
- **Contrast.** The default and quiet actions use the text and surface or
  canvas roles. The primary action uses `--ds-color-on-accent` on
  `--ds-color-accent`. The current state uses the text role on the highlight
  role. axe reports no violation in either scheme. A disabled action is exempt
  from contrast requirements, and its muted text still reads.
- **Non-colour cues.** An action link has no underline; its border and fill
  mark it as a control. The current state adds strong weight. The quiet variant
  keeps a transparent border, which forced-colors mode draws.
- **Reduced motion.** The primitives add no motion.

## Classification and checks

`primitives.tsv` lists each primitive, variant, and state as `public-preview`,
and each other candidate as `consumer`, `deferred`, or `rejected` with a reason
code from a fixed list. Adding, removing, or changing a hook, a state, or its
documented contract is a reviewed public-preview change. `ds-check primitive`
fails when:

- `primitives.css` does anything other than declare the sub-layer order and
  import one module per promoted primitive, in inventory order, or a module is
  missing or unexpected;
- a module holds an at-rule, including a media query, container query,
  `@supports`, or `@scope`;
- a selector is anything other than the rules the inventory derives, in any
  spelling, or a rule is missing, repeated, or out of order;
- a selector tests a `data-*` attribute;
- a base primitive declares a layout property, a state, or a variant;
- a declaration restyles the outline, declares a custom property, or is not one
  of the kind's classified properties;
- a value uses any unit, a literal length or color, a `--ds-ref-*` reference, a
  non-Design-System `var()`, a `var()` fallback, or a function other than
  `var()` and `calc()`;
- a primitive's subsection of this document does not name its kind, its hooks,
  and its states, or the contract names an unpromoted hook;
- this document's candidate table does not list exactly the inventory's
  candidates, each with the same disposition and reason;
- the primitives fixture has no `main`, misses a hook or a markup-expressible
  state, uses an unpromoted `ds-` class, puts a variant without its primitive
  or an action on anything other than `<button>` or `<a>`, gives a primitive a
  `data-*` or `role` attribute, puts two primitives on one element, or never
  composes the surface with a layout.

The browser suite (`tests/browser/specs/primitives.spec.mjs`) checks the
primitives in Chromium, Firefox, and WebKit:

- the rules sit in `ds.primitives.<name>` with zero specificity;
- each variant and state computes its documented roles, in both schemes;
- the states stay native: a disabled button and an unlinked `<a>` cannot be
  focused or activated, and `aria-current="false"` is not current;
- a button activates from the keyboard, and a link action follows its link;
- the focus outline and target size on every variant;
- surface and layout composition;
- layered and unlayered consumer overrides;
- no axe violation in either scheme.
