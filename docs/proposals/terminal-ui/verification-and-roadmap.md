# Research-to-prototype plan, risks, tests and handoff

**Status:** planning only; no source implementation approved. Every milestone has a non-invasive stop point; approval is required to publish a compatibility surface.

## Milestones

### M0 — Documentation review (this PR)

- Record current source contracts and their non-goals, proposed architecture, token matrix, patterns, technology evidence, decisions, risks, and owner questions.
- Confirm docs are internal in `bootstrap-surfaces.tsv`. Ensure `core.css` and surface registries do not change.
- Run inventory/boundary checks and link integrity where executable. Have an independent reviewer challenge naming, source quality and release claims.
- **Done when:** docs are navigable, citations explain their limits, classification check passes and no public-preview change was made. **Stop:** draft PR review.

### M1 — Token and terminal capability proof

- In a separate implementation PR, prototype an internal role-ID-to-`Color` mapping using typed Rust structs. Compare browser **computed** defaults and a brand override in light/dark schemes.
- Contrast after quantization, test dark/light terminal defaults, RGB/256/16/mono, `NO_COLOR` and no TTY. Document unsupported CSS roles.
- Make sample fixture regeneration deterministic; prove a deliberate token-role change fails a stale projection test.
- **Done when:** no second token authority is silently created, fallbacks remain legible, release CSS bytes unchanged. **Stop:** decide whether schema, projection or CSS-style engine deserves deeper evaluation.

### M2 — Interaction behavior spike

- Build a small private/internal Ratatui test surface: selectable list, pane navigation, confirmation, status/key-hint strip. All components are composed from existing primitives initially.
- Compare a native list/editor with ratcn and/or `ratatui-textarea` only on one real task. Measure source complexity and upgrades, don't just compare screenshots.
- Include key trace, mouse on/off, focus restore, Unicode widths, resizing, no color, disabled states, errors, noninteractive equivalent and terminal teardown.
- **Done when:** repeatable tests and documented integration cost show which aspects should be shared. **Stop:** evaluate scope before shipping any crate.

### M3 — Two consumer trials

- Integrate a bounded pattern in **Prompt-TUI** and **wiki-reader** (or another materially different Rust terminal app), not just two views of the same product.
- Preserve each consumer's app state, commands, user-specific fields, clipboard policy and harness interface. Measure which shared code reduced duplication and which coupling caused friction.
- Validate compatibility within a real herdr narrow pane and through SSH/multiplexer conditions used by consumers; do not mutate workstation environment policies merely for trial convenience.
- **Done when:** at least two distinct products confirm the same stable semantic contract; divergence and exceptions are explicitly recorded. **Stop:** choose docs-only, helpers, headless behavior module or widgets.

### M4 — Optional crate and preview boundary (requires separate decision)

- If earned, propose minimal crate ownership, feature flags, MSRV, dependency pins, sample apps, stability classifications, changelog, versioning and supply-chain checks.
- Prefer independent optional exports; no requirement for web consumers to install Rust, JavaScript, a terminal runtime or CSS processors.
- Update source inventory, release documentation and distribution only through a reviewed release design. The current CSS archive is **not** automatically a channel for the crate.
- **Done when:** tested consumer installs from a pinned artifact, rollback and compatibility expectations are documented. **Stop:** owner explicitly accepts API promotion.

### M5 — Future native adapter (conditional)

- Only if a concrete GPUI/other native application exists, trial the *same* semantics and pattern checks in a GUI implementation. Maintain separate native accessibility and rendering evidence.
- No scheduled work or runtime dependency otherwise.

## Cross-cutting test checklist

| Concern | Minimum evidence |
| --- | --- |
| Token parity | computed CSS roles vs terminal truecolor fixture across light/dark and override; alias behavior |
| Color fallback | truecolor / 256 / 16 / mono, `NO_COLOR` and terminal-default background |
| Keyboard | complete navigation and activation without pointer; disabled, modal, cancel and focus restoration |
| Layout | widths 40/80/120+ columns, shallow screen, resize while editing, scroll and selection preservation |
| Text | CJK, emoji, combining graphemes, long strings, bidi/IME where relevant; terminal-cell width correctness |
| Output | deterministic plain/JSON CLI bytes, stdout versus stderr separation, no ANSI when redirected |
| Trust boundary | sanitize terminal escape/control strings, untrusted input and errors, zero implicit clipboard reads/writes |
| Lifecycle | terminal state restoration after normal completion, signal/cancel, error/panic where recoverable |
| Assistive tech | explicitly tested environment/screen reader where supported, with recognized gaps and linear fallback |
| Packaging | no unexpected release inventory/public surface; dependency and advisory review for a new crate |
| Performance | baseline startup, draw/update latency, memory and binary size measured on same task/backend |

No test matrix row is a current PASS; these are future gates.

## Risks and mitigations

| Risk | Why it matters | Mitigation |
| --- | --- | --- |
| Accidental second token authority | CSS brand mapping, aliases, light/dark semantics can drift | explicit role mapping, generated comparison fixtures, separate schema ADR if needed |
| Premature runtime/framework | app-specific focus/state logic migrates to an unneeded abstraction | prototype two consumers, adopt narrow helpers |
| Inflated “CSS reuse” claim | CSS parsing subset cannot recreate browser semantics | inventory unsupported syntax; compare actual computed values |
| Accessibility overclaim | a terminal generally lacks the web's native accessible semantics | real AT tests; deterministic CLI/linear fallback; document limitations |
| Color and Unicode mismatch | 16 colors, light terminal backgrounds and wide glyphs can erase meaning | capability matrix, display width and no-color tests |
| Clipboard/security | OSC 52 and untrusted escape sequences can cause unwanted effects | explicit permission policy, sanitize strings, always offer stdout/file |
| Multi-app coupling | a shared controller can own too much consumer state | stable event contracts and consumer-owned data/side effects |
| Release contract confusion | draft docs read as published API | internal classification, source-only linking, separate preview proposal |
| Dependency breakage | Ratatui/Crossterm and components may diverge | pin/test exact versions, MSRV and feature audit |
| Misattributed ecosystem provenance | “Zed UI” may be confused with Zed/GPUI | separate sources, maintainers and implementations in docs |

## Handoff workflow for next session

1. Read [overview](README.md), [decisions](decisions.md), [tokens](tokens-and-themes.md), [patterns](patterns-and-components.md) and [source ledger](sources.md) **before** proposing code.
2. Read latest Design System `main` README, `tokens.tsv`, `theme.tsv`, `bootstrap-surfaces.tsv`, release inventory and existing CSS contracts. Re-check source evidence and commit refs for updated consumers.
3. Identify one high-value consumer scenario; document assumptions and exact task/terminal conditions.
4. Compare two deliberately small prototypes, not generalized frameworks; capture measured results and failures.
5. Open a **separate** implementation PR, with explicit owner decisions for any token-authority migration, crate publication or compatibility promotion.

## Questions for independent review

- Does documentation imply a new token authority without a deterministic mapping mechanism?
- Is the first shared pattern truly product-neutral, or just Prompt-TUI's layout extracted?
- Are source citations and repository affiliations correct, especially Zed UI versus GPUI?
- Does any proposed test misrepresent what terminal accessibility or NO_COLOR can guarantee?
- Could the design be achieved with existing Ratatui primitives and no new runtime?
- Is the CSS release/archive boundary still exact and unchanged?

