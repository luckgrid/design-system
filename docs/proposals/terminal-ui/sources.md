# Evidence and source ledger

**Research cut:** 2026-11-18. Links below distinguish *observed existing contracts*, *third-party documented capabilities*, *catalog/directory pointers* and *unverified proposals*. A link is not proof of performance, maturity, compatibility, licensing fit, or current availability at implementation time.

## Luckgrid — repository-local authorities

| Subject | Source and what it actually establishes |
| --- | --- |
| Current Design System product boundary | [README](https://github.com/luckgrid/design-system/blob/09199f763ffebb9258717310c45312b5c74b4103/README.md): web-platform-first, CSS export, Rust checks, no general components yet |
| CSS token semantics and alias details | [token architecture](https://github.com/luckgrid/design-system/blob/09199f763ffebb9258717310c45312b5c74b4103/docs/architecture/tokens.md), [tokens.tsv](https://github.com/luckgrid/design-system/blob/09199f763ffebb9258717310c45312b5c74b4103/tokens.tsv): reference/internal vs semantic/public-preview, `light-dark()` and role types |
| Browser theme | [theme doc](https://github.com/luckgrid/design-system/blob/09199f763ffebb9258717310c45312b5c74b4103/docs/architecture/theme.md): root `data-ds-scheme`, consumer ownership and override precedence |
| Web primitives and taxonomy | [primitives](https://github.com/luckgrid/design-system/blob/09199f763ffebb9258717310c45312b5c74b4103/docs/architecture/primitives.md), [layouts](https://github.com/luckgrid/design-system/blob/09199f763ffebb9258717310c45312b5c74b4103/docs/architecture/layouts.md): surface/action, stack/cluster/grid and native states |
| Source boundary | [bootstrap](https://github.com/luckgrid/design-system/blob/09199f763ffebb9258717310c45312b5c74b4103/docs/architecture/bootstrap.md), [inventory](https://github.com/luckgrid/design-system/blob/09199f763ffebb9258717310c45312b5c74b4103/bootstrap-surfaces.tsv): all tracked files classified and privacy-scanned |
| Existing role projection precedent | [Tailwind projection table](https://github.com/luckgrid/design-system/blob/09199f763ffebb9258717310c45312b5c74b4103/adapters/tailwind/projection.tsv) and [Tailwind adapter doc](https://github.com/luckgrid/design-system/blob/09199f763ffebb9258717310c45312b5c74b4103/docs/architecture/tailwind.md): 38 public roles projected, 29 reference roles excluded, verified by `ds-check tailwind`; the model for a terminal role table |
| Published artifact boundary | [release inventory](https://github.com/luckgrid/design-system/blob/09199f763ffebb9258717310c45312b5c74b4103/release/inventory.tsv) and [release docs](https://github.com/luckgrid/design-system/blob/09199f763ffebb9258717310c45312b5c74b4103/docs/release.md): authored CSS archive, not a registry crate |
| Prompt-TUI current interaction design | [composition interface](https://github.com/luckgrid/wiki-nest/blob/main/prompt-tui/architecture/interface.md): regions, narrow-pane constraints, copy labels, keyboard, fallback and editor lifecycle; its domain policies are not generic component requirements |
| Prompt-TUI product and roadmap | [product foundation](https://github.com/luckgrid/wiki-nest/blob/main/prompt-tui/foundation.md), [program](https://github.com/luckgrid/wiki-nest/blob/main/prompt-tui/program.md): brief/variant/slot model and staged product plan |
| Existing Rust terminal technology assessment | [Prompt-TUI Rust review](https://github.com/luckgrid/wiki-nest/blob/main/prompt-tui/records/2026-11-14-rust-cli-review.md): prior ranking based on manifests for Prompt-TUI; needs re-check before install |
| Current Workbench tooling context | [Workstation](https://github.com/luckgrid/workstation): terminal and harness workspace, not a Design System implementation; verify latest source before depending on commands |

## External primary sources

| Source | Verified claim / caution |
| --- | --- |
| [Ratatui](https://ratatui.rs/), [Style](https://docs.rs/ratatui/latest/ratatui/style/struct.Style.html), [backend](https://ratatui.rs/concepts/backends/) | native terminal layout/rendering/`Style` and compatible backends; input and app state still need explicit design |
| [ratcn](https://ratcn.com/docs/introduction), [components](https://ratcn.com/docs/components/textarea), [custom components](https://ratcn.com/docs/concepts/custom-components) | themeable Ratatui components; optional runtime for focus/events; may split render-only and behavior; pre-1.0 `0.0.x`, upstream warns of breaking API changes, components distributed by source copy (`cargo ratcn add`) |
| [ratatui-textarea](https://docs.rs/ratatui-textarea/) | existing multiline editor candidate; no verified Luckgrid integration in this document |
| [tui-realm](https://github.com/veeso/tui-realm), [Cursive](https://docs.rs/cursive/) | alternate component/event models, not evaluated in a live comparison here |
| [ratatui-style](https://docs.rs/ratatui-style/latest/ratatui_style/) | CSS-inspired styling/cascade and custom properties, **not** evidence of complete browser CSS compatibility |
| [Revue](https://github.com/hawk90/revue) | separate Vue-style TUI framework with CSS stylesheets and an accessibility module; comparison reference for CSS-like styling, not Ratatui-compatible |
| [`ratatui-css`](https://docs.rs/ratatui-css/) | crate name reserved under the Ratatui project for CSS-inspired styling experiments; no implementation; reassessment trigger |
| [rdom-css](https://docs.rs/rdom-css/latest/rdom_css/) and [gronke/ui-components](https://github.com/gronke/ui-components) | alternative DOM/CSS or cross-renderer research; distinct architecture tradeoffs |
| [Opaline](https://github.com/hyperb1iss/opaline), [guide](https://hyperb1iss.github.io/opaline/) | semantic Rust theme and multi-renderer adapters, not automatic import of Luckgrid CSS |
| [Zed `ui` crate](https://github.com/zed-industries/zed/tree/main/crates/ui) | Zed's own UI primitives and components, built on GPUI; the primary "Zed UI" reference; internal to the Zed editor, not a reusable dependency |
| [zedcraftstudio/zed-ui](https://github.com/zedcraftstudio/zed-ui) | small early-stage React + CSS-variable library sharing the name; no stated Zed Industries affiliation; **not** the Zed UI reference |
| [Zed's GPUI source](https://github.com/zed-industries/zed/tree/main/crates/gpui), [key dispatch](https://github.com/zed-industries/zed/blob/main/crates/gpui/docs/key_dispatch.md), [styled API](https://github.com/zed-industries/zed/blob/main/crates/gpui/src/styled.rs) | separate Rust native GPU UI implementation and style/event architecture |
| [longbridge/gpui-component](https://github.com/longbridge/gpui-component) | third-party GPUI component library used in a shipped desktop app; documents a headless (`gpui-base`) versus styled (`gpui-component`) split |
| [awesome-gpui](https://github.com/zed-industries/awesome-gpui) | curated project directory only, not a renderer |
| [NO_COLOR](https://no-color.org/) | conventional environment hint to suppress ANSI color; no guarantee of full accessibility |
| [Ratatui accessibility issue](https://github.com/ratatui/ratatui/issues/2610), [experimental tui-a11y](https://docs.rs/tui-a11y/latest/tui_a11y/) | terminal screen-reader support is evolving and platform-specific; test rather than claim parity; [ftui-a11y](https://docs.rs/ftui-a11y/) is a similar tree for the non-Ratatui FrankenTUI kernel |
| [awesome-ratatui](https://github.com/ratatui/awesome-ratatui), [legacy tui-rs](https://github.com/fdehau/tui-rs) | discovery and historical background, not basis for compatibility claims |

## Evidence not yet available

No executed comparison of CSS-like engine versus typed palette; no verified display-color transfer across theme overrides; no real terminal/AT compatibility matrix; no Ratatui component adoption or measured binary/performance cost; no downstream consumers using a Design System Rust adapter; no owner acceptance of a schema migration, crate or versioning contract.

A future session should report source revision/commit, inspected crate version and flags, tested platform, exact test command and results. Avoid turning proposals above into factual current product capabilities.

