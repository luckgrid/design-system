# Layouts fixture

Classification: **internal**.

This fixture exercises the layout primitives (`.ds-stack`, `.ds-cluster`,
`.ds-grid`) in a consumer page. It loads:

1. the declared `core.css` export at its public path, `/packages/styles/index.css`,
   which brings the tokens, the classless base, and the layouts;
2. its own `consumer.css`, which declares the consumer layer `app` after `ds`.

The page shell (`header`, `main`, `footer`, and the page measure) belongs to the
consumer, as the layouts contract requires. The layouts arrange content inside
it:

- a stack with long words and a long fragment URL;
- clusters of links and buttons, including a right-to-left cluster;
- grids with many items, one item, no items, long unbroken words, and a
  right-to-left grid;
- stacks and clusters nested inside a grid;
- a grid the consumer overrides with an ordinary class that sets `gap` and
  `grid-template-columns`.

It uses no framework, Tailwind, script, network fetch, or private path. Its
in-page links are same-document fragments. `ds-check layout` fails if the page
has no `main`, misses a promoted hook, uses an unpromoted `ds-` class, or puts
two layout hooks on one element. The browser suite checks source order,
keyboard order, reflow, overrides, and accessibility on it.
