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

WP-42 / WP-47 are a separate cluster: prefer **WP-47 first** so WP-42's socket
filter can call its `is_game_visible_to_user` rather than forking the predicate.

---

## 5. Line-number caveat

Every line number in this file is an **approximate navigational hint, verified at
2026-07-25**. Locate code by file plus function name. If what you find does not
match the description here, **stop and report** - do not adapt the edit.
