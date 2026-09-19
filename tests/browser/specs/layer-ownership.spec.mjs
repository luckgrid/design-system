import { expect, loadStylesheets, probeColor, test } from "./support.mjs";

const fixture = (name) => `/fixtures/layer-ownership/${name}/index.css`;

async function importedLayer(page, href) {
  return page.evaluate((entry) => {
    const sheet = [...document.styleSheets].find((candidate) =>
      candidate.href?.endsWith(entry),
    );
    const imported = [...sheet.cssRules].find(
      (rule) => rule.constructor.name === "CSSImportRule",
    );
    const firstChild = imported.styleSheet.cssRules[0];
    return {
      importLayer: imported.layerName,
      childNames: firstChild.nameList ? [...firstChild.nameList] : null,
    };
  }, href);
}

test.describe("layer ownership", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/tests/browser/probes/blank.html");
  });

  test("import-owned parent with unqualified sub-layers keeps the intended order", async ({ page }) => {
    await loadStylesheets(page, [fixture("accepted-import-owned")]);
    expect(await importedLayer(page, fixture("accepted-import-owned"))).toEqual({
      importLayer: "ds.components",
      childNames: ["base", "variants"],
    });
    // ds.components.variants follows ds.components.base.
    expect(await probeColor(page)).toBe("rgb(0, 0, 255)");
  });

  test("source-owned stylesheet enters ds.tokens once without an import wrapper", async ({ page }) => {
    await loadStylesheets(page, [fixture("accepted-source-owned")]);
    expect(await probeColor(page)).toBe("rgb(0, 128, 0)");
  });

  test("prefix-qualified names under an import-owned parent break the intended order", async ({ page }) => {
    // Regression for the rejected pattern: the statement nests as
    // ds.components.ds.components.*, so it never orders the populated
    // ds.components.base/variants layers and first appearance lets base win.
    await loadStylesheets(page, [fixture("rejected-prefix-qualified")]);
    expect(await importedLayer(page, fixture("rejected-prefix-qualified"))).toEqual({
      importLayer: "ds.components",
      childNames: ["ds.components.base", "ds.components.variants"],
    });
    expect(await probeColor(page)).toBe("rgb(255, 0, 0)");
  });
});
