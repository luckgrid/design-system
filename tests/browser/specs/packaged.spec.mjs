import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import path from "node:path";

import { expect, loadStylesheets, probeColor, test } from "./support.mjs";

// The packaged-archive specs. Every stylesheet here is served from an UNPACKED release
// archive outside the repository (server.mjs, DS_PACKAGED_ROOT). They prove the final
// supported load path, /design-system/core.css, and re-check the historical Chromium
// @import layer-order race against the packaged entrypoint.
const ROOT = process.env.DS_PACKAGED_ROOT;
const PAGE = "/tests/browser/probes/packaged-load.html";
const REVERSED = "/tests/browser/probes/ds-layers-reversed.css";
const CORE = "/design-system/core.css";
const ITERATIONS = Number(process.env.DS_RACE_ITERATIONS ?? 100);

/** The archive's core stylesheets: MANIFEST.tsv css rows, without the Tailwind adapter. */
const archiveStylesheets = readFileSync(path.join(ROOT, "MANIFEST.tsv"), "utf8")
  .split("\n")
  .filter((line) => line && !line.startsWith("#"))
  .map((line) => line.split("\t"))
  .filter(([, , , kind, file]) => kind === "css" && file !== "css/tailwind.css")
  .map(([sha256, , , , file]) => ({ sha256, file, url: `/design-system/${file.slice("css/".length)}` }));

test.describe("packaged release archive", () => {
  test("the archive holds the whole core CSS the page loads, and nothing else is fetched from it", async ({ page }) => {
    const seen = [];
    page.on("response", (response) => {
      const { pathname } = new URL(response.url());
      if (pathname.startsWith("/design-system/")) {
        seen.push({ pathname, source: response.headers()["x-ds-source"] });
      }
    });
    await page.goto(PAGE);
    await page.waitForLoadState("networkidle");
    expect(seen.length).toBeGreaterThan(0);
    for (const entry of seen) {
      expect(entry.source, `${entry.pathname} served from the archive`).toBe("packaged-archive");
    }
    expect(seen.map((entry) => entry.pathname).sort()).toEqual(archiveStylesheets.map((entry) => entry.url).sort());
  });

  test("the bytes served equal the manifest checksums", async ({ request }) => {
    for (const { sha256, url } of archiveStylesheets) {
      const response = await request.get(url);
      expect(response.status(), url).toBe(200);
      const digest = createHash("sha256").update(await response.body()).digest("hex");
      expect(digest, url).toBe(sha256);
    }
  });

  test("a file outside the archive graph is not served", async ({ request }) => {
    expect((await request.get("/design-system/README.md")).status()).toBe(404);
    expect((await request.get("/design-system/tailwind.css")).status()).toBe(404);
    expect((await request.get("/packages/styles/README.md")).status()).toBe(404);
  });

  test(`the packaged entrypoint's order wins on ${ITERATIONS} head-link loads`, async ({ page }) => {
    const colors = new Set();
    for (let index = 0; index < ITERATIONS; index += 1) {
      await page.goto(PAGE, { waitUntil: "load" });
      colors.add(await probeColor(page));
    }
    expect([...colors]).toEqual(["rgb(6, 0, 0)"]);
  });

  test(`the packaged entrypoint's order wins on ${ITERATIONS} dynamic-link loads`, async ({ page }) => {
    const colors = new Set();
    for (let index = 0; index < ITERATIONS; index += 1) {
      await page.goto("/tests/browser/probes/blank.html");
      await loadStylesheets(page, [CORE]);
      await loadStylesheets(page, [REVERSED]);
      colors.add(await probeColor(page));
    }
    expect([...colors]).toEqual(["rgb(6, 0, 0)"]);
  });
});
