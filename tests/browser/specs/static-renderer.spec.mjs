import AxeBuilder from "@axe-core/playwright";

import { DS_LAYER_ORDER, PUBLISHED_STYLESHEETS, expect, test } from "./support.mjs";

// The site the static renderer produced from the staged public-export boundary.
// These are regression checks in the current engines, not the browser-floor or
// assistive-technology evidence that belongs to the release-candidate review.
const SITE = "/fixtures/static-renderer/public";
const PAGES = {
  overview: `${SITE}/`,
  hooks: `${SITE}/hooks/`,
  article: `${SITE}/article/`,
  forms: `${SITE}/forms/`,
};

/** The staged boundary publishes each export under its name, imports beside it. */
const STAGED = PUBLISHED_STYLESHEETS.map((href) =>
  `${SITE}/design-system${href.replace("/packages/styles/index.css", "/core.css").replace("/packages/styles", "")}`,
);
const CONSUMER = [`${SITE}/consumer/site.css`, `${SITE}/consumer/theme.css`];

/** Resolve a CSS value on a probe element, as the engine computes it. */
async function computed(page, property, value) {
  return page.evaluate(
    ([name, input]) => {
      const probe = document.createElement("span");
      document.body.append(probe);
      probe.style.setProperty(name, input);
      const result = getComputedStyle(probe).getPropertyValue(name);
      probe.remove();
      return result;
    },
    [property, value],
  );
}

const style = (page, selector, property) =>
  page.locator(selector).first().evaluate((element, name) => getComputedStyle(element).getPropertyValue(name), property);

test.describe("static renderer consumer fixture", () => {
  test("loads only the staged export graph and its own stylesheets, without script", async ({ page }) => {
    const requested = [];
    page.on("request", (request) => {
      if (request.resourceType() === "stylesheet") {
        requested.push(new URL(request.url()).pathname);
      }
    });
    await page.goto(PAGES.overview);
    expect([...requested].sort()).toEqual([...STAGED, ...CONSUMER].sort());
    expect(requested.some((pathname) => pathname.includes("/packages/"))).toBe(false);
    expect(await page.locator("script").count()).toBe(0);
  });

  test("declares the consumer layer after the Design System order", async ({ page }) => {
    await page.goto(PAGES.overview);
    const statements = await page.evaluate(() =>
      [...document.styleSheets].flatMap((sheet) =>
        [...sheet.cssRules]
          .filter((rule) => rule.constructor.name === "CSSLayerStatementRule")
          .map((rule) => [...rule.nameList]),
      ),
    );
    expect(statements).toEqual([DS_LAYER_ORDER, ["app"], ["app"]]);
  });

  test("renders every page from static files without a failed request", async ({ page }) => {
    for (const path of Object.values(PAGES)) {
      const response = await page.goto(path);
      expect(response.status()).toBe(200);
      await expect(page.locator("main h1")).toHaveCount(1);
    }
  });

  test.describe("promoted hooks", () => {
    test.beforeEach(async ({ page }) => {
      await page.setViewportSize({ width: 1280, height: 900 });
      await page.goto(PAGES.hooks);
    });

    test("layouts arrange their children through the hooks", async ({ page }) => {
      expect(await style(page, "#stack", "display")).toBe("flex");
      expect(await style(page, "#stack", "flex-direction")).toBe("column");
      expect(await style(page, "#stack > p", "margin-top")).toBe("0px");
      expect(await style(page, "#cluster", "display")).toBe("flex");
      expect(await style(page, "#cluster", "flex-wrap")).toBe("wrap");
      expect(await style(page, "#grid", "display")).toBe("grid");
      const columns = await style(page, "#grid", "grid-template-columns");
      expect(columns.split(" ").length).toBeGreaterThan(1);
    });

    test("the surface bounds its region and composes with a layout", async ({ page }) => {
      expect(await style(page, "#card", "border-top-style")).toBe("solid");
      expect(Number.parseFloat(await style(page, "#card", "padding-top"))).toBeGreaterThan(0);
      expect(await style(page, "#card", "display")).toBe("flex");
      expect(await style(page, "#plain-surface", "display")).toBe("block");
      expect(await style(page, "#card", "background-color")).toBe(
        await computed(page, "color", "var(--ds-color-surface)"),
      );
    });

    test("action variants differ through the public hooks", async ({ page }) => {
      const accent = await computed(page, "color", "var(--ds-color-accent)");
      // Inside a cluster the control is a flex item, so its display is blockified.
      expect(["flex", "inline-flex"]).toContain(await style(page, "#default", "display"));
      expect(await style(page, "#primary", "background-color")).toBe(accent);
      expect(await style(page, "#quiet", "background-color")).toBe("rgba(0, 0, 0, 0)");
      expect(await style(page, "#quiet", "border-top-color")).toBe("rgba(0, 0, 0, 0)");
      expect(await style(page, "#icon", "aspect-ratio")).toBe("1 / 1");
      expect(await style(page, "#link-action", "text-decoration-line")).toBe("none");
    });

    test("native and ARIA states are styled, and non-current is not", async ({ page }) => {
      const muted = await computed(page, "color", "var(--ds-color-text-muted)");
      expect(await style(page, "#disabled", "cursor")).toBe("not-allowed");
      expect(await style(page, "#disabled", "color")).toBe(muted);
      expect(await style(page, "#disabled-primary", "color")).toBe(muted);
      expect(await style(page, "#unlinked", "cursor")).toBe("not-allowed");
      expect(await style(page, "#unlinked", "color")).toBe(muted);
      const strong = await computed(page, "font-weight", "var(--ds-weight-strong)");
      expect(await style(page, "#current-quiet", "font-weight")).toBe(strong);
      expect(await style(page, "#not-current", "font-weight")).not.toBe(strong);
    });

    test("lists that carry a layout hook drop their markers and padding in the consumer stylesheet and keep the list role", async ({ page }) => {
      for (const selector of ["#cluster", "#grid"]) {
        await expect(page.locator(selector), selector).toHaveAttribute("role", "list");
        expect(await style(page, selector, "list-style-type"), selector).toBe("none");
        expect(await style(page, selector, "padding-inline-start"), selector).toBe("0px");
        expect(await style(page, `${selector} > li`, "display"), selector).toBe("list-item");
      }
      // Ordinary lists keep the native presentation: the classless base is not reset.
      const native = await page.evaluate(() => {
        const list = document.createElement("ul");
        list.innerHTML = "<li>one</li>";
        document.body.append(list);
        const style = getComputedStyle(list);
        const result = [style.listStyleType, style.paddingInlineStart];
        list.remove();
        return result;
      });
      expect(native[0]).toBe("disc");
      expect(Number.parseFloat(native[1])).toBeGreaterThan(0);
    });

    test("hover and keyboard focus use the native states", async ({ page, browserName }) => {
      const accent = await computed(page, "color", "var(--ds-color-accent)");
      const control = page.locator("#default");
      await control.hover();
      expect(await style(page, "#default", "border-top-color")).toBe(accent);

      for (let count = 0; count < 40; count += 1) {
        await page.keyboard.press(browserName === "webkit" ? "Alt+Tab" : "Tab");
        if ((await page.evaluate(() => document.activeElement?.id)) === "default") {
          break;
        }
      }
      expect(await page.evaluate(() => document.activeElement?.id)).toBe("default");
      expect(await style(page, "#default", "outline-style")).toBe("solid");
      expect(await style(page, "#default", "outline-width")).toBe(
        await computed(page, "outline-width", "var(--ds-focus-width)"),
      );
    });
  });

  test.describe("consumer-owned overrides", () => {
    test("the semantic theme maps public roles in each scheme and stays live", async ({ page }) => {
      await page.goto(PAGES.hooks);
      const light = await computed(page, "color", "var(--ds-color-accent)");
      expect(light).toBe(await computed(page, "color", "var(--harbor-rust)"));
      expect(await style(page, "body", "font-family")).toContain("Harbor Serif");

      await page.locator("html").evaluate((element) => element.setAttribute("data-ds-scheme", "dark"));
      const dark = await computed(page, "color", "var(--ds-color-accent)");
      expect(dark).toBe(await computed(page, "color", "var(--harbor-rust-bright)"));
      expect(dark).not.toBe(light);

      // Removing the consumer theme restores the Design System default.
      await page.evaluate(() => {
        const sheet = [...document.styleSheets].find((candidate) => candidate.href?.endsWith("/consumer/theme.css"));
        sheet.disabled = true;
      });
      expect(await computed(page, "color", "var(--ds-color-accent)")).not.toBe(dark);
    });

    test("a plain consumer rule outranks Design System layers without escalation", async ({ page }) => {
      await page.goto(PAGES.hooks);
      expect(await style(page, "#pill", "border-top-left-radius")).toBe("999px");
      expect(await style(page, "#flat-surface", "border-top-style")).toBe("dashed");
      expect(await style(page, "#flat-surface", "background-color")).toBe("rgba(0, 0, 0, 0)");
      await page.addStyleTag({
        content: "@layer ds.utilities { html body main .ds-action#pill { border-radius: 0; } }",
      });
      expect(await style(page, "#pill", "border-top-left-radius")).toBe("999px");
    });
  });

  test.describe("scheme hook", () => {
    for (const [name, scheme, hook] of [
      ["overview", "light", null],
      ["hooks", "dark", "light"],
      ["article", "light", "dark"],
    ]) {
      test(`${name} resolves color-scheme ${hook ?? "light dark"} against a ${scheme} preference`, async ({ page }) => {
        await page.emulateMedia({ colorScheme: scheme });
        await page.goto(PAGES[name]);
        expect(await page.locator("html").getAttribute("data-ds-scheme")).toBe(hook);
        expect(await style(page, "html", "color-scheme")).toBe(hook ?? "light dark");
      });
    }

    test("explicit light and dark pages paint different canvases", async ({ page }) => {
      await page.goto(PAGES.hooks);
      const light = await style(page, "html", "background-color");
      expect(light).toBe(await computed(page, "color", "var(--ds-color-canvas)"));
      await page.goto(PAGES.article);
      const dark = await style(page, "html", "background-color");
      expect(dark).toBe(await computed(page, "color", "var(--ds-color-canvas)"));
      expect(dark).not.toBe(light);
    });
  });

  test.describe("classless base", () => {
    test("styles ordinary HTML with no class attribute", async ({ page }) => {
      await page.setViewportSize({ width: 1280, height: 900 });
      await page.goto(PAGES.article);
      expect(await page.locator("[class]").count()).toBe(0);
      expect(await style(page, "body", "color")).toBe(await computed(page, "color", "var(--ds-color-text)"));
      expect(await style(page, "h1", "font-size")).toBe(await computed(page, "font-size", "var(--ds-text-heading-1)"));
      expect(await style(page, "pre", "overflow-x")).toBe("auto");
    });
  });

  for (const [name, path] of Object.entries(PAGES)) {
    test(`${name} does not overflow horizontally on a narrow viewport`, async ({ page }) => {
      await page.setViewportSize({ width: 320, height: 800 });
      await page.goto(path);
      const overflow = await page.evaluate(
        () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
      );
      expect(overflow).toBeLessThanOrEqual(0);
    });

    test(`axe-core finds no violation on ${name}`, async ({ page }) => {
      await page.goto(path);
      const results = await new AxeBuilder({ page })
        .options({ preload: false })
        .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa", "best-practice"])
        .analyze();
      expect(
        results.violations.map(({ id, nodes }) => `${id}: ${nodes.map((node) => node.target).join(" | ")}`),
      ).toEqual([]);
      expect(results.passes.length).toBeGreaterThan(10);
    });
  }
});
