import { Page } from "@playwright/test";
import { Client } from "pg";

const DATABASE_URL =
  process.env.E2E_DATABASE_URL ??
  "postgres://brdgme_user:brdgme_password@localhost:5432/brdgme_e2e";

let uniqueCounter = 0;

/** A per-run-unique email so tests don't collide with each other's users. */
export function uniqueEmail(label: string): string {
  uniqueCounter += 1;
  return `e2e-${label}-${Date.now()}-${uniqueCounter}@example.com`;
}

async function fetchLoginConfirmation(email: string): Promise<string> {
  const client = new Client({ connectionString: DATABASE_URL });
  await client.connect();
  try {
    for (let attempt = 0; attempt < 20; attempt += 1) {
      const result = await client.query(
        `SELECT code FROM login_confirmations WHERE email = $1`,
        [email],
      );
      const code = result.rows[0]?.code as string | undefined;
      if (code) {
        return code;
      }
      await new Promise((resolve) => setTimeout(resolve, 250));
    }
    throw new Error(`Timed out waiting for login_confirmations code for ${email}`);
  } finally {
    await client.end();
  }
}

/**
 * Drives the login UI end-to-end for a fresh email: submits the email form,
 * reads the confirmation code straight out of Postgres (SMTP is unset in the
 * e2e stack), then submits the code. Leaves the page on /.
 */
export async function login(page: Page, email: string): Promise<void> {
  await page.goto("/login");
  // Wait for WASM hydration to finish attaching event listeners before
  // interacting with the form. `networkidle` isn't reliable here: it only
  // guarantees network requests have settled, not that the WASM module has
  // finished instantiating and running `hydrate()`. If we click too early,
  // the submit falls back to a native (unhandled) form submission that just
  // reloads the page instead of invoking the login server function. The
  // `hydrate()` fn sets `document.body.dataset.hydrated` once it's done, so
  // wait on that definitive signal instead.
  await page.waitForFunction(() => document.body.dataset.hydrated === "true");
  await page.getByPlaceholder("Email address").first().fill(email);
  await page.getByRole("button", { name: "Get code" }).click();

  const code = await fetchLoginConfirmation(email);

  await page.getByPlaceholder("Login code").fill(code);
  await page.getByRole("button", { name: "Play!" }).click();
  await page.waitForURL("**/");
}

/**
 * Collects `console.error` messages and uncaught page errors for the
 * lifetime of the page. Hydration panics surface here via
 * `console_error_panic_hook`. Call `assertNoErrors()` at the end of a test.
 */
export function collectConsoleErrors(page: Page): { assertNoErrors: () => void } {
  const errors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") {
      errors.push(`console.error: ${msg.text()}`);
    }
  });
  page.on("pageerror", (err) => {
    errors.push(`pageerror: ${err.message}`);
  });
  return {
    assertNoErrors: () => {
      if (errors.length > 0) {
        throw new Error(`Unexpected console errors:\n${errors.join("\n")}`);
      }
    },
  };
}
