# DS-E01.S3.T4 migration comparison evidence

Status: **implementation evidence; pending independent review**.

This ledger compares the accepted E01 portable core with the Luna and
Luckgrid.net evidence that informed it. It is a contract comparison, not a claim
of pixel identity and not a migration of either consumer. The source products
remain independent consumers.

## Evidence boundary

| Evidence | Revision | Observation |
|---|---|---|
| Luna | `84a443a2e868d79035a003bb8d4ac3e6bff73305` | Accepted inventory pin. The locally available descendant `d7b5de5bc2c6adf4a1eaca7093f257511751b9e3` has no `packages/ds` diff from this pin. |
| Luckgrid.net | `10524a93281800d1e23ce9cfaca26dd71369a1a1` | Accepted second implementation-evidence pin. |
| Design System | `a0ea2927180946d7119e2de007a19c9e48bbe3c1` | Accepted T3 input: tokens, theme, base, three layouts, two primitives, and nine hooks. |

The accepted browser floor is Chromium 123, Firefox 121, and Safari 17.5.
Feature support below uses two different facts deliberately: the feature's own
minimum and whether every browser at the E01 floor supports it. “At the floor”
does not mean every historical or embedded browser.

The repeatable evidence is:

- the pinned Luna and Luckgrid.net files named below;
- `tokens.tsv`, `theme.tsv`, `base.tsv`, `layouts.tsv`, and `primitives.tsv`;
- the architecture contracts and fixtures in this repository;
- the eight `ds-check` commands and the Chromium/Firefox/WebKit browser suite.

Screenshots are not used as semantic, state, cascade, or compatibility proof.

## Selected E01 replacement contracts

| Luna / Luckgrid.net pattern | E01 equivalent | Preserved behavior | Intentional simplification / migration effect |
|---|---|---|---|
| Ad-hoc vertical flow in Luna article/landmark rules; Luckgrid.net stack hooks | `.ds-stack` | Source-ordered vertical rhythm with token-bound spacing | One zero-specificity hook; no page landmarks, named areas, sticky offsets, or layout custom properties. |
| Wrapping tags, metadata, links, and actions | `.ds-cluster` | Source-ordered wrapping row with logical gaps | One hook replaces product-specific group selectors; list and navigation semantics remain in consumer markup. |
| Luna section-list grids; Luckgrid.net collection grids | `.ds-grid` | Intrinsic equal-column collection layout | No 12-column/page grid, named areas, breakpoints, dense placement, or reordering. |
| Bordered card/panel regions | `.ds-surface`, optionally composed with a layout | Surface, border, radius, padding, and theme roles | “Card” is composition, not a new shared component or hook. |
| Luna button and Luckgrid.net action rules | `.ds-action` plus primary, quiet, and icon variants | Native button/link semantics, target size, focus, hover, current, disabled, and unavailable-link presentation | Native/ARIA state replaces `data-state`, `data-active`, `data-disabled`, and `role="button"` workarounds. Consumer code still owns commands, navigation, and business state. |
| Luna provider/theme variable plane and Luckgrid.net semantic variables | 29 internal references + 38 public semantic roles | Fluid spacing/type, scheme-aware colors, geometry, focus, and target-size values | Provider namespaces and brand names are removed; identical formulas consolidate; accessibility-sensitive values are re-derived. |
| `.dark`, `[data-theme]`, and `[data-color-scheme]` aliases | default `color-scheme: light dark` plus root `data-ds-scheme="light"|"dark"` | User-preference default and explicit light/dark selection | One root attribute; aliases and duplicated scheme token sets are rejected. |
| Broad Luna `data-*` vocabulary | Eight class hooks plus `data-ds-scheme` | Portable opt-in styling and consumer overrides | The hook is the scope root; no `@scope`, descendant slot API, component identity hook, or custom state vocabulary. |

### Layout candidate disposition

These rows match `layouts.tsv`. “Deferred” is an evidence disposition, not a
backlog promise.

| Candidate | Disposition | Migration decision |
|---|---|---|
| `stack` | promoted | `.ds-stack`. Replaces repeated vertical-flow relationships. |
| `cluster` | promoted | `.ds-cluster`. Replaces repeated wrapping inline groups. |
| `grid` | promoted | `.ds-grid`. Intrinsic collection grid only. |
| `page-shell` | consumer | Viewport and landmark composition stays local. |
| `article-layout` | consumer | Named areas, sidebar panels, and sticky offsets stay local. |
| `column-grid` / 12-column grid | consumer | Page art direction, not a reusable collection layout. |
| `scroll-timeline` | consumer | Product presentation behavior, not layout. |
| `center` | deferred | No multi-consumer evidence beyond ordinary max-size/margin rules. |
| `sidebar` | deferred | Existing uses are page composition, not a repeated portable contract. |
| `switcher` | deferred | No repeated need beyond cluster/grid behavior. |
| `cover` | deferred | Full-height sections are page composition. |
| `frame` | deferred | No repeated media-ratio contract. |
| `reel` | deferred | No horizontal-scroller evidence; scroll snap would be progressive. |
| `container` | deferred | A generic wrapper would duplicate consumer page composition. |
| `flex-switch` | rejected | Cluster and intrinsic grid cover the evidenced relationship. |
| `footer-reorder` | rejected | Visual reordering conflicts with the source-order contract. |

## Complete Luna stylesheet and asset disposition

Every authored Luna `packages/ds` stylesheet and SVG family at the pin is
accounted for below. Aggregates are listed as well as their leaves so moving an
import does not look like an unreviewed migration.

### Package, provider, base, and layout plane

| Luna unit | E01 disposition | Reason / replacement |
|---|---|---|
| `tailwind.css` | adapter, deferred to S4 | Tailwind-backed entrypoint is not portable core authority. E01 exposes plain CSS. |
| `src/theme.css` | replaced | Values map through the 67-token ledger below; Tailwind `@theme` does not migrate. |
| `src/utilities.css` | split / consumer | Sticky behavior stays consumer-owned; broad utility/provider decisions belong to S4. |
| `src/variants.css` | rejected as core | Tailwind variants and duplicated `data-*` states do not become public hooks. |
| `src/base.css` | structurally replaced | E01 `base.css` is an import-only aggregate with explicit layer ownership. |
| `src/base/resets.css` | base-covered | E01 owns a smaller classless semantic base rather than copying Tailwind/preflight overlap. |
| `src/base/typography.css` | base-covered | Semantic document typography binds directly to public roles. |
| `src/layouts.css` | structurally replaced | E01 `layouts.css` imports only the three promoted modules and owns each sub-layer exactly once. |
| `src/layouts/base.css` | consumer | Luna page shell, landmarks, scroll triggers, and sticky header remain local. |
| `src/layouts/article.css` | consumer | Article/collection named areas and sidebars remain local. |
| `src/primitives.css` | structurally replaced | E01 imports `surface` and `action` only. |
| `src/components.css` | no E01 component plane yet | Component layer stays empty; candidates are dispositioned below. |

### Luna primitive leaves

| Luna unit | Disposition | E01 mapping / owner |
|---|---|---|
| `primitives/article.css` | covered / consumer | Semantic article base plus `.ds-surface`/layouts where useful; product card rules remain local. |
| `primitives/aside.css` | consumer | TOC/collapsible composition and sticky behavior remain local. |
| `primitives/button.css` | promoted in simplified form | `.ds-action` and its three variants; native/ARIA state only. |
| `primitives/details.css` | base-covered, refinement deferred | Native `details`/`summary` base exists; open-state refinement belongs to T5. |
| `primitives/dialog.css` | base-covered, refinement deferred | Native `dialog` surface is base-owned; panel synchronization, positioning, and motion belong to consumer/T5. |
| `primitives/figure.css` | base-covered | Semantic figure/caption defaults; no figure hook. |
| `primitives/form.css` | structurally replaced | E01 base owns native forms; no public form aggregate hook. |
| `primitives/form/base.css` | base-covered | Semantic form defaults and roles. |
| `primitives/form/field.css` | deferred | No repeated field-group contract; no `data-field` hook. |
| `primitives/form/input.css` | base-covered, select refinement deferred | Native inputs/select/textarea keep classic-control behavior; customizable select belongs to T5. |
| `primitives/form/label.css` | base-covered | Native label/required/invalid semantics; no duplicate data state. |
| `primitives/form/output.css` | deferred | Alert/status semantics need repeat and accessibility evidence. |
| `primitives/header.css` | consumer | Product header/sticky composition remains local. |
| `primitives/link.css` | base-covered | Classless links remain links and retain an underline; control-like links may opt into `.ds-action`. |
| `primitives/nav.css` | consumer | Landmark/list composition remains local; `aria-current` replaces `data-active`. |
| `primitives/progress.css` | deferred to T5 | Native `progress`/`meter`, busy state, motion, and forced-colors need focused evidence. |
| `primitives/section.css` | base/consumer | Semantic section stays unhooked; spacing comes from layouts or consumer composition. |
| `primitives/table.css` | base-covered | Classless table defaults; responsive wrappers remain consumer-owned. |
| `primitives/text.css` | base-covered | Document typography and measure roles replace product content scopes. |

### Luna component leaves

| Luna unit | Disposition | E01 mapping / owner |
|---|---|---|
| `components/accordion.css` | deferred to T5 | Native disclosure is the candidate; no accordion component is promoted. |
| `components/brand.css` | consumer | Product identity. |
| `components/breadcrumbs.css` | consumer | Navigation composition from lists/links; not a shared component. |
| `components/card.css` | rejected as a distinct primitive | `.ds-surface.ds-stack` covers the shared relationship; card meaning stays local. |
| `components/hero.css` | consumer | Page art direction and product composition. |
| `components/icon.css` | consumer | Icon set/assets are product identity; `.ds-action-icon` covers the control geometry only. |
| `components/loader.css` | deferred | No direct consumer evidence; `@property` and busy-state motion require T5 evidence if revisited. |
| `components/tag.css` | deferred | Insufficient repeated semantic contract; cluster can arrange consumer tags. |
| `components/tooltip.css` | deferred to T5 | Popover, anchor positioning, semantics, keyboard access, and fallback require focused evidence. |

### Raw assets

All 18 raw icons—`arrow.svg`, `box-check.svg`, `box.svg`, `calendar.svg`,
`check.svg`, `chevron.svg`, `circle-dot.svg`, `circle.svg`, `clock.svg`,
`close.svg`, `link.svg`, `list.svg`, `menu.svg`, `minus.svg`, `plus.svg`,
`reference.svg`, `search.svg`, and `spinner.svg`—remain consumer-owned. E01
promotes neither an icon package nor embedded icon custom properties. This closes
the S1 “unknown” by choosing no shared asset surface for the current core.

## Public primitive candidate disposition

This condensed view matches `primitives.tsv` and explains the requested
promoted / T5-deferred / consumer / rejected split.

| Candidate group | Disposition |
|---|---|
| `surface`, `action`, `action-primary`, `action-quiet`, `action-icon`, and the four native/ARIA action states | promoted |
| breadcrumbs, hero, nav | consumer composition |
| brand, icon | consumer identity |
| disclosure, dialog, popover, tooltip, progress | deferred to T5 native-pattern refinement |
| field, select, table, tag, alert, critical action, pressed action | deferred for insufficient repeat evidence; select also remains a T5 platform candidate |
| card, text action, outline action, link primitive | rejected because promoted/base contracts already cover them |
| disabled data hook, nav-active data hook | rejected because they duplicate native/ARIA state |
| link with `role="button"` | rejected because it conflicts with native semantics and keyboard behavior |

## Token migration: all 67 target tokens

Relationship codes: **1:1** retains one source role under a neutral name;
**many:1** consolidates equivalent Luna/Luckgrid.net values; **re-derived**
changes a value for portability, gamut, contrast, or geometry; **new** adds an
E01 role required by the selected contracts. A row can have more than one code.

### Internal reference tier (29)

| E01 token | Source evidence | Relationship |
|---|---|---|
| `--ds-ref-gray-100` | Luna light + Luckgrid.net base-light | many:1 |
| `--ds-ref-gray-96` | Luckgrid.net light surface | re-derived |
| `--ds-ref-gray-72` | Luna light-3 + Luckgrid.net neutral | many:1, re-derived |
| `--ds-ref-gray-60` | Luna light-5 / stroke | 1:1 |
| `--ds-ref-gray-52` | Luna dark-5 | 1:1 |
| `--ds-ref-gray-44` | Luna dark-4 | re-derived |
| `--ds-ref-gray-20` | Luna dark-1 + Luckgrid.net dark surface | many:1, re-derived |
| `--ds-ref-gray-12` | Luna dark-1 + Luckgrid.net base-dark | many:1, re-derived |
| `--ds-ref-blue-93` | Luna light highlight | re-derived in-gamut highlight |
| `--ds-ref-blue-72` | Luna dark accent | re-derived in-gamut accent |
| `--ds-ref-blue-54` | Luna light accent | re-derived in-gamut accent |
| `--ds-ref-blue-30` | Luna dark highlight | re-derived in-gamut highlight |
| `--ds-ref-red-72` | Luna + Luckgrid.net dark alert | many:1, re-derived |
| `--ds-ref-red-52` | Luna + Luckgrid.net light alert | many:1, re-derived |
| `--ds-ref-font-sans` | both sans stacks without brand fonts | many:1, re-derived |
| `--ds-ref-font-mono` | both mono stacks without brand fonts | many:1, re-derived |
| `--ds-ref-space-6-12` | Luna `spacing-6-12` + Luckgrid.net `space-tag-1` | many:1 |
| `--ds-ref-space-8-16` | Luna `spacing-8-16` + Luckgrid.net `space-tag-2` | many:1 |
| `--ds-ref-space-16-32` | Luna `spacing-16-32` + Luckgrid.net `space-box-1` | many:1 |
| `--ds-ref-space-24-96` | both container-space formulas | many:1 |
| `--ds-ref-space-48-144` | both layout-space formulas | many:1 |
| `--ds-ref-text-12-16` | Luna small + Luckgrid.net small | many:1, coefficients re-derived |
| `--ds-ref-text-16-22` | Luna base + Luckgrid.net body | many:1, coefficients re-derived |
| `--ds-ref-text-18-24` | Luna large + Luckgrid.net lead | many:1, coefficients re-derived |
| `--ds-ref-text-20-26` | Luna extra-large + Luckgrid.net display | many:1, coefficients re-derived |
| `--ds-ref-text-24-32` | Luna 2xl + Luckgrid.net subtitle | many:1, coefficients re-derived |
| `--ds-ref-text-28-36` | Luna 3xl + Luckgrid.net headline | many:1, coefficients re-derived |
| `--ds-ref-text-32-40` | Luna 4xl + Luckgrid.net heading | many:1, coefficients re-derived |
| `--ds-ref-text-36-44` | Luna 5xl + Luckgrid.net title | many:1, coefficients re-derived |

### Public semantic tier (38)

| E01 token(s) | Source evidence | Relationship |
|---|---|---|
| `--ds-color-canvas`, `--ds-color-text` | both background/foreground pairs | many:1, renamed |
| `--ds-color-surface`, `--ds-color-text-muted`, `--ds-color-border` | both surface/muted/stroke families | many:1, re-derived |
| `--ds-color-accent`, `--ds-color-on-accent` | Luna accent + Luckgrid.net on-accent seam | re-derived |
| `--ds-color-critical` | both alert colors | many:1, renamed to avoid ARIA-role ambiguity |
| `--ds-color-highlight` | both highlight colors | many:1, re-derived |
| `--ds-color-focus` | Luna accent outline behavior | new semantic alias |
| `--ds-font-body` | both sans roles | many:1, renamed |
| `--ds-font-heading` | no distinct source role | new brand override seam |
| `--ds-font-code` | both mono roles | many:1, renamed |
| `--ds-text-small`, `--ds-text-body`, `--ds-text-lead` | corresponding source roles | many:1, renamed |
| `--ds-text-heading-1`, `--ds-text-heading-2`, `--ds-text-heading-3`, `--ds-text-heading-4`, `--ds-text-heading-5`, `--ds-text-heading-6` | Luna t-shirt sizes + Luckgrid.net named display roles | many:1, renamed by semantic heading level |
| `--ds-leading-body`, `--ds-leading-heading` | Tailwind-provided Luna values + Luckgrid.net literals | re-derived as owned literals |
| `--ds-weight-body`, `--ds-weight-strong`, `--ds-weight-heading` | Tailwind-provided Luna weights + Luckgrid.net literals | re-derived as owned literals |
| `--ds-space-control-block`, `--ds-space-control-inline` | Luna tag spacing on consolidated references | new roles |
| `--ds-space-flow` | Luna 16–32 + Luckgrid.net base space | many:1, renamed |
| `--ds-space-gutter` | both container-space roles | many:1, renamed |
| `--ds-space-section` | both layout-space roles | many:1, renamed |
| `--ds-border-width` | fluid source strokes | re-derived fixed hairline |
| `--ds-radius-control` | fluid Luna radius | re-derived fixed geometry |
| `--ds-focus-width`, `--ds-focus-offset` | Luna fluid outline family | re-derived fixed accessibility geometry |
| `--ds-size-target-min` | source tap spacing | re-derived fixed minimum target |
| `--ds-size-measure` | no source token | new readability bound |

The semantic rows above contain exactly 38 names: ten colors, three fonts, nine
text sizes, two leading roles, three weights, five spacing roles, and six
geometry roles. Together with the 29 references they cover every `tokens.tsv`
row. Deferred source-only shadow, filter, motion, display-type, icon, panel, and
brand families are not hidden: they remain consumer-owned or require a selected
module before a role can be added.

## Hook and theme comparison

| Luna pattern | E01 result | Decision |
|---|---|---|
| `data-component`, component identity roots | no equivalent | Reserved; selected modules already have a class root. |
| `data-slot`, `@slot` | no equivalent | Reserved; no selected module exposes internal slots. |
| `data-state`, `data-active`, `data-disabled` | native attributes/pseudo-classes and ARIA | Rejected as duplicate state. |
| `data-variant`, `data-ds-layout` | documented class hooks | Rejected because a parallel attribute API would double the compatibility surface. |
| `.dark`, `[data-theme]`, `[data-color-scheme]` | root `[data-ds-scheme]` | Alias hooks rejected. |
| broad descendant component selectors | hook element is the boundary | No `@scope`; only layouts reach direct children. |

The nine public hooks are `.ds-stack`, `.ds-cluster`, `.ds-grid`, `.ds-surface`,
`.ds-action`, `.ds-action-primary`, `.ds-action-quiet`, `.ds-action-icon`, and
root `[data-ds-scheme]`. State selectors are part of the action contract but are
not additional hooks.

## Native-platform boundary for T5

T4 records these dispositions; it does not implement them.

| Feature | Current E01 approach | Native candidate / support fact | Disposition |
|---|---|---|---|
| Fluid spacing/type | `clamp()` in selected reference tokens | Native below the E01 floor | Keep. It already is the native mechanism. |
| Light/dark roles | `light-dark()` in semantic colors | Native at the controlling E01 floor; `color-scheme` selects resolution | Keep. No duplicate light/dark token sets. |
| Container queries | Selected layouts are intrinsic and need no query | Size queries are supported below the E01 floor | Defer. Use only when a future selected module changes with its allocated container space. |
| Dialog | Classless native `<dialog>` surface | Native dialog is required at the floor | T5 may refine selected behavior; Luna viewport/panel synchronization remains consumer code unless native behavior replaces it completely. |
| Popover and anchor positioning | Surface-only `[popover]`; no shared positioning | Popover and anchor positioning are progressive at the floor | T5. Essential content needs a complete non-popover/non-anchor baseline. |
| Select | Classic native `<select>` with a target-size block size | Customizable `<select>` uses `appearance: base-select` and picker pseudo-elements; not interoperable at the floor | T5 progressive candidate. `<selectmenu>` is not a selected E01 API. |
| `:focus-visible` | One classless-base outline | Native below the E01 floor | Keep; primitives must not replace it. |
| `color-scheme` | Default on `:root`, explicit value on the root hook | Native below the E01 floor | Keep; required for native controls and `light-dark()` resolution. |

Other T5 candidates—native disclosure, progress/meter, `@property`,
`@starting-style`, discrete transitions, scroll snap, scroll-driven animation,
and scroll-state queries—remain governed by the accepted compatibility ledger.
None is silently adopted by this comparison.

## Accessibility, cascade, and override differences

- Native elements keep their native semantics and keyboard behavior. A styling
  hook never creates a role.
- The base owns the single `:focus-visible` outline; Luna component-specific
  outline rules do not migrate.
- Action disabled/current/unlinked behavior uses native or ARIA state. Custom
  business state and any script needed to suppress `aria-disabled` activation
  remain consumer-owned.
- E01 rules are zero-specificity and live in declared `ds.*` layers. Consumer
  classes and later layers override real properties without private descendants,
  `!important`, or Design System sub-layer knowledge.
- Logical properties and source order replace Luna page-specific physical
  placement and reordering. RTL and assistive-technology release evidence remain
  later assurance obligations where already recorded.
- T4 makes no new forced-colors, manual assistive-technology, or unlike-consumer
  claim. The existing carried findings retain their S5/DS-E02 owners.

## Completeness and handoff

No current comparison finding requires a CSS, hook, token, layout, or primitive
change. `layouts.tsv` and `primitives.tsv` remain unchanged. The bootstrap
checker admits `resources/` as an internal, classified, privacy-scanned evidence
root; it does not publish that directory or relax completeness. T5 may change
only an already selected contract through its own browser/fallback/a11y evidence
and must then refresh the affected rows here. T6 must rerun the same comparison
after any material stylesheet or toolchain refactor.

For S5 migration verification, the current `@luna/ds` end-state remains a later
release decision: retire, re-point, or time-bound parallel operation. This ledger
supplies the module/token/hook dispositions needed to verify that outcome; it does
not migrate Luna or declare a release artifact.
