# Verification: web-server crypto.rs + admin.rs (F16-F33)

Snapshot: /home/beefsack/Development/brdgme-review-snapshot/rust (commit f8763a5).
All line numbers are snapshot lines. Verifier: independent read of
web/src/crypto.rs (full), web/src/main.rs (full), web/src/admin.rs (full, 2300
lines), web/migrations/001 + 013, bot/src/config.rs, bot/src/main.rs, and the
vendored reactive_graph 0.2.14 Action source
(~/.cargo/registry/.../reactive_graph-0.2.14/src/actions/action.rs).

---

## F16 (major) Hardcoded fallback key when DATABASE_ENCRYPTION_KEY unset

**Verdict: CONFIRMED. Severity: major (agree).**

- crypto.rs:53-57: `load_key()` returns `Ok(default_key())` on `Err(_)` from
  `std::env::var("DATABASE_ENCRYPTION_KEY")`. crypto.rs:42-47: `default_key()`
  is the in-repo constant `b"brdgme-dev-key-not-for-prod!!!"` zero-padded to 32
  bytes.
- crypto.rs:49-51: `using_default_key()` is `std::env::var(...).is_err()` -
  presence-only, exactly as claimed; a set-but-malformed value passes this
  check and only fails later inside `load_key()` at first use.
- main.rs:25-29: warn-and-continue only (`tracing::warn!`), no startup
  validation call to `load_key()`.
- Deployment context (live repo, outside snapshot): k8s/base/web/deployment.yaml:38-39
  does `envFrom: secretRef: database-encryption-key` (not optional), so prod
  pods fail to start if the secret is absent - but a secret that exists with a
  wrong inner key name would silently produce the fallback. The fail-open code
  path is real.

**Recommendation validity:** Valid. Fail-closed startup + one `load_key()`
call in main is correct and cheap; startup panics are allowed per project
rules (main.rs already uses `.expect()` for pool/NATS).

---

## F17 (nit) No AAD binding, no zeroize

**Verdict: CONFIRMED. Severity: nit (agree).**

- crypto.rs:17-28 `encrypt` and :30-40 `decrypt` use plain
  `Aead::encrypt/decrypt` with no associated data; ciphertexts are not bound
  to any row identity, so an attacker with DB write access could swap
  `api_key_encrypted` blobs between provider rows.
- No `zeroize` anywhere in the crate; `load_key()` returns `[u8; 32]` by
  value and decrypted plaintexts are ordinary `Vec<u8>`/`String`s
  (admin.rs:224-227, 489-491, 562-564).

**Recommendation validity:** Valid (optional hardening; note using row id as
AAD means decryption breaks if rows are ever re-keyed/copied - acceptable
tradeoff, worth mentioning at fix time).

---

## F18 (major) reorder_bots not atomic

**Verdict: CONFIRMED. Severity: suggest DOWNGRADE major -> minor.**

- admin.rs:184-197: `for (i, id) in ordered_ids.iter().enumerate()` issues one
  `UPDATE bots SET display_order = $2 ... WHERE id = $1` per id via
  `.execute(pool)` - separate pool connections, no transaction. A mid-loop
  failure leaves mixed orderings; concurrent `create_bot` (MAX+1, admin.rs:136)
  can interleave to duplicates.
- Schema claim verified: migrations/013_bot_efficacy.sql:1-11 - `bots` has
  `name TEXT NOT NULL UNIQUE` but NO unique constraint on `display_order`; no
  later migration adds one (grep across migrations/*.sql).
- Severity opinion: the corrupted state is only bot display ordering
  (admin table + `find_enabled_bots ... ORDER BY display_order`, db.rs:538).
  No data loss, admin-only trigger, fully self-repairable by pressing
  Up/Down again. Clear defect, low blast radius -> minor is the honest tier.

**Recommendation validity:** Valid. Single-statement
`UPDATE ... FROM unnest(...) WITH ORDINALITY` or a transaction both work; the
note that a uniqueness constraint needs a NEW migration is correct (and a
constraint would actually break the current per-row loop mid-flight, so the
single-statement form is the right pairing if a constraint is ever added).

---

## F19 (minor) create_bot MAX+1 race

**Verdict: CONFIRMED. Severity: minor (agree; nit also defensible).**

- admin.rs:134-137: `VALUES ($1, COALESCE((SELECT MAX(display_order) + 1 FROM
  bots), 0), ...)`. Two concurrent inserts at READ COMMITTED can both read the
  same MAX. No unique constraint (see F18) so the result is silent duplicate
  display_order, i.e. nondeterministic ordering - not an error.

**Recommendation validity:** Valid ("accept and document" is the proportionate
option for an admin-only single-operator UI).

---

## F20 (minor) Mask exposes whole key for keys <= 4 chars; fabricated sk- prefix

**Verdict: CONFIRMED. Severity: minor (agree; leans nit - real provider keys
are never <= 4 chars).**

- admin.rs:228-236 (`list_providers`) and :279-289 (`create_provider`): both
  compute last-4 via the double-reverse idiom and emit
  `format!("sk-...{last4}")`. For a plaintext of length <= 4 the "last 4" IS
  the whole key, so `api_key_masked` round-trips the entire secret; the `sk-`
  prefix is fabricated regardless of the actual key shape.
- The test at admin.rs:2188-2190 only asserts the normal-length case
  (`"sk-...1234"`), as the finding says.

**Recommendation validity:** Valid; fixing it will require updating the
assertion at admin.rs:2189 if the prefix changes.

---

## F21 (minor) No way to clear a provider API key

**Verdict: CONFIRMED. Severity: minor (agree).**

- admin.rs:309-339 `update_provider`: `Some(key_str)` -> encrypt and overwrite;
  `None` -> UPDATE omitting the column. There is no path that sets
  `api_key_encrypted = NULL`. `Some("")` would encrypt the empty string (a
  non-NULL "key"), though the UI filters empty input to `None`
  (admin.rs:1602-1605), so the empty-string case is API-only.
- The edit form's help text "Leave blank to keep existing key"
  (admin.rs:1620) confirms clearing is unrepresentable in the UI too.

**Recommendation validity:** Valid.

---

## F22 (minor) test_provider hardcodes model gpt-4o-mini

**Verdict: CONFIRMED. Severity: minor (agree).**

- admin.rs:496-501: `"model": "gpt-4o-mini"` hardcoded in the provider-level
  health check body.
- Contrast confirmed: `test_bot_provider` uses the configured `bp.model`
  (admin.rs:569-571).

**Recommendation validity:** Valid; "reuse a configured bot_providers.model
with a fallback" is the better of the two options (no UI change).

---

## F23 (minor) Upstream test response body/headers uncapped

**Verdict: CONFIRMED. Severity: minor (agree).**

- admin.rs:513-517 (`test_provider` error branch) and :605-608
  (`test_bot_provider`): `resp.text().await` with no size cap; the result is
  returned in the server fn response. :600-604 returns ALL upstream headers
  verbatim.
- Mitigating context: the shared reqwest client has a 10s total timeout
  (main.rs:32-35), bounding duration but not size; `max_tokens: 5`
  (admin.rs:500) bounds only the success-path content of `test_provider`, not
  error bodies and not `test_bot_provider` at all. Admin-only surface.

**Recommendation validity:** Valid (cap via `resp.bytes()` + truncate, since
reqwest has no built-in body limit on `text()`; a naive `take(n)` on the byte
stream is the standard fix).

---

## F24 (minor) Test result attributed to wrong row (Action input/value race)

**Verdict: ADJUSTED - the code shape matches, but the claimed mechanism is
not supported by the vendored Action source; the actual defect per source
reading is different (and possibly worse). Severity: minor (keep).**

- Code shape confirmed: admin.rs:1424-1435 and :1760-1771 both gate on
  `value().get().is_some() && !pending().get() && let Some(id) =
  input().get()`, then store `(id-from-input, result-from-value)`. Re-dispatch
  is only blocked for the SAME id (admin.rs:1472-1475, 1833-1837), so
  overlapping tests for different rows are possible. All as described.
- Checked against reactive_graph-0.2.14/src/actions/action.rs (the version in
  the cargo registry for leptos 0.8.20):
  - dispatch (:262-303): `in_flight += 1`, then `input = Some(input)` -
    input always tracks the LATEST dispatch.
  - completion: `in_flight -= 1`, `value = Some(result)` (the `is_latest`
    guard at :288 is vacuous - the `dispatched` counter is initialized at
    :209 and never incremented anywhere, so every completion writes `value`,
    last-completion-wins), then at :295-297 `if in_flight == 0 { input = None }`.
  - `pending()` (:535-538) is `in_flight > 0`.
- Consequence for the claimed scenario (A resolves after B dispatched):
  when A completes with B still in flight, `pending()` is still true -> the
  Effect guard fails. When B completes, `in_flight` hits 0 -> `input` is
  cleared to `None` in the same synchronous poll that sets `value` -> by the
  time the (asynchronously scheduled) Effect re-runs, `input().get()` is
  `None` and the guard fails again. So the specific "A's result stored under
  B's id" interleaving cannot occur per this source.
- What the source reading DOES imply: after any completion the guard
  `input().get() == Some(_)` should never hold (input is cleared before the
  Effect runs), i.e. `test_result` would never be set and the result panel
  never rendered. That contradicts the feature presumably working when
  manually tested, so either Effect scheduling observes an intermediate state
  I cannot rule in from source alone, or the panel is genuinely broken.
  UNVERIFIABLE without a runtime check - flagged for the Lead.
- Either way the underlying point stands: pairing `input()` with `value()` in
  a completion Effect is incorrect - `input()` is a latest-dispatch/cleared
  signal, not "the input that produced value()".

**Recommendation validity:** Valid and fixes BOTH readings: have the async
block return `(Uuid, Result<...>)` and key results off `value()` alone -
`value()` reliably survives completion; `input()` does not.

---

## F25 (minor) No server-side validation of bot/provider inputs

**Verdict: CONFIRMED. Severity: minor (agree).**

- admin.rs:637-666 (`admin_create_bot`), :668-701 (`admin_update_bot`),
  :760-781 (`admin_create_provider`), :846-879 (`admin_create_bot_provider`):
  each is gate-then-passthrough; no length/emptiness/range/finite checks
  anywhere in the file. Constraints exist only in HTML (`required`,
  `min="0" max="2"` at admin.rs:1265-1268, 1320-1330).
- The "Postgres REAL accepts NaN / serde boundary unverified" caveat is
  honest and stays unverified here (would need a build/run).

**Recommendation validity:** Valid; the suggested checks are cheap and match
the codebase's ServerFnError style.

---

## F26 (minor) Admin page hard-fails if one provider key undecryptable

**Verdict: CONFIRMED. Severity: minor (agree).**

- admin.rs:222-227: inside the per-row loop of `list_providers`, decrypt
  failure propagates with `?` -> the whole Vec<ProviderRow> becomes `Err`.
- admin.rs:1022-1025: `(Some(Err(e)), _) | (_, Some(Err(e))) => <p
  class="error">` - any resource error replaces the ENTIRE admin body (bots
  section included), so one corrupt row bricks the whole page.

**Recommendation validity:** Valid (per-row degrade + log). Note the mask
tests (admin.rs:2169-2191) don't cover this path, so the fix needs a new test.

---

## F27 (minor) Bot rename/delete strands in-flight games

**Verdict: CONFIRMED - and the previously-uncertain worker fallback is now
traced. Severity: suggest UPGRADE minor -> major.**

- Name coupling confirmed: migrations/013:36 `ALTER TABLE game_bots RENAME
  COLUMN difficulty TO bot_name` - games reference bots by NAME, no FK.
- Worker resolution confirmed: bot/src/config.rs:26-28
  `FROM bots WHERE name = $1 AND enabled = true` (and load_providers
  :56-61 `WHERE b.name = $1 AND b.enabled = true ...`).
- The fallback the original reviewer could not trace: bot/src/main.rs:166-189 -
  on `None` with a non-empty bots table:
  `tracing::info!(... outcome = "skipped", reason = "bot not found or
  disabled" ...); return Ok(());` - the turn is silently skipped and reported
  as SUCCESS to the consumer, so the message is acked and never retried. A
  renamed or deleted (or merely disabled) bot means every game using that
  bot_name deadlocks permanently with only an info-level log line.
- admin.rs:159-181 (`update_bot` allows rename), :200-207 (`delete_bot`) have
  no in-use check or warning; the UI confirm dialog (admin.rs:1204-1210) says
  only "Delete this bot?".
- Severity opinion: with the fallback resolved to "silent permanent game
  deadlock triggered by a routine admin rename", this is a clear user-facing
  defect -> major.

**Recommendation validity:** Valid; note "warn in the flows" alone still
leaves rename breakage - the id-reference schema change is the real fix, and
correctly flagged as needing a NEW migration.

---

## F28 (minor) Admin-gate boilerplate duplicated 15x

**Verdict: CONFIRMED. Severity: minor (agree).**

- Counted 15 `#[server(...)]` fns (lines 618, 637, 668, 703, 722, 741, 760,
  783, 808, 827, 846, 881, 916, 935, 955); each repeats the identical
  get_current_user -> is_user_admin -> "Admin access required" block.
- Precedent claim spot-checked: `require_user` exists in
  web/src/friends/server_fns.rs.

**Recommendation validity:** Valid.

---

## F29 (minor) Mutations don't verify rows_affected

**Verdict: CONFIRMED. Severity: minor (agree).**

- `update_bot` (admin.rs:168-179), `reorder_bots` (:189-194), `delete_bot`
  (:201-205), `update_provider` (:315-337), `delete_provider` (:345-349),
  `update_bot_provider` (:446-457), `delete_bot_provider` (:463-467) all
  `.execute(...).map_err(...)?` and discard the `PgQueryResult`; a
  non-existent id yields `Ok(())` and the UI shows success.

**Recommendation validity:** Valid; the update/delete distinction (deletes
stay idempotent) is the right call.

---

## F30 (nit) update_bot_provider omits updated_at = now()

**Verdict: REJECTED - the premise is wrong: `bot_providers` has NO
updated_at column at all. Recommendation is INVALID (would break the query).**

- migrations/013_bot_efficacy.sql:23-34: `bot_providers` columns are id,
  bot_id, provider_id, model, reasoning_effort, extra_body, priority, enabled,
  `created_at` - and nothing else. No later migration alters bot_providers
  (grep "bot_providers" across migrations/*.sql matches only 013:23).
- So `update_bot_provider` (admin.rs:446-457) is not "omitting" anything and
  no column "silently goes stale" - there is no column to stale.
- **The RECOMMENDATION ("Add updated_at = now() to the UPDATE") would cause a
  runtime SQL error** (`column "updated_at" of relation "bot_providers" does
  not exist`) unless paired with a NEW migration adding the column. Flagged.
- The finding's secondary claim IS accurate and worth keeping as context for
  the db.rs consistency finding: `bots` and `llm_providers` (013:10, :20) DO
  have updated_at but NO update_updated_at trigger (001:392-444 creates
  triggers only for migration-001 tables; 013 creates none), so the manual
  `updated_at = now()` in admin.rs:169/:189/:316/:329 is required and correct
  where it appears.
- If the project wants bot_providers timestamps, the correct finding would be
  "bot_providers lacks an updated_at column (schema inconsistency)" - a NEW
  migration, different fix.

---

## F31 (nit) AdminPage redirect matches on error text

**Verdict: CONFIRMED. Severity: nit (agree).**

- admin.rs:1005-1012: `if msg.contains("Admin access required") {
  navigate2("/", ...) }` against `bots.get()`'s error string. The literal is
  duplicated 15x server-side; any rewording silently breaks the redirect
  (non-admins then just see the error text - no security impact, gate is
  server-side).

**Recommendation validity:** Valid.

---

## F32 (nit) Action-value unwraps in Effects

**Verdict: CONFIRMED. Severity: nit (agree).**

- Pattern `action.value().get().is_some() && !action.pending().get()` then
  `action.value().get().unwrap()` appears at admin.rs:1089-1100, 1102-1113,
  1115-1125, 1127-1137, 1386-1397, 1399-1410, 1412-1422, 1424-1435 (variant),
  1722-1733, 1735-1746, 1748-1758, 1760-1771 (variant) - 12 occurrences, 10
  in the plain double-get-unwrap form, matching "appears 10 times".
- "Safe in practice" holds: single-threaded reactive graph, the second
  `.get()` cannot observe a different value within one Effect run.

**Recommendation validity:** Valid; `if let Some(res) = ...` also removes the
double signal read.

---

## F33 (nit) Local type BotProviderRow shadows public struct

**Verdict: CONFIRMED. Severity: nit (agree).**

- admin.rs:538-544: `type BotProviderRow = (String, Option<Vec<u8>>, String,
  Option<String>, Option<serde_json::Value>);` inside `test_bot_provider`,
  shadowing the public `struct BotProviderRow` at :53-65 (which is also the
  fn's sibling DTO). Pure readability hazard.

**Recommendation validity:** Valid.

---

## Sanity check: "Clean: ALL 15 admin server fns are gated"

**CONFIRMED.** Exactly 15 `#[server]` fns in admin.rs (AdminListBots :618,
AdminCreateBot :637, AdminUpdateBot :668, AdminReorderBots :703,
AdminDeleteBot :722, AdminListProviders :741, AdminCreateProvider :760,
AdminUpdateProvider :783, AdminDeleteProvider :808, AdminListBotProviders
:827, AdminCreateBotProvider :846, AdminUpdateBotProvider :881,
AdminDeleteBotProvider :916, AdminTestProvider :935, AdminTestBotProvider
:955). Each was individually read and contains the full
`get_current_user -> Not authenticated -> is_user_admin -> Admin access
required` gate before touching data. No ungated fn; no missed critical.

---

## Tally

- CONFIRMED: F16, F17, F18, F19, F20, F21, F22, F23, F25, F26, F27, F28,
  F29, F31, F32, F33 (16)
- ADJUSTED: F24 (mechanism unsupported by Action source; underlying
  input()/value() pairing defect real; runtime behavior of the result panel
  flagged UNVERIFIABLE)
- REJECTED: F30 (no updated_at column on bot_providers; recommendation would
  produce a SQL error)
- Severity changes proposed: F18 major -> minor; F27 minor -> major
- Invalid recommendations: F30 only
- Admin-gate recount: 15/15 gated - Clean claim stands
