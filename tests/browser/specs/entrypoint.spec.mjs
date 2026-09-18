import { CORE_EXPORT, DS_LAYER_ORDER, expect, loadStylesheets, test } from "./support.mjs";

test.describe("core.css export", () => {
  test("resolves at its declared public path as plain CSS", async ({ request }) => {
    const response = await request.get(CORE_EXPORT);
    expect(response.status()).toBe(200);
    expect(response.headers()["content-type"]).toContain("text/css");
    const source = await response.text();
    for (const directive of ["@tailwind", "@theme", "@apply", "@utility", "@variant", "@source"]) {
      expect(source, `no Tailwind ${directive}`).not.toContain(directive);
    }
  });

  test("an undeclared path is not served", async ({ request }) => {
    const response = await request.get("/packages/styles/README.md");
    expect(response.status()).toBe(404);
  });

  test("publishes exactly the documented layer order and no rules", async ({ page }) => {
    await page.goto("/tests/browser/probes/blank.html");
    await loadStylesheets(page, [CORE_EXPORT]);
    const rules = await page.evaluate((href) => {
      const sheet = [...document.styleSheets].find((candidate) =>
        candidate.href?.endsWith(href),
      );
      return [...sheet.cssRules].map((rule) => ({
        type: rule.constructor.name,
        names: rule.nameList ? [...rule.nameList] : null,
      }));
    }, CORE_EXPORT);
    expect(rules).toEqual([{ type: "CSSLayerStatementRule", names: DS_LAYER_ORDER }]);
  });
});
