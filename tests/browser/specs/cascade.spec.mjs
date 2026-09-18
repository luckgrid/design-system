import { CORE_EXPORT, expect, loadStylesheets, probeColor, test } from "./support.mjs";

const REVERSED_PROBE = "/tests/browser/probes/ds-layers-reversed.css";

test.describe("cascade contract", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/tests/browser/probes/blank.html");
  });

  test("without the entrypoint, first appearance decides and ds.tokens wins", async ({ page }) => {
    await loadStylesheets(page, [REVERSED_PROBE]);
    expect(await probeColor(page)).toBe("rgb(1, 0, 0)");
  });

  test("the entrypoint's order makes ds.utilities win over higher specificity", async ({ page }) => {
    await loadStylesheets(page, [CORE_EXPORT, REVERSED_PROBE]);
    expect(await probeColor(page)).toBe("rgb(6, 0, 0)");
  });

  test("a consumer layer after ds wins with a plain element selector", async ({ page }) => {
    await loadStylesheets(page, [
      CORE_EXPORT,
      REVERSED_PROBE,
      "/tests/browser/probes/consumer-layer.css",
    ]);
    expect(await probeColor(page)).toBe("rgb(0, 100, 0)");
  });

  test("unlayered consumer CSS outranks every layer", async ({ page }) => {
    await loadStylesheets(page, [
      CORE_EXPORT,
      REVERSED_PROBE,
      "/tests/browser/probes/consumer-layer.css",
      "/tests/browser/probes/consumer-unlayered.css",
    ]);
    expect(await probeColor(page)).toBe("rgb(0, 0, 100)");
  });
});
