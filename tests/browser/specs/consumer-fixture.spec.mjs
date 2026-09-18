import { DS_LAYER_ORDER, PUBLISHED_STYLESHEETS, expect, test } from "./support.mjs";

const FIXTURE = "/fixtures/plain-html/index.html";

test.describe("plain consumer fixture", () => {
  test("loads only the declared export and its own stylesheet, without script", async ({ page }) => {
    const stylesheets = [];
    page.on("request", (request) => {
      if (request.resourceType() === "stylesheet") {
        stylesheets.push(new URL(request.url()).pathname);
      }
    });
    await page.goto(FIXTURE);
    // The export brings its own imports; the page links nothing else.
    expect([...stylesheets].sort()).toEqual(
      [...PUBLISHED_STYLESHEETS, "/fixtures/plain-html/consumer.css"].sort(),
    );
    expect(await page.locator("script").count()).toBe(0);
  });

  test("declares the consumer layer after the Design System order", async ({ page }) => {
    await page.goto(FIXTURE);
    const statements = await page.evaluate(() =>
      [...document.styleSheets].flatMap((sheet) =>
        [...sheet.cssRules]
          .filter((rule) => rule.constructor.name === "CSSLayerStatementRule")
          .map((rule) => [...rule.nameList]),
      ),
    );
    expect(statements).toEqual([DS_LAYER_ORDER, ["app"]]);
  });

  test("consumer rules read public semantic roles", async ({ page }) => {
    await page.goto(FIXTURE);
    const colors = await page.evaluate(() => {
      const probe = document.createElement("span");
      document.body.append(probe);
      const resolve = (value) => {
        probe.style.color = value;
        return getComputedStyle(probe).color;
      };
      return {
        background: getComputedStyle(document.body).backgroundColor,
        text: getComputedStyle(document.body).color,
        canvas: resolve("var(--ds-color-canvas)"),
        ink: resolve("var(--ds-color-text)"),
      };
    });
    expect(colors.background).toBe(colors.canvas);
    expect(colors.text).toBe(colors.ink);
  });

  test("consumer rules apply and outrank a Design System layer rule", async ({ page }) => {
    await page.goto(FIXTURE);
    const main = page.locator("main");
    await expect(page.locator("body")).toHaveCSS("padding-top", "32px");
    await expect(main).toHaveCSS("max-width", "768px");

    // A higher-specificity rule in the last Design System layer still loses to
    // the consumer's plain `main` selector in `app`.
    await page.addStyleTag({
      content: "@layer ds.utilities { html body main { max-inline-size: 10rem; } }",
    });
    await expect(main).toHaveCSS("max-width", "768px");
  });
});
