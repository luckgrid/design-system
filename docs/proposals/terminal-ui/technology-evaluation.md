# Renderer, component and CSS-like styling research

**Checked:** 2026-11-18, public upstream descriptions and source-linked Luckgrid plans. **Purpose:** decision support, **not** an endorsement, installed dependency list or claim of benchmarked suitability.

## Decision criteria

Fit for existing Rust terminal consumers; composability with Ratatui; ownership of focus/events/app state; theme projection and CSS semantics; dependency/build/MSRV footprint; project maturity and release stability; licensing; capability/accessibility limits; maintenance cost; testability; packaging and opt-in scope. Popularity alone is not a score. Inspect **current** versions and compatible backend pairings again before adding dependencies.

## Baseline choices

| Technology | Verified characteristics / relevance | Tradeoff and next test | Proposed position |
| --- | --- | --- | --- |
| [Ratatui](https://ratatui.rs/) and [Style API](https://docs.rs/ratatui/latest/ratatui/style/struct.Style.html) | Rust terminal widgets, cell layout, `Color` / `Style` / modifiers, rendering backend and buffer tests | App must own semantic focus, input routing, state, accessible alternative output | Preferred low-level first proof |
| [Crossterm](https://docs.rs/crossterm/) through [Ratatui backend](https://ratatui.rs/concepts/backends/) | Input, mouse/resize and terminal setup/teardown; existing Prompt-TUI plan prefers matching re-export. Since 0.30 each backend is its own crate (`ratatui-crossterm`, `-termion`, `-termwiz`, and `-termina` from 0.30.2), and `crossterm_0_28` / `crossterm_0_29` features pick the Crossterm version (0.29 by default), re-exported as `ratatui_crossterm::crossterm` | Incompatible duplicate versions can complicate terminal raw mode/events; use the re-export and the version feature rather than a second direct Crossterm dependency, and confirm the exact dependency tree | Pair with Ratatui and test lifecycle |
| [ratatui-textarea](https://docs.rs/ratatui-textarea/) | Multi-line editing and cursor/selection/scroll support; planned Prompt-TUI candidate. The editor's lineage is split: rhysd's original `tui-textarea` stopped at Ratatui 0.29, `tui-textarea-2` is one maintained fork, and `ratatui-textarea` 0.9.x is the Ratatui-native fork with undo/redo and soft wrapping. An unrelated older crate of the same name was deprecated, so check the owner and version, not just the name | Needs integration tests for Unicode, wrapping, undo and output model; confirm which fork Prompt-TUI's plan names before a trial | Try direct before building editor |
| [ratcn](https://ratcn.com/docs/introduction) / [ecosystem entry](https://ratatui.rs/ecosystem/ratcn/) | Themeable Ratatui components, split paint-only widgets and optional focus, hover, keyboard/mouse routing. Pre-1.0 (`0.0.x`); upstream states the API will break and recommends pinning an exact version. Distribution is source-copy first (`cargo ratcn add` copies component modules into the consumer). TextArea was withdrawn in an early preview and has since returned with documented limits (e.g. no undo/redo) | Source-copy means each consumer owns upgrades of copied modules; check the current component list and TextArea limits before any trial. Its optional terminal session asks the terminal for its foreground and background and solves an adaptive theme from the pair, which is prior art for the unknown-background problem in [token projection](tokens-and-themes.md#capabilities-and-fallback) | Evaluate on second interactive control |
| [tui-realm / tuirealm](https://github.com/veeso/tui-realm) | Message-oriented component/event model for a larger TUI | Larger application architecture commitment | Defer unless focus routing grows complex |
| [Cursive](https://docs.rs/cursive/) | Alternative view/callback-based forms and lifecycle | Separate UI framework, cannot assume Ratatui widget reuse | Comparative experiment only |
| [Bare Crossterm](https://docs.rs/crossterm/) | Lightweight for a tiny picker | Must build layout/selection/redraw and state ourselves | Avoid for multipane app |

The [Prompt-TUI Rust CLI/TUI review](https://github.com/luckgrid/wiki-nest/blob/main/prompt-tui/records/2026-11-14-rust-cli-review.md) already ranks clap derive + Rust standard I/O for CLI and Ratatui + Crossterm for its editor. Reuse that decision **as consumer evidence**, not a mandate that Design System implement command parsing or harness lifecycle.

## CSS-like engines: proof of possibility, not CSS compatibility

**[ratatui-style](https://docs.rs/ratatui-style/latest/ratatui_style/):** CSS-inspired stylesheet API, selectors, cascade and custom variables that computes Ratatui styling. Useful for exploring declarative style classes. Its documented selector support is a pragmatic subset (compound `Type.class#id:pseudo` selectors, comma lists and `*`), so descendant/child combinators should not be assumed. Test exact API and parsing behavior; a terminal `Style` does not implement CSS layout, HTML intrinsic semantics, viewport-fluid sizing, `light-dark()`, arbitrary cascade layers or browser computed styles. Do not promise it accepts `core.css`.

**[rdom-css / rdom-tui](https://docs.rs/rdom-css/latest/rdom_css/):** another experimental cross-renderer DOM/CSS subset direction; the parser/cascade can be studied, but adopting a DOM layer is a separate architectural commitment.

**[Revue](https://github.com/hawk90/revue):** a Vue-style Rust TUI *framework* (retained mode, signals, CSS stylesheets with variables and selectors, an accessibility module). It is the most complete "CSS for terminals" data point, but it replaces Ratatui rather than composing with it, so it is a comparison reference for the CSS-like question, not a candidate dependency for existing Ratatui consumers.

**[`ratatui-css`](https://docs.rs/ratatui-css/) (name reservation):** a crate name reserved under the Ratatui project for future CSS-inspired styling experiments. It ships no implementation today. Treat any real release as a reassessment trigger for the CSS-like engine question, because upstream support would change the maintenance calculus.

**[Opaline](https://github.com/hyperb1iss/opaline):** token-based Rust theme engine (39 built-in themes in 0.4.1) with adapters for Ratatui, egui, iced, Crossterm and CSS among others. Its `css` feature generates CSS from the same tokens, so it is also a working example of [Alternative A](tokens-and-themes.md#projection-pipeline-hypotheses), one token source feeding both a web and a terminal target. Strong reference for role mapping and theme loading; *does not* establish that existing Design System CSS semantics flow through automatically.

**[gronke/ui-components](https://github.com/gronke/ui-components):** research example defining components in Rust with terminal/browser projection. Relevant to a future unified behavioral API, but it also has an explicit retained DOM/CSS implementation model; not justification to replace the present CSS-first web implementation.

**Alternative evaluation:** benchmark a *tiny* typed role palette against a stylesheet-powered button/list prototype. Compare actual code required, startup time, binary impact, cascading/state behavior, accessibility cues, browser/CSS token drift, debugging, and license/dependency surfaces. Reject any “less code” conclusion without an end-to-end interaction.

## Zed references: GPUI, Zed's own UI crate, and look-alikes

"Zed UI" most usefully means **Zed's own [`ui` crate](https://github.com/zed-industries/zed/tree/main/crates/ui)**, whose crate documentation describes it as the UI primitives and components used to build all of Zed's interface. It is built on GPUI, so it is the primary reference for how a production native Rust application layers a component vocabulary (buttons, lists, labels, key-binding hints, selectable/disableable/clickable traits) over a GPU renderer. Study its traits and component boundaries alongside the matching GPUI concepts below; do not treat it as a reusable dependency, because it is an internal crate of the Zed editor.

**[GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui):** the Rust, GPU-backed interface framework developed for Zed. It exposes styled elements and action/key dispatch, with CSS-inspired fluent API, but is not a standards-compliant browser CSS runtime or a TUI backend. Use it to study declarative composition, style calculation, component boundaries, input routing and entity/app-state ownership. The [GPUI examples](https://github.com/zed-industries/zed/tree/main/crates/gpui/examples) and [key dispatch](https://github.com/zed-industries/zed/blob/main/crates/gpui/docs/key_dispatch.md) are more probative than broad marketing comparisons.

**[gpui-component](https://github.com/longbridge/gpui-component) (Longbridge):** a third-party GPUI component library (60+ components, theming) that powers a shipped commercial desktop app, Longbridge Pro. Its documented layering separates GPUI (rendering), `gpui-base` (unstyled behavior and state, analogous to Base UI) and `gpui-component` (styled layer, analogous to shadcn). Since 2026 the layers ship together as [GPUI Kit](https://docs.rs/gpui-kit/): one `gpui-kit` crate pins the matching GPUI release and re-exports both, and `gpui-base` 0.6.0 (2026-09-03) is a published crate an application can depend on to build its own design system. That headless/styled split is direct evidence for open question 2 in the [decision register](decisions.md) about the lowest stable pattern unit, and the most relevant GUI-side reference if a native desktop consumer enters scope.

**[awesome-gpui](https://github.com/zed-industries/awesome-gpui):** curated ecosystem directory for native GPUI projects, not a framework or direct terminal UI component source. Search it for concrete adoption examples and maintenance concerns only if a native desktop consumer enters scope.

**Name collisions:** several unrelated web projects use the name "Zed UI". One example is [zedcraftstudio/zed-ui](https://github.com/zedcraftstudio/zed-ui), a small early-stage React component library (CSS-variable tokens via a `ThemeProvider`) with no stated affiliation with Zed Industries. It is at most a minor web-component reference; do not cite it as "Zed UI" or attribute it to the Zed editor team.

A future native desktop implementation may share role names and pattern specs; it should not be promised a single codebase with Ratatui, nor should “React-like” be treated as evidence that a terminal virtual DOM is appropriate.

## Other pointers and limitations

- [Legacy tui-rs](https://github.com/fdehau/tui-rs) and [old tui docs](https://docs.rs/tui/latest/tui/) are legacy background; new proposals should target maintained Ratatui.
- [awesome-ratatui](https://github.com/ratatui/awesome-ratatui) is a discovery catalog, not independent performance/quality evidence.
- [ratcn component docs](https://ratcn.com/docs/components/textarea) and [custom component design](https://ratcn.com/docs/concepts/custom-components) can guide focused component trials; source-copy and dependency installation have different upgrade obligations.
- [NO_COLOR](https://no-color.org/), terminal capability differences and evolving [Ratatui accessibility discussions](https://github.com/ratatui/ratatui/issues/2610) reinforce that accessibility claims require real evidence. In that discussion, maintainers note that terminals do not expose their content through accessibility APIs, so screen-reader support needs an out-of-band channel. Current experiments take that route: [tui-a11y](https://docs.rs/tui-a11y/) serves an app-declared tree over AT-SPI via AccessKit (Linux only; Ratatui is a dev-dependency; not re-located in the 2026-11-20 recheck), and FrankenTUI's [ftui-a11y](https://docs.rs/ftui-a11y/) defines an accessibility tree for a separate, non-Ratatui TUI kernel, with the bridge to AccessKit and platform APIs still a planned later phase. A plain CLI interface is an important fallback, not proof of full AT support.
- Terminal rendering can be fast, but **no benchmarks** of these libraries have been conducted for this proposal; do not rank by guessed startup, redraw or binary size.

## Preliminary ranking by responsibility

- **Renderer:** Ratatui (first choice), Cursive (alternative study); GPUI is *native GUI only*.
- **Input/lifecycle:** matching Crossterm backend, app-owned event policy.
- **Editing:** direct textarea trial, ratcn if interaction-control reuse is demonstrated.
- **Theme:** typed palette first; Opaline design reference; ratatui-style CSS-like experiment.
- **Cross-renderer architecture:** docs/contracts first; gronke/GPUI, Zed's `ui` crate and gpui-component research only.
- **CLI commands and exports:** consumer-owned; not a Design System component concern.

## Versions observed at this research cut

Versions are what public registries and project docs showed on 2026-11-18, rechecked on 2026-11-20 ([review](review-2026-11-20.md)), and are recorded so a later session can detect drift. They are **not** pins or approved dependencies; re-check before any trial.

| Project | 2026-11-18 | 2026-11-20 recheck | Note |
| --- | --- | --- | --- |
| ratatui | 0.30.0 | **0.30.2** | 0.30.1 (2026-06-05) added block shadows; 0.30.2 added the Termina backend and fixed 0.30.1 regressions |
| ratatui-crossterm / crossterm | not recorded | crossterm 0.29 by default, 0.28 selectable | version picked by feature flag |
| ratatui-textarea | not recorded | 0.9.4 (2026-10-31) | Ratatui-native fork of tui-textarea; see the baseline table |
| ratcn | 0.0.3 (release) | 0.0.3 (2026-09-07) | pre-1.0, API breaks expected, source-copy distribution |
| ratatui-style | 0.2.0 | 0.2.0 | CSS cascade subset |
| ratatui-css | 0.0.0 | 0.0.0 | name reservation only |
| opaline | 0.4.1 | 0.4.1 | token theme engine, multi-adapter, CSS output |
| rdom-css | 0.6.1 | not rechecked | parser for rdom-tui |
| revue | 2.74.0 | not rechecked | separate framework, comparison only |
| tui-a11y | 0.1.0 | not re-located | experimental, Linux AT-SPI; confirm it still exists before citing it |
| ftui-a11y | not recorded | 0.4.x | accessibility tree for FrankenTUI; platform bridge still planned |
| gpui-component | 0.4.x on crates.io (git main is current) | **gpui-kit 0.6.x**, `gpui-base` 0.6.0 | layers now released together as GPUI Kit; GPUI-native, GUI only |

tui-realm and Cursive versions were not recorded.

See [source ledger](sources.md) for evidence scope and [verification plan](verification-and-roadmap.md) for next experiments.

