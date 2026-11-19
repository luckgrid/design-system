# Proposed cross-platform architecture and ownership

**Status:** proposed, architecture hypothesis, 2026-11-18. Not the current shipped architecture.

## Design principle

Maintain one vocabulary of **semantics and behavior** with independent web, terminal and (potentially) native desktop implementations. This is not an attempt to make Ratatui parse `core.css` or to replicate DOM, ARIA, CSS cascade, or GPUI in a terminal.

    Product-specific applications
      Web products      Prompt-TUI       wiki-reader      Workbench tools
           |                |                |                  |
      HTML + CSS        Ratatui adapter   Ratatui adapter  native / Rust UIs
           |                |                |                  |
           +----------------+----------------+------------------+
                            |
             Proposed Design System contracts
        semantic roles | behaviors | pattern documentation
        variant meaning | states | testing expectations
                            |
        Existing web authority (unchanged at this stage)
         tokens.tsv / CSS roles / theme and hooks

The arrows represent conceptual contract consumption, not shared runtime dependencies. A future native desktop adapter (e.g. GPUI) is optional and explicitly out of scope for the first milestone.

## Layer taxonomy

| Layer | Renderer-neutral contract | Web implementation | Terminal implementation |
| --- | --- | --- | --- |
| Semantics | roles such as foreground, accent, focus, critical, selected | existing `--ds-*` CSS roles | typed palette projected by identity |
| Interaction | semantic state transitions, key actions, focus, validation | HTML native behavior first, ARIA where necessary | explicit event/focus handling |
| Layout relationship | stack, horizontal cluster, columns, region hierarchy | responsive CSS layouts | cell constraints and narrow-pane switching |
| Primitive | bounded surface; action/control | semantic elements and opt-in CSS hooks | frame/panel; focusable action semantics in app/controller |
| Pattern | searchable picker, property editor, confirmation, status feedback | future reviewed web composition | optional Ratatui components or composition guides |
| Application | user task, data model, routing, persistence | consumer | consumer |

An HTML button's native activation semantics and terminal key-press routing are **not** interchangeable code. The shared promise can say “a primary action has an accessible label, a disabled state that cannot activate, a visible focus state, and a consistent cancel path”; the renderer must prove how it provides that outcome.

## Contracts must not leak implementation

1. **State:** distinguish focused, hovered (when present), selected/current, pressed, disabled, loading, invalid, critical, dirty, pending, success. The meanings of focus and selection must not be collapsed. Define precedence and non-color indicators; a terminal adapter cannot assume CSS pseudo-classes or HTML `aria-current`.
2. **Events:** application decides commands, field validation, undo, persistence and side effects. Patterns define navigation conventions and events to deliver (activate, move, cancel, edit, commit) but do not execute business commands.
3. **Identity:** stable item keys, deterministic ordering and reversible navigation; do not tie state to display row or color.
4. **Responsive behavior:** width and height in terminal cells; preserve focus and selected items on resize; a one-pane mode has a way back. Do not directly translate pixel breakpoints or viewport-fluid CSS.
5. **Accessibility:** keyboard-first operation, discoverable shortcuts, status in text, observable focus, alt/linear CLI output where full-screen UI is inaccessible. Do not claim ARIA equivalence or screen-reader parity.
6. **Security:** sanitize untrusted terminal-control sequences; terminal restoration on cancellation and panic/error; no hidden clipboard reads or automatic OSC 52 transfer; no ANSI controls on machine-facing output.
7. **Theming:** styles resolve through role names, with capability fallback. Consumer brand themes may remap roles; shared patterns must not hardcode product brand palettes.
8. **Dependency boundary:** using a pattern spec must not require a framework, JS, CSS processing, a daemon, or a shared application runtime. Any future Rust crate is opt-in.

## Composition versus shared framework

**Recommended starting position:** documented patterns and typed tokens -> small opt-in Ratatui adapter. Prefer core widgets and narrow third-party widgets; use ratcn only where component/focus reuse demonstrably wins. Rebuilding a reconciler, virtual DOM, new CSS parser, or a shared event runtime requires evidence from at least two different applications and a written alternative analysis.

**Web remains web-first.** CSS `core.css`, semantic HTML, browser-driven interaction and the existing release/inventory/compatibility tables remain unchanged. This proposal does not replace the default implementation with a universal schema and does not introduce an abstract component DSL.

**Terminal constraints are first-class:** monospaced cells, ambiguous glyph widths, no dependable pointer, limited color depth, unknown terminal background, variable emulators/multiplexers, no standard DOM accessibility tree, resize events, SSH/remote clipboard behavior.

## Consumer scenarios from existing project planning

- **Prompt-TUI:** the source design describes header, navigation, properties, preview and footer. Narrow view switches to one pane. A visible copy scope, unresolved-conflict count, target and selected variant must survive. These are product requirements, not default Design System components. Source: [Prompt-TUI composition architecture](https://github.com/luckgrid/wiki-nest/blob/main/prompt-tui/architecture/interface.md).
- **wiki-reader:** actual Rust/Ratatui app, plausible testbed for scrolling lists, navigation/focus and readable content in a herdr pane. Verify its current code before integrating.
- **Workbench:** agent task lists, status, notifications and command actions benefit from shared conventions; orchestration and terminal security decisions stay with Workbench.
- **Web:** a future component can share behavioral descriptions with terminal patterns, but native HTML semantics remain preferred. No promise of identical markup, Rust components or visual geometry.

## Potential workspace placement (illustrative, not a commitment)

Documentation remains here. If prototyping earns code, consider an **optional** Rust crate in `adapters/ratatui/` with its own feature flags, examples, tests and consumer boundaries. `adapters/` is preferred over `packages/terminal/` because the repository already places the optional [Tailwind adapter](https://github.com/luckgrid/design-system/blob/main/docs/architecture/tailwind.md) there and registers adapter outputs in `adapter-exports.tsv`; a terminal adapter should follow the same registration, projection-table and `ds-check` pattern (see [token projection](tokens-and-themes.md#in-repository-precedent-the-tailwind-adapter-projection)). Do not add these paths before a reviewed decision. Avoid giving external consumers access to internal `packages/styles/tokens/reference.css` values.

See [decisions](decisions.md), [patterns](patterns-and-components.md) and [roadmap](verification-and-roadmap.md).

