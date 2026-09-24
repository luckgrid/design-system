import { expect, test } from "./support.mjs";

// Print adaptation is emulated with print media in the current engines. These
// are regression checks for the print rules, not printer or Safari print
// evidence: a printer that omits backgrounds is modeled by measuring text
// against a white sheet.

/** Contrast ratio of an element's text color against a white sheet. */
async function contrastAgainstWhite(page, selector) {
  return page.locator(selector).first().evaluate((element) => {
    const context = document.createElement("canvas").getContext("2d", { willReadFrequently: true });
    context.fillStyle = "#000";
    context.fillStyle = getComputedStyle(element).color;
    context.fillRect(0, 0, 1, 1);
    const [r, g, b] = context.getImageData(0, 0, 1, 1).data;
    const channel = (value) => {
      const scaled = value / 255;
      return scaled <= 0.03928 ? scaled / 12.92 : ((scaled + 0.055) / 1.055) ** 2.4;
    };
    const luminance = 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
    return 1.05 / (luminance + 0.05);
  });
}

const style = (page, selector, property) =>
  page.locator(selector).first().evaluate((element, name) => getComputedStyle(element).getPropertyValue(name), property);

test.describe("print", () => {
  for (const scheme of ["light", "dark"]) {
    test(`every theme state resolves to the light scheme in print (${scheme} preference)`, async ({ page }) => {
      await page.goto("/fixtures/plain-html/");
      for (const attribute of [null, "light", "dark"]) {
        await page.evaluate((value) => {
          if (value === null) document.documentElement.removeAttribute("data-ds-scheme");
          else document.documentElement.setAttribute("data-ds-scheme", value);
        }, attribute);
        await page.emulateMedia({ media: "print", colorScheme: scheme });
        await expect(page.locator("html")).toHaveCSS("color-scheme", "light");
        expect(await contrastAgainstWhite(page, "h1"), `${attribute} ${scheme}`).toBeGreaterThan(7);
      }
    });
  }

  test("the screen theme is unchanged by the print rule", async ({ page }) => {
    await page.goto("/fixtures/plain-html/");
    await page.evaluate(() => document.documentElement.setAttribute("data-ds-scheme", "dark"));
    await page.emulateMedia({ media: "screen", colorScheme: "light" });
    await expect(page.locator("html")).toHaveCSS("color-scheme", "dark");
  });

  test("code wraps in print and still scrolls on screen", async ({ page }) => {
    await page.goto("/fixtures/plain-html/");
    await page.setViewportSize({ width: 718, height: 900 });
    await page.emulateMedia({ media: "screen" });
    expect(await style(page, "pre", "overflow-x")).toBe("auto");
    await page.emulateMedia({ media: "print" });
    expect(await style(page, "pre", "white-space")).toBe("pre-wrap");
    expect(await style(page, "pre", "overflow-x")).toBe("visible");
    const overflow = await page.locator("pre").first().evaluate((element) => element.scrollWidth - element.clientWidth);
    expect(overflow).toBeLessThanOrEqual(1);
  });

  test("an open dialog sits in document flow in print without overlapping its neighbors", async ({ page }) => {
    await page.goto("/fixtures/plain-html/");
    await page.setViewportSize({ width: 718, height: 900 });
    await page.emulateMedia({ media: "print" });
    const dialog = page.locator("dialog[open]").first();
    await expect(dialog).toHaveCSS("position", "static");
    const overlaps = await dialog.evaluate((element) => {
      const box = element.getBoundingClientRect();
      const found = [];
      for (const sibling of element.parentElement.children) {
        if (sibling === element) continue;
        const other = sibling.getBoundingClientRect();
        if (other.width === 0 || other.height === 0) continue;
        const across = box.left < other.right && other.left < box.right;
        const down = box.top < other.bottom && other.top < box.bottom;
        if (across && down) found.push(sibling.tagName.toLowerCase());
      }
      return found;
    });
    expect(overlaps).toEqual([]);
  });

  test("a primary action label stays readable when the printer omits backgrounds", async ({ page }) => {
    await page.goto("/fixtures/primitives/");
    await page.emulateMedia({ media: "screen" });
    const accent = await style(page, ".ds-action-primary:not(:disabled)", "background-color");
    await page.emulateMedia({ media: "print" });
    const primary = ".ds-action-primary:not(:disabled)";
    expect(await contrastAgainstWhite(page, primary)).toBeGreaterThan(7);
    expect(await style(page, primary, "background-color")).toMatch(/^rgba\(0, 0, 0, 0\)$|^transparent$/);
    // The accent border still marks the primary action.
    expect(await style(page, primary, "border-top-color")).toBe(accent);
    // A later state rule still outranks the print rule: a disabled primary action prints muted.
    const disabled = ".ds-action-primary:disabled";
    if ((await page.locator(disabled).count()) > 0) {
      await page.emulateMedia({ media: "screen" });
      const muted = await style(page, disabled, "color");
      await page.emulateMedia({ media: "print" });
      expect(await style(page, disabled, "color")).toBe(muted);
    }
  });
});
