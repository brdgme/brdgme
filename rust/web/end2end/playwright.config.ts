import { devices, defineConfig } from "@playwright/test";

/**
 * E2E smoke suite against a locally-started release build of the web binary
 * and the lost_cities_2_http game service - see run.sh. Chromium only,
 * single worker: this is a smoke suite, not a cross-browser regression suite.
 * See https://playwright.dev/docs/test-configuration.
 */
export default defineConfig({
  testDir: "./tests",
  timeout: 30 * 1000,
  expect: {
    timeout: 5000,
  },
  fullyParallel: true,
  /* Fail the build on CI if you accidentally left test.only in the source code. */
  forbidOnly: !!process.env.CI,
  /* Retry on CI only */
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  reporter: "html",
  use: {
    baseURL: "http://127.0.0.1:3010",
    actionTimeout: 0,
    trace: "on-first-retry",
  },

  projects: [
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
        // In Nix/devenv environments Playwright's downloaded Chromium often
        // can't launch (missing system libs, no root). Set E2E_CHROMIUM_PATH
        // to a working Chromium binary (e.g. `command -v chromium`) to use
        // it instead; CI leaves this unset and uses Playwright's own browser.
        ...(process.env.E2E_CHROMIUM_PATH
          ? {
              launchOptions: {
                executablePath: process.env.E2E_CHROMIUM_PATH,
                args: ["--no-sandbox"],
              },
            }
          : {}),
      },
    },
  ],
});
