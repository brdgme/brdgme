# Pre-Go-Live UI/UX Polish Batch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the nine jank items recorded in `docs/pre-go-live-polish.md` before go-live, each as an independent, individually-committed task.

**Architecture:** Nine tasks, one per polish-doc entry, touching `rust/web` (Leptos 0.8 SSR+hydrate app, Axum server) and `.github/workflows/ci.yml`. Ordered so independent, low-risk, single-file tasks land first; the two tasks that share a code dependency (sidebar reload fix, then the title badge that reads the same lifted resource) are sequenced back to back; the autofocus task lands last because it extends a file two earlier tasks already touched. Every task's exact code was written against the real files in this repo and verified with `cargo check` (ssr + hydrate/wasm32 targets), `cargo clippy -D warnings`, `cargo fmt --check`, and `cargo test -p web --features ssr` (58 lib tests + 15 integration tests, all green) before this plan was written - see each task's Steps for the exact commands.

**Tech Stack:** Rust 2024 edition, Leptos 0.8.20 (SSR + hydrate) + leptos_router 0.8.14 + leptos_meta, Axum, sqlx/Postgres, resend-rs 0.27 (Resend HTTP email API), GitHub Actions + dorny/paths-filter v4.

## Global Constraints

- ASCII-only source edits: no em dashes, no smart/curly quotes, no ellipsis character, no Unicode bullets - use hyphens/asterisks and plain three-dot `...` if ever needed.
- User-facing branding is always **"brdg.me"**, never "brdgme" (polish entry 4's core complaint - the current email subject literally says "Your brdgme login code").
- The type-anywhere-focus behavior (entry 7) must never intercept a keystroke when a link, button, or other input already has focus - Enter on a focused link must keep navigating normally. Only single, unmodified, printable-character keydowns are eligible to divert, and only when nothing is focused.
- Do not implement backlog #27 WP3 (deleting `WebSocketTrigger`, merging it into the per-game `RwSignal<Option<(Uuid, u64)>>` context, or deduplicating the `GameLogs`/`RecentGameLogs` fetch) as part of this plan - `docs/superpowers/plans/2026-07-07-27-web-simplification.md` "Deferred work" item 1 owns that. Entry 2's fix here is additive only: it hoists the *existing* `LocalResource`s and `ServerAction<Logout>` to a shared context so they survive client-side navigation; it does not touch `WebSocketTrigger`, does not change what signal re-keys them, and does not touch `GameLogs`/`RecentGameLogs`.
- Rust edition 2024 (`rust/Cargo.toml`, `rust/web/Cargo.toml`); toolchain per `rust/Dockerfile`'s pinned `rust:1.94.0`.
- Commit messages follow this repo's `<Area> #33: <what>` style (see `git log`), ending with the trailer `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`.
- Never call a real game/network service from a test - this plan's one new Rust test (Task 3) is a pure function test with no I/O, matching `docs/CODING.md`'s existing convention.

---

### Task 1: Replace the default favicon with a brdg.me dice SVG

**Files:**
- Create: `rust/web/public/favicon.svg`
- Modify: `rust/web/src/app.rs` (`shell()`, the `<head>` block)

**Interfaces:**
- Produces: `/favicon.svg` served as a static asset (cargo-leptos copies `assets-dir = "public"` into the site root the same way it already serves `/favicon.ico` and `/fonts/...` today - no server wiring needed beyond the `<link>` tag).

- [ ] **Step 1: Add the SVG**

Flat, material-style die showing the 6 face, two colors only (per the polish entry): `#ffffff` body, `#e0e0e0` for both the pips and the outline, no gradients.

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
  <rect x="3" y="3" width="26" height="26" rx="6" ry="6" fill="#ffffff" stroke="#e0e0e0" stroke-width="2.5"/>
  <circle cx="10.5" cy="9.5" r="2.3" fill="#e0e0e0"/>
  <circle cx="10.5" cy="16" r="2.3" fill="#e0e0e0"/>
  <circle cx="10.5" cy="22.5" r="2.3" fill="#e0e0e0"/>
  <circle cx="21.5" cy="9.5" r="2.3" fill="#e0e0e0"/>
  <circle cx="21.5" cy="16" r="2.3" fill="#e0e0e0"/>
  <circle cx="21.5" cy="22.5" r="2.3" fill="#e0e0e0"/>
</svg>
```

Save this to `rust/web/public/favicon.svg`.

- [ ] **Step 2: Wire it up in the document head**

In `rust/web/src/app.rs`, find `pub fn shell(options: LeptosOptions)`:

```rust
                <meta name="apple-mobile-web-app-capable" content="yes"/>
                <meta name="mobile-web-app-capable" content="yes"/>
                <AutoReload options=options.clone() />
```

Add a `<link rel="icon">` between the viewport meta tags and `<AutoReload>`:

```rust
                <meta name="apple-mobile-web-app-capable" content="yes"/>
                <meta name="mobile-web-app-capable" content="yes"/>
                <link rel="icon" type="image/svg+xml" href="/favicon.svg"/>
                <AutoReload options=options.clone() />
```

- [ ] **Step 3: Delete the stale default favicon**

The polish entry's Observed state is "the site still serves the default Leptos favicon" - `rust/web/public/favicon.ico` is that default. Delete it so nothing falls back to it:

```bash
git rm rust/web/public/favicon.ico
```

- [ ] **Step 4: Verify it compiles**

Run: `cd rust && cargo check -p web --features ssr --no-default-features`
Expected: `Finished` with no errors (the `<link>` tag is plain HTML in the `view!` macro, nothing Rust-typed to break).

- [ ] **Step 5: Manual verification**

Bring up the dev stack (`docs/DEV.md`): `tilt up`, wait for the `web` resource to report ready, open `http://localhost:3000`. Check the browser tab: it should show the two-tone dice icon, not the Leptos default. Hard-refresh (Ctrl+Shift+R) to bypass any cached favicon from a previous run.

- [ ] **Step 6: Commit**

```bash
git add rust/web/public/favicon.svg rust/web/src/app.rs
git rm rust/web/public/favicon.ico
git commit -m "$(cat <<'EOF'
web #33: replace default favicon with brdg.me dice SVG

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Gate CI jobs on changed paths

**Files:**
- Modify: `.github/workflows/ci.yml` (whole file - every job gets a `needs`/`if` addition, plus one new `changes` job)

**Interfaces:**
- Produces: job outputs `needs.changes.outputs.{rust,go,k8s,legacy}` (each `'true'`/`'false'`), consumed by every other job's `if:`.

**Verified path coverage (read every Dockerfile before writing filters):**
- `rust/Dockerfile` (`build-rust`, all `rust/**` images) does `COPY rust .` with build context `.` - only depends on `rust/**` and `docker-bake.hcl`. Matches the polish doc's rough gating exactly.
- `brdgme-go/Dockerfile` (`test-go`, `build-go-games`) does `COPY brdgme-go brdgme-go` **and** `COPY go.mod .` - the polish doc's rough gating (`brdgme-go/**` only) misses `go.mod`. Added it below. (No `go.sum` exists in this repo - `go.mod` alone is `go 1.15` with no external requires.)
- `rust/api/Dockerfile` (`build-legacy`'s `api` matrix leg) does `COPY rust/ .` (the **whole** `rust/` tree, not just `rust/api/`) then `cargo build -p brdgme_api`. `brdgme_api`'s `Cargo.toml` path-depends on `rust/lib/{cmd,game,color,markup}` and resolves against the workspace-level `rust/Cargo.toml`/`rust/Cargo.lock`. The polish doc's rough gating (`rust/api/** ` only) misses all of that - a change to e.g. `rust/lib/game` would silently not rebuild `api`. Added `rust/lib/**`, `rust/Cargo.toml`, `rust/Cargo.lock` to the `legacy` filter group below.
- `web/Dockerfile` / `websocket/Dockerfile` only `COPY web .` / `COPY websocket .` respectively (plus their own `package*.json`, already inside those dirs) - `web/**` / `websocket/**` alone is correct, matches the doc.
- `kubeconform` runs `kustomize build k8s/dev` / `k8s/prod`; both kustomizations only reference `../base/*` paths inside `k8s/` - `k8s/**` alone is correct.
- Deliberate deviation from the doc's literal per-group list: `.github/workflows/ci.yml` itself is added to **every** filter group (the doc only listed it for two of the four), so a workflow-file change always re-validates every gated job instead of silently shipping a gating bug in an ungated group.

- [ ] **Step 1: Add the gate job**

In `.github/workflows/ci.yml`, right after the `jobs:` line (before `test-rust:`):

```yaml
jobs:
  # Single gate job computing which path groups changed, so every other job
  # can skip with an `if:` instead of the workflow-level `on.paths` filter
  # (which would make GitHub report skipped jobs as *missing*, breaking
  # required-status checks that name them). A skipped job here still reports
  # a (skipped) status, which satisfies required checks.
  changes:
    runs-on: ubuntu-latest
    outputs:
      rust: ${{ steps.filter.outputs.rust }}
      go: ${{ steps.filter.outputs.go }}
      k8s: ${{ steps.filter.outputs.k8s }}
      legacy: ${{ steps.filter.outputs.legacy }}
    steps:
      - uses: actions/checkout@v7
      - uses: dorny/paths-filter@v4
        id: filter
        with:
          filters: |
            rust:
              - 'rust/**'
              - 'docker-bake.hcl'
              - '.github/workflows/ci.yml'
            go:
              - 'brdgme-go/**'
              - 'go.mod'
              - '.github/workflows/ci.yml'
            k8s:
              - 'k8s/**'
              - '.github/workflows/ci.yml'
            # rust/api/Dockerfile builds with context `.` and `COPY rust/ .`,
            # then `cargo build -p brdgme_api` - brdgme_api path-depends on
            # rust/lib/{cmd,game,color,markup} and resolves against the
            # workspace-level Cargo.toml/Cargo.lock, so all of those must
            # gate this build too, not just rust/api/**.
            legacy:
              - 'web/**'
              - 'websocket/**'
              - 'rust/api/**'
              - 'rust/lib/**'
              - 'rust/Cargo.toml'
              - 'rust/Cargo.lock'
              - '.github/workflows/ci.yml'

  test-rust:
    needs: [changes]
    if: needs.changes.outputs.rust == 'true'
    runs-on: ubuntu-latest
```

(That last `runs-on: ubuntu-latest` line already exists in the file as the second line of the `test-rust:` job - just add the `needs`/`if` above it, do not duplicate it.)

- [ ] **Step 2: Gate `cargo-deny`**

Find:
```yaml
  cargo-deny:
    runs-on: ubuntu-latest
```
Replace with:
```yaml
  cargo-deny:
    needs: [changes]
    if: needs.changes.outputs.rust == 'true'
    runs-on: ubuntu-latest
```

- [ ] **Step 3: Gate `kubeconform`**

Find:
```yaml
  kubeconform:
    runs-on: ubuntu-latest
```
Replace with:
```yaml
  kubeconform:
    needs: [changes]
    if: needs.changes.outputs.k8s == 'true'
    runs-on: ubuntu-latest
```

- [ ] **Step 4: Gate `e2e`**

Find:
```yaml
  e2e:
    runs-on: ubuntu-latest
    # Flaky (hydration-race in the login flow, still being tracked down) -
    # don't let it block merges/deploys while that's investigated.
    continue-on-error: true
```
Replace with:
```yaml
  e2e:
    needs: [changes]
    if: needs.changes.outputs.rust == 'true'
    runs-on: ubuntu-latest
    # Flaky (hydration-race in the login flow, still being tracked down) -
    # don't let it block merges/deploys while that's investigated.
    continue-on-error: true
```

- [ ] **Step 5: Gate `test-go`**

Find:
```yaml
  test-go:
    runs-on: ubuntu-latest
    steps:
```
Replace with:
```yaml
  test-go:
    needs: [changes]
    if: needs.changes.outputs.go == 'true'
    runs-on: ubuntu-latest
    steps:
```

- [ ] **Step 6: Gate `build-rust`, handling its existing `needs: [test-rust]`**

Find:
```yaml
  build-rust:
    needs: [test-rust]
    runs-on: ubuntu-latest
```
Replace with:
```yaml
  build-rust:
    needs: [changes, test-rust]
    # `test-rust` reports "skipped" (not "success") when the gate skips it,
    # so this can't rely on the default implicit `success()` check - it
    # would skip build-rust too. Proceed on skip, still block on a real
    # failure/cancellation.
    if: |
      needs.changes.outputs.rust == 'true' &&
      needs.test-rust.result != 'failure' &&
      needs.test-rust.result != 'cancelled'
    runs-on: ubuntu-latest
```

- [ ] **Step 7: Gate `build-legacy`**

Find:
```yaml
  build-legacy:
    runs-on: ubuntu-latest
```
Replace with:
```yaml
  build-legacy:
    needs: [changes]
    if: needs.changes.outputs.legacy == 'true'
    runs-on: ubuntu-latest
```

- [ ] **Step 8: Gate `build-go-games`, handling its existing `needs: [test-go]`**

Find:
```yaml
  build-go-games:
    needs: [test-go]
    runs-on: ubuntu-latest
```
Replace with:
```yaml
  build-go-games:
    needs: [changes, test-go]
    # See build-rust's comment: proceed when test-go was skipped, still
    # block on a real failure/cancellation.
    if: |
      needs.changes.outputs.go == 'true' &&
      needs.test-go.result != 'failure' &&
      needs.test-go.result != 'cancelled'
    runs-on: ubuntu-latest
```

- [ ] **Step 9: Validate YAML syntax**

Run: `yq eval '.' .github/workflows/ci.yml > /dev/null && echo "YAML VALID"`
Expected: `YAML VALID` (this only checks the file parses - it does not check GitHub Actions expression semantics; that's Step 10).

Run: `yq eval '.jobs | keys' .github/workflows/ci.yml`
Expected: 9 job names in this order: `changes`, `test-rust`, `cargo-deny`, `kubeconform`, `e2e`, `test-go`, `build-rust`, `build-legacy`, `build-go-games`.

- [ ] **Step 10: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "$(cat <<'EOF'
Infra #33: gate CI jobs on changed paths

dorny/paths-filter gate job + per-job `if:` on its outputs, keeping a
single ci.yml so `needs:` chains and required-status checks still work
(unlike workflow-level `on.paths`, which makes skipped-but-required jobs
report as missing rather than skipped).

Corrects two gaps in the original rough path list: brdgme-go/Dockerfile
also COPYs the root go.mod, and rust/api/Dockerfile COPYs the whole rust/
tree (brdgme_api path-depends on rust/lib/* and the workspace
Cargo.toml/Cargo.lock, not just rust/api/**).

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 11: Manual verification (real CI, not locally reproducible)**

This step exercises the actual gating logic on GitHub Actions, since there is no local GitHub Actions runner in this repo's toolset.

1. Push this commit's branch and open a PR (or, if working directly on a feature branch that will be reviewed, just push it).
2. In the PR's "Checks" tab, confirm the `changes` job ran and the jobs whose path group didn't change in this diff (this commit only touches `.github/workflows/ci.yml`, so **every** group's filter list includes it and **all** jobs should run this one time) show as run, not skipped.
3. Make a second, throwaway commit on the same branch touching only a docs file (e.g. append a blank line to `README.md`), push it. Confirm in the Checks tab: `changes` runs, and `test-rust`/`cargo-deny`/`kubeconform`/`e2e`/`test-go`/`build-rust`/`build-legacy`/`build-go-games` all show **Skipped** (not missing, not failed).
4. Amend that commit (or add a third) touching only `rust/web/src/lib.rs` with a no-op comment. Confirm `test-rust`, `cargo-deny`, `build-rust`, `e2e` run; `test-go`, `build-go-games`, `kubeconform`, `build-legacy` stay skipped.
5. Revert/drop the throwaway docs+lib.rs commits before merging (`git reset --soft` back to the gating commit, or squash them out), so this task's actual diff stays just the workflow file.

---

### Task 3: Fix login confirmation email branding and styling

**Files:**
- Modify: `rust/web/src/auth/server.rs` (`send_login_email`, plus its test module)

**Interfaces:**
- Produces: `login_email_bodies(token: &str) -> (String, String)` (returns `(text_body, html_body)`), a private `#[cfg(feature = "ssr")]` helper only `send_login_email` calls.

**Note on the legacy copy's expiry time:** the polish entry quotes legacy wording with "expire in 30 minutes", but this codebase's actual validity window (`login()`'s `code_valid = row.created_at > now - time::Duration::hours(1)`, `auth/server.rs`) is **1 hour**, not 30 minutes, and this task does not touch that logic. Sending an email that claims a shorter expiry than reality is a worse bug than not matching the legacy copy byte-for-byte, so this task's copy says "1 hour" instead of the legacy doc's literal "30 minutes". **Flag this for confirmation** - if the intent was actually to shorten the real validity window to 30 minutes to match legacy, that is a separate, out-of-scope change to `LOGIN_RESEND_COOLDOWN_SECS`-adjacent logic, not this polish item.

- [ ] **Step 1: Write the failing test**

In `rust/web/src/auth/server.rs`, find the `#[cfg(test)] mod tests` block's existing email test:

```rust
    async fn send_login_email_logs_when_resend_unset() {
        // Must not panic or attempt any network I/O when `resend` is `None`.
        send_login_email(None, "someone@example.com", "123456").await;
    }

    #[sqlx::test]
    async fn confirm_login_rejects_unknown_email(pool: PgPool) {
```

Insert a new test between them:

```rust
    async fn send_login_email_logs_when_resend_unset() {
        // Must not panic or attempt any network I/O when `resend` is `None`.
        send_login_email(None, "someone@example.com", "123456").await;
    }

    #[test]
    fn login_email_bodies_use_brdg_me_branding_and_token() {
        let (text, html) = login_email_bodies("643856");

        assert!(text.contains("Your brdg.me confirmation is 643856"));
        assert!(text.contains("expire in 1 hour"));
        assert!(
            !text.contains("brdgme"),
            "must never render unbranded 'brdgme': {text}"
        );

        assert!(html.contains("<b>643856</b>"));
        assert!(html.contains("Source Code Pro"));
        assert!(html.contains("background-color: white"));
        assert!(html.contains("color: black"));
        assert!(
            !html.contains("brdgme"),
            "must never render unbranded 'brdgme': {html}"
        );
    }

    #[sqlx::test]
    async fn confirm_login_rejects_unknown_email(pool: PgPool) {
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `cd rust && cargo test -p web --features ssr --lib login_email_bodies`
Expected: compile error - `cannot find function 'login_email_bodies' in this scope` (it doesn't exist yet).

- [ ] **Step 3: Add `login_email_bodies` and use it in `send_login_email`**

Find the current `send_login_email`:

```rust
#[cfg(feature = "ssr")]
async fn send_login_email(resend: Option<&resend_rs::Resend>, to_email: &str, token: &str) {
    let Some(resend) = resend else {
        // No RESEND_API_KEY configured (dev default): log instead of sending.
        println!("\n==> LOGIN CODE for {}: {}\n", to_email, token);
        return;
    };

    // Counts actual Resend API calls only (feeds the Resend quota alert), not
    // the dev-mode logging fallback above which never touches Resend at all.
    axum_prometheus::metrics::counter!("login_emails_sent_total").increment(1);

    let from_addr = std::env::var("EMAIL_FROM").unwrap_or_else(|_| "login@brdg.me".to_string());
    let email = resend_rs::types::CreateEmailBaseOptions::new(
        from_addr,
        [to_email.to_string()],
        "Your brdgme login code",
    )
    .with_text(&format!(
        "Your login code is: {}\n\nThis code expires in 1 hour.",
        token
    ));

    if let Err(e) = resend.emails.send(email).await {
        tracing::error!("Failed to send login email to {}: {}", to_email, e);
    }
}
```

Replace with:

```rust
/// Builds the login-confirmation email's plain-text and HTML bodies. Pure
/// (no I/O), so it is unit-testable without a Resend account.
/// `code_valid` in `login()` allows a 1-hour validity window - this copy
/// must stay in sync with that if the window ever changes.
#[cfg(feature = "ssr")]
fn login_email_bodies(token: &str) -> (String, String) {
    let text = format!(
        "Your brdg.me confirmation is {token}\n\n\
         This confirmation will expire in 1 hour if not used."
    );
    let html = format!(
        r#"<link
    href="https://fonts.googleapis.com/css?family=Source+Code+Pro:400,700"
    rel="stylesheet"
>
<pre
    style="
        background-color: white;
        color: black;
        font-family: 'Source Code Pro', 'Lucida Console', monospace;
    "
>Your brdg.me confirmation is <b>{token}</b>

This confirmation will expire in 1 hour if not used.</pre>"#
    );
    (text, html)
}

#[cfg(feature = "ssr")]
async fn send_login_email(resend: Option<&resend_rs::Resend>, to_email: &str, token: &str) {
    let Some(resend) = resend else {
        // No RESEND_API_KEY configured (dev default): log instead of sending.
        println!("\n==> LOGIN CODE for {}: {}\n", to_email, token);
        return;
    };

    // Counts actual Resend API calls only (feeds the Resend quota alert), not
    // the dev-mode logging fallback above which never touches Resend at all.
    axum_prometheus::metrics::counter!("login_emails_sent_total").increment(1);

    let from_addr = std::env::var("EMAIL_FROM").unwrap_or_else(|_| "login@brdg.me".to_string());
    let (text_body, html_body) = login_email_bodies(token);
    let email = resend_rs::types::CreateEmailBaseOptions::new(
        from_addr,
        [to_email.to_string()],
        "brdg.me login confirmation",
    )
    .with_text(&text_body)
    .with_html(&html_body);

    if let Err(e) = resend.emails.send(email).await {
        tracing::error!("Failed to send login email to {}: {}", to_email, e);
    }
}
```

Note: `from_addr` stays `login@brdg.me` (unchanged) - the polish entry's Note says this is fine; `play@brdg.me` is only required for future game-play emails, out of scope here.

- [ ] **Step 4: Run the test again to confirm it passes**

Run: `cd rust && cargo test -p web --features ssr --lib login_email_bodies`
Expected:
```
test auth::server::tests::login_email_bodies_use_brdg_me_branding_and_token ... ok

test result: ok. 1 passed; 0 failed; ...
```

- [ ] **Step 5: Run the full existing auth test suite (needs Postgres up)**

Run: `cd rust && cargo test -p web --features ssr --lib auth::server::tests`
Expected: all tests pass, including the pre-existing `send_login_email_logs_when_resend_unset` (it already exercises the dev log-fallback path, unaffected by this change). If it fails with connection errors, start the dev Postgres first (`docs/DEV.md` - devenv's `DATABASE_URL` expects an externally-running DB).

- [ ] **Step 6: Lint and format**

Run: `cd rust && cargo fmt --all -- --check && cargo clippy -p web --all-targets --features ssr -- -D warnings`
Expected: both exit 0 with no output (no diffs, no warnings).

- [ ] **Step 7: Commit**

```bash
git add rust/web/src/auth/server.rs
git commit -m "$(cat <<'EOF'
web #33: fix login email branding and monospace styling

Subject and body previously said "brdgme"; body was plain text with no
styling. Now sends both a text and an HTML body matching the legacy
brdg.me monospace template (Source Code Pro, black-on-white <pre> block),
with the "brdg.me" branding fixed throughout. Expiry copy says 1 hour to
match the actual login_confirmations validity window (login()'s
code_valid check), not the legacy doc's literal "30 minutes".

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: Add a loading state to the login email step

**Files:**
- Modify: `rust/web/src/app.rs` (`LoginPage`)

**Interfaces:**
- Consumes: `login_action: Action<String, Result<LoginResponse, ServerFnError>>` (already exists in `LoginPage`) - uses its `.pending()` signal (`Action::pending(&self) -> Signal<bool>`, part of Leptos's `Action` API, no new type).

- [ ] **Step 1: Add the disabled state and spinner to the email-step form**

In `rust/web/src/app.rs`, find the email-step `<form>` inside `LoginPage`:

```rust
                    <form on:submit=on_email_submit>
                        <div>
                            <input
                                type="email"
                                node_ref=email_input
                                placeholder="Email address"
                                required
                            />
                            <input type="submit" value="Get code"/>
                        </div>
                        <div class="hasCode">
                            <a on:click=show_code_link style="cursor:pointer">"I already have a login code"</a>
                        </div>
                    </form>
```

Replace with:

```rust
                    <form on:submit=on_email_submit>
                        <div>
                            <input
                                type="email"
                                node_ref=email_input
                                placeholder="Email address"
                                required
                                disabled=move || login_action.pending().get()
                            />
                            <input
                                type="submit"
                                value="Get code"
                                disabled=move || login_action.pending().get()
                            />
                        </div>
                        <Show when=move || login_action.pending().get()>
                            <div class="spinner">
                                <div class="bounce1"></div>
                                <div class="bounce2"></div>
                                <div class="bounce3"></div>
                            </div>
                        </Show>
                        <div class="hasCode">
                            <a on:click=show_code_link style="cursor:pointer">"I already have a login code"</a>
                        </div>
                    </form>
```

The `spinner`/`bounce1`/`bounce2`/`bounce3` classes and their bounce animation already exist in `rust/web/style/main.scss` (lines 30-75) - this is the same three-dot spinner the legacy React app used on this exact form (`web/src/components/spinner.tsx`, `web/src/components/login.tsx`), so no CSS changes are needed.

- [ ] **Step 2: Verify it compiles**

Run: `cd rust && cargo check -p web --features hydrate --no-default-features --target wasm32-unknown-unknown`
Expected: `Finished` with no errors.

Run: `cd rust && cargo check -p web --features ssr --no-default-features`
Expected: `Finished` with no errors.

- [ ] **Step 3: Lint and format**

Run: `cd rust && cargo fmt --all -- --check && cargo clippy -p web --all-targets --features ssr -- -D warnings`
Expected: both exit 0 with no output.

- [ ] **Step 4: Manual verification**

`tilt up`, open `http://localhost:3000/login`. Type an email address, click "Get code". Expected: the email field and button become disabled and the three-dot bounce spinner appears immediately (no dead pause), then the code-entry form replaces it once the server responds (a second or so later, per the polish entry's Observed timing). Check the Tilt `web` resource log for the printed `LOGIN CODE` line to get the code for testing the rest of the flow if needed.

- [ ] **Step 5: Commit**

```bash
git add rust/web/src/app.rs
git commit -m "$(cat <<'EOF'
web #33: add loading state to login email step

Email field and submit button disable and a spinner (existing legacy
.spinner/.bounce1-3 CSS, previously unused in the Leptos port) appears
immediately on submit, closing the ~1s dead-air gap before the code form
renders.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 5: Wire up the sidebar Menu button on narrow viewports

**Files:**
- Modify: `rust/web/src/components/layout.rs` (`MainLayout`, `SidebarMenu`)

**Interfaces:**
- Produces: `SidebarMenu` now takes two required props: `open: Signal<bool>` (`#[prop(into)]`), `set_open: WriteSignal<bool>`. Task 7 below builds on this signature - do not change it there.
- Consumes: no new external state; `leptos_router::hooks::use_location()` (already available, used elsewhere in the router).

The mobile-collapse CSS already exists in `rust/web/style/main.scss` (`@media only screen and (max-width: 80em)` block: `.layout .menu`, `.layout .menu.open`, `.menu-close-underlay`) - this task only adds the Rust-side state and event wiring, no CSS changes. Note the underlay's CSS sets `display: block` unconditionally inside that media query (not gated on any class), so it must only ever be *mounted* while open, not just hidden with the `hidden` attribute (an attribute-selector UA rule loses to that unconditional `display: block` author rule) - the `<Show>` below handles that.

- [ ] **Step 1: Give `MainLayout` menu-open state and wire the Menu button + underlay**

In `rust/web/src/components/layout.rs`, find:

```rust
#[component]
pub fn MainLayout(
    #[prop(into, default = Signal::from(false))] is_my_turn: Signal<bool>,
    #[prop(into, default = Signal::from(false))] has_sub_menu: Signal<bool>,
    #[prop(into, default = Signal::from(false))] has_next_game: Signal<bool>,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="layout">
            <div class="layout-header" class:my-turn=move || is_my_turn.get()>
                <input type="button" value="Menu"/>
                <span class="header-title">"brdg.me"</span>
                // Always render same element type; toggle visibility to avoid structural mismatch
                <input type="button" value="Sub menu" hidden=move || !has_sub_menu.get()/>
                <input type="button" value="Next game" hidden=move || !has_next_game.get()/>
            </div>
            <div class="layout-body">
                <SidebarMenu />
                <div class="content">
                    {children()}
                </div>
            </div>
        </div>
    }
}
```

Replace with:

```rust
#[component]
pub fn MainLayout(
    #[prop(into, default = Signal::from(false))] is_my_turn: Signal<bool>,
    #[prop(into, default = Signal::from(false))] has_sub_menu: Signal<bool>,
    #[prop(into, default = Signal::from(false))] has_next_game: Signal<bool>,
    children: Children,
) -> impl IntoView {
    let (menu_open, set_menu_open) = signal(false);

    view! {
        <div class="layout">
            <div class="layout-header" class:my-turn=move || is_my_turn.get()>
                <input
                    type="button"
                    value="Menu"
                    on:click=move |_| set_menu_open.update(|v| *v = !*v)
                />
                <span class="header-title">"brdg.me"</span>
                // Always render same element type; toggle visibility to avoid structural mismatch
                <input type="button" value="Sub menu" hidden=move || !has_sub_menu.get()/>
                <input type="button" value="Next game" hidden=move || !has_next_game.get()/>
            </div>
            <div class="layout-body">
                <SidebarMenu open=menu_open set_open=set_menu_open />
                // Mobile-only overlay (see the `@media (max-width: 80em)` block in
                // main.scss); only mounted while the menu is open so it never
                // covers the page underneath it when closed.
                <Show when=move || menu_open.get()>
                    <div
                        class="menu-close-underlay"
                        on:click=move |_| set_menu_open.set(false)
                    ></div>
                </Show>
                <div class="content">
                    {children()}
                </div>
            </div>
        </div>
    }
}
```

- [ ] **Step 2: Accept the props in `SidebarMenu`, toggle the `open` class, close on navigation**

Find the start of `SidebarMenu`:

```rust
#[component]
pub fn SidebarMenu() -> impl IntoView {
    let logout_action = ServerAction::<crate::auth::Logout>::new();
    let navigate = use_navigate();
    Effect::new(move |_| {
        if logout_action.value().get().is_some_and(|r| r.is_ok()) {
            navigate("/login", NavigateOptions::default());
        }
    });
    let on_logout = move |_| {
        logout_action.dispatch(crate::auth::Logout {});
    };
```

Replace the signature only (leave the logout logic body as-is for this task - `logout_action`'s source changes in Task 7):

```rust
#[component]
pub fn SidebarMenu(#[prop(into)] open: Signal<bool>, set_open: WriteSignal<bool>) -> impl IntoView {
    let logout_action = ServerAction::<crate::auth::Logout>::new();
    let navigate = use_navigate();
    Effect::new(move |_| {
        if logout_action.value().get().is_some_and(|r| r.is_ok()) {
            navigate("/login", NavigateOptions::default());
        }
    });
    let on_logout = move |_| {
        logout_action.dispatch(crate::auth::Logout {});
    };
```

Then find the `logged_in` line right before the `view!` block:

```rust
    let logged_in = move || matches!(current_user.get(), Some(Ok(Some(_))));

    view! {
        <div class="menu">
            <h1><A href="/">"brdg.me"</A></h1>
```

Replace with (adds the close-on-navigate effect and the `open` class):

```rust
    let logged_in = move || matches!(current_user.get(), Some(Ok(Some(_))));

    // Close the mobile menu overlay on every route change - covers
    // "navigating closes it" for every link without per-link handlers.
    let location = leptos_router::hooks::use_location();
    Effect::new(move |_| {
        location.pathname.get();
        set_open.set(false);
    });

    view! {
        <div class="menu" class:open=move || open.get()>
            <h1><A href="/">"brdg.me"</A></h1>
```

- [ ] **Step 3: Verify it compiles**

Run: `cd rust && cargo check -p web --features hydrate --no-default-features --target wasm32-unknown-unknown && cargo check -p web --features ssr --no-default-features`
Expected: both `Finished` with no errors.

- [ ] **Step 4: Lint and format**

Run: `cd rust && cargo fmt --all -- --check && cargo clippy -p web --all-targets --features ssr -- -D warnings`
Expected: both exit 0 with no output.

- [ ] **Step 5: Manual verification**

`tilt up`, open `http://localhost:3000` in a browser, open devtools and switch to responsive/device mode at a width under 80em (roughly under ~1280px depending on root font size - anything phone/tablet-sized works). Expected:
1. The sidebar is hidden and the header (with "Menu" button) is visible.
2. Click "Menu": the sidebar slides in as an overlay with a dimmed underlay behind it.
3. Click the dimmed underlay (not the sidebar itself): the sidebar closes.
4. Click "Menu" again, then click a sidebar link (e.g. "New game"): the sidebar closes and the page navigates.
5. Widen the viewport back over 80em: the sidebar returns to its normal always-visible layout.

- [ ] **Step 6: Commit**

```bash
git add rust/web/src/components/layout.rs
git commit -m "$(cat <<'EOF'
web #33: wire up sidebar Menu button on narrow viewports

The button had no on:click at all; the mobile collapse/overlay CSS
already existed (carried over from the legacy app) but nothing toggled
the `.menu.open` class or rendered the close underlay. Closes on
underlay click or on any client-side navigation (via use_location).

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 6: Fix white flash on game command submit

**Files:**
- Modify: `rust/web/src/app.rs` (`GamePage`)

**Interfaces:** none new - this swaps one Leptos component (`Suspense` -> `Transition`) with an identical `fallback` prop signature; nothing else in the file references the swapped tag.

**Verification that no other Suspense boundary exists:** `grep -rn "Suspense\|Transition" rust/web/src` returns exactly one hit before this change - `GamePage`'s wrapper at `app.rs`. `GameLogs`, `RecentGameLogs`, and `GameMeta` (`rust/web/src/components/game.rs`) read their `LocalResource`s directly with `.get().map(...)`, with no `Suspense`/`Transition` boundary at all, so the polish entry's "check the sibling GameLogs/GameMeta suspense boundaries for the same pattern" turns up nothing to fix there - this is the only site.

- [ ] **Step 1: Swap `Suspense` for `Transition`**

In `rust/web/src/app.rs`, inside `GamePage`, find:

```rust
    // MainLayout is outside Suspense so it is always in the initial SSR HTML
    // with no streaming placeholder risk. Suspense defers hydration of game
    // content until game_data deserializes, matching SSR and client structure.
    view! {
        <MainLayout
            is_my_turn=Signal::from(is_my_turn)
            has_sub_menu=Signal::from(true)
            has_next_game=Signal::from(is_my_turn)
        >
            <Suspense fallback=|| view! { <div></div> }>
```

Replace with:

```rust
    // MainLayout is outside Transition so it is always in the initial SSR
    // HTML with no streaming placeholder risk. Transition (not Suspense)
    // wraps the game content: Suspense's fallback replaces its children on
    // every refetch, blanking the board to white on each WS-triggered
    // update; Transition keeps the last-rendered children visible during a
    // refetch and only shows `fallback` before the first load.
    view! {
        <MainLayout
            is_my_turn=Signal::from(is_my_turn)
            has_sub_menu=Signal::from(true)
            has_next_game=Signal::from(is_my_turn)
        >
            <Transition fallback=|| view! { <div></div> }>
```

Then find the matching closing tag near the end of the same `view!` block:

```rust
                }}
            </Suspense>
        </MainLayout>
    }
}
```

Replace with:

```rust
                }}
            </Transition>
        </MainLayout>
    }
}
```

`Transition` is re-exported by `leptos::prelude::*` (already glob-imported at the top of `app.rs`), so no new `use` is needed.

- [ ] **Step 2: Verify it compiles**

Run: `cd rust && cargo check -p web --features hydrate --no-default-features --target wasm32-unknown-unknown && cargo check -p web --features ssr --no-default-features`
Expected: both `Finished` with no errors.

- [ ] **Step 3: Run the SSR integration tests**

Run: `cd rust && cargo test -p web --features ssr --test ssr_pages`
Expected: `test result: ok. 10 passed; 0 failed`. If `game_page_logged_in_player_renders_game` (or any other single test) fails when run as part of the full `cargo test -p web --features ssr` (all three test binaries together) but passes when re-run alone (`cargo test -p web --features ssr --test ssr_pages`) or with `--test-threads=1`, that is pre-existing sandbox/CI resource-contention flakiness unrelated to this change (observed once during this plan's own verification, did not reproduce in the next two full runs) - retry before treating it as a regression signal.

- [ ] **Step 4: Lint and format**

Run: `cd rust && cargo fmt --all -- --check && cargo clippy -p web --all-targets --features ssr -- -D warnings`
Expected: both exit 0 with no output.

- [ ] **Step 5: Manual verification**

`tilt up`, log in, open or start a game where it's your turn. Type a valid command and submit. Expected: the board and log stay visible the whole time - no blank/white frame - then update in place once the new state arrives. Compare against current `master` (before this fix) if unsure what the bug looks like: on `master`, submitting blanks the entire game panel to an empty `<div>` for a moment.

- [ ] **Step 6: Commit**

```bash
git add rust/web/src/app.rs
git commit -m "$(cat <<'EOF'
web #33: fix white flash on game command submit

Suspense -> Transition for the game-data resource boundary in GamePage.
Suspense's empty-<div> fallback replaces its children on every refetch,
which is what caused the blank frame on every command/WS update;
Transition keeps the last-rendered board visible during a refetch and
only shows the fallback before the very first load. Confirmed via grep
that GameLogs/RecentGameLogs/GameMeta have no Suspense boundary of their
own to fix - this was the only site.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 7: Fix sidebar flash on client-side navigation

**Files:**
- Modify: `rust/web/src/app.rs` (`App`)
- Modify: `rust/web/src/components/layout.rs` (`SidebarMenu` - builds on Task 5's signature)

**Interfaces:**
- Produces (via `provide_context` in `App`, consumed by `SidebarMenu`):
  - `ServerAction<crate::auth::Logout>`
  - `LocalResource<Result<Vec<crate::game::server_fns::GameSummary>, ServerFnError>>` (the active-games list)
  - `LocalResource<Result<Option<crate::auth::AuthUser>, ServerFnError>>` (the current-user check)
- Task 8 below consumes the `active_games` `LocalResource` from this same context - do not change its provided type there.

**Root cause (confirmed by reading the code, not guessed):** every page component (`HomePage`, `GamesPage`, `DashboardPage`, `GamePage`) wraps its own `<MainLayout>` inside its route `view=`. `leptos_router` unmounts the outgoing route's view and mounts the incoming one on every navigation, so `MainLayout` - and the `SidebarMenu` nested inside it, and the two `LocalResource`s `SidebarMenu` currently creates with its own `LocalResource::new(...)` - are destroyed and recreated from scratch on every link click. A freshly-created `LocalResource` starts at `None`, which is exactly the "Loading games..." / logged-out flash. This is a client-side-only, resource-lifecycle bug - not the WS-signal-merge concern backlog #27 WP3 owns (see Global Constraints).

**The minimal fix:** the polish entry's own suggested lever is "keep stale data while refetching" - achieved here not by adding a cache, but by creating these two `LocalResource`s (and the `ServerAction<Logout>` that keys one of them) exactly once, in `App`, which sits above `<Router>` and therefore never unmounts on navigation. `SidebarMenu` reads them via `expect_context` instead of creating its own. The resources themselves never remount, so a freshly-mounted `SidebarMenu` immediately reads whatever value is already sitting in them instead of starting at `None`.

- [ ] **Step 1: Hoist the resources into `App`, above `<Router>`**

In `rust/web/src/app.rs`, find:

```rust
    provide_context(RwSignal::<Option<(Uuid, u64)>>::new(None));
    crate::websocket_client::use_websocket();

    view! {
        <Stylesheet id="leptos" href="/pkg/web.css"/>
        <Title text="brdg.me"/>

        <Router>
```

Replace with:

```rust
    provide_context(RwSignal::<Option<(Uuid, u64)>>::new(None));
    crate::websocket_client::use_websocket();

    // Hoisted above <Router> so these survive client-side navigation instead
    // of being torn down and recreated by every page's own <MainLayout>
    // (each page wraps its own <MainLayout>, so the sidebar remounts on
    // every route change). Fixes the sidebar's Logout->Login and "Loading
    // games..." flash: the resources themselves never remount, only the
    // components reading them do, so a fresh mount just reads the value
    // already sitting in these signals instead of starting from None.
    let logout_action = ServerAction::<crate::auth::Logout>::new();
    provide_context(logout_action);

    let active_games: LocalResource<
        Result<Vec<crate::game::server_fns::GameSummary>, ServerFnError>,
    > = LocalResource::new(move || async move {
        let _ = last_update.get();
        crate::game::server_fns::get_active_games().await
    });
    provide_context(active_games);

    // None until the fetch resolves; treat that as logged-out so anonymous
    // visitors never see "Logout". Re-fetches after a logout dispatch.
    let current_user: LocalResource<Result<Option<crate::auth::AuthUser>, ServerFnError>> =
        LocalResource::new(move || async move {
            let _ = logout_action.version().get();
            crate::auth::get_current_user().await
        });
    provide_context(current_user);

    view! {
        <Stylesheet id="leptos" href="/pkg/web.css"/>
        <Title text="brdg.me"/>

        <Router>
```

(This reuses the `last_update` `ReadSignal<u64>` already bound two lines above in `App` - the same signal `WebSocketTrigger.last_update` wraps - so `active_games` re-fetches on WS bumps exactly as it did inside `SidebarMenu` before. This task does not touch `WebSocketTrigger` itself.)

- [ ] **Step 2: Point `SidebarMenu` at the hoisted context instead of creating its own resources**

In `rust/web/src/components/layout.rs`, find the top-of-file import:

```rust
use crate::components::game::PlayerName;
use crate::game::server_fns::{GameSummary, get_active_games};
```

Replace with (drops the now-unused `get_active_games` import):

```rust
use crate::components::game::PlayerName;
use crate::game::server_fns::GameSummary;
```

Then find (this is `SidebarMenu`'s body as left by Task 5 - the `logout_action` local creation is still there):

```rust
pub fn SidebarMenu(#[prop(into)] open: Signal<bool>, set_open: WriteSignal<bool>) -> impl IntoView {
    let logout_action = ServerAction::<crate::auth::Logout>::new();
    let navigate = use_navigate();
    Effect::new(move |_| {
        if logout_action.value().get().is_some_and(|r| r.is_ok()) {
            navigate("/login", NavigateOptions::default());
        }
    });
    let on_logout = move |_| {
        logout_action.dispatch(crate::auth::Logout {});
    };

    let trigger = expect_context::<crate::websocket_client::WebSocketTrigger>();
    let active_games: LocalResource<Result<Vec<GameSummary>, ServerFnError>> =
        LocalResource::new(move || async move {
            let _ = trigger.last_update.get();
            get_active_games().await
        });

    // None until the fetch resolves; treat that as logged-out so anonymous
    // visitors never see "Logout". Re-fetches after a logout dispatch.
    let current_user: LocalResource<Result<Option<crate::auth::AuthUser>, ServerFnError>> =
        LocalResource::new(move || async move {
            let _ = logout_action.version().get();
            crate::auth::get_current_user().await
        });
    let logged_in = move || matches!(current_user.get(), Some(Ok(Some(_))));
```

Replace with:

```rust
pub fn SidebarMenu(#[prop(into)] open: Signal<bool>, set_open: WriteSignal<bool>) -> impl IntoView {
    let logout_action = expect_context::<ServerAction<crate::auth::Logout>>();
    let navigate = use_navigate();
    Effect::new(move |_| {
        if logout_action.value().get().is_some_and(|r| r.is_ok()) {
            navigate("/login", NavigateOptions::default());
        }
    });
    let on_logout = move |_| {
        logout_action.dispatch(crate::auth::Logout {});
    };

    // Provided once in `App` (outside the router) so these resources survive
    // client-side navigation instead of being torn down and recreated by
    // every page's own `<MainLayout>` - see the comment at their
    // `provide_context` call sites in `app.rs`.
    let active_games = expect_context::<LocalResource<Result<Vec<GameSummary>, ServerFnError>>>();
    let current_user =
        expect_context::<LocalResource<Result<Option<crate::auth::AuthUser>, ServerFnError>>>();
    let logged_in = move || matches!(current_user.get(), Some(Ok(Some(_))));
```

- [ ] **Step 3: Verify it compiles**

Run: `cd rust && cargo check -p web --features hydrate --no-default-features --target wasm32-unknown-unknown && cargo check -p web --features ssr --no-default-features`
Expected: both `Finished` with no errors.

- [ ] **Step 4: Run the full test suite**

Run: `cd rust && cargo test -p web --features ssr`
Expected: all suites `ok`, `0 failed` (58 lib tests + 5 `nats_bot_eventing` + 10 `ssr_pages` + 1 `websocket_hygiene`, 2 lib tests pre-existing `#[ignore]`d as flaky per `docs/superpowers/plans/2026-07-07-27-web-simplification.md`).

- [ ] **Step 5: Lint and format**

Run: `cd rust && cargo fmt --all -- --check && cargo clippy -p web --all-targets --features ssr -- -D warnings`
Expected: both exit 0 with no output.

- [ ] **Step 6: Manual verification**

`tilt up`, log in, then click rapidly between "New game" (`/games`), the brdg.me home link (`/`), and any active game link in the sidebar several times. Expected: the "Logout" link stays "Logout" the whole time (no flash to "Login"), and the active-games list stays populated the whole time (no flash to "Loading games..."). This is a regression check for the exact bug the polish entry describes - it is easy to miss if you click slowly, so click several times in quick succession.

- [ ] **Step 7: Commit**

```bash
git add rust/web/src/app.rs rust/web/src/components/layout.rs
git commit -m "$(cat <<'EOF'
web #33: fix sidebar flash on client-side navigation

Every page wraps its own <MainLayout>, so the sidebar (and the two
LocalResources it read) was destroyed and recreated on every route
change, flashing "Loading games..." and Logout->Login. Hoisted the
active-games resource, current-user resource, and the ServerAction<Logout>
that keys the latter into App (above <Router>, so they never remount) and
have SidebarMenu read them via context instead of creating its own.

Does not touch WebSocketTrigger or the WS-signal-merge backlog #27 WP3
owns - same last_update signal, same re-fetch triggers as before, just
relocated to where they survive navigation.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 8: Show turn count in the page title

**Files:**
- Modify: `rust/web/src/app.rs` (`App`, plus a new pure helper `count_my_turn` and its tests at the bottom of the file)

**Interfaces:**
- Consumes: the `active_games: LocalResource<Result<Vec<GameSummary>, ServerFnError>>` context value Task 7 provided in `App` - same data the sidebar renders, no new query, per the polish entry's note.
- Produces: `fn count_my_turn(games: &[crate::game::server_fns::GameSummary]) -> usize` (private, pure, unit-tested).

- [ ] **Step 1: Write the failing tests for the pure counting helper**

At the very end of `rust/web/src/app.rs` (after the closing `}` of `GamePage`), add:

```rust

/// Counts active games where it's the user's turn - the title's "(N)" badge.
/// Pure (no resource/DOM access) so it's unit-testable on its own.
fn count_my_turn(games: &[crate::game::server_fns::GameSummary]) -> usize {
    games.iter().filter(|g| g.is_turn).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::server_fns::{GameSummary, OpponentSummary};

    fn game_summary(is_turn: bool) -> GameSummary {
        GameSummary {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            type_name: "Test Game".to_string(),
            opponents: vec![OpponentSummary {
                name: "Bob".to_string(),
                color: "#000".to_string(),
            }],
            is_turn,
        }
    }

    #[test]
    fn count_my_turn_counts_only_is_turn_games() {
        let games = vec![game_summary(true), game_summary(false), game_summary(true)];
        assert_eq!(count_my_turn(&games), 2);
    }

    #[test]
    fn count_my_turn_zero_for_empty() {
        assert_eq!(count_my_turn(&[]), 0);
    }
}
```

(`Uuid` is already imported at the top of `app.rs` via `use uuid::Uuid;`. `GameSummary`/`OpponentSummary` have no `#[cfg(feature = "ssr")]` gate - only the server functions around them do - so this test module compiles under the `ssr` feature test build same as the rest of the file's tests would.)

- [ ] **Step 2: Run it to confirm the tests pass on the trivial pure function**

Run: `cd rust && cargo test -p web --features ssr --lib count_my_turn`
Expected:
```
test app::tests::count_my_turn_counts_only_is_turn_games ... ok
test app::tests::count_my_turn_zero_for_empty ... ok

test result: ok. 2 passed; 0 failed; ...
```
(This step is technically "implement then test" rather than red-green, since the function is a one-line filter+count with no meaningful failing state to observe first - the two assertions themselves are the specification. If you want a true red-green cycle, write the `#[test]` functions first, run to confirm `cannot find function 'count_my_turn'`, then add the function from Step 1's snippet above.)

- [ ] **Step 3: Wire the Memo and the reactive `<Title>`**

In `rust/web/src/app.rs`, find (this is `App`'s body as left by Task 7):

```rust
    provide_context(current_user);

    view! {
        <Stylesheet id="leptos" href="/pkg/web.css"/>
        <Title text="brdg.me"/>

        <Router>
```

Replace with:

```rust
    provide_context(current_user);

    // Derived from the same active-games data the sidebar renders, not a
    // new query - counts games where it's this user's turn.
    let turn_count = Memo::new(move |_| {
        active_games
            .get()
            .and_then(|r| r.ok())
            .map(|games| count_my_turn(&games))
            .unwrap_or(0)
    });
    let title_text = move || {
        let n = turn_count.get();
        if n > 0 {
            format!("brdg.me ({n})")
        } else {
            "brdg.me".to_string()
        }
    };

    view! {
        <Stylesheet id="leptos" href="/pkg/web.css"/>
        <Title text=title_text/>

        <Router>
```

- [ ] **Step 4: Verify it compiles**

Run: `cd rust && cargo check -p web --features hydrate --no-default-features --target wasm32-unknown-unknown && cargo check -p web --features ssr --no-default-features`
Expected: both `Finished` with no errors.

- [ ] **Step 5: Run the full test suite**

Run: `cd rust && cargo test -p web --features ssr --lib`
Expected: `test result: ok. 60 passed; 0 failed; 2 ignored` (58 pre-existing + the 2 new `count_my_turn` tests; the 2 ignored are the pre-existing flaky-NATS ones, unrelated).

- [ ] **Step 6: Lint and format**

Run: `cd rust && cargo fmt --all -- --check && cargo clippy -p web --all-targets --features ssr -- -D warnings`
Expected: both exit 0 with no output.

- [ ] **Step 7: Manual verification (cannot be unit tested - needs a live browser tab and turn-state changes)**

`tilt up`, log in, go to `/games`, create a new game with a bot opponent (select "Bot" for the opponent, submit). Since it becomes your turn immediately, expected: the browser tab title changes from "brdg.me" to "brdg.me (1)" as soon as the game is created (no page reload needed - it is reactive on the same WS-bumped resource the sidebar uses). Submit a command that ends your turn (or wait for the bot to act, if the game auto-advances) - expected: the title returns to plain "brdg.me" once `is_turn` for that game goes false. If you have two active own-turn games, expect "brdg.me (2)".

- [ ] **Step 8: Commit**

```bash
git add rust/web/src/app.rs
git commit -m "$(cat <<'EOF'
web #33: show turn count in page title

<Title> is now reactive: "brdg.me (N)" where N is the count of active
games where it's the user's turn, plain "brdg.me" when N is 0. Derives
from the same active-games LocalResource the sidebar already reads
(hoisted to App context in the prior commit) rather than a new query.
count_my_turn extracted as a pure, unit-tested helper.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 9: Autofocus login and game command inputs

**Files:**
- Modify: `rust/web/src/app.rs` (`LoginPage` - builds on Task 4's already-modified form)
- Modify: `rust/web/src/components/game.rs` (`GameCommandInput`)

**Interfaces:** none new - adds two `Effect`s in `LoginPage` and one `window_event_listener` + `on_cleanup` pair in `GameCommandInput`.

**Already implemented - verify before touching, do not duplicate:** `GameCommandInput` (`rust/web/src/components/game.rs`) already has a mount `Effect` that focuses the command input, and a second `Effect` that refocuses it after a successful `submit_action` - i.e. two of this entry's four required behaviors ("opening a game" and "after a play is submitted") already exist in the code as of this plan being written:

```rust
    // Focus input on mount (works for both hard refresh and client-side navigation).
    Effect::new(move |_| {
        if let Some(el) = input_ref.get() {
            let _ = el.focus();
        }
    });
```
and, further down, inside the submit-success effect:
```rust
            if let Some(el) = input_ref.get() {
                let _ = el.focus();
            }
```

**This plan could not verify in-browser whether these already work** - the polish doc's Observed section says the command field "requires a click before typing" even on open, which contradicts what the code appears to do. Possible explanations not distinguishable by reading code alone: the doc predates a later unrelated fix, the effect fires but loses the race against something else (e.g. the white-flash Suspense remount Task 6 just fixed - worth re-checking after Task 6 lands), or it already works and the doc entry is stale. **Step 1 below is a mandatory manual check before writing any new code for these two sub-behaviors** - only patch them if the check shows they are actually broken; this task's new code (Steps 2-4) only covers the two behaviors that are genuinely absent (login autofocus, type-anywhere-focus).

- [ ] **Step 1: Manually verify the two "already implemented" behaviors first**

`tilt up`, log in, open a game where it is your turn. Without clicking anywhere, type a character. Expected if already working: it appears in the command input. Then submit a command. Expected if already working: after the board updates, type another character without clicking - it should appear in the command input again. If **both** already work, skip re-implementing them (nothing to change in `game.rs` for those two sub-behaviors - only add the type-anywhere listener in Step 4). If **either** is broken, note the exact repro (which one, hard refresh vs client-nav, etc.) and treat it as a bug to fix inside this task before proceeding, using the same `input_ref.get()` + `.focus()` pattern already present.

- [ ] **Step 2: Autofocus the login email field on load, and the code field once it renders**

In `rust/web/src/app.rs`, find (this is `LoginPage` as left by Task 4 - `show_code_link` then the `view!` block):

```rust
    let show_code_link = move |_| {
        set_show_code_input.set(true);
    };

    view! {
        <div class="login">
```

Replace with:

```rust
    let show_code_link = move |_| {
        set_show_code_input.set(true);
    };

    // Autofocus: email field on load, code field once the code step renders
    // (both re-fire once their `NodeRef` resolves, matching the pattern
    // already used by `GameCommandInput`'s mount effect).
    Effect::new(move |_| {
        if let Some(el) = email_input.get() {
            let _ = el.focus();
        }
    });
    Effect::new(move |_| {
        if show_code_input.get()
            && let Some(el) = code_input.get()
        {
            let _ = el.focus();
        }
    });

    view! {
        <div class="login">
```

- [ ] **Step 3: Verify it compiles**

Run: `cd rust && cargo check -p web --features hydrate --no-default-features --target wasm32-unknown-unknown`
Expected: `Finished` with no errors.

- [ ] **Step 4: Add the type-anywhere-focuses-command-field listener**

In `rust/web/src/components/game.rs`, inside `GameCommandInput`, find:

```rust
    // Focus input on mount (works for both hard refresh and client-side navigation).
    Effect::new(move |_| {
        if let Some(el) = input_ref.get() {
            let _ = el.focus();
        }
    });

    // Clear command, refocus input, and trigger re-fetch on successful submit.
```

Replace with:

```rust
    // Focus input on mount (works for both hard refresh and client-side navigation).
    Effect::new(move |_| {
        if let Some(el) = input_ref.get() {
            let _ = el.focus();
        }
    });

    // Type-anywhere-focuses-command-field: only single, unmodified,
    // printable-character keystrokes are diverted, and only when nothing is
    // already focused - so Tab-focused links/buttons keep their normal
    // keyboard behaviour, especially Enter navigating a focused link.
    let keydown_listener = window_event_listener(leptos::ev::keydown, move |ev| {
        if ev.ctrl_key() || ev.meta_key() || ev.alt_key() {
            return;
        }
        if ev.key().chars().count() != 1 {
            return;
        }
        let nothing_focused = document()
            .active_element()
            .map(|el| el.tag_name() == "BODY")
            .unwrap_or(true);
        if !nothing_focused {
            return;
        }
        if let Some(el) = input_ref.get_untracked() {
            let _ = el.focus();
        }
    });
    on_cleanup(move || keydown_listener.remove());

    // Clear command, refocus input, and trigger re-fetch on successful submit.
```

`window_event_listener`, `document`, and `on_cleanup` are all re-exported by `leptos::prelude::*` (already glob-imported at the top of `game.rs`) - `window_event_listener` is a no-op on the server (it checks `is_server()` internally), so no `#[cfg(feature = "hydrate")]` guard is needed, matching the existing unguarded `Effect`s in this same function.

- [ ] **Step 5: Verify it compiles**

Run: `cd rust && cargo check -p web --features hydrate --no-default-features --target wasm32-unknown-unknown && cargo check -p web --features ssr --no-default-features`
Expected: both `Finished` with no errors.

- [ ] **Step 6: Run the full test suite**

Run: `cd rust && cargo test -p web --features ssr`
Expected: all suites `ok`, `0 failed`.

- [ ] **Step 7: Lint and format**

Run: `cd rust && cargo fmt --all -- --check && cargo clippy -p web --all-targets --features ssr -- -D warnings`
Expected: both exit 0 with no output.

- [ ] **Step 8: Manual verification**

`tilt up`:
1. Open `http://localhost:3000/login` fresh (hard load). Expected: the email field has focus immediately (blinking cursor, no click needed) - type and confirm characters land in it.
2. Submit the email. Expected: once the code form renders, the code field has focus immediately.
3. Log in, open a game where it's your turn (covered by Step 1 above too). Click somewhere neutral on the page (not a link/input) to defocus, then type a letter. Expected: the command input gains focus and the typed character appears in it.
4. Press `Tab` until a link (e.g. "Logout" in the sidebar, or "Undo"/"Concede" if visible) has visible focus, then press `Enter`. Expected: the link's normal action fires (navigates or triggers its click handler) - it must **not** be swallowed by the type-anywhere handler. This is the accessibility caveat from the polish entry and from Global Constraints - do not skip this check.

- [ ] **Step 9: Commit**

```bash
git add rust/web/src/app.rs rust/web/src/components/game.rs
git commit -m "$(cat <<'EOF'
web #33: autofocus login and game command inputs

Login page: email field focused on load, code field focused once the
code step renders. Game page: command field already refocused on mount
and after a successful play (pre-existing); added the missing
type-anywhere-focuses-command-field behavior via a window keydown
listener, gated to single unmodified printable characters and only when
nothing is already focused, so Tab+Enter on a focused link keeps working.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
EOF
)"
```
