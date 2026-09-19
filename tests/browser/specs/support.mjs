import { test as base, expect } from "@playwright/test";

export { expect };

export const DS_LAYER_ORDER = [
  "ds.tokens",
  "ds.base",
  "ds.layouts",
  "ds.primitives",
  "ds.components",
  "ds.utilities",
];

export const CORE_EXPORT = "/packages/styles/index.css";

/** The core export and every stylesheet it imports: the published CSS. */
export const PUBLISHED_STYLESHEETS = [
  CORE_EXPORT,
  "/packages/styles/tokens.css",
  "/packages/styles/tokens/reference.css",
  "/packages/styles/tokens/semantic.css",
  "/packages/styles/tokens/theme.css",
  "/packages/styles/base.css",
  "/packages/styles/base/document.css",
  "/packages/styles/base/content.css",
  "/packages/styles/base/forms.css",
  "/packages/styles/base/interactive.css",
  "/packages/styles/layouts.css",
  "/packages/styles/layouts/stack.css",
  "/packages/styles/layouts/cluster.css",
  "/packages/styles/layouts/grid.css",
];

// Every test fails on any non-2xx response, failed request, console error, or
// page error. Browsers may probe for a favicon on their own; that request is not
// made by the page under test and is the only one ignored.
export const test = base.extend({
  failures: [
    async ({ page }, use) => {
      const failures = [];
      page.on("response", (response) => {
        const { pathname } = new URL(response.url());
        if (response.status() >= 400 && pathname !== "/favicon.ico") {
          failures.push(`${response.status()} ${pathname}`);
        }
      });
      page.on("requestfailed", (request) => {
        const { pathname } = new URL(request.url());
        if (pathname !== "/favicon.ico") {
          failures.push(`failed ${pathname}: ${request.failure()?.errorText}`);
        }
      });
      page.on("console", (message) => {
        if (message.type() === "error") {
          failures.push(`console: ${message.text()}`);
        }
      });
      page.on("pageerror", (error) => failures.push(`pageerror: ${error.message}`));
      await use(failures);
      expect(failures, "no failed, undeclared, or erroring requests").toEqual([]);
    },
    { auto: true },
  ],
});

/** Append stylesheet links in order and wait until every one has loaded. */
export async function loadStylesheets(page, hrefs) {
  await page.evaluate(
    (paths) =>
      Promise.all(
        paths.map(
          (href) =>
            new Promise((resolve, reject) => {
              const link = document.createElement("link");
              link.rel = "stylesheet";
              link.href = href;
              link.onload = () => resolve();
              link.onerror = () => reject(new Error(`stylesheet failed: ${href}`));
              document.head.append(link);
            }),
        ),
      ),
    hrefs,
  );
}

export async function probeColor(page) {
  return page.locator("#probe").evaluate((element) => getComputedStyle(element).color);
}

/**
 * Every style rule the stylesheet at `href` publishes, following its imports,
 * with the full cascade layer each rule lands in and the properties it declares.
 */
export async function publishedRules(page, href) {
  return page.evaluate((target) => {
    const join = (parent, name) => (parent && name ? `${parent}.${name}` : parent || name || "");
    const found = [];
    const walk = (rules, layer, sheetHref) => {
      for (const rule of rules) {
        const kind = rule.constructor.name;
        if (kind === "CSSImportRule") {
          walk(rule.styleSheet.cssRules, join(layer, rule.layerName), rule.styleSheet.href);
        } else if (kind === "CSSLayerBlockRule") {
          walk(rule.cssRules, join(layer, rule.name), sheetHref);
        } else if (kind === "CSSStyleRule") {
          found.push({
            sheet: new URL(sheetHref).pathname,
            layer,
            selector: rule.selectorText,
            properties: [...rule.style],
          });
        } else if (rule.cssRules) {
          walk(rule.cssRules, layer, sheetHref);
        }
      }
    };
    const sheet = [...document.styleSheets].find((candidate) => candidate.href?.endsWith(target));
    walk(sheet.cssRules, "", sheet.href);
    return found;
  }, href);
}
