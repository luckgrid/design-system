# Browser tests

The Playwright suite loads the checked-in fixtures in Chromium, Firefox, and
WebKit. It verifies the entrypoint and cascade, token resolution, theme changes,
classless defaults, layout behavior, primitive states, hook reach, consumer
overrides, keyboard behavior, reflow, and automated accessibility checks.

The static-renderer specs read the site Hugo renders, so stage and render it
first from the repository root (`sh fixtures/static-renderer/stage.sh` then
`sh fixtures/static-renderer/build.sh`; see
[the fixture README](/fixtures/static-renderer/README.md)).

From this directory:

```sh
npm ci
npx playwright install chromium firefox webkit
npx playwright test
```

Probe stylesheets under `probes/` create controlled override and failure cases.
Specs under `specs/` are organized by public contract.
