import { expect, test } from "./support.mjs";

const FIXTURE = "/adapters/tailwind/fixture/index.html";

test.describe("optional Tailwind adapter", () => {
  test("uses generated utilities without Preflight and resolves semantic aliases", async ({ page }) => {
    await page.goto(FIXTURE);
    const result = await page.evaluate(() => {
      const sheet = [...document.styleSheets].find((candidate) => candidate.href.endsWith("/output.css"));
      const link = document.querySelector("a");
      const canvas = document.createElement("span");
      canvas.style.color = "var(--ds-color-canvas)";
      document.body.append(canvas);
      const value = {
        layers: [...sheet.cssRules]
          .filter((rule) => rule.constructor.name === "CSSLayerStatementRule")
          .flatMap((rule) => [...rule.nameList]),
        background: getComputedStyle(document.body).backgroundColor,
        canvas: getComputedStyle(canvas).color,
        accent: getComputedStyle(link).color,
      };
      canvas.remove();
      return value;
    });
    expect(result.background).toBe(result.canvas);
    expect(result.accent).toBe("oklch(0.5 0.16 300)");
    expect(result.layers.flat()).toContain("ds.utilities");
    expect(result.layers.flat()).not.toContain("base");
  });

  test("keeps scheme selection and consumer semantic override live", async ({ page }) => {
    await page.goto(FIXTURE);
    const before = await page.locator("body").evaluate((element) => getComputedStyle(element).backgroundColor);
    await page.locator("html").evaluate((element) => element.setAttribute("data-ds-scheme", "dark"));
    const after = await page.locator("body").evaluate((element) => getComputedStyle(element).backgroundColor);
    expect(after).not.toBe(before);
    await expect(page.locator("a")).toHaveCSS("color", "oklch(0.5 0.16 300)");
  });

  test("allows consumer-local utilities and variants after the adapter", async ({ page }) => {
    await page.goto(FIXTURE);
    const link = page.locator("a");
    await link.hover();
    await expect(link).toHaveCSS("outline-width", "2px");
  });
});
