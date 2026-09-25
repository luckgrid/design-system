# Browser support

Status: **published compatibility statement for the initial preview release.**
Release maturity is `preview`; every documented surface is `public-preview`.
Preview compatibility and overall release maturity are separate axes: this page
states which browsers the maintainers have verified, not that any surface is
stable.

Every claim below is limited to what was observed. Where a result comes from a
particular build, harness, or platform, the page names it.

## Two different statements

- **Feature floor.** The CSS is written against the features available in
  Chromium and Chrome Android 123, Edge 123, Firefox and Firefox Android 121, and
  Safari 17.5 on macOS, iOS, and iPadOS. The floor is derived from the features
  the stylesheets require (`light-dark()` for Chromium and Safari, `:has()` for
  Firefox). It is a design target, not a test result.
- **Verified support claim.** The browsers the maintainers have actually run the
  test suite on. Only this claim is a compatibility statement for the initial
  preview release.

A browser outside the verified claim is **unverified**, not unsupported. Nothing
here says the CSS fails there, and nothing here says it works.

## Verified for the initial preview

| Browser family | Floor | Verified on | Status |
|---|---|---|---|
| Chromium engine, desktop (Chrome for Testing) | 123+ | Chrome for Testing 123.0.6312.122 on macOS (Apple silicon): the full browser suite, 142 of 142. Chrome for Testing is the exact Chromium engine but is not the branded Google Chrome release; **branded Chrome was not executed.** | verified at the floor version for the Chromium engine, on macOS; branded Chrome 123 is unverified |
| Microsoft Edge, desktop | 123+ | Edge 123.0.2420.97 on macOS: 142 of 142 | verified at the floor version, on macOS; **Windows is not verified** |
| Firefox, desktop | 121+ | Playwright-built Gecko 121.0 on macOS: 142 of 142 with the Tab-to-links preference set (see below); stock Firefox 121.0 by a bounded feature and style probe only | **qualified**: see below |

Current Chromium, Firefox, and WebKit builds also pass the same suite in the
project's regression lane. That lane is regression evidence, not floor evidence.

### What "123+" and "121+" mean here

The suite was run at the floor version and on current engines. Versions between
them were not each executed; support across that range rests on the feature
floor. All floor runs used macOS on Apple silicon. Windows and Linux desktop
were not separately executed. Chrome for Testing 123.0.6312.122 is the exact
Chromium engine but is not the branded Chrome release, and no branded Chrome
build was run.

### Firefox qualification

What was observed, by build. Each line names the build it comes from; none is
merged with another.

- **Playwright-patched Gecko 121.0 (macOS).** The 142-of-142 result was produced on
  a Playwright-patched Gecko 121.0 build, because stock Firefox 121.0 cannot be
  driven by the test runner. The patched build is not the stock release. That run
  set `accessibility.tabfocus` to 7. With the build's defaults on the test host,
  139 of 142 pass: plain Tab skipped links, and the three failures are the
  assertions that every link is reachable by Tab. Every Design System-dependent
  assertion at each focus stop passed. At its defaults the patched build behaved
  like `accessibility.tabfocus` set to 2 (button, input, and button reached; links
  skipped); with 7 it reached all five stops on the probe page.
- **`-AppleKeyboardUIMode` override.** Passing `-AppleKeyboardUIMode` 2 or 3 to the
  patched build changed nothing. That result is inconclusive and is not evidence
  about the operating-system setting. **There was no real system keyboard-setting
  toggle test:** no macOS system setting was changed, so no claim is made about how
  a stock Firefox responds to Full Keyboard Access.
- **Playwright Firefox 155.0 (same host).** At its defaults it reached all five
  stops on the probe page, the same as `accessibility.tabfocus` 7. The reason the
  default differs from the 121 build was not identified.
- **Stock Firefox 121.0.** It has no keyboard evidence. It has only a bounded
  feature and style probe: required features present, the Popover API absent (the
  contract treats Popover as progressive), and computed styles matching current
  Firefox except native `<option>` highlighting and the inline rendering of a
  popover element. No keyboard, accessibility, or full-suite assertion has been run
  on the stock binary.

**External documentation, not an observation.** Mozilla bug 187508 and independent
accessibility write-ups (the A11Y Project and others) describe Firefox on macOS
following the system's keyboard-navigation setting when `accessibility.tabfocus`
is unset. That is cited here as external documentation. It was not verified on
this host, and this page does not restate it as a finding.

The Design System changes no native keyboard behavior. Whether links are in the Tab
order in Firefox on macOS is decided by the browser and its settings, not by the
Design System.

The initial preview therefore claims Firefox 121+ desktop **with this
qualification**. It does not claim an unconditional stock-Firefox pass. The
qualification stays until the full suite or an equivalent set of assertions runs
on a stock Firefox 121 binary.

## Unverified in the initial preview

These are outside the verified claim. They are unverified, not excluded:

- Windows desktop (every desktop browser, including Windows Chrome and Firefox)
- Linux desktop
- Windows Edge (Edge was run on macOS only)
- Chrome for Android 123
- Firefox for Android 121
- Safari on macOS 17.5
- Safari on iOS 17.5
- Safari on iPadOS 17.5

The last five rows are deferred as `DS-E05.S4.T1-C1`. They do not block this
preview release. Windows desktop, Linux desktop, and Windows Edge are simply not
verified.

Owner-observed manual checks in current Safari on macOS 26 (keyboard,
VoiceOver, zoom, reflow, print) and the current WebKit regression lane do not
verify Safari 17.5 or any mobile browser. Embedded web views and in-app browsers
are unverified. Browsers not named on this page carry no claim.

## Accessibility modes

- **Forced colors.** Primary and default actions can look identical when a
  browser or operating system applies forced colors; primary emphasis is a
  presentational hierarchy and is not guaranteed. Do not rely on color, fill, or
  contrast alone to carry required meaning. The action's native identity, focus,
  current, disabled, and unavailable-link states remain contractual. See
  [Primitives](architecture/primitives.md).
- **Evidence for contrast modes** comes from separate sources that must not be
  merged, and each is limited to the engine it was observed on:
  - *Chromium forced-colors emulation* (`forcedColors: "active"`): observed on
    current Chromium, and the same result was observed on Chrome for Testing
    123.0.6312.122. It is a bounded probe; the checked-in suite has no
    forced-colors assertion.
  - *Firefox forced-colors emulation*: observed on **Firefox 155.0**, not on
    Gecko 121. On the Playwright-patched Gecko 121.0 build, `forcedColors: "active"`
    matches the media query but applies no forced palette, so **no forced-colors
    evidence exists for Firefox 121.**
  - *WebKit forced-colors emulation* applies no palette, so it is not evidence.
  - *macOS Increase Contrast* is owner-observed on the exercised action states. It
    is a different mechanism from forced colors, and Safari on macOS has no
    forced-colors mode.

  Emulation is not a real operating-system mode. **Windows forced colors and
  Windows High Contrast are not verified.**
- **Assistive technology.** Screen-reader evidence is one pairing, VoiceOver with
  Safari on macOS. It is not a claim for any other screen reader.

## Expanding the claim

A browser joins the verified claim only with recorded evidence for the exact
browser, version, and platform: the executable identity, the tests that ran, the
pass and fail counts, and every limit. A skipped check, a provider failure, or a
current-engine run is not evidence. Adding a browser to this page is a
compatibility change and follows the maintainers' review process.
