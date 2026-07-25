# WP-37 admin.rs pass - adversarial review of the unreviewed draft

Reviewer: Worker (Lead-delegated adversarial review), 2026-07-25.
Target: `planning/specs/WP-37-admin-pass.md` (was 2255 lines / 13 tasks; now 2355 after repairs).
Method: read-only. Live source at `/home/beefsack/Development/brdgme/rust`, review snapshot at
`/home/beefsack/Development/brdgme-review-snapshot/rust`, vendored crates under
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`. No cargo/build/test command run,
no file under `rust/` touched, no git mutation.

## Verdict

**ACCEPT-AFTER-MY-REPAIRS.** The draft's *reasoning* is sound - every one of the seven
load-bearing judgements I was asked to re-derive independently holds up, and two of them
(ws F30 rejection, ws F31 Display-prefix) are correct in ways the original findings were not.
What was wrong was (a) roughly a third of the line-number citations, (b) three concrete
defects in the test code that would have failed or not compiled, and (c) one stale
cross-package routing. All are repaired in place. Nothing needs a user decision beyond the
already-labelled ws F21 assumption block.

## 1. Load-bearing claims re-derived (the seven I was asked about)

| # | Claim | Live evidence | Verdict |
|---|---|---|---|
| 1 | ws F30 SKIPPED: `bot_providers` has NO `updated_at` | `013_bot_efficacy.sql:23-34`: `CREATE TABLE bot_providers` runs :23-34, has `created_at` at :32, then `UNIQUE (bot_id, provider_id, model)` at :33, `);` at :34. No `updated_at`. `grep -n "bot_providers\|updated_at\|display_order\|UNIQUE\|INDEX" 01[4-9]*.sql 02*.sql` returns only `game_proposals`/`game_players`/`user_emails`/`processed_webhook_events`/`users(game_visibility)` hits - nothing on `bots`/`bot_providers`. `022_concede_bot_replacement.sql:16` is the only later `bots` DDL. | **CONFIRMED.** Rejection is correct; adding `updated_at = now()` would be a runtime SQL error, and because this file uses runtime-checked `sqlx::query` (0 macros), nothing would catch it at compile time. |
| 2 | ws F22: no `model` column on `llm_providers`/`bots`; dropping the `gpt-4o-mini` fallback is right | `grep -rn "model" rust/web/migrations/*.sql` = exactly 4 hits: `013:27` (`model TEXT NOT NULL` in `bot_providers`), `013:33` (the UNIQUE), and two prose comments about the Rust `User` *model struct* (`010_friends.sql:11`, `021_add_game_visibility.sql:2`). `013:13-21` `llm_providers` has no model column; `013:1-11` `bots` has none. Literal at `admin.rs:510`. | **CONFIRMED.** The recommendation's "with a fallback" half is correctly overturned: keeping `gpt-4o-mini` preserves exactly the false negative the finding reports. |
| 3 | ws F24: `input()` clears when `in_flight` hits 0 | `reactive_graph-0.2.14/src/actions/action.rs`: `ArcAction::dispatch` writes `value.update(...)` at :291 inside `if is_latest` (:289-292) inside the `result = fut =>` arm (:286); then `if in_flight.get_untracked() == 0 { input.update(\|inp\| **inp = None); }` at :295-297. `dispatch_local` is identical at :330-350. Rustdoc at :497-500 confirms `value` is never cleared on re-dispatch. | **CONFIRMED.** The draft's cited range ":287-297" is right in substance; I tightened it to the exact write line (:291) and clear range (:295-297). The *bonus* claim - `is_latest` is vacuous because `dispatched` is never incremented - also **CONFIRMED**: `grep -n dispatched action.rs` shows declaration :104, `Default::default()` :209/:391, clones :120/:276/:329, and reads only at :269/:288/:321/:340. No `set`/`update`. |
| 4 | ws F31: exact equality on `e.to_string()` can never match | `server_fn-0.8.13/src/error.rs`: enum at :170 with `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]` at :165; `ServerError(String)` variant at :186; `ServerFnError::new` -> `Self::ServerError(msg.to_string())` at :201-203; `Display` arm `format!("error running server function: {s}")` at :233-234; `ser` writes `"ServerError\|{e}"` :281-283; `de` reconstructs :331-333. | **CONFIRMED**, all five citations accurate (I corrected ":200-203" to ":201-203"). The variant-match fix is the right call and the `Debug` derive means the new unit test's `{other:?}` compiles. |
| 5 | `bots.display_order` has no unique index | `013_bot_efficacy.sql:4` = `display_order INTEGER NOT NULL DEFAULT 0` - no `UNIQUE`. No index DDL for `bots` anywhere in 001-022. | **CONFIRMED.** |
| 6 | WP-54 does not include `admin.rs` | `work-packages.md:423-431` paths = `web/src/{friends.rs,new_game.rs,settings.rs,app.rs}` + `web/src/components/{game.rs,layout.rs,opponent_slot.rs,mod.rs}`. `WP-54-frontend-ux-error-handling.md:210` is an explicit **LEAD RULING**: "Do not open `admin.rs`. Do not add it to any file list." | **CONFIRMED** - and stronger than the draft knew. See refutation R6 below for the consequence. |
| 7 | WP-41's `is_user_admin` change is caller-compatible; 15 sites -> 1 | `error.rs:7` = `pub fn internal<E: std::fmt::Display>(...)`, so `anyhow::Error` satisfies it. 15 production sites at `admin.rs:640, 665, 700, 729, 748, 767, 790, 815, 834, 853, 879, 914, 942, 962, 985` all use `.map_err(internal(...))?`. **The 16th site, `admin.rs:2201`, is `crate::db::is_user_admin(&pool, user_id).await.unwrap()`** inside `mod tests`. WP-41's own spec records this at :93 and :128 (not :92/:127 as the draft cited) and notes it compiles because `anyhow::Error: Debug`. | **CONFIRMED with a correction.** The draft's phrasing "every `admin.rs` site does `.map_err(internal(...))?`" is **false** for :2201. The *conclusion* survives (`unwrap` needs only `E: Debug`), and WP-37 Task 1 correctly leaves :2201 alone - which is why its grep expects `is_user_admin` count 2, not 1. Repaired with the full two-shape argument. |

## 2. Snapshot drift claim

`diff -u <snapshot>/web/src/admin.rs <live>/web/src/admin.rs` exits 1 with **26** hunks
(`grep -c '^@@'`), not 21. Every changed line filters out under
`grep -iv "can_replace_humans\|replace"` to `can_replace_humans`-adjacent refactors
(`BotDbRow`/`BotCreateAction` alias extraction, `#[allow(clippy::too_many_arguments)]`,
the threaded column, the new checkbox). So the *characterisation* is right; the count was wrong.

The draft also claimed "**No finding site in this package was modified**". That is **false**:
`create_bot`'s INSERT (ws F19), `update_bot`'s UPDATE (ws F29), `list_bots`' SELECT and
`BotsSection`'s two action closures all changed text. The *defects* are untouched, but a
"before" block copied from a findings doc would no longer match. Repaired with an explicit
"take every before-block from the live file" instruction.

## 3. Citation audit

I checked **every** file:line citation that an edit step depends on, plus every cross-file
citation. Count: **~95 citations checked, 34 wrong** (all off-by-1-to-9 within the right
function, except the four listed as refutations below). None of the *symbol-level* locations
was wrong, which is what saves the spec - and the spec's own Global Constraint already tells
the implementer to locate edits by `grep -n "fn <name>"` rather than by line number.

Corrected (non-exhaustive, all repaired in place):
`BotRow` 35->36, `ProviderRow` 46->48, `BotProviderRow` 54->57, `TestBotProviderResponse`
69->71, `BotDbRow`/`ProviderDbRow`/`BotProviderDbRow` 77/79/81->79/81/83,
Task 1 insertion point 34->35, `friends::require_user` 88-92->86-91,
`AdminPage` redirect Effect 1026-1033->1022-1029, full-page error render 1036-1041->1039-1042,
`list_providers` mask 243-252->241-249, `list_providers` decrypt `?`s 243-247->237-240,
`load_key` 229->231, `create_bot` MAX+1 143-146->143-145, `reorder_bots` 197-211->197-210,
`update_bot` execute 188->190, `update_provider` executes 335/347->336/348,
`update_bot_provider` execute 465->468, `delete_bot` 215->214/216,
`delete_bot_provider` 477->476/478, `ProvidersSection` error slot 1476->1478,
`ProviderEditForm` filter 1636-1642->1639-1642, help text 1655->1657,
`test_provider` error path 530-534->524-531, success path 536-539->533-536,
`test_bot_provider` body 625-628->618-621, headers 618-622->613-617,
bots Delete button 1204-1213->1226-1233, ws F25 `required` refs 1275/1646/2138 ->
the real ten sites (1289/1350/1604/1607/1652/1655/2031/2041/2051/2141) + temperature
min/max at 1292/1357-1358, the four pre-existing test line refs (2183->2184, 2202->2205,
2232->2230, 2258->2256), `settings.rs` 62-68->62-69, `error.rs` `internal` 7-13->6-12,
`user_facing_server_error` 15-17->14-16, WP-41 spec 92/127->93/128,
ten Effect block spans 1109-1118 etc -> 1111-1122 etc, `test_prompt` markup 1817->1816.

Citations verified **exactly correct** and left alone (worth knowing, since they anchor the
riskiest tasks): all 15 `is_user_admin` gate lines; the test-only site :2201; all ten ws F32
`.unwrap()` lines (1113/1126/1139/1151/1425/1438/1451/1761/1774/1787); both ws F24 Effects
(1461-1472, 1797-1808); both `test_action` `.unwrap()`s (1466, 1802);
`BotProvidersSection`'s `is_some_and` guard shape (1871-1874); `create_provider`'s mask
closure (292-302); the local `BotProviderRow` alias (551-557); ws F30's target statement
(459-461); `mod tests` (2176-2336); `components/game.rs:550-562`; the "0 sqlx macros /
24 runtime-checked call sites" claim (`grep -c "sqlx::query"` = 24, macro grep = 0).

## 4. Refuted claims

- **R1. "21 hunks."** Actually **26**. Repaired.
- **R2. "No finding site in this package was modified" by the drift.** False for four sites. Repaired.
- **R3. "Every `admin.rs` site does `.map_err(internal(...))?`" (WP-41 compatibility argument).** False: `:2201` uses `.await.unwrap()`. The conclusion still holds via `anyhow::Error: Debug`. Repaired with both shapes spelled out.
- **R4. `rust/web/src/game/client.rs` is "the repo's established in-process mock-server pattern".** **That file does not exist.** Cited twice in Task 11 (test plan and the verification fallback instruction), so an implementer following the fallback would have been stuck. The real pattern is `tests/ssr_pages.rs:104-128` (`spawn_mock_game_service`). Repaired, including the fallback instruction.
- **R5. `tests/ssr_pages.rs` "spins a `TcpListener` + `axum::Router` at :17-31".** :17-31 is the import block. The spawn is :104-128. Repaired.
- **R6. Cross-package item 2 routed to WP-54.** WP-54 has since **refused** it by LEAD RULING (`WP-54-...md:210`, restated :1885) and re-routed it to "its own small package, or a WP-37 follow-up", noting `crate::error::action_error_message` will exist after WP-54 Task 1. The draft's routing is stale. Repaired: item 2 now records the refusal, cites it, and gives the Lead a concrete sequencing action.
- **R7. Task 2's grep step was self-contradicting**: "must return nothing (... confirm the only hit is `pub const ADMIN_REQUIRED`)". The const initialiser contains the literal, so the grep returns 1. Repaired to "exactly one hit, and it must be the const".
- **R8. Task 1's `grep -c "Admin access required"` must print 1 after Task 1.** False - `:1025`'s `msg.contains("Admin access required")` survives until Task 2, so the count is 2 after Task 1 and 1 after Task 2. Repaired with before/after counts for all three greps.
- **R9. Task 5's test-plan row "`load_key()` unavailable -> still Err".** `crypto::load_key` (`crypto.rs:53-63`) **cannot** fail when `DATABASE_ENCRYPTION_KEY` is unset - it returns `default_key()`. It errors only on a set-but-malformed value. Repaired, with the reason it stays untested.

## 5. Defects in the spec's own test code (all would have failed / not compiled)

- **D1 (would have FAILED at runtime). Task 1's `every_admin_server_fn_calls_require_admin` self-matches.** It does `include_str!("admin.rs")` and counts the literals `"#[server(Admin"` and `"require_admin(&pool,"`. Both appear *in the test's own source*, and `"#[server(Admin"` appears a **third** time in the test's doc comment - so the counts would come out 17 vs 16 and the assert would fail. It also silently broke the sibling `grep -c "require_admin(&pool," == 15` check (would be 16). **Repaired:** needles assembled with `concat!`, doc comment reworded to avoid both patterns, plus an added `assert_eq!(server_fns, 15)` so a *matched* pair of deletions cannot pass silently, plus an explicit note not to collapse the `concat!`s.
- **D2 (would NOT COMPILE). Task 2's SSR test calls `build_router(state).oneshot(...)`.** `build_router` is **async** in this repo - every live call site is `build_router(make_state(pool).await).await` (`ssr_pages.rs:1184`, :1205). **Repaired**, and rewritten to use the file's own `get(app, path, cookie)` helper (:184) instead of hand-rolling `Request::builder`/`oneshot`.
- **D3 (VACUOUS test presented as verification). Task 2's assertion `!body.contains("Add Provider")` proves nothing.** `AdminPage`'s data comes from `LocalResource`s, which do not load during SSR, so `/admin` server-renders the `<Suspense fallback>` `"Loading..."` (:1035) for admins and non-admins alike. The assertion passes today, passes after the change, and would pass with the redirect deleted. **Repaired:** downgraded and relabelled as a panic/500 smoke test, with an explicit callout that the redirect is not SSR-observable, the unit test is the real pin, and a manual check is added to the test plan. Also flagged that `make_state` needs a live NATS (`ssr_pages.rs:35-44`), so it must run via `scripts/rust-test.sh`.
- **D4 (unsound behaviour, not caught by the draft). `reorder_bots` with a duplicated id.** The single-statement `UPDATE ... FROM unnest(...) WITH ORDINALITY` matches one `bots` row from two ordinals; Postgres applies one of them, unspecified which, and `rows_affected` still equals `distinct.len()`, so the draft's check passes and the resulting order is nondeterministic. **Repaired:** an explicit duplicate-id rejection before the transaction opens, a matching `#[sqlx::test]`, and a test-plan row. The related *partial-list* hazard is genuinely out of scope and is routed as new cross-package item 3b rather than fixed.
- **D5 (doubtful compile). Task 11's `Router::fallback(axum::routing::any(closure))`.** `Router::fallback` takes a `Handler`; `any(...)` yields a `MethodRouter`, which is a `Service`, not a `Handler` (that is what `fallback_service` is for). **Repaired:** rewritten to `route("/v1/chat/completions", post(...))`, which is both the proven in-repo shape and sufficient, since both test fns POST to exactly that path (:517, :604). Also switched from `into_response()` + `headers_mut().insert` to a `(StatusCode, HeaderMap, Vec<u8>)` tuple, documented why the closure clones rather than moves its captures (axum `Handler` needs `Clone`), and confirmed `axum`/`tokio::net` availability.

## 6. Other quality-bar gaps closed

- Resolved Task 5's open instruction "Confirm `crypto::decrypt`'s error type is `Display`" - it is (`CryptoError` is `#[derive(Debug, Error)]`, `crypto.rs:5-15`), so the conditional `{e:?}` branch is deleted rather than left as a decision for the implementer.
- Justified Task 5's corrupt-row fixture from source: `decrypt` returns `DecryptionFailed` for any input under 12 bytes (`crypto.rs:31-33`), so `vec![0,1,2,3]` fails regardless of key.
- Added the `llm_providers`-is-not-seeded fact that makes Task 5's `assert_eq!(providers.len(), 3)` exact.
- Task 4: replaced "verify with grep whether `api_key` is used later" with the verified answer (read at :271 and :292, not after; fn ends :311).
- Task 6: documented the `&Vec<Uuid>` -> `uuid[]` bind (with `.bind(&ordered_ids[..])` as the fallback and a warning not to move the vec), the bigint->int4 assignment cast for `o.ord - 1`, why the lock precedes the UPDATE, and the empty-vec no-op.
- Task 7: rewrote the site table to give both the enclosing function span and the exact `.execute()` line, and added the accepted presentation caveat (messages read correctly through the `Display` prefix).
- Task 8: pinned the enum's insertion point, stated it must **not** be `#[cfg(feature = "ssr")]` (it crosses the wire), noted `create_provider`/`ProviderCreateForm` keep `Option<String>`, and gave exact line spans for the two existing test edits. Added a source-verified coherence check for the STATED ASSUMPTION - `Keep`/`Set` are faithful renames of today's `None`/`Some` (`:1642`, `:340-351`, help text `:1657`), so `Clear` is purely additive and no existing behaviour or test semantics change.
- Task 9: replaced the wrong `required` citations with the real ten, and added two source-checked behaviour notes the draft omitted - the `reasoning_effort` non-empty check is **unreachable from the UI** (both forms already `.filter(|v| !v.trim().is_empty())` at :1992-1995 and :2099-2102, so no regression), while the `extra_body` object check **is** reachable and **is** a user-visible change (both forms accept any parseable JSON at :1997-2011 / :2104-2118, and a non-object is silently ignored today by the merge at :590-591).
- Task 10: added the explicit Task-9-before-Task-10 dependency (it uses `require_text`), pinned the insertion region and the untouched `load_key` block, and gave the four guard/dispatch line spans.
- Task 11: documented the `read_capped_body` exactly-at-MAX edge case as accepted, with a "do not fix by reading one extra chunk" instruction; added `resp.json()` and `"unable to read body"` greps to the checkpoint; noted the possible duplicate `content-type` from the mock and why it is harmless.
- Task 12: filled in the one placeholder success body ("read the live body at :1148-1160 and keep it" -> the verified `error.set(None); version.update(..)`), and gave before/after counts for both greps so a wrong baseline cannot masquerade as success.
- Task 13: pinned both `test_result` declarations by line and type and explained that `.map_err(|e| e.to_string())` is what replaces the old two-arm match; gave the before/after breakdown of the `input().get()` count (6 -> 4, with all six lines listed).
- Architecture section: added the fact that each of the five `*Action` aliases is a component prop type exactly once, with lines - which is what makes Task 8's alias change a two-site edit rather than one.

## 7. Not closed (Lead's call, deliberately left alone)

1. **The ws F21 STATED ASSUMPTION itself.** It is a product decision (blank = keep, explicit checkbox = clear, checkbox wins over typed text). I verified it is *coherent* with the live form and update path and added that evidence, but I did not resolve it - it stays a labelled assumption with its rejected alternative. If the Lead wants the other semantics (blank = clear), Task 8 changes materially.
2. **Cross-package item 2's new home.** WP-54 refused it; WP-37 must not fix it. Whether it becomes a new small package or a `docs/BACKLOG.md` item, and its sequencing after WP-54 Task 1, is a Lead decision. I recorded the options and the evidence, and invented no resolution.
3. **Cross-package items 3/3b/4/5** stay routed, not fixed, exactly as the draft intended. Item 3b (partial reorder list) is new.
4. **Whether `reorder_bots` should require the complete bot list.** Unreachable from the UI, a behaviour change outside ws F18/F19. Routed as 3b; not decided.
5. **Model-resolution vs API-key-check ordering in Task 10.** As specified, a provider with neither a key nor a link now reports "no configured model" instead of "no API key configured". Harmless and arguably better, but it is a message-ordering change I did not silently reorder; not worth a decision unless the Lead disagrees.

## 8. Three fast spot-check anchors for the Lead

1. `rust/web/src/admin.rs:2201` should read
   `        let is_admin = crate::db::is_user_admin(&pool, user_id).await.unwrap();`
   - the WP-41-compatibility site the draft mis-described as using `.map_err`. Task 1 must
   leave it alone, which is why its grep expects 2.
2. `rust/web/migrations/013_bot_efficacy.sql:33` should read
   `    UNIQUE (bot_id, provider_id, model)`
   as the **last** entry before `);` at :34 - i.e. `bot_providers` really has no
   `updated_at`, so ws F30 stays rejected.
3. `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/reactive_graph-0.2.14/src/actions/action.rs:296`
   should read `                        input.update(|inp| **inp = None);`
   inside `if in_flight.get_untracked() == 0 {` at :295 - the ws F24 mechanism adjustment.

Bonus fourth, for R4: `ls rust/web/src/game/client.rs` fails; the mock pattern is
`rust/web/tests/ssr_pages.rs:104` `async fn spawn_mock_game_service() -> String {`.

## 9. Compliance

- **No file under `rust/` was created, modified or deleted.** Reads only.
- **No cargo/build/check/test/clippy/fmt/sqlx command was run.** All compile-level judgements
  are reasoned from source and from the vendored crate sources; where I could not be certain
  from reading alone (the axum handler shape in Task 11), the spec now says so and gives a
  proven in-repo fallback rather than asserting it compiles.
- **No git mutation was run.** The only git-adjacent command was a read-only `diff -u`
  between the working tree and the review-snapshot directory.
- Writes confined to `planning/specs/WP-37-admin-pass.md` and this notes file.
