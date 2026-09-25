# Browser support

Status: **draft compatibility statement for the initial preview release.** It is
prepared for release verification and is not published as a release claim.
Release maturity is `preview`; every documented surface is `public-preview`.
Preview compatibility and overall release maturity are separate axes: this page
states which browsers the maintainers have verified, not that any surface is
stable.

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
| Chromium and Google Chrome, desktop | 123+ | Chrome for Testing 123.0.6312.122 on macOS (Apple silicon): the full browser suite, 142 of 142 | verified at the floor version, on macOS |
| Microsoft Edge, desktop | 123+ | Edge 123.0.2420.97 on macOS: 142 of 142 | verified at the floor version, on macOS; **Windows is not verified** |
| Firefox, desktop | 121+ | Playwright-built Gecko 121.0 on macOS: 142 of 142 with the Tab-to-links preference set (see below); stock Firefox 121.0 by a bounded feature and style probe only | **qualified**: see below |

Current Chromium, Firefox, and WebKit builds also pass the same suite in the
project's regression lane. That lane is regression evidence, not floor evidence.

### What "123+" and "121+" mean here

The suite was run at the floor version and on current engines. Versions between
them were not each executed; support across that range rests on the feature
floor. All floor runs used macOS on Apple silicon. Windows and Linux desktop
were not separately executed. Chrome for Testing is the exact Chromium engine
but is not the branded Chrome release.

### Firefox qualification

- The 142-of-142 Firefox 121 result was produced on a Playwright-patched Gecko
  121.0 build, because stock Firefox 121.0 cannot be driven by the test runner.
  The patched build is not the stock release.
- That run set `accessibility.tabfocus` to 7 so that plain Tab reaches links.
  With the build's original macOS defaults, 139 of 142 pass. The three failures
  are the assertions that every link is reachable by Tab; every
  Design System-dependent assertion at each focus stop passed.
- On macOS, Firefox follows the system keyboard-navigation setting when that
  preference is unset, and the macOS default does not put links in the Tab order.
  Keyboard reachability of links in Firefox on macOS therefore depends on the
  user's setting, not on the Design System, and the Design System changes no
  native keyboard behavior.
- Stock Firefox 121.0 has only a bounded probe: required features present, the
  Popover API absent (the contract treats Popover as progressive), and computed
  styles matching current Firefox except native `<option>` highlighting and the
  inline rendering of a popover element. No keyboard, accessibility, or full
  suite assertion has been run on the stock binary.

The initial preview therefore claims Firefox 121+ desktop **with this
qualification**. It does not claim an unconditional stock-Firefox pass. The
qualification stays until the full suite or an equivalent set of assertions runs
on a stock Firefox 121 binary.

## Unverified in the initial preview

These are outside the verified claim. They are deferred, not excluded:

- Chrome for Android 123
- Firefox for Android 121
- Safari on macOS 17.5
- Safari on iOS 17.5
- Safari on iPadOS 17.5

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
- **Evidence for contrast modes** comes from three separate sources that must not
  be merged: forced-colors emulation in Chromium, forced-colors emulation in
  Firefox, and owner observation with macOS Increase Contrast. Emulation is not a
  real operating-system mode, and macOS Increase Contrast is not forced colors.
  **Windows forced colors and Windows High Contrast are not verified.**
- **Assistive technology.** Screen-reader evidence is one pairing, VoiceOver with
  Safari on macOS. It is not a claim for any other screen reader.

## Expanding the claim

A browser joins the verified claim only with recorded evidence for the exact
browser, version, and platform: the executable identity, the tests that ran, the
pass and fail counts, and every limit. A skipped check, a provider failure, or a
current-engine run is not evidence. Adding a browser to this page is a
compatibility change and follows the maintainers' review process.
