# WP-37: admin.rs pass

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to execute this plan task-by-task with review checkpoints.

**Goal:** Clean up `rust/web/src/admin.rs` end to end: collapse the 15 verbatim admin-gate blocks into one `require_admin` helper (ws F28) and make the non-admin redirect key off a shared constant matched on the `ServerFnError::ServerError` variant instead of a substring of the `Display` string (ws F31); stop the API-key mask from round-tripping short keys in full and from fabricating an `sk-` prefix (ws F20); degrade `list_providers` per row instead of taking the whole admin page down on one undecryptable key (ws F26); make `reorder_bots` a single atomic statement and serialize it against `create_bot`'s `MAX(display_order)+1` read-modify-write with one advisory lock (ws F18, ws F19); surface no-op updates as "not found" instead of a fake success (ws F29); give `update_provider` a representable "clear the key" state (ws F21); add cheap server-side validation of bot/provider inputs (ws F25); resolve `test_provider`'s model from the provider's configured `bot_providers` rows instead of the hardcoded `gpt-4o-mini` (ws F22); cap the upstream test body and allowlist the returned headers (ws F23); replace the ten `value().get().is_some() && !pending() { ... unwrap() }` Effects with the repo's established `if let Some(result) = action.value().get()` shape (ws F32); key test results off the action's own return value so a result can never be attributed to a different row (ws F24); and rename the shadowing local `BotProviderRow` tuple alias (ws F33).

**Architecture — how `admin.rs` is built (read this before editing):**

`rust/web/src/admin.rs` (2336 lines) is a single-file vertical slice of the `/admin` page. It has four strata, in file order:

1. **Client-visible DTOs and `Action` type aliases** (:1-94). `BotUpdateAction` :7, `BotCreateAction` :9, `ProviderUpdateAction` :10-11, `BotProviderCreateAction` :12-22, `BotProviderUpdateAction` :23-33. Public structs `BotRow` :36-45, `ProviderRow` :48-54 (carries `api_key_masked: Option<String>`, never the plaintext), `BotProviderRow` :57-68, `TestBotProviderResponse` :71-76 (each preceded by its `#[derive(Debug, Clone, Serialize, Deserialize)]` one line earlier). `#[cfg(feature = "ssr")]` tuple aliases `BotDbRow` :79, `ProviderDbRow` :81, `BotProviderDbRow` :83-94. Each of the five `*Action` aliases is used as a component prop type exactly once (`BotCreateAction` :1263, `BotUpdateAction` :1319, `ProviderUpdateAction` :1625, `BotProviderCreateAction` :1970, `BotProviderUpdateAction` :2081) — so changing an alias (Task 8) changes both the `Action::new` site and the prop type.
2. **Plain `#[cfg(feature = "ssr")]` DB/HTTP helpers taking `&PgPool`** (:96-628): `list_bots` :97, `create_bot` :134, `update_bot` :170, `reorder_bots` :197, `delete_bot` :213, `list_providers` :223, `create_provider` :265, `update_provider` :314, `delete_provider` :357, `list_bot_providers` :367, `create_bot_provider` :410, `update_bot_provider` :450, `delete_bot_provider` :475, `test_provider` :485, `test_bot_provider` :545. **These are the layer the `#[sqlx::test]`s call directly** (`mod tests` :2176-2336), because they take a pool argument instead of pulling one out of Leptos context. All new DB-level behaviour goes here so it is testable.
3. **15 `#[server]` functions** (:631-993): `admin_list_bots` :632, `admin_create_bot` :651, `admin_update_bot` :684, `admin_reorder_bots` :721, `admin_delete_bot` :740, `admin_list_providers` :759, `admin_create_provider` :778, `admin_update_provider` :801, `admin_delete_provider` :826, `admin_list_bot_providers` :845, `admin_create_bot_provider` :864, `admin_update_bot_provider` :899, `admin_delete_bot_provider` :934, `admin_test_provider` :953, `admin_test_bot_provider` :973. Each is a thin wrapper: pull `PgPool` (and for the two test fns `reqwest::Client`) out of `expect_context`, run the admin gate, delegate to the stratum-2 helper.
4. **Leptos components** (:995-2174): `AdminPage` :996, `BotsSection` :1062, `BotCreateForm` :1263, `BotEditForm` :1311, `ProvidersSection` :1388, `ProviderCreateForm` :1580, `ProviderEditForm` :1620, `BotProvidersSection` :1673, `BotProviderCreateForm` :1967, `BotProviderEditForm` :2074. `AdminPage` owns a `version: RwSignal<u32>` refetch counter; each section takes it and bumps it after a successful mutation, which re-runs the `LocalResource`s at :1012-1021.

**Schema (all in `rust/web/migrations/013_bot_efficacy.sql`, plus `022_concede_bot_replacement.sql:16`):**

- `bots(id, name UNIQUE, display_order INTEGER NOT NULL DEFAULT 0, enabled, include_basic_strategy, include_advanced_strategy, temperature REAL, created_at, updated_at)` + `can_replace_humans boolean NOT NULL DEFAULT false` from 022. **No unique constraint on `display_order`.**
- `llm_providers(id, name UNIQUE, url, api_key_encrypted BYTEA NULL, enabled, created_at, updated_at)`.
- `bot_providers(id, bot_id FK->bots ON DELETE CASCADE, provider_id FK->llm_providers ON DELETE CASCADE, model TEXT NOT NULL, reasoning_effort TEXT, extra_body JSONB, priority INTEGER, enabled, created_at, UNIQUE (bot_id, provider_id, model))`.
- **`bot_providers` has NO `updated_at` column** — verified by reading `013_bot_efficacy.sql:23-34` (the `CREATE TABLE` ends at `UNIQUE (bot_id, provider_id, model)`, no `updated_at`) and by `grep -n "bot_providers\|updated_at" migrations/01[4-9]*.sql migrations/02*.sql`, whose only `bots`/`bot_providers` hit through migration 022 is `022:16 ALTER TABLE bots ADD COLUMN can_replace_humans`. This is why **ws F30 is rejected and must not be implemented** (see Non-Goals).
- **There is no `model` column anywhere except `bot_providers.model`** — neither `bots` nor `llm_providers` has one. Task 10's fix for ws F22 therefore resolves a model out of `bot_providers`; **no new migration is needed anywhere in this package.** The highest existing migration is `022_concede_bot_replacement.sql`; if a later task ever did need one it would be `023_*.sql` (migrations are immutable once applied).

**SQL style:** every statement in this file is a runtime-checked `sqlx::query`/`query_as`/`query_scalar` (24 call sites, `grep -c "query!\|query_as!\|query_scalar!" admin.rs` = **0**). No compile-time macro, so no `.sqlx` metadata depends on this file. `cargo sqlx prepare` is still listed in each checkpoint because CI runs `--check` crate-wide and the implementer must not introduce a `query!` macro without it.

**Tech Stack:** Rust 1.97.0 edition 2024, `leptos 0.8.20` (`reactive_graph 0.2.14`, `server_fn 0.8.13`), `sqlx 0.8` (postgres, uuid, runtime-tokio-rustls), `reqwest 0.13.4` (`default-features = false`, features `json`/`form`/`rustls`), Postgres 18. Let-chains, `let ... else` and `Option::is_some_and` are all in use in this file already.

**Global Constraints:**

- Run all commands from `/home/beefsack/Development/brdgme/rust`. **`web` is feature-gated**: `cargo test -p web --features ssr`, `cargo clippy -p web --all-targets --features ssr -- -D warnings`. NEVER a workspace-wide build/test (AGENTS.md "Resource constraints").
- Each task ends with `cargo clippy -p web --all-targets --features ssr -- -D warnings` and `cargo fmt --all -- --check` clean.
- DB-backed tests need the throwaway containers. Either run the whole gate via `/home/beefsack/Development/brdgme/scripts/rust-test.sh`, or export a `DATABASE_URL` pointing at a migrated Postgres 18 first. A bare `cargo test -p web --features ssr` with no database fails on every `#[sqlx::test]` — that is pre-existing (AGENTS.md, backlog #40), **not** a regression you introduced.
- `#[sqlx::test]` gives each test its own migrated database; never share state between tests.
- If any task introduces a `sqlx::query!`-family **macro** (it should not — match the file's existing runtime-checked style), the implementer must run `(cd /home/beefsack/Development/brdgme/rust/web && cargo sqlx prepare -- --tests --features ssr --all-targets)` and commit the `.sqlx` change, because CI runs `(cd web && cargo sqlx prepare --check -- --tests --features ssr --all-targets)`.
- **Line numbers below are LIVE numbers as of the drift check.** Task 1 removes ~75 lines spread across :631-993 and adds ~20 near the top, so **every line number in Tasks 2-13 shifts by the time you get there. Tasks 2-13 must locate their edit sites by symbol name (`grep -n "fn <name>" admin.rs`), not by the numbers printed here.** The numbers are for review and for confirming you are looking at the right code.
- `crypto::load_key()` failure stays a hard error everywhere (it is a deployment misconfiguration, not row-level corruption). Only per-row decrypt/UTF-8 failures degrade (Task 5).
- No DTO field may be *removed* or *renamed* in this package. `ProviderRow`/`BotRow`/`BotProviderRow`/`TestBotProviderResponse` cross the wire; additions are fine, removals are not.
- Run `/home/beefsack/Development/brdgme/scripts/rust-test.sh` before the **final** commit of the package.

**Non-Goals (owned elsewhere — do NOT absorb):**

- **ws F27 (renaming/deleting a bot strands in-flight games)** — owned by **WP-38 "bot-turn wedge recovery", BLOCKED-ON-DECISION D-5.** Verification upgraded it minor -> **major**: `rust/bot/src/main.rs:171-187` silently skips the turn and returns `Ok(())` (message acked, never retried) when the bot name is missing or disabled, so a rename/delete/disable permanently deadlocks every game using that bot. It shares `admin.rs` as a path with this package. **Do NOT add any warning, confirm-dialog change, referential guard or `game_bots` lookup to `delete_bot`/`update_bot` or to the bots Delete button at :1226-1233 (the `confirm_with_message("Delete this bot?")` block).** The recovery design (D-5) covers all six findings in that family at once; a partial guard here would have to be reverted.
- **ws F30 (`update_bot_provider` omits `updated_at = now()`)** — **REJECTED by verification, do not implement.** `bot_providers` has no `updated_at` column (evidence above); adding `updated_at = now()` to the `UPDATE` at :459-461 would be a runtime SQL error (`column "updated_at" of relation "bot_providers" does not exist`) on every link edit. Task 7 touches that exact statement — **add the `rows_affected` check and nothing else.**
- **`rust/web/src/db.rs`** — owned by **WP-41 (db.rs quality pass)**. In particular WP-41 Task 2 changes `db::is_user_admin` from `sqlx::Result<bool>` to `anyhow::Result<bool>`. See "Coordination / landing order" below. **Do not edit `db.rs` from this package**, and do not "fix" `is_user_admin` yourself.
- **The frontend fire-and-forget / error-slot sweep** — owned by **WP-54 (frontend UX error handling)**, whose paths are `web/src/{friends.rs,new_game.rs,settings.rs,app.rs}` and `web/src/components/{game.rs,layout.rs,opponent_slot.rs,mod.rs}`. **`admin.rs` is not in WP-54's path list, and none of WP-54's 17 findings is an `admin.rs` finding** (its scope is `wd F57/F58/F59/F63/F64/F66/F73` and `wfe F52/F54..F62`; verified against `work-packages.md:423-431`). WP-54's own spec makes this a **LEAD RULING** at `WP-54-frontend-ux-error-handling.md:210`: "Do not open `admin.rs`. Do not add it to any file list." So ws F24 and ws F32 do **not** overlap WP-54 — they are yours. Tasks 12 and 13 **copy** the pattern WP-54 canonicalises (`settings.rs:62-69` `UsernameSection`, `components/game.rs:550-562` `GameCommandInput`) and must not modify those files.
- **`rust/bot/`** — owned by **WP-61 (bot service quality)** (`bot/src/{main.rs,config.rs,crypto.rs,prompt.rs,routing.rs}`). Task 10 reads `bot_providers.model` from the web side only; do not touch the bot service's own model/provider resolution.
- **`rust/web/src/crypto.rs`** — owned by **WP-36 (crypto and deploy hardening)**. Task 4/Task 5 call `crypto::encrypt`/`decrypt`/`load_key` unchanged.
- **The full-page error render at `AdminPage` :1039-1042** (`(Some(Err(e)), _) | (_, Some(Err(e))) => <p class="error">{e.to_string()}</p>`) leaks the raw `ServerFnError` `Display` text ("error running server function: Internal server error") into the page. That is a general SPA error-presentation wart, not one of the 14 findings in scope. **Leave it.** Task 5 removes the *cause* most likely to trigger it (one bad key), which is what ws F26 asked for.
- **A unique constraint on `bots.display_order`** — ws F18's recommendation explicitly flags it as optional and as requiring a NEW migration. Task 6 achieves atomicity + serialization without one. **Do not add migration 023.**
- **`db::replacement_bot_available` / `bots.can_replace_humans`** — landed after the review snapshot (see drift below). Not in scope; do not refactor it.

**Snapshot drift:** **NOT clean.** `diff -u /home/beefsack/Development/brdgme-review-snapshot/rust/web/src/admin.rs /home/beefsack/Development/brdgme/rust/web/src/admin.rs` exits 1 with **26** hunks (re-counted 2026-07-25 with `diff -u ... | grep -c '^@@'` against snapshot commit `f8763a5`). All of it is one feature landed after the review — the `bots.can_replace_humans` column from `022_concede_bot_replacement.sql` (commits `3b7252f`/`1f665b0`, issue #47): a new `BotRow.can_replace_humans` field, new `BotDbRow`/`BotCreateAction` aliases, the column threaded through `list_bots`/`create_bot`/`update_bot`/`admin_create_bot`/`admin_update_bot`, one extra tuple element in `BotUpdateAction`/`BotCreateAction`, one `#[allow(clippy::too_many_arguments)]` on `update_bot`, and a "Can replace humans" checkbox in `BotCreateForm`/`BotEditForm`.

**None of the 14 defects was changed or fixed by that feature, but four finding sites had their surrounding text edited** and their quoted "before" blocks therefore differ from the findings docs: `create_bot`'s INSERT (ws F19 — now binds `$5` and `RETURNING ... can_replace_humans`), `update_bot`'s UPDATE (ws F29 — now sets `can_replace_humans = $7`), `list_bots`' SELECT, and `BotsSection`'s `create_action`/`update_action` closures (ws F32 territory, though the ten Effects themselves are byte-identical to the snapshot). **Take every "before" block from the live file, never from a findings doc.** All numbers in this spec are live-file numbers, re-derived by reading the live file; the findings docs are stale by up to ~40 lines.

**Disposition table (re-derived from live source; verification verdicts override findings text):**

| F# | claim | verdict | what the spec does and why |
|---|---|---|---|
| **F18** | `reorder_bots` is not atomic | **CONFIRMED** (verification adjusted severity major -> minor) | Live :197-210 is an N-statement `for` loop on the pool (loop body :201-208), one `UPDATE` per id, no transaction; migration 013 has no unique index on `display_order`. **Task 6** replaces the loop with one `UPDATE ... FROM unnest($1::uuid[]) WITH ORDINALITY` (the finding's own preferred option) inside a transaction that first takes a fixed advisory lock. Single statement = atomic; the lock is what also closes F19. |
| **F19** | `create_bot` `MAX+1` race | **CONFIRMED** | Live :143-145: `VALUES ($1, COALESCE((SELECT MAX(display_order) + 1 FROM bots), 0), $2, $3, $4, $5)`. Read-modify-write with no lock; two concurrent creates get the same `display_order`, and there is no unique constraint to catch it (verified: `013_bot_efficacy.sql:1-11` declares `display_order INTEGER NOT NULL DEFAULT 0` with no `UNIQUE`, and no migration 014-022 adds an index on it). **Task 6** wraps the insert in a transaction holding the same `pg_advisory_xact_lock` key as `reorder_bots`, so create-vs-create and create-vs-reorder both serialize. The finding's own first option ("serialize via the same transaction fix as reorder"); its second option ("accept and document") is declined because the lock is 4 lines. |
| **F20** | mask leaks short keys in full; fabricated `sk-` prefix | **CONFIRMED** | Two sites, both `format!("sk-...{last4}")`: `list_providers` :241-249 and `create_provider` :292-302. For a stored key of <= 4 chars, `last4` **is** the whole key, so `ProviderRow.api_key_masked` ships the plaintext to the browser. The `sk-` prefix is a literal, not read from the key, so a Gemini/Anthropic/Bedrock key renders with a false OpenAI prefix. **Task 4** extracts one `mask_api_key` helper with a length floor and no fabricated prefix. |
| **F21** | no way to clear a provider API key | **CONFIRMED** | `update_provider` :313-354 branches on `Option<String>`: `Some` -> encrypt+set (:323-339), `None` -> omit the column (:340-351). `api_key_encrypted` is nullable (`013_bot_efficacy.sql:17` — `api_key_encrypted BYTEA` with no `NOT NULL`), but nothing can ever write `NULL` back. `ProviderEditForm` :1639-1642 filters the empty string out (`.filter(|v| !v.is_empty())`), and its help text is literally `"Leave blank to keep existing key"` (:1657), so "keep" is the only reachable no-key path. **Task 8** replaces the `Option<String>` argument with an explicit three-state `ApiKeyUpdate { Keep, Set(String), Clear }` and adds a "Clear API key" checkbox. **Carries a stated assumption — see the labelled block in Task 8.** |
| **F22** | `test_provider` hardcodes `gpt-4o-mini` | **CONFIRMED, recommendation ADJUSTED** | Literal at :510. The finding offered "take the model as a parameter, **or** reuse one of the provider's configured `bot_providers.model` values with a fallback". Re-derived from the schema: there is no model column on `llm_providers` or `bots`, so the only real configured value lives in `bot_providers.model`. **Task 10 does both and drops the fallback**: an optional caller-supplied model wins; otherwise resolve the highest-priority enabled `bot_providers.model` for that provider; if neither exists, return an actionable user error. A silent `gpt-4o-mini` fallback is explicitly **not** kept, because keeping it preserves exactly the false negative the finding reports. |
| **F23** | upstream test body/headers uncapped | **CONFIRMED** | `test_provider` :524-531 (`resp.text()` on the error path, with the read error swallowed into `"unable to read body"`) and :533-536 (`resp.json()` on the success path — also unbounded), `test_bot_provider` :618-621 (`resp.text()`) and :613-617 (**all** response headers copied into the DTO). A hostile admin-configured endpoint streams unbounded bytes inside the 10s budget. **Task 11** adds a shared 8 KiB streaming cap via `Response::chunk` and an explicit header allowlist. |
| **F24** | test result attributable to the wrong row | **CONFIRMED as a defect, mechanism ADJUSTED** | Live shape matches at `ProvidersSection` :1461-1472 and `BotProvidersSection` :1797-1808: the Effect pairs `test_action.value().get()` with `test_action.input().get()`. Verified against `reactive_graph-0.2.14/src/actions/action.rs`: `value` is written at :291 and `input` is then cleared to `None` when `in_flight` reaches 0 (:295-297), so the finding's *cross-attribution* cannot occur as written — instead the Effect's `let Some(..) = input().get()` guard can be `None` at exactly the moment the value lands, so the result panel may simply never render. Either way the correct fix is the finding's own: **Task 13** returns `(id, result)` from the async block and keys off `value()` alone. |
| **F25** | no server-side validation | **CONFIRMED** | Confirmed by reading all four cited server fns: `admin_create_bot` :651, `admin_update_bot` :684, `admin_create_provider` :778, `admin_update_provider` :801, plus `admin_create_bot_provider` :864 / `admin_update_bot_provider` :899 / `admin_test_bot_provider` :973. Every constraint is HTML-only: `required` at :1289 (`BotCreateForm` name), :1350 (`BotEditForm` name), :1604/:1607 (`ProviderCreateForm` name/url), :1652/:1655 (`ProviderEditForm` name/url), :2031/:2041/:2051 (`BotProviderCreateForm` bot/provider/model), :2141 (`BotProviderEditForm` model); `step="0.1" min="0" max="2"` on temperature at :1292 and :1357-1358. Nothing validates `reasoning_effort`, `extra_body`, `priority` or the test `prompt` at all, client or server. **Task 9** adds cheap server-side checks in the stratum-2 helpers (so `#[sqlx::test]`s cover them), returning user-facing messages. A DB `CHECK` constraint is declined — it would need a new migration and gives a worse message. |
| **F26** | one undecryptable key kills the whole admin page | **CONFIRMED** | `list_providers` :237-240: the per-row `decrypt` and `String::from_utf8` both use `?`, so one bad row makes the whole `Vec` an `Err`; `AdminPage` :1039-1042 renders any resource `Err` as a full-page error, and the bots/links sections never render either. **Task 5** degrades that row to `api_key_masked = Some("(undecryptable)")` and logs at `error!` with the provider id. `load_key()` failure (:231) stays fatal. |
| **F27** | bot rename/delete strands games | **FENCED-to-WP-38** | Not implemented here. See Non-Goals. |
| **F28** | admin gate duplicated 15 times | **CONFIRMED** | Counted on live source: `grep -n "is_user_admin" admin.rs` returns 15 non-test hits (:640, :665, :700, :729, :748, :767, :790, :815, :834, :853, :879, :914, :942, :962, :985) plus one test-only at :2201. All 15 blocks are **byte-identical apart from the `internal("<fn>: check admin")` context string** — same `use` lines, same `expect_context`, same "Not authenticated" message, same "Admin access required" message. **Task 1** extracts `require_admin(pool, context)`; the context string is a parameter precisely so the server-side `tracing::error!` breadcrumb per call site is preserved byte-for-byte. Client-observable behaviour is therefore unchanged at all 15 sites (proof in Task 1). |
| **F29** | mutations don't verify rows affected | **CONFIRMED** | Every `.execute()` discards `PgQueryResult`: `update_bot` :190, `reorder_bots` :205, `delete_bot` :216, `update_provider` :336 (key-set branch) and :348 (no-key branch), `delete_provider` :360, `update_bot_provider` :468, `delete_bot_provider` :478. **Task 7** adds a check to the four **updates** (per-site messages listed in the task) and deliberately leaves the three **deletes** idempotent, exactly as the finding recommends. |
| **F30** | `update_bot_provider` omits `updated_at` | **SKIPPED-BY-DECISION (REJECTED by verification)** | Column does not exist. See Non-Goals. |
| **F31** | non-admin redirect matches error text | **CONFIRMED, recommendation ADJUSTED** | Live :1022-1029, with the substring test at :1025: `if msg.contains("Admin access required")`. The finding offered "a shared constant, or a structured error variant if `ServerFnError` ever carries one". Re-derived: `crate::error::internal` (`error.rs:6-12`) collapses *every* infrastructure error to the literal `"Internal server error"`, which is why a string match is the only signal available for the *authorization* case. But a structured match **is** available: `server_fn-0.8.13/src/error.rs:186` defines the `ServerFnError::ServerError(String)` variant, `ServerFnError::new` constructs exactly that variant (:201-203), and it round-trips the wire (`ser` writes `"ServerError|{e}"` :281-283, `de` reconstructs it :331-333). The enum derives `Debug, Clone, PartialEq, Eq, Serialize, Deserialize` (:165), so both the match and the new unit test's `{other:?}` compile. Critically, `Display` (:233-234) formats it as `"error running server function: {s}"` — so **exact string equality on `e.to_string()` would silently never match**. **Task 2** therefore matches the variant and compares its payload to a shared `pub const ADMIN_REQUIRED: &str`, which is both drift-proof and format-independent. |
| **F32** | `.unwrap()`s in completion Effects | **CONFIRMED** | 10 occurrences of `match action.value().get().unwrap()` guarded by `is_some() && !pending()`: :1113, :1126, :1139, :1151, :1425, :1438, :1451, :1761, :1774, :1787 (plus the two `input()`-pairing variants at :1466 and :1802, which Task 13 owns). **Task 12** rewrites all 10 to the repo's established `if let Some(result) = action.value().get()` shape, citing `settings.rs:62-68`. |
| **F33** | local `BotProviderRow` alias shadows the public struct | **CONFIRMED** | `type BotProviderRow = (String, Option<Vec<u8>>, String, Option<String>, Option<serde_json::Value>);` at :551-557, inside `test_bot_provider`, shadowing the public struct at :56-68. Its single use is the `let row: Option<BotProviderRow>` annotation at :558. **Task 3** renames it `BotProviderTestRow`. |

Counts: **11 CONFIRMED** (F18, F19, F20, F21, F23, F25, F26, F28, F29, F32, F33) + **3 CONFIRMED-with-adjusted-recommendation** (F22, F24, F31) implemented = **14 implemented**; **0 OVERTURNED**; **1 FENCED** (F27 -> WP-38); **1 SKIPPED-BY-DECISION** (F30, rejected).

**Coordination / landing order:**

1. **WP-41 (db.rs quality pass) SHOULD land before this package, but is not a hard blocker.** WP-41 Task 2 (`planning/specs/WP-41-db-quality-pass.md:395`) changes `db::is_user_admin` from `sqlx::Result<bool>` to `anyhow::Result<bool>`. `admin.rs` is the single largest caller: **16 sites today** — the 15 production gates plus one in `mod tests` — reducing to **2 after Task 1** (`require_admin` plus the test). WP-41's own spec (`WP-41-db-quality-pass.md:93` disposition row for ws F45, and the coordination row at :128) verified the change is **caller-source-compatible**, and it is compatible for both shapes present in `admin.rs`:
   - the 15 production sites (:640, :665, :700, :729, :748, :767, :790, :815, :834, :853, :879, :914, :942, :962, :985) do `.map_err(internal("..."))?`, and `internal` is generic over `E: std::fmt::Display` (`error.rs:7`), which `anyhow::Error` satisfies;
   - the **test** site `admin.rs:2201` does `crate::db::is_user_admin(&pool, user_id).await.unwrap()`, which also compiles against `anyhow::Result<bool>` because `Result::unwrap` only needs `E: Debug` and `anyhow::Error: Debug`.
   **Task 1 does not touch :2201** (it is inside `mod tests`, not a `#[server]` fn) — that is why Task 1's verification expects `grep -c "is_user_admin"` to be **2**, not 1. So neither ordering breaks the build.
   - **WP-41 first (preferred):** Task 1 writes the single `require_admin` against whatever type `is_user_admin` returns; nothing to redo.
   - **WP-37 first:** WP-41 must re-verify one call site instead of fifteen — strictly less work for them. If you land first, say so in the Task 1 commit body so WP-41's grep (`grep -rn "is_user_admin" rust/web/src`) reads clean.
   - **Either way: do not pre-emptively change `is_user_admin` yourself.**
2. **WP-38 must land after this package** (or at least after Task 1). It shares `admin.rs` and its D-5 recovery design will touch `delete_bot`/`update_bot`. Landing WP-37 first gives WP-38 a deduplicated gate and `rows_affected`-checked updates to build on. **If WP-38 somehow lands first, re-derive Tasks 6 and 7 against the new `delete_bot`/`update_bot` bodies before editing.**
3. **WP-54** does not touch `admin.rs`; no coordination needed. Tasks 12/13 only *read* `settings.rs`/`components/game.rs` as pattern references.
4. **WP-36** owns `crypto.rs`; Tasks 4/5 call it unchanged. Order-independent.
5. **Internal order is fixed: Task 1 first** (it reshapes :631-993 and shifts every later line number), then Tasks 2-13 in the printed order. Task 7 depends on Task 6's rewritten `reorder_bots`. Task 13 depends on Task 12 (same Effect blocks). Task 4 must precede Task 5 (Task 5 calls the helper Task 4 creates).

---

## Task 1: extract one `require_admin` gate (ws F28)

**Problem (restated):** all 15 `#[server]` fns repeat the same six-line block. Live example, `admin_list_bots` :632-646:

```rust
pub async fn admin_list_bots() -> Result<Vec<BotRow>, ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let is_admin = crate::db::is_user_admin(&pool, user.id)
        .await
        .map_err(internal("admin_list_bots: check admin"))?;
    if !is_admin {
        return Err(ServerFnError::new("Admin access required"));
    }
```

~75 duplicated lines, and a 16th admin fn added later can silently omit the block. Precedent for the fix already exists in this codebase: `friends::require_user()` (`friends.rs:86-91` — `#[cfg(feature = "ssr")]` at :86, `pub(crate) async fn require_user()` at :87, body :88-90, close :91).

**Fix (re-derived):** one `#[cfg(feature = "ssr")]` helper. The `context` parameter is **not** decoration: it is what keeps the server-side `tracing::error!` breadcrumb identical per call site, so the log line a prod operator sees for a DB failure inside the gate is byte-for-byte what it is today.

Return `()`, not the user. Verified by reading all 15 bodies: `user` is used **only** as the `is_user_admin` argument at every single site — no admin fn needs the `AuthUser` afterwards. Returning `()` also avoids an unused-value expression statement under `-D warnings`.

**Observable-behaviour proof (why no site changes for a client):** the three outcomes are (a) unauthenticated -> `ServerFnError::new("Not authenticated")`, (b) DB failure -> `internal(ctx)` -> `ServerFnError::new("Internal server error")` (`error.rs:7-13`, identical string at every site), (c) non-admin -> `ServerFnError::new("Admin access required")`. All three strings are the same literal at all 15 sites today; the helper reproduces all three verbatim. **The only thing that could have differed — the `internal` context string — is passed through.** So zero client-observable change at zero sites.

**Files:**
- Modify: `rust/web/src/admin.rs` (add helper after the `use` block; rewrite the gate in all 15 `#[server]` fns)

**Steps:**

- [ ] Immediately after the `Action` alias block, i.e. after the `BotProviderUpdateAction` alias closes with `>;` at :33 (:34 is blank) and before `#[derive(Debug, Clone, Serialize, Deserialize)]` at :35 / `pub struct BotRow {` at :36, insert:

```rust
/// The exact message a non-admin caller gets from every admin server fn.
/// Shared so the client-side redirect in `AdminPage` cannot drift from it
/// (ws F31); see the `ServerFnError::ServerError` match there.
pub const ADMIN_REQUIRED: &str = "Admin access required";

/// Authenticate, then require `users.is_admin`. Fail-closed.
///
/// `context` is threaded through to `internal` so each call site keeps its
/// own server-side log breadcrumb; the client-visible error is identical at
/// every site. Mirrors `friends::require_user` (ws F28).
#[cfg(feature = "ssr")]
async fn require_admin(pool: &sqlx::PgPool, context: &'static str) -> Result<(), ServerFnError> {
    let user = crate::auth::server::get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    if !crate::db::is_user_admin(pool, user.id)
        .await
        .map_err(internal(context))?
    {
        return Err(ServerFnError::new(ADMIN_REQUIRED));
    }
    Ok(())
}
```

- [ ] In each of the 15 `#[server]` fns, delete the `use crate::auth::server::get_current_user;` line, the `let user = get_current_user()...?;` statement, the `let is_admin = ...?;` statement and the `if !is_admin { return Err(...); }` block, and put `require_admin(&pool, "<the original context string>").await?;` where the gate was. Keep `use sqlx::PgPool;` and `let pool = expect_context::<PgPool>();` (and, in the two test fns, `let http_client = expect_context::<reqwest::Client>();`). The 15 fns and their context strings, in file order:

| fn | line | context string to pass |
|---|---|---|
| `admin_list_bots` | :632 | `"admin_list_bots: check admin"` |
| `admin_create_bot` | :651 | `"admin_create_bot: check admin"` |
| `admin_update_bot` | :684 | `"admin_update_bot: check admin"` |
| `admin_reorder_bots` | :721 | `"admin_reorder_bots: check admin"` |
| `admin_delete_bot` | :740 | `"admin_delete_bot: check admin"` |
| `admin_list_providers` | :759 | `"admin_list_providers: check admin"` |
| `admin_create_provider` | :778 | `"admin_create_provider: check admin"` |
| `admin_update_provider` | :801 | `"admin_update_provider: check admin"` |
| `admin_delete_provider` | :826 | `"admin_delete_provider: check admin"` |
| `admin_list_bot_providers` | :845 | `"admin_list_bot_providers: check admin"` |
| `admin_create_bot_provider` | :864 | `"admin_create_bot_provider: check admin"` |
| `admin_update_bot_provider` | :899 | `"admin_update_bot_provider: check admin"` |
| `admin_delete_bot_provider` | :934 | `"admin_delete_bot_provider: check admin"` |
| `admin_test_provider` | :953 | `"admin_test_provider: check admin"` |
| `admin_test_bot_provider` | :973 | `"admin_test_bot_provider: check admin"` |

  Worked example — `admin_list_bots` becomes exactly:

```rust
#[server(AdminListBots, "/api")]
pub async fn admin_list_bots() -> Result<Vec<BotRow>, ServerFnError> {
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    require_admin(&pool, "admin_list_bots: check admin").await?;

    list_bots(&pool).await
}
```

- [ ] Verify none was missed, running each `grep` from `/home/beefsack/Development/brdgme/rust/web/src`:
  - `grep -c "is_user_admin" admin.rs` -> **2**: one inside `require_admin`, one in the pre-existing test at :2201 (which Task 1 does **not** touch — see Coordination note 1).
  - `grep -c "Admin access required" admin.rs` -> **1**: only the `ADMIN_REQUIRED` const initialiser. (Before this task it is 16: 15 `ServerFnError::new(...)` sites at :644, :669, :704, :733, :752, :771, :794, :819, :838, :857, :883, :918, :946, :966, :989, plus the `msg.contains(...)` at :1025 — and :1025 is Task 2's job, so after Task 1 alone this count is **2**. Assert **1** only after Task 2.)
  - `grep -c "require_admin(&pool," admin.rs` -> **15**.
- [ ] Add a regression test that the gate is present on every admin server fn. Append inside `mod tests` (:2176-2336), which already has `use super::*;`.

  **The needles are assembled with `concat!` on purpose.** The test reads its own source file, so a needle written as one literal would *also* match itself and inflate the count it is checking. `concat!("#[", "server(Admin")` produces the right needle at compile time while never appearing contiguously in the file — and it is also what keeps the `grep -c "require_admin(&pool," admin.rs` check above at exactly 15. Do **not** collapse the `concat!`s, and do **not** write either pattern verbatim in a comment inside this file.

```rust
    /// ws F28: the gate now lives in one place, so the thing worth pinning is
    /// that no admin server fn skipped it. Source-level check: every admin
    /// server fn body must contain a call to the shared gate helper.
    ///
    /// The two needles are built with `concat!` so they do not match this
    /// test's own source - see the spec note above.
    #[test]
    fn every_admin_server_fn_calls_require_admin() {
        let src = include_str!("admin.rs");
        let server_fn_needle = concat!("#[", "server(Admin");
        let gate_needle = concat!("require_admin", "(&pool,");
        let server_fns = src.matches(server_fn_needle).count();
        let gates = src.matches(gate_needle).count();
        assert_eq!(
            server_fns, 15,
            "expected 15 admin server fns, found {server_fns} - update this test \
             deliberately if an admin server fn was added or removed"
        );
        assert_eq!(
            server_fns, gates,
            "{server_fns} admin server fns but {gates} gates - an admin server \
             fn is missing its authorization check"
        );
    }
```

  Why `include_str!` resolves correctly: `include_str!` paths are relative to the file containing the macro, so from `src/admin.rs` the argument `"admin.rs"` is `src/admin.rs` itself.

**Test plan:**

| case | expected |
|---|---|
| `every_admin_server_fn_calls_require_admin` | passes; 15 == 15. Delete one gate locally to sanity-check it fails, then restore. |
| existing `test_admin_list_bots_rejects_non_admin` (:2184-2203) | still passes unmodified (it calls `crate::db::is_user_admin` directly at :2201, not a server fn) |
| `cargo clippy -p web --all-targets --features ssr -- -D warnings` | clean; no `unused_imports` from the removed `get_current_user` uses |

Command: `cargo test -p web --features ssr admin` from `/home/beefsack/Development/brdgme/rust`.

**Verification checkpoint:**
- [ ] `cargo test -p web --features ssr admin` — all admin tests pass (needs a `DATABASE_URL`; see Global Constraints).
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.
- [ ] The three `grep -c` counts above are **2 / 2 / 15** after this task (the `"Admin access required"` count drops to 1 only after Task 2 rewrites :1025).

**Commit:** `refactor(admin): extract require_admin, dedup 15 admin gates (ws F28)`

---

## Task 2: match the non-admin redirect on the error variant + shared constant (ws F31)

**Problem (restated):** `AdminPage` :1022-1029 decides whether to bounce a non-admin to `/` by substring-matching the rendered error text (byte-accurate live quote, indentation included):

```rust
    Effect::new(move |_| {
        if let Some(Err(e)) = bots.get() {
            let msg = e.to_string();
            if msg.contains("Admin access required") {
                navigate2("/", NavigateOptions::default());
            }
        }
    });
```

Reword the server message and non-admins silently stop redirecting — they get a raw error page instead.

**Fix (re-derived):** match the structured variant and compare its payload to the `ADMIN_REQUIRED` constant added in Task 1. Verified from `server_fn-0.8.13/src/error.rs`: `ServerFnError::new` produces `ServerFnError::ServerError(String)` (:201-203); the variant survives the wire (`ser` :281-283 / `de` :331-333). **Do not switch to `e.to_string() == ADMIN_REQUIRED`** — `Display` (:233-234) prefixes `"error running server function: "`, so equality on the rendered string can never match.

`ServerFnError` is in scope in `admin.rs` via `use leptos::prelude::*;` (:3) and defaults its type parameter to `NoCustomError` (`server_fn-0.8.13/src/error.rs:170`), so `ServerFnError::ServerError(msg)` is a valid pattern with no turbofish. The `if let` + `&&` let-chain shape below is already used elsewhere in this file (e.g. :590-591, :1798-1800), so it compiles on this edition.

**Files:**
- Modify: `rust/web/src/admin.rs` (`AdminPage`, the Effect after the `providers` `LocalResource`)

**Steps:**

- [ ] Locate `fn AdminPage` (`grep -n "fn AdminPage" admin.rs`) and replace that Effect with:

```rust
    // ws F31: match the structured error variant against the shared
    // ADMIN_REQUIRED constant. `Display` for ServerFnError prefixes
    // "error running server function: ", so a string comparison on
    // `to_string()` would be both fragile and (for equality) always false.
    Effect::new(move |_| {
        if let Some(Err(ServerFnError::ServerError(msg))) = bots.get()
            && msg == ADMIN_REQUIRED
        {
            navigate2("/", NavigateOptions::default());
        }
    });
```

- [ ] `grep -n '"Admin access required"' admin.rs` must now return **exactly one** hit, and that hit must be the `ADMIN_REQUIRED` const initialiser added in Task 1. Any second hit means a call site or the redirect still hardcodes the literal.

**Test plan:**

> **What is and is not testable here.** The redirect is a *client* `Effect` on a `LocalResource`, and `LocalResource` does not load during SSR — so a server-rendered `/admin` emits the `<Suspense fallback>` (`"Loading..."`, :1035) and never the bots/providers/links sections, **for an admin and a non-admin alike**. An SSR assertion of the form "the non-admin body does not contain `Add Provider`" is therefore **vacuous**: it passes today, it passes after the change, and it would pass with the redirect deleted. The SSR test below is kept only as a **panic/500 smoke test** on the `/admin` route, and is labelled as such so nobody mistakes it for coverage of the redirect. The behavioural pin is the unit test on the error shape; the redirect itself is a manual check.

| case | expected |
|---|---|
| `ADMIN_REQUIRED` payload round-trip | new unit test below — this is the actual pin for the change |
| logged-in non-admin GETs `/admin` | 200 with no SSR panic. **Does not** and **cannot** demonstrate the redirect. |
| manual (needs a dev stack) | log in as a non-admin, navigate to `/admin` in the SPA: the browser lands on `/`. Then reword the server-side message *without* touching `ADMIN_REQUIRED` — it must still redirect, because nothing matches on text any more. |

- [ ] Add to `mod tests`:

```rust
    /// ws F31: pin the exact shape the client redirect matches on. If this
    /// breaks, `AdminPage`'s redirect Effect breaks with it.
    #[test]
    fn admin_required_error_is_a_server_error_variant_with_the_constant() {
        let err = ServerFnError::new(ADMIN_REQUIRED);
        match err {
            ServerFnError::ServerError(msg) => assert_eq!(msg, ADMIN_REQUIRED),
            other => panic!("expected ServerError variant, got {other:?}"),
        }
    }
```

- [ ] Add an SSR **smoke** test for the `/admin` route. In `/home/beefsack/Development/brdgme/rust/web/tests/ssr_pages.rs`, after the admin-export block (which runs :1203-1300), using the file's existing helpers exactly as its neighbours do. Verified live signatures: `async fn make_user(pool: &PgPool, name: &str) -> User` (:57), `async fn login_cookie(pool: &PgPool, user: &User, email: &str) -> String` (:72), `async fn make_state(pool: PgPool) -> AppState` (:35), `async fn get(app: Router, path: &str, cookie: Option<&str>) -> (StatusCode, String, String)` (:184) returning `(status, content_type, body)`, and `build_router` is **async** — `build_router(make_state(pool).await).await` (see :1184, :1205). Use the `get` helper rather than hand-rolling `Request::builder()`/`oneshot`:

```rust
/// ws F31 smoke test only. `AdminPage`'s data comes from `LocalResource`s,
/// which do not load during SSR, so this asserts the route renders without
/// panicking or 500ing - it does NOT and cannot assert the client-side
/// non-admin redirect. The redirect's shape is pinned by
/// `admin_required_error_is_a_server_error_variant_with_the_constant` in
/// `src/admin.rs`'s test module.
#[sqlx::test]
async fn admin_page_ssr_renders_for_non_admin_without_panicking(pool: PgPool) {
    let user = make_user(&pool, "plainuser").await;
    let cookie = login_cookie(&pool, &user, "plainuser@example.com").await;
    let app = build_router(make_state(pool).await).await;
    let (status, _content_type, body) = get(app, "/admin", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        !body.contains("panicked"),
        "SSR panic leaked into the body: {body}"
    );
}
```

  **This test needs the container stack, not just Postgres:** `make_state` calls `async_nats::connect(...).expect("nats connect")` (`ssr_pages.rs:35-44`), so it requires a reachable NATS in addition to `DATABASE_URL`. Run it through `scripts/rust-test.sh`, not a bare `cargo test`.

  The route exists: `app.rs:224` — `<Route path=StaticSegment("admin") view=crate::admin::AdminPage/>`.

Command: `cargo test -p web --features ssr admin_required_error_is_a_server_error_variant` then `cargo test -p web --features ssr admin_page_ssr_renders_for_non_admin`.

**Verification checkpoint:**
- [ ] Both new tests pass.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.

**Commit:** `fix(admin): redirect non-admins on the error variant, not message text (ws F31)`

---

## Task 3: rename the shadowing local tuple alias (ws F33)

**Problem (restated):** `test_bot_provider` declares `type BotProviderRow = (String, Option<Vec<u8>>, String, Option<String>, Option<serde_json::Value>);` at :551-557, shadowing the file's public `BotProviderRow` struct (:56-68). Anyone reading or refactoring inside that fn sees a name that means something else 500 lines up.

**Fix:** rename to `BotProviderTestRow` (the finding's own suggestion), matching the existing `*DbRow` alias naming (`BotDbRow` :79, `ProviderDbRow` :81, `BotProviderDbRow` :83-94). The alias has exactly **one** use — the `let row: Option<BotProviderRow> = sqlx::query_as(` annotation at :558 — so this is a two-line rename.

**Files:**
- Modify: `rust/web/src/admin.rs` (`test_bot_provider`)

**Steps:**

- [ ] In `test_bot_provider` (`grep -n "fn test_bot_provider" admin.rs`), rename the local alias declaration and its single use on the `let row: Option<...>` line:

```rust
    type BotProviderTestRow = (
        String,
        Option<Vec<u8>>,
        String,
        Option<String>,
        Option<serde_json::Value>,
    );
    let row: Option<BotProviderTestRow> = sqlx::query_as(
```

- [ ] `grep -n "type BotProviderRow" admin.rs` must return nothing.

**Test plan:** compile-only change with no behavioural surface; the type-checker is the test.

**Verification checkpoint:**
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.

**Commit:** `refactor(admin): rename local BotProviderRow alias to BotProviderTestRow (ws F33)`

---

## Task 4: one honest API-key mask (ws F20)

**Problem (restated):** the mask is duplicated and wrong in two ways. `list_providers` :241-249 (the live block is 9 lines; the quote below is compressed for readability — locate it by the `format!("sk-...{last4}")` line, not by matching this text byte-for-byte):

```rust
                let last4: String = plaintext
                    .chars().rev().take(4).collect::<String>()
                    .chars().rev().collect();
                Some(format!("sk-...{last4}"))
```

and `create_provider` :292-302 does the same on the plaintext it was handed. Two defects: (1) for a stored key of <= 4 characters, `last4` **is** the entire key, so `ProviderRow.api_key_masked` — a DTO that goes to the browser — carries the full secret; (2) `sk-` is a hardcoded literal, so a non-OpenAI key is displayed with a prefix it does not have.

**Fix (re-derived masking rule):** one helper. Reveal the last 4 characters **only when at least 8 characters remain hidden-worthy**, i.e. only for keys of >= 8 characters, so at most half the key is ever shown. Anything shorter renders the fixed marker `(set)`, which still tells the admin a key exists — the information the column is actually for. No prefix is fabricated; the ellipsis carries the "there is more" meaning.

Enumerated rule (these are the test cases):

| key length (chars) | output |
|---|---|
| 0 (empty string stored) | `(set)` |
| 1 | `(set)` |
| 4 | `(set)` |
| 5 | `(set)` |
| 7 | `(set)` |
| 8 | `...` + last 4 |
| long (23, the existing test key) | `...1234` |

Count in **chars**, not bytes, and take the last 4 **chars** — `plaintext` is arbitrary UTF-8 after `String::from_utf8`, and a byte slice could split a multi-byte character and panic.

**Files:**
- Modify: `rust/web/src/admin.rs` (add helper; call it from `list_providers` and `create_provider`; update one existing test assertion)

**Steps:**

- [ ] Add the helper immediately before `pub async fn list_providers` (`grep -n "fn list_providers" admin.rs`):

```rust
/// Minimum key length before any characters are revealed. At 8, the mask
/// shows at most half the key; below it, nothing is shown at all (ws F20:
/// the old `sk-...{last4}` mask round-tripped keys of <= 4 chars in full).
#[cfg(feature = "ssr")]
const API_KEY_MASK_MIN_LEN: usize = 8;

/// Render a stored API key for display. Never fabricates a vendor prefix
/// (the old mask hardcoded `sk-`, which is wrong for every non-OpenAI
/// provider) and never reveals anything for a key short enough that the
/// "last 4" would be most of it (ws F20).
#[cfg(feature = "ssr")]
fn mask_api_key(plaintext: &str) -> String {
    let len = plaintext.chars().count();
    if len < API_KEY_MASK_MIN_LEN {
        return "(set)".to_string();
    }
    let last4: String = plaintext.chars().skip(len - 4).collect();
    format!("...{last4}")
}
```

- [ ] In `list_providers`, replace the `let last4: String = plaintext.chars().rev().take(4)...` chain and the `Some(format!("sk-...{last4}"))` that follows it (:241-249) with:

```rust
                Some(mask_api_key(&plaintext))
```

- [ ] In `create_provider`, replace the whole `let api_key_masked = api_key.map(|k| { ... });` closure (:292-302) with:

```rust
    let api_key_masked = api_key.as_deref().map(mask_api_key);
```

  Note `api_key` is `Option<String>` and was being consumed by `map`; `as_deref()` keeps it borrowed. Verified against the live body: `api_key` is read at :271 (`match &api_key`) and at :292 (this `map`), and **not** after — the fn ends at :311 with the `Ok(ProviderRow { ... })` construction, which uses the `api_key_masked` binding, not `api_key`. So `as_deref()` is safe and the borrow-vs-move distinction is cosmetic here; keep `as_deref()` anyway so a later edit cannot accidentally move it.
- [ ] Update the existing assertion in `test_admin_list_providers_never_returns_full_key` (fn at :2205-2228, assertion at :2226): `assert_eq!(masked, "sk-...1234");` becomes `assert_eq!(masked, "...1234");`. **This test change is mandatory and expected** — it was asserting the fabricated prefix. The line after it, `assert!(!masked.contains(api_key));` (:2227), stays.
- [ ] `grep -n '"sk-\.\.\.' admin.rs` must return nothing.

**Test plan:**

- [ ] Add a pure unit test (no DB) to `mod tests`:

```rust
    /// ws F20: the mask must never return a short key verbatim, and must not
    /// invent a vendor prefix.
    #[test]
    fn mask_api_key_rules() {
        assert_eq!(mask_api_key(""), "(set)");
        assert_eq!(mask_api_key("k"), "(set)");
        assert_eq!(mask_api_key("abcd"), "(set)");
        assert_eq!(mask_api_key("abcde"), "(set)");
        assert_eq!(mask_api_key("abcdefg"), "(set)");
        assert_eq!(mask_api_key("abcdefgh"), "...efgh");
        assert_eq!(mask_api_key("sk-test-secret-key-1234"), "...1234");
        // No fabricated prefix for a non-OpenAI key.
        assert_eq!(mask_api_key("AIzaSyAveryLongGoogleKey9876"), "...9876");
        // Multi-byte safe: last 4 chars, not last 4 bytes.
        assert_eq!(mask_api_key("aaaaaaaaéèçà"), "...éèçà");
        // Nothing short is ever echoed back.
        for k in ["", "k", "ab", "abc", "abcd", "abcde", "ab cdef"] {
            assert_eq!(mask_api_key(k), "(set)", "leaked short key {k:?}");
        }
    }
```

| case | expected |
|---|---|
| `mask_api_key_rules` | passes (7 length cases + 2 prefix cases + UTF-8 case + no-echo loop) |
| `test_admin_list_providers_never_returns_full_key` (:2205-2228, assertion at :2226 edited) | passes with `"...1234"` |
| `test_admin_create_provider_encrypts_key` (:2230-2254) | passes unmodified (it asserts on the ciphertext in the DB, never on the mask) |

Command: `cargo test -p web --features ssr mask_api_key_rules` then `cargo test -p web --features ssr admin`.

**Verification checkpoint:**
- [ ] `cargo test -p web --features ssr admin` passes (including the edited assertion).
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.

**Commit:** `fix(admin): mask short API keys and drop the fabricated sk- prefix (ws F20)`

---

## Task 5: degrade `list_providers` per row on decrypt failure (ws F26)

**Problem (restated):** `list_providers` :237-240 propagates a per-row failure with `?` (byte-accurate live quote):

```rust
                let decrypted = crate::crypto::decrypt(&key, &encrypted)
                    .map_err(internal("admin_list_providers: decrypt"))?;
                let plaintext =
                    String::from_utf8(decrypted).map_err(internal("admin_list_providers: utf8"))?;
```

One corrupt `api_key_encrypted` row (a key rotation that missed a row, a truncated write) makes the whole `Vec` an `Err`. `AdminPage` :1039-1042 then renders the entire page as `<p class="error">`, so the bots section and the links section vanish too — and the only recovery path is direct DB access, because the admin UI that could fix the row is the thing that is down.

**Fix (re-derived):** the fallible step is exactly `crypto::decrypt` + `String::from_utf8` per row. `crypto::load_key()` at :231 is *not* per-row — it is a deployment-level failure and stays fatal (degrading it would render every provider as "(undecryptable)" and hide a misconfiguration). Replace the two `?`s with an `Ok`/`Err` match producing `"(undecryptable)"` and an `error!` log carrying the provider id so the operator can find the row.

**Files:**
- Modify: `rust/web/src/admin.rs` (`list_providers`)

**Steps:**

- [ ] In `list_providers` (`grep -n "fn list_providers" admin.rs`), replace the whole `let api_key_masked = match api_key_encrypted { ... };` block (live :235-252, i.e. everything from `let api_key_masked = match` down to the `};` before `providers.push(ProviderRow {`) with:

```rust
        // ws F26: a single unreadable row must not take down the whole admin
        // page. Degrade that provider's key column and log the id so the row
        // can be re-keyed through this same UI. `load_key` failure above stays
        // fatal - that is a deployment problem, not row corruption.
        let api_key_masked = match api_key_encrypted {
            Some(encrypted) => Some(
                match crate::crypto::decrypt(&key, &encrypted)
                    .map_err(|e| e.to_string())
                    .and_then(|d| String::from_utf8(d).map_err(|e| e.to_string()))
                {
                    Ok(plaintext) => mask_api_key(&plaintext),
                    Err(e) => {
                        tracing::error!(
                            "admin_list_providers: provider {id} api_key_encrypted is unreadable: {e}"
                        );
                        "(undecryptable)".to_string()
                    }
                },
            ),
            None => None,
        };
```

  `id` is already in scope — the loop header at :234 is `for (id, name, url, api_key_encrypted, enabled) in rows {`.
- [ ] No open question about the error type: `crypto::decrypt` returns `Result<Vec<u8>, CryptoError>` and `CryptoError` is `#[derive(Debug, Error)]` with `#[error("decryption failed")]` (`crypto.rs:5-15`, `decrypt` at :30-40), so `thiserror` gives it `Display` and `{e}` / `.to_string()` both work. `String::from_utf8`'s `FromUtf8Error` is also `Display`. Use `{e}`; do **not** substitute `{e:?}`.

**Test plan:**

- [ ] Add to `mod tests`:

```rust
    /// ws F26: one corrupt row must degrade to a marker, and every other
    /// provider must still list. Before the fix this returned Err and the
    /// whole admin page rendered as a single error.
    #[sqlx::test]
    async fn test_admin_list_providers_degrades_one_undecryptable_row(pool: sqlx::PgPool) {
        let key = test_encryption_key();
        let good = crate::crypto::encrypt(&key, b"sk-good-key-1234").unwrap();

        sqlx::query(
            "INSERT INTO llm_providers (id, name, url, api_key_encrypted, enabled) VALUES ($1, $2, $3, $4, true)",
        )
        .bind(Uuid::new_v4())
        .bind("aaa-good")
        .bind("http://localhost:8080")
        .bind(&good)
        .execute(&pool)
        .await
        .unwrap();

        // Not ciphertext at all: 4 bytes, and `crypto::decrypt` returns
        // DecryptionFailed for anything under 12 (crypto.rs:31-33), so this
        // row fails independently of which key is loaded.
        sqlx::query(
            "INSERT INTO llm_providers (id, name, url, api_key_encrypted, enabled) VALUES ($1, $2, $3, $4, true)",
        )
        .bind(Uuid::new_v4())
        .bind("bbb-corrupt")
        .bind("http://localhost:8081")
        .bind(vec![0u8, 1, 2, 3])
        .execute(&pool)
        .await
        .unwrap();

        // No key at all: still None, not a marker.
        sqlx::query(
            "INSERT INTO llm_providers (id, name, url, enabled) VALUES ($1, $2, $3, true)",
        )
        .bind(Uuid::new_v4())
        .bind("ccc-nokey")
        .bind("http://localhost:8082")
        .execute(&pool)
        .await
        .unwrap();

        // Ordered by name, so aaa/bbb/ccc.
        let providers = list_providers(&pool).await.expect("must not fail wholesale");
        assert_eq!(providers.len(), 3);
        assert_eq!(providers[0].api_key_masked.as_deref(), Some("...1234"));
        assert_eq!(
            providers[1].api_key_masked.as_deref(),
            Some("(undecryptable)")
        );
        assert_eq!(providers[2].api_key_masked, None);
    }
```

| case | expected |
|---|---|
| good key + corrupt key + no key | `Ok` with 3 rows: `...1234`, `(undecryptable)`, `None` |
| all-corrupt table | still `Ok`; every row `(undecryptable)` (implied by the above; no separate test) |
| `load_key()` failure | still `Err`, unchanged and **not tested**. Note the reason: `crypto::load_key` (`crypto.rs:53-63`) *cannot* fail when `DATABASE_ENCRYPTION_KEY` is unset — it returns `default_key()`. It only errors on a **set but malformed** value (`InvalidHex` / `InvalidKeyLength`), which a `#[sqlx::test]` cannot induce without mutating process-global env. Leave it untested; do not add a `serial_test` env-mutating test for it. |
| `llm_providers` is empty | `Ok(vec![])`; migration 013 seeds `bots` but **not** `llm_providers`, which is why the test's `assert_eq!(providers.len(), 3)` is exact rather than a lower bound |

Command: `cargo test -p web --features ssr test_admin_list_providers_degrades`.

**Verification checkpoint:**
- [ ] New test passes; the four pre-existing provider tests still pass.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.

**Commit:** `fix(admin): degrade undecryptable provider keys per row (ws F26)`

---

## Task 6: make bot ordering atomic and race-free (ws F18, ws F19)

**Problem (restated):** `reorder_bots` :197-210 issues one `UPDATE` per id on the pool with no transaction (loop at :201-208):

```rust
    for (i, id) in ordered_ids.iter().enumerate() {
        sqlx::query("UPDATE bots SET display_order = $2, updated_at = now() WHERE id = $1")
```

A failure at element k leaves the list half-renumbered, and the UI immediately refetches and renders the mixture. Separately, `create_bot` :143-145 computes `COALESCE((SELECT MAX(display_order) + 1 FROM bots), 0)` as a read-modify-write, so two concurrent creates — or a create interleaved with a reorder — produce duplicate `display_order` values, which nothing in migration 013 forbids and which make `ORDER BY display_order` (`list_bots` :99) nondeterministic.

**Fix (re-derived):** two parts, one mechanism.

1. `reorder_bots` becomes a **single** `UPDATE ... FROM unnest($1::uuid[]) WITH ORDINALITY` statement — the finding's own preferred option. One statement is atomic by definition, so a partial renumber is impossible.
2. Both `reorder_bots` and `create_bot` run inside a transaction that first takes the **same** `pg_advisory_xact_lock` key. That serializes create-vs-create (closing F19) and create-vs-reorder, and the lock is released automatically at commit or rollback. No unique constraint, no migration.

`ordinality` is 1-based; the existing code writes 0-based `i as i32`, so the SQL subtracts 1 to preserve the stored values exactly (relevant because `list_bots` sorts on them and `create_bot` reads `MAX`).

**Files:**
- Modify: `rust/web/src/admin.rs` (`create_bot`, `reorder_bots`)

**Steps:**

- [ ] Add the lock-key constant immediately before `pub async fn create_bot` (`grep -n "fn create_bot" admin.rs`):

```rust
/// Advisory-lock key serializing every writer of `bots.display_order`.
/// `create_bot` reads `MAX(display_order)+1` and `reorder_bots` renumbers the
/// whole list; without this they can produce duplicate orders, and there is no
/// unique constraint on the column (migration 013). Transaction-scoped, so it
/// is released on commit or rollback (ws F18, ws F19).
#[cfg(feature = "ssr")]
const BOT_DISPLAY_ORDER_LOCK: i64 = 130_100_113;
```

- [ ] In `create_bot`, wrap the existing insert in a locked transaction. Replace the body from `let row: BotDbRow = sqlx::query_as(` (:142) through the `.map_err(internal("admin_create_bot: insert"))?;` line (:154) with the following. **Note the executor change from `.fetch_one(pool)` to `.fetch_one(&mut *tx)`** — this is the one substantive difference besides the surrounding transaction:

```rust
    let mut tx = pool
        .begin()
        .await
        .map_err(internal("admin_create_bot: begin"))?;

    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(BOT_DISPLAY_ORDER_LOCK)
        .execute(&mut *tx)
        .await
        .map_err(internal("admin_create_bot: lock"))?;

    let row: BotDbRow = sqlx::query_as(
        "INSERT INTO bots (name, display_order, temperature, include_basic_strategy, include_advanced_strategy, can_replace_humans) \
         VALUES ($1, COALESCE((SELECT MAX(display_order) + 1 FROM bots), 0), $2, $3, $4, $5) \
         RETURNING id, name, display_order, enabled, include_basic_strategy, include_advanced_strategy, temperature, can_replace_humans",
    )
    .bind(&name)
    .bind(temperature)
    .bind(include_basic_strategy)
    .bind(include_advanced_strategy)
    .bind(can_replace_humans)
    .fetch_one(&mut *tx)
    .await
    .map_err(internal("admin_create_bot: insert"))?;

    tx.commit()
        .await
        .map_err(internal("admin_create_bot: commit"))?;
```

  The SQL text is unchanged; only the executor (`&mut *tx` instead of `pool`) and the surrounding transaction are new. The `Ok(BotRow { ... can_replace_humans: row.7 })` construction at :156-165 stays exactly as it is — the `row` binding is still live after `tx.commit()` because `BotDbRow` is an owned tuple.
- [ ] Replace `reorder_bots` in full with:

```rust
#[cfg(feature = "ssr")]
pub async fn reorder_bots(
    pool: &sqlx::PgPool,
    ordered_ids: Vec<Uuid>,
) -> Result<(), ServerFnError> {
    // A duplicated id would match one `bots` row from two ordinals; Postgres
    // applies exactly one of them and does not say which, so the resulting
    // order is nondeterministic. Reject before doing any work. (The old loop
    // was deterministic here only by accident - last write won.)
    let distinct: std::collections::HashSet<&Uuid> = ordered_ids.iter().collect();
    if distinct.len() != ordered_ids.len() {
        return Err(ServerFnError::new(
            "Bot list contains a duplicate entry, please reload and try again",
        ));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(internal("admin_reorder_bots: begin"))?;

    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(BOT_DISPLAY_ORDER_LOCK)
        .execute(&mut *tx)
        .await
        .map_err(internal("admin_reorder_bots: lock"))?;

    // ws F18: one statement, so a partial renumber is impossible. WITH
    // ORDINALITY is 1-based; the stored order stays 0-based.
    let result = sqlx::query(
        "UPDATE bots SET display_order = o.ord - 1, updated_at = now() \
         FROM unnest($1::uuid[]) WITH ORDINALITY AS o(id, ord) \
         WHERE bots.id = o.id",
    )
    .bind(&ordered_ids)
    .execute(&mut *tx)
    .await
    .map_err(internal("admin_reorder_bots: update"))?;

    // ws F29: an id that no longer exists means the admin is acting on a
    // stale list; reject instead of reporting success for a partial reorder.
    // `distinct.len() == ordered_ids.len()` was proven above, so comparing
    // against either is equivalent - use `distinct` to keep the intent local.
    if result.rows_affected() as usize != distinct.len() {
        tx.rollback()
            .await
            .map_err(internal("admin_reorder_bots: rollback"))?;
        return Err(ServerFnError::new(
            "Bot list has changed, please reload and try again",
        ));
    }

    tx.commit()
        .await
        .map_err(internal("admin_reorder_bots: commit"))?;
    Ok(())
}
```

  Notes on the SQL and the bindings, so nothing here needs guessing:
  - This task lands the `rows_affected` check for `reorder_bots` because the check has to live inside the transaction it rolls back; Task 7 covers the other three updates.
  - `.bind(&ordered_ids)` binds `&Vec<Uuid>` against `$1::uuid[]`. This is the same by-reference bind shape the file already uses for `&String`/`&Vec<u8>` (e.g. :147, :287), and sqlx's postgres `uuid` feature is enabled (`web/Cargo.toml:28`), so `Uuid` arrays encode. If the borrow form ever fails to resolve, use `.bind(&ordered_ids[..])` — do **not** move `ordered_ids`, because the `distinct` check reads it afterwards.
  - `o.ord` from `WITH ORDINALITY` is `bigint`; `bots.display_order` is `INTEGER`. Postgres has an assignment cast bigint -> int4, so `SET display_order = o.ord - 1` is valid without an explicit cast. Add `::int` only if Postgres complains.
  - `reorder_bots(vec![])` is `Ok(())` and a no-op: `unnest` of an empty array yields no rows, so `rows_affected() == 0 == distinct.len()`.
  - The advisory lock is taken **before** the `UPDATE` so a concurrent `create_bot` cannot slip a `MAX+1` read between the renumber and the commit.

**Test plan:**

- [ ] Add to `mod tests`:

```rust
    async fn insert_bot(pool: &sqlx::PgPool, name: &str) -> Uuid {
        create_bot(pool, name.to_string(), 0.2, true, false, false)
            .await
            .unwrap()
            .id
    }

    /// ws F18: reorder writes 0-based orders matching the given sequence.
    #[sqlx::test]
    async fn test_reorder_bots_renumbers_zero_based(pool: sqlx::PgPool) {
        // migration 013 seeds easy/medium/hard at 0/1/2; clear for determinism.
        sqlx::query("DELETE FROM bots").execute(&pool).await.unwrap();
        let a = insert_bot(&pool, "aaa").await;
        let b = insert_bot(&pool, "bbb").await;
        let c = insert_bot(&pool, "ccc").await;

        reorder_bots(&pool, vec![c, a, b]).await.unwrap();

        let names: Vec<String> =
            sqlx::query_scalar("SELECT name FROM bots ORDER BY display_order")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(names, vec!["ccc", "aaa", "bbb"]);
        let orders: Vec<i32> =
            sqlx::query_scalar("SELECT display_order FROM bots ORDER BY display_order")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(orders, vec![0, 1, 2]);
    }

    /// ws F18 + ws F29: an unknown id rolls the whole reorder back.
    #[sqlx::test]
    async fn test_reorder_bots_rejects_unknown_id_atomically(pool: sqlx::PgPool) {
        sqlx::query("DELETE FROM bots").execute(&pool).await.unwrap();
        let a = insert_bot(&pool, "aaa").await;
        let b = insert_bot(&pool, "bbb").await;

        let err = reorder_bots(&pool, vec![b, Uuid::new_v4(), a])
            .await
            .expect_err("unknown id must be rejected");
        assert!(err.to_string().contains("please reload"));

        // Nothing moved.
        let names: Vec<String> =
            sqlx::query_scalar("SELECT name FROM bots ORDER BY display_order")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(names, vec!["aaa", "bbb"]);
    }

    /// ws F18: a duplicated id is rejected before any UPDATE runs, because
    /// Postgres would pick one of the two ordinals nondeterministically.
    #[sqlx::test]
    async fn test_reorder_bots_rejects_duplicate_id(pool: sqlx::PgPool) {
        sqlx::query("DELETE FROM bots").execute(&pool).await.unwrap();
        let a = insert_bot(&pool, "aaa").await;
        let b = insert_bot(&pool, "bbb").await;

        let err = reorder_bots(&pool, vec![a, b, a])
            .await
            .expect_err("duplicate id must be rejected");
        assert!(err.to_string().contains("duplicate entry"), "{err}");

        let names: Vec<String> =
            sqlx::query_scalar("SELECT name FROM bots ORDER BY display_order")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(names, vec!["aaa", "bbb"]);
    }

    /// ws F19: sequential creates never reuse a display_order, and the
    /// advisory lock is the thing that makes that true under concurrency.
    /// (A truly concurrent test would need two pool connections racing; the
    /// deterministic assertion here is the no-duplicate invariant.)
    #[sqlx::test]
    async fn test_create_bot_display_orders_are_unique(pool: sqlx::PgPool) {
        sqlx::query("DELETE FROM bots").execute(&pool).await.unwrap();
        for n in ["a", "b", "c", "d"] {
            insert_bot(&pool, n).await;
        }
        let dupes: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM (SELECT display_order FROM bots GROUP BY display_order HAVING count(*) > 1) d",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(dupes, 0);
        let orders: Vec<i32> =
            sqlx::query_scalar("SELECT display_order FROM bots ORDER BY display_order")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(orders, vec![0, 1, 2, 3]);
    }
```

| case | expected |
|---|---|
| `reorder_bots(vec![c,a,b])` on 3 bots | orders 0/1/2 in that sequence |
| `reorder_bots(vec![b, <random>, a])` | `Err` containing "please reload"; **no** order changed |
| `reorder_bots(vec![a, b, a])` (duplicate id) | `Err` containing "duplicate entry"; no transaction opened, nothing changed |
| `reorder_bots(vec![])` on a non-empty table | `Ok(())`, nothing changed (0 rows == 0 distinct ids) |
| `reorder_bots` with a **subset** of the bots (e.g. 2 of 3) | `Ok(())`, and the two named bots get orders 0/1 — which can collide with the unnamed bot's existing order. This is **pre-existing behaviour, not a regression**, and it is unreachable from the UI: the Up/Down buttons always dispatch the full `bot_ids` list (:1161, `let bot_ids: Vec<Uuid> = bots.iter().map(\|b\| b.id).collect();`). Do **not** add a "must cover every bot" check — that would be a behaviour change outside ws F18/F19 and is routed under Cross-package. |
| 4 sequential `create_bot`s | orders 0,1,2,3; zero duplicates |

Command: `cargo test -p web --features ssr test_reorder_bots && cargo test -p web --features ssr test_create_bot_display_orders`.

**Verification checkpoint:**
- [ ] All four new tests pass; the five pre-existing admin tests (`test_admin_list_bots_rejects_non_admin`, `..._list_providers_never_returns_full_key`, `..._create_provider_encrypts_key`, `..._update_provider_preserves_key_when_none`, `..._update_provider_replaces_key_when_some`) pass.
- [ ] `grep -n "for (i, id) in ordered_ids" admin.rs` returns nothing (the N-statement loop is gone).
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.

**Commit:** `fix(admin): atomic bot reorder + advisory lock on display_order (ws F18, ws F19)`

---

## Task 7: verify rows affected on updates (ws F29)

**Problem (restated):** every UPDATE/DELETE in this file calls `.execute()` and drops the `PgQueryResult`. Updating a row another admin just deleted returns `Ok(())`, the section clears its error slot, bumps `version`, refetches — and the change silently never happened. The admin has no way to tell a successful save from a no-op.

**Fix (re-derived), per site:** the finding's own split — check the **updates**, leave the **deletes** idempotent (a delete whose row is already gone achieved the user's intent). `reorder_bots` was done in Task 6. The remaining sites and their user-facing messages:

| site (live: fn, and the `.execute()` line) | statement | 0 rows -> return |
|---|---|---|
| `update_bot` :170-194, `.execute(pool)` at :190 | `UPDATE bots SET name, temperature, include_basic_strategy, include_advanced_strategy, enabled, can_replace_humans, updated_at` (:181) | `Err("Bot not found - it may have been deleted; reload and try again")` |
| `update_provider` key-set branch :323-339, `.execute(pool)` at :336 | `UPDATE llm_providers SET name, url, api_key_encrypted, enabled, updated_at` (:329) | `Err("Provider not found - it may have been deleted; reload and try again")` |
| `update_provider` no-key branch :340-351, `.execute(pool)` at :348 | `UPDATE llm_providers SET name, url, enabled, updated_at` (:342) | same message |
| `update_bot_provider` :450-472, `.execute(pool)` at :468 | `UPDATE bot_providers SET model, reasoning_effort, extra_body, priority, enabled` (:460) | `Err("Bot-provider link not found - it may have been deleted; reload and try again")` |
| `delete_bot` :213-220, `.execute(pool)` at :216 | `DELETE FROM bots` (:214) | **unchanged** `Ok(())` |
| `delete_provider` :357-364, `.execute(pool)` at :360 | `DELETE FROM llm_providers` (:358) | **unchanged** `Ok(())` |
| `delete_bot_provider` :475-482, `.execute(pool)` at :478 | `DELETE FROM bot_providers` (:476) | **unchanged** `Ok(())` |

These are user-facing messages, so use `ServerFnError::new(...)` directly, **not** `internal(...)` (which would collapse them to "Internal server error"). They surface in each section's error slot (`{move || error.get().map(|e| view! { <p class="error">{e}</p> })}` at :1166 `BotsSection`, :1478 `ProvidersSection`, :1815 `BotProvidersSection`) via the completion Effects.

**Presentation caveat, already accepted:** those slots render `e.to_string()`, so the admin sees `error running server function: Bot not found - ...`. The prefix is out of scope (Non-Goals, and Cross-package item 2). The messages are worded so they still read correctly through the prefix — do not try to strip it here.

**Reminder: ws F30 is rejected.** `update_bot_provider`'s statement gets a `rows_affected` check and **nothing else** — do not add `updated_at = now()`; `bot_providers` has no such column (`013_bot_efficacy.sql:23-34`: the `CREATE TABLE` has `created_at` at :32 and then `UNIQUE (bot_id, provider_id, model)` at :33, no `updated_at`; no migration 014-022 adds one). Adding it would be a runtime SQL error on every link edit, which no test would catch at compile time because these are runtime-checked `sqlx::query` calls, not macros.

**Files:**
- Modify: `rust/web/src/admin.rs` (`update_bot`, `update_provider` both branches, `update_bot_provider`)

**Steps:**

- [ ] In `update_bot`, change the trailing `.execute(pool).await.map_err(internal("admin_update_bot: update"))?;` + `Ok(())` to:

```rust
    let result = sqlx::query(/* ...unchanged SQL and .bind() chain... */)
        .execute(pool)
        .await
        .map_err(internal("admin_update_bot: update"))?;
    if result.rows_affected() == 0 {
        return Err(ServerFnError::new(
            "Bot not found - it may have been deleted; reload and try again",
        ));
    }
    Ok(())
```

  i.e. bind the `PgQueryResult` to `result` instead of discarding it, then check. Do not touch the SQL string or the bind order.
- [ ] In `update_provider`, do the same in **both** match arms. To avoid duplicating the message, capture `rows_affected` in each arm and check once after the `match`:

```rust
    let rows = match api_key {
        Some(key_str) => { /* ...unchanged... */
            sqlx::query("UPDATE llm_providers SET name = $2, url = $3, api_key_encrypted = $4, enabled = $5, updated_at = now() WHERE id = $1")
                /* ...unchanged binds... */
                .execute(pool)
                .await
                .map_err(internal("admin_update_provider: update"))?
                .rows_affected()
        }
        None => {
            sqlx::query("UPDATE llm_providers SET name = $2, url = $3, enabled = $4, updated_at = now() WHERE id = $1")
                /* ...unchanged binds... */
                .execute(pool)
                .await
                .map_err(internal("admin_update_provider: update"))?
                .rows_affected()
        }
    };
    if rows == 0 {
        return Err(ServerFnError::new(
            "Provider not found - it may have been deleted; reload and try again",
        ));
    }
    Ok(())
```

  If Task 8 has already landed, the `match` is over `ApiKeyUpdate` with three arms; apply the same `.rows_affected()` shape to all three. (Task 8 comes after this task, so as written you will have two arms here and Task 8 will add the third.)
- [ ] In `update_bot_provider`, same shape:

```rust
    let result = sqlx::query(
        "UPDATE bot_providers SET model = $2, reasoning_effort = $3, extra_body = $4, priority = $5, enabled = $6 WHERE id = $1",
    )
    /* ...unchanged binds... */
    .execute(pool)
    .await
    .map_err(internal("admin_update_bot_provider: update"))?;
    if result.rows_affected() == 0 {
        return Err(ServerFnError::new(
            "Bot-provider link not found - it may have been deleted; reload and try again",
        ));
    }
    Ok(())
```

- [ ] Add a comment above each of the three `delete_*` fns recording the deliberate choice:

```rust
// Deletes stay idempotent: a row that is already gone satisfies the request
// (ws F29 - only the updates report "not found").
```

**Test plan:**

- [ ] Add to `mod tests`:

```rust
    /// ws F29: updating a row that no longer exists must not report success.
    #[sqlx::test]
    async fn test_update_bot_unknown_id_is_not_found(pool: sqlx::PgPool) {
        let err = update_bot(
            &pool,
            Uuid::new_v4(),
            "ghost".to_string(),
            0.2,
            true,
            false,
            true,
            false,
        )
        .await
        .expect_err("unknown bot id must be rejected");
        assert!(err.to_string().contains("Bot not found"), "{err}");
    }

    #[sqlx::test]
    async fn test_update_provider_unknown_id_is_not_found(pool: sqlx::PgPool) {
        // No-key branch.
        let err = update_provider(
            &pool,
            Uuid::new_v4(),
            "ghost".to_string(),
            "http://localhost:1".to_string(),
            None,
            true,
        )
        .await
        .expect_err("unknown provider id must be rejected");
        assert!(err.to_string().contains("Provider not found"), "{err}");

        // Key-set branch.
        let err = update_provider(
            &pool,
            Uuid::new_v4(),
            "ghost".to_string(),
            "http://localhost:1".to_string(),
            Some("sk-whatever-1234".to_string()),
            true,
        )
        .await
        .expect_err("unknown provider id must be rejected on the key branch too");
        assert!(err.to_string().contains("Provider not found"), "{err}");
    }

    #[sqlx::test]
    async fn test_update_bot_provider_unknown_id_is_not_found(pool: sqlx::PgPool) {
        let err = update_bot_provider(
            &pool,
            Uuid::new_v4(),
            "gpt-4o-mini".to_string(),
            None,
            None,
            0,
            true,
        )
        .await
        .expect_err("unknown link id must be rejected");
        assert!(err.to_string().contains("link not found"), "{err}");
    }

    /// ws F29: deletes stay idempotent on purpose.
    #[sqlx::test]
    async fn test_deletes_are_idempotent(pool: sqlx::PgPool) {
        delete_bot(&pool, Uuid::new_v4()).await.unwrap();
        delete_provider(&pool, Uuid::new_v4()).await.unwrap();
        delete_bot_provider(&pool, Uuid::new_v4()).await.unwrap();
    }
```

  **Note:** if Task 8 lands before you write this, the `update_provider` calls take `ApiKeyUpdate::Keep` / `ApiKeyUpdate::Set(...)` instead of `None` / `Some(...)`. Task 8's step list says to update these.

| case | expected |
|---|---|
| `update_bot(random uuid)` | `Err` containing "Bot not found" |
| `update_provider(random uuid)`, both key branches | `Err` containing "Provider not found" |
| `update_bot_provider(random uuid)` | `Err` containing "link not found" |
| `update_bot` on an existing bot | still `Ok(())` (covered by Task 6's `insert_bot` usage and existing tests) |
| the three `delete_*` on random uuids | `Ok(())` |

Command: `cargo test -p web --features ssr test_update_bot_unknown && cargo test -p web --features ssr test_update_provider_unknown && cargo test -p web --features ssr test_update_bot_provider_unknown && cargo test -p web --features ssr test_deletes_are_idempotent`.

**Verification checkpoint:**
- [ ] Four new tests pass; all pre-existing admin tests pass (`test_admin_update_provider_preserves_key_when_none` and `..._replaces_key_when_some` both target real rows, so they are unaffected).
- [ ] `grep -n "updated_at" admin.rs` shows **no** occurrence inside `update_bot_provider` (ws F30 stays rejected).
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.

**Commit:** `fix(admin): report not-found instead of fake success on updates (ws F29)`

---

## Task 8: make "clear the API key" representable (ws F21)

**Problem (restated):** `update_provider`'s `api_key: Option<String>` (:319) has two states for three intentions. `None` means "keep the stored key" (the column is omitted from the `UPDATE`, :340-351); `Some(s)` means "replace with `s`" (:323-339). There is no value that writes `NULL`, even though `llm_providers.api_key_encrypted` is nullable (`013_bot_efficacy.sql:17`). `ProviderEditForm` reinforces this: it filters the empty string out (`.filter(|v| !v.is_empty())` at :1642) and its help text is `"Leave blank to keep existing key"` (:1657). An admin who wants to revoke a key must go to the database.

> ### STATED ASSUMPTION (product behaviour — flag to the Lead if a user decision is wanted)
>
> The empty-string submission keeps its **current** meaning: *keep the existing key*. Clearing requires ticking a new explicit **"Clear API key"** checkbox. When the checkbox is ticked, it **wins** over any text typed in the key field (the help text says so).
>
> **Why this and not the alternative:** the alternative — "empty text field means clear" — silently changes the meaning of every existing edit. Today an admin who edits a provider's URL leaves the key box blank; under that alternative that routine edit would destroy the key. Requiring an explicit checkbox is the least-surprising option, keeps every existing test's semantics intact, and cannot destroy a key by accident.
>
> **Alternative if the Lead prefers it:** drop the checkbox and use a sentinel in the text field (e.g. typing `-` clears). Rejected as undiscoverable.
>
> **Coherence check against live code (verified, not assumed).** The assumption is consistent end to end today: `ProviderEditForm` already turns a blank key box into `None` (`.filter(|v| !v.is_empty())`, :1642), `update_provider`'s `None` arm already omits the column (:340-351), and the existing help text already says `"Leave blank to keep existing key"` (:1657). So `Keep` is a faithful rename of today's `None`, `Set` of today's `Some`, and `Clear` is purely additive — **no existing behaviour changes and no existing test's semantics change.** The help text stays as-is and the new field's own help text carries the override rule.

**Fix (re-derived):** replace the `Option<String>` argument with an explicit three-state enum rather than adding a sixth positional `bool` (the update path already has five positional args and two bools; a seventh would be a footgun). `create_provider` keeps its `Option<String>` — there is no "keep" state at creation.

**Files:**
- Modify: `rust/web/src/admin.rs` (new `ApiKeyUpdate` enum; `update_provider`; `admin_update_provider`; `ProviderUpdateAction` alias; `ProvidersSection`'s `update_action`; `ProviderEditForm`; two existing tests)

**Steps:**

- [ ] Add the enum next to the other public DTOs, immediately after the closing `}` of `pub struct ProviderRow` (live :54; `grep -n "pub struct ProviderRow" admin.rs`) and before the `#[derive]` of `BotProviderRow`. It must **not** be `#[cfg(feature = "ssr")]` — it crosses the wire in the `ProviderUpdateAction` tuple, so it has to exist under `hydrate` too:

```rust
/// What an update should do to a provider's stored API key. `Option<String>`
/// could only express two of these three intentions, so "revoke this key"
/// was unrepresentable on the public API surface (ws F21).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiKeyUpdate {
    /// Leave `api_key_encrypted` exactly as it is.
    Keep,
    /// Encrypt and store this new key.
    Set(String),
    /// Set `api_key_encrypted` to NULL.
    Clear,
}
```

- [ ] Change the `ProviderUpdateAction` alias (live :10-11) to carry it:

```rust
type ProviderUpdateAction =
    Action<(Uuid, String, String, ApiKeyUpdate, bool), Result<(), ServerFnError>>;
```

- [ ] Change `update_provider`'s signature to `api_key: ApiKeyUpdate` and give the `match` three arms. With Task 7 already landed, the body is:

```rust
    let rows = match api_key {
        ApiKeyUpdate::Set(key_str) => {
            let enc_key =
                crate::crypto::load_key().map_err(internal("admin_update_provider: load key"))?;
            let encrypted = crate::crypto::encrypt(&enc_key, key_str.as_bytes())
                .map_err(internal("admin_update_provider: encrypt"))?;
            sqlx::query(
                "UPDATE llm_providers SET name = $2, url = $3, api_key_encrypted = $4, enabled = $5, updated_at = now() WHERE id = $1",
            )
            .bind(id)
            .bind(&name)
            .bind(&url)
            .bind(&encrypted)
            .bind(enabled)
            .execute(pool)
            .await
            .map_err(internal("admin_update_provider: update"))?
            .rows_affected()
        }
        ApiKeyUpdate::Clear => sqlx::query(
            "UPDATE llm_providers SET name = $2, url = $3, api_key_encrypted = NULL, enabled = $4, updated_at = now() WHERE id = $1",
        )
        .bind(id)
        .bind(&name)
        .bind(&url)
        .bind(enabled)
        .execute(pool)
        .await
        .map_err(internal("admin_update_provider: clear key"))?
        .rows_affected(),
        ApiKeyUpdate::Keep => sqlx::query(
            "UPDATE llm_providers SET name = $2, url = $3, enabled = $4, updated_at = now() WHERE id = $1",
        )
        .bind(id)
        .bind(&name)
        .bind(&url)
        .bind(enabled)
        .execute(pool)
        .await
        .map_err(internal("admin_update_provider: update"))?
        .rows_affected(),
    };
    if rows == 0 {
        return Err(ServerFnError::new(
            "Provider not found - it may have been deleted; reload and try again",
        ));
    }
    Ok(())
```

- [ ] Change `admin_update_provider`'s `api_key: Option<String>` parameter to `api_key: ApiKeyUpdate` and pass it straight through. The body is otherwise untouched (`require_admin` + delegate).
- [ ] In `ProvidersSection`, change the `update_action` closure's tuple type at :1401 from `&(Uuid, String, String, Option<String>, bool)` to `&(Uuid, String, String, ApiKeyUpdate, bool)`. The `let api_key = api_key.clone();` line at :1405 still works (`ApiKeyUpdate: Clone`). **`ProviderCreateForm`'s prop type at :1581 (`Action<(String, String, Option<String>), ...>`) and `ProvidersSection`'s `create_action` at :1393 keep `Option<String>` — creation has no "keep" state.**
- [ ] In `ProviderEditForm` (:1620-1670), add a `clear_input` node ref to the four existing ones at :1630-1633 and rewrite `on_submit` (:1635-1645) to build the three-state value. The block below is the whole replacement for :1630-1645:

```rust
    let name_input = NodeRef::<html::Input>::new();
    let url_input = NodeRef::<html::Input>::new();
    let key_input = NodeRef::<html::Input>::new();
    let clear_input = NodeRef::<html::Input>::new();
    let enabled_input = NodeRef::<html::Input>::new();

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let name = name_input.get().map(|el| el.value()).unwrap_or_default();
        let url = url_input.get().map(|el| el.value()).unwrap_or_default();
        // ws F21: blank still means "keep" - the explicit checkbox is the
        // only way to revoke, and it wins over anything typed in the field.
        let api_key = if clear_input.get().map(|el| el.checked()).unwrap_or(false) {
            ApiKeyUpdate::Clear
        } else {
            match key_input.get().map(|el| el.value()) {
                Some(v) if !v.is_empty() => ApiKeyUpdate::Set(v),
                _ => ApiKeyUpdate::Keep,
            }
        };
        let enabled = enabled_input.get().map(|el| el.checked()).unwrap_or(true);
        update_action.dispatch((provider_id, name, url, api_key, enabled));
    };
```

- [ ] In `ProviderEditForm`'s `view!`, insert the checkbox directly after the existing API Key `FormField` (:1657-1659) and before the `Enabled` one (:1660-1662). `FormField`'s `label`/`help` props are already used in this exact shape at :1657, so no component change is needed:

```rust
                    <FormField label="API Key" help="Leave blank to keep existing key">
                        <input type="password" node_ref=key_input/>
                    </FormField>
                    <FormField
                        label="Clear API key"
                        help="Removes the stored key. Overrides anything typed above."
                    >
                        <input type="checkbox" node_ref=clear_input/>
                    </FormField>
```

- [ ] Update the two existing tests that call `update_provider`: `test_admin_update_provider_preserves_key_when_none` (fn at :2256-2293, the call at :2274-2283, the `None` argument at :2279) passes `ApiKeyUpdate::Keep` instead; `test_admin_update_provider_replaces_key_when_some` (fn at :2295-2335, the call at :2314-2323, the `Some(new_key.to_string())` at :2319) passes `ApiKeyUpdate::Set(new_key.to_string())`. Also update **both** `update_provider` calls in Task 7's `test_update_provider_unknown_id_is_not_found` (`None` -> `ApiKeyUpdate::Keep`, `Some("sk-whatever-1234".to_string())` -> `ApiKeyUpdate::Set("sk-whatever-1234".to_string())`). Rename neither test — the names `..._when_none` / `..._when_some` now describe `Keep`/`Set`, which is mildly stale but renaming them loses the link to the findings and to ws F21's history.

**Test plan:**

- [ ] Add to `mod tests`:

```rust
    /// ws F21: Clear must actually NULL the column, and the listing must then
    /// report no key rather than a mask.
    #[sqlx::test]
    async fn test_admin_update_provider_clears_key(pool: sqlx::PgPool) {
        let key = test_encryption_key();
        let encrypted = crate::crypto::encrypt(&key, b"sk-to-be-revoked-1234").unwrap();
        let provider_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO llm_providers (id, name, url, api_key_encrypted, enabled) VALUES ($1, $2, $3, $4, true)",
        )
        .bind(provider_id)
        .bind("clear-test")
        .bind("http://localhost:8080")
        .bind(&encrypted)
        .execute(&pool)
        .await
        .unwrap();

        update_provider(
            &pool,
            provider_id,
            "clear-test".to_string(),
            "http://localhost:8080".to_string(),
            ApiKeyUpdate::Clear,
            true,
        )
        .await
        .unwrap();

        let raw: Option<Vec<u8>> =
            sqlx::query_scalar("SELECT api_key_encrypted FROM llm_providers WHERE id = $1")
                .bind(provider_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(raw, None, "Clear must NULL api_key_encrypted");

        let providers = list_providers(&pool).await.unwrap();
        assert_eq!(providers[0].api_key_masked, None);

        // And the name/url/enabled columns still updated in the Clear arm.
        update_provider(
            &pool,
            provider_id,
            "clear-test-renamed".to_string(),
            "http://localhost:9999".to_string(),
            ApiKeyUpdate::Clear,
            false,
        )
        .await
        .unwrap();
        let (name, url, enabled): (String, String, bool) =
            sqlx::query_as("SELECT name, url, enabled FROM llm_providers WHERE id = $1")
                .bind(provider_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(name, "clear-test-renamed");
        assert_eq!(url, "http://localhost:9999");
        assert!(!enabled);
    }
```

| case | expected |
|---|---|
| `ApiKeyUpdate::Clear` on a provider with a key | column becomes `NULL`; `api_key_masked` becomes `None`; name/url/enabled still applied |
| `ApiKeyUpdate::Clear` on a provider with no key | `Ok(())`, still `NULL` (idempotent; covered by the second call above) |
| `ApiKeyUpdate::Keep` | key byte-identical (existing `..._preserves_key_when_none`) |
| `ApiKeyUpdate::Set(k)` | key replaced and encrypted (existing `..._replaces_key_when_some`) |
| Clear + text typed | Clear wins (form-level; asserted by inspection of `on_submit`, not testable without a browser) |

Command: `cargo test -p web --features ssr test_admin_update_provider`.

**Verification checkpoint:**
- [ ] New test plus the two edited pre-existing provider-update tests pass.
- [ ] `grep -n "Option<String>" admin.rs` shows no remaining `api_key: Option<String>` on `update_provider`/`admin_update_provider` (the `create_provider` pair keeps theirs).
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.

**Commit:** `feat(admin): allow revoking a provider API key (ws F21)`

---

## Task 9: server-side validation of bot/provider inputs (ws F25)

**Problem (restated):** every constraint is client-side — `required` at :1289, :1350, :1604, :1607, :1652, :1655, :2031, :2041, :2051, :2141, and `step="0.1" min="0" max="2"` on temperature at :1292 and :1357-1358. Nothing at all constrains `reasoning_effort`, `extra_body`, `priority` or the test `prompt`. The `#[server]` fns accept an empty bot name, a non-finite or wildly out-of-range `temperature` (`bots.temperature` is `REAL NOT NULL DEFAULT 0.2` per `013_bot_efficacy.sql:8`, and Postgres accepts `'NaN'::real`), a URL that is not a URL, an empty `model`, and an unbounded `prompt`/`extra_body`. The admin gate limits the blast radius to admins and to anyone who can forge an admin session, but there is no defence in depth, and a `NaN` temperature silently poisons every LLM call that bot makes.

**Fix (re-derived):** validate in the **stratum-2 helpers**, not in the `#[server]` wrappers, so the `#[sqlx::test]`s exercise the checks. Messages are user-facing, so `ServerFnError::new` (never `internal`). Store the **trimmed** values.

Chosen limits and why:

| field | check | why that bound |
|---|---|---|
| bot `name` | trimmed non-empty, <= 64 chars | `game_bots.bot_name` references bots by name (see ws F27 fencing); an empty name is unresolvable. 64 is generous vs the seeded `easy`/`medium`/`hard`. |
| `temperature` | `is_finite()` and `(0.0..=2.0)` | matches the HTML `min`/`max` the forms already advertise, and excludes `NaN`/`inf`, which `REAL` would happily store. |
| provider `name` | trimmed non-empty, <= 64 | `llm_providers.name` is `UNIQUE` and is shown in the links table. |
| provider `url` | trimmed non-empty, <= 512, starts with `http://` or `https://` | `test_provider`/`test_bot_provider` do `format!("{url}/v1/chat/completions")` and `POST` it; a non-HTTP scheme cannot succeed and a relative value would be a request-forgery footgun. |
| `model` | trimmed non-empty, <= 128 | goes into the JSON body verbatim; empty is a guaranteed upstream 400. |
| `reasoning_effort` | if `Some`: trimmed non-empty, <= 32 | the field is free text on purpose (providers disagree on the vocabulary), so only length is bounded. **Do not** restrict to low/medium/high. |
| `extra_body` | if `Some`: must be a JSON **object**, serialized length <= 8192 | it is merged into the request body object (`test_bot_provider`'s merge loop requires `patch.as_object()`), so a non-object is silently ignored today. |
| `prompt` (test only) | <= 4096 chars | bounds an admin-triggered upstream request. |
| `priority` | none | any `i32` is meaningful as a sort key. |
| `api_key` | **none** | legitimately optional (`api_key_encrypted` is nullable). The empty-string case is a real but out-of-scope wart — see Cross-package item 4. Do not add a check here. |

**Two consequences for existing UI behaviour, both checked against the live forms:**

1. **`reasoning_effort`'s non-empty check is unreachable from the UI, by design.** Both `BotProviderCreateForm` (:1992-1995) and `BotProviderEditForm` (:2099-2102) already do `.filter(|v| !v.trim().is_empty())`, so a blank field arrives as `None`, not `Some("")`. The server check exists only for crafted calls. **No regression.**
2. **`extra_body`'s object check *is* reachable and *is* a behaviour change.** Both forms accept any `serde_json::from_str`-parseable value (:1997-2011 and :2104-2118), so an admin can submit `[1,2,3]` today. It is stored and then **silently ignored** by `test_bot_provider`'s merge, which requires `patch.as_object()` (:590-591). After this task that submission gets a clear `"Extra body must be a JSON object"` instead. That is the intended improvement — call it out in the commit body so it is not mistaken for a bug.

**Files:**
- Modify: `rust/web/src/admin.rs` (validation helpers; `create_bot`, `update_bot`, `create_provider`, `update_provider`, `create_bot_provider`, `update_bot_provider`, `test_bot_provider`)

**Steps:**

- [ ] Add the helpers immediately after the `BOT_DISPLAY_ORDER_LOCK` constant from Task 6:

```rust
/// ws F25: cheap server-side validation. Every constraint below is duplicated
/// in the HTML forms; these exist because the server fns are a public surface
/// and a crafted call otherwise stores NaN temperatures, empty bot names or
/// non-HTTP provider URLs.
#[cfg(feature = "ssr")]
fn require_text(value: &str, field: &'static str, max: usize) -> Result<String, ServerFnError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ServerFnError::new(format!("{field} is required")));
    }
    if trimmed.chars().count() > max {
        return Err(ServerFnError::new(format!(
            "{field} must be at most {max} characters"
        )));
    }
    Ok(trimmed.to_string())
}

#[cfg(feature = "ssr")]
fn validate_temperature(temperature: f32) -> Result<(), ServerFnError> {
    if !temperature.is_finite() || !(0.0..=2.0).contains(&temperature) {
        return Err(ServerFnError::new(
            "Temperature must be a number between 0.0 and 2.0",
        ));
    }
    Ok(())
}

#[cfg(feature = "ssr")]
fn validate_provider_url(url: &str) -> Result<String, ServerFnError> {
    let url = require_text(url, "URL", 512)?;
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(ServerFnError::new(
            "URL must start with http:// or https://",
        ));
    }
    Ok(url)
}

#[cfg(feature = "ssr")]
fn validate_extra_body(
    extra_body: Option<serde_json::Value>,
) -> Result<Option<serde_json::Value>, ServerFnError> {
    let Some(value) = extra_body else {
        return Ok(None);
    };
    if !value.is_object() {
        return Err(ServerFnError::new(
            "Extra body must be a JSON object",
        ));
    }
    if value.to_string().len() > 8192 {
        return Err(ServerFnError::new(
            "Extra body must be at most 8192 bytes of JSON",
        ));
    }
    Ok(Some(value))
}

#[cfg(feature = "ssr")]
fn validate_reasoning_effort(
    reasoning_effort: Option<String>,
) -> Result<Option<String>, ServerFnError> {
    match reasoning_effort {
        // Free text on purpose: providers disagree on the vocabulary.
        Some(v) => Ok(Some(require_text(&v, "Reasoning effort", 32)?)),
        None => Ok(None),
    }
}
```

- [ ] At the top of `create_bot` and `update_bot`, before any SQL:

```rust
    let name = require_text(&name, "Bot name", 64)?;
    validate_temperature(temperature)?;
```

  `name` is already an owned `String` parameter in both, so shadowing it with the trimmed value is what makes the trimmed form get stored. `create_bot` binds `&name`; `update_bot` binds `&name`. No other change.
- [ ] At the top of `create_provider` and `update_provider`:

```rust
    let name = require_text(&name, "Provider name", 64)?;
    let url = validate_provider_url(&url)?;
```

- [ ] At the top of `create_bot_provider` and `update_bot_provider`:

```rust
    let model = require_text(&model, "Model", 128)?;
    let reasoning_effort = validate_reasoning_effort(reasoning_effort)?;
    let extra_body = validate_extra_body(extra_body)?;
```

- [ ] At the top of `test_bot_provider` (whose `prompt: &str`):

```rust
    if prompt.chars().count() > 4096 {
        return Err(ServerFnError::new(
            "Test prompt must be at most 4096 characters",
        ));
    }
```

**Test plan:**

- [ ] Add to `mod tests`:

```rust
    /// ws F25: crafted server-fn arguments are rejected before they reach SQL.
    #[sqlx::test]
    async fn test_bot_input_validation(pool: sqlx::PgPool) {
        // Empty / whitespace-only name.
        for name in ["", "   "] {
            let err = create_bot(&pool, name.to_string(), 0.2, true, false, false)
                .await
                .expect_err("empty bot name must be rejected");
            assert!(err.to_string().contains("Bot name is required"), "{err}");
        }
        // Over-long name.
        let err = create_bot(&pool, "x".repeat(65), 0.2, true, false, false)
            .await
            .expect_err("over-long bot name must be rejected");
        assert!(err.to_string().contains("at most 64"), "{err}");
        // Non-finite and out-of-range temperature.
        for t in [f32::NAN, f32::INFINITY, -0.1, 2.1, 1e9] {
            let err = create_bot(&pool, "tempbot".to_string(), t, true, false, false)
                .await
                .expect_err("bad temperature must be rejected");
            assert!(err.to_string().contains("between 0.0 and 2.0"), "{t}: {err}");
        }
        // Boundaries accepted, and the name is stored trimmed.
        let bot = create_bot(&pool, "  edge  ".to_string(), 0.0, true, false, false)
            .await
            .unwrap();
        assert_eq!(bot.name, "edge");
        create_bot(&pool, "edge2".to_string(), 2.0, true, false, false)
            .await
            .unwrap();
    }

    #[sqlx::test]
    async fn test_provider_input_validation(pool: sqlx::PgPool) {
        let err = create_provider(&pool, "  ".to_string(), "https://a.example".to_string(), None)
            .await
            .expect_err("empty provider name must be rejected");
        assert!(err.to_string().contains("Provider name is required"), "{err}");

        for url in ["", "   ", "a.example", "ftp://a.example", "//a.example"] {
            let err = create_provider(&pool, "p".to_string(), url.to_string(), None)
                .await
                .expect_err("bad url must be rejected");
            assert!(
                err.to_string().contains("URL"),
                "{url}: {err}"
            );
        }

        let p = create_provider(
            &pool,
            " trimmed ".to_string(),
            " https://a.example ".to_string(),
            None,
        )
        .await
        .unwrap();
        assert_eq!(p.name, "trimmed");
        assert_eq!(p.url, "https://a.example");
    }

    #[sqlx::test]
    async fn test_bot_provider_input_validation(pool: sqlx::PgPool) {
        let bot = create_bot(&pool, "linkbot".to_string(), 0.2, true, false, false)
            .await
            .unwrap();
        let provider = create_provider(
            &pool,
            "linkprovider".to_string(),
            "https://a.example".to_string(),
            None,
        )
        .await
        .unwrap();

        let err = create_bot_provider(&pool, bot.id, provider.id, "  ".to_string(), None, None, 0)
            .await
            .expect_err("empty model must be rejected");
        assert!(err.to_string().contains("Model is required"), "{err}");

        let err = create_bot_provider(
            &pool,
            bot.id,
            provider.id,
            "gpt-4o-mini".to_string(),
            None,
            Some(serde_json::json!([1, 2, 3])),
            0,
        )
        .await
        .expect_err("non-object extra_body must be rejected");
        assert!(err.to_string().contains("JSON object"), "{err}");

        let big = serde_json::json!({ "pad": "x".repeat(9000) });
        let err = create_bot_provider(
            &pool,
            bot.id,
            provider.id,
            "gpt-4o-mini".to_string(),
            None,
            Some(big),
            0,
        )
        .await
        .expect_err("oversized extra_body must be rejected");
        assert!(err.to_string().contains("8192"), "{err}");

        // Valid link still works, and model/reasoning are trimmed.
        let link = create_bot_provider(
            &pool,
            bot.id,
            provider.id,
            " gpt-4o-mini ".to_string(),
            Some(" low ".to_string()),
            Some(serde_json::json!({"top_p": 0.9})),
            3,
        )
        .await
        .unwrap();
        assert_eq!(link.model, "gpt-4o-mini");
        assert_eq!(link.reasoning_effort.as_deref(), Some("low"));
    }
```

| case | expected |
|---|---|
| bot name `""` / `"   "` / 65 chars | `Err` |
| temperature `NaN`, `inf`, `-0.1`, `2.1`, `1e9` | `Err` "between 0.0 and 2.0" |
| temperature `0.0` and `2.0` | `Ok` (inclusive bounds) |
| provider url `""`, `"a.example"`, `"ftp://..."`, `"//a.example"` | `Err` |
| `extra_body` = JSON array | `Err` "JSON object" |
| `extra_body` > 8192 bytes serialized | `Err` "8192" |
| whitespace-padded name/url/model/effort | stored trimmed |

Command: `cargo test -p web --features ssr validation`.

**Verification checkpoint:**
- [ ] Three new tests pass; all pre-existing admin tests pass (their inputs — `"test-provider"`, `"http://localhost:8080"`, `"enc-test"` — are all already valid; note `http://localhost:8080` passes the `http://` check deliberately, so local dev providers keep working).
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.

**Commit:** `feat(admin): server-side validation of bot and provider inputs (ws F25)`

---

## Task 10: resolve `test_provider`'s model instead of hardcoding it (ws F22)

**Problem (restated):** the provider health check sends a fixed model (`test_provider`, :509-514, literal at :510):

```rust
    let body = serde_json::json!({
        "model": "gpt-4o-mini",
        ...
```

Any endpoint that does not serve that exact model id — which is most non-OpenAI endpoints — fails the admin's primary health check even when perfectly healthy. The per-link check `test_bot_provider` already uses the configured `bot_providers.model`.

**Schema fact this fix depends on (verified 2026-07-25):** there is **no** model column on `llm_providers` or `bots`. `grep -rn "model" rust/web/migrations/*.sql` returns exactly four hits: `013_bot_efficacy.sql:27` (`model TEXT NOT NULL` inside `CREATE TABLE bot_providers`), `013_bot_efficacy.sql:33` (`UNIQUE (bot_id, provider_id, model)`), and two unrelated prose comments about the Rust `User` *model struct* (`010_friends.sql:11`, `021_add_game_visibility.sql:2`). So the only configured model for a provider is whatever its `bot_providers` rows say. **No new migration is required by this task** (which is fortunate — the last migration is `022_concede_bot_replacement.sql`, and adding a column would mean an immutable new `023_*.sql`).

**Fix (re-derived):** the model becomes an optional argument. When absent, resolve the provider's highest-priority **enabled** link model. When neither is available, return an actionable error rather than silently guessing — a guess is exactly the false negative the finding is about. Surface the optional model in the UI as one shared text input, mirroring `BotProvidersSection`'s existing shared `test_prompt` input (live :1816-1823: the `RwSignal` at :1751, the `<div class="form-actions">` block at :1816-1823).

**Ordering note:** this task uses `require_text`, which Task 9 introduces. Task 9 must be landed first (it is, in the printed order).

**Files:**
- Modify: `rust/web/src/admin.rs` (`test_provider`, `admin_test_provider`, `ProvidersSection`'s `test_action` + a `test_model` signal + the input markup)

**Steps:**

- [ ] Change `test_provider`'s signature (:485-489) to take the model and resolve it. Replace the region from `let (url, api_key_encrypted) = row.ok_or_else(...)` (:497) down to and including the `let body = serde_json::json!({ ... });` (:509-514) with the following. The `load_key`/`decrypt`/`api_key` block at :499-507 stays exactly where it is, between the two:

```rust
pub async fn test_provider(
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    provider_id: Uuid,
    model: Option<String>,
) -> Result<String, ServerFnError> {
    // ...unchanged SELECT url, api_key_encrypted...
    let (url, api_key_encrypted) = row.ok_or_else(|| ServerFnError::new("Provider not found"))?;

    // ws F22: never fabricate a model id. An explicit model wins; otherwise
    // use the provider's highest-priority enabled link model. `bot_providers`
    // is the only place a model is configured (migration 013) - neither
    // `llm_providers` nor `bots` has a model column.
    let model = match model.map(|m| require_text(&m, "Model", 128)).transpose()? {
        Some(m) => m,
        None => sqlx::query_scalar::<_, String>(
            "SELECT model FROM bot_providers \
             WHERE provider_id = $1 AND enabled \
             ORDER BY priority, model LIMIT 1",
        )
        .bind(provider_id)
        .fetch_optional(pool)
        .await
        .map_err(internal("admin_test_provider: resolve model"))?
        .ok_or_else(|| {
            ServerFnError::new(
                "No enabled bot-provider link for this provider, so there is no configured \
                 model to test with. Enter a model above, or add a link first.",
            )
        })?,
    };

    // ...unchanged load_key / decrypt / api_key block...

    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": "Say hello"}],
        "stream": false,
        "max_tokens": 5
    });
```

  Keep the `load_key`/`decrypt`/`api_key` block exactly where it is; only the `model` resolution is inserted and the `json!` `"model"` value changes from the literal to the binding. `ORDER BY priority, model` makes the choice deterministic when two links share a priority (important for the test below).
- [ ] Change `admin_test_provider` to accept and forward the model:

```rust
#[server(AdminTestProvider, "/api")]
pub async fn admin_test_provider(
    provider_id: Uuid,
    model: Option<String>,
) -> Result<String, ServerFnError> {
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let http_client = expect_context::<reqwest::Client>();
    require_admin(&pool, "admin_test_provider: check admin").await?;

    test_provider(&pool, &http_client, provider_id, model).await
}
```

- [ ] In `ProvidersSection`, add a shared model signal and change `test_action`'s input type. Put the signal immediately before `test_action`:

```rust
    // Optional model override for the provider health check; blank means
    // "use the provider's configured link model" (ws F22).
    let test_model = RwSignal::new(String::new());
    let test_action = Action::new(|(id, model): &(Uuid, Option<String>)| {
        let id = *id;
        let model = model.clone();
        async move { admin_test_provider(id, model).await }
    });
```

- [ ] Update the Test button's dispatch and both `input().get()` comparisons in `ProvidersSection`, which currently compare `== Some(id)` against a bare `Uuid`: the `disabled=` closure at :1509-1512, the label closure at :1517-1525, and the `on:click` at :1513-1515. (Task 13 changes the **Effect**'s use of `input()`; these two guards stay, but both must be re-shaped here or the tuple change will not compile.) Dispatch becomes:

```rust
                                        on:click=move |_| {
                                            let m = test_model.get();
                                            let m = if m.trim().is_empty() { None } else { Some(m) };
                                            test_action.dispatch((id, m));
                                        }
```

  and the two `test_action.input().get() == Some(id)` guards become `test_action.input().get().is_some_and(|(tid, _)| tid == id)` (the same shape `BotProvidersSection` already uses at live :1871-1874).
- [ ] Add the input above the providers table in `ProvidersSection`'s `view!`, between the `{move || error.get().map(...)}` line (:1478) and `<table class="admin-table">` (:1479), copying the `test_prompt` markup from `BotProvidersSection` (:1816-1823). `event_target_value` is already in scope in this file (used at :1821):

```rust
        <div class="form-actions">
            <label>"Test model (blank = provider's configured model): "</label>
            <input
                type="text"
                prop:value=move || test_model.get()
                on:input=move |ev| test_model.set(event_target_value(&ev))
            />
        </div>
```

**Test plan:** `test_provider` performs a real HTTP POST, so the model-resolution branch is what is testable without a network. Never call a real LLM in a test (CODING.md).

- [ ] Add to `mod tests`:

```rust
    /// ws F22: the model must come from configuration, never from a literal.
    /// Asserted through the resolution failure path, which runs before any
    /// HTTP request is attempted.
    #[sqlx::test]
    async fn test_provider_requires_a_configured_model(pool: sqlx::PgPool) {
        let key = test_encryption_key();
        let encrypted = crate::crypto::encrypt(&key, b"sk-modeltest-1234").unwrap();
        let provider_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO llm_providers (id, name, url, api_key_encrypted, enabled) VALUES ($1, $2, $3, $4, true)",
        )
        .bind(provider_id)
        .bind("modeltest")
        .bind("http://127.0.0.1:1")
        .bind(&encrypted)
        .execute(&pool)
        .await
        .unwrap();

        let client = reqwest::Client::new();

        // No links at all: actionable error, and no request attempted.
        let err = test_provider(&pool, &client, provider_id, None)
            .await
            .expect_err("no configured model must be an error, not a guess");
        assert!(err.to_string().contains("no configured"), "{err}");

        // A disabled link does not count.
        let bot = create_bot(&pool, "modelbot".to_string(), 0.2, true, false, false)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO bot_providers (bot_id, provider_id, model, priority, enabled) VALUES ($1, $2, $3, 0, false)",
        )
        .bind(bot.id)
        .bind(provider_id)
        .bind("disabled-model")
        .execute(&pool)
        .await
        .unwrap();
        let err = test_provider(&pool, &client, provider_id, None)
            .await
            .expect_err("a disabled link must not supply the model");
        assert!(err.to_string().contains("no configured"), "{err}");

        // An empty explicit model is a validation error, not a fallback.
        let err = test_provider(&pool, &client, provider_id, Some("  ".to_string()))
            .await
            .expect_err("blank explicit model must be rejected");
        assert!(err.to_string().contains("Model is required"), "{err}");

        // With an enabled link, resolution succeeds and we get past the model
        // step - the request to 127.0.0.1:1 then fails, which is the proof
        // that resolution no longer short-circuits.
        sqlx::query(
            "INSERT INTO bot_providers (bot_id, provider_id, model, priority, enabled) VALUES ($1, $2, $3, 1, true)",
        )
        .bind(bot.id)
        .bind(provider_id)
        .bind("configured-model")
        .execute(&pool)
        .await
        .unwrap();
        let err = test_provider(&pool, &client, provider_id, None)
            .await
            .expect_err("connection to port 1 must fail");
        assert!(
            !err.to_string().contains("no configured"),
            "model resolution should have succeeded: {err}"
        );
    }
```

  The final assertion relies on `internal("admin_test_provider: request")` collapsing the connection error to "Internal server error"; asserting only that it is **not** the resolution message keeps the test independent of that wording.

| case | expected |
|---|---|
| provider with no links, `model = None` | `Err` "no configured" |
| provider with only a **disabled** link | `Err` "no configured" |
| explicit `model = Some("  ")` | `Err` "Model is required" |
| provider with an enabled link | resolution succeeds (error is the connection failure, not the resolution failure) |
| two enabled links, priorities 5 and 1 | the priority-1 model is chosen (add if you want the extra assertion; `ORDER BY priority, model` makes it deterministic) |
| `grep -c "gpt-4o-mini" admin.rs` | only test-file occurrences remain; **zero** inside `test_provider` |

Command: `cargo test -p web --features ssr test_provider_requires_a_configured_model`.

**Verification checkpoint:**
- [ ] New test passes.
- [ ] `grep -n '"gpt-4o-mini"' admin.rs` shows no hit inside `test_provider`.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.

**Commit:** `fix(admin): resolve test_provider's model from configuration (ws F22)`

---

## Task 11: cap the upstream test body and allowlist headers (ws F23)

**Problem (restated):** four unbounded reads of an admin-configured, possibly hostile upstream:

- `test_provider` error path (:524-531): `resp.text().await` then `Ok(format!("HTTP {status}: {text}"))`, with the read error swallowed into `"unable to read body"`.
- `test_provider` success path (:533-536): `resp.json().await` — also unbounded.
- `test_bot_provider` (:618-621): `resp.text().await` into `TestBotProviderResponse.body`.
- `test_bot_provider` (:613-617): **every** response header copied into `TestBotProviderResponse.headers`.

The 10s client timeout bounds *duration*, not *size*: an endpoint streaming 2 GB/s inside 10s spikes server memory and then serialises the whole thing into the server-fn response. Headers may echo credentials or set cookies.

**Fix (re-derived):** `reqwest 0.13.4` exposes `pub async fn chunk(&mut self) -> crate::Result<Option<Bytes>>` (`src/async_impl/response.rs:310`, verified — it is **not** behind a cargo feature, so the crate's `default-features = false, features = ["json","form","rustls"]` config in `web/Cargo.toml:70` is sufficient), so a real streaming cap is available — dropping the `Response` after the cap cancels the rest of the transfer. `bytes()`/`text()` cannot do this. Cap at 8 KiB, which is far more than any `max_tokens: 5` completion or provider error envelope, and mark truncation visibly. Allowlist headers by lowercase name.

**Files:**
- Modify: `rust/web/src/admin.rs` (two helpers; `test_provider`; `test_bot_provider`)

**Steps:**

- [ ] Add the helpers immediately before `pub async fn test_provider`:

```rust
/// Cap on bytes read from an admin-configured upstream during a test call.
/// The 10s reqwest timeout bounds how long a hostile endpoint can stream, not
/// how much (ws F23). Comfortably above any real completion or error envelope.
#[cfg(feature = "ssr")]
const MAX_TEST_BODY_BYTES: usize = 8 * 1024;

/// Response headers worth showing an admin. Everything else is dropped rather
/// than round-tripped: an upstream can set arbitrary headers, including
/// cookies and echoed credentials (ws F23).
#[cfg(feature = "ssr")]
const TEST_HEADER_ALLOWLIST: &[&str] = &[
    "content-type",
    "content-length",
    "date",
    "retry-after",
    "x-request-id",
    "x-ratelimit-limit",
    "x-ratelimit-remaining",
    "x-ratelimit-reset",
];

/// Read at most `MAX_TEST_BODY_BYTES` of a response body, then stop. Dropping
/// the response cancels the remainder of the transfer.
#[cfg(feature = "ssr")]
async fn read_capped_body(mut resp: reqwest::Response) -> String {
    let mut buf: Vec<u8> = Vec::new();
    let mut truncated = false;
    loop {
        match resp.chunk().await {
            Ok(Some(chunk)) => {
                let remaining = MAX_TEST_BODY_BYTES.saturating_sub(buf.len());
                if chunk.len() > remaining {
                    buf.extend_from_slice(&chunk[..remaining]);
                    truncated = true;
                    break;
                }
                buf.extend_from_slice(&chunk);
                if buf.len() >= MAX_TEST_BODY_BYTES {
                    break;
                }
            }
            Ok(None) => break,
            Err(_) => return String::from_utf8_lossy(&buf).into_owned() + "\n<error reading body>",
        }
    }
    let mut text = String::from_utf8_lossy(&buf).into_owned();
    if truncated {
        text.push_str(&format!(
            "\n<truncated at {MAX_TEST_BODY_BYTES} bytes>"
        ));
    }
    text
}

/// Filter response headers down to `TEST_HEADER_ALLOWLIST`.
#[cfg(feature = "ssr")]
fn allowlisted_headers(headers: &reqwest::header::HeaderMap) -> Vec<(String, String)> {
    headers
        .iter()
        .filter(|(k, _)| TEST_HEADER_ALLOWLIST.contains(&k.as_str()))
        .map(|(k, v)| {
            (
                k.to_string(),
                v.to_str().unwrap_or("<binary>").to_string(),
            )
        })
        .collect()
}
```

  `HeaderName::as_str()` is already lowercase, so the allowlist comparison needs no normalisation.

  **One documented limitation of the cap.** If the body is *exactly* `MAX_TEST_BODY_BYTES` and more data follows in a later chunk, the `buf.len() >= MAX_TEST_BODY_BYTES` break fires with `truncated == false`, so the output is capped but unmarked. This is a cosmetic edge (an 8192-byte-aligned hostile body) and is accepted deliberately rather than adding a look-ahead read. Do **not** "fix" it by reading one extra chunk — that defeats the point of the cap.
- [ ] In `test_provider`, replace **both** body reads (:524-536). The tail of the fn becomes:

```rust
    let status = resp.status();
    let text = read_capped_body(resp).await;

    if !status.is_success() {
        return Ok(format!("HTTP {status}: {text}"));
    }

    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(internal("admin_test_provider: parse response"))?;

    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("No content in response");
    Ok(content.to_string())
```

  Note `status` must be captured **before** `read_capped_body` consumes the response. This also removes the `resp.text().await.unwrap_or_else(...)` swallow on the error path — `read_capped_body` handles read errors internally.
- [ ] In `test_bot_provider`, replace the header collection and body read (:612-621). Note the outer `let mut body = serde_json::json!({...})` at :582 is a *different* binding that the live code already shadows with the response text at :618 — keep that shadowing:

```rust
    let status = resp.status().as_u16();
    let headers = allowlisted_headers(resp.headers());
    let body = read_capped_body(resp).await;
```

  `allowlisted_headers` borrows, so it must run before `read_capped_body` takes ownership. The `Ok(TestBotProviderResponse { status, headers, body, elapsed_ms })` construction is unchanged.

**Test plan:** these need an upstream, so use the repo's established in-process mock-server pattern.

> **Corrected citation — read this before writing the helper.** An earlier draft of this spec pointed at `rust/web/src/game/client.rs`. **That file does not exist.** The real, live pattern is `spawn_mock_game_service` in `rust/web/tests/ssr_pages.rs:104-128`: build an `axum::Router` with a concrete `route(path, post(handler))`, `TcpListener::bind("127.0.0.1:0")`, read `local_addr()`, `tokio::spawn(axum::serve(listener, app))`, return `format!("http://{addr}")`. The helper below is that pattern with the handler swapped. Copy the surrounding mechanics from `ssr_pages.rs:120-127` if anything about the spawn does not compile.
>
> Two deliberate shape choices, so they are not "simplified" away:
> - **A concrete `route("/v1/chat/completions", post(...))`, not `Router::fallback(any(...))`.** Both `test_provider` (:517) and `test_bot_provider` (:604) POST to `format!("{url}/v1/chat/completions")`, so one route is enough; and `Router::fallback` takes a `Handler`, not the `MethodRouter` that `any(...)` returns, so the `fallback(any(...))` form is not guaranteed to resolve.
> - **The closure clones its captures instead of moving them**, which keeps it `Fn + Clone` — the bound axum's `Handler` impls require `Clone`.
>
> `axum` is available to `src/` unit tests (it is an `ssr`-feature dependency, `web/Cargo.toml:20`, default features on, so `axum::serve`/`http1` are present), and `tokio::net` is reachable through feature unification — `tests/ssr_pages.rs:22` already does `use tokio::net::TcpListener;`.

- [ ] Add to `mod tests`:

```rust
    /// Spawn a throwaway HTTP server on an ephemeral port that answers
    /// POST /v1/chat/completions with a fixed status/body/headers, and return
    /// its base URL. Same in-process pattern as `spawn_mock_game_service` in
    /// `tests/ssr_pages.rs:104-128`; never calls a real LLM (docs/CODING.md).
    async fn spawn_upstream(
        status: u16,
        body: Vec<u8>,
        extra_headers: Vec<(&'static str, &'static str)>,
    ) -> String {
        use axum::routing::post;
        let app = axum::Router::new().route(
            "/v1/chat/completions",
            post(move || {
                let body = body.clone();
                let extra_headers = extra_headers.clone();
                async move {
                    let mut headers = axum::http::HeaderMap::new();
                    for (k, v) in extra_headers {
                        headers.insert(
                            axum::http::HeaderName::from_static(k),
                            axum::http::HeaderValue::from_static(v),
                        );
                    }
                    (
                        axum::http::StatusCode::from_u16(status).unwrap(),
                        headers,
                        body,
                    )
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}")
    }
```

  `HeaderName::from_static` requires an already-lowercase `&'static str` — every name passed in below is lowercase. Because `Vec<u8>`'s own `IntoResponse` sets `content-type: application/octet-stream` and the `HeaderMap` part is applied on top, a response may carry two `content-type` values; that is harmless for these assertions (which only ask whether a name is present) and for `test_bot_provider`, which never parses the body.

```rust

    async fn provider_with_link(pool: &sqlx::PgPool, url: &str) -> Uuid {
        let key = test_encryption_key();
        let encrypted = crate::crypto::encrypt(&key, b"sk-capped-1234").unwrap();
        let provider_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO llm_providers (id, name, url, api_key_encrypted, enabled) VALUES ($1, $2, $3, $4, true)",
        )
        .bind(provider_id)
        .bind("capped")
        .bind(url)
        .bind(&encrypted)
        .execute(pool)
        .await
        .unwrap();
        let bot = create_bot(pool, "cappedbot".to_string(), 0.2, true, false, false)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO bot_providers (bot_id, provider_id, model, priority, enabled) VALUES ($1, $2, 'm', 0, true)",
        )
        .bind(bot.id)
        .bind(provider_id)
        .execute(pool)
        .await
        .unwrap();
        provider_id
    }

    /// ws F23: an oversized upstream body is truncated, not buffered whole.
    #[sqlx::test]
    async fn test_provider_truncates_huge_error_body(pool: sqlx::PgPool) {
        let url = spawn_upstream(500, vec![b'x'; 1_000_000], vec![]).await;
        let provider_id = provider_with_link(&pool, &url).await;
        let out = test_provider(&pool, &reqwest::Client::new(), provider_id, None)
            .await
            .unwrap();
        assert!(out.starts_with("HTTP 500"), "{out}");
        assert!(out.contains("<truncated at 8192 bytes>"), "not truncated");
        assert!(
            out.len() < 9_000,
            "body was not capped: {} bytes",
            out.len()
        );
    }

    /// ws F23: only allowlisted headers reach the client.
    #[sqlx::test]
    async fn test_bot_provider_filters_headers_and_caps_body(pool: sqlx::PgPool) {
        let url = spawn_upstream(
            200,
            vec![b'y'; 1_000_000],
            vec![
                ("content-type", "application/json"),
                ("set-cookie", "session=leaked"),
                ("x-upstream-secret", "nope"),
            ],
        )
        .await;
        let provider_id = provider_with_link(&pool, &url).await;
        let link_id: Uuid =
            sqlx::query_scalar("SELECT id FROM bot_providers WHERE provider_id = $1")
                .bind(provider_id)
                .fetch_one(&pool)
                .await
                .unwrap();

        let resp = test_bot_provider(&pool, &reqwest::Client::new(), link_id, "hi")
            .await
            .unwrap();
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("<truncated at 8192 bytes>"));
        assert!(resp.body.len() < 9_000, "body not capped");
        let names: Vec<&str> = resp.headers.iter().map(|(k, _)| k.as_str()).collect();
        assert!(names.contains(&"content-type"), "{names:?}");
        assert!(!names.contains(&"set-cookie"), "leaked set-cookie: {names:?}");
        assert!(
            !names.contains(&"x-upstream-secret"),
            "leaked unknown header: {names:?}"
        );
    }

    /// A small body is returned intact with no truncation marker.
    #[sqlx::test]
    async fn test_bot_provider_small_body_intact(pool: sqlx::PgPool) {
        let url = spawn_upstream(200, br#"{"ok":true}"#.to_vec(), vec![]).await;
        let provider_id = provider_with_link(&pool, &url).await;
        let link_id: Uuid =
            sqlx::query_scalar("SELECT id FROM bot_providers WHERE provider_id = $1")
                .bind(provider_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        let resp = test_bot_provider(&pool, &reqwest::Client::new(), link_id, "hi")
            .await
            .unwrap();
        assert_eq!(resp.body, r#"{"ok":true}"#);
        assert!(!resp.body.contains("truncated"));
    }
```

| case | expected |
|---|---|
| upstream 500 with 1 MB body (`test_provider`) | `Ok` starting `HTTP 500`, contains truncation marker, < 9 KB |
| upstream 200 with 1 MB body (`test_bot_provider`) | `body` truncated with marker, < 9 KB |
| upstream sets `set-cookie` + `x-upstream-secret` | neither appears in `resp.headers`; `content-type` does |
| upstream 200 with an 11-byte body | body returned byte-exact, no marker |
| oversized prompt (Task 9) | rejected before any request |

Command: `cargo test -p web --features ssr test_provider_truncates && cargo test -p web --features ssr test_bot_provider_filters && cargo test -p web --features ssr test_bot_provider_small_body`.

**Verification checkpoint:**
- [ ] Three new tests pass. If `spawn_upstream`'s axum handler shape does not compile against the pinned axum 0.8.9, copy the router/listener/serve mechanics verbatim from `rust/web/tests/ssr_pages.rs:104-128` (`spawn_mock_game_service`) and change only the handler body — do **not** weaken the assertions and do **not** replace the mock with a real endpoint.
- [ ] `grep -n "resp.text()" admin.rs` returns nothing, and `grep -n "resp.json()" admin.rs` returns nothing.
- [ ] `grep -n "unable to read body" admin.rs` returns nothing (both swallow sites are gone; `read_capped_body` reports read failures inline instead).
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.

**Commit:** `fix(admin): cap upstream test bodies and allowlist headers (ws F23)`

---

## Task 12: replace the ten `.unwrap()` completion Effects (ws F32)

**Problem (restated):** ten Effects share this shape. Live `Effect::new` .. `});` spans, in file order: **:1111-1122** (shown below), :1124-1135, :1137-1147, :1149-1159, :1423-1434, :1436-1447, :1449-1459, :1759-1770, :1772-1783, :1785-1795. The `.unwrap()` lines themselves are :1113, :1126, :1139, :1151, :1425, :1438, :1451, :1761, :1774, :1787.

```rust
    Effect::new(move |_| {
        if create_action.value().get().is_some() && !create_action.pending().get() {
            match create_action.value().get().unwrap() {
                Ok(_) => { ... }
                Err(e) => error.set(Some(e.to_string())),
            }
        }
    });
```

Three reads of the same signal, an `unwrap` re-justified by the guard above it, and the `pending()` read only exists to make the `unwrap` feel safe. Safe today, copy-prone forever.

**Fix (re-derived, and it is the in-repo pattern):** `settings.rs:62-69` (`UsernameSection`, `Effect::new` at :62, `if let Some(result) = save_action.value().get()` at :63) and `components/game.rs:550-562` (`GameCommandInput`, which uses the `let ... else { return; }` variant) already do it right:

```rust
    Effect::new(move |_| {
        if let Some(result) = save_action.value().get() {
            match result { ... }
        }
    });
```

Dropping the `!pending()` read is safe, verified against `reactive_graph-0.2.14/src/actions/action.rs`: inside `ArcAction::dispatch`'s spawned task, `value.update(|n| **n = Some(result));` runs **only** on the completion arm (:291, inside the `if is_latest` at :289-292, inside `result = fut =>` at :286), and `value` is **never** cleared on dispatch — the rustdoc example at :497-500 asserts exactly that ("dispatch another value, and it still holds the old value"). So an Effect tracking `value()` alone fires exactly once per completion, which is precisely when the old guard's condition became true. The Effect bodies only `set`/`update` other signals, which do not create tracking dependencies.

**Files:**
- Modify: `rust/web/src/admin.rs` (`BotsSection` x4, `ProvidersSection` x3, `BotProvidersSection` x3)

**Steps:**

- [ ] Rewrite all ten. Locate them by section (`grep -n "fn BotsSection\|fn ProvidersSection\|fn BotProvidersSection" admin.rs`) and by the action name in each. Each becomes, with **the same body it has today**:

```rust
    Effect::new(move |_| {
        if let Some(result) = create_action.value().get() {
            match result {
                Ok(_) => {
                    show_create.set(false);
                    error.set(None);
                    version.update(|v| *v += 1);
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        }
    });
```

  The ten sites, with the action and the success body to preserve verbatim:

| section | action | success body (unchanged) |
|---|---|---|
| `BotsSection` | `create_action` | `show_create.set(false); error.set(None); version.update(..)` |
| `BotsSection` | `update_action` | `editing_id.set(None); error.set(None); version.update(..)` |
| `BotsSection` | `delete_action` | `error.set(None); version.update(..)` |
| `BotsSection` | `reorder_action` | `error.set(None); version.update(..)` — same as `delete_action`; live body at :1149-1159, verified identical to the delete Effect apart from the action name |
| `ProvidersSection` | `create_action` | `show_create.set(false); error.set(None); version.update(..)` |
| `ProvidersSection` | `update_action` | `editing_id.set(None); error.set(None); version.update(..)` |
| `ProvidersSection` | `delete_action` | `error.set(None); version.update(..)` |
| `BotProvidersSection` | `create_action` | `show_create.set(false); error.set(None); version.update(..)` |
| `BotProvidersSection` | `update_action` | `editing_id.set(None); error.set(None); version.update(..)` |
| `BotProvidersSection` | `delete_action` | `error.set(None); version.update(..)` |

  **Do not change any `Err` arm** — `Err(e) => error.set(Some(e.to_string()))` stays as-is at all ten sites. (Yes, that renders the `Display` prefix; that presentation wart is out of scope, see Non-Goals. The Task 7/8/9 messages still read fine through it.)
- [ ] Leave the two `test_action` Effects alone — Task 13 owns them.
- [ ] `grep -c "value().get().unwrap()" admin.rs` must print **2** after this task (the two `test_action` Effects), and **0** after Task 13.

**Test plan:** these are client-side reactive changes with no SSR-observable surface, so there is no unit test to add. The verification is (a) the grep count, (b) clippy, and (c) the SSR page test from Task 2 still rendering the admin page for an admin.

| case | expected |
|---|---|
| `grep -c "value().get().unwrap()" admin.rs` | **2** (it is **12** before this task: the ten here plus the two `test_action` Effects at :1466 and :1802) |
| `grep -c "is_some() && !" admin.rs` | **0** (it is **10** before this task; the two `test_action` guards split `.is_some()` and `&& !...pending()` across :1462/:1463 and :1798/:1799, so they never matched this single-line pattern) |
| `admin_page_ssr_renders_for_non_admin_without_panicking` (Task 2) | still passes |
| hydration smoke (`rust/web/end2end/tests/page-loads.spec.ts`) | unaffected (does not cover `/admin`); do not extend it |

Command: `cargo clippy -p web --all-targets --features ssr -- -D warnings && cargo test -p web --features ssr admin`.

**Verification checkpoint:**
- [ ] Both grep counts as above.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.

**Commit:** `refactor(admin): if-let completion Effects instead of guarded unwraps (ws F32)`

---

## Task 13: key test results off the action's own value (ws F24)

**Problem (restated):** two Effects pair a *value* with an *input*:

```rust
    Effect::new(move |_| {
        if test_action.value().get().is_some()
            && !test_action.pending().get()
            && let Some(provider_id) = test_action.input().get()
        {
            let res = match test_action.value().get().unwrap() { ... };
            test_result.set(Some((provider_id, res)));
        }
    });
```

(`ProvidersSection` :1461-1472 and, with a `(bp_id, _)` destructure, `BotProvidersSection` :1797-1808.) The UI only blocks re-dispatch for the *same* id (`disabled=... input().get() == Some(id)` at :1509-1512, `.is_some_and(|(tid, _)| tid == id)` at :1870-1874), so two providers can be tested concurrently, and `input()` and `value()` are independent signals.

**Mechanism, re-derived (this is where the finding is ADJUSTED):** in `reactive_graph-0.2.14/src/actions/action.rs`, `ArcAction::dispatch`'s spawned task writes `value` at :291 and then clears `input` to `None` at :296 once `in_flight.get_untracked() == 0` (:295-297). So the finding's claimed cross-attribution — A's result stored under B's id — cannot happen as described; what happens instead is that the Effect's `let Some(..) = input().get()` guard can be `None` at the moment `value` lands, and the result panel silently never renders. Separately, the library's `is_latest` guard (:288) is **vacuous**: the `dispatched: ArcStoredValue<usize>` field (:104) is initialised with `Default::default()` (:209, :391), cloned into the task (:276, :329), and then only ever **read** via `get_value()` (:269, :288 in `dispatch`; :321, :340 in `dispatch_local`) — `grep -n dispatched action.rs` shows no `set`/`update`/`+= 1` anywhere, so `dispatched.get_value() <= current_version` is always `true`. `value` is therefore unconditionally overwritten by whichever dispatch completes last, regardless of order. **Under both readings the finding's recommendation is the correct fix**, and it is the only shape that is robust to either: put the id inside the value.

**Files:**
- Modify: `rust/web/src/admin.rs` (`ProvidersSection` and `BotProvidersSection`: `test_action`, its completion Effect, and the two pending-guard sites in each)

**Steps:**

- [ ] In `ProvidersSection`, make `test_action` return the id alongside the result. Building on Task 10's signature:

```rust
    // ws F24: the id travels with the result, so a completed test can never be
    // attributed to another row (and cannot be dropped because `input()` has
    // already been cleared by the time `value()` lands).
    let test_action = Action::new(|(id, model): &(Uuid, Option<String>)| {
        let id = *id;
        let model = model.clone();
        async move { (id, admin_test_provider(id, model).await) }
    });
```

  The action's output type is now `(Uuid, Result<String, ServerFnError>)`.
- [ ] Replace `ProvidersSection`'s test Effect with:

```rust
    Effect::new(move |_| {
        if let Some((provider_id, result)) = test_action.value().get() {
            let res = result.map_err(|e| e.to_string());
            test_result.set(Some((provider_id, res)));
        }
    });
```

- [ ] The two pending guards in `ProvidersSection` (the Test button's `disabled=` and its label closure) still read `input()`. `input()` is the right signal for "is *this* row currently testing" and it is *not* the bug — keep them, but they must match Task 10's tuple shape:

```rust
                                        disabled=move || {
                                            test_action.pending().get()
                                                && test_action.input().get()
                                                    .is_some_and(|(tid, _)| tid == id)
                                        }
```

  and identically inside the label closure. This is the exact shape `BotProvidersSection` already uses.
- [ ] In `BotProvidersSection`, do the same to `test_action`:

```rust
    let test_action = Action::new(|(id, prompt): &(Uuid, String)| {
        let id = *id;
        let prompt = prompt.clone();
        async move { (id, admin_test_bot_provider(id, prompt).await) }
    });
```

- [ ] Replace `BotProvidersSection`'s test Effect with:

```rust
    Effect::new(move |_| {
        if let Some((bp_id, result)) = test_action.value().get() {
            let res = result.map_err(|e| e.to_string());
            test_result.set(Some((bp_id, res)));
        }
    });
```

- [ ] Leave both `test_result` signal declarations unchanged — the stored shape is the same. Live: `ProvidersSection` :1421 `RwSignal::new(None::<(Uuid, Result<String, String>)>)`, `BotProvidersSection` :1757 `RwSignal::new(None::<(Uuid, Result<TestBotProviderResponse, String>)>)`. The `.map_err(|e| e.to_string())` in each new Effect is what converts the action's `Result<_, ServerFnError>` into the stored `Result<_, String>`, replacing the old two-arm `match`. All `test_result.with(...)` render sites (`ProvidersSection` :1547-1565, `BotProvidersSection`'s equivalent) are untouched.
- [ ] `grep -c "value().get().unwrap()" admin.rs` must now print **0**.

**Test plan:** client-side reactivity, no SSR surface. Verification is structural.

| case | expected |
|---|---|
| `grep -c "value().get().unwrap()" admin.rs` | 0 |
| `grep -c "test_action.input().get()" admin.rs` | **4**. It is **6** before this task: the two Effect reads at :1464 and :1800 (removed here) plus the four pending-guard reads at :1511, :1519, :1872, :1881 (kept). |
| every `test_result.set` call | its `Uuid` comes from the destructured action **value**, never from `input()` |
| manual check (optional, needs a dev stack) | with two providers configured to slow endpoints, click Test on A then B; each row shows its own result |
| `cargo clippy` | clean; in particular no type mismatch on the `Action` output type at the render sites |

Command: `cargo clippy -p web --all-targets --features ssr -- -D warnings && cargo test -p web --features ssr admin`.

**Verification checkpoint:**
- [ ] Both grep counts as above.
- [ ] `cargo test -p web --features ssr admin` — every admin test still passes.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.
- [ ] **Package gate:** `/home/beefsack/Development/brdgme/scripts/rust-test.sh` passes (this is the final task; run the full suite before committing).

**Commit:** `fix(admin): key provider test results off the action value (ws F24)`

---

## Cross-package / newly discovered

Not fixed by this package. Evidence recorded; routed per `work-packages.md`.

1. **`reactive_graph 0.2.14`'s `Action` `is_latest` guard is dead code (upstream library defect).** `ArcAction::dispatch` snapshots `let current_version = self.dispatched.get_value();` (`~/.cargo/registry/src/index.crates.io-*/reactive_graph-0.2.14/src/actions/action.rs:269`) and later gates the value write on `let is_latest = dispatched.get_value() <= current_version;` (:288, identically :340 in `dispatch_local`). `grep -n dispatched action.rs` shows the `ArcStoredValue<usize>` field (declared :104) is **only ever read** (`get_value()` at :269, :288, :321, :340) and otherwise only initialised (`Default::default()` at :209, :391) and cloned (:120, :276, :329) — never incremented — so `is_latest` is always `true` and every completing dispatch overwrites `value`, out-of-order results included. **Route:** third-party, not a brdgme defect; nearest owner is **WP-43 (web cargo deps)**. This package does not depend on the guard (Task 13 makes the id travel inside the value, which is correct under either behaviour). Worth an upstream leptos issue; not worth pinning or patching. **Already cross-referenced by `WP-54-frontend-ux-error-handling.md:1886`, which inherits the same hazard in its revert snapshots and also routes it to WP-43** — so this item is agreed across specs, not duplicated work.
2. **`AdminPage` renders raw `ServerFnError` `Display` text to users.** `admin.rs` :1039-1042 (`<p class="error">{e.to_string()}</p>`), :1024 (`let msg = e.to_string();`) and every `Err(e) => error.set(Some(e.to_string()))` arm (:1119, :1132, :1144, :1156, :1431, :1444, :1456, :1767, :1780, :1792) produce strings like `"error running server function: Internal server error"` (`server_fn-0.8.13/src/error.rs:233-234`). `crate::error::user_facing_server_error` (`error.rs:14-16`) exists for exactly this and is not used here.
   **Route — CORRECTED, this is no longer WP-54's.** This spec originally routed it to WP-54. **WP-54 has since explicitly refused it by LEAD RULING** (`WP-54-frontend-ux-error-handling.md:210` — "Do not open `admin.rs`. Do not add it to any file list."; restated in its cross-package list at :1885), on the grounds that `work-packages.md:423-431` is authoritative on WP-54's paths and does not list `admin.rs`. WP-54's own routing note says the correct home is **"its own small package, or a WP-37 follow-up"**, and observes that after WP-54's Task 1 lands, `crate::error::action_error_message` will exist and each of these sites becomes a one-line change.
   **Action for the Lead:** file it as a follow-up package (or a `docs/BACKLOG.md` item) sequenced **after WP-54 Task 1**, so it can reuse that helper instead of inventing a second one. Do **not** fix it inside WP-37 — Tasks 7/8/9 add user-facing messages that read correctly through the prefix, and stripping the prefix here would fork the helper.
3. **`bots.display_order` has no unique constraint.** Verified: `013_bot_efficacy.sql:4` declares `display_order INTEGER NOT NULL DEFAULT 0` with no `UNIQUE`, the table has no separate index DDL, and a grep for `UNIQUE`/`INDEX` across migrations 014-022 returns only `game_proposals`, `game_proposal_players`, `game_players`, `user_emails`, `processed_webhook_events` and `users(game_visibility)` — nothing on `bots`. After Task 6 the only writers are serialized, so duplicates are unreachable through the app — but nothing at the schema level enforces it, and a manual `UPDATE` or a future writer that forgets the advisory lock can still create them, silently making `list_bots`' `ORDER BY display_order` (:99) nondeterministic. Adding `CREATE UNIQUE INDEX` would require a new migration `023_*.sql` **and** a deferred/reorder-safe strategy (a straight renumber transiently collides). **Route:** schema hardening; ws F18 itself flags it as optional. Recommend the Lead file it as backlog rather than fold it into any current package.

3b. **`reorder_bots` accepts a partial list and can produce colliding orders.** Independent of the duplicate-id check Task 6 adds: `reorder_bots(vec![a, b])` on a three-bot table renumbers `a`/`b` to 0/1 and leaves `c` wherever it was, which can collide. Unreachable from the UI (the Up/Down buttons always send the complete `bot_ids` list built at :1161), and rejecting partial lists would be a behaviour change outside ws F18/F19. **Route:** fold into the same schema-hardening/backlog item as (3) — a unique index would surface it, and a "must cover every bot" server check is the alternative. **Not fixed here.**
4. **`llm_providers.api_key_encrypted` can hold an empty-string ciphertext.** `create_provider` (:265-311) accepts `Some("")` and encrypts it: the `match &api_key` at :271 only distinguishes `Some`/`None`, never emptiness. The create form filters empty **client-side** only, and Task 9 deliberately does not validate `api_key`, which is legitimately optional. After Task 4 that renders as `(set)` (length 0 < 8) while `test_provider` sends a bare `Authorization: Bearer ` (:518) and fails confusingly. One-line fix (`filter(|k| !k.trim().is_empty())` in `create_provider`) but it is a behaviour change outside all 14 findings. **Route:** flag to the Lead for this package's follow-up or backlog; **do not fix in-flight.** Note the interaction with Task 8: `ApiKeyUpdate::Set("")` on *update* has the same hole, and the same one-line fix would cover both.
5. **`test_provider`/`test_bot_provider` will `POST` to any admin-supplied URL, including internal addresses.** Task 9 restricts the scheme to http/https; it does not block `http://169.254.169.254/...` or cluster-internal hosts. This is admin-gated by design (the whole point of the feature is configuring arbitrary LLM endpoints), so it is a documented trust boundary rather than a defect — but it is worth stating explicitly. **Route:** note for the review's threat-model summary; no code change proposed.
