import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { CORE_EXPORT, PUBLISHED_STYLESHEETS, expect, loadStylesheets, test } from "./support.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const read = (relative) => readFileSync(path.join(root, relative), "utf8");

/** The theme inventory: the default and the one root hook with its values. */
const THEME = Object.fromEntries(
  read("theme.tsv")
    .split("\n")
    .filter((line) => line.trim() && !line.startsWith("#"))
    .map((line) => line.split("\t"))
    .map(([, kind, name, values]) => [kind, { name, values: values.trim().split(/\s+/) }]),
);
const HOOK = THEME.attribute.name;
const DEFAULT = THEME.default.values.join(" ");

/** Every light-dark() semantic role and its two reference branches. */
const PAIRS = [
  ...read("packages/styles/tokens/semantic.css")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .matchAll(/(--ds-[a-z0-9-]+)\s*:\s*light-dark\(var\((--ds-ref-[a-z0-9-]+)\), var\((--ds-ref-[a-z0-9-]+)\)\)\s*;/g),
].map(([, name, light, dark]) => ({ name, light, dark }));

const REFERENCES = read("tokens.tsv")
  .split("\n")
  .filter((line) => line.startsWith("internal\t"))
  .map((line) => line.split("\t")[1]);

const BRAND_FIXTURE = "/fixtures/brand-theme/index.html";
const PLAIN_FIXTURE = "/fixtures/plain-html/index.html";
const SCHEMES = ["light", "dark"];

/** Set or remove the hook on <html>. */
async function setHook(page, value) {
  await page.evaluate(
    ([name, next]) => {
      if (next === null) {
        document.documentElement.removeAttribute(name);
      } else {
        document.documentElement.setAttribute(name, next);
      }
    },
    [HOOK, value],
  );
}

/**
 * The root's computed color-scheme, and for every pair the color the role
 * resolves to on an element that inherits the root scheme, next to both
 * reference branches.
 */
async function snapshot(page) {
  return page.evaluate((pairs) => {
    const probe = document.createElement("div");
    document.body.append(probe);
    const resolve = (value) => {
      probe.style.color = value;
      return getComputedStyle(probe).color;
    };
    const roles = pairs.map(({ name, light, dark }) => ({
      name,
      role: resolve(`var(${name})`),
      light: resolve(`var(${light})`),
      dark: resolve(`var(${dark})`),
    }));
    probe.remove();
    return { scheme: getComputedStyle(document.documentElement).colorScheme, roles };
  }, PAIRS);
}

function expectBranch(state, branch, label) {
  expect(state.roles.length, "light-dark() roles found").toBeGreaterThan(0);
  for (const { name, role, light, dark } of state.roles) {
    expect(light, `${name} has distinct branches`).not.toBe(dark);
    expect(role, `${label}: ${name}`).toBe(branch === "light" ? light : dark);
  }
}

test.describe("theme contract", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/tests/browser/probes/blank.html");
    await loadStylesheets(page, [CORE_EXPORT]);
  });

  test("the inventory names one default that follows the preference and one root hook", () => {
    expect(DEFAULT).toBe("light dark");
    expect(HOOK).toBe("data-ds-scheme");
    expect(THEME.attribute.values).toEqual(SCHEMES);
    expect(Object.keys(THEME).sort()).toEqual(["attribute", "default"]);
  });

  test("with no hook value, every role follows the OS preference", async ({ page }) => {
    for (const preference of SCHEMES) {
      await page.emulateMedia({ colorScheme: preference });
      const state = await snapshot(page);
      expect(state.scheme).toBe(DEFAULT);
      expectBranch(state, preference, `default under ${preference} preference`);
    }
  });

  test("an explicit hook value wins over the opposite preference", async ({ page }) => {
    for (const value of SCHEMES) {
      for (const preference of SCHEMES) {
        await page.emulateMedia({ colorScheme: preference });
        await setHook(page, value);
        const state = await snapshot(page);
        expect(state.scheme, `${value} under ${preference}`).toBe(value);
        expectBranch(state, value, `explicit ${value} under ${preference} preference`);
      }
    }
  });

  test("an unknown hook value keeps the default", async ({ page }) => {
    for (const value of ["", "auto", "system", "Dark", "LIGHT", "light dark", "blue"]) {
      for (const preference of SCHEMES) {
        await page.emulateMedia({ colorScheme: preference });
        await setHook(page, value);
        const state = await snapshot(page);
        expect(state.scheme, `"${value}" under ${preference}`).toBe(DEFAULT);
        expectBranch(state, preference, `"${value}" under ${preference} preference`);
      }
    }
  });

  test("the hook is read only on the root element", async ({ page }) => {
    await page.emulateMedia({ colorScheme: "light" });
    const result = await page.evaluate(
      ([name, pair]) => {
        const scope = document.createElement("section");
        scope.setAttribute(name, "dark");
        const child = document.createElement("p");
        scope.append(child);
        document.body.append(scope);
        child.style.color = `var(${pair.name})`;
        const role = getComputedStyle(child).color;
        child.style.color = `var(${pair.light})`;
        const light = getComputedStyle(child).color;
        const scheme = getComputedStyle(scope).colorScheme;
        scope.remove();
        return { role, light, scheme };
      },
      [HOOK, PAIRS[0]],
    );
    expect(result.scheme).toBe(DEFAULT);
    expect(result.role).toBe(result.light);
  });

  test("switching at runtime moves roles and native controls together", async ({ page }) => {
    await page.emulateMedia({ colorScheme: "light" });
    await page.evaluate(() => {
      const form = document.createElement("form");
      form.innerHTML = `
        <input id="text" type="text">
        <select id="select"><option>One</option></select>
        <input id="check" type="checkbox">
        <button id="button" type="button">Button</button>
        <div id="system" style="background-color: Canvas; color: CanvasText; border-color: Field"></div>`;
      document.body.append(form);
    });
    const controls = () =>
      page.evaluate(() =>
        Object.fromEntries(
          ["text", "select", "check", "button", "system"].map((id) => {
            const style = getComputedStyle(document.getElementById(id));
            return [
              id,
              { scheme: style.colorScheme, background: style.backgroundColor, color: style.color },
            ];
          }),
        ),
      );

    const seen = {};
    for (const value of ["dark", "light", null, "dark"]) {
      await setHook(page, value);
      const state = await snapshot(page);
      const expected = value ?? "light";
      expect(state.scheme).toBe(value ?? DEFAULT);
      expectBranch(state, expected, `runtime ${value ?? "default"}`);
      const current = await controls();
      for (const [id, { scheme }] of Object.entries(current)) {
        expect(scheme, `${id} inherits the root scheme`).toBe(value ?? DEFAULT);
      }
      seen[value ?? "default"] = current;
    }
    // The user agent draws its own controls and system colors for the scheme.
    expect(seen.dark.text.background, "text field").not.toBe(seen.light.text.background);
    expect(seen.dark.system.background, "Canvas").not.toBe(seen.light.system.background);
    expect(seen.dark.system.color, "CanvasText").not.toBe(seen.light.system.color);
    // Under a light preference the default draws exactly what explicit light draws.
    const drawn = (state) =>
      Object.fromEntries(
        Object.entries(state).map(([id, { background, color }]) => [id, { background, color }]),
      );
    expect(drawn(seen.default)).toEqual(drawn(seen.light));
  });

  test("a consumer root color-scheme outranks the hook, as documented", async ({ page }) => {
    await page.emulateMedia({ colorScheme: "light" });
    await page.addStyleTag({ content: "@layer app { :root { color-scheme: light; } }" });
    await setHook(page, "dark");
    const state = await snapshot(page);
    expect(state.scheme).toBe("light");
    expectBranch(state, "light", "consumer root color-scheme with hook dark");
  });

  test("published stylesheets use no alias theme hook", async ({ request }) => {
    for (const href of PUBLISHED_STYLESHEETS) {
      const source = (await (await request.get(href)).text()).replace(/\/\*[\s\S]*?\*\//g, "");
      for (const alias of [".dark", "[data-theme", "[data-color-scheme", "prefers-color-scheme"]) {
        expect(source, `${href} has no ${alias}`).not.toContain(alias);
      }
    }
  });
});

test.describe("consumer fixtures under the theme", () => {
  test("the plain fixture follows the preference and the hook", async ({ page }) => {
    await page.emulateMedia({ colorScheme: "dark" });
    await page.goto(PLAIN_FIXTURE);
    const canvas = (scheme) =>
      page.evaluate((branch) => {
        const probe = document.createElement("span");
        probe.style.colorScheme = branch;
        probe.style.color = "var(--ds-color-canvas)";
        document.body.append(probe);
        const value = getComputedStyle(probe).color;
        probe.remove();
        return value;
      }, scheme);
    const body = () => page.evaluate(() => getComputedStyle(document.body).backgroundColor);

    expect(await body()).toBe(await canvas("dark"));
    await setHook(page, "light");
    expect(await body()).toBe(await canvas("light"));
    await page.emulateMedia({ colorScheme: "light" });
    await setHook(page, "dark");
    expect(await body()).toBe(await canvas("dark"));
  });

  test("a consumer brand maps through public roles in every scheme state", async ({ page }) => {
    await page.goto("/tests/browser/probes/blank.html");
    await loadStylesheets(page, [CORE_EXPORT]);
    const referencesOf = () =>
      page.evaluate((names) => {
        const style = getComputedStyle(document.documentElement);
        return names.map((name) => style.getPropertyValue(name).trim());
      }, REFERENCES);
    const coreReferences = await referencesOf();

    await page.goto(BRAND_FIXTURE);
    expect(await referencesOf(), "no reference value is overridden").toEqual(coreReferences);

    const measure = () =>
      page.evaluate(() => {
        const resolve = (value) => {
          const probe = document.createElement("span");
          probe.style.color = value;
          document.body.append(probe);
          const result = getComputedStyle(probe).color;
          probe.remove();
          return result;
        };
        const style = (selector) => getComputedStyle(document.querySelector(selector));
        return {
          scheme: getComputedStyle(document.documentElement).colorScheme,
          body: style("body").backgroundColor,
          text: style("body").color,
          surface: style(".panel").backgroundColor,
          cta: style(".cta").backgroundColor,
          onCta: style(".cta").color,
          muted: style(".note").color,
          font: style("body").fontFamily,
          radius: style(".cta").borderTopLeftRadius,
          input: style("input[type=text]").colorScheme,
          brand: {
            light: {
              body: resolve("var(--tidewater-sand-98)"),
              text: resolve("var(--tidewater-slate-18)"),
              surface: resolve("var(--tidewater-sand-92)"),
              cta: resolve("var(--tidewater-teal-42)"),
              onCta: resolve("var(--tidewater-sand-98)"),
              muted: resolve("var(--tidewater-slate-45)"),
            },
            dark: {
              body: resolve("var(--tidewater-slate-18)"),
              text: resolve("var(--tidewater-slate-95)"),
              surface: resolve("var(--tidewater-slate-26)"),
              cta: resolve("var(--tidewater-teal-80)"),
              onCta: resolve("var(--tidewater-slate-18)"),
              muted: resolve("var(--tidewater-slate-75)"),
            },
          },
        };
      });

    const cases = [
      { preference: "light", hook: null, branch: "light" },
      { preference: "dark", hook: null, branch: "dark" },
      { preference: "light", hook: "dark", branch: "dark" },
      { preference: "dark", hook: "light", branch: "light" },
    ];
    for (const { preference, hook, branch } of cases) {
      await page.emulateMedia({ colorScheme: preference });
      await setHook(page, hook);
      const state = await measure();
      const label = `${hook ?? "default"} under ${preference}`;
      expect(state.scheme, label).toBe(hook ?? DEFAULT);
      expect(state.input, label).toBe(hook ?? DEFAULT);
      const expected = state.brand[branch];
      expect(
        {
          body: state.body,
          text: state.text,
          surface: state.surface,
          cta: state.cta,
          onCta: state.onCta,
          muted: state.muted,
        },
        label,
      ).toEqual(expected);
      expect(state.brand.light.body, "brand branches differ").not.toBe(state.brand.dark.body);
      expect(state.font, label).toContain("Tidewater Grotesk");
      expect(state.radius, label).toBe("0px");
    }
  });
});
