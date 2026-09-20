# Tailwind adapter

Status: **public-preview adapter input**. This optional Tailwind v4 entrypoint
projects the portable Design System's public semantic roles; it is not a token
source and it does not change plain-CSS consumption.

Process `index.css` with the locked local provider. A consumer owns its source
detection boundary and supplies `@source` in its own input stylesheet. Do not
add private repository paths to `index.css`.

The adapter imports Tailwind's `theme` and `utilities` pieces only. It does not
import the aggregate `tailwindcss` stylesheet and therefore does not include
Preflight; `ds.base` remains the classless base authority. Load the processed
adapter before a consumer layer; the consumer layer can then override both DS
and Tailwind rules through normal layer precedence.

`projection.tsv` is the complete mapping ledger. Each projected Tailwind theme
variable aliases a `public-preview` `--ds-*` role through `@theme inline`; no
literal duplicate and no `--ds-ref-*` reference value is allowed. Generic
Tailwind utilities and defaults remain provider behavior, not Design System API.

## Utilities and variants

The shared adapter intentionally publishes no custom `@utility`, `@variant`,
or `@custom-variant` surface. Luna's scrollbar, sticky, state, pointer, and
theme shorthand candidates are either native/provider behavior, consumer-local,
or incompatible with the portable hook contract. Consumers may add local
utilities and variants after this entrypoint; those extensions are not Design
System API.
