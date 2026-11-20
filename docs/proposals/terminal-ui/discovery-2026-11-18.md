# Discovery record — cross-platform TUI patterns

**Date:** 2026-11-18. **Status:** recorded; design decisions remain proposed. **Origin:** session discussing Workstation terminal tools, Prompt-TUI seed, Ratatui-style theme projection, Zed UI and Zed's GPUI.

## User intent and central reasoning

The Design System already provides semantic HTML and CSS primitives, while Workstation/Workbench and Prompt-TUI are growing a family of terminal interfaces. The owner suggested that interaction patterns and component conventions could be more valuable than CSS itself in TUIs, and asked whether existing style tokens can be reused with Ratatui. Zed UI and GPUI were proposed as research references for a declarative/CSS-inspired design language.

The resulting recommendation is to extend the Design System **conceptually** with terminal semantics and reusable patterns, while leaving the CSS implementation intact. Prefer documentation plus a small typed token/theme experiment over building a new renderer. Ratatui and its ecosystem remain likely implementation substrate, but no library decision for the Design System has been accepted.

## Findings to preserve

1. Current Design System has preview CSS semantic tokens, light/dark aliasing, base/primitives/layouts and a comprehensive source/release boundary. It **does not** yet publish general web components or a Rust TUI API.
2. Terminal interfaces can reuse the *intent* of color/foreground/focus/state roles, not browser units, native semantics or arbitrary CSS declarations.
3. Terminal input, selection, hover, modal behavior, resized-pane navigation and no-color/Unicode/AT fallbacks require independent design/test contracts.
4. Prompt-TUI's narrow one-pane editing, property focus, explicit copy scope, accurate export and status are strong source examples; avoid promoting its application-specific content model as generic UI.
5. Ratatui/Crossterm can provide rendering/events; ratcn, ratatui-textarea, tui-realm, Opaline and CSS-style engines deserve constrained evaluations. None was benchmarked for this proposal.
6. "Zed UI" is best read as **Zed's own `ui` crate** (the primitives and components Zed builds its interface from, on top of **GPUI**, Zed Industries' native Rust GPU framework). The similarly named **zedcraftstudio/zed-ui** is an unrelated small React/CSS-variable library, **longbridge/gpui-component** is a separate third-party GPUI component library, and **awesome-gpui** is a directory. (Corrected during review: the first draft treated zedcraftstudio/zed-ui as the Zed UI reference.) Avoid claims of shared authorship without evidence.
7. One architecture might support web, terminal and future native products through semantic contracts, but one universal rendering runtime is not justified. Shared behavior should be verified with two materially unlike consumers.
8. Keep research documents internal/source-only until API, packaging, accessibility and owner-acceptance requirements are satisfied.

## Record relationships

- [Overview](README.md), [architecture](architecture.md), [token projection](tokens-and-themes.md), [patterns](patterns-and-components.md), [technologies](technology-evaluation.md), [decision register](decisions.md), [roadmap](verification-and-roadmap.md), [sources](sources.md), [pre-merge review](review-2026-11-20.md).
- External project planning remains in [Prompt-TUI](https://github.com/luckgrid/wiki-nest/tree/main/prompt-tui) and [Workstation](https://github.com/luckgrid/workstation); this record is not a migration of those projects.

## Suggested next-session kickoff

“Read Design System's `docs/proposals/terminal-ui/` research suite and current web contract before implementation. Verify upstream library versions, then prototype one typed semantic color mapping and one shared focusable-list or pane-navigation pattern using Ratatui. Compare native Ratatui against one optional component engine with real keyboard, resize, Unicode, no-color and cancellation tests. Treat every decision as proposed; preserve CSS exports, inventory and consumer ownership. Report evidence, tradeoffs and a recommendation in a separate PR.”

## What was not done

No terminal adapter was compiled, no theme engine evaluated in a running application, no tests of alternate terminals or screen readers were executed, and no release/public-preview change was authorized.

