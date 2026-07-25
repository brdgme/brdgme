# Raw findings: `rust/web/src/admin.rs` (2,299 lines)

Reviewer: Worker subagent (read-only review, snapshot worktree
`/home/beefsack/Development/brdgme-review-snapshot`).
**All 2,299 lines of `rust/web/src/admin.rs` were read in full** (three Read
passes: 1–800, 800–1600, 1600–2299). No code was edited; no builds/tests run.

Cross-references read for ground truth:

- `rust/web/src/crypto.rs` (full) — AES-256-GCM, random 96-bit nonce, key from
  `DATABASE_ENCRYPTION_KEY` env (hex), dev default key otherwise.
- `rust/web/src/db.rs:560-566` — `is_user_admin` (SELECT is_admin, missing row
  → false; fail-closed).
- `rust/web/src/error.rs:7-12` — `internal()` logs real error via tracing,
  returns opaque "Internal server error" (intentional per brief; not flagged).
- `rust/web/migrations/013_bot_efficacy.sql` (full) — schema for `bots`,
  `llm_providers`, `bot_providers` (FKs `ON DELETE CASCADE`,
  `UNIQUE (bot_id, provider_id, model)`).
- `rust/web/src/main.rs:32-36` — shared `reqwest::Client` has
  connect_timeout 5s, total timeout 10s.
- `rust/bot/src/config.rs` (full) — runtime bot/provider selection
  (`load_bot_config` by **bot name**, `load_providers` ORDER BY priority,
  created_at). Round-robin/priority selection lives in the `bot` crate,
  **outside the review target**; admin.rs only stores the fields.
- `expect_context::<PgPool>()` in server fns is the codebase-wide convention
  (75 occurrences across 9 files) — not flagged.

---

## Pass 1 — Server functions, admin gating, crypto handling

### CLEAN: every admin server fn is gated server-side
All 12 `#[server]` functions (`admin_list_bots`, `admin_create_bot`,
`admin_update_bot`, `admin_reorder_bots`, `admin_delete_bot`,
`admin_list_providers`, `admin_create_provider`, `admin_update_provider`,
`admin_delete_provider`, `admin_list_bot_providers`,
`admin_create_bot_provider`, `admin_update_bot_provider`,
`admin_delete_bot_provider`, `admin_test_provider`,
`admin_test_bot_provider`) perform the identical, correct sequence:
`get_current_user().await?` → `ok_or("Not authenticated")` →
`is_user_admin(&pool, user.id)` → `Err("Admin access required")` when false.
The check is server-side (inside the server fn body), uses the session-derived
user id (never a client-supplied id), and `is_user_admin` fails closed
(unknown user → false). No privilege-escalation path found: no server fn
trusts a client-supplied user/admin flag, and the non-`#[server]` DB helpers
(`list_bots`, `create_bot`, …) are only reachable through the gated wrappers.

### CLEAN: no plaintext API key exposure in responses
`list_providers` (admin.rs:210-249) decrypts only to compute a last-4 mask;
`create_provider` (admin.rs:279-289) masks from the input plaintext the same
way. Full keys never leave the server in any response struct (`ProviderRow`
carries `api_key_masked` only). Error paths go through `internal()` which
logs server-side and returns an opaque message — decrypt/utf8/load-key
failures cannot leak key material to the client. `test_provider` /
`test_bot_provider` put the key only in the upstream `Authorization` header.

### CLEAN: SQL injection
All queries use bound parameters (`$1…$n`); no string interpolation of user
input into SQL anywhere in the file. No dynamic table/column names.

### reorder_bots is not atomic (no transaction)
- severity: major
- category: correctness
- location: web/src/admin.rs:184-197
- finding: `reorder_bots` runs N separate `UPDATE bots SET display_order …`
  statements on the pool with no transaction. Any failure mid-loop (DB error,
  connection drop) leaves display_order in a half-updated mix of old and new
  orderings, and the admin UI then re-fetches and renders that mangled state.
  Also N round-trips for what is logically one write. Interleaving with a
  concurrent `create_bot` (which computes `MAX(display_order)+1`,
  admin.rs:136) can also produce duplicate display_order values — there is no
  unique constraint on `display_order` in migration 013.
- recommendation: wrap the loop in a single transaction
  (`pool.begin().await?` … `tx.commit().await?`), or better, one statement:
  `UPDATE bots SET display_order = u.ord, updated_at = now() FROM
  (SELECT * FROM unnest($1::uuid[]) WITH ORDINALITY AS u(id, ord)) u
  WHERE bots.id = u.id`. A new migration adding a deferrable uniqueness story
  for display_order is optional; the transaction is the real fix.

### create_bot display_order race (MAX+1 without lock)
- severity: minor
- category: correctness
- location: web/src/admin.rs:134-145
- finding: `COALESCE((SELECT MAX(display_order) + 1 FROM bots), 0)` is a
  read-modify-write with no isolation; two concurrent creates can compute the
  same value, giving duplicate display_order and an ambiguous admin ordering.
  Low likelihood (admin-only, single operator) but real.
- recommendation: acceptable to leave, or serialize via the same transaction
  fix as reorder, or `SELECT … FOR UPDATE`. If a uniqueness constraint is ever
  desired it must be a NEW migration (migrations are immutable).

### API key mask exposes the whole key for keys ≤ 4 chars; `sk-...` prefix is fabricated
- severity: minor
- category: quality
- location: web/src/admin.rs:228-236 (list), 279-289 (create)
- finding: the mask is `format!("sk-...{last4}")`. For a key of ≤ 4 chars the
  "last 4" is the entire key, so `api_key_masked` round-trips the full
  plaintext secret to the client. For keys not starting with `sk-` (many
  providers use other prefixes) the displayed mask is misleading. Edge case,
  but it defeats the masking guarantee the tests
  (`test_admin_list_providers_never_returns_full_key`, admin.rs:2168) assert
  for normal-length keys.
- recommendation: if `plaintext.len() <= 4` (or some small threshold), return
  a fixed mask like `"•••"` or `Some("(set)".to_string())`; consider dropping
  the hardcoded `sk-` prefix and showing only `...{last4}`.

### No way to clear a provider API key once set
- severity: minor
- category: correctness
- location: web/src/admin.rs:301-341 (update_provider), 1620 (UI help text)
- finding: `api_key: Option<String>` conflates "keep existing" (None) with
  any desire to remove the key — `None` skips the column, `Some("")` would
  store an empty key (the UI filters empty→None, but the server fn is a
  public API surface). There is no mass-assignment hole here (columns are
  enumerated explicitly), but the operation "unset key" is unrepresentable.
- recommendation: if clearing is wanted, use a tri-state (e.g. an enum or a
  separate `clear_api_key: bool`); otherwise document that keys can only be
  replaced.

### update_bot_provider omits `updated_at = now()`
- severity: nit
- category: consistency
- location: web/src/admin.rs:446-458
- finding: every other UPDATE in the file (`update_bot` :169,
  `update_provider` :316/:329, `reorder_bots` :189) sets `updated_at =
  now()`; `update_bot_provider` does not, so the column silently goes stale
  for that table. The `bot_providers` schema has `updated_at` (migration
  013:32).
- recommendation: add `updated_at = now()` to the UPDATE.

### Mutations don't verify rows affected
- severity: minor
- category: quality
- location: web/src/admin.rs:159-207 (update_bot/delete_bot), 301-351
  (update_provider/delete_provider), 437-469
  (update_bot_provider/delete_bot_provider)
- finding: all UPDATE/DELETE helpers `.execute()` and discard
  `rows_affected()`. Updating or deleting a non-existent id returns `Ok(())`,
  so the admin UI reports success for a no-op (e.g. stale row after another
  admin deleted it). Low stakes in a single-admin tool.
- recommendation: check `.rows_affected() == 0` and return a "not found"
  `ServerFnError` where the UX matters (updates at least); deletes can
  reasonably stay idempotent.

## Pass 2 — test_provider / test_bot_provider (network paths)

### test_provider hardcodes model "gpt-4o-mini"
- severity: minor
- category: correctness
- location: web/src/admin.rs:496-501
- finding: the provider-level test always sends `"model": "gpt-4o-mini"`.
  Providers that don't serve that exact model id (most non-OpenAI endpoints,
  and the project's own links use ids like `openai/gpt-4o-mini` per the form
  help text at :2013) will fail the test even when healthy — a false
  negative built into the admin's primary health check. The per-link test
  (`test_bot_provider`) does use the configured model.
- recommendation: take the model as a parameter, or reuse one of the
  provider's configured `bot_providers.model` values (e.g. `SELECT model FROM
  bot_providers WHERE provider_id = $1 LIMIT 1`) with a fallback.

### Upstream response body/headers returned to client uncapped
- severity: minor
- category: quality
- location: web/src/admin.rs:511-528 (test_provider), 599-615
  (test_bot_provider)
- finding: `resp.text().await` reads the upstream body with no size limit and
  the result (plus all response headers) is returned verbatim in the server
  fn response and rendered in the admin page. A misbehaving or hostile
  provider endpoint (admin-configurable URL) can return an arbitrarily large
  body within the client's 10s timeout → memory spike and a huge serialized
  response. Error bodies from upstream may also embed sensitive detail
  (occasionally including echoed credentials) — admin-only visibility
  mitigates this, but the data is then also in the Leptos action payload.
- recommendation: cap the read (e.g. `resp.chunk()` loop or take first N KB)
  and consider trimming the header list to a safe subset.

### test result can be attributed to the wrong row (Action input/value race)
- severity: minor
- category: correctness
- location: web/src/admin.rs:1424-1435 (ProvidersSection), 1760-1771
  (BotProvidersSection)
- finding: the completion Effect reads `test_action.input().get()` (the most
  recently *dispatched* input) together with `test_action.value().get()` (the
  most recently *completed* value). Leptos `Action` allows multiple in-flight
  dispatches; the UI only blocks re-dispatch for the *same* id
  (`disabled` logic at :1472-1475 and :1833-1837), so tests for two different
  providers can overlap. If provider A's request resolves after B was
  dispatched, A's result is stored under B's id and rendered in B's row.
  (Uncertainty: exact Leptos 0.8 `Action::input()` semantics — it tracks the
  latest dispatch, not the dispatch that produced `value()`.) Same pattern in
  the bot-provider section, keyed on `(bp_id, _)`.
- recommendation: key results off the resolved future itself — e.g. dispatch
  with the id and have the async block return `(Uuid, Result<…>)`, then read
  only `value()`.

### CLEAN: SSRF posture
`test_provider`/`test_bot_provider` fetch admin-configured URLs server-side;
this is inherent to the feature and gated to admins (who can already set the
URL via `admin_create_provider`). The shared client has 5s connect / 10s
total timeouts (main.rs:32-36). Not a finding; noted for completeness.

### Local `type BotProviderRow` shadows the public struct
- severity: nit
- category: quality
- location: web/src/admin.rs:538-544
- finding: inside `test_bot_provider` a tuple type alias named
  `BotProviderRow` shadows the file's public `BotProviderRow` struct
  (:53-65). Confusing when reading/refactoring; no behavioral impact.
- recommendation: rename the local alias (e.g. `BotProviderTestRow`).

## Pass 3 — Server-fn input validation, UI components, tests

### No server-side validation of bot/provider inputs
- severity: minor
- category: quality
- location: web/src/admin.rs:637-666 (create_bot), 668-701 (update_bot),
  760-781 (create_provider), 846-879 (create_bot_provider)
- finding: all constraints live in the HTML form (`min="0" max="2"` on
  temperature, `required` on name/url/model). The server fns accept any
  values a crafted request sends: empty names, temperature outside 0–2
  (including negative or huge values; non-finite f32 like NaN — depending on
  the server-fn codec, NaN may fail serde deserialization before reaching the
  handler, but there is no explicit guard either way; Postgres REAL accepts
  'NaN'), arbitrary-length strings, unbounded `extra_body` JSONB and
  unbounded `prompt` length in `admin_test_bot_provider` (:955-976). Admin
  gating caps the blast radius, but there is no defense-in-depth.
  (Uncertainty: whether the default Leptos 0.8 server-fn encoding rejects
  non-finite floats at the boundary was not verified.)
- recommendation: cheap server-side checks where invariants are real:
  non-empty trimmed `name`/`url`/`model`, `temperature.is_finite() && (0.0..=2.0).contains(&temperature)`
  (DB has no CHECK constraint; adding one would require a NEW migration),
  and a sane length cap on `prompt`.

### Admin-gate boilerplate duplicated 15 times
- severity: minor
- category: simplicity
- location: web/src/admin.rs:618-976 (every `#[server]` fn)
- finding: each server fn repeats the same 6-line authenticate + is_user_admin
  + reject block verbatim (15 copies in this file). Beyond noise, the risk is
  a future admin fn forgetting the check — the repetition is the hazard.
  There is precedent for a helper in the same codebase: `proposals.rs` uses
  `crate::friends::require_user().await?` (:1050, :1296) for the auth half.
- recommendation: extract
  `async fn require_admin(pool: &PgPool) -> Result<User, ServerFnError>`
  (auth + admin check) and call it in one line per server fn.

### AdminPage non-admin redirect matches on error-message text
- severity: nit
- category: quality
- location: web/src/admin.rs:1005-1012
- finding: `msg.contains("Admin access required")` decides the redirect to
  `/`. If the server message wording changes, non-admins silently stop being
  redirected and just see an error. String-matching on error text is fragile
  coupling across the wire.
- recommendation: match on a structured variant if ServerFnError ever carries
  one, or centralize the message in a shared constant used by both sides.

### Action-value `.unwrap()`s in completion Effects
- severity: nit
- category: quality
- location: web/src/admin.rs:1089-1137, 1386-1435, 1722-1771
- finding: pattern
  `if action.value().get().is_some() && !action.pending().get() { match action.value().get().unwrap() { … } }`
  appears 10 times. Each `.unwrap()` is reachable only after an `is_some()`
  guard on the same signal within one synchronous reactive run, so it cannot
  fire on `None` in practice (client-side single-threaded reactive graph);
  this is not a panic risk, but it is 10 unwraps that rely on that invariant
  and will be copied into future code.
- recommendation: `if let Some(res) = action.value().get() { … }` collapses
  the check and the unwrap. (Project no-unwrap rule targets server handlers;
  flagged as polish only.)

### Admin page hard-fails entirely if any single provider key is undecryptable
- severity: minor
- category: quality
- location: web/src/admin.rs:218-247 (list_providers), 1019-1037 (AdminPage)
- finding: in `list_providers` a single corrupt/undecryptable
  `api_key_encrypted` row (e.g. after an encryption-key rotation that missed
  a row, or manual DB tampering) makes the whole listing `Err`, and because
  `AdminPage` renders `(Some(Err(e)), _) | (_, Some(Err(e)))` as a full-page
  error, one bad row takes down the entire admin UI — bots, providers, and
  links all unmanageable until the row is fixed via direct DB access.
- recommendation: degrade per-row: on decrypt failure, render that provider
  with `api_key_masked: Some("(undecryptable)".into())` (or log + mask)
  instead of failing the list.

### Renaming/deleting a bot can strand in-flight games (cross-module coupling)
- severity: minor
- category: correctness
- location: web/src/admin.rs:159-181 (update_bot allows free rename), 200-207
  (delete_bot)
- finding: games reference bots by **name** (`game_bots.bot_name`, migration
  013:36), and the bot worker resolves config at turn time with
  `load_bot_config(pool, bot_name)` → `WHERE name = $1 AND enabled = true`
  (rust/bot/src/config.rs:25-33). Admin rename or delete of a bot therefore
  changes/breaks resolution for existing games using that bot — likely
  degrading to "bot not found" behavior rather than a hard failure.
  (Uncertainty: the worker's fallback path on `None` was not traced; impact
  depends on it. Also outside admin.rs proper, but admin.rs is the mutation
  surface that creates the condition.)
- recommendation: at minimum surface a warning in the delete/rename flows;
  structurally, referencing bots by id from `game_bots` would remove the
  coupling (schema change → NEW migration).

## Explicitly checked and CLEAN

- **Admin gating**: all 15 server fns gated, server-side, session-derived
  user, fail-closed helper. No client-supplied identity or admin flag
  consumed anywhere.
- **Key material**: no plaintext key in any response DTO, log path
  (`internal()` logs sqlx/crypto errors only — sqlx errors do not include
  bound values, crypto errors are fixed strings), or error message.
  Encryption is AES-256-GCM with random 96-bit nonces per message
  (crypto.rs) — nonce handling correct.
- **SQL**: fully parameterized; join/returning logic in
  `create_bot_provider` (:406-420) is correct (FK constraints from migration
  013 guarantee the name subselects resolve; `UNIQUE(bot_id, provider_id,
  model)` violations surface as opaque internal errors — acceptable).
- **Panics/unwraps in server request paths**: none. All server-side
  fallible operations propagate via `?` / `map_err(internal(...))` /
  `ok_or_else`. The `unwrap_or_else`/`unwrap_or` uses in test_provider
  (:513-516, :605-608) are non-panicking fallbacks. `expect_context` is the
  established codebase convention (75 uses).
- **Error swallowing**: none found server-side; every DB/crypto/HTTP error
  is either propagated or converted to a deliberate user-facing `Ok(msg)`
  (e.g. upstream HTTP error body at :517 — intended "test result" data, not
  a swallowed error).
- **Round-robin/priority selection races**: selection logic is not in this
  file; it lives in `rust/bot/src/config.rs:50-96` (`ORDER BY bp.priority
  ASC, bp.created_at ASC` — deterministic, no shared mutable state in
  admin.rs). Admin.rs only stores `priority`; no race in-scope. The only
  concurrency defects found in-file are the reorder transaction gap and the
  MAX+1 create race above.
- **Mass assignment**: all update statements enumerate columns explicitly;
  no client-controlled column selection.
- **Deletes**: FK `ON DELETE CASCADE` on `bot_providers` (migration
  013:25-26) makes `delete_bot`/`delete_provider` referentially safe for
  links.
- **Tests** (:2139-2299): `sqlx::test` coverage for non-admin rejection,
  mask-never-contains-key, encrypt-on-create, and key preserve/replace on
  update — good coverage of the crypto-critical paths. `unwrap()`s here are
  test-only and fine.
