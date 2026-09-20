import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import AxeBuilder from "@axe-core/playwright";

import {
  CORE_EXPORT,
  DS_LAYER_ORDER,
  PUBLISHED_STYLESHEETS,
  expect,
  loadStylesheets,
  publishedRules,
  test,
} from "./support.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..");

/** The owned subjects in base.tsv, by group. */
const OWNED = readFileSync(path.join(root, "base.tsv"), "utf8")
  .split("\n")
  .filter((line) => line.startsWith("public-preview\t"))
  .map((line) => line.split("\t"))
  .map(([, group, subject]) => ({ group, subject }));

const FIXTURE = "/fixtures/plain-html/index.html";
const BASE_SHEETS = PUBLISHED_STYLESHEETS.filter((href) => href.includes("/packages/styles/base"));
const SCHEMES = ["light", "dark"];

/** Scheme states: the preference alone, and the hook opposite the preference. */
const STATES = [
  { label: "light preference", preference: "light", hook: null, scheme: "light" },
  { label: "dark preference", preference: "dark", hook: null, scheme: "dark" },
  { label: "hook dark over light preference", preference: "light", hook: "dark", scheme: "dark" },
  { label: "hook light over dark preference", preference: "dark", hook: "light", scheme: "light" },
];

async function openFixture(page, { preference, hook }) {
  await page.emulateMedia({ colorScheme: preference });
  await page.goto(FIXTURE);
  if (hook) {
    await page.evaluate((value) => {
      document.documentElement.dataset.dsScheme = value;
    }, hook);
  }
}

/**
 * Install helpers in the page: `__rgb(color)` converts any CSS color to sRGB
 * bytes through a canvas, so oklch() and rgb() compare equal when they are the
 * same color; `__role(name, property)` resolves a role on a probe in <body>.
 */
async function installHelpers(page) {
  await page.evaluate(() => {
    const context = document.createElement("canvas").getContext("2d", { willReadFrequently: true });
    window.__rgb = (color) => {
      context.clearRect(0, 0, 1, 1);
      context.fillStyle = "rgb(0 0 0 / 0)";
      context.fillStyle = color;
      context.fillRect(0, 0, 1, 1);
      return [...context.getImageData(0, 0, 1, 1).data];
    };
    window.__role = (name, property = "color") => {
      const probe = document.createElement("div");
      document.body.append(probe);
      probe.style.setProperty(property, `var(${name})`);
      const value = getComputedStyle(probe).getPropertyValue(property);
      probe.remove();
      return property.endsWith("color") ? window.__rgb(value) : value;
    };
  });
}

/** WCAG relative-luminance contrast between two sRGB byte triples. */
function contrast(a, b) {
  const luminance = ([r, g, b]) =>
    [r, g, b]
      .map((channel) => channel / 255)
      .map((c) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4))
      .reduce((sum, c, index) => sum + c * [0.2126, 0.7152, 0.0722][index], 0);
  const [light, dark] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (light + 0.05) / (dark + 0.05);
}

/** Tab forward. WebKit on macOS skips links on plain Tab, like Safari. */
async function tab(page, browserName) {
  await page.keyboard.press(browserName === "webkit" ? "Alt+Tab" : "Tab");
}

test.describe("classless semantic base", () => {
  test("the inventory owns subjects in every group and the base ships in the core export", () => {
    expect(new Set(OWNED.map(({ group }) => group))).toEqual(
      new Set(["document", "content", "forms", "interactive"]),
    );
    expect(BASE_SHEETS).toEqual([
      "/packages/styles/base.css",
      "/packages/styles/base/document.css",
      "/packages/styles/base/content.css",
      "/packages/styles/base/forms.css",
      "/packages/styles/base/interactive.css",
    ]);
  });

  test("base rules sit in ds.base.<group>, carry zero specificity, and declare no token or motion", async ({ page }) => {
    await page.goto("/tests/browser/probes/blank.html");
    await loadStylesheets(page, [CORE_EXPORT]);
    const rules = (await publishedRules(page, CORE_EXPORT)).filter((rule) =>
      rule.sheet.startsWith("/packages/styles/base"),
    );
    expect(rules.length).toBeGreaterThan(0);
    for (const rule of rules) {
      const group = rule.sheet.match(/\/base\/([a-z]+)\.css$/)?.[1];
      expect(group, `${rule.sheet} is a group module`).toBeTruthy();
      expect(rule.layer, rule.selector).toBe(`ds.base.${group}`);
      for (const part of rule.selector.split(/,(?![^(]*\))/)) {
        expect(part.trim(), rule.selector).toMatch(/^:where\(.*\)(::[a-z-]+)?$/);
      }
      for (const property of rule.properties) {
        expect(property, rule.selector).not.toMatch(/^--|^transition|^animation|^color-scheme$/);
      }
    }
    // The T1 order is unchanged by the base.
    const order = await page.evaluate(() => {
      const sheet = [...document.styleSheets].find((candidate) => candidate.href?.endsWith("/index.css"));
      return [...sheet.cssRules[0].nameList];
    });
    expect(order).toEqual(DS_LAYER_ORDER);
  });

  for (const state of STATES) {
    test(`owned defaults bind to semantic roles (${state.label})`, async ({ page }) => {
      await openFixture(page, state);
      await installHelpers(page);
      const bound = await page.evaluate(() => {
        const style = (selector, property) => {
          const element = document.querySelector(selector);
          const value = getComputedStyle(element).getPropertyValue(property);
          return property.endsWith("color") ? window.__rgb(value) : value;
        };
        const role = window.__role;
        return [
          ["html background", style("html", "background-color"), role("--ds-color-canvas")],
          ["html text", style("html", "color"), role("--ds-color-text")],
          ["body size", style("body", "font-size"), role("--ds-text-body", "font-size")],
          ["h1 size", style("h1", "font-size"), role("--ds-text-heading-1", "font-size")],
          ["h6 size", style("h6", "font-size"), role("--ds-text-heading-6", "font-size")],
          ["link", style("p a", "color"), role("--ds-color-accent")],
          ["mark", style("mark", "background-color"), role("--ds-color-highlight")],
          ["code surface", style("p code", "background-color"), role("--ds-color-surface")],
          ["pre surface", style("pre", "background-color"), role("--ds-color-surface")],
          ["hr rule", style("hr", "border-top-color"), role("--ds-color-border")],
          ["figcaption", style("figcaption", "color"), role("--ds-color-text-muted")],
          ["cell rule", style("td", "border-bottom-color"), role("--ds-color-border")],
          ["field box", style("#name", "border-top-color"), role("--ds-color-border")],
          ["field canvas", style("#name", "background-color"), role("--ds-color-canvas")],
          ["button surface", style("button[type=button]", "background-color"), role("--ds-color-surface")],
          ["button text", style("button[type=button]", "color"), role("--ds-color-text")],
          ["checkbox accent", style("#updates", "accent-color"), role("--ds-color-accent")],
          ["disabled text", style("#disabled", "color"), role("--ds-color-text-muted")],
          ["dialog canvas", style("dialog", "background-color"), role("--ds-color-canvas")],
          ["popover border", style("[popover]", "border-top-color"), role("--ds-color-border")],
        ];
      });
      for (const [label, actual, expected] of bound) {
        expect(actual, label).toEqual(expected);
      }
      expect(await page.evaluate(() => getComputedStyle(document.documentElement).colorScheme)).toBe(
        state.hook ?? "light dark",
      );
    });
  }

  for (const scheme of SCHEMES) {
    test(`links keep an underline and 4.5:1 against the canvas (${scheme})`, async ({ page, browserName }) => {
      await openFixture(page, { preference: scheme, hook: null });
      await installHelpers(page);
      const underlined = () =>
        page.evaluate(() =>
          [...document.querySelectorAll("a[href]")].map(
            (link) => getComputedStyle(link).textDecorationLine,
          ),
        );
      const initial = await underlined();
      expect(initial.length).toBeGreaterThan(1);
      for (const line of initial) {
        expect(line).toContain("underline");
      }
      const first = page.locator("p a").first();
      await first.hover();
      expect(await first.evaluate((link) => getComputedStyle(link).textDecorationLine)).toContain("underline");
      await first.focus();
      expect(await first.evaluate((link) => getComputedStyle(link).textDecorationLine)).toContain("underline");

      const [link, canvas, text] = await page.evaluate(() => [
        window.__rgb(getComputedStyle(document.querySelector("p a")).color),
        window.__role("--ds-color-canvas"),
        window.__role("--ds-color-text"),
      ]);
      expect(contrast(link, canvas), `${browserName} link on canvas`).toBeGreaterThanOrEqual(4.5);
      // The colour difference from body text alone is below 3:1 in dark,
      // which is why the underline is required.
      expect(contrast(link, text)).toBeLessThan(4.5);
    });
  }

  test("keyboard focus visits every interactive element in DOM order with the focus outline", async ({ page, browserName }) => {
    await openFixture(page, { preference: "light", hook: null });
    await installHelpers(page);
    // An open non-modal <dialog> is itself a tab stop in Firefox and WebKit but
    // not in Chromium, so it may be visited; every other candidate must be.
    const candidates = await page.evaluate(() => {
      const found = [
        ...document.querySelectorAll("a[href], button, input, select, textarea, summary, [tabindex], dialog[open]"),
      ].filter(
        (element) =>
          !element.disabled &&
          !(element.type === "radio" && !element.checked) &&
          element.checkVisibility(),
      );
      window.__stops = found;
      return found.map((element) => ({ optional: element.localName === "dialog" }));
    });
    const required = candidates.flatMap(({ optional }, index) => (optional ? [] : [index]));
    expect(required.length).toBeGreaterThan(10);
    const width = await page.evaluate(() => Number.parseFloat(window.__role("--ds-focus-width", "outline-width")));
    const color = await page.evaluate(() => window.__role("--ds-color-focus"));
    const visited = [];
    for (let index = 0; visited.at(-1) !== candidates.length - 1 && index < candidates.length; index += 1) {
      await tab(page, browserName);
      const stop = await page.evaluate(() => {
        const active = document.activeElement;
        const style = getComputedStyle(active);
        return {
          index: window.__stops.indexOf(active),
          visible: active.matches(":focus-visible"),
          outlineStyle: style.outlineStyle,
          outlineWidth: Number.parseFloat(style.outlineWidth),
          outlineColor: window.__rgb(style.outlineColor),
        };
      });
      visited.push(stop.index);
      expect(stop.visible, `stop ${index} is :focus-visible`).toBe(true);
      expect(stop.outlineStyle, `stop ${index}`).toBe("solid");
      expect(stop.outlineWidth, `stop ${index}`).toBeGreaterThanOrEqual(width);
      expect(stop.outlineColor, `stop ${index}`).toEqual(color);
    }
    // Strictly increasing DOM order, with every required stop present.
    expect(visited).toEqual([...visited].sort((a, b) => a - b));
    expect(new Set(visited).size).toBe(visited.length);
    expect(visited.filter((index) => !candidates[index].optional)).toEqual(required);
  });

  test("native controls stay keyboard operable", async ({ page }) => {
    await openFixture(page, { preference: "light", hook: null });

    const button = page.getByRole("button", { name: "Native button" });
    await button.evaluate((element) => {
      window.__clicks = 0;
      element.addEventListener("click", () => {
        window.__clicks += 1;
      });
    });
    await button.focus();
    await page.keyboard.press("Enter");
    await page.keyboard.press(" ");
    expect(await page.evaluate(() => window.__clicks)).toBe(2);

    await page.locator("#updates").focus();
    await page.keyboard.press(" ");
    await expect(page.locator("#updates")).toBeChecked();

    await page.locator("#reply-email").focus();
    await page.keyboard.press("ArrowDown");
    await expect(page.locator("#reply-none")).toBeChecked();

    await page.locator("#topic").focus();
    await page.keyboard.type("f");
    await expect(page.locator("#topic")).toHaveValue("Feedback");

    await page.locator("summary").focus();
    await page.keyboard.press("Enter");
    await expect(page.locator("details")).toHaveJSProperty("open", true);
    await page.keyboard.press("Enter");
    await expect(page.locator("details")).toHaveJSProperty("open", false);
  });

  test("a native disclosure keeps semantic fallback and adds token spacing only while open", async ({ page }) => {
    await openFixture(page, { preference: "light", hook: null });
    await installHelpers(page);
    const disclosure = page.locator("details");
    const summary = page.locator("summary");

    await expect(summary).toBeVisible();
    await expect(disclosure).toHaveJSProperty("open", false);
    expect(await disclosure.evaluate((element) => getComputedStyle(element).paddingBlockEnd)).toBe("0px");

    await summary.focus();
    await expect(summary).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(disclosure).toHaveJSProperty("open", true);
    const spacing = await disclosure.evaluate((element) => ({
      actual: getComputedStyle(element).paddingBlockEnd,
      expected: window.__role("--ds-space-control-block", "padding-block-end"),
    }));
    expect(spacing.actual).toBe(spacing.expected);

    await page.keyboard.press("Enter");
    await expect(disclosure).toHaveJSProperty("open", false);
    expect(await disclosure.evaluate((element) => getComputedStyle(element).paddingBlockEnd)).toBe("0px");
  });

  test("a modal dialog takes focus, closes on Escape, and returns focus", async ({ page }) => {
    await openFixture(page, { preference: "light", hook: null });
    const opener = page.getByRole("button", { name: "Native button" });
    await opener.focus();
    await page.evaluate(() => {
      const dialog = document.querySelector("dialog");
      dialog.close();
      dialog.showModal();
    });
    expect(await page.evaluate(() => document.querySelector("dialog").contains(document.activeElement))).toBe(true);
    await page.keyboard.press("Escape");
    await expect(page.locator("dialog")).toHaveJSProperty("open", false);
    expect(await opener.evaluate((element) => document.activeElement === element)).toBe(true);
  });

  test("a popover toggles from the keyboard where supported and stays readable where not", async ({ page }) => {
    await openFixture(page, { preference: "light", hook: null });
    const supported = await page.evaluate(() => Object.hasOwn(HTMLElement.prototype, "popover"));
    const note = page.locator("[popover]");
    if (!supported) {
      await expect(note).toBeVisible();
      return;
    }
    await expect(note).toBeHidden();
    await page.locator("[popovertarget]").focus();
    await page.keyboard.press("Enter");
    await expect(note).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(note).toBeHidden();
  });

  // The fluid control padding and body text already clear the minimum on wide
  // viewports, so the narrowest viewport is where min-block-size must bind.
  for (const width of [320, 1280]) {
    test(`controls meet the minimum target size at ${width} CSS pixels`, async ({ page }) => {
      await page.setViewportSize({ width, height: 800 });
      await openFixture(page, { preference: "light", hook: null });
      await installHelpers(page);
      const result = await page.evaluate(() => {
        const target = Number.parseFloat(window.__role("--ds-size-target-min", "min-height"));
        const controls = [
          ...document.querySelectorAll("button, input[type=text], input[type=email], input[type=reset], select, textarea"),
        ].filter((element) => element.checkVisibility());
        return {
          target,
          heights: controls.map((element) => [element.outerHTML.slice(0, 50), element.getBoundingClientRect().height]),
        };
      });
      expect(result.target).toBeGreaterThan(40);
      for (const [label, height] of result.heights) {
        expect(height, label).toBeGreaterThanOrEqual(result.target - 0.5);
      }
    });
  }

  test("the fixture reflows at 320 CSS pixels without horizontal page scrolling", async ({ page }) => {
    await page.setViewportSize({ width: 320, height: 800 });
    await openFixture(page, { preference: "light", hook: null });
    const { scrollWidth, clientWidth } = await page.evaluate(() => ({
      scrollWidth: document.documentElement.scrollWidth,
      clientWidth: document.documentElement.clientWidth,
    }));
    expect(scrollWidth).toBeLessThanOrEqual(clientWidth);
  });

  test("consumer layer and unlayered rules win over the base with plain element selectors", async ({ page }) => {
    await openFixture(page, { preference: "light", hook: null });
    await loadStylesheets(page, ["/tests/browser/probes/base-override.css"]);
    const link = page.locator("p a").first();
    await expect(link).toHaveCSS("color", "rgb(0, 100, 0)");
    await expect(link).toHaveCSS("text-decoration-line", "none");
    await expect(page.getByRole("button", { name: "Native button" })).toHaveCSS("background-color", "rgb(0, 0, 100)");
    await expect(page.locator("h1")).toHaveCSS("font-size", "12px");

    await loadStylesheets(page, ["/tests/browser/probes/base-unlayered.css"]);
    await expect(link).toHaveCSS("color", "rgb(100, 0, 0)");
    await expect(page.locator("#name")).toHaveCSS("border-top-color", "rgb(0, 100, 100)");
  });

  test("the base adds no transition or animation", async ({ page }) => {
    await openFixture(page, { preference: "light", hook: null });
    const moving = await page.evaluate(() =>
      [...document.querySelectorAll("*")]
        .map((element) => [element.tagName, getComputedStyle(element)])
        .filter(
          ([, style]) =>
            style.animationName !== "none" ||
            style.transitionDuration.split(",").some((value) => Number.parseFloat(value) > 0),
        )
        .map(([tag]) => tag),
    );
    expect(moving).toEqual([]);
  });

  for (const state of STATES.slice(0, 3)) {
    test(`axe-core finds no violation on the fixture (${state.label})`, async ({ page }) => {
      await openFixture(page, state);
      // preload: false keeps axe from re-fetching @import-ed sheets against the
      // page URL; the served stylesheets are already in the CSSOM.
      const results = await new AxeBuilder({ page })
        .options({ preload: false })
        .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa", "best-practice"])
        .analyze();
      expect(
        results.violations.map(({ id, nodes }) => `${id}: ${nodes.map((node) => node.target).join(" | ")}`),
      ).toEqual([]);
      expect(results.passes.length).toBeGreaterThan(20);
    });
  }
});
