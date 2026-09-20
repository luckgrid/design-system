# Browser tests

The Playwright suite loads the checked-in fixtures in Chromium, Firefox, and
WebKit. It verifies the entrypoint and cascade, token resolution, theme changes,
classless defaults, layout behavior, primitive states, hook reach, consumer
overrides, keyboard behavior, reflow, and automated accessibility checks.

From this directory:

```sh
npm ci
npx playwright install chromium firefox webkit
npx playwright test
```

Probe stylesheets under `probes/` create controlled override and failure cases.
Specs under `specs/` are organized by public contract.
