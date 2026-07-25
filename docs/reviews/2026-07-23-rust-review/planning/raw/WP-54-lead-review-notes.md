# WP-54 frontend UX error handling - adversarial review of the unreviewed draft

Reviewed by a Worker on behalf of the Lead (the draft was left unreviewed and unlogged
by a Lead that died mid-session). Live repo `/home/beefsack/Development/brdgme`,
snapshot `/home/beefsack/Development/brdgme-review-snapshot/rust` (commit `f8763a5`).
Date: 2026-07-25. Spec repaired in place: 1923 -> 2051 lines.

**Verdict: ACCEPT-AFTER-MY-REPAIRS.**

The draft's architecture, task decomposition, non-goals fencing and honesty about test
coverage were genuinely good — better than the peer drafts reviewed this unit on
structure. Its *citations* were not: **89 load-bearing citations checked, 41 wrong**
(46%), including two build-breaking delete ranges, one non-compiling API call, and one
fix that does not fix its finding. Four placeholder hedges. One invalid finding
recommendation carried through into a task and into a manual-test expected outcome.

Nothing under `rust/` was created, modified or deleted. No cargo/build/test command was
run — every claim below was verified by reading source (repo, `~/.cargo/registry`,
`docs/`, peer specs). No git mutation was run.

---

## Blocking defects found and fixed

1. **Task 11's delete range ate `window_key`'s body.** Draft: "Replace `format_log_time`
   and its doc comment (currently `components/game.rs:307-325`)". Live: `window_key` is
   `:305-308` (comment :305, `fn` :306, body `dt.assume_utc().unix_timestamp() / 600` on
   **:307**, `}` on :308); :309 blank; `format_log_time`'s comment `:310-311`; the fn
   `:312-325`. Deleting :307-325 removes `window_key`'s only statement and its closing
   brace while the replacement supplies neither. **Would not compile.** Corrected to
   **:310-325**, with an explicit "do NOT start at :307" warning.

2. **Task 6's first delete range ate `provide_context(current_user)`.** Draft: "Replace
   the profile-theme block (currently `app.rs:141-172`)". Live: `current_user`'s
   `LocalResource` is `:138-142`, `provide_context(current_user);` is **:143**, :144 is
   blank, the profile-theme comment starts **:145**, and the `Effect` closes on **:173**.
   :141-172 deletes three lines of the resource + the `provide_context` and orphans the
   `});` on :173. **Would not compile, and would break `SidebarMenu`'s `expect_context`
   at runtime even if it did.** Corrected to **:145-173**. Second range `:174-193`
   corrected to **:175-193** (:174 is the separating blank line).

3. **Task 11's `web_sys::Window::navigator()` does not compile in this crate.**
   `Window::navigator` is gated on the `Navigator` web-sys feature. Verified it is not
   enabled anywhere in the `--features ssr` graph:
   - `rust/web/Cargo.toml:77` lists `["Location","Window","Document","HtmlDocument","Element","MediaQueryList","VisibilityState"]` — no `Navigator`.
   - `tachys-0.2.18/Cargo.toml:193-312` adds ~110 features including `HtmlSelectElement` (:304), `HtmlOptionElement` (:311), `MouseEvent` (:230) — no `Navigator`.
   - The only registry crates that enable it are `leptos-use` (features `use_window` :419-424 and `use_web_lock` :396-403 — the crate enables only `use_websocket`/`use_event_listener`/`use_document`, `rust/web/Cargo.toml:80`, and none of those pull `use_window`) and `whoami`, whose `web-sys` dep is `optional` **and** `[target.'cfg(all(target_arch = "wasm32", ...))'.dependencies]` (`whoami-1.6.1/Cargo.toml:59-66`), so it contributes nothing to a native ssr build.
   The draft then *hedged*: "If `web_sys::Navigator::language` is not available … revert
   to `en-US` … Record which branch you took." That is a placeholder hedge whose likely
   branch silently abandons wfe F60. **Closed deterministically** by reading
   `globalThis.navigator.language` through `js_sys::Reflect` off `js_sys::global()`
   (`js-sys-0.3.98/src/lib.rs:13578`), matching `get_turnstile_response`
   (`app.rs:458-468`). Also verified `Date::to_locale_string`'s binding really is
   `(this: &Date, locale: &str, options: &JsValue)` (`js-sys-0.3.98/src/lib.rs:6689`), so
   the draft was right that `undefined` cannot be passed — that part held.

4. **Task 8's fix does not fix wfe F58.** This is the most substantive finding of the
   review. The draft's fix was "mount the `<select>` only once `bot_names` has settled".
   Verified from library source that `prop:value` **never applies on first build at all**:
   - `HtmlElement::build` (`tachys-0.2.18/src/html/element/mod.rs:349-367`) runs
     `let attrs = self.attributes.build(&el);` on **:352** and `self.children.build()` on
     **:357**.
   - A reactive `prop:` goes `tachys-0.2.18/src/reactive_graph/property.rs:36-48` ->
     `html/property.rs:83-88` -> `RenderEffect::new`, whose own doc comment
     (`reactive_graph-0.2.14/src/effect/render_effect.rs:61-62`) is *"Creates a new render
     effect, which immediately runs `fun`"*.
   So `select.value = "medium"` executes against a `<select>` with zero `<option>`
   children — a no-op — and the browser then selects the **first** option as the children
   mount. A select mounted later (which is all the gate achieves) is still built
   attributes-first, so the control is wrong from the first paint regardless.
   **Repair:** Task 8 now does both halves — keep the settled-resource gate (kills the
   option-rewrite half) **and** apply the value from an `Effect` over a
   `NodeRef::<leptos::html::Select>` (Effects run after the render pass, so the options
   exist). Verified `leptos::html::Select` exists (`leptos-0.8.20/src/lib.rs:320` re-exports
   `tachys::html::element`; `elements.rs:380` `select HtmlSelectElement`) and that the
   `HtmlSelectElement` web-sys feature is enabled (`tachys-0.2.18/Cargo.toml:304`), so
   `el.set_value(&…)` compiles. Also factored the value into one `bot_name_value` closure
   used by both `prop:value` and the effect, and documented why both closures are `Copy`.
   Also widened the wfe F58 disposition row and rewrote the manual checklist, which
   previously asserted "**Before the fix:** the select appears instantly showing `medium`"
   — false; it shows the first option.

---

## OVERTURNED justification: wd F57's select re-sync (the draft inherited an invalid finding recommendation)

The wd F57 finding's recommendation ends *"and re-sync the selects from the refetched
overview on failure"*, and the draft built that into Task 2 (bump `set_refresh` on the
failure arm of the two select actions), into the wd F58 disposition rationale ("the
refetch is load-bearing"), into Cross-package #6 ("it happens to work only because the
whole select is re-created from scratch on every refetch") and into manual checklist
step 7 ("go back online, the page refetches and the select snaps back to `open`").

**Independently re-derived: it cannot work.** Three verified reasons:

1. A rejected mutation changes nothing server-side, so the refetch returns identical
   data. `selected` is an eager `bool` (`friends.rs:559`, `:571`), and
   `impl AttributeValue for bool::rebuild`
   (`tachys-0.2.18/src/html/attribute/value.rs:554-563`) compares against the previous
   value and returns without touching the DOM when equal.
2. The enclosing `{move || match overview.get() { … }}` arms all `.into_any()`. Taking the
   same arm again means the same `TypeId`, and `AnyView::rebuild`
   (`tachys-0.2.18/src/view/any_view.rs:386-400`) then rebuilds **in place** rather than
   recreating. `collect_view()`'s `Vec` rebuild likewise reuses the `<option>` elements.
   So the draft's "re-created from scratch on every refetch" premise is false.
3. Even a genuinely changed `selected` content attribute would not move a select the user
   has clicked: `selected` sets `defaultSelected`, and per HTML it only reassigns
   selectedness while the option's dirtiness flag is false; user interaction sets it true.
4. Supporting fact: resources are stale-while-revalidate, so a refetch never even passes
   through `None` to force an arm change —
   `reactive_graph-0.2.14/src/computed/async_derived/arc_async_derived.rs:380-389` only
   writes `value` when the future resolves; nothing clears it. (`app.rs:763-765`'s comment
   says the same about `game_data`.)

**Repairs:** failure-arm `set_refresh` bumps removed from Task 2 (they were a wasted
round trip plus a lying comment); the wd F57 verdict now reads "CONFIRMED; the finding's
re-sync recommendation is OVERTURNED"; the wd F58 rationale was rewritten (see below);
Cross-package #6's mechanism claim was corrected; new assumption **A6** and new
Cross-package **#7** record the residual and route it; manual checklist step 7 now
asserts the residual as the *expected* outcome so nobody bakes it in as a regression or
"fixes" it wrongly.

**Consequence for wd F58 (independently re-derived, narrowed):** with the re-sync
rationale gone, the missing `policy_action` refetch has almost no user-visible effect —
`o.invite_policy` is read at exactly one place (`friends.rs:559`), and a successful
change already displays correctly. Verdict changed to "CONFIRMED as an inconsistency;
impact NARROWED to ~nil", still fixed, for consistency with the five siblings and so the
client's cached overview does not lie. This is the one place where I weakened a draft
verdict; the fix itself is unchanged.

Other verdicts I re-derived independently and which **hold**:
- **wd F73's ADJUSTED Theme arm** (do not revert the theme). Holds, and I strengthened
  it: `set_theme_client` writes `<html data-theme>` at `app.rs:255-264` **and** the
  `max-age=31536000` cookie at `:265-271` (literal on :267), and the dispatch only
  happens when logged in (`settings.rs:496`). Reverting would undo a change that worked.
  I also added the reason the *Colors/EmailPrefs* revert genuinely works while the
  friends-select one cannot (signal-driven `prop:checked`/`prop:value` vs eager
  `selected=`).
- **wd F66's rejection of the finding's second option** (render the union including the
  prefill count). Holds — it would offer a count the game type cannot start. I added the
  missing verification that `gt.player_counts` (not `pf.player_counts`) is the right
  clamp target: `RestartPrefill` does carry `player_counts` (`game/server_fns.rs:151`,
  filled from the latest non-deprecated version at `:1284-1287`) but `new_game.rs` never
  reads it (Cross-package #8).
- **wfe F54's "adopt the finding's second option, and do both id-keying and
  logout-clearing".** Holds. I added the premise the draft never cited: a re-login in one
  tab really does re-resolve `current_user` without a page load —
  `app.rs:511-517` calls `current_user.refetch()` then client-side-navigates.
- **wfe F55's choice of `user_facing_server_error` over `action_error_message`.** Holds
  (failed read, and the only distinguishable case would need string-matching).
- **wfe F61's rejection of `<button>` at all three sites.** Holds; the styling reasons
  check out (`main.scss:17-21`, `:116-118`).
- **Task 1's `_` arm rather than exhaustive matching in `action_error_message`.** Holds:
  `WrappedServerError` is `#[deprecated(since = "0.8.0")]`
  (`server_fn-0.8.13/src/error.rs:171-178`).
- **The `admin.rs` LEAD RULING.** Holds and is now *better* evidenced — WP-37 has since
  recorded the corrected routing itself (`WP-37-admin-pass.md:2349-2350`, `:45`, `:89`).

No verdict was "SKIPPED-BY-DECISION", so there is nothing to relabel. The draft claimed
"0 overturned premises" — true, all 17 premises are real. But it claimed only 2 changed
recommendations; the real count is 4 (wd F57 added, wfe F58 both options rejected rather
than one adopted). Corrected in the counts paragraph.

---

## Placeholder hedges found and closed (4)

| Hedge | Where | How closed |
|---|---|---|
| "if the borrow does not typecheck, use `&e`. Let the compiler decide; do not guess." | Task 8 rider, `opponent_slot.rs:129` | It **is** `&e`. `search_action.value().get()` returns the `Option` **by value** — proof: the sibling arm at `:116` returns its bound `results` as the fn's owned `Vec<UserSearchResult>` (`search_results` is `move || -> Vec<UserSearchResult>`, `:110`). So `e` is an owned `ServerFnError`. Also noted why `*tag == current` still compiles (`impl PartialEq<String> for str`). |
| "Verify the borrow compiles; if `gt` has already been moved … move the `let gt_counts = …;` line up" | Task 9 prefill effect | No conditional needed. `gt` is the component parameter (`new_game.rs:233`), moved into `StoredValue` at **:394** (draft said :390) — 124 lines after the effect at :270, and only read by reference in between (`:243`, `:244`). Compiles as written. |
| "If `web_sys::Navigator::language` is not available … take A4's alternative … Record which branch you took" | Task 11 | Determined it is definitively unavailable (feature analysis above) and rewrote the code to use `js_sys::Reflect`. Replaced the hedge with a STOP-and-report gate. |
| "**If the second assertion fails** … delete only the second assertion" | Task 5 SSR test | Replaced with a three-step reproduce-then-fix protocol: add an inverted assertion first and prove wfe F55 is reproducible; only then make the change and add both real assertions; if the reproduction fails, **STOP and report** — because that would falsify the wfe F55 disposition rationale, which is a Lead call, not an implementer's licence to delete coverage. |

---

## Full citation-verification table

Legend: OK = correct as written; WRONG = corrected in the spec.
Only load-bearing citations are listed (line anchors used by a step, disposition-table
anchors, counts, negative claims, library facts, fixture references).

### `rust/web/src/components/game.rs` (681 lines, the one drifted file)

| Draft claim | Live reality | |
|---|---|---|
| `GameMeta` :26-217 | `#[component]` :25, fn :26, ends :217 | OK |
| five `ServerAction`s :49-53 | :49-53 | OK |
| undo effect :58-63 | :58-63 | OK |
| concede effect :64-69 | :64-69 | OK |
| end_game effect :70-75 | :70-75 | OK |
| force_delete effect :80-86 | **:80-85** (:86 blank) | WRONG |
| effect block to replace :55-86 | **:55-85** | WRONG |
| `bump_bot_action` dispatched :173-175 | **:176-179** | WRONG |
| `<h3>"Actions"</h3>` :110 | :110 | OK |
| concede confirm block :125-128 / step ":124-131" | **:126-128** / step **:126-131** | WRONG |
| end-game confirm :138-141 / step ":137-144" | **:139-141** / step **:139-144** | WRONG |
| force-delete confirm :190-193 / step ":189-196" | **:191-193** / step **:191-196** | WRONG |
| `dyn_ref::<web_sys::HtmlElement>` at :519 | **:525** | WRONG |
| `use web_sys::wasm_bindgen::JsCast;` :9 | :9 | OK |
| `PlayerInfo` :219-301 | fn :220, ends **:303** | WRONG (minor) |
| `PlayerInfo` add_friend match ":284-287" / drift-table ":281-297" | `ServerAction` :281, match **:284-296**, `Err` arm **:286** | WRONG |
| `class="error"` at `game.rs:286` | :286 | OK |
| `window_key`/`format_log_time`/`render_log_entries` :303-366 | **:305-366** | WRONG |
| `window_key` :304-306 | **:305-308** (comment :305, fn :306-308) | WRONG |
| `format_log_time` :307-325 | **:310-325** (comment :310-311, fn :312-325) | **WRONG - build-breaking** |
| locale arg :324, `hour12` :323 | :324, :323 | OK |
| `render_log_entries` calls `format_log_time` at :340 | **:346** | WRONG |
| `GameLogs` calls `render_log_entries` at :406 | **:409** | WRONG |
| `RecentGameLogs` at :446 | :446 | OK |
| `mounted` gates :386-387 and :425-426 | :386-387, :425-426 | OK |
| mounted-gate comment :379-385 | :379-385 | OK |
| `"Failed to load logs."` :405 | **:406** | WRONG |
| `GameCommandInput` :492-681 | :492-681 | OK |
| `error_msg` :583-592, rendered :656-659 | **:583-591**, rendered **:657-659** | WRONG |
| "never leak the raw ServerFnError text" :589 | **:588** | WRONG |
| keydown listener :512-548 | **:512-542** | WRONG |
| Space-guard comment :509-511 | :509-511 | OK |
| suggestion links :641 | **:624** | WRONG |
| Task 10 idiom refs: Undo :114-117, Concede :122-133, End game :136-146, Bump bot :170-176, Delete :187-197 | **:116-119**, **:124-132**, **:137-145**, **:176-179**, **:189-197** | WRONG (all five) |
| `CommandInputText` newtype :14-16 | struct :15-16, doc :12-14 | WRONG (minor) |
| `SubMenuOpen` read via `crate::components::layout::` at "game.rs:88" | **:90** | WRONG |

### `rust/web/src/app.rs` (924 lines)

| Draft claim | Live reality | |
|---|---|---|
| `App` :105-238 | :105-**237** | WRONG (minor) |
| existing hoists :126-138 | **:126-143** (`logout_action` :126-127, `active_games` :129-134, `current_user` :138-143) | WRONG |
| `active_games` fetcher :128-132 | **:129-133** | WRONG |
| `current_user` refetch-on-logout :133-138 | **:138-142**, `provide_context` :143 | WRONG |
| `logout_action` created :124-125 | **:126**, provided :127 | WRONG |
| latches at :150-172 and :174-193 | **:145-173** and **:175-193** | WRONG |
| `applied_profile_theme` :150, guard :151-153, set :154 | decl **:154**, guard **:156-157**, set **:158** | WRONG |
| `presence_started` :174, guard :176, set :178 | decl **:179**, guard **:181**, set **:182** | WRONG |
| ping-loop break :180-183 / :181-183 | **:185-187** | WRONG |
| Task 6 replace range :141-172 | **:145-173** | **WRONG - build-breaking** |
| Task 6 replace range :174-193 | **:175-193** | WRONG |
| Task 7 insert "after `provide_context(current_user);` (:139)" | **:143** | WRONG |
| `use uuid::Uuid;` present | :9 | OK |
| `set_theme_client` :246-283 | **:246-272** (`local_data_theme` is :277-282) | WRONG |
| cookie `max-age=31536000` | :267 | OK (line not previously given) |
| `get_turnstile_response` :458-468 | :458-468 | OK |
| `LoginPage` :471-651 | fn :471, ends **:653** | WRONG (minor) |
| `show_code_link` :551-553 | **:549-551** | WRONG |
| `<form on:submit=on_email_submit>` :576 | **:574** | WRONG |
| Turnstile block :589-598 (div :595) | :589-598, div :595 | OK |
| anchor "I already have a login code" :603 | :603 | OK |
| anchor "Logging in as" :623 | :623 | OK |
| `class="error"` :607, :610, :646, :762 | all four | OK |
| `js_string_escape` :53-57 | fn **:55-57** (doc :52-54) | WRONG (minor) |
| `game_data = Resource::new_blocking` :701-710 | **:700-708** | WRONG |
| `"Invalid Game ID"` :707 | **:705** (also :722 in the logs resource) | WRONG |
| `app.rs:762` Err branch | :762 | OK |
| `StoredValue` used at :775 | **:777** | WRONG |
| `GamePage` :653-… | **:655-…** (`#[component]` :655, fn :656) | WRONG (minor) |
| (added by review) login refetches `current_user`, no reload | :511-517 | verified |

### `rust/web/src/friends.rs` (581 lines)

| Draft claim | Live reality | |
|---|---|---|
| server-fn region :84-350 | :84-**351** | WRONG (minor) |
| `FriendsPage` :353-581 | :353-581 | OK |
| six `ServerAction`s :363-368 | :363-368 | OK |
| effect block :371-398 / ":373-398" | comment **:372**, effects **:373-398** | WRONG (step range) |
| per-action effect ranges :373-378, :379-383, :384-388, :389-393, :394-398 | all five exact | OK |
| `<h1>"Friends"</h1>` :404 | **:403** | WRONG |
| page-load error :406 | :406 | OK |
| `add_action` inline error :426-428 | :426-428 | OK |
| `set_add_name.set(String::new())` in add effect | :375 | OK |
| invite-policy select ":556-561", per-option `selected=` ":558" | select **:555-562**, bool **:559** | WRONG |
| visibility select ":568-573", `selected=` ":570" | select **:567-574**, bool **:571** | WRONG |
| `"No user named bob"` :165 | **:167** | WRONG |
| `"You cannot friend yourself"` :169 | **:171** | WRONG |
| `"Request not found"` :202, :214 | **:205**, :214 | WRONG (one of two) |
| `get_friends_overview` :99 | **:95** (fn :95-133) | WRONG |
| `get_incoming_friend_request_count` :135-144 | :135-144 | OK |
| `block_user` body ":226-238" | `#[server]` :229, body **:230-240** | WRONG |
| `href="#"` precedents :447-465, :494-500, :543-546 | :447, :452, :457, **:489**, :543 | WRONG (one) |
| "`grep to_string()` remaining hits are in `#[server]` bodies above :350" | there are **none**; the only two hits are :406 and :427, both removed by Task 2 | WRONG |

### `rust/web/src/settings.rs` (572 lines)

| Draft claim | Live reality | |
|---|---|---|
| `UsernameSection` :47-105, slot pattern :59-69, `FormField` error slot :85 | fn **:49-107**, pattern **:56-69**, slot **:84** | WRONG |
| `ColorsSection` :110-172 | **:112-177** | WRONG |
| `save_action` :132 | **:135** | WRONG |
| `colors.update(...)` :135-140 | **:137-144** | WRONG |
| dispatch :141-143 | **:145-147** | WRONG |
| `pick` closes at :144 (step range :132-144) | closes **:148** (range **:135-148**) | WRONG |
| `EmailPreferencesSection` :174-232 | **:179-240** | WRONG |
| three actions :196-198 | **:200-202** | WRONG |
| dispatches :205-208/:207-208, :217-220/:219-220, :229-232/:231-232 | **:213**, **:224**, **:235** (`set` at :212, :223, :234) | WRONG |
| `initialized` latches :126-134, :184-192 | **:126-133**, **:189-198** | WRONG |
| `EmailSection` :234-455 | **:242-465** | WRONG |
| `EmailSection` four effects :276-333 | **:276-336** | WRONG (minor) |
| `error.set(Some(e.to_string()))` at :285, :301, :316, :331 | all four exact | OK |
| `ThemeSection` :457-572 | **:471-572** | WRONG |
| `set_theme_action` :461 | **:475** | WRONG |
| `select()` :487-498 | **:488-499** (`set_theme_client` :494, dispatch :497) | WRONG |
| `<h2>` anchors "Preferred colours"/"Email notifications"/"Theme" unique | :151 / :205 / :532, each unique | OK |
| `grep -c action_error_message` must be "exactly 10: 1+3+4+2 in nothing else" | **8** (1+3+4); the draft's arithmetic is incoherent | WRONG |
| `grep "e.to_string()"` returns nothing afterwards | correct — no other `e.`-prefixed `to_string()` in the file | OK |
| module doc "email placeholder" at :1-2 | :1-3 (three `//!` lines) | OK enough |

### `rust/web/src/components/layout.rs` (316 lines)

| Draft claim | Live reality | |
|---|---|---|
| `SubMenuOpen` :12-16 | :12-16 | OK |
| `next_game_id` + tests :281-315 | `mod tests` **:281-316**, the two tests **:301-309** and **:311-315** | WRONG |
| unit tests ":288-315" | **:301-315** | WRONG |
| `MainLayout` :28-114 | :28-114 | OK |
| `SidebarMenu` :116-279 | :116-279 | OK |
| logout effect :120-124 | :120-124 | OK |
| step range ":117-124" | **:118-124** (:117 is the fn signature) | WRONG |
| remount comment :126-129 | :126-129 | OK |
| `friend_request_count` :135-136 | :135-136 | OK |
| logged-in block :151-176 | **:154-175** (inner closure :155-174, inner `view!` :163-173) | WRONG |
| logout anchor :165-171 | **:166-171** (`style="cursor:pointer"` on :170) | WRONG |
| badge render :183-192 | **:184-192** | WRONG (minor) |
| route-change reset effect :141-144 | :141-144 | OK |
| `class="error"` :197 | :197 | OK |

### `rust/web/src/components/opponent_slot.rs` (352 lines) and `components/mod.rs` (15)

| Draft claim | Live reality | |
|---|---|---|
| `OpponentSlotEditor` :53-352 | `#[component]` :53, fn :54, ends :352 | OK |
| `OpponentSlot::default()` = `Player` :40-47 | :40-47 | OK |
| `set_mode` :69-87, `bot_name: "medium"` :75 | :69-87, :75 | OK |
| `bot_default_name` StoredValue :64 | :64 | OK |
| `search_error` message :129 | :129 | OK |
| `let (search_seq, …)` :134 | :134 | OK |
| Bot branch :313-349 | :313-349 | OK |
| `prop:value` :317-320 | :317-320 | OK |
| `on:change` :321-326 | :321-326 | OK |
| options closure :328-346, fallback :331-335 | :328-346, :331-335 | OK |
| `href="#"` precedents :244-253, :273-284 | both | OK |
| "the option list is *replaced*, resetting selectedness" | mechanism is **in-place reuse of the `<option>` elements with rewritten `value` attributes** (`collect_view` Vec rebuild); replacement only when the new list is shorter | WRONG (mechanism) |
| `components/mod.rs:1-2` stale comment; :11 `pub use confirm::*;`; 15 lines; six mods, five globs | all exact | OK |
| `components/confirm.rs` is 5 lines, `confirm(&str) -> bool` | exact | OK |

### `rust/web/src/new_game.rs` (660 lines)

| Draft claim | Live reality | |
|---|---|---|
| `player_range` :18-35 | **:18-36** | WRONG (minor) |
| `prefill_to_slots` "ends at :56" | ends **:57** | WRONG |
| `GameSetupPanel` :232-548 | :232-**549** | WRONG (minor) |
| `bot_names` :234 | **:235** | WRONG |
| `selected_version_id` starts `None` :243 | :243 | OK |
| `form_error` signal :246 | :246 | OK |
| resize effect :265-268 | :265-268 | OK |
| prefill effect :270-279, guard :271-273, count set :274/:278 | all exact | OK |
| create-success effect ":315-323" | **:316-324** (`navigate_create` on :315) | WRONG |
| restart effect `Created(po)` arm :330-337 | **:330-336** | WRONG |
| `AlreadyRestarted { .. } => {}` :348 | :348 | OK |
| `on_submit` guard :355-357 | :355-357 | OK |
| sibling guard sets `form_error` :368-373 | **:367-372** | WRONG (minor) |
| `OpponentSlot::Email(email)` arm :374 | :374 | OK |
| `set_form_error.set(None)` :378 | :378 | OK |
| `let gt = StoredValue::new(gt);` :390 | **:394** | WRONG |
| `"Restarting <name>"` heading :392-397 | **:398-403** (`format!` on :400) | WRONG |
| version-select parse :429 | :429 | OK |
| radios :470-487 | :470-487 (`prop:checked` :478) | OK |
| `form_error` render :530-534 | **:532-536** | WRONG |
| `user_facing_server_error` at :106, :212, :539 | :106, :212, **:541** | WRONG |
| `mod tests` :550-660 | **:551-660** (`use super::*;` :553) | WRONG (minor) |
| "five sibling pure-fn tests" | **six** (:573, :601, :609, :615, :631, :641) | WRONG |
| `"Start game"` at :524 (cross-package #2) | **:525** | WRONG |

### `rust/web/tests/ssr_pages.rs` (1458 lines) and `style/main.scss`

| Draft claim | Live reality | |
|---|---|---|
| `assert_clean_html_body` panic assert :178-181 | :178-181 (fn :168-182) | OK |
| `game_page_anonymous_visitor_gets_clean_error_not_panic` :322-358 | attr :322, fn :323-358, last assert **:357** | OK |
| `game_page_logged_in_player_renders_game` :360-401 | attr :360, fn :361-401 | OK |
| `game_page_player_names_link…` :403 | attr :403, fn :404 | OK |
| `login_page_anonymous` :258-268 | attr :258, fn :259-268 | OK |
| `home_page_logged_in_renders_index_shell` :245-256 | attr :245, fn :246-256 | OK |
| `new_game_type_page_anonymous` :291 | attr :291, fn :292 | OK |
| `games_route_is_unused_returns_not_found` :302-320 | attr :302, fn :303-320 | OK |
| `restart_game_on_finished_game_succeeds` :519 | attr :519, fn :520 | OK |
| `restart_game_with_roster_uses_passed_version` :579 | fn :579 | OK |
| `.command-error` :373-376, `.form-error` :715-717, `.error` has no rule | all exact; `grep -n error main.scss` returns exactly those two | OK |
| `a, a:hover, …` :17-21 with `cursor: pointer` | :17-21, cursor on :20 | OK |
| `.login .hasCode` :116-118 | :116-118 | OK |
| "eight existing `class=\"error\"` sites" / "~15" | **22** across the crate (21 after Task 2) | WRONG |
| `grep cursor:pointer` returns only the three sites | exactly `app.rs:603`, `app.rs:623`, `layout.rs:170` | OK |
| Task 10: "all three anchors are in server-rendered HTML" | only two — `app.rs:623` is inside `<Show when=show_code_input>` (:620), false on SSR | WRONG |

### Library / registry facts

| Draft claim | Live reality | |
|---|---|---|
| `server_fn-0.8.13/src/error.rs`: Display :218-250, `ServerError` arm :233-234, `Request` :230-232, `new` :198-202, variants :165-197, `WrappedServerError` deprecated :171-178, not `#[non_exhaustive]` | all exact | OK |
| `leptos_server-0.8.7/src/local_resource.rs:294 impl<T> Copy for LocalResource<T>` | exact | OK |
| `reactive_graph-0.2.14/src/traits.rs`: `get_value` :766, `update_value` :819, `set_value` :855 | exact | OK |
| `reactive_graph-0.2.14/src/actions/action.rs:269, :288` dead `is_latest` guard | exact (also :321, :340) | OK |
| `tachys-0.2.18/src/html/element/mod.rs:349-361` attributes-before-children | fn is **:349-367**; `attributes.build` **:352**, `children.build` **:357** | OK (range imprecise, fact right) |
| `Cargo.toml:19-21` = leptos/leptos_router/leptos_meta | **:18**, **:19**, **:23** | WRONG |
| `error.rs` is 18 lines; `user_facing_server_error` :15-17 | **16** lines; **:14-16** | WRONG |
| `proposals.rs` confirm sites :2007, :2023, :2051, :2063, :2133 | all five exact | OK |
| `auth::AuthUser { id, name, email }` at `auth/server.rs:118-122` | struct :117-122, fields :119-121 | OK enough |
| `docs/CODING.md:87` LocalResource `None` on SSR | :87 | OK |
| `docs/CODING.md:135-137` Transition renders children on SSR | :135-137 | OK |
| `docs/CODING.md:139-152` / `:146-149` structural vs attribute | :139-148 / :146-148 | OK enough |
| `docs/CODING.md:298-304` save model | **:297-303** | WRONG (minor) |
| `docs/CODING.md:306-311` `<option selected>` | **:305-310** | WRONG (minor) |
| "Effects are inert during SSR — `docs/CODING.md:69-153`" | CODING.md never states it; the source is **`docs/hydration.md:80-104`** (and it names `format_log_time` as its worked example at :75-79) | WRONG |
| `get_restart_prefill` is `game/server_fns.rs:1180-1204` and can return `"Game version not found"` | `get_restart_prefill` is **:1321-1332**, delegating to `get_restart_prefill_impl` **:1257-1319**; errors are `"Not authenticated"` :1329, `"Game not found"` :1265, `"Game is not finished"` :1268, `"You are not a player in this game"` :1275, `"Game type not found"` :1287. `:1180-1204` is `restart_game_with_roster`, and `"Game version not found"` (:1204) is **its** error, not the prefill's | **WRONG (wrong function)** |
| `game/server_fns.rs:251`, `:256`, `:885` | exact | OK |
| `game/server_fns.rs:1191`, `:1198` (as `restart_game_with_roster`) | exact | OK |
| `rust-toolchain.toml` 1.97.0, edition 2024 `Cargo.toml:5` | exact | OK |
| `work-packages.md:423-431` is WP-54's block | **:423-429**; scope list :424, severity :426 | WRONG (minor) |
| `WP-37-admin-pass.md:2251` routes admin.rs here | :2251 is about `ProvidersSection`; the real routing note is **:2349-2350**, restated :45 and :89 | WRONG |
| `WP-59…:2494-2504` routes admin.rs here | :2494-2504 is about `bump_reply`; the real note is **:2754** | WRONG |
| `WP-37-admin-pass.md:2250` records the reactive_graph item | **:2347** | WRONG |
| `admin.rs` sites ":1024, :1041, :1119, :1132, :1144 and siblings" | all real; the full set is 15 (`:1024, 1041, 1119, 1132, 1144, 1156, 1431, 1444, 1456, 1468, 1767, 1780, 1792, 1804, 1827`) | OK, under-enumerated |
| Cross-package #2: e2e spec broken (`/games` 404, no `.form-row`, no "Create Game") | all verified; **also** expects a `"Welcome to brdg.me"` heading that exists nowhere in `rust/web/src` | OK (understated) |
| Snapshot: 7 of 8 files identical, `components/game.rs` 660->681, 35 `^[+-]` lines | verified by `diff -ru`; also `error.rs`, `tests/ssr_pages.rs`, `style/main.scss` identical | OK |
| game.rs drift caused by five commits `0243472/1f665b0/3b7252f/998a081/ecfc17a` | `git log -- rust/web/src/components/game.rs` shows **only `1f665b0`** post-snapshot | WRONG |
| drift includes the `ranked_placing`/`FormStrip` block ":268-278" | that block (`:269-277`) is **byte-identical** in the snapshot — not drift | WRONG |
| WP-41-routed `ssr`-gated predicates: `db.rs:2001/2002, 2909/2910, 2916/2917, 2923/2924, 2938/2939`; `validate_username:849` ungated | re-verified all six against live `db.rs` | OK |

**Tally: 89 load-bearing citations checked, 41 WRONG.** Of those, 2 were build-breaking
delete ranges, 1 was a non-compiling API call, 1 attributed a finding's error set to the
wrong function, 1 was incoherent arithmetic (`grep -c` = 10), and 1 was a broken mechanism
claim; the remaining ~35 were off-by-1-to-14 anchors, several of which would have sent a
weak implementer to the wrong line (e.g. `settings.rs`'s `ThemeSection` insertion point
was 14 lines off, `app.rs`'s latches 4-5 lines off).

Notably, in three places the *findings* were right and the draft's "re-derivation" was
wrong: wfe F54's `app.rs` anchors, wfe F61's `layout.rs:166`, and wd F73's `settings.rs`
anchors. The spec now says so explicitly so a future reader does not "correct" them back.

---

## Newly discovered defects and routing (all recorded in the spec's Cross-package section)

- **#7 (new): rejected select changes stay visually desynced on the friends page.** No
  package owns `FriendsPage`'s markup except WP-54. Recorded with the full mechanism and
  the correct fix shape (`prop:value` over an `Effect`-seeded `RwSignal<String>`, which
  also closes #6). **Needs a Lead decision:** absorb into Task 2, or file separately.
  Deliberately not silently fixed, and Task 2's checklist states the residual as expected.
- **#8 (new): `RestartPrefill::player_counts` is computed, serialised and never read by
  the client.** Routed to **WP-53** (`game/server_fns.rs`). Not a correctness bug today.
- **#9 (new): nothing guarantees the hard-coded default `bot_name: "medium"` exists in
  `get_available_bots`' list.** If it ever does not, Task 8's `set_value` selects nothing
  and the desync returns, unfixable client-side. Seeded data currently makes it moot
  (`migrations/013_bot_efficacy.sql:41-45`). Routed to **WP-53**, or **no owner - Lead to
  file**.
- **#10: the WP-41-routed `ssr`-gated pure predicates note was MISSING from the draft
  entirely.** Added, with all six `db.rs` anchors re-verified against live source and the
  WP-41 routing sentence (`WP-41-db-quality-pass.md:1988`) quoted. Action for the
  implementer: none.
- **wfe F58's widened mechanism** is not a new defect so much as a bigger version of a
  known one; it is handled inside Task 8 rather than routed, since wfe F58 is in scope.
- **`components/game.rs:286`** (`PlayerInfo` add-friend `e.to_string()`) remains routed
  as a one-line follow-up (Cross-package #3), unchanged from the draft's decision, which
  I agree with.

## Needing a user decision

**None new.** #7 needs a *Lead* scope decision, not a user one. If the Lead prefers to
record it formally, proposed decision text:

> **D-NN. Friends-page select revert.** A rejected invite-policy or game-visibility
> change on `/friends` currently keeps displaying the unsaved value until the page
> reloads. Fixing it means converting both `<select>`s from per-`<option>` `selected=` to
> a `prop:value`-over-signal binding driven by an `Effect` (which `docs/CODING.md:305-310`
> already prescribes). Options: **(A)** absorb into WP-54 Task 2 — bigger diff in markup
> WP-54 otherwise only reads, but the user-visible behaviour is then correct;
> **(B)** ship WP-54 with the error message only and file the conversion against the
> `friends.rs` owner (WP-53 already touches the file). Recommendation: **B**, because the
> error message alone removes the silent failure, and the conversion belongs with the
> other `friends.rs` component work.

## D-15 / D-16 gates

- **D-15 (reopened; the email verb `end` collides with acquire-1 / starship-catan-1 top-level
  moves and with `rust/web/src/email/commands.rs:1217`'s own `end` arm — verified, that
  line is `"end" => return run_end(ctx).await,`)** gates **WP-59 Task 14 only**. WP-54
  adds no email verb, no dispatcher arm and no email copy. The only `end`-adjacent things
  it touches are the UI label `"End game failed: "` and the existing `EndGame` action
  dispatch. **Not gated.** An explicit Non-Goals bullet now says so.
- **D-16** gates WP-55; the draft's WP-55 fencing was already correct and I only corrected
  the `show_code_link` anchor inside it.

## Snapshot-vs-live drift

`diff -ru` per file: `friends.rs`, `new_game.rs`, `settings.rs`, `app.rs`,
`components/layout.rs`, `components/opponent_slot.rs`, `components/mod.rs`, `error.rs`,
`tests/ssr_pages.rs`, `style/main.scss` — **all identical**. `components/game.rs`
**drifted**, 660 -> 681 lines, from **one** commit (`1f665b0`), not five as the draft
said. The draft's hunk inventory was otherwise right except for including the
`Place:`/`Form:`/`FormStrip` block, which is identical in the snapshot. Two additional
real hunks the draft omitted: the concede `<Show>` condition (`is_2player` -> `can_concede`,
:122) and `profile_link=player.user_id.is_some()` (:253). All corrected in the spec.

## Caveat for the Lead

My repairs shift WP-54's own line numbers, so three cross-references from peer specs now
point at the wrong lines: `WP-37-admin-pass.md:45` and `:2349` cite
`WP-54-frontend-ux-error-handling.md:210` for the admin.rs LEAD RULING (now ~:229), and
`WP-37:2347` cites `WP-54:1886` for the reactive_graph item and `:1885` for admin.rs (now
~:2001 / ~:1999). The quoted *text* is still present and still says the same thing, so
nothing is substantively stale. Separately, `WP-37:45` cites
`components/game.rs:550-562` for `GameCommandInput`'s error slot; live it is
**:583-591** — a WP-37 anchor error I did not touch, flagged here for the Lead.

## Compliance

- **No file under `rust/` was created, modified or deleted.** Every `rust/` access was a
  read (`sed -n`, `awk`, `grep`, `wc`, `diff`, `git log`).
- **No cargo/build/test/clippy/fmt command was run.** All library facts come from reading
  `~/.cargo/registry/src/index.crates.io-*/` sources and `Cargo.toml`/`Cargo.lock`.
- **No git mutation was run** (`git log`, `git diff` read-only only).
- Files written: this notes file, and in-place edits to
  `planning/specs/WP-54-frontend-ux-error-handling.md`. Nothing appended to
  `specs-LOG.md` or `decisions-needed.md`.
