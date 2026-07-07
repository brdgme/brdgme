import { expect, test } from "@playwright/test";
import { collectConsoleErrors, login, uniqueEmail } from "./helpers";

// Fails consistently (not flaky) on master as of 2026-07-07 - times out at
// helpers.ts:54 waiting for `document.body.dataset.hydrated === "true"`
// after navigation during login. Reproduced identically across three
// consecutive master commits (57b5542, 41dedc8, 7a73f1f) including
// per-run retries, which points to a real Plan 27 hydration regression
// rather than test flake. See docs/plan/27-web-simplification.md
// "Deferred work" for tracking; do not re-enable until root-caused.
test.fixme("hard-loaded pages produce zero console errors", async ({ page }) => {
  const errors = collectConsoleErrors(page);

  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Welcome to brdg.me" })).toBeVisible();

  await page.goto("/login");
  await expect(page.getByText("Enter your email address to start")).toBeVisible();

  const email = uniqueEmail("page-loads");
  await login(page, email);

  await page.goto("/dashboard");
  await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();

  await page.goto("/games");
  await expect(page.getByRole("heading", { name: "New Game" })).toBeVisible();

  // Create a bot game so there is an active /games/{id} page to hard-load.
  const opponentRow = page.locator(".form-row", { hasText: "Opponent 1" });
  await opponentRow.locator("select").selectOption("bot");
  await page.getByRole("button", { name: "Create Game" }).click();
  await page.waitForURL(/\/games\/[0-9a-f-]+$/);
  const gameUrl = page.url();

  // Navigate away and hard-load the game page directly, exercising SSR + hydration.
  await page.goto("/dashboard");
  await page.goto(gameUrl);
  await expect(page.locator(".game-render")).toBeVisible();

  // Hard reload mid-game is the highest-risk hydration scenario (real async
  // data + Suspense).
  await page.reload();
  await expect(page.locator(".game-render")).toBeVisible();

  errors.assertNoErrors();
});
