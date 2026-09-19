# Scoping fixture

Classification: **internal**.

This fixture checks the scoping boundary in `docs/architecture/hooks.md`: each public
hook styles only the element that carries it, and a layout reaches only its
direct children. It loads:

1. the declared `core.css` export at its public path, `/packages/styles/index.css`;
2. its own `consumer.css`, which declares the consumer layer `app` after `ds`.

The page shows:

- a navigation list of quiet link actions, one for each `aria-current` value:
  `page`, `true`, empty, `false`, `FALSE`, `False`, and the bare attribute;
- hooks nested inside each other: a stacked surface that holds a grid of
  surfaces, which hold a cluster of actions, and a stack whose grandchildren
  keep their own margins;
- lookalike markup that must stay unstyled: variant classes without their
  primitive, classes that contain `ds-action`, `data-*` attributes that name a
  component, a layout, or a slot, and an element whose id is `ds-action`;
- consumer overrides that select only consumer classes, including a class on a
  layout child that overrides the layout's reset.

It uses no framework, Tailwind, script, network fetch, or private path. Its
links are same-document fragments. `ds-check hooks` fails if the page has no
`main`, misses a class hook, uses a `ds-` class that is not a hook, or puts a
layout hook on a UI primitive. The browser suite checks computed styles,
reach, overrides, and accessibility on this page.
