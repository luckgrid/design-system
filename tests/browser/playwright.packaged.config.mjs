import { defineConfig, devices } from "@playwright/test";

// Release verification only: runs the packaged-archive specs. Requires
// DS_PACKAGED_ROOT (an unpacked release archive outside this repository); see
// release/verify-packaged.sh, which sets it and starts this run.
const port = Number(process.env.DS_BROWSER_PORT ?? 4173);
const baseURL = `http://127.0.0.1:${port}`;

if (!process.env.DS_PACKAGED_ROOT) {
  throw new Error("DS_PACKAGED_ROOT is required for the packaged-archive specs");
}

export default defineConfig({
  testDir: "./packaged",
  fullyParallel: true,
  forbidOnly: true,
  retries: 0,
  reporter: [["list"]],
  use: { baseURL },
  webServer: {
    command: "node server.mjs",
    url: `${baseURL}/tests/browser/probes/blank.html`,
    reuseExistingServer: false,
  },
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"] } },
    { name: "firefox", use: { ...devices["Desktop Firefox"] } },
    { name: "webkit", use: { ...devices["Desktop Safari"] } },
  ],
});
