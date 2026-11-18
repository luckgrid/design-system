# TUI primitives, component contracts and interaction patterns

**Status:** proposed catalog. **No component APIs or generic runtime are accepted.** The existing Design System's [primitive taxonomy](https://github.com/luckgrid/design-system/blob/main/docs/architecture/primitives.md) distinguishes a base primitive (`surface`), a UI primitive (`action`), and higher-level product patterns. Keep those distinctions here rather than labeling every terminal widget a primitive.

## Three levels of reuse

1. **Presentation primitive:** a panel/surface, separator, text role, stack or cluster of terminal cells. May be render-only and stateless.
2. **Interaction primitive:** button-like action, selectable row, editable field, focus target, modal controller. Requires semantic state and input contract. Its name does not grant web-like native keyboard behavior.
3. **Product-neutral pattern:** searchable picker, command palette, property inspector, confirmation flow, task status list, preview/copy work area. Composes primitives, reserves app/domain policy to the consumer.

Default widgets from Ratatui are useful *drawing* building blocks, not automatically complete interactive controls. ratcn separates paint-only widgets from optional focus/event routing; this is one possible implementation boundary, not an architectural prerequisite.

## Shared behavior specification template

Every promoted candidate should document:

- **Intent and non-goals:** what problem, how to avoid overgeneralizing a product feature.
- **Inputs:** stable item ID, label, secondary copy, help text, available/disabled, validation, actions.
- **States:** focus versus selection versus current item; idle, hover, pressed, pending, error, success, disabled, dirty.
- **Actions/events:** activate, navigate, cancel, apply, commit, edit, copy; exact terminal shortcut defaults and overridable application bindings.
- **Focus contract:** entry/exit, wrap/no wrap, focus restoration, active descendant, modal containment if applicable.
- **Layout:** bounded width/height, overflow, truncation/wrapping, resize behavior, wide characters and mouse hit testing.
- **Theme roles:** foreground, surface, border, accent, highlight, critical, focus; non-color equivalents.
- **Accessibility:** visible keyboard focus, labels, help, text output, no-color, narrow/linear alternatives; explicit AT limitations.
- **Output/security:** safe render of untrusted strings, no escape sequence injection, no implicit side-effect, deterministic export.
- **Testing:** buffer snapshots, key sequences, mouse/resize, cancellation, golden output, terminal cleanup, fallback modes.
- **Compatibility:** internal/proposed until consumer evidence and reviewed published API.

## High-priority pattern catalog

| Candidate | Reusable contract | Initial consumer examples | Priority |
| --- | --- | --- | --- |
| Focusable selectable list | stable IDs, focus/selection distinction, next/previous, disabled items, scrolling, empty | wiki-reader navigation, Prompt-TUI variant navigation | P0 |
| Pane navigation | focus traversal, active region, one-pane narrow behavior, return path | Prompt-TUI, Workbench | P0 |
| Status / key-hint footer | operation and error visibility, shortcut help priority, no-color | Prompt-TUI, Workbench | P0 |
| Confirmation dialog | focus containment and restoration, confirm/cancel, destructive default | workspace operations, overwrites | P0 |
| Searchable picker | filtered results, selection semantics, query state, zero-result feedback | prompt templates, tool selection | P1 |
| Property editor | typed text/number/choice/toggle/list, per-field validation, dirty state | Prompt-TUI, workstation configurations | P1 |
| Command palette | searchable commands, shortcut discoverability, command invocation boundary | Workbench, workstation | P1 |
| Activity / task list | pending/running/blocked/done/failed as text and structure | agent sessions, batch operations | P1 |
| Preview / selection / copy area | preview matches selected output, copy scope explicit, clipboard fallback | Prompt-TUI, wiki-reader exports | P1 |
| Toast / inline feedback | polite status, persistent errors, log or noninteractive equivalent | broad | P2 |
| Tabs / menus / overlays | selection/activation and modality semantics | future applications | P2 |
| Tables / trees / diffs | virtualization, nested navigation, accessible flattening | future workbench and wiki tools | P2 |

P0 is **first trial priority**, not a promise that each becomes a shipped widget.

## Behavioral acceptance examples

### Selectable list

- A directional key changes **focused row**; selected items remain selected until the documented selection action.
- Disabled rows are visibly unavailable and do not activate. Decide whether arrow navigation skips disabled rows or permits inspection, then use that rule consistently.
- Search/filter does not accidentally transfer selection to a different stable ID when visible row index changes.
- Resize preserves focused ID if still visible; if filtered out, define a deterministic nearest-focus fallback.
- Empty/filtered-out lists show textual guidance, not blank space. Wide/combining Unicode must not corrupt cursor or border alignment.
- Mouse remains optional; every operation has a keyboard path.

### Pane navigation (Prompt-TUI example, not canonical layout)

Proposed product regions are header (target/brief), navigation (template/variant), property editor, output preview and footer. In wide mode multiple panes are visible; narrow mode shows **one focused pane** plus an obvious switch/return affordance. The source Prompt-TUI plan requires visible target/model, selected variant/slot, copy scope and conflict/reference reminders even when narrow. The Design System could provide reusable focus transitions and width-triggered pane-mode conventions, but not make those product-specific fields universal.

Cases: resize while editing a wrapped line; switch to preview and back without losing cursor position; trigger Escape in nested pane/dialog and return to originating focus; preserve unsaved data; avoid keyboard binding conflicts with embedded harness and multiplexer.

### Confirmation dialog

- Entering the dialog captures keyboard navigation; cancel always returns to the original focus target. Escape dismisses when safe; if cancellation would lose unsaved work, provide a distinct decision.
- Primary/destructive meaning is communicated by text, not red alone. Destructive actions should not become the default merely due to focus order.
- If underlying operation fails, retain the user's context and show a persistent explanation. Confirm only invokes a consumer-supplied action; the component never performs file/network changes itself.

### Property editor and multiline editing

- Typed fields have stable keys, visible labels/units, source of defaults, and local error messages. A dirty marker and undo scope are explicit.
- A multiline editor needs tested grapheme-aware movement, wrapping, selection, scroll and input method behavior. Prefer evaluating `ratatui-textarea` instead of hand-rolling this early.
- Editing/accepting/canceling must not mutate exported text unexpectedly; source-backed Prompt-TUI examples require byte-precise output selection and export.
- When an edited output is an override, external source changes should mark it stale rather than silently rewrite it. This is Prompt-TUI domain behavior; the editor widget should simply expose edit events.

### Status, task list and copy scope

- A task state must be legible in monochrome: e.g. “blocked: awaiting input” rather than just a yellow icon.
- Progress text announces meaningful transition; avoid a constantly redrawing screen that destroys terminal scrollback or produces unusable assistive-technology output.
- Copy actions must name the selected scope (“Copy style”, “Copy variant”, “Copy 3 variants”). A shared visual pattern can render and label the action; export-format policy stays with the consumer.
- OSC 52 clipboard output is a capability/permission decision owned by the terminal environment and app. Always provide stdout/file export fallback; do not read clipboard or copy implicitly.

## Test matrix for each component

| Dimension | Required evidence |
| --- | --- |
| View widths | narrow (e.g. 40 columns), common (80), wide (120+), shallow height |
| Input | keyboard-only; mouse when enabled; terminal without mouse |
| Unicode | ASCII, wide CJK, emoji, combining marks, long unbroken strings |
| Appearance | light/dark, no-color, 16-color, 256-color, truecolor where available |
| Interaction | focus restore, disabled, selection, cancel, retry, undo (if in scope) |
| Output | no side effects on render, determinism, sanitized untrusted text |
| Lifecycle | clean restore on normal exit, failure, cancellation and panic where recoverable |
| Alternate surface | plain CLI/structured output when full-screen TUI is inappropriate |

No fixed CSS pixel breakpoint, border radius or tap-size threshold is directly applicable to terminal cells. Numeric columns are examples for trial, not final minimum support promises.

## Explicit deferrals

Do not start with a huge catalog, a universal router, generalized forms engine, component marketplace, terminal DOM, CSS-compatible cascade, AI-generated component API, or renderer-independent state store. Promote only patterns proven in at least two unlike consumers with a measured maintenance benefit.

Sources: [Prompt-TUI interface](https://github.com/luckgrid/wiki-nest/blob/main/prompt-tui/architecture/interface.md), [Ratatui](https://ratatui.rs/), [ratcn custom component concept](https://ratcn.com/docs/concepts/custom-components), [Ratatui TestBackend](https://docs.rs/ratatui/latest/ratatui/backend/struct.TestBackend.html).

