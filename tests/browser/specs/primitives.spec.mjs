import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import AxeBuilder from "@axe-core/playwright";

import {
  CORE_EXPORT,
  PUBLISHED_STYLESHEETS,
  expect,
  loadStylesheets,
  publishedRules,
  test,
} from "./support.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..");

/** The promoted rows of primitives.tsv: name and kind, in inventory order. */
const PROMOTED = readFileSync(path.join(root, "primitives.tsv"), "utf8")
  .split("\n")
  .filter((line) => line.startsWith("public-preview\t"))
  .map((line) => {
    const [, name, kind] = line.split("\t");
    return { name, kind };
  });
const PRIMITIVES = PROMOTED.filter(({ kind }) => kind.endsWith("-primitive")).map(({ name }) => name);

const FIXTURE = "/fixtures/primitives/index.html";
const PRIMITIVE_SHEETS = PUBLISHED_STYLESHEETS.filter((href) => href.includes("/packages/styles/primitives"));
const TRANSPARENT = [0, 0, 0, 0];

async function openFixture(page, { width = 1280, scheme = "light" } = {}) {
  await page.setViewportSize({ width, height: 900 });
  await page.emulateMedia({ colorScheme: scheme });
  await page.goto(FIXTURE);
  await installHelpers(page);
}

/**
 * Install helpers in the page: `__rgb(color)` converts any CSS color to sRGB
 * bytes through a canvas; `__role(name, property)` resolves a role on a probe;
 * `__style(selector)` reads the computed values a test compares.
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
    window.__style = (selector) => {
      const element = document.querySelector(selector);
      const style = getComputedStyle(element);
      const box = element.getBoundingClientRect();
      return {
        display: style.display,
        parentDisplay: getComputedStyle(element.parentElement).display,
        color: window.__rgb(style.color),
        background: window.__rgb(style.backgroundColor),
        border: window.__rgb(style.borderBlockStartColor),
        borderWidth: style.borderBlockStartWidth,
        borderStyle: style.borderBlockStartStyle,
        radius: style.borderStartStartRadius,
        paddingBlock: Number.parseFloat(style.paddingBlockStart),
        paddingInline: Number.parseFloat(style.paddingInlineStart),
        weight: style.fontWeight,
        cursor: style.cursor,
        decoration: style.textDecorationLine,
        width: box.width,
        height: box.height,
      };
    };
  });
}

async function roles(page) {
  return page.evaluate(() => ({
    canvas: window.__role("--ds-color-canvas"),
    surface: window.__role("--ds-color-surface"),
    text: window.__role("--ds-color-text"),
    muted: window.__role("--ds-color-text-muted"),
    border: window.__role("--ds-color-border"),
    accent: window.__role("--ds-color-accent"),
    onAccent: window.__role("--ds-color-on-accent"),
    highlight: window.__role("--ds-color-highlight"),
    // Fluid lengths compare as numbers: engines round them differently.
    flow: Number.parseFloat(window.__role("--ds-space-flow", "padding-top")),
    controlBlock: Number.parseFloat(window.__role("--ds-space-control-block", "padding-top")),
    controlInline: Number.parseFloat(window.__role("--ds-space-control-inline", "padding-top")),
    // Lengths resolve through padding: a border width computes to 0 without a style.
    borderWidth: window.__role("--ds-border-width", "padding-top"),
    radius: window.__role("--ds-radius-control", "border-top-left-radius"),
    target: Number.parseFloat(window.__role("--ds-size-target-min", "min-height")),
    strong: window.__role("--ds-weight-strong", "font-weight"),
    body: window.__role("--ds-weight-body", "font-weight"),
  }));
}

const style = (page, selector) => page.evaluate((target) => window.__style(target), selector);

/** Tab forward. WebKit on macOS skips links on plain Tab, like Safari. */
async function tab(page, browserName) {
  await page.keyboard.press(browserName === "webkit" ? "Alt+Tab" : "Tab");
}

/** Count click events on the element with `id`. */
async function countClicks(page, id) {
  await page.evaluate((target) => {
    window.__clicks = 0;
    document.getElementById(target).addEventListener("click", () => {
      window.__clicks += 1;
    });
  }, id);
}

test.describe("primitives", () => {
  test("the inventory promotes the surface base primitive and the action UI primitive, and they ship in the core export", () => {
    expect(PROMOTED).toEqual([
      { name: "surface", kind: "base-primitive" },
      { name: "action", kind: "ui-primitive" },
      { name: "action-primary", kind: "variant" },
      { name: "action-quiet", kind: "variant" },
      { name: "action-icon", kind: "variant" },
      { name: "action-hover", kind: "state" },
      { name: "action-current", kind: "state" },
      { name: "action-disabled", kind: "state" },
      { name: "action-unlinked", kind: "state" },
    ]);
    expect(PRIMITIVE_SHEETS).toEqual([
      "/packages/styles/primitives.css",
      "/packages/styles/primitives/surface.css",
      "/packages/styles/primitives/action.css",
    ]);
  });

  test("primitive rules sit in ds.primitives.<name>, carry zero specificity, and keep state and focus native", async ({ page }) => {
    await page.goto("/tests/browser/probes/blank.html");
    await loadStylesheets(page, [CORE_EXPORT]);
    const rules = (await publishedRules(page, CORE_EXPORT)).filter((rule) =>
      rule.sheet.startsWith("/packages/styles/primitives"),
    );
    expect(rules.map((rule) => rule.selector)).toEqual([
      ":where(.ds-surface)",
      ":where(.ds-action)",
      ":where(.ds-action.ds-action-primary)",
      ":where(.ds-action.ds-action-quiet)",
      ":where(.ds-action.ds-action-icon)",
      ":where(.ds-action:hover)",
      ':where(.ds-action[aria-current]:not([aria-current=""], [aria-current="false" i]))',
      ":where(.ds-action:disabled)",
      ":where(a.ds-action:not(:any-link))",
    ]);
    for (const rule of rules) {
      const primitive = rule.sheet.match(/\/primitives\/([a-z]+)\.css$/)?.[1];
      expect(PRIMITIVES, `${rule.sheet} is a primitive module`).toContain(primitive);
      expect(rule.layer, rule.selector).toBe(`ds.primitives.${primitive}`);
      expect(rule.selector).not.toContain("data-");
      for (const property of rule.properties) {
        expect(property, rule.selector).not.toMatch(
          /^--|^outline|^pointer-events$|^opacity$|^visibility$|^position$|^order$|^transition|^animation/,
        );
      }
    }
    // The base primitive declares no layout property, so it composes with any layout.
    const surface = rules.find((rule) => rule.selector === ":where(.ds-surface)");
    for (const property of surface.properties) {
      expect(property).not.toMatch(/^(display|gap|row-gap|column-gap|flex|grid|align|justify|margin|min-)/);
    }
  });

  for (const scheme of ["light", "dark"]) {
    test(`the surface binds its relationship to public roles and composes with a layout (${scheme})`, async ({ page }) => {
      await openFixture(page, { scheme });
      const role = await roles(page);
      for (const selector of ["#card", "#plain-surface", "#cards > li:first-child"]) {
        const surface = await style(page, selector);
        expect(surface.background, selector).toEqual(role.surface);
        expect(surface.color, selector).toEqual(role.text);
        expect(surface.border, selector).toEqual(role.border);
        expect(surface.borderStyle, selector).toBe("solid");
        expect(surface.borderWidth, selector).toBe(role.borderWidth);
        expect(surface.radius, selector).toBe(role.radius);
        expect(surface.paddingBlock, selector).toBeCloseTo(role.flow, 1);
        expect(surface.paddingInline, selector).toBeCloseTo(role.flow, 1);
      }
      // The stack on the same element keeps its model and its child margin reset.
      await expect(page.locator("#card")).toHaveCSS("display", "flex");
      await expect(page.locator("#card")).toHaveCSS("flex-direction", "column");
      await expect(page.locator("#card > h3")).toHaveCSS("margin-block-start", "0px");
      await expect(page.locator("#rtl")).toHaveCSS("flex-wrap", "wrap");
      // Without a layout the surface leaves display and child margins alone.
      await expect(page.locator("#plain-surface")).toHaveCSS("display", "block");
      const margin = await page
        .locator("#plain-surface > p")
        .evaluate((element) => Number.parseFloat(getComputedStyle(element).marginBlockEnd));
      expect(margin).toBeCloseTo(role.flow, 1);
    });

    test(`each action variant computes its documented roles (${scheme})`, async ({ page }) => {
      await openFixture(page, { scheme });
      const role = await roles(page);
      for (const selector of ["#default", "#link-action"]) {
        const action = await style(page, selector);
        // A flex item is blockified, so an action in a cluster computes to `flex`.
        expect(action.display, selector).toBe(action.parentDisplay === "flex" ? "flex" : "inline-flex");
        expect(action.background, selector).toEqual(role.surface);
        expect(action.color, selector).toEqual(role.text);
        expect(action.border, selector).toEqual(role.border);
        expect(action.borderWidth, selector).toBe(role.borderWidth);
        expect(action.radius, selector).toBe(role.radius);
        expect(action.paddingBlock, selector).toBeCloseTo(role.controlBlock, 1);
        expect(action.paddingInline, selector).toBeCloseTo(role.controlInline, 1);
        expect(action.decoration, selector).toBe("none");
        expect(action.cursor, selector).toBe("pointer");
      }
      for (const selector of ["#primary", "#link-primary"]) {
        const primary = await style(page, selector);
        expect(primary.background, selector).toEqual(role.accent);
        expect(primary.border, selector).toEqual(role.accent);
        expect(primary.color, selector).toEqual(role.onAccent);
      }
      const quiet = await style(page, "#quiet");
      expect(quiet.background).toEqual(TRANSPARENT);
      expect(quiet.border).toEqual(TRANSPARENT);
      expect(quiet.borderWidth).toBe(role.borderWidth);
      expect(quiet.color).toEqual(role.text);
      const icon = await style(page, "#icon");
      expect(icon.paddingBlock).toBe(0);
      expect(icon.paddingInline).toBe(0);
      expect(icon.width).toBeCloseTo(icon.height, 0);
      expect(icon.width).toBeGreaterThanOrEqual(role.target - 0.5);
    });

    test(`native and ARIA states compute their documented roles (${scheme})`, async ({ page }) => {
      await openFixture(page, { scheme });
      const role = await roles(page);
      for (const selector of ["#current", "#current-quiet"]) {
        const current = await style(page, selector);
        expect(current.background, selector).toEqual(role.highlight);
        expect(current.border, selector).toEqual(role.accent);
        expect(current.color, selector).toEqual(role.text);
        expect(current.weight, selector).toBe(role.strong);
      }
      const notCurrent = await style(page, "#not-current");
      expect(notCurrent.background).toEqual(role.surface);
      expect(notCurrent.weight).toBe(role.body);
      for (const selector of ["#disabled", "#disabled-primary", "#unlinked"]) {
        const unavailable = await style(page, selector);
        expect(unavailable.background, selector).toEqual(role.surface);
        expect(unavailable.border, selector).toEqual(role.border);
        expect(unavailable.color, selector).toEqual(role.muted);
        expect(unavailable.cursor, selector).toBe("not-allowed");
      }
      // Hover is a pointer state: the border turns to the accent.
      await page.locator("#default").hover();
      expect((await style(page, "#default")).border).toEqual(role.accent);
      await page.locator("#disabled").hover({ force: true });
      expect((await style(page, "#disabled")).border).toEqual(role.border);
    });
  }

  test("the explicit data-ds-scheme hook drives the primitives like the preference does", async ({ page }) => {
    await openFixture(page, { scheme: "light" });
    await page.evaluate(() => {
      document.documentElement.dataset.dsScheme = "dark";
    });
    const role = await roles(page);
    expect((await style(page, "#primary")).background).toEqual(role.accent);
    expect((await style(page, "#card")).background).toEqual(role.surface);
    await page.emulateMedia({ colorScheme: "dark" });
    const dark = await roles(page);
    expect(role.accent).toEqual(dark.accent);
    expect(role.surface).toEqual(dark.surface);
  });

  test("a button action activates from Enter and Space; a disabled one never activates", async ({ page }) => {
    await openFixture(page);
    await countClicks(page, "default");
    await page.locator("#default").focus();
    await page.keyboard.press("Enter");
    await page.keyboard.press("Space");
    await page.locator("#default").click();
    expect(await page.evaluate(() => window.__clicks)).toBe(3);

    await countClicks(page, "disabled");
    // A user click on a disabled button never reaches its listeners.
    await page.locator("#disabled").click({ force: true });
    await page.locator("#disabled").focus();
    await page.keyboard.press("Enter");
    await page.keyboard.press("Space");
    expect(await page.evaluate(() => window.__clicks)).toBe(0);
    expect(await page.locator("#disabled").evaluate((element) => element.matches(":disabled"))).toBe(true);
  });

  test("a primary action submits its form from the keyboard", async ({ page }) => {
    await openFixture(page);
    await page.evaluate(() => {
      window.__submits = 0;
      document.getElementById("form").addEventListener("submit", (event) => {
        event.preventDefault();
        window.__submits += 1;
      });
    });
    await page.locator("#submit").focus();
    await page.keyboard.press("Enter");
    await page.keyboard.press("Space");
    expect(await page.evaluate(() => window.__submits)).toBe(2);
  });

  test("a link action follows its link from Enter; an unlinked one is not a link", async ({ page }) => {
    await openFixture(page);
    await page.locator("#link-action").focus();
    await page.keyboard.press("Enter");
    await expect(page).toHaveURL(/#surfaces$/);
    const unlinked = await page.locator("#unlinked").evaluate((element) => {
      element.focus();
      return {
        link: element.matches(":any-link"),
        focusable: document.activeElement === element,
        role: element.getAttribute("role"),
      };
    });
    expect(unlinked).toEqual({ link: false, focusable: false, role: null });
    await page.locator("#unlinked").click({ force: true });
    await expect(page).toHaveURL(/#surfaces$/);
  });

  test("keyboard focus reaches every available action in DOM order with the base outline and skips unavailable ones", async ({ page, browserName }) => {
    await openFixture(page);
    const expected = await page.evaluate(() => {
      const stops = [
        ...document.querySelectorAll("a[href], button, input"),
      ].filter((element) => !element.disabled);
      window.__stops = stops;
      return stops.map((element) => element.id || element.localName);
    });
    const width = await page.evaluate(() => Number.parseFloat(window.__role("--ds-focus-width", "outline-width")));
    const color = await page.evaluate(() => window.__role("--ds-color-focus"));
    const visited = [];
    for (let index = 0; index < expected.length; index += 1) {
      await tab(page, browserName);
      const stop = await page.evaluate(() => {
        const active = document.activeElement;
        const computed = getComputedStyle(active);
        return {
          id: active.id || active.localName,
          visible: active.matches(":focus-visible"),
          action: active.classList.contains("ds-action"),
          outlineStyle: computed.outlineStyle,
          outlineWidth: Number.parseFloat(computed.outlineWidth),
          outlineColor: window.__rgb(computed.outlineColor),
        };
      });
      visited.push(stop.id);
      if (stop.action) {
        expect(stop.visible, stop.id).toBe(true);
        expect(stop.outlineStyle, stop.id).toBe("solid");
        expect(stop.outlineWidth, stop.id).toBeGreaterThanOrEqual(width);
        expect(stop.outlineColor, stop.id).toEqual(color);
      }
    }
    expect(visited).toEqual(expected);
    for (const skipped of ["disabled", "disabled-primary", "unlinked"]) {
      expect(visited).not.toContain(skipped);
    }
  });

  for (const width of [320, 1280]) {
    test(`every action meets the target size and the page reflows at ${width} CSS pixels`, async ({ page }) => {
      await openFixture(page, { width });
      const role = await roles(page);
      const small = await page.evaluate((target) => {
        return [...document.querySelectorAll(".ds-action")]
          .map((element) => ({ id: element.id || element.textContent.trim(), box: element.getBoundingClientRect() }))
          .filter(({ box }) => box.width < target - 0.5 || box.height < target - 0.5)
          .map(({ id, box }) => `${id} ${box.width}x${box.height}`);
      }, role.target);
      expect(small).toEqual([]);
      const overflow = await page.evaluate(() => {
        const page = document.documentElement;
        const found = page.scrollWidth > page.clientWidth ? [`page ${page.scrollWidth} > ${page.clientWidth}`] : [];
        for (const element of document.querySelectorAll(".ds-surface, .ds-action")) {
          if (element.scrollWidth > element.clientWidth + 1) {
            found.push(`#${element.id || element.localName} ${element.scrollWidth} > ${element.clientWidth}`);
          }
        }
        return found;
      });
      expect(overflow).toEqual([]);
    });
  }

  test("consumer classes override a primitive by setting its real property", async ({ page }) => {
    await openFixture(page);
    const role = await roles(page);
    await expect(page.locator("#pill")).toHaveCSS("border-start-start-radius", "999px");
    await expect(page.locator("#pill")).toHaveCSS("padding-inline-start", "32px");
    const flat = await style(page, "#flat-surface");
    expect(flat.border).toEqual(TRANSPARENT);
    expect(flat.background).toEqual(role.canvas);
  });

  test("a later consumer layer wins over every primitive rule, including variants and states", async ({ page }) => {
    await openFixture(page);
    await loadStylesheets(page, ["/tests/browser/probes/primitives-override.css"]);
    for (const selector of ["#primary", "#disabled-primary", "#current", "#unlinked"]) {
      await expect(page.locator(selector), selector).toHaveCSS("background-color", "rgb(1, 2, 3)");
    }
    await expect(page.locator("#card")).toHaveCSS("padding-inline-start", "3px");
  });

  test("unlayered consumer rules win over every primitive rule", async ({ page }) => {
    await openFixture(page);
    await loadStylesheets(page, ["/tests/browser/probes/primitives-unlayered.css"]);
    for (const selector of ["#link-primary", "#current", "#unlinked"]) {
      await expect(page.locator(selector), selector).toHaveCSS("color", "rgb(4, 5, 6)");
    }
    await expect(page.locator("#card")).toHaveCSS("border-top-color", "rgb(4, 5, 6)");
  });

  for (const scheme of ["light", "dark"]) {
    test(`axe-core finds no violation on the fixture (${scheme})`, async ({ page }) => {
      await openFixture(page, { scheme });
      // preload: false keeps axe from re-fetching @import-ed sheets against the
      // page URL; the served stylesheets are already in the CSSOM.
      const results = await new AxeBuilder({ page })
        .options({ preload: false })
        .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa", "best-practice"])
        .analyze();
      expect(
        results.violations.map(({ id, nodes }) => `${id}: ${nodes.map((node) => node.target).join(" | ")}`),
      ).toEqual([]);
      expect(results.passes.length).toBeGreaterThan(15);
    });
  }
});
