import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  CORE_EXPORT,
  PUBLISHED_STYLESHEETS,
  expect,
  loadStylesheets,
  publishedRules,
  test,
} from "./support.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const read = (relative) => readFileSync(path.join(root, relative), "utf8");

const INVENTORY = read("tokens.tsv")
  .split("\n")
  .filter((line) => line.trim() && !line.startsWith("#"))
  .map((line) => {
    const [compatibility, name, type] = line.split("\t");
    return { compatibility, name, type };
  });
const PUBLIC = INVENTORY.filter((row) => row.compatibility === "public-preview");

/** `name: value` pairs authored in a token stylesheet. */
function authored(relative) {
  const source = read(relative).replace(/\/\*[\s\S]*?\*\//g, "");
  return Object.fromEntries(
    [...source.matchAll(/(--ds-[a-z0-9-]+)\s*:\s*([^;]+);/g)].map(([, name, value]) => [
      name,
      value.replace(/\s+/g, " ").trim(),
    ]),
  );
}
const REFERENCE = authored("packages/styles/tokens/reference.css");
const SEMANTIC = authored("packages/styles/tokens/semantic.css");

const PAIRS = Object.entries(SEMANTIC)
  .map(([name, value]) => [name, value.match(/^light-dark\(var\((--ds-ref-[a-z0-9-]+)\), var\((--ds-ref-[a-z0-9-]+)\)\)$/)])
  .filter(([, match]) => match)
  .map(([name, [, light, dark]]) => ({ name, light, dark }));

/** Fluid roles and the clamp() bounds of the reference each one aliases. */
const FLUID = Object.entries(SEMANTIC)
  .map(([name, value]) => [name, value.match(/^var\((--ds-ref-(?:space|text)-[0-9-]+)\)$/)])
  .filter(([, match]) => match)
  .map(([name, [, reference]]) => {
    const [, min, intercept, slope, max] = REFERENCE[reference].match(
      /^clamp\(([\d.]+)rem, ([\d.]+)rem \+ ([\d.]+)vw, ([\d.]+)rem\)$/,
    );
    return { name, reference, min: +min * 16, intercept: +intercept * 16, slope: +slope, max: +max * 16 };
  });

const FIXED_LENGTHS = PUBLIC.filter(
  (row) => row.type === "length" && !FLUID.some((fluid) => fluid.name === row.name),
).map((row) => row.name);

const VIEWPORTS = [320, 480, 1520, 2560, 3200];
const BRAND_PROBE = "/tests/browser/probes/brand-mapping.css";

/** Resolve CSS values on throwaway elements inside the loaded page. */
async function resolve(page, requests) {
  return page.evaluate((items) => {
    const host = document.createElement("div");
    document.body.append(host);
    const results = items.map(({ property, value, scheme, outer }) => {
      const parent = document.createElement("div");
      const element = document.createElement("div");
      if (outer) {
        parent.style.colorScheme = outer.scheme;
        if (outer.variable) {
          parent.style.setProperty("--probe", outer.variable);
        }
      }
      if (scheme) {
        element.style.colorScheme = scheme;
      }
      element.style.setProperty(property, value);
      parent.append(element);
      host.append(parent);
      return getComputedStyle(element).getPropertyValue(property);
    });
    host.remove();
    return results;
  }, requests);
}

const color = (value, scheme, outer) => ({ property: "color", value, scheme, outer });

test.describe("token authority", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/tests/browser/probes/blank.html");
    await loadStylesheets(page, [CORE_EXPORT]);
  });

  test("published stylesheets are plain CSS without Tailwind directives", async ({ request }) => {
    for (const href of PUBLISHED_STYLESHEETS) {
      const response = await request.get(href);
      expect(response.status(), href).toBe(200);
      expect(response.headers()["content-type"], href).toContain("text/css");
      const source = await response.text();
      for (const directive of ["@tailwind", "@theme", "@apply", "@utility", "@variant", "@source"]) {
        expect(source, `${href} has no ${directive}`).not.toContain(directive);
      }
    }
  });

  test("declared tokens are exactly the inventory, on :root, in their tier's layer", async ({ page }) => {
    const rules = await publishedRules(page, CORE_EXPORT);
    const declared = [];
    for (const rule of rules) {
      expect(rule.selector).toBe(":root");
      for (const property of rule.properties) {
        declared.push(property);
        const tier = property.startsWith("--ds-ref-") ? "reference" : "semantic";
        expect(rule.layer, property).toBe(`ds.tokens.${tier}`);
        expect(rule.sheet, property).toBe(`/packages/styles/tokens/${tier}.css`);
      }
    }
    expect(declared.sort()).toEqual(INVENTORY.map((row) => row.name).sort());
  });

  test("every public role resolves as its value type", async ({ page }) => {
    const sentinel = await page.evaluate((rows) => {
      const results = {};
      const parent = document.createElement("div");
      parent.style.color = "rgb(1, 2, 3)";
      parent.style.fontFamily = "sentinel-family";
      document.body.append(parent);
      for (const { name, type } of rows) {
        const element = document.createElement("div");
        parent.append(element);
        const property = {
          color: "color",
          "font-family": "font-family",
          length: "margin-left",
          number: "flex-grow",
        }[type];
        element.style.setProperty(property, `var(${name})`);
        results[name] = { type, value: getComputedStyle(element).getPropertyValue(property) };
      }
      parent.remove();
      return results;
    }, PUBLIC);
    for (const [name, { type, value }] of Object.entries(sentinel)) {
      if (type === "color") {
        expect(value, name).not.toBe("rgb(1, 2, 3)");
      } else if (type === "font-family") {
        expect(value, name).not.toContain("sentinel-family");
      } else {
        // An invalid substitution falls back to the initial value, 0.
        expect(Number.parseFloat(value), `${name} = ${value}`).toBeGreaterThan(0);
      }
    }
  });

  test("the tokens set no color-scheme; the default resolves light even under a dark preference", async ({ page }) => {
    await page.emulateMedia({ colorScheme: "dark" });
    const scheme = await page.evaluate(() => getComputedStyle(document.documentElement).colorScheme);
    expect(scheme).toBe("normal");
    for (const { name, light } of PAIRS) {
      const [role, expected] = await resolve(page, [color(`var(${name})`), color(`var(${light})`)]);
      expect(role, name).toBe(expected);
    }
  });

  test("light-dark() roles follow the consuming element's color-scheme", async ({ page }) => {
    expect(PAIRS.length).toBeGreaterThan(0);
    for (const { name, light, dark } of PAIRS) {
      const role = `var(${name})`;
      const [lightRef, darkRef, ...cases] = await resolve(page, [
        color(`var(${light})`),
        color(`var(${dark})`),
        color(role, "light"),
        color(role, "dark"),
        // A nested scope flips the scheme back.
        color(role, "light", { scheme: "dark" }),
        color(role, "dark", { scheme: "light" }),
        // Stored in a custom property on a light ancestor, consumed in a dark scope.
        color("var(--probe)", "dark", { scheme: "light", variable: role }),
        color("var(--probe)", "light", { scheme: "dark", variable: role }),
      ]);
      expect(lightRef, `${name} has distinct branches`).not.toBe(darkRef);
      expect(cases, name).toEqual([lightRef, darkRef, lightRef, darkRef, darkRef, lightRef]);
    }
  });

  test("fluid roles stay within their bounds and interpolate between them", async ({ page }) => {
    expect(FLUID.length).toBeGreaterThan(0);
    const lengths = (names) =>
      resolve(
        page,
        names.map((name) => ({ property: "margin-left", value: `var(${name})` })),
      ).then((values) => values.map(Number.parseFloat));
    const fixedAtFirst = [];
    for (const width of VIEWPORTS) {
      await page.setViewportSize({ width, height: 800 });
      const measured = await lengths(FLUID.map((fluid) => fluid.name));
      FLUID.forEach((fluid, index) => {
        const expected = Math.min(
          Math.max(fluid.min, fluid.intercept + (fluid.slope * width) / 100),
          fluid.max,
        );
        expect(measured[index], `${fluid.name} at ${width}px`).toBeCloseTo(expected, 0);
        expect(measured[index]).toBeGreaterThanOrEqual(fluid.min - 0.5);
        expect(measured[index]).toBeLessThanOrEqual(fluid.max + 0.5);
      });
      if (width === 480) {
        FLUID.forEach((fluid, index) => expect(measured[index]).toBeCloseTo(fluid.min, 0));
      }
      if (width === 2560) {
        FLUID.forEach((fluid, index) => expect(measured[index]).toBeCloseTo(fluid.max, 0));
      }
      const fixed = await lengths(FIXED_LENGTHS);
      if (fixedAtFirst.length === 0) {
        fixedAtFirst.push(...fixed);
      }
      expect(fixed, `fixed roles at ${width}px`).toEqual(fixedAtFirst);
    }
    // Spot-check the fixed values themselves.
    const [border, focus, target] = await lengths([
      "--ds-border-width",
      "--ds-focus-width",
      "--ds-size-target-min",
    ]);
    expect([border, focus, target]).toEqual([1, 2, 44]);
  });

  test("an alias follows its target when a consumer maps the target", async ({ page }) => {
    await page.addStyleTag({
      content: "@layer app { :root { --ds-color-accent: rgb(10 20 30); --ds-font-body: alias-probe, serif; } }",
    });
    const [focus, heading] = await resolve(page, [
      color("var(--ds-color-focus)"),
      { property: "font-family", value: "var(--ds-font-heading)" },
    ]);
    expect(focus).toBe("rgb(10, 20, 30)");
    expect(heading).toContain("alias-probe");
  });

  test("a consumer brand maps into semantic roles without touching reference values", async ({ page }) => {
    const references = () =>
      page.evaluate((names) => {
        const style = getComputedStyle(document.documentElement);
        return names.map((name) => style.getPropertyValue(name));
      }, INVENTORY.filter((row) => row.compatibility === "internal").map((row) => row.name));
    const before = await references();
    const [defaultAccent] = await resolve(page, [color("var(--ds-color-accent)")]);

    await loadStylesheets(page, [BRAND_PROBE]);
    expect(await references()).toEqual(before);

    const [accentLight, accentDark, brandLight, brandDark, canvasDark, brandInk, family, radius] =
      await resolve(page, [
        color("var(--ds-color-accent)", "light"),
        color("var(--ds-color-accent)", "dark"),
        color("oklch(45% 0.2 300)"),
        color("oklch(80% 0.1 300)"),
        color("var(--ds-color-canvas)", "dark"),
        color("oklch(22% 0.02 90)"),
        { property: "font-family", value: "var(--ds-font-heading)" },
        { property: "border-top-left-radius", value: "var(--ds-radius-control)" },
      ]);
    expect(accentLight).not.toBe(defaultAccent);
    expect([accentLight, accentDark, canvasDark]).toEqual([brandLight, brandDark, brandInk]);
    expect(family).toContain("Brand Probe Sans");
    expect(radius).toBe("0px");

    // The same mapping still wins over a higher-specificity Design System rule.
    await page.addStyleTag({
      content: "@layer ds.tokens { html:root { --ds-color-accent: rgb(200 0 0); } }",
    });
    const [stillBrand] = await resolve(page, [color("var(--ds-color-accent)", "light")]);
    expect(stillBrand).toBe(brandLight);
  });
});
