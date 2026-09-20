import {
  CORE_EXPORT,
  DS_LAYER_ORDER,
  expect,
  loadStylesheets,
  publishedRules,
  test,
} from "./support.mjs";

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

  test("publishes the documented layer order, then imports the tokens, the base, the layouts, and the primitives", async ({ page }) => {
    await page.goto("/tests/browser/probes/blank.html");
    await loadStylesheets(page, [CORE_EXPORT]);
    const rules = await page.evaluate((href) => {
      const sheet = [...document.styleSheets].find((candidate) =>
        candidate.href?.endsWith(href),
      );
      return [...sheet.cssRules].map((rule) => ({
        type: rule.constructor.name,
        names: rule.nameList ? [...rule.nameList] : null,
        layer: rule.layerName ?? null,
        href: rule.href ?? null,
      }));
    }, CORE_EXPORT);
    expect(rules).toEqual([
      { type: "CSSLayerStatementRule", names: DS_LAYER_ORDER, layer: null, href: null },
      { type: "CSSImportRule", names: null, layer: "ds.tokens", href: "./tokens.css" },
      { type: "CSSImportRule", names: null, layer: "ds.base", href: "./base.css" },
      { type: "CSSImportRule", names: null, layer: "ds.layouts", href: "./layouts.css" },
      { type: "CSSImportRule", names: null, layer: "ds.primitives", href: "./primitives.css" },
    ]);
  });

  test("populates only ds.tokens, ds.base, ds.layouts, and ds.primitives; every later layer is still empty", async ({ page }) => {
    await page.goto("/tests/browser/probes/blank.html");
    await loadStylesheets(page, [CORE_EXPORT]);
    const rules = await publishedRules(page, CORE_EXPORT);
    expect(rules.some((rule) => rule.layer.startsWith("ds.tokens."))).toBe(true);
    expect(rules.some((rule) => rule.layer.startsWith("ds.base."))).toBe(true);
    expect(rules.some((rule) => rule.layer.startsWith("ds.layouts."))).toBe(true);
    expect(rules.some((rule) => rule.layer.startsWith("ds.primitives."))).toBe(true);
    for (const rule of rules) {
      expect(rule.layer, `${rule.sheet} ${rule.selector}`).toMatch(/^ds\.(tokens|base|layouts|primitives)\./);
    }
  });
});
