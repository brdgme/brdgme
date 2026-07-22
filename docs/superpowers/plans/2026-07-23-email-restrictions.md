# Email Restrictions (Implementation Plan)

Research + planning doc only. No source was modified. All paths relative to
repo root. Crate is `rust/web` (Leptos SSR + WASM, Axum backend).

Requirements:
- R1 Block '+' (plus-addressing) in ALL email addresses universally - reject any
  email containing '+' before the '@'.
- R2 Static blocklist of disposable email domains - reject emails whose domain
  matches a curated embedded list.
- R3 Cloudflare Turnstile CAPTCHA widget on the login form - free, integrates
  with the existing CF zone on beta.brdg.me.
- R4 Clear error messages when blocked ("Plus-addressing is not supported" /
  "This email domain is not supported").
- R5 Grandfather existing verified users - restrictions apply only to NEW
  registrations and NEW email additions, not to existing verified addresses.
- R6 Global send cap (50/day) is sufficient for custom-domain spam; no
  per-domain rate limiting needed.
- R7 Cloudflare Bot Fight Mode already active; Turnstile is the additional layer.

---

## 1. Current behaviour

### Registration / login flow

All authentication is email-code based (no passwords). Flow:

1. User submits email on `LoginPage` (`app.rs:458`).
2. `login()` server fn (`auth/server.rs:234`) validates `!email.is_empty() &&
   email.contains('@')`, then calls `request_confirmation_code()`.
3. `request_confirmation_code()` (`auth/server.rs:130`) is the SINGLE CHOKEPOINT:
   advisory-lock transaction, GC stale rows, per-email cooldown (60s), per-email
   cap (5 sends/code-validity), global 24h cap (50), upsert code, send via Resend.
4. User enters code -> `confirm_login()` (`auth/server.rs:249`) validates code,
   creates user if new, sets session.

Secondary entry point: `add_email_address()` (`auth/server.rs:722`) - a signed-in
user adds a new address. Same validation (`is_empty || !contains('@')`), then
ownership check, insert unverified, call `request_confirmation_code()`.

### Current validation

Only check in both `login()` and `add_email_address()`:
```rust
if email.is_empty() || !email.contains('@') {
    return ... "Invalid email address"
}
```

No plus-address filtering, no domain blocklist, no CAPTCHA.

### Cloudflare setup (`infra/cloudflare.tf`)

- Zone `brdg.me` on free plan, `beta.brdg.me` proxied (orange-cloud).
- Bot Fight Mode active (`cloudflare_bot_management.brdgme`, `fight_mode = true`).
- Rate limit: 60 req/10s/IP on `/api/` prefix (covers all server fns).
- No Turnstile widget or site key configured yet.

### Existing rate limits (in-app)

- 60s resend cooldown per email (`LOGIN_RESEND_COOLDOWN_SECS`).
- 5 sends per email per code-validity window (`LOGIN_MAX_SENDS_PER_EMAIL`).
- 50 global sends per 24h (`LOGIN_GLOBAL_MAX_SENDS_PER_DAY`).
- 10 failed confirm attempts per code (`CONFIRM_MAX_ATTEMPTS_PER_CODE`).

---

## 2. Shared building blocks

### 2a. Email validation module

New file `rust/web/src/auth/email_validation.rs` (or inline in `server.rs` if
small enough). Exposes:

```rust
pub fn validate_email_restricted(email: &str) -> Result<(), &'static str>
```

Returns `Err` with a user-facing message if the email violates R1 or R2.
Order of checks:
1. Plus-address: if the local part (before '@') contains '+', return
   `Err("Plus-addressing is not supported")`.
2. Disposable domain: extract domain (after last '@'), lowercase, check against
   the blocklist. Return `Err("This email domain is not supported")`.

This is a pure fn, trivially unit-testable, no I/O.

### 2b. Disposable domain blocklist

Embed a static `HashSet<&'static str>` (or `phf::Set` for compile-time
perfection, but a `const` array + binary search or a `lazy_static` HashSet is
fine given the list is ~3-4k entries). Source: the `disposable-email-domains`
npm/crate list (https://github.com/disposable-email-domains/disposable-email-domains).
Options:
- Add the `disposable-email-domains` crate (if it exists on crates.io and is
  maintained) - check availability first.
- Otherwise: generate a Rust source file (`auth/blocked_domains.rs`) containing
  a `pub const BLOCKED_DOMAINS: &[&str] = &[...]` from the upstream JSON at
  build time or vendored once. A `HashSet` built via `std::sync::LazyLock`
  gives O(1) lookup.

The list is static (no runtime updates needed). Updating is a future manual
step (re-vendor the file).

### 2c. Turnstile token verification (backend)

Server-side POST to `https://challenges.cloudflare.com/turnstile/v0/siteverify`
with `secret` + `response` (token) + optional `remoteip`. Returns JSON
`{ "success": bool, "error-codes": [...] }`.

Use the existing `reqwest` dependency (already in Cargo.toml, SSR-gated). Add a
new SSR-only helper:

```rust
async fn verify_turnstile_token(token: &str, remote_ip: Option<&str>) -> Result<bool, ServerFnError>
```

Secret key from env var `TURNSTILE_SECRET_KEY` (read at startup, stored in
Axum state or `expect_context`). Site key from env var `TURNSTILE_SITE_KEY`
(injected into the HTML page for the widget script).

---

## 3. Implementation units (dependency order)

### E1. Plus-address rejection (backend, ~20 lines)

- **Goal:** R1 - reject any email with '+' in the local part.
- **Files:** `rust/web/src/auth/server.rs` (or new `auth/email_validation.rs`).
- **Change:**
  - Add a validation check in `login()` (`:234`) AFTER the existing
    `is_empty/contains('@')` check and BEFORE calling
    `request_confirmation_code()`:
    ```rust
    if email.split('@').next().is_some_and(|local| local.contains('+')) {
        return Ok(LoginResponse { success: false, message: "Plus-addressing is not supported".to_string() });
    }
    ```
  - Add the same check in `add_email_address()` (`:722`) returning
    `Err(ServerFnError::new("Plus-addressing is not supported"))`.
  - Grandfathering (R5): `confirm_login()` resolves existing users by email.
    An existing user whose email already has '+' can still log in because
    `login()` is called with THEIR email as-is. The restriction blocks NEW
    sign-ups and NEW address additions. No DB query needed for grandfathering -
    the check is at the "request a code" step, and existing users who already
    have a verified address with '+' will still match in `confirm_login`'s
    user lookup. However, if an existing user tries to RE-LOGIN with a
    plus-address that is NOT yet in the DB, they get blocked - correct
    behaviour (that's a new registration attempt).
- **Acceptance criteria:**
  - `login("user+tag@gmail.com")` returns `success: false` with the message.
  - `login("user@gmail.com")` still works.
  - `add_email_address("x+y@z.com")` returns an error.
  - Existing user with `user+tag@gmail.com` already verified can still call
    `confirm_login` (the code was already sent before this feature, or they
    log in with a non-plus address).
- **Tests:**
  - Unit test: `validate_email_restricted("a+b@c.com")` returns Err.
  - Unit test: `validate_email_restricted("a@b.com")` returns Ok.
  - `#[sqlx::test]`: `login()` with a plus-address returns the rejection
    message and does NOT insert a `login_confirmations` row.
- **Depends on:** nothing.

### E2. Disposable domain blocklist (backend, ~50 lines + data file)

- **Goal:** R2 - reject emails from known disposable domains.
- **Files:**
  - New `rust/web/src/auth/blocked_domains.rs` (the static list).
  - `rust/web/src/auth/server.rs` (or `email_validation.rs`) - the lookup.
  - `rust/web/src/auth/mod.rs` - add `mod blocked_domains;`.
- **Change:**
  - Vendor the disposable-email-domains list into `blocked_domains.rs` as a
    `pub const` slice or a `LazyLock<HashSet<&'static str>>`.
  - In `login()` and `add_email_address()`, after the plus-address check:
    ```rust
    let domain = email.rsplit('@').next().unwrap_or("").to_lowercase();
    if blocked_domains::is_blocked(&domain) {
        return Ok(LoginResponse { success: false, message: "This email domain is not supported".to_string() });
    }
    ```
  - Grandfathering (R5): same logic as E1 - the check is at code-request time.
    Existing verified users with a disposable domain who are already in the DB
    can still log in IF they already have a code outstanding or use the
    "I already have a login code" path. For a fresh login attempt (new code
    send), they ARE blocked. This is acceptable: the restriction prevents NEW
    registrations, and an existing user logging in again would need a new code.
    **Decision needed (D-grandfather-login):** should existing verified users
    with disposable domains be allowed to request new login codes? If yes,
    add a DB check: "does this email already belong to a verified user?" before
    rejecting. This adds one query to the hot path. Proposed: YES, check
    `find_email_owner` and skip the domain block if the email is already
    verified by an existing user. This keeps grandfathering airtight.
- **Acceptance criteria:**
  - `login("x@mailinator.com")` returns `success: false` with the domain message.
  - `login("x@gmail.com")` still works.
  - If D-grandfather-login is YES: an existing verified user with a disposable
    domain can still request a code.
- **Tests:**
  - Unit test: `is_blocked("mailinator.com")` true; `is_blocked("gmail.com")` false.
  - Unit test: case-insensitive (`"MAILINATOR.COM"` blocked).
  - `#[sqlx::test]`: login with disposable domain rejected, no row inserted.
  - `#[sqlx::test]` (if grandfathering): existing verified user with disposable
    domain can still get a code.
- **Depends on:** E1 (shares the validation insertion point; land E1 first to
  avoid conflicting edits in the same lines).

### E3. Cloudflare Turnstile - backend verification (backend, ~60 lines)

- **Goal:** R3 server-side - verify the Turnstile token on login.
- **Files:**
  - `rust/web/src/auth/server.rs` - new `verify_turnstile_token` helper +
    call site in `login()`.
  - `rust/web/src/main.rs` (or wherever app state is built) - read
    `TURNSTILE_SECRET_KEY` env var, store in state.
  - `rust/web/src/auth/server.rs` - `login()` signature gains a
    `turnstile_token: String` parameter.
- **Change:**
  - New env vars: `TURNSTILE_SITE_KEY`, `TURNSTILE_SECRET_KEY`.
  - New async helper (SSR-only):
    ```rust
    async fn verify_turnstile_token(secret: &str, token: &str) -> bool
    ```
    POST form-encoded `secret` + `response` to
    `https://challenges.cloudflare.com/turnstile/v0/siteverify`. Parse JSON
    response, return `success` field. On network error, fail OPEN (log a
    warning; do not block legitimate users if CF is down) - D-fail-open.
  - `login()` gains `turnstile_token: String` param. Before the email
    validation, verify the token. On failure, return
    `LoginResponse { success: false, message: "CAPTCHA verification failed. Please try again." }`.
  - If `TURNSTILE_SECRET_KEY` is unset/empty, skip verification entirely
    (dev/local mode without Turnstile).
  - `add_email_address()` does NOT get Turnstile (the user is already
    authenticated; the login form is the unauthenticated entry point).
- **Acceptance criteria:**
  - With a valid token, login proceeds normally.
  - With an invalid/empty token (and secret configured), login is rejected.
  - With secret unset (local dev), login works without a token.
  - Network failure to CF -> login proceeds (fail-open).
- **Tests:**
  - Unit test: `verify_turnstile_token` with a mock/stub (or test the
    parse logic with a canned JSON response).
  - `#[sqlx::test]`: `login()` with empty token + secret configured returns
    CAPTCHA failure.
  - `#[sqlx::test]`: `login()` with secret unset skips verification.
- **Depends on:** nothing (but touches `login()` signature - coordinate with
  E1/E2 which also modify `login()` body; recommended order: E1, E2, E3).

### E4. Cloudflare Turnstile - frontend widget (frontend, ~80 lines)

- **Goal:** R3 client-side - render the Turnstile widget on the login form and
  pass the token to the `login()` server fn.
- **Files:**
  - `rust/web/src/app.rs` - `LoginPage` component (`:458`).
  - Possibly `rust/web/index.html` or the Leptos `App` head - add the
    Turnstile script tag:
    `<script src="https://challenges.cloudflare.com/turnstile/v0/api.js" async defer></script>`.
  - `rust/web/src/auth/server.rs` - expose `TURNSTILE_SITE_KEY` to the client
    (via a server fn or env injection into the HTML template).
- **Change:**
  - Add the Turnstile JS script to the page `<head>` (via `leptos_meta`'s
    `Script` or directly in `index.html`). The script is external and
    hydration-safe (it does not alter DOM structure Leptos controls).
  - In `LoginPage`, render a `<div class="cf-turnstile" data-sitekey=...
    data-theme="auto"></div>` inside the email form (between the email input
    and the submit button). The widget renders into this div.
  - Site key delivery: a small server fn `get_turnstile_site_key() -> String`
    (returns the env var, or empty string if unset). `LoginPage` calls it via
    a `LocalResource` and only renders the widget div if the key is non-empty.
    Alternatively, inject the key into the initial HTML via the Axum
    `Html` response template (simpler, no extra round-trip).
  - On form submit (`on_email_submit`), read the Turnstile response token
    from the widget. The Turnstile API stores it in a hidden input named
    `cf-turnstile-response` inside the widget div, OR via
    `window.turnstile.getResponse()`. Read it via web-sys/js-sys:
    ```rust
    let token = web_sys::window()
        .and_then(|w| js_sys::Reflect::get(&w, &"turnstile".into()).ok())
        .and_then(|ts| js_sys::Reflect::get(&ts, &"getResponse".into()).ok())
        .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
        .and_then(|f| f.call0(&JsValue::UNDEFINED).ok())
        .and_then(|v| v.as_string())
        .unwrap_or_default();
    ```
  - Pass `token` as the new `turnstile_token` param to `login()`.
  - If site key is empty (dev mode), skip the widget and pass an empty token.
  - After a failed login attempt, reset the widget:
    `window.turnstile.reset()`.
  - **Hydration safety:** the widget div is always present in the DOM
    (rendered unconditionally when site key is non-empty). The Turnstile
    script injects an iframe INTO the div - this is post-hydration and does
    not cause structural mismatch. The `data-sitekey` attribute is set from
    a resource, so use an attribute binding (not a structural `<Show>`).
    If the site key resource is pending, render the div with an empty
    `data-sitekey` and let Turnstile init once the key arrives (or use a
    `Show` on a signal that is set once the key loads - this is a structural
    swap but happens BEFORE hydration completes if the resource resolves
    server-side via SSR). Safest: always render the div, set `data-sitekey`
    via `prop:` or `attr:` binding.
- **Acceptance criteria:**
  - Login page shows the Turnstile widget (a checkbox or invisible challenge).
  - Submitting the form sends the token to the server.
  - If Turnstile is not configured (no site key), the form works as before.
  - Hydration: no console errors on hard load (Playwright smoke test).
- **Tests:**
  - SSR page test: `/login` returns 200, no panic (`tests/ssr_pages.rs`).
  - Playwright hard-load smoke: zero console errors on `/login`.
  - Manual: verify widget renders and login works end-to-end on beta.
- **Depends on:** E3 (the `login()` fn must accept the token param).

### E5. Error messages and UX polish (frontend, ~20 lines)

- **Goal:** R4 - display clear, specific error messages for each rejection
  reason on both the login page and the settings "add email" form.
- **Files:**
  - `rust/web/src/app.rs` - `LoginPage` error display.
  - `rust/web/src/settings.rs` - add-email error display.
- **Change:**
  - `LoginPage` currently shows a generic "Failed to send login email. Please
    try again." on `login_action.value().is_err()`. But `login()` returns
    `Ok(LoginResponse { success: false, message })` for rejections (not an
    `Err`). Update the effect/display logic:
    - Show `resp.message` when `success == false` (covers plus-address,
      disposable domain, CAPTCHA failure, global cap).
    - Keep the generic error for `is_err()` (network/server failures).
  - `settings.rs` add-email: `add_email_address()` returns
    `Err(ServerFnError::new(msg))` - the existing error display
    (`error.set(Some(e.to_string()))`) already shows the message. Verify it
    renders the specific strings from E1/E2.
  - Style: reuse existing `.error` class (already used on login page).
- **Acceptance criteria:**
  - Plus-address rejection shows "Plus-addressing is not supported" on the
    login page.
  - Disposable domain shows "This email domain is not supported".
  - CAPTCHA failure shows "CAPTCHA verification failed. Please try again."
  - Settings page shows the same messages for add-email rejections.
- **Tests:**
  - SSR page test stays green.
  - Manual verification on beta.
- **Depends on:** E1, E2, E3, E4 (needs the rejection paths and messages to
  exist).

---

## 4. Decisions for the user

1. **D-grandfather-login - existing users with disposable domains:** should an
   existing verified user whose email is on the disposable blocklist be allowed
   to request new login codes? Proposed: YES - add a `find_email_owner` check
   before the domain rejection; if the email belongs to an existing verified
   user, skip the block. Cost: one extra indexed query on the login hot path
   (only for blocked domains, so negligible). Alternative: block them too
   (harsher, forces them to add a non-disposable address). Confirm.

2. **D-fail-open - Turnstile network failure:** if the server cannot reach
   Cloudflare's siteverify endpoint, should login proceed (fail-open, proposed)
   or be rejected (fail-closed)? Fail-open risks bots during a CF outage;
   fail-closed risks locking out all users. Proposed: fail-open with a
   tracing::warn + Prometheus counter.

3. **D-site-key-delivery - how the frontend gets the Turnstile site key:**
   (a) inject into the HTML template at SSR time (simplest, no extra request),
   or (b) a `get_turnstile_site_key()` server fn called by a `LocalResource`.
   Proposed: (a) - pass it through the Leptos `App` context or a `<meta>` tag
   read by the widget init. Alternatively, hardcode the site key in the
   frontend (it is public by design) - but env-driven is cleaner for
   multi-env (staging/prod use different keys).

4. **D-blocklist-source - disposable domain list provenance:** use the
   `disposable-email-domains` crate if available on crates.io, or vendor the
   list from the GitHub repo's JSON into a generated `.rs` file? Proposed:
   vendor once into `auth/blocked_domains.rs` with a comment noting the source
   URL and date. Update manually as needed.

5. **D-turnstile-theme - widget appearance:** Turnstile supports
   `data-theme="light|dark|auto"`. Proposed: `auto` (respects user's
   OS preference). The login page is minimal (no dark mode in the app yet),
   so `light` is also fine.

---

## 5. Known issues / gotchas

- **Migrations are immutable.** This feature needs NO migration - all checks
  are application-level. No new DB columns or tables.

- **SQLX_OFFLINE=true for clippy/check.** Canonical gates (DEV.md):
  `cargo fmt --all -- --check`;
  `cargo clippy -p web --all-targets --features ssr -- -D warnings`;
  `cargo clippy --workspace --exclude web --all-targets -- -D warnings`;
  `cargo test -p web --features ssr` (needs live Postgres).

- **DB tests need real Postgres.** Plain local runs fail DB tests (pre-existing,
  backlog #40). Use `scripts/rust-test.sh`.

- **`login()` signature change (E3):** adding `turnstile_token: String` changes
  the server fn's public API. The `LoginPage` call site and any tests calling
  `login()` directly must be updated. The `#[server]` macro generates the
  endpoint from the fn signature - the WASM client and SSR server must agree.
  Land E3 and E4 together (or E3 first with a default-empty token from the
  existing UI, then E4 wires the real token).

- **Hydration safety (`docs/hydration.md`):** the Turnstile widget div must be
  structurally stable across SSR and hydration. Do NOT conditionally render it
  with `<Show>` based on an async resource that resolves differently on server
  vs client. Safest: always render the div, set `data-sitekey` from a
  synchronously-available value (injected at SSR time via context or a const).
  The Turnstile script loads `async defer` and initializes post-hydration.

- **Turnstile script is external:** add it to `index.html` or via
  `leptos_meta::Script`. It must load on the `/login` page. If added globally
  in `index.html`, it loads on every page (small overhead, ~30KB). Acceptable
  for now; can be scoped later if needed.

- **`reqwest` is already a dependency** (SSR-gated, `rustls` feature). The
  Turnstile verification POST uses it. No new dependency needed for the HTTP
  call.

- **No new crate for the blocklist** unless `disposable-email-domains` exists
  on crates.io. Prefer vendoring to avoid a supply-chain dependency for a
  static list.

- **Grandfathering is enforcement-point-based, not DB-flag-based.** There is no
  "restricted" boolean on users/emails. The restrictions live at the
  code-request chokepoint. Existing verified addresses are never re-validated.
  This means: if an existing user with `user+tag@x.com` is already in the DB,
  they can still log in via `confirm_login` (code already sent or via the
  "I already have a code" path). They just cannot request a NEW code with a
  plus-address (unless D-grandfather-login adds the exemption for disposable
  domains; plus-addresses have no exemption - an existing plus-address user
  who needs a new code is stuck; this is acceptable and intentional per R1's
  "universally" wording). **Clarification:** if this is too harsh for existing
  plus-address users, the same `find_email_owner` exemption could apply to R1
  too. Flag for user decision.

- **Org is `brdgme`** (not `beefsack`) for any image/URL references.

- **Terraform for Turnstile:** the Turnstile site/secret keys are created in
  the CF dashboard (or via the `cloudflare_turnstile_widget` Terraform
  resource). The keys then go into the app's environment (sealed-secret in
  k8s, or `vars` in Terraform -> Secret). This is an infra step outside the
  Rust crate - document it but do not block the app code on it (the app
  degrades gracefully with keys unset).

- **Pre-existing flake:** `invite_expiry_threshold_defaults_to_14_days` (env
  race) - do not chase it as a regression.

---

## Suggested commit order

E1, E2, E3, E4, E5. E1-E3 are backend-only and can be verified independently.
E4 is frontend and depends on E3's signature change. E5 is a thin UX layer on
top. E3+E4 may land as one commit if the signature change makes intermediate
states awkward.
