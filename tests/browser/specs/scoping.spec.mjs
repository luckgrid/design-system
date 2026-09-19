import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import AxeBuilder from "@axe-core/playwright";

import { CORE_EXPORT, expect, loadStylesheets, publishedRules, test } from "./support.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..");

function promoted(file) {
  return readFileSync(path.join(root, file), "utf8")
    .split("\n")
    .filter((line) => line.startsWith("public-preview\t"))
    .map((line) => line.split("\t"));
}

/** Every public class hook, from the layout and primitive inventories. */
const CLASS_HOOKS = [
  ...promoted("layouts.tsv").map(([, name]) => `ds-${name}`),
  ...promoted("primitives.tsv")
    .filter(([, , kind]) => kind !== "state")
    .map(([, name]) => `ds-${name}`),
];
/** The one attribute hook, from the theme inventory. */
const ATTRIBUTE_HOOKS = promoted("theme.tsv")
  .filter(([, kind]) => kind === "attribute")
  .map(([, , name]) => name);
const LAYOUT_HOOKS = promoted("layouts.tsv").map(([, name]) => `ds-${name}`);
/** Each variant hook with the primitive hook it refines. */
const VARIANTS = promoted("primitives.tsv")
  .filter(([, , kind]) => kind === "variant")
  .map(([, name]) => [`ds-${name}`, `ds-${name.split("-")[0]}`]);

/**
 * The characters of `selector` that sit directly inside its outermost
 * parentheses, with bracket and deeper contents dropped: for
 * `:where(nav .x:not(.y))` that is `nav .x:not`.
 */
function firstLevel(selector) {
  let depth = 0;
  let found = "";
  for (const character of selector) {
    if (character === ")" || character === "]") depth -= 1;
    if (depth === 1 && character !== "(" && character !== "[") found += character;
    if (character === "(" || character === "[") depth += 1;
  }
  return found;
}

const FIXTURE = "/fixtures/scoping/index.html";

/** Elements that only look like hooks: each must compute like its hookless copy. */
const LOOKALIKES = ["#variant-alone", "#quiet-alone", "#icon-alone", "#prefixed", "#infixed", "#data-lookalike", "#ds-action", "#data-layout"];

/**
 * Elements inside hooked or lookalike markup that no hook reaches, each with a
 * reference element in the same position outside every hook. A copy would
 * share the element's ancestors, so an ancestor that leaks needs a reference.
 */
const DESCENDANTS = {
  "#grandchild": "#plain-panel > p",
  "#data-layout-text": "#plain-panel > p",
};

/** Computed properties that a hook sets on its element or a layout on a child. */
const COMPARED = [
  "display",
  "flex-direction",
  "gap",
  "background-color",
  "border-top-color",
  "border-top-width",
  "border-top-left-radius",
  "padding-top",
  "padding-left",
  "margin-top",
  "margin-bottom",
  "min-height",
  "min-width",
  "font-weight",
  "text-decoration-line",
  "cursor",
];

async function openFixture(page, { scheme = "light" } = {}) {
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.emulateMedia({ colorScheme: scheme });
  await page.goto(FIXTURE);
}

/** Each element that computes differently from its reference element. */
async function referenceViolations(page, pairs, properties) {
  return page.evaluate(
    ({ pairs, properties }) => {
      const found = [];
      for (const [selector, reference] of Object.entries(pairs)) {
        const actual = getComputedStyle(document.querySelector(selector));
        const expected = getComputedStyle(document.querySelector(reference));
        for (const name of properties) {
          if (actual.getPropertyValue(name) !== expected.getPropertyValue(name)) {
            found.push(`${selector} ${name}: ${actual.getPropertyValue(name)} != ${expected.getPropertyValue(name)}`);
          }
        }
      }
      return found;
    },
    { pairs, properties },
  );
}

/**
 * Each element in `selectors` that computes differently from a copy of itself
 * with no class, id, or data-* attribute, inserted beside it. A difference means
 * a Design System rule reached an element that carries no hook.
 */
async function reachViolations(page, selectors, properties) {
  return page.evaluate(
    ({ selectors, properties }) => {
      const found = [];
      for (const selector of selectors) {
        const element = document.querySelector(selector);
        const copy = element.cloneNode(true);
        for (const attribute of [...copy.attributes]) {
          if (attribute.name === "class" || attribute.name === "id" || attribute.name.startsWith("data-")) {
            copy.removeAttribute(attribute.name);
          }
        }
        // The copy takes the element's place, so it has the same parent and
        // the same position among its siblings.
        element.replaceWith(copy);
        const expected = getComputedStyle(copy);
        const snapshot = Object.fromEntries(properties.map((name) => [name, expected.getPropertyValue(name)]));
        copy.replaceWith(element);
        const actual = getComputedStyle(element);
        for (const name of properties) {
          if (actual.getPropertyValue(name) !== snapshot[name]) {
            found.push(`${selector} ${name}: ${actual.getPropertyValue(name)} != ${snapshot[name]}`);
          }
        }
      }
      return found;
    },
    { selectors, properties },
  );
}

async function computed(page, selector, properties) {
  return page.locator(selector).evaluate(
    (element, names) => Object.fromEntries(names.map((name) => [name, getComputedStyle(element).getPropertyValue(name)])),
    properties,
  );
}

async function role(page, name, property = "background-color") {
  return page.evaluate(
    ({ name, property }) => {
      const probe = document.createElement("div");
      document.body.append(probe);
      probe.style.setProperty(property, `var(${name})`);
      const value = getComputedStyle(probe).getPropertyValue(property);
      probe.remove();
      return value;
    },
    { name, property },
  );
}

test.describe("scoping", () => {
  test("the inventories give nine public hooks", () => {
    expect(CLASS_HOOKS).toEqual([
      "ds-stack",
      "ds-cluster",
      "ds-grid",
      "ds-surface",
      "ds-action",
      "ds-action-primary",
      "ds-action-quiet",
      "ds-action-icon",
    ]);
    expect(ATTRIBUTE_HOOKS).toEqual(["data-ds-scheme"]);
  });

  test("every published rule that selects a hook is anchored at the hooked element, in its own layer, with no @scope", async ({ page }) => {
    await page.goto("/tests/browser/probes/blank.html");
    await loadStylesheets(page, [CORE_EXPORT]);
    const scoped = await page.evaluate((target) => {
      const names = [];
      const walk = (rules) => {
        for (const rule of rules) {
          names.push(rule.constructor.name);
          if (rule.styleSheet) walk(rule.styleSheet.cssRules);
          if (rule.cssRules) walk(rule.cssRules);
        }
      };
      walk([...document.styleSheets].find((sheet) => sheet.href?.endsWith(target)).cssRules);
      return names.filter((name) => /scope/i.test(name));
    }, CORE_EXPORT);
    expect(scoped).toEqual([]);

    const rules = await publishedRules(page, CORE_EXPORT);
    let hooked = 0;
    let attributeHooked = 0;
    for (const rule of rules) {
      const classes = [...rule.selector.matchAll(/\.(-?[_a-zA-Z][\w-]*)/g)].map(([, name]) => name);
      const attributes = [...rule.selector.matchAll(/\[\s*([\w-]+)/g)].map(([, name]) => name.toLowerCase());
      for (const name of classes) {
        expect(CLASS_HOOKS, `${rule.selector} selects a public hook`).toContain(name);
      }
      for (const name of attributes.filter((name) => name.startsWith("data-"))) {
        expect(ATTRIBUTE_HOOKS, `${rule.selector} tests only the theme attribute`).toContain(name);
      }
      const themed = attributes.some((name) => ATTRIBUTE_HOOKS.includes(name));
      if (classes.length === 0 && !themed) {
        continue;
      }
      expect(rule.selector, "no :has() beside a hook").not.toContain(":has(");
      if (classes.length === 0) {
        // The theme attribute sits on the root element and reaches nothing else.
        attributeHooked += 1;
        expect(rule.selector, "the theme attribute is on :root").toMatch(/^:root\[/);
        expect(rule.selector, "no combinator beside the theme attribute").not.toMatch(/\]\s*[\s>+~]\s*\S/);
        expect(rule.layer, rule.selector).toMatch(/^ds\.tokens\./);
        continue;
      }
      hooked += 1;
      for (const [variant, primitive] of VARIANTS) {
        if (classes.includes(variant)) {
          expect(classes, `${rule.selector}: ${variant} sits with ${primitive}`).toContain(primitive);
        }
      }
      // Strip the one reach a layout has, then no top-level combinator may remain.
      const layoutChild = /^:where\(\.(ds-[a-z]+)\) > :where\(\*\)$/.exec(rule.selector);
      if (layoutChild) {
        expect(LAYOUT_HOOKS, `${rule.selector} is a layout's child rule`).toContain(layoutChild[1]);
      } else {
        let depth = 0;
        let topLevel = "";
        for (const character of rule.selector) {
          if (character === "(" || character === "[") depth += 1;
          if (character === ")" || character === "]") depth -= 1;
          if (depth === 0) topLevel += character;
        }
        // One :where() anchored at the hooked element: no top-level combinator,
        // and no combinator or alternative inside it either.
        expect(topLevel.trim(), rule.selector).toBe(":where)");
        expect(firstLevel(rule.selector), rule.selector).not.toMatch(/[\s>+~,]/);
      }
      expect(rule.layer, rule.selector).toMatch(/^ds\.(layouts|primitives)\.[a-z]+$/);
    }
    expect(hooked).toBe(15);
    expect(attributeHooked).toBeGreaterThan(0);
  });

  for (const scheme of ["light", "dark"]) {
    test(`hooks nested inside each other keep their own contracts (${scheme})`, async ({ page }) => {
      await openFixture(page, { scheme });
      const surface = await role(page, "--ds-color-surface");
      const accent = await role(page, "--ds-color-accent");
      const flow = await role(page, "--ds-space-flow", "padding-top");
      for (const selector of ["#outer", "#inner-surface", "#inner-plain"]) {
        const style = await computed(page, selector, ["background-color", "padding-top", "border-top-style"]);
        expect(style["background-color"], selector).toBe(surface);
        expect(Number.parseFloat(style["padding-top"]), selector).toBeCloseTo(Number.parseFloat(flow), 1);
        expect(style["border-top-style"], selector).toBe("solid");
      }
      await expect(page.locator("#outer")).toHaveCSS("flex-direction", "column");
      await expect(page.locator("#inner-grid")).toHaveCSS("display", "grid");
      await expect(page.locator("#inner-surface")).toHaveCSS("display", "flex");
      await expect(page.locator("#inner-plain")).toHaveCSS("display", "list-item");
      await expect(page.locator("#inner-actions")).toHaveCSS("flex-wrap", "wrap");
      await expect(page.locator("#inner-primary")).toHaveCSS("background-color", accent);
      // A direct child loses its block margin; the child of a surface with no layout keeps it.
      await expect(page.locator("#inner-text")).toHaveCSS("margin-bottom", "0px");
      await expect(page.locator("#stack-child")).toHaveCSS("margin-top", "0px");
    });
  }

  test("lookalike markup and a layout's grandchildren compute as if no hook existed", async ({ page }) => {
    await openFixture(page);
    expect(await reachViolations(page, LOOKALIKES, COMPARED)).toEqual([]);
    expect(await referenceViolations(page, DESCENDANTS, COMPARED)).toEqual([]);
    // Neither check is vacuous: each real hook differs from its hookless copy,
    // and a layout's direct child differs from the reference.
    const hooked = await reachViolations(page, ["#inner-primary", "#inner-plain", "#nested-stack"], COMPARED);
    for (const selector of ["#inner-primary", "#inner-plain", "#nested-stack"]) {
      expect(hooked.some((line) => line.startsWith(`${selector} `)), selector).toBe(true);
    }
    expect(await referenceViolations(page, { "#inner-text": "#plain-panel > p" }, COMPARED)).not.toEqual([]);
  });

  test("the reach check detects Design System rules that reach past their element", async ({ page }) => {
    await openFixture(page);
    await loadStylesheets(page, ["/tests/browser/probes/scoping-leak.css"]);
    const violations = [
      ...(await reachViolations(page, LOOKALIKES, COMPARED)),
      ...(await referenceViolations(page, DESCENDANTS, COMPARED)),
    ];
    for (const selector of ["#grandchild", "#variant-alone", "#data-layout"]) {
      expect(violations.some((line) => line.startsWith(`${selector} `)), `${selector} in ${violations}`).toBe(true);
    }
  });

  test("the current state follows the ARIA values: empty and false in any case are not current", async ({ page }) => {
    await openFixture(page);
    const strong = await role(page, "--ds-weight-strong", "font-weight");
    const highlight = await role(page, "--ds-color-highlight");
    for (const selector of ["#current-page", "#current-true"]) {
      await expect(page.locator(selector), selector).toHaveCSS("font-weight", strong);
      await expect(page.locator(selector), selector).toHaveCSS("background-color", highlight);
    }
    for (const selector of ["#current-empty", "#current-false", "#current-upper", "#current-mixed", "#current-bare"]) {
      await expect(page.locator(selector), selector).not.toHaveCSS("font-weight", strong);
      await expect(page.locator(selector), selector).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
    }
  });

  test("consumer classes override hooked elements and layout children without any Design System selector", async ({ page }) => {
    await openFixture(page);
    const canvas = await role(page, "--ds-color-canvas");
    await expect(page.locator("#toolbar")).toHaveCSS("column-gap", "0px");
    await expect(page.locator("#step")).toHaveCSS("margin-right", "32px");
    await expect(page.locator("#flat-action")).toHaveCSS("background-color", canvas);
    await expect(page.locator("#flat-action")).toHaveCSS("border-top-color", "rgba(0, 0, 0, 0)");
    await expect(page.locator("#spaced")).toHaveCSS("margin-top", "48px");
    await expect(page.locator("#tight > p:last-child")).toHaveCSS("margin-top", "0px");
  });

  test("unlayered consumer rules on consumer classes win over hooked rules", async ({ page }) => {
    await openFixture(page);
    await loadStylesheets(page, ["/tests/browser/probes/scoping-unlayered.css"]);
    await expect(page.locator("#step")).toHaveCSS("background-color", "rgb(4, 5, 6)");
    await expect(page.locator("#spaced")).toHaveCSS("margin-bottom", "5px");
  });

  for (const scheme of ["light", "dark"]) {
    test(`axe-core finds no violation on the fixture (${scheme})`, async ({ page }) => {
      await openFixture(page, { scheme });
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
