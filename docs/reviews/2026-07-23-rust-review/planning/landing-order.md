# Landing order - sequencing facts for the implementing agent

Written 2026-07-25 (refinement unit Lead). Every claim below was **verified by
reading source and planning docs**; no build, test or git-mutating command was run.

Read this before speccing or implementing any of **WP-40, WP-41, WP-54, WP-56,
WP-59**. It exists so these three interactions are in one place instead of buried
in five spec files.

---

## 1. WP-41 must land before WP-40 - VERIFIED

**WP-41 has NOT landed.** WP-41 is `db.rs quality pass - READY`, 16 findings
(ws F35-F51), single path `web/src/db.rs`.

Evidence it is not in the tree:

- `rust/web/src/db.rs` still carries the manual `updated_at = NOW()` clauses at
  approximately :1300, :1321, :1397, :1427, :1444 - WP-41 Task 1 deletes them.
- The file still opens with `use` statements; WP-41 adds a module doc header.
- `git log --oneline -40` contains **zero** `WP-*` commits. The most recent work is
  the `(#47)` concede/end-game series.

**Why WP-40 depends on it** (`specs/WP-40-undo-concede-toctou-ratings-integrity.md`,
approximately :47-65): WP-41 touches all three of WP-40's `db.rs` functions, and
every touch is a **deletion**:

- removes the manual `updated_at = NOW()` clauses from `concede_game`,
  `concede_game_replace`, `end_game`, `undo_game` and `apply_rating_changes`
  (WP-41 Task 1)
- rewrites `apply_rating_changes`'s all-pairs loop header to slice form
  (WP-41 Task 5)
- makes `is_finished` sticky in `update_game_command_success` (WP-41 Task 3)

WP-40's spec already instructs: **"If WP-41 has NOT landed: stop and say so."**
That instruction is live. Land WP-41 first.

The same constraint is recorded at `critical-path.md` approximately :278 ("must
land before WP-40 and WP-47") and :286, and in `specs-LOG.md` approximately
:2115-2118.

---

## 2. WP-59 vs WP-56 - the conflict is SMALLER than previously reported

The prior Lead reported that **WP-59 Task 10 and part of Task 9 fix commands that
WP-56 deletes**. Rechecked under the 2026-07-25 D-1 refinement, which narrows
WP-56 to deleting **only** `emails add`, `emails confirm`, `emails active`/`use`
and `emails remove`.

WP-56 Task 4, verbatim: *"delete the `"add"`, `"confirm"`, `"active" | "use"` and
`"remove"` match arms. Keep `on`/`off`/`invite`/`reminder` and the bare `emails`
listing (`run_emails_list` - read-only, harmless)."*

### Verdict per WP-59 task

| WP-59 item | Target | Fate under WP-56 |
|---|---|---|
| **Task 10** (wfe F23, minor) - `emails confirm <code>` matches any pending address | rewrites `run_emails_confirm` | **FULLY DEAD.** WP-56 deletes the whole function. |
| **Task 9** - `error.rs`: add `pub const INTERNAL_ERROR_MESSAGE`, use it in `internal` | `error.rs` | **SURVIVES** |
| **Task 9** - add `classify_server_fn_error` after the `CommandError` enum (~commands.rs:25) | `commands.rs` | **SURVIVES - and WP-40 explicitly consumes it** (WP-40 spec ~:68-71) |
| **Task 9** - `map_err` site 1: `run_restart` (~commands.rs:1113) | the `restart` **server** verb, not an `emails` subcommand | **SURVIVES.** This is the wfe F21 **major** and the higher-value half: it fixes both the misclassification and the `"error running server function: "` prefix leaking into every failed `restart` reply. |
| **Task 9** - `map_err` site 2: inside `run_emails_confirm` (~commands.rs:747, wfe F24, minor) | deleted function | **DEAD** |
| **Task 11** - 2 of its 6 inline-SQL sites (~:732-737, ~:751-755) | inside `run_emails_confirm` | **DEAD**, and its new `db::delete_login_confirmation` helper becomes unnecessary |
| **Task 11** - the other 4 sites (`run_settings_summary` ~:827-834 plus the three pref toggles) | KEPT verbs | **SURVIVE** |

### Net

Two **minor** findings become no-ops (wfe F23, wfe F24), plus two of Task 11's six
sites. **Nothing major is lost**, and **WP-40's dependency on Task 9's
`classify_server_fn_error` is untouched**. The reported conflict was real but
over-stated: Task 9 survives roughly three-quarters intact, and its major half is
entirely unaffected.

**Either order is fine.** If WP-56 lands first, whoever executes WP-59 drops
Task 10, Task 9's second `map_err` site, and Task 11's two `run_emails_confirm`
sites as no-ops. If WP-59 lands first, WP-56 deletes them as expected dead code -
note it in the commit message so nobody re-adds the command to satisfy the older
spec.

### Email settings verb inventory (as of 2026-07-25, `rust/web/src/email/commands.rs`)

Recorded so the WP-56 narrowing can be stated by exact verb name.

- `settings_verb` (~:224-232) top-level verbs: `name`, `colors` | `colours`,
  `theme`, `emails`, `settings`
- `run_settings_emails` (~:547-603) subcommands: bare `emails` (list), `on`, `off`,
  `add`, `confirm`, `active` | `use`, `remove`, `invite on` | `invite off`,
  `reminder on` | `reminder off`
- In `dispatch` (~:1215-1244) but **not** the settings surface: `concede`, `end`,
  `undo`, `restart`, `rules`, `help` | `commands`, `new`, `bump`, `list`; plus
  `subscribe`/`unsubscribe` (~:43-45, account-wide turn notifications)

**WP-56 DELETES exactly:** `emails add`, `emails confirm`, `emails active`,
`emails use`, `emails remove`.
**WP-56 KEEPS:** `name`, `colors`/`colours`, `theme`, `settings`, bare `emails`,
`emails on`, `emails off`, `emails invite on|off`, `emails reminder on|off`.
There is no separate top-level colour verb beyond `colors`/`colours`, and `theme`
takes a name argument (`theme system` = default). Both are on the KEEP side.

---

## 3. WP-40's new conflict errors are INVISIBLE in the UI until WP-54 lands - VERIFIED

WP-40 introduces a new `db::GameAlreadyFinished` alongside the existing
`StaleStateConflict`, so "already finished" and "someone moved first" get
distinguishable text (`specs-LOG.md` ~:2097-2099). Those messages are returned as
`ServerFnError`.

**They render nowhere today.** `GameMeta`'s five `ServerAction` effects in
`rust/web/src/components/game.rs` match `Some(Ok(()))` and drop `Some(Err(_))` on
the floor - approximately :58-63 (undo), :64-69 (concede), :70-75 (end game),
:80-85 (force delete); `bump_bot_action` has no watcher at all. Because the
mutation failed there is no websocket bump, so no refetch, so **the user's only
signal is the absence of change**.

**WP-54 Task 1 is the fix** (WP-54 spec ~:257): one shared
`RwSignal<Option<String>>` on `GameMeta`, rendered under the "Actions" heading,
plus a new `crate::error::action_error_message` that unwraps
`ServerFnError::ServerError(msg)` so a deliberate rejection survives without the
framework prefix.

WP-40's spec states this explicitly (~:75-79): *"WP-54 ... Task 1 adds the shared
error slot that finally renders `undo_action` / `concede_action` failures in
`components/game.rs`. Either order. WP-40 does not edit `components/game.rs` -
your guard messages are returned as `ServerFnError`; WP-54 makes them visible."*
Reinforced at ~:461: *"NO frontend work: `components/game.rs` is WP-54's (error
slot)"*.

**Consequence for sequencing:** no hard ordering constraint - WP-40 may land
first, it just **ships mute on the web surface**. If WP-40 lands without WP-54,
do not treat the silence as a WP-40 bug. The **email** surface is unaffected: it
goes through `CommandError`, via WP-59 Task 9's `classify_server_fn_error`.

Already recorded at `specs-LOG.md` ~:2122-2125: *"Without WP-54, the new conflict
errors will be silent on the web surface - flag to the Orchestrator when
sequencing."*

---

## 4. Recommended order for this cluster

1. **WP-41** (db.rs quality pass) - hard prerequisite for WP-40.
2. **WP-40** (undo/concede TOCTOU + ratings integrity).
3. **WP-54** (frontend error slot) - as soon as practical after WP-40, since it is
   what makes WP-40's guards visible. Landing it *before* WP-40 is also fine.
4. **WP-56** and **WP-59** - either order, per section 2. Whichever goes second
   drops the dead items listed there.

WP-42 / WP-47 / WP-84 are a separate cluster, **re-ordered 2026-07-26 by the SSE
pivot (D-44)** - see section 10. **WP-47 first**, so WP-42's per-connection
filter calls its `is_game_visible_to_viewer` dispatcher rather than forking the
predicate; then **WP-42's predicate work only** (its pre-upgrade WebSocket auth
is superseded and must NOT be written); then **WP-84** (the SSE migration).

---

## 5. Line-number caveat

Every line number in this file is an **approximate navigational hint, verified at
2026-07-25**. Locate code by file plus function name. If what you find does not
match the description here, **stop and report** - do not adapt the edit.

---

## 6. Constraints added by the T2-B1/B2 and T2-B3/B4 spec Leads

Line numbers are not used here on purpose. Locate code by file plus function
name; if what you find does not match, **stop and report**.

### 6.1 Auth cluster (from the T2-B1/B2 Lead)

**WP-41 -> WP-36 -> WP-34 -> WP-35.**

WP-36 changes `crypto::load_key`'s return type and WP-35 rewrites the same
function, so WP-36 must land first or WP-35's rewrite will be re-derived
against the wrong signature. WP-41 stays at the head of the chain (it is
already a hard prerequisite for every `db.rs` consumer).

### 6.2 Delivery / ack cluster (T2-B3)

- **WP-37 -> WP-38.** Both touch `rust/web/src/admin.rs`, and WP-37 Task 1
  reshapes every `#[server]` fn there. If WP-38 lands first, WP-37 Tasks 6-7
  must be re-derived.
- **WP-59 -> WP-57.** Both own `rust/web/src/email/inbound.rs`. WP-57 also
  widens WP-59's new `fetch_inbound_text(state, email_id)` from an
  `Option`-shaped to a `Result`-shaped return so a failed fetch is
  distinguishable from an empty body. WP-57 changes the shape only, not the
  body - but the WP-59 implementer must know the change is coming.
- **WP-38 and WP-46 both own `rust/web/src/email/sweep.rs`** (WP-46 is blocked
  on D-11). WP-38's edit there is purely additive - one new sweep plus one
  extra `spawn_periodic_sweeps` parameter - so either order works, but
  whichever lands second must **rebase on, not fork,** the sweep scaffolding.
- **WP-51 -> WP-46** (added by the WP-46 spec Worker, 2026-07-26; D-11 is now
  answered so WP-46 is unblocked). WP-51 rewrites `send_reminder`'s body
  (`NotifyKind::Reminder`), the six `RealInviteMailer` methods, and collapses
  the five `spawn_*` interval loops into one helper. WP-46 then changes
  `send_reminder`'s **return type** (`ReminderOutcome`) and its **recipient
  gate** (D-11), splits `RealInviteMailer::send_invite` into an awaited core
  plus a `tokio::spawn` wrapper, and adds a `resend` parameter to
  `spawn_invite_auto_decline_sweep` - which means `spawn_periodic_sweeps` gains
  a parameter from **both** WP-46 and WP-38. Either order compiles; whichever
  lands second rebases on the other's shape rather than reintroducing its own.
  WP-46 makes exactly one `outbound.rs` edit (a `reminder_emails_enabled` field
  on `EmailRecipient` + its SELECT), the sole agreed exception to WP-60's
  `outbound.rs` fence. WP-57 and WP-76: checked, no collision.

### 6.3 Game-state boundary cluster (T2-B4)

- **WP-09 is SPLIT into WP-09a then WP-09b.** WP-09a (requester boundary)
  introduces the defaulted no-op `Gamer::validate(&self) -> Result<(),
  GameError>` hook in `rust/lib/game/src/game.rs`; WP-09b's whole per-crate
  sweep consists of filling that hook in. **WP-09a is a hard prerequisite for
  WP-09b.**
- **WP-09a -> WP-21 Task 10.** WP-21 Task 10 refactors sushizock-2's
  `steal_blue` / `steal_red` into a shared helper. WP-09a adds a
  `target < self.players` bounds guard to both. Task 10 must **carry that
  guard forward into the helper**, not drop it during the refactor.
- **WP-28 Task 3 deliberately leaves `self.hands[player]` panicking** so
  WP-09a's red test stays reproducible. That is correct. Do not "fix" it
  early and do not widen WP-28.
- **WP-06 must not be retro-edited** to carry the `gamer.rs` bounds check -
  that check belongs to WP-09a.
- **WP-10 is independent of WP-13.** WP-13 Task 5 render-guards
  starship-catan-1's `peeking`; WP-10 fixes the same leak at the JSON level
  with a one-line `player == self.current_player` guard in `player_state()`.
  Neither collides; either order works. As of 2026-07-25 WP-13 has not landed
  and the leak exists at both levels.

### 6.4 Email canonicalization (WP-50, T2-B5)

- **WP-50 -> WP-78. WITHDRAWN - see section 7.3.** The item is now WP-82 (WP-78
  is SUPERSEDED) and the direction is reversed: **WP-82 -> WP-50**.
- **WP-50 is independent of WP-56 and WP-59.** WP-50 touches
  `auth/server.rs`, `proposals.rs`, `game/server_fns.rs`, `new_game.rs`,
  `settings.rs`, `db.rs` (comments only) and a new migration. It does **not**
  touch `email/inbound.rs` or `email/commands.rs`. Either order works.
- **Migration numbering collision: FOUR packages, not two.** Corrected
  2026-07-26 by direct read of all four specs and of
  `rust/web/migrations/` (`ls` only). The highest existing migration really is
  **`022_concede_bot_replacement.sql`**, so every spec's "022 is highest today"
  premise is currently true - but only one of them can be `023`.

  | Package | Filename the spec names | What it adds |
  |---|---|---|
  | **WP-34** auth races and session mechanical | `023_login_email_sends.sql` (hard-coded, spec sections 0 and 3) | login-email send-rate table |
  | **WP-50** email canonicalization | `023_canonical_emails.sql` (hard-coded, spec 3e) | lowercase backfill + `lower(email)` unique index |
  | **WP-56** email-from-auth redesign | `0NN_settings_email_token.sql` ("next free number") | `users.settings_email_token` |
  | **WP-58** unsubscribe / RFC 8058 | `0NN_unsubscribe_token.sql` ("next free number") | `users.unsubscribe_token` |

  **WP-34 and WP-50 name the same file, `023`.** A direct filename clash, not
  just a number clash.

  **Rule: only the package that lands FIRST may use `023`. The second, third and
  fourth must each renumber to the then-next free number** (`024`, `025`, `026`
  in landing order) **and must not collide with each other either** - re-`ls`
  `rust/web/migrations/` immediately before writing the file, do not trust the
  number written in the spec. Migrations are immutable once applied: renumber
  before landing, never edit an applied file.

  Note WP-34 also lands early in the auth chain (6.1: `WP-41 -> WP-36 -> WP-34
  -> WP-35`), so in the current plan it is the likeliest `023` and WP-50/56/58
  should all expect to renumber.

### 6.5 Unsubscribe / RFC 8058 (WP-58, T2-B5)

- **WP-59 -> WP-58.** WP-59 Task 5 shrinks `handle_settings_reply_route` and
  explicitly defers every unsubscribe concern to WP-58/D-10. WP-58 must not
  land first and re-derive that route.
- **WP-56 -> WP-58.** WP-56 maps `unsubscribe@brdg.me` to the inbound ignore
  arm (which is only correct once WP-58 has removed the advertised mailto) and
  adds `ensure_settings_email_token` to `email/outbound.rs`; WP-58 adds
  `ensure_unsubscribe_token` immediately beside it. Landing WP-56 first avoids
  a merge in that file.
- **WP-58 takes a second exception to WP-60's `outbound.rs` fence** - exactly
  one new function, `ensure_unsubscribe_token`. (WP-46's
  `reminder_emails_enabled` field on `EmailRecipient` is the first, see 6.2.)
- **WP-58 vs WP-51 / WP-46 / WP-38.** WP-58 appends a 7th parameter to
  `render_game_email` and touches its call sites in `email/notify.rs`
  (`send_one`) and `email/sweep.rs` (`send_reminder`) - two functions WP-51 and
  WP-46 rewrite. Either order compiles; whichever lands second **rebases on,
  not forks,** the other's shape.
- **Migration numbering: WP-58 also adds one** (`users.unsubscribe_token`).
  It is one of the four packages in 6.4's collision table - **see 6.4 for the
  full list (WP-34, WP-50, WP-56, WP-58) and the renumbering rule.**

### 6.6 Turnstile full-page-load for `/login` (WP-55, T2-B6)

- **WP-54 -> WP-55.** Both rewrite the **same** `Effect` in
  `rust/web/src/components/layout.rs::SidebarMenu`: WP-54 Task 4 replaces the
  `is_some_and(|r| r.is_ok())` logout effect with a three-arm `match` plus an
  error slot; WP-55 then swaps the `navigate("/login", …)` inside that new
  `Some(Ok(()))` arm for a hard `crate::app::hard_navigate("/login")`. WP-54 is
  READY, WP-55 was blocked on D-16 (now answered). WP-54's own spec already
  fences this ("do not convert `/login` links to hard navigations") and states
  the same order. **WP-55 rebases onto WP-54's arm; it must not fork it.**
- **WP-37 -> WP-38 -> WP-55 in `rust/web/src/admin.rs`.** 6.2 already records
  WP-37 -> WP-38. WP-55 adds a third: it edits `AdminPage`'s anonymous
  `/login` redirect effect and collapses the `navigate` / `navigate2` clone
  pair, while WP-37 rewrites the **adjacent** non-admin `"/"` bounce effect in
  the same function. Different effects, same statement block - land WP-55 last
  and keep WP-37's rewritten `"/"` bounce as-is.
- **No conflict in `app.rs` or `settings.rs`.** WP-54's `app.rs` edits are the
  latches, the `GamePage` error line, the `friend_request_count` hoist and
  `LoginPage`'s `show_code_link` / two click-only anchors; WP-55 touches
  `HomePage`'s `index-cta` link and adds a new `hard_navigate` helper beside
  `set_theme_client`. WP-54's `settings.rs` edits are `ColorsSection`,
  `EmailPreferencesSection`, `EmailSection` and `ThemeSection`; WP-55 touches
  only `SettingsPage`'s redirect effect and the file's imports. Disjoint.

---

## 7. WP-82 (`db.rs` module split) is a hard predecessor for the web cluster

Added 2026-07-26 by the WP-82 spec Lead. Verified by reading
`work-packages.md` and `rust/web/src/db.rs`; no build, test or git-mutating
command was run.

### 7.1 The constraint

**WP-82 lands FIRST, before every remaining package that writes into
`rust/web/src/db.rs`:**

| Package | Why it depends on WP-82 |
|---|---|
| **WP-35** auth edge semantics | lists `web/src/db.rs` in its paths |
| **WP-40** undo/concede TOCTOU + ratings | edits `concede_game`, `concede_game_replace`, `end_game`, `undo_game`, `apply_rating_changes` |
| **WP-45** bot-slot validation | lists `web/src/db.rs` |
| **WP-47** game_visibility gates | adds `is_game_visible_to_viewer` and `visible_user_ids` to `db.rs` |
| **WP-49** rules and game-info pages | lists `web/src/db.rs` |
| **WP-50** email canonicalization | `db.rs` doc-comment edits |
| **WP-52** stats and query performance | lists `web/src/db.rs` |
| **WP-53** domain misc server fns | lists `web/src/db.rs` |
| **WP-59** inbound processing quality | lists `web/src/db.rs` |
| **WP-42** realtime visibility predicates | adds `is_proposal_visible_to_user` to `db.rs` and consumes WP-47's `is_game_visible_to_viewer` (rescoped 2026-07-26, see section 10) |
| **WP-84** SSE migration | consumes WP-47's and WP-42's `db.rs` predicates; lands after both |

**WP-41** (db.rs quality pass) is the one exception: it has **already landed**
(+1397/-125) and WP-82 is specced against the post-WP-41 shape.

### 7.2 Why this order and not the other one

The split is a **pure move**: same functions, same SQL, same signatures, and
`pub use` re-exports in `db/mod.rs` keep all 293 external `crate::db::foo(...)`
call sites compiling unchanged. So landing it first costs the ten packages
above **nothing** except locating their target function in a smaller file.

Landing it last costs every one of them a rebase onto a moved file, and makes
the split itself a merge against ten sets of edits to the file it is moving.

### 7.3 This REVERSES section 6.4's `WP-50 -> WP-78`

`landing-order.md` 6.4 records *"WP-50 -> WP-78: the `db.rs` module split waits
for WP-50's `db.rs` doc-comment edits."* **That constraint is withdrawn.**

- The item is now **WP-82**, not WP-78. `work-packages.md`'s WP-78 entry is
  marked SUPERSEDED and retained only so the 6.4 reference resolves.
- The direction is now **WP-82 -> WP-50**. WP-50's `db.rs` change is
  doc-comment only; it applies just as easily to `db/mod.rs` after the split,
  and WP-82 rewrites the module doc comment's "Module map" section anyway.
- Everything else in 6.4 (WP-50 vs WP-56/WP-59 independence, the migration
  numbering collision - now WP-34, WP-50, WP-56 and WP-58) is unaffected.
- 6.4's first bullet has been amended in place to point here.

### 7.4 What does NOT change

- Section 4's intra-cluster order still holds **after** WP-82: WP-40 then
  WP-54; WP-56 and WP-59 either order; WP-47 before WP-42 (and, since
  2026-07-26, WP-42 before WP-84 - see section 10).
- Section 6.1's `WP-41 -> WP-36 -> WP-34 -> WP-35` chain is unaffected; WP-82
  simply inserts after WP-41 and before WP-35.
- Non-`db.rs` packages (the game crates, the Tier 3 checklists, WP-51, WP-54,
  WP-55, WP-56, WP-57, WP-58) are **not** gated on WP-82 and may proceed in
  parallel.

---

## 8. The dependency cluster (WP-64..WP-73) - added by the dependency-cluster spec Lead, 2026-07-26

Sources: D-17, D-18, D-19, D-21, D-22, D-23, D-24 in `decisions-ANSWERED.md`,
plus the standing process constraint recorded there.

### 8.0 The binding precondition on the whole cluster

**Every package in this cluster opens with "upgrade all dependencies to latest
and see where we stand."** This is Michael's standing strategy (stay close to
latest so deps never go stale), extracted from D-17 but binding on WP-64
through WP-73. Several of these packages may partly or wholly dissolve at that
step. Do the upgrade once, at the start of WP-64, and let the later packages
re-assess against the post-upgrade tree rather than each re-running it.

### 8.1 Intra-cluster order

```
WP-64 (workspace tables)          FIRST - everything after is a one-line root edit
  |
  +-- WP-66 (sqlx unification)    either order between themselves
  +-- WP-67 (sentry feature trim)
  +-- WP-68 (term_size -> terminal_size)   already specced
  +-- WP-70 (serde_yaml -> serde_yaml_ng)  independent of all of the above
  +-- WP-71 (warp -> axum in lib/cmd)      gated on WP-06, see 8.3
  |
WP-69 (deny.toml hardening + WP-72 combine note)   LAST
```

- **WP-64 first (D-19).** All three tables - `[workspace.dependencies]`,
  `[workspace.package]`, `[workspace.lints]` - in one pass. Marginal cost
  inside one sweep is near zero and workspace lints help every later package.
  After it lands, every version change in WP-66/67/70 is a single root edit.
- **WP-66 and WP-67 have no ordering constraint on each other.** Both land
  after WP-64.
- **WP-69 lands LAST among the dependency packages (D-23).** The
  `multiple-versions = "warn"` -> `"deny"` flip must happen only after WP-66,
  WP-67 and WP-68 have shrunk the duplicate set, so the `skip`/`skip-tree`
  list starts minimal. The one part of WP-69 that is *not* gated is clearing
  the 4 stale advisory ignores - that may be done at any time.
- **WP-72 has no spec of its own.** D-24 reduces it to "record `combine` 4.6 as
  an accepted risk in `deny.toml`", which is a section of
  `specs/WP-69-deny-toml-hardening.md`. Do not look for a `WP-72-*.md`.

### 8.2 WP-70 is a two-crate atomic change

D-21 chose `serde_yaml_ng` (drop-in, maintained). The two direct consumers -
`rust/bot` and `rust/lib/game_client` - **must move together**; migrating only
one leaves the archived `serde_yaml` in the tree via the other. JSON was
explicitly rejected: it would change a file format ops and users may depend on.
No ordering constraint against the rest of the cluster beyond WP-64.

### 8.3 WP-71 is gated on WP-06 - NEW CONSTRAINT

`WP-06 -> WP-71.` Both touch `rust/lib/cmd/src/http.rs`, the HTTP layer of all
28 game binaries.

- WP-06 is already specced (`specs/WP-06-lib-cmd-tools-http.md`, Task 1) and
  fixes the urgent production defect **within warp**: the handler `.unwrap()`
  that panics the connection task on any malformed `game` string (ls F19). It
  extracts a private `route::<G>()`, maps requester errors to
  `Response::SystemError`, deletes the dead `impl Reject`, and adds a 16 MiB
  `content_length_limit` (ls F28). It adds three tests against `route()`.
- WP-06's own Non-Goals section already records the handoff: *"WP-71's spec
  must re-apply the F19/F28 semantics (SystemError mapping + body cap) when it
  ports."* WP-06 also states F19 must not wait on D-22.
- **Therefore: land WP-06 first, then WP-71 ports the fixed surface to axum.**
  D-22's "in the same window as WP-06's `http.rs` fixes, so the surface is
  touched once" is satisfied by sequencing them adjacently, not by merging
  them. Doing WP-71 first would mean writing the F19 fix twice.
- WP-71 must carry forward, provably: the `SystemError` mapping, the body-size
  cap, and axum ports of WP-06 Task 1's three `route()` tests.
- WP-71 does **not** conflict with WP-68, which touches `lib/cmd/src/repl.rs`.

### 8.4 Cross-cluster

- **WP-73 (game-bins consolidation)** sequences after WP-64 - `work-packages.md`
  already records this. Not otherwise part of this cluster's chain.
  D-41/D-42/D-43 (2026-07-26) changed its scope but **not** its ordering.
- **WITHDRAWN: file overlap, WP-73 x WP-63.** This was flagged post-D-41, when
  WP-73 was to delete `fuzz_gamer` from `rust/tools/fuzz/src/lib.rs` (the file
  WP-63, `bo F26`-`bo F31`, rewrites `fuzz()` in). **D-43 reversed that**: the
  27 per-game `_fuzz` bins and `fuzz_gamer` both survive, and WP-73 no longer
  touches that file at all. **No overlap, no ordering requirement.** Reverse
  consequence: WP-63's `bo F29` reasoning that "all ~30 game `*_fuzz` bins go
  through `fuzz_gamer`" **stays TRUE** and is not made stale - the bins remain,
  and `fuzz_gamer` gains one more caller (`brdgme_game_bin::fuzz_main`).
- **WP-65 (workspace hygiene)** is best after WP-64. Its `dp F9` row in
  `checklists/T3-B8-workspace-hygiene-red7-docs.md` is a version pin that
  WP-64's section 3d assigns explicitly - read that before touching the row so
  it is not done twice. It carries an unresolved tension (pin *back* to dedupe
  vs stay on latest) that WP-64 flags for escalation to Michael rather than
  silently resolving.

### 8.5 UPDATE 2026-07-26: WP-06 Task 1 has already landed in the live tree

Verified live by the dependency-cluster Worker 3 while writing WP-71:
`rust/lib/cmd/src/http.rs` already contains the private `route::<G>()`, the
16 MiB `MAX_CONTENT_LENGTH` / `content_length_limit`, the
`unwrap_or_else(... Response::SystemError)` mapping, no `impl Reject for
RequestError`, and WP-06 Task 1's three tests
(`malformed_game_json_returns_system_error_not_panic`,
`valid_request_still_served`, `oversized_content_length_is_rejected`).
`rust/lib/cmd/src/lib.rs` gates `pub mod http` on the `http-server` feature and
declares `#[cfg(test)] mod test_game`.

**So the 8.3 `WP-06 -> WP-71` gate is already satisfied for Task 1** and WP-71
is unblocked today. The gate stays recorded because WP-06's Tasks 2-5 are
separate and because the tree is under concurrent edit - `specs/WP-71-warp-to-axum.md`
section 3 tells the implementer to re-read `http.rs` and STOP if it no longer
matches, rather than trusting this note.

### 8.6 If WP-66 vendors, the member count changes - WP-64 assertions go stale

WP-66's vendor branch creates `rust/lib/session_store/` as a **41st workspace
member**. Consequences, recorded so nobody trips over them:

- `WP-64 -> WP-66` becomes a **hard** ordering, not merely a convenience. The
  new member must inherit `[workspace.package]` and `[lints]`, which only
  exist once WP-64 has landed.
- **`specs/WP-64-workspace-tables.md` asserts "40 members" in its section 5
  regression checks** (`cargo metadata` shows all 40 members at `0.1.0` /
  `2024` / publish-disabled, `authors` non-empty for all 40). Those assertions
  are correct at WP-64 time and **become stale the moment WP-66 vendors**.
  Whoever lands WP-66's vendor branch must update the count to 41 and add the
  new member to the `members` array - WP-66's rider 5 already requires the
  array edit; this note extends it to the WP-64 assertion.
- Re-check the count before asserting it. If WP-66 took Branch A (no
  vendoring), it stays 40.

## 9. WP-81 vs WP-19 - `stats.rs` deletion collides with `c F11`

Added by the Batch 6 spec Lead, 2026-07-26. **Verified against live source and
against `specs/WP-19-acquire-fixes.md`.**

`WP-19` Task 5 fixes `c F11` ("`Trades` stat reports merges") with a one-token
edit **inside `rust/game/acquire-1/src/stats.rs`'s `to_brdgme_stats`**, and its
task list also adds "the crate's first `stats.rs` test".

`WP-81` (D-40 option B) **deletes `rust/game/acquire-1/src/stats.rs` entirely** -
`to_brdgme_stats` has zero callers workspace-wide (confirmed: `grep -rn
to_brdgme_stats rust/` returns exactly one hit, the definition itself) and
`Gamer::status` returns `stats: vec![]`.

**Resolution:**

- **Land WP-81 first and drop `c F11` / Task 5 from WP-19.** WP-81 makes it moot -
  there is no point fixing a field in a file that is about to be deleted, and
  the "first `stats.rs` test" would be deleted along with it.
- If WP-19 lands first anyway, its one-token fix and its new test are simply
  removed by WP-81. **Whichever package lands second must NOT resurrect
  `stats.rs`.**
- Coverage bookkeeping: `c F11` stays in WP-19's scope list as *superseded by
  WP-81*, not reassigned - the one-package-per-finding invariant is unaffected.

---

## 10. The SSE pivot re-orders the realtime cluster - added 2026-07-26

Source: **D-44** in `decisions-session3.md` (Michael has COMMITTED to SSE and
ruled that it migrates NOW, ahead of WebSocket hardening), plus D-45, D-46,
D-47. Companion docs: `sse-topology-decision.md` (the D-46 topology ruling) and
`specs/WP-84-sse-migration.md` (the migration spec, written and accepted).

### 10.1 The order

```
WP-82 (db.rs module split)
  |
WP-47 (game_visibility gates - creates is_game_visible_to_viewer)
  |
WP-42 (PREDICATE WORK ONLY - is_proposal_visible_to_user + the TTL cache)
  |
WP-84 (SSE migration - /events and /events/public, then delete /ws)
```

### 10.2 Why - do not build machinery that SSE deletes

The 101 upgrade hijacks the connection, which is the **only** reason WP-42's
original §3a hand-rolled a pre-upgrade auth path. On SSE the route is an
ordinary `GET`, so identity resolves through plain extractors. **Hardening the
WebSocket first means writing that path twice and deleting one.** D-44, verbatim
motivation: *"I'd like to consider SSE now purely to avoid wasting effort in the
immediate term."*

### 10.3 What this does to WP-42

`specs/WP-42-websocket-auth-and-filtering.md` has been amended in place (the
filename is kept so cross-references resolve; the package is no longer a
WebSocket auth package).

- **SUPERSEDED - do not build:** the `ws_handler` pre-upgrade auth dance.
  Replaced by `specs/WP-84-sse-migration.md` §3c.
- **SURVIVES - this is now all of WP-42:** `is_proposal_visible_to_user` in
  `db.rs`, consumption of WP-47's `is_game_visible_to_viewer`, the bounded
  per-connection TTL cache (~256 entries, 30s, fail-closed on `sqlx` error),
  and the accepted <=30s staleness. All transport-independent; WP-84 §3d
  consumes them by reference.
- **ELIMINATED - never build:** Task B (`sub`/`unsub`). There is no
  client->server channel under SSE; scope lives in the URL.
- **WP-42 makes no edit to `rust/web/src/websocket.rs`** - the filter cannot be
  wired there without the superseded auth dance, so `/ws` stays unfiltered until
  WP-84's client switch. Accepted; see WP-42 §3e.

### 10.4 WP-84's blocker is DISCHARGED - two streams

The HTTP/2 question is **answered by measurement, 2026-07-26**:
`curl -sI https://brdg.me | head -1` -> `HTTP/2 200`. The browser leg is HTTP/2
through the Cloudflare edge, so the ~6-connections-per-origin cap does not bite.

**D-48 rules TWO STREAMS:** `GET /events` (private, identity-scoped, opened once,
never swapped) plus `GET /events/public?topic=game:<uuid>` (unauthenticated,
swapped on navigation; repeatable `topic` param per D-50). **WP-84's
single-stream fallback section is deleted** - the spec is now one settled design
with no conditionality, and its sections renumber accordingly (regression tests
are §8, riders §9). Hard cap carried forward: **never three held streams** -
future SSE uses ride the private `/events` stream (D-49).

WP-84 is unblocked. The ordering above is unchanged.
