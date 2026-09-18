import { CORE_EXPORT, DS_LAYER_ORDER, expect, test } from "./support.mjs";

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
    expect(stylesheets).toEqual([CORE_EXPORT, "/fixtures/plain-html/consumer.css"]);
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
