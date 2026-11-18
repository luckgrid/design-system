# Decision register — proposals only

**All entries are proposed, not accepted.** This file intentionally records choices, competing options, confidence and evidence needed to change status. The user asked to preserve research for later refinement, not to authorize a new implementation or public compatibility surface.

## TUI-001 — One design language, separate renderers

**Proposed:** Keep the Design System's current HTML/CSS web implementation and add platform-independent *semantic and interaction* documentation. Map to native Ratatui widgets for terminal. Do not create a universal renderer or demand same visual output.

**Alternatives:** (A) separate autonomous TUI design system (duplication/drift); (B) universal component abstraction that replaces web CSS (breaks current intent, premature complexity); (C) CSS itself as runtime input to every platform (unsupported syntax and semantics). **Confidence:** high as a starting architecture. **Promotion gate:** two unrelated consumers implement common pattern contracts without hiding platform semantics.

## TUI-002 — Tokens before runtime stylesheets

**Proposed:** Prove a typed Rust theme palette projected from public semantic role identities, retaining CSS token authority and release contracts. CSS-like terminal style parsers are research alternatives rather than mandatory dependencies.

**Alternatives:** duplicate palette per app; generated platform-neutral schema replacing CSS authority; CSS runtime cascade. **Confidence:** medium, because exact value/source propagation through CSS overrides is unsolved. **Promotion gate:** live browser computed-token comparison; quantified drift; brand alias/scheme behavior; low-color tests.

## TUI-003 — Patterns before component breadth

**Proposed:** Publish internal pattern specs first (focus navigation, list/picker, confirmation, feedback). Start reusable code only after consumer evidence. Component contract is state/events/a11y/keyboard behavior + styling roles, not just a screenshot or widget name.

**Alternatives:** import full component library as the Design System; copy controls independently to each app; build generic runtime now. **Confidence:** high. **Promotion gate:** demonstration in Prompt-TUI **and** a materially different Rust terminal consumer.

## TUI-004 — Ratatui first, runtime optional

**Proposed:** Ratatui + compatible Crossterm for initial terminal proof; compare native widgets, `ratatui-textarea`, and ratcn for real interactions. Do not introduce a second event engine by default. Test the existing Workbench backend/event policies rather than silently overriding them.

**Alternatives:** Cursive, bare Crossterm, tui-realm, ratcn for all controls, bespoke runtime. **Confidence:** medium-high given existing consumers. **Promotion gate:** measured task and lifecycle tests with dependency analysis.

## TUI-005 — Accessibility is an independent platform obligation

**Proposed:** Define keyboard-only operation, text/status feedback, no-color, linear CLI fallback and terminal capability handling. Treat accessible-technology support as unverified until tested. ARIA/HTML and CSS WCAG evidence cannot be inherited by Ratatui.

**Alternatives:** claim equivalence through common naming; provide only full-screen visual UI. **Confidence:** high. **Promotion gate:** real terminal, screen-reader/assistive-tech tests where support exists, explicit unresolved limitations.

## TUI-006 — Documentation and packaging boundary

**Proposed:** Keep this research in `docs/proposals/terminal-ui/`, classified `internal`, linked from source documentation but not the published CSS archive. No `core.css`, `exports.tsv`, `tokens.tsv` or shipped release inventory modifications. New Rust adapter/crate requires a separate release and compatibility decision.

**Alternatives:** add draft docs into the consumer release (risks implicit promises and broken references); place plans only in application workspaces (knowledge divergence). **Confidence:** high. **Promotion gate:** explicit packaging and consumer contract review.

## Questions to answer in later sessions

1. Should the longer-term authoritative source become a neutral typed schema generating CSS, or remain CSS with a validated Rust projection? What are the costs to current browser/theming contracts?
2. What is the lowest stable pattern unit: documentation, theme/style helpers, a headless interaction controller, or full widgets? Which pairs can be demonstrated in two products?
3. How much cross-platform behavioral parity is useful when web uses native semantics and terminal uses manual focus/event routing?
4. Which terminal emulators, platforms, accessibility tools and workbench/SSH configurations form the support matrix?
5. Will a future GPUI desktop consumer actually exist? If not, avoid native-desktop adapter commitments.
6. Can existing ratcn/ratatui components be used as dependencies, or does source-copy maintenance better fit audited/pinned releases?
7. Who owns token generation and versioning if terminals add additional roles such as status/pending/focus border that current CSS tokens do not express?
8. What explicit owner acceptance and new ADR format should mark these proposals as decided?

See the [roadmap](verification-and-roadmap.md) and [source ledger](sources.md).

