# Tailwind v4 adapter

Status: **public-preview** optional adapter. The `tailwind.css` adapter input is
`adapters/tailwind/index.css`; process it with the exact provider in that
directory's lockfile. Plain `core.css` consumers do not install or execute it.

The adapter loads portable `core.css` first, then Tailwind's theme and utility
pieces in `tailwind.theme` and `tailwind.utilities`. It deliberately does not
import Tailwind's aggregate stylesheet or Preflight. `ds.base` remains the one
classless normalization authority. A consumer declares its own layer after the
processed adapter to override both systems without specificity escalation.

`projection.tsv` maps all 38 `public-preview` semantic `--ds-*` roles to
Tailwind theme variables using `@theme inline`. The inline form preserves live
semantic aliases, light/dark selection, and consumer-owned mappings. The 29
internal `--ds-ref-*` variables are explicitly excluded. Generic Tailwind
defaults and generated utilities remain provider behavior, not Design System
token or utility API.

The reusable adapter has no `@source` directive. Consumers own source detection
in their own input stylesheet; the checked-in fixture demonstrates a bounded
local `@source` only for verification.
