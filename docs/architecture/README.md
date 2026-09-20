# Architecture

The Design System ships plain CSS with an explicit cascade contract. Read these
pages in order when integrating it:

1. [CSS entrypoint and cascade](css-entrypoint.md) — load `core.css`, declare a
   consumer layer, and override zero-specificity defaults.
2. [Tokens](tokens.md) — use or remap public semantic roles without depending on
   internal reference values.
3. [Theme](theme.md) — follow the user preference or select light/dark on the
   document root.
4. [Classless base](base.md) — understand which semantic HTML elements receive
   defaults and which remain consumer-owned.
5. [Layouts](layouts.md) — opt into stack, cluster, and intrinsic grid.
6. [Primitives](primitives.md) — compose surfaces and native actions.
7. [Hooks and scoping](hooks.md) — use the complete public hook vocabulary and
   override it safely.
8. [CSS authoring and processing toolchain](css-toolchain.md) — understand
   source authority, deterministic audit measurements, and the bounded
   low-level processing evaluation.

[Bootstrap architecture](bootstrap.md) describes repository classification and
the checks that keep internal files out of the public contract.
