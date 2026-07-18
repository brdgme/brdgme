import { expect, test } from "@playwright/test";
import { collectConsoleErrors, login, uniqueEmail } from "./helpers";

test("hard-loaded pages produce zero console errors", async ({ page }) => {
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
  await page.waitForFunction(() => document.body.dataset.hydrated === "true");
  await expect(page.locator(".game-render")).toBeVisible();

  // Hard reload mid-game is the highest-risk hydration scenario (real async
  // data + Suspense).
  await page.reload();
  await page.waitForFunction(() => document.body.dataset.hydrated === "true");
  await expect(page.locator(".game-render")).toBeVisible();

  errors.assertNoErrors();
});
