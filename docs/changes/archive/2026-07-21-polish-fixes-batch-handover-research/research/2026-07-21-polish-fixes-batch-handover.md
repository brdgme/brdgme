# Polish/Fixes Batch - Self-Contained Implementation Handover

Date: 2026-07-21

This is a self-contained handover to implement 11 polish/fix items in the
`brdgme` repo (crate `rust/web`, Axum + Leptos SSR/WASM, plus `rust/bot`).
All design decisions are resolved below - there are no open questions. Implement
in the unit order given (Units 1-10). Every file:line reference was verified
against the working tree on 2026-07-21. A fresh session needs only this document.

---

## How to use this doc

- Implement Units 1-10 **in order**. Each unit is independently verifiable.
- Unit 6 (history page) depends on Unit 5 (`rating_before` column) being done
  first - the match-ELO column reads the stored value. All other units are
  independent.
- After each unit, run that unit's verification, then the global verification.
- **Do NOT commit unless the user explicitly asks.** Leave changes in the
  working tree for review.
- Read only the **Global conventions** section + the unit you are implementing;
  each unit is written to be actionable on its own.

---

## Global conventions & constraints

Folded from `docs/CODING.md`, `docs/DEV.md`, `AGENTS.md`. These apply to every
unit.

### Rust / error handling
- **No panicking code in runtime paths.** No `.unwrap()`/`.expect()`/`panic!`/
  `unreachable!()` in server fns, DB fns, or component code. Panics allowed only
  at process startup, in `#[cfg(test)]`, and in `#[cfg(not(feature="ssr"))]`
  client stubs. Propagate with `?`; convert `Option` via `ok_or_else(...)`.
- **DOM access in handlers:** `NodeRef::get()` is `Option` - `.map(...)`, never
  `.unwrap()`.

### Leptos SSR / hydration (structural stability)
- Hydration checks **element type + hierarchy**, NOT attributes/classes/styles.
  - Structural differences (different element types, presence/absence) => panic.
  - Attribute/class differences => safe (reactive bindings update after).
- **Toggle classes/attributes on always-present elements; never swap element
  types on async state.** (`if cond { <input/> } else { <span/> }` on async data
  panics; use `<input hidden=move || !cond/>`.)
- **Core page content:** `Resource::new_blocking` + `<Suspense
  fallback=|| view!{<div></div>}>` **inside** `<MainLayout>` (layout outside
  Suspense, data-dependent content inside). Read the resource unconditionally
  before any branching in the Suspense closure. Templates: `PlayersPage`
  (`players.rs:145`), `PlayerGameTypePage` (`players.rs:442`).
- **Non-core data:** `LocalResource` (always `None` on SSR; cannot mismatch).
- A resource must be created in the component (or direct ancestor) owning the
  Suspense that tracks it - don't pass resources via context.
- See `docs/hydration.md` before touching Suspense/Transition/resource structure.

### Forms
- **`FormField` for every new form control** (`components/form.rs`): bold block
  label + optional help + optional red error (`.form-field`/`.form-label`/
  `.form-control`/`.form-help`/`.form-error`). Button rows use `.form-actions`.
- **Save model:**
  - Rejectable fields (e.g. username: format/uniqueness) => explicit Save button
    + inline error via `FormField`'s `error` slot. (`UsernameSection`.)
  - Choice-among-valid-options fields (theme, colours, on/off toggles) => save
    fire-and-forget on change, no Save button, no inline error. (`ColorsSection`
    `settings.rs:112`, `ThemeSection` `settings.rs:408`.)
- **`prop:value` on the `<select>`** to keep it in sync (not per-`<option>
  selected`). (`ColorsSection:159`.)
- **`class:selected`** is the reactive-highlight pattern for tile/chip pickers
  (always-present element, one toggled class). (`ThemeSection`.)
- "Adopt once" idiom: a section copies server data into local signals once via an
  `initialized` `RwSignal`, then the signal is the source of truth
  (`ColorsSection:124-132`).

### Server functions
- **Guard logged-in-only server fns with `get_current_user`, inline:**
  `get_current_user().await?.ok_or_else(|| ServerFnError::new("Not
  authenticated"))?` (see `get_settings`/`set_pref_colors` in `auth/server.rs`).
- **Expected rejections are data / typed `UserError`, NOT raw `ServerFnError`.**
  `set_username` returns `Result<Option<String>, ServerFnError>` (`Ok(None)` =
  success, `Ok(Some(msg))` = field error). `Err` = real failure. Typed-error
  example: `ExecuteCommandError::UserError` (`game/mod.rs:66,72`) -> mapped to
  `Ok(Some(msg))` (`game/server_fns.rs:313`) -> client renders `Invalid command:
  {msg}` (`components/game.rs:549`). Opaque-infra helper: `internal()`
  (`error.rs:6`).
- **Map Postgres unique-violation (SQLSTATE 23505) in the DB helper**, not the
  server fn.

### SQL / migrations
- **PLAIN (non-macro) sqlx queries for new columns** (`sqlx::query`/`query_as`,
  not `query!`/`query_as!`) to avoid `.sqlx` offline-cache regeneration (no local
  DB to `cargo sqlx prepare` against). Precedent + rationale: `db.rs:254-264`.
  Existing `query!` macros are fine to REUSE as-is; only NEW queries go plain.
- **When a macro query IS added/changed**, regenerate per `docs/DEV.md`:
  `cd rust/web && sqlx migrate run && cargo sqlx prepare -- --features ssr`
  (use `--tests --features ssr --all-targets` for full coverage). Verify with
  `SQLX_OFFLINE=true cargo check --features ssr`.
- **Migrations are IMMUTABLE once applied.** Never edit an existing migration
  file (not even comments - sqlx checksums contents). New work goes in a NEW
  numbered file. Latest migration is `016_invite_nudge.sql`.
- `games.updated_at` is trigger-maintained (overwritten on every UPDATE). Tests
  backdating it must `ALTER TABLE games DISABLE TRIGGER update_games_updated_at`.

### Testing
- **`db.rs`, `game/mod.rs`, and `auth/` changes REQUIRE `#[sqlx::test]` tests.**
- **`#[sqlx::test]` for anything touching the DB** (isolated migrated DB per
  test; no shared fixtures, no ordering dependence).
- **Mock the game service; NEVER call the real game service or the LLM in a
  test.** In-process Axum mock returning canned JSON (`game/client.rs` pattern;
  `spawn_mock_new_game_service` in `tests/ssr_pages.rs:389`).
- **In-process SSR page tests** in `tests/ssr_pages.rs` via
  `web::router::build_router` + `tower::ServiceExt::oneshot`. Assert 200,
  `text/html`, a page marker, no SSR panic. Server-fn POSTs use
  `<Fn as leptos::server_fn::ServerFn>::PATH` with an
  `application/x-www-form-urlencoded` body (Leptos 0.8 encodes `#[server]` args
  via serde_qs url-encoded POST). Template: `restart_game_via_http`
  (`ssr_pages.rs:428`) + `restart_game_on_finished_game_succeeds`
  (`ssr_pages.rs:457`). Shared helpers: `make_state` (:35), `make_user` (:57),
  `login_cookie` (:72), `make_game_version` (:130).
- **Known issue:** DB-dependent tests fail in a plain local/agent run (no
  Postgres). This is pre-existing (backlog #40), NOT a regression - do not chase
  it. They run in CI / `scripts/rust-test.sh`.

### Verification commands
- Before any commit (full CI suite, several minutes): `bash scripts/rust-test.sh`
  (spins up temp Postgres + NATS, runs migrations, fmt + clippy + tests).
- At minimum:
  - `cargo fmt --all -- --check`
  - `cargo clippy -p web --all-targets --features ssr -- -D warnings`
  - `cargo clippy --workspace --exclude web --all-targets -- -D warnings`
- **Target `-p <crate>` only; never workspace-wide builds** (they link ~30
  binaries and spike RAM/disk). `web` always needs `--features ssr` (no default
  features).
- Do not background-poll CI runs - the user watches CI.
- Never start `tilt`/kind on a machine with <32GB RAM.
- `web` crate: `cargo test -p web --features ssr` needs a running Postgres at
  `DATABASE_URL` to pass at runtime.

---

## Resolved decisions (source of truth)

| Item | Resolved design |
|------|-----------------|
| 1 - Profile colour bar under username | CSS-only: `.profile-header h1 { margin-bottom: 0; }` in the profile section of `main.scss`. Margin is the browser UA default for `<h1>` (no heading reset exists). Also tightens the per-game-type header (`players.rs:512`) - accepted, harmless. |
| 2 - Unfriend confirmation | Wrap the unfriend dispatch (`friends.rs:440-443`) in a native `web_sys::window().confirm_with_message(...)` guard, matching the in-file "Decline and block" pattern (`friends.rs:412`) and force-delete-game (`components/game.rs:166`). Clone `name` into the closure. Handler-only; no server-fn change. Scope to unfriend ONLY. Message: "Unfriend {name}? You'll need to send a new friend request to add them again." |
| 3 - "invite" copy | Display-only, no DB change (stored value is the slug `open`/`friends`/`none`; DB CHECK on slugs in `010_friends.sql`). Reword the 3 `INVITE_POLICIES` labels (`friends.rs:12-15`) "...add me to a game" -> "...invite me to a game"; reword the section heading (`friends.rs:498`) to "Who can invite me to games"; reword the "Decline and block" dialog (`friends.rs:412`) "...or add you to games" -> "...or invite you to games". Invite email already says "invited you to play" (`proposals.rs:206`) - no email change. This section is on the `/friends` page (`friends.rs`), NOT `settings.rs`. |
| 4 - Can't start new games (bug) | Root cause: `create_proposal` (`proposals.rs:879-884`) declares `opponent_ids`/`opponent_emails`/`bot_slots` as bare `Vec<T>` with no `#[serde(default)]`; serde_qs url-encoded POST omits empty Vecs => "missing field" deserialization error for any game with no email invitees (the common case). Fix: add `#[serde(default)]` to all THREE Vec params; if the server macro won't forward the attribute, fall back to `Option<Vec<T>>` + `.unwrap_or_default()` (proven by `create_new_game` `server_fns.rs:461,480-482`) and update the form call site (`new_game.rs:208`). Plus: shared CLIENT helper `user_facing_server_error` mapping technical/framework errors to a generic string ("Something went wrong, please try again"); apply at the new-game form (`new_game.rs:445`) at minimum. Plus: regression test in `tests/ssr_pages.rs` POSTing to `CreateProposal::PATH` with `opponent_emails` OMITTED (fails pre-fix, passes post-fix). Plus: verify whether `create_new_game` is now dead code; if so, remove it. |
| 5 - Numeric form lines + medal colours | No data change (`FormResult.place` already carried, `stats/mod.rs:81`). Rewrite `form_cell` (`stats/viz.rs:43`) to numeric-only + medal class, DROP the dead `player_count` param: Some(1)->("1","form-gold"), Some(2)->("2","form-silver"), Some(3)->("3","form-bronze"), Some(p)->(p,"form-other"), None->("-","form-none"). Update `FormStrip` call site (`viz.rs:66`). CSS (`main.scss:651-665`): gold=`--mk-yellow` (1), silver=`--mk-grey` (2), bronze=`--mk-red` (3), other=`--mk-foreground`, none=`--mk-blue`. Both render sites (profile `players.rs:311`, game panel `components/game.rs:239`) share `FormStrip` => both fixed. Update tests `viz.rs:317-337` + `ssr_pages.rs:941`. |
| 6a - Store `rating_before` (migration 017) | Single NULLABLE column `game_players.rating_before integer`. `rating_after = rating_before + rating_change` (derivable, don't store). NULL for bots, single-human/all-bot games, zero-change seats (keeps `rating_before IS NOT NULL` == `rating_change IS NOT NULL`). Migration 017: `ADD COLUMN IF NOT EXISTS` + idempotent backfill reusing the migration-011 running-sum with window frame `ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING` (before, not after), guarded `WHERE rating_before IS NULL AND rating_change IS NOT NULL`. Write path: ONE function `apply_rating_changes` (`db.rs:1348`); pre-game rating already in scope (`RatedPlayer.rating`, loaded `db.rs:1409`); build position->rating map, write `rating_before` alongside `rating_change` in the final UPDATE loop (`db.rs:1472-1484`). Zero extra queries. No new index. Do NOT modify `game/import.rs` (new imports NULL - accepted). Do NOT rewrite `rating_series`. |
| 6b - Full game history page | Route `/players/:name/history` (declare static `history` route BEFORE the `:game_type` param route in `app.rs`). Page size 50. Filters as SHAREABLE QUERY PARAMS: `?page=&status=&type=` (status: finished/active/all, default all; type by name), honour `?bots=1`. Default sort `created_at DESC, g.id`. Start time = `games.created_at`; end = `finished_at` (NULL if active). Columns: game type; opponents WITH each opponent's placing in one "Opponents" column (extend `opponents_by_game` `stats/queries.rs:197` to add `gp.place`); viewer's placing; min/max/avg match ELO = `min/max/avg(rating_before) WHERE game_id=$1 AND rating_before IS NOT NULL` (cheap, no reconstruction); viewer's ELO change (`rating_change`); start; end; clickable row -> `/games/{id}`. PLAIN sqlx queries. Server fn `#[server(GetPlayerHistory, "/api")]` mirroring `get_player_profile` (`stats/mod.rs:129`). Component `PlayerHistoryPage` copying `PlayerGameTypePage` (`players.rs:442`); prev/next as plain `<A href>` query-param links (no client state). |
| 7 - Game information page | Route `/games/type/{name}` (3-segment; avoids collision with `/games/{id}` which is a Uuid, `app.rs:202`). Identify game type by percent-encoded NAME, matched case-insensitively (`encode_path_segment` `players.rs:34`, `find_game_type_name` `stats/queries.rs:31`). Blurb: use `game_types.blurb` (exists, migration 012, `models/game.rs:14`). Rules/strategy: LINK to `/rules/{version_id}` (`app.rs:203`); resolve type -> a public/non-deprecated version (reuse `find_available_game_types` logic, `db.rs:282`). Stats: total games (FINISHED-only); games active today = `updated_at` today (ANY activity); total distinct players (in finished games). All NEW PLAIN-sqlx queries. Ranking: top 10 by `game_type_users.rating DESC`, filtering never-rated rows; each row name(link) + current ELO + trend sparkline via cheap `recent_form_for_game_type` (`stats/queries.rs:474`) + `rating_trend` (`players.rs:15`, make `pub(crate)`) + `Sparkline` (`stats/viz.rs:37`). Start-new-game link -> `/games?game={name}` (ADD `use_query_map` + preselect Effect to `new_game.rs`; selection is internal signal state today, `new_game.rs:148-149,189`). Page is PUBLIC. Copy `PlayerGameTypePage` template. |
| 8 - Bot reasoning time limit | Prompt-only. Edit `rust/bot/system_prompt.md` `# Task` section (lines 5-9) - `include_str!`'d at `prompt.rs:5`, so editing the markdown is the whole change. Add a time-pressure sentence (aggressive timeout; be quick/efficient; reply with command in under 1 minute). Keep the 300s hard timeout (`main.rs:783`) as backstop - do NOT change it. Add one test assertion in `prompt.rs`. |
| 9 - Rules rendering: newlines + GFM tables | Renderer is pulldown-cmark 0.13.4; `render_markdown` (`rules.rs:150`) uses `Parser::new` = NO extensions. Newlines = CSS-only: `.rules-page .rules-doc { white-space: pre-line; }` in `main.scss` (~line 549). Tables = switch `Parser::new` -> `Parser::new_ext` + `Options::ENABLE_TABLES`; ALSO enable `ENABLE_STRIKETHROUGH` + `ENABLE_TASKLISTS`. (In 0.13.4 `ENABLE_GFM` is ONLY blockquote alerts - tables need `ENABLE_TABLES` explicitly.) No new crate. Strategy docs use the same pipeline => inherit (accepted). Email: same `render_doc` feeds the `rules` command (`commands.rs:879`); email wraps HTML in a plain `<body>` (`inbound.rs:891`), so the `docs/email.md` font-size:0 hazard does NOT apply; pulldown's bare `<table>` is borderless in Gmail - ACCEPTED, no table styling. |
| 10 - Email settings (per-type toggles) + reorder | Single nudge + on/off model - NO frequency column. `users.turn_emails_enabled` (bool NOT NULL DEFAULT true) ALREADY exists (`014_email_play.sql`) and gates turn+invite+reminder today. Migration 018: `ADD COLUMN IF NOT EXISTS invite_emails_enabled boolean NOT NULL DEFAULT true;` and `... reminder_emails_enabled boolean NOT NULL DEFAULT true;` (defaults true => no backfill). Split the gate: TURN keeps `turn_emails_enabled`; INVITE: change the three `!recip.turn_emails_enabled` checks (`proposals.rs:187,287,335`) to `invite_emails_enabled` + add column to `fetch_invite_recipient` (`proposals.rs:150`) + `InviteRecipient` (`proposals.rs:143-147`); REMINDER: change `u.turn_emails_enabled = true` (`sweep.rs:72`) to `reminder_emails_enabled`. Reminder cadence stays a single 24h nudge. Settings UI: new `EmailPreferencesSection` copying `ColorsSection` fire-and-forget; three independent toggles (turn/invite/reminder), each in `<FormField>`; setter server fns `set_email_invite_enabled(bool)`/`set_email_reminder_enabled(bool)` following `set_pref_colors` (`auth/server.rs:608`); `SettingsData` (`auth/server.rs:526`) + `get_settings` gain the fields (PLAIN queries). Reorder sections to: Username, Colours, EmailPreferences, EmailAddresses, Theme (theme LAST). Email-command parity: extend `help_text` (`commands.rs:153`), `SettingsSummary`/`format_settings_summary` (`commands.rs:183,190`), add `emails invite on/off` + `emails reminder on/off` (model on `run_emails_toggle` `commands.rs:563`, `set_turn_emails_enabled` `commands.rs:743`). |
| 11 - Email "bump" command | Keyword `bump`. Reuse the 22d switch-digest loop (`auth/server.rs:740-744`): `find_active_turn_games` (`db.rs:2420`) + `cap_digest` (`db.rs:2258`, cap `SWITCH_DIGEST_CAP=20` `db.rs:2225`) + `send_turn_digest` (`notify.rs:309`). ALWAYS SEND regardless of `turn_emails_enabled` (explicit pull) - add a bypass to the `send_one` opt-out check (`notify.rs:231-232`). Do NOT build a new render path - reuse `render_game_email` (inherits the `docs/email.md` Gmail fix). Reply: one email PER GAME + one Status confirmation; handler returns `CommandReply::Status(summary)` ("Re-sent N games to your active address." / "No games are waiting on your turn."). Wiring: recognise `bump` in the standalone path (`dispatch_standalone_server_command` `commands.rs:292` and/or `dispatch_settings_standalone` `commands.rs:259`) AND add a `bump` arm in `dispatch_email_command` (`commands.rs:939`) building a `StandaloneCommandCtx` (`commands.rs:283`) from the `EmailCommandCtx` (like the existing "new" arm). Add `bump` to `help_text`. Abuse backstop: the 20-game cap only (parity with 22d), NO cooldown. |

---

## Migration number assignments

- **017** = `game_players.rating_before` (Unit 5 / Item 6a). File:
  `rust/web/migrations/017_game_player_rating_before.sql`.
- **018** = email preference columns `invite_emails_enabled` +
  `reminder_emails_enabled` (Unit 9 / Item 10). File:
  `rust/web/migrations/018_email_preferences.sql`.

These are reserved - do not collide. Latest existing migration is
`016_invite_nudge.sql`. Migrations are immutable once applied; new work in a new
file only.

---

## Unit 1 - Profile colour bar (#1) + Unfriend confirmation (#2)

### Items covered
Item 1 (profile colour bar under username), Item 2 (unfriend confirmation).

### Spec
- **#1:** Make the colour ribbon sit flush under the username heading like an
  underline. The gap is the browser UA default `<h1>` bottom margin (no heading
  reset exists in `main.scss`).
- **#2:** The destructive "Unfriend" action currently dispatches immediately with
  no confirmation. Add a native browser confirm dialog before dispatching.

### Plan
**#1 (CSS-only):** Add to the profile section of `rust/web/style/main.scss`
(the `.color-ribbon` rules are at `main.scss:641`; profile rules live nearby):
```scss
.profile-header h1 {
  margin-bottom: 0;
}
```
- Heading markup: `players.rs:186` (`<h1>{d.user.name}</h1>`) inside `<header
  class="profile-header">` (`players.rs:185`); ribbon at `players.rs:187`
  (`ColorRibbon`, defined `components/form.rs`).
- The per-game-type page header (`players.rs:512`) also uses `.profile-header`
  and gets tightened too - accepted, harmless.
- Precedent for scoped heading-margin override: `.login h1 { margin-bottom: 0; }`
  (`main.scss:102`), `.new-game-panel h2 { margin-top: 0; }` (`main.scss:751`).

**#2 (handler-only):** Edit ONLY the unfriend click handler at `friends.rs:440-443`.
The closure (`friends.rs:429-446`) already has `let uid = f.user_id;` (:430) and
`let name = f.name.clone();` (:431, moved into the view at :436). Add an owned
clone for the message and gate the dispatch:
```rust
o.friends.iter().map(|f| {
    let uid = f.user_id;
    let name = f.name.clone();
    let unfriend_name = name.clone();          // NEW: owned copy for the closure
    view! {
        <div class="friend-row">
            <span>
                <A href=format!("/players/{}", crate::players::encode_path_segment(&name))>
                    {name.clone()}
                </A>
            </span>
            " "
            <a href="#" on:click=move |ev| {
                ev.prevent_default();
                let confirmed = web_sys::window()
                    .and_then(|w| w.confirm_with_message(
                        format!("Unfriend {}? You'll need to send a new friend request to add them again.", unfriend_name),
                    ).ok())
                    .unwrap_or(false);
                if confirmed {
                    unfriend_action.dispatch(Unfriend { user_id: uid });
                }
            }>"Unfriend"</a>
        </div>
    }
}).collect_view().into_any()
```
- In-file precedent: "Decline and block" handler `friends.rs:412`
  (`web_sys::window().confirm_with_message(...)`); `web_sys` already used there.
- Reference pattern: force-delete game `components/game.rs:166`, concede
  `components/game.rs:125`.
- No change to the `Unfriend` server fn (`friends.rs:195-196`), the refetch
  Effect, or `db::unfriend`.

### Gotchas
- #1: pure CSS - zero hydration impact.
- #2: entirely inside a client-side event-handler closure; the `<a>` stays
  always-present; no structural markup change - zero hydration impact.
  `web_sys::window()` runs only on click (never during SSR), like the existing
  "Decline and block" handler. `.unwrap_or(false)` defaults to NOT unfriending
  (safe direction) - keep it, never `.unwrap()`.
- Scope to unfriend ONLY (not Unblock/Decline).

### Tests
- No new tests required (CSS + handler-only). Existing friends tests unaffected.

### Verification
- #1 needs no Rust rebuild. #2 touches `friends.rs`:
  `cargo fmt --all -- --check` and
  `cargo clippy -p web --all-targets --features ssr -- -D warnings`.

---

## Unit 2 - "invite" copy (#3)

### Items covered
Item 3 (privacy/invite-policy copy: "add me to a game" -> "invite me to a
game").

### Spec
Reword the invite-policy labels and related copy from "add" to "invite".
Display-only - the stored DB value is the slug, fully separate from the label.

### Plan
All in `rust/web/src/friends.rs` (this section is on the `/friends` page, NOT
`settings.rs`):
1. The 3 labels in `INVITE_POLICIES` (`friends.rs:12-15`):
   - `("open", "Anyone can invite me to a game")`
   - `("friends", "Only friends can invite me to a game")`
   - `("none", "Nobody can invite me to a game")`
2. Section heading (`friends.rs:498`): `<h2>"Who can invite me to games"</h2>`.
3. "Decline and block" confirm dialog (`friends.rs:412`): change "...or add you
   to games" -> "...or invite you to games".

### Gotchas
- **Display-only - confirmed.** DB CHECK is on the slug (`010_friends.sql`:
  `CHECK (invite_policy IN ('open','friends','none'))`); `set_invite_policy`
  validates against slugs (`friends.rs:231`); `get_invite_policy` returns the
  slug (`db.rs:1897`); the `<select>` compares `o.invite_policy == *slug`
  (`friends.rs:502-505`). No migration, no `.sqlx` impact, no stored-data change.
- Invite EMAIL text already uses "invite" ("{owner} invited you to play {game}",
  `proposals.rs:206`) - no email change.
- Labels render at `friends.rs:502-505` (the `<select>` over `INVITE_POLICIES`).

### Tests
- None required (string-only). 

### Verification
- `cargo fmt --all -- --check`,
  `cargo clippy -p web --all-targets --features ssr -- -D warnings`.

---

## Unit 3 - Game-creation bug + error surfacing + regression test (#4)

### Items covered
Item 4 (can't start new games; surface generic errors; regression test; dead-code
check).

### Spec
- New-game submission fails with "missing field `opponent_emails`" for any game
  with no email invitees (the common case). Fix the deserialization bug.
- Stop showing raw framework/internal error strings to users; show a generic
  message via a shared client helper.
- Add a regression test that exercises the server-fn deserialization path.
- Remove `create_new_game` if it is dead code.

### Plan
**(a) Root-cause fix.** `create_proposal` (`proposals.rs:877-884`) declares bare
`Vec<T>` params with no `#[serde(default)]`:
```rust
#[server(CreateProposal, "/api")]
pub async fn create_proposal(
    game_version_id: Uuid,
    opponent_ids: Vec<Uuid>,        // :881
    opponent_emails: Vec<String>,   // :882  <- reported field
    bot_slots: Vec<crate::game::server_fns::BotSlot>, // :883
) -> Result<ProposalOutcome, ServerFnError>
```
Leptos 0.8 `#[server]` args are serde_qs url-encoded POST; an empty Vec emits no
key, and a missing non-`Option`/non-`default` Vec field is a hard deserialization
error. Fix all THREE params (opponent_ids and bot_slots have the identical latent
bug):
```rust
pub async fn create_proposal(
    game_version_id: Uuid,
    #[serde(default)] opponent_ids: Vec<Uuid>,
    #[serde(default)] opponent_emails: Vec<String>,
    #[serde(default)] bot_slots: Vec<crate::game::server_fns::BotSlot>,
) -> Result<ProposalOutcome, ServerFnError>
```
- **Verify first** that the Leptos 0.8 `#[server]` macro forwards `#[serde(default)]`
  from fn params onto the generated args struct (no in-codebase precedent; the
  only `#[serde(default)]` usages are on a plain Deserialize struct in
  `email/inbound.rs`). If the macro does NOT forward it, use the fallback below.
- **Fallback (proven by `create_new_game`):** change the three params to
  `Option<Vec<T>>`, `unwrap_or_default()` in-body, and update the form call site
  (`new_game.rs:208`) to pass `Some(ids)`, `Some(emails)`, `Some(bots)`.
  `create_new_game` (`server_fns.rs:461`, unwraps at :480-482) is the working
  reference. Field is default-empty, not optional-in-meaning.

**(b) Error surfacing.** The deserialization failure is produced by the server_fn
machinery BEFORE the fn body runs, so the body's `internal()` guard (`error.rs:6`)
can't catch it - the raw `ServerFnError` reaches the client and is rendered
verbatim via `e.to_string()` at `new_game.rs:445` (and also `new_game.rs:133`).
- Add ONE shared CLIENT helper, e.g.:
  ```rust
  pub fn user_facing_server_error(e: &ServerFnError) -> String {
      // Expected, user-meaningful rejections should travel as data/typed
      // UserError, not ServerFnError. Anything reaching here as a ServerFnError
      // is technical/internal -> generic message; raw detail must not reach UI.
      "Something went wrong, please try again".to_string()
  }
  ```
  Place it somewhere shared (e.g. `error.rs` or a small client util module).
- Apply at the new-game form (`new_game.rs:445`, and `:133`) at minimum. Broader
  rollout across the scattered raw `e.to_string()` sites (proposals.rs,
  settings.rs:221/237/252/267, friends.rs, rules.rs) is optional - do the
  new-game form for sure.
- Note as a follow-up (out of scope): server-side logging of deserialization
  failures would need a tower layer / custom server_fn error codec.

**(c) Regression test.** Add to `rust/web/tests/ssr_pages.rs`, modeled on
`restart_game_on_finished_game_succeeds` (`ssr_pages.rs:457`) and its
`restart_game_via_http` helper (`ssr_pages.rs:428`), which POST a url-encoded
body to `<Fn as leptos::server_fn::ServerFn>::PATH`:
```rust
#[sqlx::test]
async fn create_proposal_without_opponent_emails_succeeds(pool: PgPool) {
    let uri = spawn_mock_new_game_service().await;          // handles Request::New
    let game_version_id = make_game_version(&pool, &uri).await; // 2-player type
    let user = make_user(&pool, "creator").await;
    let cookie = login_cookie(&pool, &user, "creator@example.com").await;
    let app = build_router(make_state(pool).await).await;

    // Body deliberately OMITS opponent_emails (and opponent_ids): 1 human + 1 bot.
    let path = <web::proposals::CreateProposal as leptos::server_fn::ServerFn>::PATH;
    let body = format!(
        "game_version_id={game_version_id}&bot_slots[0][name]=Botty&bot_slots[0][bot_name]=easy"
    );
    let resp = app.oneshot(
        Request::builder().method("POST").uri(path)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .header("cookie", cookie)
            .body(Body::from(body)).unwrap(),
    ).await.unwrap();

    let status = resp.status();
    let text = String::from_utf8_lossy(
        &axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap()
    ).into_owned();
    assert_eq!(status, StatusCode::OK, "body: {text}");
    assert!(!text.contains("missing field"), "body: {text}");
    let outcome: web::proposals::ProposalOutcome = serde_json::from_str(&text).unwrap();
    assert!(outcome.game_id.is_some(), "solo-vs-bots creates a game directly");
}
```
- `ProposalOutcome` (`proposals.rs:54-56`) has `proposal_id: Option<Uuid>` and
  `game_id: Option<Uuid>`; solo-vs-bots creates a game directly => `game_id`
  Some.
- Confirm the exact serde_qs key encoding for `Vec<BotSlot>` (likely
  `bot_slots[0][name]=...`); adjust the body to match what the browser sends. The
  essential property is that `opponent_emails` is ABSENT.
- Mocks the game service (`spawn_mock_new_game_service` `ssr_pages.rs:389`),
  never the real service/LLM. **Fails pre-fix, passes post-fix.**
- Why it slipped through: existing game-creation tests call
  `db::create_game_with_users` directly (`ssr_pages.rs:271,310,354,462,527`);
  none exercise server-fn deserialization.

**(d) Dead-code check.** Verify whether `create_new_game` (`server_fns.rs:461`)
has any frontend caller. The form was switched onto `create_proposal` in #24
(`new_game.rs:208`). If `create_new_game` has no caller, remove it (and its
client stub) to avoid two divergent creation fns. Grep for `create_new_game` /
`CreateNewGame` across `rust/web/src` to confirm before deleting.

### Gotchas
- Fix all THREE Vec params, not just `opponent_emails`.
- The deserialization error string ("error deserializing server function
  arguments: missing field ...") is Leptos/server_fn-internal, not in our
  codebase - you can't grep for it.
- `#[serde(default)]` forwarding by the `#[server]` macro is unverified in-repo;
  have the `Option<Vec<T>>` fallback ready.
- Regression test needs the mock game service + `#[sqlx::test]`; DB tests fail in
  plain local runs (known #40) - it runs in CI / `scripts/rust-test.sh`.

### Tests
- The regression test above (new). 
- If `create_new_game` is removed, ensure no test references it.

### Verification
- `cargo fmt --all -- --check`,
  `cargo clippy -p web --all-targets --features ssr -- -D warnings`.
- `bash scripts/rust-test.sh` before any commit (runs the new `#[sqlx::test]`).

---

## Unit 4 - Numeric form lines + medal colours (#5)

### Items covered
Item 5 (form lines: numeric only + medal colours).

### Spec
Render the form strip as the numeric placing (1, 2, 3, ...) with medal colours
instead of W/L. 1st = yellow, 2nd = grey, 3rd = red, others = normal foreground,
no placing = muted "-".

### Plan
No data change: `FormResult` already carries `place: Option<i32>`
(`stats/mod.rs:81`). W/L is purely a rendering decision in ONE function.

1. **Rewrite `form_cell`** (`stats/viz.rs:43`) to numeric-only + medal class;
   DROP the now-dead `player_count` param:
   ```rust
   pub fn form_cell(place: Option<i32>) -> (String, &'static str) {
       match place {
           Some(1) => ("1".to_string(), "form-gold"),
           Some(2) => ("2".to_string(), "form-silver"),
           Some(3) => ("3".to_string(), "form-bronze"),
           Some(p) => (p.to_string(), "form-other"),
           None    => ("-".to_string(), "form-none"),
       }
   }
   ```
2. **Update the `FormStrip` call site** (`viz.rs:66`, calls `form_cell` at :72-ish)
   to `form_cell(r.place)`.
3. **CSS** (`main.scss:651-665`): replace the old `.form-win`/`.form-loss` rules
   (`.form-strip` at :651, `.form-win` :655, `.form-loss` :659, `.form-none` :663):
   ```scss
   .form-strip .form-gold   { color: var(--mk-yellow); }
   .form-strip .form-silver { color: var(--mk-grey); }
   .form-strip .form-bronze { color: var(--mk-red); }
   .form-strip .form-other  { color: var(--mk-foreground); }
   .form-strip .form-none   { color: var(--mk-blue); }
   ```
   Theme palette vars (`--mk-*`) exist for every `NamedColor` in every theme
   (`rust/lib/color/src/css.rs`): red green blue yellow purple cyan pink orange
   brown grey foreground background.

Both render sites share `FormStrip` (`stats/viz.rs:66`), so no per-site change:
- Profile "Form" column: `players.rs:311`.
- Game meta panel `PlayerInfo`: `components/game.rs:239` (form vec populated
  server-side in `game/server_fns.rs`).

### Gotchas
- **No hydration risk:** `FormStrip` is a pure presentational component (no
  resource, no async state, no element-type swapping). Class + label computed
  from data present at SSR.
- **No `.sqlx` concern:** no query change.
- **Contrast:** `--mk-yellow` may be low-contrast on some themes - spot-check
  yellow across a light + a dark theme. (`--mk-grey` is body text, always clears.)

### Tests (update existing - they WILL break)
- `stats/viz.rs:317-337`: `form_cell_*` tests assert "W"/"L" + `form-win`/
  `form-loss`. Rewrite to the new labels/classes (e.g. `form_cell(Some(1))` ==
  `("1","form-gold")`, `form_cell(Some(2))` == `("2","form-silver")`,
  `form_cell(Some(3))` == `("3","form-bronze")`, `form_cell(Some(4))` ==
  `("4","form-other")`, `form_cell(None)` == `("-","form-none")`).
- `tests/ssr_pages.rs:941`: asserts `body.contains("form-win") &&
  body.contains("form-loss")`. Update to the new classes (e.g. `form-gold`/
  `form-silver`).

### Verification
- `cargo fmt --all -- --check`,
  `cargo clippy -p web --all-targets --features ssr -- -D warnings`,
  `cargo test -p web --features ssr` (the updated unit tests).
- SCSS change needs a style build to visually confirm (no Rust test covers CSS).

---

## Unit 5 - Store `rating_before` on `game_players` (migration 017) (#6a)

### Items covered
Item 6a (store historical rating so match min/max/avg ELO is a cheap read).

### Spec
Add a nullable `game_players.rating_before` column capturing the rating each
participant brought into a game. Backfill existing rows. Write it in the single
rating-write function. This unblocks the cheap match-ELO column in Unit 6.

### Plan
**(a) Migration 017** - `rust/web/migrations/017_game_player_rating_before.sql`:
```sql
ALTER TABLE public.game_players ADD COLUMN IF NOT EXISTS rating_before integer;

WITH ordered AS (
    SELECT
        gp.id AS game_player_id,
        1200 + COALESCE(
            sum(gp.rating_change) OVER (
                PARTITION BY gp.user_id, gv.game_type_id
                ORDER BY g.finished_at, g.id
                ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING
            ),
            0
        ) AS rating_before
    FROM game_players gp
    JOIN games g ON g.id = gp.game_id
    JOIN game_versions gv ON gv.id = g.game_version_id
    WHERE gp.user_id IS NOT NULL
      AND gp.rating_change IS NOT NULL
      AND gp.rating_before IS NULL
      AND g.finished_at IS NOT NULL
)
UPDATE game_players gp
SET rating_before = ordered.rating_before
FROM ordered
WHERE gp.id = ordered.game_player_id;
```
- `ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING` excludes the current game's
  own delta => rating *before* (migration-011's default frame includes the
  current row because it wants *after*).
- `gp.rating_before IS NULL` predicate makes re-runs a no-op.
- Joins mirror migration 011 exactly (`games` + `game_versions` for
  `game_type_id` + ordering by `finished_at, id`) => consistent with
  `rating_series` and the 011 peaks.
- NULL semantics: bots, single-human/all-bot games, and zero-change seats stay
  NULL (keeps `rating_before IS NOT NULL` == `rating_change IS NOT NULL`,
  matching the `rating_series` filter `stats/queries.rs:156`).
- **No new index** (existing `idx_game_players_game_id` serves the per-game
  aggregate).

**(b) Write-path change** - ONE function `apply_rating_changes` (`db.rs:1348`).
The pre-game rating is ALREADY in scope: `RatedPlayer.rating` (`db.rs:1386-1390`),
loaded via `SELECT rating FROM game_type_users ...` (`db.rs:1409-1415`) BEFORE the
delta is applied (`db.rs:1453-1470`).
1. After `rated_players` is built (after `db.rs:1422`), add a position->before map:
   ```rust
   let rating_befores: std::collections::HashMap<i32, i32> = rated_players
       .iter()
       .map(|p| (p.position, p.rating))
       .collect();
   ```
2. Change the final write loop (`db.rs:1472-1484`) to write `rating_before`
   alongside `rating_change`:
   ```rust
   for p in &players {
       let change = rating_changes.get(&p.position).copied().unwrap_or(0);
       if change == 0 {
           continue;
       }
       let rating_before = rating_befores.get(&p.position).copied();
       sqlx::query!(
           "UPDATE game_players SET rating_change = $1, rating_before = $2 WHERE id = $3",
           change,
           rating_before,
           p.id
       )
       .execute(&mut *tx)
       .await?;
   }
   ```
- `change != 0` is only ever true for rated humans, so `rating_before` is always
  `Some(_)` at the UPDATE, but typed `Option<i32>` to match the nullable column.
- The `change == 0` `continue` keeps `rating_before` NULL exactly when
  `rating_change` stays NULL.
- Zero extra queries. Both finish paths (concede `db.rs:1159`, normal finish
  `db.rs:1583`) call `apply_rating_changes`, so both are covered.
- **This adds a macro query** (`query!`) on a new column => run `cargo sqlx
  prepare` per `docs/DEV.md` (see Global conventions). Alternatively write it as
  a plain query to avoid `.sqlx` regen - either is acceptable; if plain, follow
  the `db.rs:254-264` pattern.

### Gotchas
- **Migration immutability:** new file (017), never edit existing migrations.
  Deploy-window note: migrate Job at sync-wave 1, web at wave 2; a nullable ADD
  COLUMN + data-only backfill is safe (old pods never reference `rating_before`).
- **`game/import.rs` is a second `rating_change` writer** that bypasses
  `apply_rating_changes`. Backfill covers existing imports; NEW imports will be
  NULL - ACCEPTED, do NOT modify `import.rs`.
- Admin force-delete (`db.rs:1171`) doesn't rewind ratings (pre-existing); stored
  values are frozen as-of-write.
- Do NOT rewrite `rating_series` to read stored values (out of scope, avoids risk
  to the ELO chart).

### Tests (`#[sqlx::test]`, in `db.rs` / `stats/queries.rs` mod tests)
- **Backfill migration test** (model on
  `peak_rating_backfill_corrects_historical_peaks`, `stats/queries.rs:1228`):
  `include_str!("../../migrations/017_game_player_rating_before.sql")` via
  `sqlx::raw_sql`; insert finished games with known rating_changes (+16, +20,
  -30) plus an interleaved bot game (NULL) and a single-human game; assert
  `rating_before` = 1200, 1216, 1236; bot/single-human rows NULL; idempotent on
  re-run; an already-set row is not overwritten.
- **Write-path capture test** (in `db.rs` mod tests, alongside existing rating
  tests using `find_rating_change` `db.rs:4031` / `game_type_rating` `db.rs:4042`):
  finish a game, assert each rated `game_player.rating_before` equals the
  player's `game_type_users.rating` immediately before finish (seed distinct
  starting ratings e.g. 1300 vs 1100); bot seat NULL; `rating_before +
  rating_change` == post-game `game_type_users.rating`.
- **History aggregate test** (near `finished_games` tests, `stats/queries.rs:972`):
  seed a game whose rated humans have `rating_before` 1200 and 1300; assert
  `min/max/avg(rating_before) WHERE rating_before IS NOT NULL` == 1200/1300/1250;
  bot seat (NULL) excluded.

### Verification
- `cargo fmt --all -- --check`,
  `cargo clippy -p web --all-targets --features ssr -- -D warnings`.
- If a macro query was added: `cargo sqlx prepare -- --features ssr` (per
  `docs/DEV.md`) and `SQLX_OFFLINE=true cargo check --features ssr`.
- `bash scripts/rust-test.sh` before any commit.

---

## Unit 6 - Full game history page (#6b)

### Items covered
Item 6b (paginated, filterable full game history page). **Depends on Unit 5**
(`rating_before` must exist for the cheap match-ELO column).

### Spec
A new public page `/players/:name/history` listing every game a player has
played, paginated (50/page), filterable by status and game type via shareable
query params, with opponents+placings, the viewer's placing, match min/max/avg
ELO, the viewer's ELO change, start/end times, and clickable rows.

### Plan
**(a) Route** (`rust/web/src/app.rs:200-201`): declare the static `history` route
BEFORE the `:game_type` param route (collision: `:game_type` would match
"history"):
```rust
<Route path=(StaticSegment("players"), ParamSegment("name"), StaticSegment("history")) view=crate::players::PlayerHistoryPage/>
<Route path=(StaticSegment("players"), ParamSegment("name"), ParamSegment("game_type")) view=crate::players::PlayerGameTypePage/>
```
Game links = `/games/{id}`.

**(b) Queries** (`rust/web/src/stats/queries.rs`, PLAIN `sqlx::query_as` to avoid
`.sqlx` regen - touches `created_at` + aggregates; precedent `db.rs:254-264`,
plain `query_as` at `queries.rs` near :1302):
- Core paged query (`game_history(pool, user_id, status: Option<bool>, game_type:
  Option<&str>, limit, offset)`):
  ```sql
  SELECT g.id AS game_id, gt.name AS game_type_name, g.is_finished AS is_finished,
         g.created_at AS started_at, g.finished_at AS finished_at,
         gp.place AS my_place, gp.rating_change AS my_rating_change,
         (SELECT count(*) FROM game_players gp2 WHERE gp2.game_id = g.id) AS player_count,
         (SELECT min(r.rating_before) FROM game_players r WHERE r.game_id = g.id AND r.rating_before IS NOT NULL) AS match_min,
         (SELECT max(r.rating_before) FROM game_players r WHERE r.game_id = g.id AND r.rating_before IS NOT NULL) AS match_max,
         (SELECT avg(r.rating_before)::int FROM game_players r WHERE r.game_id = g.id AND r.rating_before IS NOT NULL) AS match_avg
  FROM game_players gp
  JOIN games g          ON g.id = gp.game_id
  JOIN game_versions gv ON gv.id = g.game_version_id
  JOIN game_types gt    ON gt.id = gv.game_type_id
  WHERE gp.user_id = $1
    AND ($2::boolean IS NULL OR g.is_finished = $2)   -- status filter
    AND ($3::text    IS NULL OR gt.name = $3)         -- game-type filter
  ORDER BY g.created_at DESC, g.id
  LIMIT $4::bigint OFFSET $5::bigint;
  ```
- Companion count query (same WHERE, `SELECT count(*)`) for prev/next + "page X
  of Y".
- **Extend `opponents_by_game`** (`stats/queries.rs:197`) to also select
  `gp.place` (opponent placing). Keep the existing `LEFT JOIN game_bots` +
  `COALESCE(u.name, gb.name, 'Bot')` bot handling.

**(c) DTOs + server fn** (`rust/web/src/stats/mod.rs`):
- DTOs derive `Debug, Clone, PartialEq, Serialize, Deserialize`: `HistoryRow
  { game_id, game_type_name, is_finished, started_at, finished_at, my_place,
  player_count, my_rating_change, opponents: Vec<OpponentWithPlace>, match_elo:
  Option<MatchElo> }`, `OpponentWithPlace { user_id, name, place }`, `MatchElo {
  min, max, avg }`, `PlayerHistoryData { user, rows, page, page_size, total,
  filters }`.
- Server fn `#[server(GetPlayerHistory, "/api")]` mirroring `get_player_profile`
  (`stats/mod.rs:129`): resolve user via the same profile-user helper, return
  `Option<PlayerHistoryData>` (None = not found). Guard pattern matches the
  public profile (profiles are public; viewer is `Option`).

**(d) Component** (`rust/web/src/players.rs`): `PlayerHistoryPage` copying
`PlayerGameTypePage` (`players.rs:442`):
- `Resource::new_blocking` keyed on `(use_params_map(), use_query_map())` (so
  `?page`/`?status`/`?type`/`?bots` changes refetch) + `<Suspense fallback=||
  view!{<div></div>}>` inside `<MainLayout>`.
- Filter controls + table + prev/next as plain `<A href>` query-param links (no
  client state). Honour `?bots=1` (include-single-human toggle, like profiles).
- Reuse helpers: `format_placing` (`players.rs:61`), `opponents_view`
  (`players.rs:71`, extended to show placing), rating-change spans
  (`players.rs:357`, `rating-change-up/down/none`).
- Columns: game type; Opponents (with each opponent's placing, single column);
  viewer's placing; min/max/avg match ELO; viewer's ELO change; start
  (`created_at`); end (`finished_at`, blank if active); clickable row ->
  `/games/{id}`.
- Add pagination control styles to `main.scss` (none exist yet).

### Gotchas
- **Route collision:** declare the static `history` route BEFORE the param route.
  Today `/players/foo/history` resolves to `PlayerGameTypePage`,
  `find_game_type_name("history")` returns None => "Not found"; adding the route
  repurposes it.
- **No `started_at`:** use `games.created_at` as start. End = `finished_at` (NULL
  if active).
- **Match ELO is now cheap** (Unit 5 stored `rating_before`) - NO read-time
  reconstruction. Do NOT reconstruct via the migration-011 window here.
- **Hydration:** follow the profile pages exactly - `Resource::new_blocking` read
  inside `<Suspense>`, `<MainLayout>` OUTSIDE Suspense, filter/pagination via
  query params (resource re-keys on query change). Keep rows structurally stable
  (no element-type swaps on async state). A paginated table IS core page content
  => `new_blocking` (not `LocalResource`).
- **`.sqlx` regen:** use plain (non-macro) queries for the new SQL.
- **Bots:** `game_players.user_id` is nullable; opponent join must `LEFT JOIN
  game_bots` + `COALESCE` (existing pattern in `opponents_by_game`).

### Tests
- `#[sqlx::test]` for the new queries (`game_history` paging/filters/count,
  `opponents_by_game` with placing, match-ELO aggregate). Fixtures:
  `stats/queries.rs:540-686` (`make_user`, `make_game_type`, `insert_finished_game`,
  `set_game_type_rating`).
- An SSR page test in `tests/ssr_pages.rs` (form-strip test at `ssr_pages.rs:908`
  region is a template): assert 200 + a page marker + no SSR panic.

### Verification
- `cargo fmt --all -- --check`,
  `cargo clippy -p web --all-targets --features ssr -- -D warnings`.
- `bash scripts/rust-test.sh` before any commit.

---

## Unit 7 - Game information page (#7)

### Items covered
Item 7 (per-game-type information page: blurb, rules link, stats, ranking,
start-new-game link).

### Spec
A new public page `/games/type/{name}` showing the game type's blurb, a link to
its rules/strategy, aggregate stats (total games, active today, distinct
players), a top-10 ranking with trend sparklines, and a "start game" link that
preselects this game in the new-game form.

### Plan
Suggested sub-order: (1) queries + server fn + DTOs + tests; (2) page component
+ route + CSS; (3) new-game preselect + link wiring.

**(a) Routing** (`app.rs:195,202`): add a 3-segment route (avoids collision with
`/games/{id}` which is a Uuid):
```rust
<Route path=(StaticSegment("games"), StaticSegment("type"), ParamSegment("name")) view=crate::game_info::GameInfoPage/>
```
- Game types have NO slug - identify by percent-encoded NAME, matched
  case-insensitively (`encode_path_segment` `players.rs:34`, `find_game_type_name`
  `stats/queries.rs:31`).
- Create a new module `rust/web/src/game_info.rs` (mirror `stats/mod.rs` +
  `stats/queries.rs` split), registered in `rust/web/src/lib.rs` (see `pub mod
  rules;`).

**(b) Queries** (NEW PLAIN-sqlx, pattern `db.rs:254-264`):
- Header: one `game_types` row by `lower(name)` + its `game_versions WHERE
  is_public AND NOT is_deprecated ORDER BY name` (pick `.first()` for the rules
  link). Reuse `find_available_game_types` selection logic (`db.rs:282`).
- Total games played (FINISHED-only scope):
  `SELECT count(*) FROM games g JOIN game_versions gv ON gv.id = g.game_version_id
  WHERE gv.game_type_id = $1 AND g.is_finished = true`.
- Games active today (ANY activity today):
  `... AND g.updated_at >= date_trunc('day', now() AT TIME ZONE 'utc')`. (No index
  on `updated_at`; rides `idx_games_game_version_id` first; acceptable at beta
  scale.)
- Total distinct players (in finished games):
  `SELECT count(DISTINCT gp.user_id) FROM game_players gp JOIN games g ... JOIN
  game_versions gv ... WHERE gv.game_type_id = $1 AND gp.user_id IS NOT NULL AND
  g.is_finished = true`.
- Top 10 ranking:
  `SELECT gtu.user_id, u.name, gtu.rating, gtu.peak_rating FROM game_type_users
  gtu JOIN users u ON u.id = gtu.user_id WHERE gtu.game_type_id = $1 [filter
  never-rated rows] ORDER BY gtu.rating DESC, u.name LIMIT 10`.

**(c) Blurb / rules link:**
- Blurb: `game_types.blurb` (exists, migration 012, `models/game.rs:14`; loaded
  via `find_available_game_types` `db.rs:282`; already rendered in the new-game
  browser). Do not carve from rules.
- Rules/strategy: LINK to `/rules/{version_id}` (`app.rs:203`, rendered by
  `rules.rs`). Rules are per-VERSION; resolve type -> a public/non-deprecated
  version (`.first()`). Strategy links go to the same rules page (it surfaces
  BASIC/ADVANCED for V2 games). If a type has zero public/non-deprecated
  versions, handle gracefully (no rules link).

**(d) Ranking trend (cheap):**
- Do NOT run `rating_series` per top player (N full history scans). Use:
  - `recent_form_for_game_type(pool, user_ids, game_type_id, per_user)`
    (`stats/queries.rs:474`) - ONE query for all top players.
  - `rating_trend(current: Option<i32>, results: &[FormResult]) -> Vec<f64>`
    (`players.rs:15`) - currently PRIVATE; make it `pub(crate)` (or move to
    `stats`).
  - `Sparkline` component (`stats/viz.rs:37`) for the inline sparkline.
- Per player: `Sparkline values=rating_trend(Some(rating), &form_for_player)`.

**(e) Server fn + component:**
- Server fn shape like `get_player_game_type_stats` (`stats/mod.rs:186`): resolve
  canonical name via `find_game_type_name`, return `Option<Data>` (None = not
  found). Page is PUBLIC (like profiles).
- Component `GameInfoPage` copying `PlayerGameTypePage` (`players.rs:442`):
  `Resource::new_blocking` + `<Suspense>` inside `<MainLayout>`.

**(f) Start-new-game preselect** (`rust/web/src/new_game.rs`):
- `new_game.rs` has NO query-param handling today (selection is internal signal
  state: `selected_type_id`/`selected_version_id` at `new_game.rs:148-149`;
  `select_game` at `new_game.rs:189`).
- ADD `leptos_router::hooks::use_query_map` (precedent: `players.rs` reads
  `?bots=1`) + an `Effect` that, on first types load, finds the `GameTypeInfo`
  matching `?game={name}` (case-insensitive against `gt.name`) and calls the same
  selection logic as `select_game`.
- The info page links to `/games?game={encoded_name}`.

### Gotchas
- **Routing arity:** keep the info route 3-segment so it never collides with
  `/games/{id}`.
- **`.sqlx` regen:** all NEW queries plain sqlx.
- **Tests required:** new aggregate queries get `#[sqlx::test]` (fixtures
  `stats/queries.rs:540-686`).
- **Query cost:** never `rating_series` per top player; use
  `recent_form_for_game_type` + `rating_trend`. "Active today" on `updated_at`
  has no index - acceptable at beta scale.
- **`rating_trend` is private** (`players.rs:15`) - make `pub(crate)`/move.
- **No panics:** handle missing game type / no versions / empty stats with
  graceful "not found"/"-" rendering.

### Tests
- `#[sqlx::test]` for the new queries (totals, active-today, distinct players,
  top-10 ranking).
- An SSR page test in `tests/ssr_pages.rs` (assert 200 + marker + no panic).
- A test for the new-game preselect query-param handling if feasible at the SSR
  layer.

### Verification
- `cargo fmt --all -- --check`,
  `cargo clippy -p web --all-targets --features ssr -- -D warnings`.
- `bash scripts/rust-test.sh` before any commit.

---

## Unit 8 - Bot timeout prompt (#8) + Rules rendering (#9)

### Items covered
Item 8 (tell bots there is a reasoning time limit), Item 9 (rules rendering:
newlines + GFM tables).

### Spec
- **#8:** Add a time-pressure instruction to the bot's static system prompt so
  the model reasons quickly instead of hitting the 300s hard timeout.
- **#9:** Make rules prose honour single newlines, and render GFM pipe tables
  (plus strikethrough/tasklists).

### Plan
**#8 (prompt-only):** Edit `rust/bot/system_prompt.md` `# Task` section (lines
5-9). It is `include_str!`'d at `rust/bot/src/prompt.rs:5` and rendered by
`render_system` (`prompt.rs:65`) - editing the markdown is the whole change (no
Rust). Insert after line 7 ("...No explanation."):
```
You are operating under an aggressive time limit. Reason quickly and
efficiently, and reply with your command in under 1 minute. A fast,
good-enough move beats a slow, perfect one - do not over-analyse.
```
- Keep the 300s hard timeout (`rust/bot/src/main.rs:783`, inline
  `Duration::from_secs(300)`) as the backstop - do NOT change it. (Game-service
  client is a separate 60s client at `main.rs:788`.)
- Add ONE test assertion in `prompt.rs` (mod tests `prompt.rs:138+`), e.g.
  `render_system_states_time_limit` asserting the output contains "under 1
  minute" (mirror `render_system_contains_command_parser_docs` `prompt.rs:203`).
  No existing test touches the `# Task` section, so nothing breaks.

**#9 (CSS + renderer option):**
- **Newlines (CSS-only):** add to `rust/web/style/main.scss` near the existing
  `.rules-page .rules-doc .game-render` block (`main.scss:551`):
  ```scss
  .rules-page .rules-doc {
    white-space: pre-line;
  }
  ```
  `pre-line` preserves source newlines as breaks but collapses space runs (better
  than `pre-wrap`). Embedded `<pre>` boards set their own `white-space: pre`,
  which wins for their subtree. `.rules-section`/`.rules-doc` have no whitespace
  rule today.
- **Tables (renderer option):** in `render_markdown` (`rules.rs:150-151`), switch
  `Parser::new` -> `Parser::new_ext` + options:
  ```rust
  let mut opts = pulldown_cmark::Options::empty();
  opts.insert(pulldown_cmark::Options::ENABLE_TABLES);
  opts.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);
  opts.insert(pulldown_cmark::Options::ENABLE_TASKLISTS);
  let parser = pulldown_cmark::Parser::new_ext(markdown, opts);
  pulldown_cmark::html::push_html(out, parser);
  ```
  - In pulldown-cmark 0.13.4 (`rust/web/Cargo.toml`, `ssr` feature), `ENABLE_GFM`
    is ONLY GFM blockquote alerts - tables need `ENABLE_TABLES` explicitly. No new
    crate (honours "no bespoke code / no non-Rust deps").
  - `render_doc` (`rules.rs:162`) splits ```brdgme fences (-> semantic board
    markup) from prose (-> pulldown); reused for rules + basic + advanced
    strategy, so strategy docs inherit the change (accepted).

### Gotchas
- **#8:** the 300s value is not a named constant; no compile-time link between
  the "under 1 minute" wording and the backstop. Keep the backstop unchanged.
- **#9 email:** the same `render_doc` feeds the `rules` email command
  (`email/commands.rs:879`, `run_rules`). Email wraps HTML in a plain `<body>`
  (`email/inbound.rs:891`) - NOT `<pre>`/`<mj-raw>` - so the `docs/email.md`
  `font-size:0`/foster-parenting hazard does NOT apply. BUT pulldown emits a bare
  borderless `<table>` in Gmail - ACCEPTED, do not add table styling. The email
  TEXT part uses raw markdown source (unaffected). The CSS newline fix does not
  touch email (different DOM, no `.rules-doc` class).
- **#9 existing tests won't break:** `rules.rs` tests (`rules.rs:371+`) use prose
  with no pipe tables. DB integration test at `rules.rs` (~:478) fails to connect
  locally (known #40) - not a regression.

### Tests
- **#8:** add `render_system_states_time_limit` in `prompt.rs`.
- **#9:** add a table-render test in `rules.rs` (mirror `prose_only_renders_markdown`
  `rules.rs:371`): a GFM pipe table renders to `<table>`/`<th>`/`<td>`. Newline
  rendering is CSS, not unit-testable in Rust.

### Verification
- **#8:** `cargo test -p bot` (and `cargo clippy --workspace --exclude web
  --all-targets -- -D warnings`).
- **#9:** `cargo clippy -p web --all-targets --features ssr -- -D warnings`,
  `SQLX_OFFLINE=true cargo test -p web --features ssr rules`.
- `bash scripts/rust-test.sh` before any commit. SCSS changes need a style build
  to visually confirm.

---

## Unit 9 - Email settings (per-type toggles) + reorder (migration 018) (#10)

### Items covered
Item 10 (split the single email gate into independent turn/invite/reminder
toggles, expose them in settings, reorder settings sections, email-command
parity). Single nudge + on/off model - NO frequency column.

### Spec
- Today `users.turn_emails_enabled` (bool NOT NULL DEFAULT true,
  `014_email_play.sql`) gates turn + invite + reminder emails together. Split it
  into three independent toggles. Reminders stay a single 24h nudge; the toggle
  owns on/off.
- Add a settings UI section (fire-and-forget toggles), reorder sections so theme
  is last, and add email-command parity.

### Plan
**(a) Migration 018** - `rust/web/migrations/018_email_preferences.sql`:
```sql
ALTER TABLE public.users
    ADD COLUMN IF NOT EXISTS invite_emails_enabled boolean NOT NULL DEFAULT true;
ALTER TABLE public.users
    ADD COLUMN IF NOT EXISTS reminder_emails_enabled boolean NOT NULL DEFAULT true;
```
- Defaults true => existing users keep getting emails (no backfill needed).
- NO `reminder_frequency` column (user chose single nudge + on/off).

**(b) Split the gate:**
- TURN: keeps `turn_emails_enabled` (no change beyond exposing the toggle in UI).
- INVITE: change the three `!recip.turn_emails_enabled` checks
  (`proposals.rs:187,287,335`) to `!recip.invite_emails_enabled`; add the column
  to `fetch_invite_recipient` (`proposals.rs:150`, plain `query_as` selecting
  `COALESCE(u.turn_emails_enabled, false)` at :155) + the `InviteRecipient` struct
  (`proposals.rs:143-147`, `turn_emails_enabled` at :145).
- REMINDER: change `u.turn_emails_enabled = true` (`sweep.rs:72`, in
  `fetch_candidates` `sweep.rs:61`) to `u.reminder_emails_enabled = true`.
  Reminder cadence stays a single 24h nudge (`turn_reminder_sent_at`;
  `DEFAULT_REMINDER_THRESHOLD` `sweep.rs:9`) - do NOT make reminders repeat.

**(c) Settings UI** (`rust/web/src/settings.rs`):
- New `EmailPreferencesSection` copying `ColorsSection` (`settings.rs:112`)
  fire-and-forget pattern: `ServerAction` dispatched on change, no Save button;
  "adopt once" `initialized` `RwSignal` (`settings.rs:124-132`).
- Three independent toggles (turn / invite / reminder), each wrapped in
  `<FormField>`. (The turn toggle is newly exposed in UI; it had no settings UI
  before - only email commands.)
- Setter server fns `set_email_invite_enabled(bool)` / `set_email_reminder_enabled(bool)`
  (and a turn setter if not already present) following `set_pref_colors`
  (`auth/server.rs:608`): guard `get_current_user`, plain UPDATE, `Ok(())`.
  Expected rejections are data not `ServerFnError`, but since the client only
  offers valid options a `ServerFnError` on invalid input is acceptable (matches
  `set_pref_colors`).
- `SettingsData` (`auth/server.rs:526`) + `get_settings` (`auth/server.rs:533`)
  gain the new fields (PLAIN queries).

**(d) Reorder sections** (`settings.rs:35-38`): current order Username, Colours,
Theme, EmailAddresses. Target: Username, Colours, EmailPreferences, EmailAddresses,
Theme (theme LAST - it is huge). Move `<EmailSection/>` (`settings.rs:38`) above
`<ThemeSection/>` (`settings.rs:37`) and place the new `EmailPreferencesSection`
adjacent (both above Theme).

**(e) Email-command parity** (`rust/web/src/email/commands.rs`):
- Extend `help_text` (`commands.rs:153`), `SettingsSummary`/`format_settings_summary`
  (`commands.rs:183,190`).
- Add `emails invite on/off` + `emails reminder on/off` subcommands, modeling on
  `run_emails_toggle` (`commands.rs:563`) and `set_turn_emails_enabled`
  (`commands.rs:743`). The existing `emails on/off` / `subscribe` / `unsubscribe`
  (`commands.rs:42,971`) map to the TURN toggle.

### Gotchas
- **Migration immutability:** never touch 014; new columns in 018.
- **`.sqlx` regen:** use plain (non-macro) queries for the new columns. The
  existing `fetch_email_recipient`/`fetch_invite_recipient` are already plain
  `query_as`, so adding columns there is safe.
- **Hydration:** toggles are attribute-only reactive bindings; follow
  `ColorsSection`'s "adopt once, signal is source of truth" idiom. No conditional
  element swaps on the pref values.
- **`db.rs`/`auth/` changes REQUIRE `#[sqlx::test]` tests.**
- **Defaults:** all enabled (DEFAULT true) - requirement met by migration
  defaults; no backfill.

### Tests (`#[sqlx::test]`)
- New db readers/writers (the new columns + setters).
- The send-path gate changes: invite gate (`proposals.rs` invite flow respects
  `invite_emails_enabled`), reminder gate (`sweep.rs` `fetch_candidates` respects
  `reminder_emails_enabled`). Existing sweep tests (`sweep.rs:547,603`) insert
  users with `turn_emails_enabled` - update/extend for the new column.

### Verification
- `cargo fmt --all -- --check`,
  `cargo clippy -p web --all-targets --features ssr -- -D warnings`.
- `bash scripts/rust-test.sh` before any commit.

---

## Unit 10 - Email "bump" command (#11)

### Items covered
Item 11 (re-send all my-turn games to the active address via an email `bump`
command).

### Spec
A new email command `bump` that re-sends all of a user's my-turn games (one email
per game) to their active/primary verified address, plus one Status confirmation
email. Always sends regardless of `turn_emails_enabled` (explicit pull). Capped
at 20 games (parity with the 22d switch-digest), no cooldown.

### Plan
**(a) Handler** (in `rust/web/src/email/commands.rs`), reusing the 22d loop
(`auth/server.rs:740-744`):
```rust
async fn run_bump_command(ctx: &StandaloneCommandCtx<'_>) -> Result<CommandReply, CommandError> {
    let games = crate::db::find_active_turn_games(ctx.pool, ctx.user_id, crate::db::SWITCH_DIGEST_CAP)
        .await.map_err(|e| CommandError::Internal(anyhow::anyhow!("bump: find turn games: {e}")))?;
    let capped = crate::db::cap_digest(games, crate::db::SWITCH_DIGEST_CAP);
    let n = capped.len();
    for (game_id, game_player_id) in capped {
        crate::email::notify::send_turn_digest(ctx.resend, ctx.pool, ctx.http_client, game_id, game_player_id).await;
    }
    Ok(CommandReply::Status(match n {
        0 => "No games are waiting on your turn.".to_string(),
        1 => "Re-sent 1 game to your active address.".to_string(),
        n => format!("Re-sent {n} games to your active address."),
    }))
}
```
- `find_active_turn_games(pool, user_id, cap)` (`db.rs:2420`) ->
  `Vec<(game_id, game_player_id)>`, is_turn + not finished, oldest turn first.
- `cap_digest(items, SWITCH_DIGEST_CAP)` (`db.rs:2258`, cap=20 `db.rs:2225`).
- `send_turn_digest(resend, pool, http, game_id, game_player_id)` (`notify.rs:309`)
  - full turn-notification re-send to the verified primary (active) address
  (`fetch_email_recipient` `outbound.rs:255`), each with its own Reply-To. Reuses
  `render_game_email` unchanged (inherits the `docs/email.md` Gmail fix) - do NOT
  build a new render path.
- `StandaloneCommandCtx` (`commands.rs:283`) has pool, http_client, broadcaster,
  jetstream, resend, user_id - everything bump needs.
- `CommandReply` enum (`commands.rs:13`): `GameMove | Status(String) |
  FullContent{html,text}`.

**(b) ALWAYS SEND bypass:** `send_turn_digest` -> `send_one` bypass path checks
`recipient.turn_emails_enabled` (`notify.rs:231-232`). Add a bypass (a flag or
dedicated send path) so bump sends even when turn emails are disabled (user
decision: bump is an explicit pull). Do not change the default behaviour for
non-bump sends.

**(c) Wiring:**
- Recognise `bump` in the standalone path: `dispatch_standalone_server_command`
  (`commands.rs:292`) and/or `dispatch_settings_standalone` (`commands.rs:259`).
- Add a `bump` arm in `dispatch_email_command` (`commands.rs:939`) that builds a
  `StandaloneCommandCtx` from the `EmailCommandCtx` (same as the existing "new"
  arm around `commands.rs:954-966`).
- Add `bump` to `help_text()` (`commands.rs:153`).
- Reply composition: one email PER GAME (the re-sent turn notifications) PLUS one
  Status confirmation from the existing reply path (`send_settings_response`
  `inbound.rs` / `send_game_reply_response`). Side-effecting sends happen inside
  the handler (like `run_new_command` calls `notify_game_emails`).

### Gotchas
- **Abuse backstop:** the 20-game cap only (parity with 22d), NO cooldown.
  Repeated bumps re-send again (`send_turn_digest` marks nothing). The 22d digest
  has no cooldown either.
- **Gmail hazard:** bump reuses `render_game_email` unchanged, so the
  `<tr><td font-size:13px>` fix (`email/render.rs:171`) is inherited - no new
  rendering risk.
- **Hydration:** N/A (server-side email command, no UI).
- Sender authorisation: From must match a verified address
  (`resolve_user_by_verified_from`, `inbound.rs`); From is never trusted alone.

### Tests
- **Parsing test:** `bump` verb recognised, case-insensitive (mirror
  `subscribe_toggle` tests `commands.rs:1014-1020` / `settings_verb_is_case_insensitive`).
- **`#[sqlx::test]`:** seed two my-turn games + one finished/non-turn game, call
  the handler, assert the Status count ("Re-sent 2 games..."). Sends are
  fire-and-forget (no Resend in test => logged), so assert on the returned count,
  not outbound delivery. Model on `settings_standalone_rejects_game_command`
  (`inbound.rs:1567`) and the `dispatch_settings_command_for_user` tests.

### Verification
- `cargo fmt --all -- --check`,
  `cargo clippy -p web --all-targets --features ssr -- -D warnings`.
- `bash scripts/rust-test.sh` before any commit.

---

## Appendix - Key file map

Most-touched files, one line each:

- `rust/web/src/settings.rs` - Settings page; section order (:35-38), `ColorsSection` fire-and-forget template (:112), `ThemeSection` (:408), `EmailSection` (:179). Unit 9.
- `rust/web/src/friends.rs` - Friends page + server fns; `INVITE_POLICIES` labels (:12-15), unfriend handler (:440-443), "Decline and block" confirm (:412), heading (:498), `Unfriend` server fn (:195). Units 1, 2.
- `rust/web/src/players.rs` - Profile pages; `PlayersPage` (:145), `PlayerGameTypePage` template (:442), profile header (:185-187), helpers `encode_path_segment` (:34), `format_placing` (:61), `opponents_view` (:71), `rating_trend` (:15, private), FormStrip use (:311). Units 1, 4, 6, 7.
- `rust/web/src/proposals.rs` - Proposals/invites; `create_proposal` (the bug, :877-884), `InviteRecipient`/`fetch_invite_recipient` (:143-163), invite gates (:187,287,335), invite email text (:206), `ProposalOutcome` (:54-56). Units 3, 9.
- `rust/web/src/new_game.rs` - New-game form; `NewGamePage` (:121), `GameBrowser` selection state (:148-149), `select_game` (:189), `create_proposal` call (:208), raw error render (:133,445). Units 3, 7.
- `rust/web/src/stats/mod.rs` - Stats server fns + DTOs; `FormResult` (:81), `get_player_profile` (:129), `get_player_game_type_stats` (:186). Units 4, 6, 7.
- `rust/web/src/stats/queries.rs` - Stats DB queries; `find_game_type_name` (:31), `rating_series` (:156), `opponents_by_game` (:197), `finished_games` (:233), `recent_form` (:401), `recent_form_for_game_type` (:474), fixtures (:540-686), peak-backfill test (:1228). Units 5, 6, 7.
- `rust/web/src/stats/viz.rs` - Stats visualisation; `sparkline`/`Sparkline` (:17,:37), `form_cell` (:43), `FormStrip` (:66), `form_cell` tests (:317-337). Units 4, 7.
- `rust/web/src/components/game.rs` - Game components; force-delete confirm (:166), concede confirm (:125), `PlayerInfo` FormStrip (:239), `Invalid command` render (:549). Units 1, 4.
- `rust/web/src/rules.rs` - Rules rendering; `RulesPage` (:22), `render_markdown` (:150), `render_doc` (:162), `get_rendered_rules` (auth-gated, :297,:304), tests (:371+). Unit 8.
- `rust/web/src/email/outbound.rs` - Email send choke point + recipient; `try_send_rendered_email` (:165), `EmailRecipient` (:243), `fetch_email_recipient` (:255), `should_email_recipient` (:281). Units 9, 10.
- `rust/web/src/email/notify.rs` - Turn/invite/reminder notification; `send_one` opt-out check (:192,:231-232), `send_turn_digest` (:309). Units 9, 10.
- `rust/web/src/email/sweep.rs` - Turn-reminder sweep; `DEFAULT_REMINDER_THRESHOLD` (:9), `is_reminder_candidate` (:31), `fetch_candidates` gate (:61,:72). Unit 9.
- `rust/web/src/email/commands.rs` - Inbound command parser/dispatch; `CommandReply` (:13), `help_text` (:153), `SettingsSummary`/`format_settings_summary` (:183,:190), `dispatch_settings_standalone` (:259), `StandaloneCommandCtx` (:283), `dispatch_standalone_server_command` (:292), `run_emails_toggle` (:563), `set_turn_emails_enabled` (:743), `run_rules` (:879), `dispatch_email_command` (:939). Units 8, 9, 10.
- `rust/web/src/email/inbound.rs` - Inbound routing; email HTML wrap (:891), `handle_settings_reply` (:944), `settings_standalone_rejects_game_command` test (:1567). Units 8, 10.
- `rust/web/src/auth/server.rs` - Auth server fns; `SettingsData`/`get_settings` (:526,:533), `set_pref_colors` setter template (:608), 22d switch-digest loop (bump template, :740-744). Units 9, 10.
- `rust/web/src/db.rs` - DB layer; `find_game_version_rules` plain-query pattern (:254-264), `find_available_game_types` (:282), `apply_rating_changes` (rating write, :1348; rating SELECT :1409; write loop :1472-1484), `get_invite_policy` (:1897), `SWITCH_DIGEST_CAP` (:2225), `cap_digest` (:2258), `find_active_turn_games` (:2420), rating test helpers (:4031,:4042). Units 5, 6, 7, 10.
- `rust/web/src/game/mod.rs` - Game command types; `ExecuteCommandError::UserError` (:66,:72). Unit 3 (typed-error reference).
- `rust/web/src/game/server_fns.rs` - Game server fns; `BotSlot` (:9), `blurb` DTO field (:98), `Ok(Some(msg))` typed-error mapping (:313), `CreateGameSeed`/`create_game_from_service` (:352,:367), `create_new_game` (Option-based sibling, :461,:480-482). Unit 3.
- `rust/web/src/app.rs` - Router; routes (:193-204): `games` (:195), `players/:name` (:200), `players/:name/:game_type` (:201), `games/:id` (:202), `rules/:version_id` (:203), `GamePage` (:594). Units 6, 7.
- `rust/web/style/main.scss` - All styles; `.login h1` (:102), `.rules-page .rules-doc .game-render` (:551), `.settings` (:561), `.color-ribbon` (:641), `.form-strip`/`.form-win`/`.form-loss`/`.form-none` (:651-665), `.new-game-panel h2` (:751). Units 1, 4, 8.
- `rust/bot/system_prompt.md` - Static bot system prompt; `# Persona` (:1-3), `# Task` (:5-9). Unit 8.
- `rust/bot/src/prompt.rs` - Bot prompt rendering; `include_str!` (:5), `render_system` (:65), tests (:138+), parser-docs test (:203). Unit 8.
- `rust/bot/src/main.rs` - Bot entry; LLM 300s timeout (:783), game-service 60s (:788). Unit 8.
- `rust/web/migrations/` - Immutable migrations; latest `016_invite_nudge.sql`. New: `017_game_player_rating_before.sql` (Unit 5), `018_email_preferences.sql` (Unit 9). Reference: `010_friends.sql` (invite_policy CHECK), `012_game_type_blurb.sql` (blurb), `014_email_play.sql` (turn_emails_enabled).
- `rust/web/tests/ssr_pages.rs` - In-process SSR page tests; helpers `make_state` (:35), `make_user` (:57), `login_cookie` (:72), `make_game_version` (:130), `spawn_mock_new_game_service` (:389), `restart_game_via_http` (:428), `restart_game_on_finished_game_succeeds` (:457), form-strip assertion (:941). Units 3, 4, 6, 7.
- `rust/web/src/error.rs` - `internal()` opaque-error helper (:6). Unit 3.
- `rust/web/src/models/game.rs` - `GameType.blurb` (:14). Unit 7.
