# WP-36: crypto and deploy hardening

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to execute this plan task-by-task with review checkpoints.

**Goal:** Close four web-server hardening findings: session cookies get the `Secure` flag by default so prod stops sending the session token over plaintext HTTP (ws F52, major); web's `main` defensively installs a rustls `CryptoProvider` so a dependency bump can never trigger the dual-provider startup panic (ws F54, minor); WebSocket connections receive a proper close frame on graceful shutdown instead of being hard-dropped (ws F55, minor); and the AES-GCM master key gets `zeroize` hygiene (ws F17, nit — AAD half of the recommendation is DECLINED, see Task 4 rationale).

**Architecture — how the pieces fit today (verified against live source 2026-07-25):**

- **Session cookie:** `create_session_layer` (`rust/web/src/auth/session.rs:26-39`) builds the one and only `tower_sessions::SessionManagerLayer` for the whole app (`rust/web/src/router.rs:117`, applied at `router.rs:155`). Lines 32-34:
  ```rust
  let secure = std::env::var("SECURE_COOKIE")
      .map(|v| v == "true")
      .unwrap_or(false);
  ```
  i.e. **insecure unless `SECURE_COOKIE=true`**, and a repo-wide grep confirms `SECURE_COOKIE` is set NOWHERE — not in `k8s/` (base, dev, prod), not in `rust/web/.env.template`, not in the `Tiltfile`. Prod therefore runs `with_secure(false)`. `HttpOnly` is tower-sessions' default (on), `SameSite=Lax` and 30-day `OnInactivity` expiry are set explicitly (lines 37-38) and are fine. Cookie *removal* is also handled inside the same layer (tower-sessions emits the removal cookie with the layer's own attributes), so flipping the layer's flag covers set, refresh, and delete uniformly — there is no separate deletion code path to fix. The only other cookie in the codebase is the client-side JS `theme` cookie (`rust/web/src/app.rs:266-270`) — not a credential, not in scope. `/ws` (`router.rs:142`) sits under the same session layer; browsers apply the `Secure` attribute to cookie transmission on the upgrade request themselves, so **no `websocket.rs` change is needed for F52**.
- **k8s overlay structure (why default-secure-in-code, not a base env var):** `k8s/dev/kustomization.yaml` consumes `../base/web` directly with NO patches; `k8s/prod/kustomization.yaml` -> `prod/app/kustomization.yaml` -> `../../base/brdgme` -> `../web`, with `prod/app/web-patch.yaml` patching only Sentry DSNs. So an env var added to `k8s/base/web/deployment.yaml` lands in BOTH dev and prod — the triage note is confirmed: setting `SECURE_COOKIE=true` in base would force `Secure` onto in-cluster dev. In-cluster dev web (Tilt `WEB_IN_CLUSTER=1`) is reached at `http://web.brdgme.lvh.me:8080` — plain HTTP on a **non-localhost hostname**, where browsers reject `Secure` cookies outright (the localhost carve-out is by host string, and `lvh.me` names don't qualify). Dev genuinely needs an explicit opt-out. Decision: **default secure in code (`unwrap_or(true)` semantics), opt-out via `SECURE_COOKIE=false`**, wired into the dev overlay, the Tiltfile local web resource, and `.env.template`. Prod needs no manifest change at all — unset now means secure.
- **rustls provider:** `cargo tree -p web --features ssr` (run 2026-07-25) resolves `rustls v0.23.42` with features `aws-lc-rs,...,ring,...` — BOTH backends enabled in web's graph (reqwest pulls `aws-lc-rs`; sqlx/async-nats pull `ring`). This settles the finding's "UNVERIFIABLE" caveat: web IS in the dual-backend state where any crate reading the process-default provider panics at first use (`docs/CODING.md:408-431`, the operator CrashLooped on exactly this on 2026-07-08). Today's deps (sqlx, reqwest) select their backend explicitly, so nothing fires — the fix is the defensive install the project rule already mandates, copied from `rust/operator/src/main.rs:23-25`.
- **Shutdown:** `main.rs:104-110` uses `with_graceful_shutdown(shutdown_signal())`, which drains HTTP requests. Upgraded WebSockets run in detached tasks (`websocket.rs:86` `ws.on_upgrade` -> `handle_socket` at `websocket.rs:108`) that get no signal and are aborted when the runtime drops — every deploy hard-drops all connected clients with no close frame. Fix: a `CancellationToken` + `TaskTracker` carried on `GameBroadcaster` (already in `AppState` and already the state extracted by `ws_handler`, so no `AppState`/test-constructor churn); `main` cancels it after `shutdown_signal()` fires and waits (bounded) for the tracker to drain. The bot command consumer (`main.rs:55-74`) and email sweeps (`main.rs:75-80`) stay as-is: interrupting them is safe (un-acked NATS messages redeliver after `ack_wait`; sweeps are periodic and idempotent), and consumer supervision is web F53's territory (WP-38 bot-pipeline).
- **Crypto:** `rust/web/src/crypto.rs` is AES-256-GCM with fresh random 96-bit nonces, used only by `admin.rs` to encrypt/decrypt `llm_providers.api_key_encrypted` (call sites: `admin.rs:231/237, 274-276, 325-327, 499-503, 572+`, tests at `admin.rs:2180-2182, 2207, 2232, 2258, 2297`). `bot` has a **byte-identical sibling** `rust/bot/src/crypto.rs` that decrypts the SAME database rows (`bot/src/main.rs:757`).

**Tech Stack:** Rust 1.97.0 workspace at `/home/beefsack/Development/brdgme/rust`; `web` crate (server code behind `--features ssr`); kustomize overlays at `/home/beefsack/Development/brdgme/k8s/`; Tilt dev environment (`/home/beefsack/Development/brdgme/Tiltfile`). New deps (all already in `Cargo.lock` transitively, so no lockfile version churn): `rustls 0.23`, `zeroize 1`, `tokio-util 0.7`.

**Global Constraints:**

- All cargo commands run from `/home/beefsack/Development/brdgme/rust`. Per-crate only; web needs `--features ssr`: `cargo test -p web --features ssr`, `cargo clippy -p web --all-targets --features ssr -- -D warnings`. NEVER workspace-wide builds (AGENTS.md resource constraints).
- DB/NATS-backed web tests fail in a bare local run without the throwaway containers — pre-existing, known (backlog #40). The authoritative gate is `/home/beefsack/Development/brdgme/scripts/rust-test.sh`, which provides them; it MUST pass before the final commit.
- `cargo fmt --all -- --check` clean after every task.
- **k8s: the implementer NEVER applies anything to prod (no `kubectl apply`, no `argocd` commands). Verification of manifest changes is limited to `kubectl kustomize <overlay>` output inspection. The user deploys.**
- Do not touch `rust/bot/src/crypto.rs` or the shared ciphertext format (see Task 4 rationale).
- Migrations: none needed; do not create any.

**Non-Goals:**

- Auth fail-open findings and their decisions (WP-35, D-12/D-14); session mechanical/race findings (WP-34) — do not restructure `session.rs` beyond the `secure` flag and the new helper.
- WebSocket findings ws F59-F62 (WP-42) — no auth, backpressure, or per-game-subscription changes to `handle_socket`'s message loop beyond the one new shutdown select arm.
- Bot consumer supervision (web F53) and poison-message stranding (web F56) — WP-38. `main.rs:55-80` spawn structure stays.
- The insecure default encryption key fallback (`crypto.rs:42-51` + `main.rs:25-29` warn) — that is a separate finding ("fail closed on missing key", not in WP-36 scope); do not change `default_key`/`using_default_key` semantics beyond the return-type wrapper in Task 4.
- The client-side `theme` cookie (`app.rs:266-270`).
- CODING.md prose updates (optional F54 follow-up; the code fix supersedes the "record the answer" alternative).

**Snapshot drift:** Checked 2026-07-25 by diff against `/home/beefsack/Development/brdgme-review-snapshot/rust` (commit f8763a5). `web/src/crypto.rs`, `web/src/auth/session.rs`, `web/src/main.rs`, `web/src/websocket.rs`, `web/src/router.rs`, `web/src/state.rs`, `web/Cargo.toml`: byte-identical — all cited line numbers valid. `web/src/admin.rs` HAS drifted (#47 `can_replace_humans` bot work: type aliases and query shapes near the top shifted lines by a few), but every crypto call site cited above was re-grepped against the LIVE file and the quoted line numbers are live-file numbers. The snapshot contains only `rust/`; all k8s/Tiltfile analysis above is from the live repo (the F52 verification annotation's grep claims were independently re-confirmed live).

---

### Task 1: F52 — session cookie defaults to Secure (code)

**Problem (restated):** `rust/web/src/auth/session.rs:32-34` opts INTO security: `Secure` only if `SECURE_COOKIE=true`, and nothing sets it, so prod serves the session cookie without `Secure`. A browser holding a prod session will transmit the cookie on ANY plaintext `http://brdg.me/...` request (ssl-strip, a stray http link, a non-redirected endpoint) — the session token crosses the wire unencrypted and is interceptable, and it is a full login credential.

**Fix (re-derived):** Invert the default: secure unless the operator explicitly writes `SECURE_COOKIE=false`. Only the literal string `false` opts out — any other value (unset, `true`, garbage, `0`) stays secure, so misconfiguration fails safe. Extract the decision into a pure function so the env semantics are unit-testable without process-global env mutation (env-var tests race under parallel `cargo test`).

**Files:**
- `rust/web/src/auth/session.rs`

**Steps:**

- [ ] Add the failing test. In `rust/web/src/auth/session.rs`, append at the bottom of the file:
  ```rust
  #[cfg(all(test, feature = "ssr"))]
  mod tests {
      use super::secure_cookie;

      #[test]
      fn secure_cookie_defaults_to_secure_when_unset() {
          assert!(secure_cookie(None));
      }

      #[test]
      fn secure_cookie_explicit_false_opts_out() {
          assert!(!secure_cookie(Some("false")));
      }

      #[test]
      fn secure_cookie_any_other_value_stays_secure() {
          assert!(secure_cookie(Some("true")));
          assert!(secure_cookie(Some("0")));
          assert!(secure_cookie(Some("")));
          assert!(secure_cookie(Some("FALSE"))); // opt-out is the exact literal only
      }
  }
  ```
- [ ] Run: `cargo test -p web --features ssr secure_cookie` — expected: **compile error** (`secure_cookie` does not exist). That is the failing state.
- [ ] Implement. In `session.rs`, add above `create_session_layer`:
  ```rust
  /// `SECURE_COOKIE` env semantics: the session cookie carries the `Secure`
  /// attribute unless the value is the exact literal "false". Unset (prod
  /// k8s, which sets nothing) means Secure. The opt-out exists for dev
  /// environments served over plain HTTP on non-localhost hostnames
  /// (http://web.brdgme.lvh.me:8080 in-cluster Tilt), where browsers refuse
  /// Secure cookies entirely; see k8s/dev/web-patch.yaml and the Tiltfile
  /// local web resource.
  #[cfg(feature = "ssr")]
  fn secure_cookie(env_value: Option<&str>) -> bool {
      env_value != Some("false")
  }
  ```
  and replace lines 32-34 (`let secure = std::env::var("SECURE_COOKIE")...unwrap_or(false);`) with:
  ```rust
  let secure = secure_cookie(std::env::var("SECURE_COOKIE").ok().as_deref());
  ```
- [ ] Run: `cargo test -p web --features ssr secure_cookie` — expected: **3 tests PASS**. (The DB-backed tests elsewhere in the crate still need containers; filter keeps this fast.)
- [ ] Confirm no other behavior consumed the old default: `grep -rn "SECURE_COOKIE" rust/` must show only `session.rs`. Existing `tests/ssr_pages.rs` login tests are unaffected — `login_cookie` (ssr_pages.rs:70-90) inserts the session row directly and sends the `cookie` request header by hand; the `Secure` attribute constrains browsers, not test clients sending headers.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` and `cargo fmt --all -- --check` — clean.
- [ ] Commit: `fix(web): session cookie Secure by default, SECURE_COOKIE=false opts out (review ws F52, WP-36)`

### Task 2: F52 — dev opt-out wiring (k8s dev overlay, Tiltfile, .env.template)

**Problem (restated):** After Task 1, every environment without `SECURE_COOKIE=false` serves `Secure` cookies. Prod (HTTPS via Cloudflare/Gateway): correct, no manifest change needed. But both dev modes serve plain HTTP and must opt out explicitly:
1. **In-cluster dev** (Tilt `WEB_IN_CLUSTER=1`, `kustomize("k8s/dev")` at Tiltfile:111): web pod from unmodified `k8s/base/web/deployment.yaml`, reached at `http://web.brdgme.lvh.me:8080`. Non-localhost plain HTTP: browsers reject `Secure` cookies — login would silently break.
2. **Local dev** (default Tilt path, Tiltfile:129-135): `cargo leptos watch` on the host, `http://localhost:3000`. Chromium/Firefox accept `Secure` cookies on localhost (trustworthy-origin carve-out), Safari does not — set the opt-out explicitly rather than relying on per-browser carve-outs. `main.rs:14` runs `dotenvy::dotenv()`, so `rust/web/.env` is also honored for bare `cargo leptos watch` runs outside Tilt.

**Fix (re-derived):** (a) new dev-only kustomize patch — NOT a base env var, since both dev and prod kustomizations consume `base/web` and a base value would be unoverridable-per-env without patches anyway; (b) `SECURE_COOKIE=false` in the Tiltfile local web `serve_cmd`; (c) document in `rust/web/.env.template` (triage's `web/.env.template` path is actually `rust/web/.env.template` — confirmed, no file exists at `web/.env.template`). `k8s/base/web/deployment.yaml` itself needs NO change; it is listed in the finding only as the place prod env comes from.

**Files:**
- `k8s/dev/web-patch.yaml` (new)
- `k8s/dev/kustomization.yaml`
- `Tiltfile` (repo root)
- `rust/web/.env.template`

**Steps:**

- [ ] Create `/home/beefsack/Development/brdgme/k8s/dev/web-patch.yaml`:
  ```yaml
  # Dev-only: in-cluster dev web is served over plain HTTP at
  # http://web.brdgme.lvh.me:8080 (non-localhost, no TLS), where browsers
  # refuse cookies carrying the Secure attribute. The app defaults to
  # Secure (rust/web/src/auth/session.rs); this is the explicit dev opt-out.
  # Prod sets nothing and therefore gets Secure.
  apiVersion: apps/v1
  kind: Deployment
  metadata:
    name: web
  spec:
    template:
      spec:
        containers:
          - name: web
            env:
              - name: SECURE_COOKIE
                value: "false"
  ```
- [ ] Edit `/home/beefsack/Development/brdgme/k8s/dev/kustomization.yaml` — current full content is `namespace: brdgme` plus a `resources:` list of six `../base/*` entries; append at the end:
  ```yaml
  patches:
  - path: web-patch.yaml
  ```
- [ ] Verify overlays (build-only, no cluster access needed):
  - `kubectl kustomize /home/beefsack/Development/brdgme/k8s/dev | grep -B1 -A1 SECURE_COOKIE` — expected output includes:
    ```
    - name: SECURE_COOKIE
      value: "false"
    ```
  - `kubectl kustomize /home/beefsack/Development/brdgme/k8s/prod | grep -c SECURE_COOKIE` — expected output: `0` (prod inherits the secure code default; the grep exits 1, that is the pass condition).
  - `kubectl kustomize /home/beefsack/Development/brdgme/k8s/dev-without-web | grep -c "name: web"` — expected `0` (overlay has no web deployment; unaffected, nothing to patch there).
  - `kubectl kustomize /home/beefsack/Development/brdgme/k8s/dev > /dev/null && echo OK` — expected `OK` (whole overlay still builds).
- [ ] Edit `/home/beefsack/Development/brdgme/Tiltfile` line 131 (the `local_resource("web", serve_cmd=...)` in the `else:` branch): insert `SECURE_COOKIE=false ` into the env-prefix list, i.e. change
  `serve_cmd="cd rust/web && SQLX_OFFLINE=true DATABASE_URL=...`
  to
  `serve_cmd="cd rust/web && SQLX_OFFLINE=true SECURE_COOKIE=false DATABASE_URL=...`
  (everything else on the line unchanged).
- [ ] Edit `/home/beefsack/Development/brdgme/rust/web/.env.template` — in the `# Application configuration` block, add:
  ```
  # Session cookie Secure attribute. Defaults to secure (prod). Local dev over
  # plain http://localhost needs the explicit opt-out below (Safari, and any
  # non-localhost hostname in every browser, rejects Secure cookies over HTTP).
  SECURE_COOKIE=false
  ```
- [ ] Resulting behavior matrix (record in the PR/commit body):
  - prod k8s: `SECURE_COOKIE` unset -> `Secure` ON (the fix; takes effect when the user deploys the new image — no prod manifest change to apply).
  - dev k8s overlay (in-cluster web): patch sets `false` -> unchanged dev behavior.
  - Tilt local web / bare `cargo leptos watch` with a `.env` from the template: `false` -> unchanged dev behavior. A stale `.env` missing the var run outside Tilt yields Secure-on-localhost: still works in Chromium/Firefox; Safari users add the line per template.
  - tests: unaffected (header-level cookie handling, see Task 1).
- [ ] Commit: `fix(deploy): dev SECURE_COOKIE=false opt-out (k8s dev overlay, Tiltfile, .env.template) (review ws F52, WP-36)`

### Task 3: F54 — install rustls CryptoProvider in web's main

**Problem (restated):** Project rule (`docs/CODING.md:408-431`): with both rustls backend features enabled in a binary's graph, rustls 0.23 cannot auto-select a process-default `CryptoProvider` and PANICS at first use by any crate that reads the process default (this CrashLooped the operator in prod, 2026-07-08). `cargo tree -p web --features ssr` confirms web's resolved `rustls v0.23.42` carries BOTH `aws-lc-rs` and `ring` features (reqwest -> aws-lc-rs; sqlx/async-nats -> ring) — the finding's "UNVERIFIABLE which provider wins" is now resolved: neither wins, web is in the dual-backend state. Nothing panics today only because sqlx and reqwest select their backends explicitly; any future dependency that reads the process default (or an upstream refactor of an existing one) flips web into the panic. `web`'s `main` (`rust/web/src/main.rs:5`) installs nothing.

**Fix (re-derived):** Mirror `rust/operator/src/main.rs:23-25`: direct `rustls` dep with only the `aws-lc-rs` backend feature, `install_default()` as the first statement of `main`. First-statement placement means it can never race another install; `.expect` is fine (startup-only panics are allowed per project rules, and it only errs if a provider was already installed). ssr-optional so the WASM/hydrate build is untouched.

**Files:**
- `rust/web/Cargo.toml`
- `rust/web/src/main.rs`

**Steps:**

- [ ] In `rust/web/Cargo.toml` `[dependencies]`, next to the existing `reqwest` line (~line 70), add:
  ```toml
  # The workspace enables both rustls backends (reqwest -> aws-lc-rs;
  # sqlx/async-nats defaults -> ring), so rustls can't auto-select a
  # process-default provider. Direct dep exists solely for the
  # install_default() call in main.rs; see CODING.md's rustls section.
  rustls = { version = "0.23", default-features = false, features = ["aws-lc-rs"], optional = true }
  ```
  and add `"dep:rustls",` to the `ssr = [` feature list (e.g. after `"dep:reqwest",`).
- [ ] In `rust/web/src/main.rs`, make this the FIRST statement of the ssr `main` body (before `dotenvy::dotenv().ok();` at line 14; the `use` block at lines 6-12 stays above it):
  ```rust
  // Both rustls backends are enabled in this binary's graph (reqwest ->
  // aws-lc-rs, sqlx/async-nats -> ring), so any crate reading the process
  // default provider would panic without an explicit install. See
  // docs/CODING.md "rustls crypto backends" and rust/operator/src/main.rs.
  rustls::crypto::aws_lc_rs::default_provider()
      .install_default()
      .expect("failed to install rustls crypto provider");
  ```
- [ ] Run: `cargo clippy -p web --all-targets --features ssr -- -D warnings` — expected: clean compile (this also proves the dep/feature wiring; there is no unit-testable behavior — the guard is against a future dependency-graph state).
- [ ] Run: `cargo check -p web` (default features, lib target) — expected: clean, proving `rustls` stays out of the non-ssr build.
- [ ] Commit: `fix(web): install rustls aws-lc-rs provider in main (dual-backend panic guard) (review ws F54, WP-36)`

### Task 4: F17 — zeroize key material in crypto.rs (AAD declined)

**Problem (restated):** `rust/web/src/crypto.rs:17-40` — two hardening gaps per the finding: (1) no AAD, so an attacker with DB write access could swap `api_key_encrypted` blobs between `llm_providers` rows undetected; (2) the 32-byte master key (and decrypted plaintexts) linger in memory after use.

**Disposition (re-derived — partially OVERTURNS the recommendation):**
- **AAD binding: DECLINED.** Three concrete blockers found in live source: (a) `rust/bot/src/crypto.rs` is a byte-identical sibling that decrypts the SAME `llm_providers.api_key_encrypted` bytes (`bot/src/main.rs:757`) — a web-only format change breaks the bot, and bot is out of WP-36 scope; (b) the create flow encrypts BEFORE the row id exists (`admin.rs:274-276` encrypt, then `INSERT ... RETURNING id` at `admin.rs:283`), so row-id AAD would need an insert-then-update restructure; (c) existing prod ciphertexts have no AAD — decryption of every current row would fail without a re-encryption migration, and a decrypt-without-AAD fallback would nullify the binding it adds. Cost is grossly disproportionate to a nit-severity "marginal threat model" (finding's own words); the finding already marks both items "Optional".
- **Plaintext (decrypted API key) zeroization: DECLINED.** Every decrypt call site immediately converts to `String` and ships it in an HTTP Authorization header (`admin.rs:237-240`, `admin.rs:502-504`, etc.) — reqwest and the String copies outlive any wrapper on the intermediate `Vec`; wrapping it adds copies without removing exposure.
- **Master-key zeroization: IMPLEMENT.** Wrap the `[u8; 32]` in `zeroize::Zeroizing` at the source (`load_key`/`default_key`) so every caller's key copy is wiped on drop, and wipe the intermediate hex-decode buffer. Call sites need no changes for `&key` arguments: `&Zeroizing<[u8; 32]>` deref-coerces to `&[u8; 32]`, which is what `encrypt`/`decrypt` take (signatures unchanged).

**Files:**
- `rust/web/Cargo.toml`
- `rust/web/src/crypto.rs`
- `rust/web/src/admin.rs` (one test helper's return type)

**Steps:**

- [ ] Add regression tests first (web's `crypto.rs` currently has none; these pass against the OLD code too apart from the type change — the zeroize property itself is not observably testable, so this is compile-driven, not red/green). Append to `rust/web/src/crypto.rs`:
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;

      #[test]
      fn encrypt_decrypt_roundtrip() {
          let key = default_key();
          let ct = encrypt(&key, b"secret api key").unwrap();
          assert_ne!(&ct[12..], b"secret api key");
          assert_eq!(decrypt(&key, &ct).unwrap(), b"secret api key");
      }

      #[test]
      fn decrypt_rejects_tampered_ciphertext() {
          let key = default_key();
          let mut ct = encrypt(&key, b"secret").unwrap();
          let last = ct.len() - 1;
          ct[last] ^= 0x01;
          assert!(matches!(decrypt(&key, &ct), Err(CryptoError::DecryptionFailed)));
      }

      #[test]
      fn decrypt_rejects_short_input() {
          let key = default_key();
          assert!(matches!(decrypt(&key, &[0u8; 11]), Err(CryptoError::DecryptionFailed)));
      }
  }
  ```
  (No env-var tests here: `load_key` env cases are covered by the sibling `bot` crate's tests, and env mutation races parallel test threads in `web`.)
- [ ] In `rust/web/Cargo.toml` `[dependencies]`, next to `aes-gcm` (~line 71): `zeroize = { version = "1", optional = true }`; add `"dep:zeroize",` to the `ssr = [` list (after `"dep:hex",`).
- [ ] Implement in `rust/web/src/crypto.rs`:
  - Add `use zeroize::{Zeroize, Zeroizing};` to the imports.
  - `default_key` (line 42): return `Zeroizing<[u8; 32]>`; body becomes `let mut key = Zeroizing::new([0u8; 32]);` ... (copy_from_slice unchanged) ... `key`.
  - `load_key` (line 53): return `Result<Zeroizing<[u8; 32]>, CryptoError>`; replace the `try_into` (which moves the `Vec` and frees its buffer unwiped) with an explicit length check + copy + wipe:
    ```rust
    pub fn load_key() -> Result<Zeroizing<[u8; 32]>, CryptoError> {
        let hex_str = match std::env::var("DATABASE_ENCRYPTION_KEY") {
            Ok(v) => v,
            Err(_) => return Ok(default_key()),
        };
        let mut bytes = hex::decode(&hex_str).map_err(|_| CryptoError::InvalidHex)?;
        if bytes.len() != 32 {
            bytes.zeroize();
            return Err(CryptoError::InvalidKeyLength);
        }
        let mut key = Zeroizing::new([0u8; 32]);
        key.copy_from_slice(&bytes);
        bytes.zeroize();
        Ok(key)
    }
    ```
  - `encrypt`/`decrypt`/`using_default_key`/`rand_nonce`: UNCHANGED.
- [ ] In `rust/web/src/admin.rs` tests (~line 2180), change the helper's return type only:
  ```rust
  fn test_encryption_key() -> zeroize::Zeroizing<[u8; 32]> {
      crate::crypto::load_key().unwrap()
  }
  ```
  Its four callers (`admin.rs` ~2207, ~2232, ~2258, ~2297) all use `&key` — deref coercion, no edits. The five production `load_key` call sites (`admin.rs:231, 274, 325, 499, 572`) likewise pass `&key` and need no edits; `main.rs:25` calls only `using_default_key()` (untouched).
- [ ] Run: `cargo test -p web --features ssr crypto::` — expected: **3 tests PASS**.
- [ ] Run: `cargo clippy -p web --all-targets --features ssr -- -D warnings` — clean (this compiles the admin tests, proving the coercion claim).
- [ ] Commit: `fix(web): zeroize AES master key material in crypto.rs (review ws F17, WP-36; AAD declined - shared format with bot + existing prod ciphertexts)`

### Task 5: F55 — WebSocket close frames on graceful shutdown

**Problem (restated):** `main.rs:104-110` `with_graceful_shutdown(shutdown_signal())` drains HTTP, but each upgraded WebSocket lives in a detached task (`websocket.rs:86` `on_upgrade` -> `handle_socket`, line 108) that never learns about shutdown; when `main` returns and the runtime drops, those tasks are aborted mid-write — every deploy hard-drops all connected clients with no close frame (clients see an abnormal closure and must rely on reconnect logic). The bot consumer and sweep tasks also get no signal, but interrupting them is harmless (NATS redelivery; periodic idempotent sweeps) and their supervision is web F53 / WP-38 — out of scope here.

**Fix (re-derived):** Put a `tokio_util::sync::CancellationToken` and `tokio_util::task::TaskTracker` on `GameBroadcaster` — it is already the state `ws_handler` extracts and already lives in `AppState`, so `main.rs` and both test harnesses (`tests/websocket_hygiene.rs` `spawn_app`, `tests/ssr_pages.rs`) keep their existing constructors: `GameBroadcaster::new` creates a fresh token/tracker internally. `ws_handler` wraps each socket future in `tracker.track_future`; `handle_socket` gains one `select!` arm that, on cancellation, sends a proper `Message::Close(None)` and breaks. `main` cancels after `shutdown_signal()` fires (inside the future handed to `with_graceful_shutdown`) and, after `axum::serve` returns, waits up to 5s for the tracker to drain so close frames actually flush before process exit.

**Files:**
- `rust/web/Cargo.toml`
- `rust/web/src/websocket.rs`
- `rust/web/src/main.rs`
- `rust/web/tests/websocket_hygiene.rs`

**Steps:**

- [ ] In `rust/web/Cargo.toml` `[dependencies]`: `tokio-util = { version = "0.7", features = ["rt"], optional = true }` (the `rt` feature provides `TaskTracker`; `CancellationToken` needs no feature); add `"dep:tokio-util",` to the `ssr = [` list (after `"dep:tokio",`).
- [ ] Write the failing test. In `rust/web/tests/websocket_hygiene.rs`, add (reusing the existing `spawn_app` helper, which already returns `(SocketAddr, GameBroadcaster)`, and the file's existing tokio-tungstenite client pattern):
  ```rust
  /// review ws F55: on graceful shutdown the server must send a proper close
  /// frame instead of hard-dropping the TCP connection.
  #[sqlx::test]
  async fn shutdown_sends_close_frame_to_connected_websockets(pool: PgPool) {
      let (addr, broadcaster) = spawn_app(pool).await;
      let (mut ws, _resp) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws"))
          .await
          .expect("ws connect");

      broadcaster.begin_shutdown();

      let deadline = Instant::now() + Duration::from_secs(5);
      loop {
          let remaining = deadline.saturating_duration_since(Instant::now());
          match timeout(remaining, ws.next()).await {
              Ok(Some(Ok(Message::Close(_)))) => break, // PASS
              Ok(Some(Ok(_))) => continue,              // pings etc.
              other => panic!("expected close frame before timeout, got: {other:?}"),
          }
      }
  }
  ```
- [ ] Run: `cargo test -p web --features ssr shutdown_sends_close_frame --no-run` — expected: **compile error** (`begin_shutdown` does not exist). (Full execution needs the rust-test.sh containers; compile failure is the red state.)
- [ ] Implement in `rust/web/src/websocket.rs` (all inside the `ssr` module):
  - Imports: `use tokio_util::sync::CancellationToken; use tokio_util::task::TaskTracker;`
  - `GameBroadcaster` (line 29-32) gains fields:
    ```rust
    #[derive(Clone)]
    pub struct GameBroadcaster {
        client: async_nats::Client,
        shutdown: CancellationToken,
        ws_tasks: TaskTracker,
    }
    ```
  - `new` (line 35): `Self { client, shutdown: CancellationToken::new(), ws_tasks: TaskTracker::new() }`.
  - New methods on `impl GameBroadcaster`:
    ```rust
    /// Signals every live websocket task to send a close frame and exit,
    /// and closes the tracker so `drain_ws_tasks` can complete. Called from
    /// main's graceful-shutdown path (review ws F55).
    pub fn begin_shutdown(&self) {
        self.shutdown.cancel();
        self.ws_tasks.close();
    }

    /// Resolves when all tracked websocket tasks have finished. Only
    /// terminates after `begin_shutdown` (the tracker must be closed).
    pub async fn drain_ws_tasks(&self) {
        self.ws_tasks.wait().await;
    }
    ```
  - `ws_handler` (line 82-87): track the socket future:
    ```rust
    pub async fn ws_handler(
        ws: WebSocketUpgrade,
        State(broadcaster): State<GameBroadcaster>,
    ) -> impl IntoResponse {
        let tracker = broadcaster.ws_tasks.clone();
        ws.on_upgrade(move |socket| tracker.clone().track_future(handle_socket(socket, broadcaster)))
    }
    ```
    (If clippy objects to the double clone shape, bind `let tracked = tracker.track_future(...)` inside the closure body instead — behavior identical.)
  - `handle_socket` (line 108): before the loop, `let shutdown = broadcaster.shutdown.clone();` (place next to the existing subscribe setup — note `broadcaster` is moved into the NATS subscriptions' scope but stays alive; it is the function argument), and add ONE new `select!` arm alongside the existing four (lines 133-172):
    ```rust
    _ = shutdown.cancelled() => {
        // Graceful deploy: tell the client we're going away instead of
        // hard-dropping the TCP stream (review ws F55). Best-effort.
        let _ = sender.send(Message::Close(None)).await;
        break;
    }
    ```
    Do NOT touch the existing arms (WS message-loop findings F59-F62 belong to WP-42).
- [ ] Implement in `rust/web/src/main.rs`:
  - Replace `.with_graceful_shutdown(shutdown_signal())` (line 108) with:
    ```rust
    .with_graceful_shutdown({
        let broadcaster = broadcaster.clone();
        async move {
            shutdown_signal().await;
            broadcaster.begin_shutdown();
        }
    })
    ```
  - After the `axum::serve(...).await.unwrap();` statement (line 110), append:
    ```rust
    // Bounded wait so websocket close frames flush before the runtime is
    // dropped (which would abort the tasks mid-write). The bot command
    // consumer and email sweeps are deliberately NOT awaited: un-acked
    // bot.commands redeliver after ack_wait, and sweeps are periodic and
    // idempotent (their supervision is a separate finding, review ws F53).
    if tokio::time::timeout(std::time::Duration::from_secs(5), broadcaster.drain_ws_tasks())
        .await
        .is_err()
    {
        tracing::warn!("websocket tasks did not drain within 5s of shutdown");
    }
    ```
- [ ] Run: `cargo clippy -p web --all-targets --features ssr -- -D warnings` — clean (proves the test now compiles and existing harnesses needed no changes).
- [ ] Run the new test under the container harness as part of the final gate (next task) — expected: `shutdown_sends_close_frame_to_connected_websockets` **PASS**, and the pre-existing `websocket_hygiene` idle-survival test still PASS (the new select arm must not perturb the ping loop).
- [ ] Commit: `fix(web): send WS close frames on graceful shutdown via CancellationToken/TaskTracker (review ws F55, WP-36)`

### Task 6: Final gate

- [ ] Run `/home/beefsack/Development/brdgme/scripts/rust-test.sh` — expected: full pass (spins up Postgres/NATS containers; runs fmt, both clippy splits, sqlx prepare check, workspace-minus-web tests, `cargo test -p web --features ssr` including the new session/crypto/websocket tests). This is mandatory before the package is considered done (AGENTS.md).
- [ ] Re-run the three kustomize verification commands from Task 2 and record their output in the PR body.
- [ ] Remind the user in the final report: **prod picks up F52/F54/F55 when they deploy the new web image; no prod manifest change exists to apply.** Do not run any kubectl/argocd mutation.

---

## Findings disposition

| Finding | Severity | Disposition | Notes |
|---|---|---|---|
| ws F52 — session cookies lack Secure flag in prod | major | FIX (Tasks 1-2) | Triage note VALIDATED: dev overlay consumes `base/web` unpatched and in-cluster dev is plain HTTP on a non-localhost hostname (`web.brdgme.lvh.me:8080`), so a base env var would break dev login. Default-secure-in-code (`Secure` unless literal `SECURE_COOKIE=false`) + dev opt-out in new `k8s/dev/web-patch.yaml`, Tiltfile local web env, `.env.template`. Prod needs no manifest change. Removal cookie inherits the flag via the shared `SessionManagerLayer`; no websocket.rs change needed. Triage path `web/.env.template` corrected to `rust/web/.env.template`. |
| ws F54 — no rustls CryptoProvider installed in web's main | minor | FIX (Task 3) | Finding's UNVERIFIABLE resolved: `cargo tree` shows web's rustls with BOTH `aws-lc-rs` and `ring` features — dual-backend state is real, guard justified. Operator's exact pattern copied; ssr-only optional dep. |
| ws F55 — graceful shutdown misses WS/background tasks | minor | FIX WS half (Task 5); ACCEPT background half | Close frame + bounded drain via `CancellationToken`/`TaskTracker` on `GameBroadcaster` (no `AppState`/test-constructor churn). Bot consumer + sweeps deliberately not signaled: interruption is safe (NATS redelivery, idempotent sweeps) and consumer supervision is web F53 / WP-38. |
| ws F17 — no AAD binding, no key zeroization | nit | FIX zeroize half (Task 4); DECLINE AAD + plaintext-Vec zeroization | AAD overturned on live-source evidence: `rust/bot/src/crypto.rs` decrypts the same rows (format is shared), create-flow encrypts before the row id exists, and existing prod ciphertexts would break without a re-encryption migration — disproportionate for an "Optional" nit. Master key wrapped in `Zeroizing` at `load_key`/`default_key`; hex buffer wiped; call sites unchanged via deref coercion except one test-helper return type. |
