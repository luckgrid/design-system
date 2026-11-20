# Semantic tokens and terminal theme projection

**Status:** research/specification candidate. No terminal tokens or API have been published.

## Existing source of truth

The current CSS authority is [tokens.tsv](https://github.com/luckgrid/design-system/blob/main/tokens.tsv), [token architecture](https://github.com/luckgrid/design-system/blob/main/docs/architecture/tokens.md) and [theme](https://github.com/luckgrid/design-system/blob/main/docs/architecture/theme.md). Semantic `--ds-*` roles are public-preview; `--ds-ref-*` reference values are internal and **not** an override seam. Brand themes map to public roles. Light/dark color values can resolve through CSS `light-dark()` using the effective `color-scheme`; the documented browser contract is a root `data-ds-scheme` hook and system preference.

Do **not** generate another authoritative source by copy/pasting resolved RGB values into unrelated apps. A terminal projection should reference role identity, and explicitly record how theme values were resolved. A stable shared source format and export compiler remain open questions.

## In-repository precedent: the Tailwind adapter projection

The repository already projects semantic roles to a second target and checks that projection mechanically. [`adapters/tailwind/projection.tsv`](https://github.com/luckgrid/design-system/blob/main/adapters/tailwind/projection.tsv) maps all 38 `public-preview` semantic `--ds-*` roles to Tailwind theme variables and explicitly excludes the 29 internal `--ds-ref-*` values, one row per role with a recorded disposition. `ds-check tailwind` verifies that table against `tokens.tsv`, `adapter-exports.tsv` and `bootstrap-surfaces.tsv`, so an added, removed or renamed role fails the check rather than drifting silently. See the [Tailwind adapter architecture](https://github.com/luckgrid/design-system/blob/main/docs/architecture/tailwind.md).

A terminal projection should reuse this shape rather than invent a new one:

- a `projection.tsv`-style table with one row per public role and a disposition such as `projected`, `reinterpreted` (e.g. spacing to cells) or `omitted` (e.g. font families), so the mapping candidate below becomes checkable data instead of prose;
- internal `--ds-ref-*` rows recorded as excluded, exactly as the Tailwind table does;
- a `ds-check` subcommand that fails when `tokens.tsv` and the terminal table disagree;
- registration through `adapter-exports.tsv` and the existing `adapters/` directory if code is ever promoted.

**Where the precedent stops.** The Tailwind adapter uses `@theme inline` so the browser keeps resolving live aliases, `light-dark()` and consumer overrides at runtime. A terminal has no CSS engine, so a terminal projection needs **resolved** color values per brand and scheme. That resolution step (the computed-browser fixture described below) is the genuinely new part, and the only part not already solved in this repository.

## Mapping candidate

| Current semantic role | Terminal candidate | Transfer status / limits |
| --- | --- | --- |
| `--ds-color-canvas` | pane/screen background | semantic yes; terminal defaults may be transparent/unknown |
| `--ds-color-surface` | separated panel fill | yes; omit fill or draw border in constrained palettes |
| `--ds-color-text` | foreground | yes |
| `--ds-color-text-muted` | secondary copy | yes; terminal contrast depends on actual palette |
| `--ds-color-border` | separators/border glyph color | yes; terminals may lack contrast or box-drawing glyphs |
| `--ds-color-accent` | primary emphasis | yes; avoid color-only focus/selection |
| `--ds-color-on-accent` | text over accent background | yes; validate contrast after quantization |
| `--ds-color-critical` | errors/destructive actions | yes; pair with text/symbol |
| `--ds-color-highlight` | selected/current background | yes; distinguish selection from focus |
| `--ds-color-focus` | focused border/marker/emphasis | semantic yes; CSS alias of accent by default, but terminal needs non-color cue |
| `--ds-font-body` / heading / code | terminal-provided glyph face | **not** portable as font family settings |
| `--ds-text-*` / `--ds-leading-*` | text hierarchy by labels, emphasis and region spacing | not pixel/line-height transferable |
| `--ds-weight-*` | normal/bold modifiers | conditional; font/terminal determines effect |
| `--ds-space-*` | integer cell gutters, gaps, controls | relationship transferable; values require new cell-specific scale |
| border width, focus width/offset, radius | border glyphs and separate focus indicator | meaning partially transferable, not CSS units/rounded corners |
| `--ds-size-target-min` | minimum usable keyboard/pointer target | reinterpret for cells and keyboard, not 2.75rem |
| `--ds-size-measure` | capped column/line width | translate to display cells after Unicode-width measurement |

These mappings are **proposals**, not one-to-one transformations. No terminal adapter should claim standards compliance solely by inheriting the CSS token names.

## Projection pipeline hypotheses

**Preferred first experiment: typed Rust palette using role identities.** For a chosen brand and scheme, resolve the CSS public roles in a controlled browser fixture, export explicit RGB/theme fixture values for comparison, and load a typed terminal palette. Any generated snapshot is a *derivative fixture*, not the canonical token definition. Document regeneration, and implement drift detection the way the Tailwind projection does: a role table checked against `tokens.tsv` by `ds-check`. Terminal-only style modifiers (bold, underline, reversed, border glyph) stay terminal-specific.

**Alternative A: declarative source-of-truth schema.** Define typed semantic tokens once, generate CSS and Rust projections. This could remove duplicate values but would relocate existing CSS authority and alter release validation. Requires separate ADR, migration plan, and proof that dynamic CSS semantics, consumer overrides and light/dark behavior are preserved. Do **not** assume it is a harmless refactor. [Opaline](https://github.com/hyperb1iss/opaline) shows the shape working outside this repository (one token file, a Ratatui adapter and a `css` feature that emits CSS), but its CSS output is a flat palette, not this repository's cascade layers, `light-dark()` aliases and consumer override seam.

**Alternative B: CSS-like runtime engine.** Evaluate a defined CSS subset and map properties to Ratatui `Style`. Assess `var()`, theme aliases, selector specificity, inheritance, supported color parsing, event-state selectors, binary size and runtime cost. Current `core.css` includes constructs (including `light-dark()` and fluid `clamp()`) that cannot be presumed supported by a terminal CSS parser; partial compatibility is not full reuse.

## Capabilities and fallback

Define a capability record rather than blindly trusting `TERM` or `COLORTERM`. Trial ordered appearance levels: truecolor -> 256 color -> 16 color -> monochrome; support no-color, limited glyphs, unknown background, no mouse, no OSC 52 and reduced-animation behavior. For terminal default colors, do not assume the background matches CSS “canvas”: either choose intentional opaque panel backgrounds or use terminal-default foreground/background with a proven contrast strategy. Many terminals answer a query for their current foreground and background colors; ratcn's optional terminal session uses that reply to solve an adaptive theme and re-solves it when the user changes theme. That is prior art for picking the light or dark scheme, with a fallback for terminals and multiplexers that don't answer.

- Treat `NO_COLOR` and redirected output as distinct use cases; no ANSI control sequences in machine-oriented output.
- Support explicit app-selected theme and system/terminal preference where detectable; unlike browsers, terminals have no universal `prefers-color-scheme` or root `data-ds-scheme`.
- Quantize color with measured contrast checks, not naive conversion. After quantization verify text/selection/focus distinguishability and legibility in both dark and light emulator themes.
- Validate visual meaning in low-color/monochrome and after removing color. Critical, focus, selected and disabled states must have independent text/glyph cues.
- Handle wide characters, grapheme clusters, wrapping, cursor alignment and border fallback via display-cell measurements; byte length and Unicode scalar count are not layout width.
- Keep brand-specific themes with the consumer; document a mapping seam only after a real downstream mapping is proven.

## Example of a conceptual typed adapter (not compiled or committed API)

    struct TerminalPalette {
        accent: Color,
        on_accent: Color,
        text: Color,
        focus: Color,
        critical: Color,
    }

    fn action_style(p: &TerminalPalette) -> Style {
        Style::default().fg(p.on_accent).bg(p.accent)
    }

The real prototype must specify error handling, source role mapping, scheme choice, terminal capabilities, and state modifiers. The simple example cannot encode accessibility by itself.

## Theme acceptance probes

1. Browser default light/dark and an overridden brand theme: enumerate **computed** public color roles, compare with projected truecolor RGB for both schemes.
2. Verify alias behavior: changing accent maps focus where its CSS alias follows the default; verify deliberate independent focus overrides.
3. Verify actual 256/16/mono appearance in a real terminal; retain focus, selected, critical and disabled distinctions after color removal.
4. Test default terminal light/dark backgrounds, SSH/herdr nesting, resize and no-color cases.
5. Verify that unsupported role types are **documented omissions**, never silent constant substitutions.
6. Detect token inventory changes and fail projection/fixture checks rather than shipping stale mappings.
7. Separate static theme configuration from the app's ephemeral focused/selected/loading states.

Further research: [Ratatui Style](https://docs.rs/ratatui/latest/ratatui/style/struct.Style.html), [Opaline](https://github.com/hyperb1iss/opaline), [ratatui-style](https://docs.rs/ratatui-style/latest/ratatui_style/), [NO_COLOR](https://no-color.org/).

