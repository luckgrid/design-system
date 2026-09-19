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

/** The promoted layouts in layouts.tsv, in sub-layer order. */
const LAYOUTS = readFileSync(path.join(root, "layouts.tsv"), "utf8")
  .split("\n")
  .filter((line) => line.startsWith("public-preview\t"))
  .map((line) => line.split("\t")[1]);

const FIXTURE = "/fixtures/layouts/index.html";
const LAYOUT_SHEETS = PUBLISHED_STYLESHEETS.filter((href) => href.includes("/packages/styles/layouts"));
const WIDTHS = [320, 640, 768, 1280];
const HOOKS = ".ds-stack, .ds-cluster, .ds-grid";

async function openFixture(page, { width = 1280, scheme = "light" } = {}) {
  await page.setViewportSize({ width, height: 900 });
  await page.emulateMedia({ colorScheme: scheme });
  await page.goto(FIXTURE);
}

/** WebKit on macOS skips links on plain Tab, like Safari. */
async function tab(page, browserName) {
  await page.keyboard.press(browserName === "webkit" ? "Alt+Tab" : "Tab");
}

/**
 * Every place where a layout child is painted before an earlier sibling in the
 * reading direction: on an earlier line, or earlier on the same line. Lines are
 * found by vertical overlap, so centered clusters and stretched grid rows read
 * as one line. An empty result means visual order equals source order.
 */
async function orderViolations(page) {
  return page.evaluate((hooks) => {
    const violations = [];
    for (const container of document.querySelectorAll(hooks)) {
      const rtl = getComputedStyle(container).direction === "rtl";
      const boxes = [...container.children]
        .map((child) => child.getBoundingClientRect())
        .filter((box) => box.width > 0 || box.height > 0);
      for (let index = 1; index < boxes.length; index += 1) {
        const previous = boxes[index - 1];
        const next = boxes[index];
        const sameLine = next.top < previous.bottom - 0.5 && next.bottom > previous.top + 0.5;
        const inOrder = sameLine
          ? rtl
            ? next.right <= previous.left + 0.5
            : next.left >= previous.right - 0.5
          : next.top >= previous.bottom - 0.5;
        if (!inOrder) {
          violations.push(`#${container.id || container.tagName.toLowerCase()} child ${index + 1}`);
        }
      }
    }
    return violations;
  }, HOOKS);
}

/** Resolve a public role to pixels on a probe, as a layout would read it. */
async function rolePixels(page, name) {
  return page.evaluate((role) => {
    const probe = document.createElement("div");
    probe.style.rowGap = `var(${role})`;
    document.body.append(probe);
    const value = getComputedStyle(probe).rowGap;
    probe.remove();
    return value;
  }, name);
}

function columnCount(template) {
  return template.trim().split(/\s+/).length;
}

test.describe("layout primitives", () => {
  test("the inventory promotes stack, cluster, and grid, and they ship in the core export", () => {
    expect(LAYOUTS).toEqual(["stack", "cluster", "grid"]);
    expect(LAYOUT_SHEETS).toEqual([
      "/packages/styles/layouts.css",
      "/packages/styles/layouts/stack.css",
      "/packages/styles/layouts/cluster.css",
      "/packages/styles/layouts/grid.css",
    ]);
  });

  test("layout rules sit in ds.layouts.<layout>, carry zero specificity, and cannot reorder", async ({ page }) => {
    await page.goto("/tests/browser/probes/blank.html");
    await loadStylesheets(page, [CORE_EXPORT]);
    const rules = (await publishedRules(page, CORE_EXPORT)).filter((rule) =>
      rule.sheet.startsWith("/packages/styles/layouts"),
    );
    expect(rules.length).toBe(LAYOUTS.length * 2);
    for (const rule of rules) {
      const layout = rule.sheet.match(/\/layouts\/([a-z]+)\.css$/)?.[1];
      expect(LAYOUTS, `${rule.sheet} is a layout module`).toContain(layout);
      expect(rule.layer, rule.selector).toBe(`ds.layouts.${layout}`);
      expect([`:where(.ds-${layout})`, `:where(.ds-${layout}) > :where(*)`]).toContain(rule.selector);
      for (const property of rule.properties) {
        expect(property, rule.selector).not.toMatch(
          /^--|^order$|^float$|^position$|^grid-(row|column|area|auto-flow)|^transition|^animation/,
        );
      }
    }
    const order = await page.evaluate(() => {
      const sheet = [...document.styleSheets].find((candidate) => candidate.href?.endsWith("/index.css"));
      return [...sheet.cssRules[0].nameList];
    });
    expect(order).toEqual(DS_LAYER_ORDER);
  });

  test("each hook computes its documented model and binds its gaps to public roles", async ({ page }) => {
    await openFixture(page);
    const flow = await rolePixels(page, "--ds-space-flow");
    const controlBlock = await rolePixels(page, "--ds-space-control-block");
    const controlInline = await rolePixels(page, "--ds-space-control-inline");

    const stack = page.locator("#stack");
    await expect(stack).toHaveCSS("display", "flex");
    await expect(stack).toHaveCSS("flex-direction", "column");
    await expect(stack).toHaveCSS("row-gap", flow);

    const cluster = page.locator("#tags");
    await expect(cluster).toHaveCSS("display", "flex");
    await expect(cluster).toHaveCSS("flex-direction", "row");
    await expect(cluster).toHaveCSS("flex-wrap", "wrap");
    await expect(cluster).toHaveCSS("align-items", "center");
    await expect(cluster).toHaveCSS("row-gap", controlBlock);
    await expect(cluster).toHaveCSS("column-gap", controlInline);

    const grid = page.locator("#cards");
    await expect(grid).toHaveCSS("display", "grid");
    await expect(grid).toHaveCSS("row-gap", flow);
    await expect(grid).toHaveCSS("column-gap", flow);

    // The layouts outrank the base: base block margins on direct children are gone.
    for (const selector of ["#stack > p", "#stack > blockquote", "#cards > li", "#page > h1"]) {
      const margins = await page.locator(selector).first().evaluate((element) => {
        const style = getComputedStyle(element);
        return [style.marginBlockStart, style.marginBlockEnd];
      });
      expect(margins, selector).toEqual(["0px", "0px"]);
    }
    // A grandchild keeps its base margins; only direct children are reset.
    const nested = await page
      .locator("#stack > blockquote > p")
      .evaluate((element) => getComputedStyle(element).marginBlockEnd);
    // WebKit rounds used margins to its 1/64 px layout unit but reports gaps unrounded.
    expect(Number.parseFloat(nested)).toBeCloseTo(Number.parseFloat(flow), 1);
  });

  test("the grid fits columns to its space without a breakpoint", async ({ page }) => {
    await openFixture(page, { width: 320 });
    const narrow = await page.locator("#cards").evaluate((element) => getComputedStyle(element).gridTemplateColumns);
    expect(columnCount(narrow)).toBe(1);

    await openFixture(page, { width: 1280 });
    const wide = await page.locator("#cards").evaluate((element) => getComputedStyle(element).gridTemplateColumns);
    expect(columnCount(wide)).toBeGreaterThanOrEqual(3);

    // One item keeps its column width; empty tracks are kept.
    const single = await page.locator("#single").evaluate((element) => ({
      container: element.getBoundingClientRect().width,
      item: element.firstElementChild.getBoundingClientRect().width,
      columns: getComputedStyle(element).gridTemplateColumns,
    }));
    expect(columnCount(single.columns)).toBeGreaterThanOrEqual(3);
    expect(single.item).toBeLessThan(single.container / 2);

    // A grid with no children is valid and takes no space.
    const empty = await page.locator("#empty").evaluate((element) => element.getBoundingClientRect().height);
    expect(empty).toBe(0);
  });

  for (const width of WIDTHS) {
    test(`visual order follows source order at ${width} CSS pixels, left to right and right to left`, async ({ page }) => {
      await openFixture(page, { width });
      expect(await orderViolations(page)).toEqual([]);
      // Both directions are exercised.
      const directions = await page.evaluate((hooks) =>
        [...new Set([...document.querySelectorAll(hooks)].map((element) => getComputedStyle(element).direction))].sort(),
      HOOKS);
      expect(directions).toEqual(["ltr", "rtl"]);
    });

    test(`the fixture reflows at ${width} CSS pixels with no horizontal overflow`, async ({ page }) => {
      await openFixture(page, { width });
      const overflow = await page.evaluate((hooks) => {
        const page = document.documentElement;
        const found = page.scrollWidth > page.clientWidth ? [`page ${page.scrollWidth} > ${page.clientWidth}`] : [];
        for (const element of document.querySelectorAll(hooks)) {
          if (element.scrollWidth > element.clientWidth + 1) {
            found.push(`#${element.id || element.tagName.toLowerCase()} ${element.scrollWidth} > ${element.clientWidth}`);
          }
          for (const child of element.children) {
            if (child.getBoundingClientRect().right > element.getBoundingClientRect().right + 1) {
              found.push(`#${element.id || element.tagName.toLowerCase()} child escapes`);
            }
          }
        }
        return found;
      }, HOOKS);
      expect(overflow).toEqual([]);
    });
  }

  test("the source-order check detects a consumer that reorders children", async ({ page }) => {
    await openFixture(page);
    await loadStylesheets(page, ["/tests/browser/probes/layouts-reorder.css"]);
    const violations = await orderViolations(page);
    expect(violations).toEqual(expect.arrayContaining([expect.stringMatching(/^#tags /), expect.stringMatching(/^#cards /)]));
  });

  test("keyboard focus visits every control in DOM order", async ({ page, browserName }) => {
    await openFixture(page);
    const expected = await page.evaluate(() =>
      [...document.querySelectorAll("a[href], button")].map((element, index) => {
        element.dataset.focusIndex = String(index);
        return String(index);
      }),
    );
    expect(expected.length).toBeGreaterThan(30);
    const visited = [];
    for (let step = 0; step < expected.length; step += 1) {
      await tab(page, browserName);
      visited.push(await page.evaluate(() => document.activeElement?.dataset.focusIndex ?? "none"));
    }
    expect(visited).toEqual(expected);
  });

  test("nested layouts arrange only their own children", async ({ page }) => {
    await openFixture(page);
    const nested = await page.locator("#nested-grid").evaluate((grid) => {
      const cells = [...grid.children];
      return {
        display: getComputedStyle(grid).display,
        cells: cells.map((cell) => ({
          display: getComputedStyle(cell).display,
          direction: getComputedStyle(cell).flexDirection,
          cluster: getComputedStyle(cell.querySelector(".ds-cluster")).flexWrap,
        })),
        sameTop: cells.every((cell) => cell.getBoundingClientRect().top === cells[0].getBoundingClientRect().top),
      };
    });
    expect(nested.display).toBe("grid");
    expect(nested.sameTop).toBe(true);
    for (const cell of nested.cells) {
      expect(cell).toEqual({ display: "flex", direction: "column", cluster: "wrap" });
    }
  });

  test("consumer rules override a layout's own properties, layered and unlayered", async ({ page }) => {
    await openFixture(page);
    // The fixture's own consumer class, in its `app` layer.
    const tight = page.locator("#overridden");
    await expect(tight).toHaveCSS("row-gap", "4px");
    await expect(tight).toHaveCSS("column-gap", "4px");
    const columns = await tight.evaluate((element) => getComputedStyle(element).gridTemplateColumns);
    expect(columnCount(columns)).toBe(2);

    // Plain element selectors in a consumer layer, with no Design System class.
    await loadStylesheets(page, ["/tests/browser/probes/layouts-override.css"]);
    const cards = page.locator("#cards");
    await expect(cards).toHaveCSS("row-gap", "3px");
    const overridden = await cards.evaluate((element) => getComputedStyle(element).gridTemplateColumns);
    expect(columnCount(overridden)).toBe(3);
    await expect(page.locator("#stack")).toHaveCSS("row-gap", "5px");
    await expect(cards).toHaveCSS("display", "grid");

    // Unlayered consumer CSS wins too.
    await loadStylesheets(page, ["/tests/browser/probes/layouts-unlayered.css"]);
    const nav = page.locator("#nav");
    await expect(nav).toHaveCSS("flex-wrap", "nowrap");
    await expect(nav).toHaveCSS("column-gap", "7px");
    await expect(nav).toHaveCSS("display", "flex");
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
