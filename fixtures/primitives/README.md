# Primitives fixture

Classification: **internal**.

This fixture exercises the primitives in a consumer page: the `.ds-surface` base
primitive and the `.ds-action` UI primitive with its `.ds-action-primary`,
`.ds-action-quiet`, and `.ds-action-icon` variants. It loads:

1. the declared `core.css` export at its public path, `/packages/styles/index.css`,
   which brings the tokens, the classless base, the layouts, and the primitives;
2. its own `consumer.css`, which declares the consumer layer `app` after `ds`.

The page shell (`header`, `main`, `footer`, and the page measure) belongs to the
consumer. The page shows:

- a surface composed with a stack, surfaces in a grid, a surface with no layout,
  and a right-to-left surface composed with a cluster;
- every action variant on a `<button>`, and the default and primary actions on
  a link;
- a primary action that submits a form;
- a navigation list of quiet link actions, one with `aria-current="page"`;
- the disabled, unlinked (an `<a>` with no `href`), current, and not-current
  states;
- an action and a surface that the consumer overrides with ordinary classes.

It uses no framework, Tailwind, script, network fetch, or private path. Its
links are same-document fragments. `ds-check primitive` fails if the page has
no `main`, or uses an unpromoted `ds-` class. It also fails if the page:

- misses a promoted hook or a markup-expressible state;
- puts a variant without its primitive, or an action on anything but `<button>`
  or `<a>`;
- gives a primitive a `data-*` or `role` attribute;
- never composes the surface with a layout.

The browser suite checks computed styles, states, activation, keyboard, target
size, overrides, and accessibility on this page.
