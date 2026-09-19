# Migration comparison

The portable core is a deliberate reduction of Luna and Luckgrid.net evidence,
not a renamed copy. The complete, revision-pinned ledger is
`resources/evidence/e01-s3-t4-migration-comparison.md`.

## Selected outcome

- Layouts are stack, cluster, and an intrinsic collection grid. Page shells,
  12-column grids, named areas, sidebars, sticky offsets, and visual reordering
  remain consumer composition.
- Primitives are surface and action. Card is surface-plus-layout composition;
  navigation, breadcrumbs, hero, brand, and icon systems remain consumer-owned.
- State is native or ARIA. No `data-state`, `data-active`, `data-disabled`,
  `data-variant`, component identity, or slot API is published.
- The public vocabulary remains nine hooks: eight classes and the root
  `data-ds-scheme` attribute. The hook is the scope root; no `@scope` dependency
  is introduced.
- The token authority consolidates source values into 29 internal references and
  38 public semantic roles. Brand/provider namespaces and unselected component
  families do not migrate.

Every difference is intentional: portable semantics, bounded selector reach,
zero-specificity overrides, explicit layers, native state, and role-based tokens
take precedence over file/name/selector or pixel parity.

## T4 / T5 boundary

T4 changes no runtime contract. It keeps `clamp()`, `light-dark()`,
`:focus-visible`, and `color-scheme`; the current layouts stay intrinsic without
container queries. T5 owns any refinement of native disclosure, dialog, Popover,
anchor positioning, customizable select, progress/meter, registered properties,
transitions, or scroll behavior. A progressive feature must retain a complete,
operable baseline at the accepted browser floor.

Customizable select means enhancement of native `<select>` with
`appearance: base-select` and picker pseudo-elements. `<selectmenu>` is not an
E01 public API. Popover and anchor positioning remain progressive rather than
baseline requirements.

## Change rule

Adding a component merely to reduce a migration difference is forbidden. A later
task that changes an accepted contract must refresh the affected comparison,
browser, accessibility, and consumer evidence at its exact reviewed head.
