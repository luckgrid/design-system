# Terminal UI patterns and cross-platform Design System research

**Status:** proposed research; not an accepted architecture decision or consumer API.
**Opened:** 2026-11-18. **Evidence baseline:** Design System `main` at `09199f763ffebb9258717310c45312b5c74b4103`.
**Owner:** Design System. **Consumers for validation:** Prompt-TUI, wiki-reader, Workbench/Workstation.
**Scope:** design language and reusable interface contracts, not application behavior, harness execution, or a new frontend runtime.

## Why this exists

The current Design System is web-platform-first: semantic HTML, CSS-first styling, the published `core.css` preview, reference and semantic tokens, themes, classless base, layout hooks, surface/action primitives, and reproducible release verification. Emerging Rust terminal applications need consistent focus, navigation, density, feedback, controls, and visual semantics even though terminal rendering does not use browser CSS. The goal is to **share meaning and patterns, not a DOM or a universal renderer**.

The questions are (1) what reusable contracts belong in Design System, (2) which token semantics transfer to terminal cells, (3) whether Ratatui alone, third-party component systems or CSS-like styling add enough value, and (4) what consumer evidence is needed before an implementation or public API.

## Read in this order

1. [Architecture and boundaries](architecture.md): proposed division between platform-neutral design language and renderer-owned behavior.
2. [Token and theme projection](tokens-and-themes.md): explicit CSS-to-terminal mapping, fallbacks, non-transferable roles, brand themes, capability matrix.
3. [Patterns and components](patterns-and-components.md): candidate taxonomy and concrete interaction contracts.
4. [Technology evaluation](technology-evaluation.md): Ratatui, Crossterm, ratcn, CSS-like engines, cross-renderer references and GPUI.
5. [Verification and roadmap](verification-and-roadmap.md): staged prototype, measurements, acceptance, risks and non-goals.
6. [Decision register](decisions.md): proposal-level decisions, alternatives, questions and evidence needed to promote them.
7. [Source ledger](sources.md): inspected evidence, direct project links, limits and reassessment triggers.
8. [Discovery record](discovery-2026-11-18.md): session provenance and handoff context.
9. [Review record](review-2026-11-20.md): the pre-merge review pass, its corrections and rechecked versions.

## Existing authorities that must not be silently changed

- [Design tokens](https://github.com/luckgrid/design-system/blob/main/docs/architecture/tokens.md), [theme](https://github.com/luckgrid/design-system/blob/main/docs/architecture/theme.md), [primitives](https://github.com/luckgrid/design-system/blob/main/docs/architecture/primitives.md) and [layouts](https://github.com/luckgrid/design-system/blob/main/docs/architecture/layouts.md) define the current **web** contracts.
- `tokens.tsv` describes public-preview semantic CSS roles and internal reference roles. `exports.tsv` declares the CSS export; `bootstrap-surfaces.tsv` classifies tracked files; `release/inventory.tsv` governs shipped artifacts.
- The Design System explicitly has no promoted general-purpose component layer yet. A terminal pattern catalog does not retroactively promote web components.
- These research files are **internal documentation**. They are deliberately absent from the immutable `0.1.0-preview.1` CSS release inventory and are not a Rust package, schema, hook, or public-preview surface.
- The planned interfaces are **examples**, not normative Rust traits or stable wire formats.
- Links to Prompt-TUI's plans in `luckgrid/wiki-nest` and to `luckgrid/workstation` point to Luckgrid repositories that are not public yet, so readers without access get a 404. These pages restate what the proposal takes from them; the links are for maintainers with access.

## Scope and ownership

| Owner | Owns | Does not own |
| --- | --- | --- |
| Design System | semantic interaction contracts, pattern taxonomy, projection guidance, optional future Ratatui adapter | app content, persistence, model selection, harness gateway, application state machine |
| Prompt-TUI | prompt brief, editable fields, output slot selection/export, narrow-pane composition | common theme authority or general harness gateway |
| Workbench / Workstation | terminal/worktree orchestration, harness handoffs, terminal tooling and environment policies | cross-product design contracts |
| wiki-reader | its own navigation, content and editor/view state; candidate consumer tests | default Design System API solely because it uses Ratatui |

## Change discipline

Research -> measured prototype -> two materially different consumers -> reviewed compatibility boundary -> optional packaged preview. Do not expand `core.css`, add a general CSS parser, publish a crate, or claim accessibility conformance from this proposal alone. Keep product-specific layouts consumer-owned.

## Status of the proposal

No runtime, dependency or styling change is authorized by these documents. Every choice in [decisions.md](decisions.md) remains proposed until owner acceptance and consumer evidence.

