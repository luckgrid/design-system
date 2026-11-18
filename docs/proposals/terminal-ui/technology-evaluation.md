# Renderer, component and CSS-like styling research

**Checked:** 2026-11-18, public upstream descriptions and source-linked Luckgrid plans. **Purpose:** decision support, **not** an endorsement, installed dependency list or claim of benchmarked suitability.

## Decision criteria

Fit for existing Rust terminal consumers; composability with Ratatui; ownership of focus/events/app state; theme projection and CSS semantics; dependency/build/MSRV footprint; project maturity and release stability; licensing; capability/accessibility limits; maintenance cost; testability; packaging and opt-in scope. Popularity alone is not a score. Inspect **current** versions and compatible backend pairings again before adding dependencies.

## Baseline choices

| Technology | Verified characteristics / relevance | Tradeoff and next test | Proposed position |
| --- | --- | --- | --- |
| [Ratatui](https://ratatui.rs/) and [Style API](https://docs.rs/ratatui/latest/ratatui/style/struct.Style.html) | Rust terminal widgets, cell layout, `Color` / `Style` / modifiers, rendering backend and buffer tests | App must own semantic focus, input routing, state, accessible alternative output | Preferred low-level first proof |
| [Crossterm](https://docs.rs/crossterm/) through [Ratatui backend](https://ratatui.rs/concepts/backends/) | Input, mouse/resize and terminal setup/teardown; existing Prompt-TUI plan prefers matching re-export | Incompatible duplicate versions can complicate terminal raw mode/events; confirm exact dependency tree | Pair with Ratatui and test lifecycle |
| [ratatui-textarea](https://docs.rs/ratatui-textarea/) | Multi-line editing and cursor/selection/scroll support; planned Prompt-TUI candidate | Needs integration tests for Unicode, wrapping, undo and output model | Try direct before building editor |
| [ratcn](https://ratcn.com/docs/introduction) / [ecosystem entry](https://ratatui.rs/ecosystem/ratcn/) | Themeable Ratatui components, split paint-only widgets and optional focus, hover, keyboard/mouse routing | Dependency vs source-copy maintenance, implementation-model coupling, version pinning | Evaluate on second interactive control |
| [tui-realm / tuirealm](https://github.com/veeso/tui-realm) | Message-oriented component/event model for a larger TUI | Larger application architecture commitment | Defer unless focus routing grows complex |
| [Cursive](https://docs.rs/cursive/) | Alternative view/callback-based forms and lifecycle | Separate UI framework, cannot assume Ratatui widget reuse | Comparative experiment only |
| [Bare Crossterm](https://docs.rs/crossterm/) | Lightweight for a tiny picker | Must build layout/selection/redraw and state ourselves | Avoid for multipane app |

The [Prompt-TUI Rust CLI/TUI review](https://github.com/luckgrid/wiki-nest/blob/main/prompt-tui/records/2026-11-14-rust-cli-review.md) already ranks clap derive + Rust standard I/O for CLI and Ratatui + Crossterm for its editor. Reuse that decision **as consumer evidence**, not a mandate that Design System implement command parsing or harness lifecycle.

## CSS-like engines: proof of possibility, not CSS compatibility

**[ratatui-style](https://docs.rs/ratatui-style/latest/ratatui_style/):** CSS-inspired stylesheet API, selectors, cascade and custom variables that computes Ratatui styling. Useful for exploring declarative style classes. Test exact API and parsing behavior; a terminal `Style` does not implement CSS layout, HTML intrinsic semantics, viewport-fluid sizing, `light-dark()`, arbitrary cascade layers or browser computed styles. Some selector operations may need tree context absent in one-shot matching. Do not promise it accepts `core.css`.

**[rdom-css / rdom-tui](https://docs.rs/rdom-css/latest/rdom_css/):** another experimental cross-renderer DOM/CSS subset direction; the parser/cascade can be studied, but adopting a DOM layer is a separate architectural commitment.

**[Opaline](https://github.com/hyperb1iss/opaline):** token-based Rust theme engine with adapters for Ratatui, egui, iced, Crossterm and CSS among others. Strong reference for role mapping and theme loading; *does not* establish that existing Design System CSS semantics flow through automatically.

**[gronke/ui-components](https://github.com/gronke/ui-components):** research example defining components in Rust with terminal/browser projection. Relevant to a future unified behavioral API, but it also has an explicit retained DOM/CSS implementation model; not justification to replace the present CSS-first web implementation.

**Alternative evaluation:** benchmark a *tiny* typed role palette against a stylesheet-powered button/list prototype. Compare actual code required, startup time, binary impact, cascading/state behavior, accessibility cues, browser/CSS token drift, debugging, and license/dependency surfaces. Reject any “less code” conclusion without an end-to-end interaction.

## Zed references: two different systems

**[zedcraftstudio/zed-ui](https://github.com/zedcraftstudio/zed-ui):** public README describes **React** components with CSS variables and a `ThemeProvider`, drawing on Base UI for complex accessible interactions. It is a source of component API, design token and accessibility-testing patterns, **not** the native UI renderer of Zed Editor. Do not describe it as made by the Zed Editor team absent corporate/project evidence. Its docs acknowledge early `0.0.x` stability and limited coverage; avoid assumptions about production maturity.

**[GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui):** the Rust, GPU-backed interface framework developed for Zed. It exposes styled elements and action/key dispatch, with CSS-inspired fluent API, but is not a standards-compliant browser CSS runtime or a TUI backend. Use it to study declarative composition, style calculation, component boundaries, input routing and entity/app-state ownership. The [GPUI examples](https://github.com/zed-industries/zed/tree/main/crates/gpui/examples) and [key dispatch](https://github.com/zed-industries/zed/blob/main/crates/gpui/docs/key_dispatch.md) are more probative than broad marketing comparisons.

**[awesome-gpui](https://github.com/zed-industries/awesome-gpui):** curated ecosystem directory for native GPUI projects, not a framework or direct terminal UI component source. Search it for concrete adoption examples and maintenance concerns only if a native desktop consumer enters scope.

A future native desktop implementation may share role names and pattern specs; it should not be promised a single codebase with Ratatui, nor should “React-like” be treated as evidence that a terminal virtual DOM is appropriate.

## Other pointers and limitations

- [Legacy tui-rs](https://github.com/fdehau/tui-rs) and [old tui docs](https://docs.rs/tui/latest/tui/) are legacy background; new proposals should target maintained Ratatui.
- [awesome-ratatui](https://github.com/ratatui/awesome-ratatui) is a discovery catalog, not independent performance/quality evidence.
- [ratcn component docs](https://ratcn.com/docs/components/textarea) and [custom component design](https://ratcn.com/docs/concepts/custom-components) can guide focused component trials; source-copy and dependency installation have different upgrade obligations.
- [NO_COLOR](https://no-color.org/), terminal capability differences and evolving [Ratatui accessibility discussions](https://github.com/ratatui/ratatui/issues/2610) reinforce that accessibility claims require real evidence. A plain CLI interface is an important fallback, not proof of full AT support.
- Terminal rendering can be fast, but **no benchmarks** of these libraries have been conducted for this proposal; do not rank by guessed startup, redraw or binary size.

## Preliminary ranking by responsibility

- **Renderer:** Ratatui (first choice), Cursive (alternative study); GPUI is *native GUI only*.
- **Input/lifecycle:** matching Crossterm backend, app-owned event policy.
- **Editing:** direct textarea trial, ratcn if interaction-control reuse is demonstrated.
- **Theme:** typed palette first; Opaline design reference; ratatui-style CSS-like experiment.
- **Cross-renderer architecture:** docs/contracts first; gronke/GPUI research only.
- **CLI commands and exports:** consumer-owned; not a Design System component concern.

See [source ledger](sources.md) for evidence scope and [verification plan](verification-and-roadmap.md) for next experiments.

