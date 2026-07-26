# WP-54: frontend UX error handling

> **CITATION WARNING - line numbers in this spec are approximate and unverified.**
> Corpus-wide they measured **33-46% wrong**, and two "delete lines A-B" ranges
> would have destroyed live code. **Navigate by the named function, type or
> symbol** - never by line number alone. If the code at a cited location does not
> match this spec's description, **STOP and report**; do not improvise a fix or
> guess at the intended target.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to execute this plan task-by-task with review checkpoints.

> **Adversarial review pass, 2026-07-25.** This draft was written without a Lead review. It has since been re-verified line by line against live source and repaired in place. Notes and the full citation table: `planning/raw/WP-54-lead-review-notes.md`. Four defects that would have broken the build or the fix were corrected — do **not** revert them:
>
> 1. **Task 11's edit range** was `components/game.rs:307-325`, which deletes `window_key`'s body. It is **:310-325**.
> 2. **Task 6's first edit range** was `app.rs:141-172`, which deletes part of the `current_user` resource and its `provide_context`, and orphans a `});`. It is **:145-173**. (Its second was `:174-193`; it is **:175-193**.)
> 3. **Task 11 called `w.navigator()`**, which does not compile here — the `Navigator` web-sys feature is not enabled in this crate's `ssr` graph. It now reads the locale through `js_sys::Reflect`.
> 4. **Task 8's fix was incomplete.** Gating the `<select>` on the settled resource fixes only half of wfe F58; `prop:value` never applies on first build at all, so the control was wrong from the first paint. Task 8 now also applies the value from an `Effect` over a `NodeRef`.
>
> Also: **wd F57's "re-sync the selects from the refetched overview on failure" is OVERTURNED** — it cannot work, and Task 2 no longer pretends to do it (see Task 2's preamble, assumption A6 and Cross-package #7). Four placeholder hedges were closed deterministically. All disposition-table and step line anchors were corrected; roughly a third of the original draft's citations were wrong.

**Goal:** Make every fire-and-forget `ServerAction`/`Action` in the Leptos frontend *observe its own result* and surface a user-visible error, using the error-slot pattern the repo already established in `GameCommandInput` and `UsernameSection` — starting with `GameMeta`'s three destructive actions (concede, **end game** (added by #47), admin force-delete) plus undo and bump-bot (wfe F52, major); then the five unobserved friends-page mutations plus the missing invite-policy refetch (wd F57, wd F58), the three optimistic settings sections (wd F73), and the silent logout failure that affects every page (wfe F59). Stop `ServerFnError`'s `Display` prefix reaching users (wfe F55 and every new slot). Reset the two SPA one-shot latches in `App` so presence-ping and profile-theme sync work after logout→login in one tab (wfe F54). Hoist `friend_request_count` out of the per-navigation remount (wfe F57). Stop the bot-difficulty `<select>` desyncing when `bot_names` resolves late (wfe F58). Fix the restart-prefill silent-`Err`, the silent no-version submit, the navigate-nowhere success, and the out-of-range prefill player count in `new_game.rs` (wd F59, wd F63, wd F64, wd F66). Route the three click-only anchors through the codebase's `href="#"` + `prevent_default` pattern so they are keyboard-reachable (wfe F61). Replace `GameMeta`'s two inline confirm dialogs with `crate::components::confirm` (wfe F56). Follow the browser locale in log timestamps (wfe F60) and delete the stale `components/mod.rs` placeholder comment (wfe F62).

---

## Architecture — the code you are changing, and the pattern you must copy

`rust/web` is a single Axum + Leptos 0.8 monolith (`rust/web/Cargo.toml:18` `leptos 0.8.20`, `:19` `leptos_router 0.8.14`, `:23` `leptos_meta 0.8.6`; resolved sub-crates `leptos_server-0.8.7`, `tachys-0.2.18`, `reactive_graph-0.2.14`, `server_fn-0.8.13`, `js-sys-0.3.98`). Server code is behind the `ssr` feature; the same `src/` compiles to WASM under `hydrate`. Every file in this package contains **client-side component code** — reactive graph, not SQL.

**`web-sys` feature set — read this before writing any `web_sys` call.** `rust/web/Cargo.toml:77` declares `web-sys` with `["Location", "Window", "Document", "HtmlDocument", "Element", "MediaQueryList", "VisibilityState"]`. Cargo unions that with what the dependency graph enables — `tachys-0.2.18/Cargo.toml:193-312` adds `HtmlElement`, `HtmlInputElement`, `HtmlSelectElement`, `HtmlOptionElement`, `MouseEvent`, `SubmitEvent`, … and `leptos-use` adds `NodeList`/`EventTarget`/`EventListenerOptions` through the three features this crate enables (`use_websocket`, `use_event_listener`, `use_document`; `rust/web/Cargo.toml:80`). **`Navigator` is NOT enabled by anything in the `ssr` graph** — the only crates that would enable it are `leptos-use`'s `use_window`/`use_web_lock` features (not enabled, `leptos-use-0.19.0/Cargo.toml:402`, `:423`) and `whoami`, whose `web-sys` dependency is `optional` and target-gated to `cfg(target_arch = "wasm32")` (`whoami-1.6.1/Cargo.toml:59-66`), so it contributes nothing to a native `--features ssr` build. Consequence: **`web_sys::Window::navigator()` does not compile in this crate.** Task 11 depends on this fact. Do not add a feature to `Cargo.toml` to work around it — dependency changes belong to WP-43 (Non-Goals).

### THE ERROR-SLOT PATTERN (copy this; do not invent another)

Two existing implementations. Read both before writing a line.

**1. `GameCommandInput` (`rust/web/src/components/game.rs:583-591`, rendered at :657-659)** — a derived closure over the action's value, a per-outcome message, and *never* the raw error:

```rust
    let error_msg = move || {
        submit_action.value().get().and_then(|r| match r {
            // Game-rejected command: expected user-input feedback.
            Ok(Some(msg)) => Some(format!("Invalid command: {}", msg)),
            Ok(None) => None,
            // Transport/server fault: never leak the raw ServerFnError text.
            Err(_) => Some("Failed to submit command. Please try again.".to_string()),
        })
    };
```

```rust
                {move || error_msg().map(|e| view! {
                    <div class="command-error">{e}</div>
                })}
```

**2. `UsernameSection` (`rust/web/src/settings.rs:56-69`, rendered via `FormField`'s `error` slot at :84)** — an `RwSignal<Option<String>>` slot written by an `Effect` that matches on the action value:

```rust
    let error = RwSignal::new(None::<String>);

    let save_action = Action::new(|name: &String| {
        let name = name.clone();
        async move { crate::auth::set_username(name).await }
    });
    Effect::new(move |_| {
        if let Some(result) = save_action.value().get() {
            match result {
                Ok(field_error) => error.set(field_error),
                Err(_) => error.set(Some("Failed to save. Please try again.".to_string())),
            }
        }
    });
```

**The canonical shape this package standardises on** (variant 2, because every action in scope has *side effects* on success — a refetch, a signal revert, a navigation — which an `Effect` already owns):

```rust
let action_error = RwSignal::new(None::<String>);          // one slot per page/section
Effect::new(move |_| {
    match some_action.value().get() {
        Some(Ok(())) => { action_error.set(None); /* existing success work */ }
        Some(Err(e)) => action_error.set(Some(format!(
            "<Verb> failed: {}", crate::error::action_error_message(&e)
        ))),
        None => {}
    }
});
// ...
{move || action_error.get().map(|e| view! { <div class="form-error">{e}</div> })}
```

Three rules that come with it:

- **One slot per page or per section**, not per action (the triage note's "shared error slot is fine"). The prefix (`"Concede failed: …"`) is what tells the user which action broke.
- **Render class `form-error`, not `error`.** `main.scss` defines exactly two error classes: `.game-main .game-command-input .command-error` (:373-376) and `.form-error` (:715-717, `color: var(--mk-red)`). **`.error` has no rule at all** — `grep -n error rust/web/style/main.scss` returns exactly those two hits and nothing else (verified 2026-07-25) — so the **22** existing `class="error"` sites render in body colour, not red. See "Cross-package / newly discovered" #1 for the full inventory. Every slot you *add* uses `form-error`. Existing `class="error"` sites you touch for a different reason keep their class (restyling `.error` is out of scope).
- **The slot signal is `None` during SSR and on the first hydration pass** (it is only ever written from an `Effect` or an action result, and Effects are inert during SSR — `docs/hydration.md:80-104`, which quotes leptos's own "effects do not run on the server" and shows the `mounted`-gate idiom this crate uses; the in-crate statements of the same fact are `app.rs:151-153`, `components/game.rs:379-385` and `settings.rs:477-480`). So `{move || slot.get().map(|e| view!{ … })}` renders nothing on both sides: no structural hydration mismatch. This is why the slot is safe to add anywhere. **Note:** `docs/CODING.md` does *not* state this — an earlier draft of this spec cited `CODING.md:69-153` for it, which is wrong; that range is the resource-type/Suspense section.

### `ServerFnError` never goes through `Display`

`crate::error` (`rust/web/src/error.rs`, **16** lines, the whole file):

```rust
#[cfg(feature = "ssr")]
pub fn internal<E: std::fmt::Display>(context: &'static str) -> impl FnOnce(E) -> ServerFnError { … }

pub fn user_facing_server_error(_e: &ServerFnError) -> String {
    "Something went wrong, please try again".to_string()
}
```

`user_facing_server_error` (`error.rs:14-16`) is the *load-failure* helper — it discards the error entirely. It is already used at `new_game.rs:106`, `:212`, `:541` (and nowhere else — `grep -rn user_facing_server_error rust/web/src/` returns the definition plus those three).

**Verified crate fact (WP-37 and WP-59 both routed this here):** `impl<CustErr> Display for ServerFnError<CustErr>` renders the `ServerError(s)` variant as `format!("error running server function: {s}")` — `~/.cargo/registry/src/index.crates.io-*/server_fn-0.8.13/src/error.rs:218-250`, the `ServerError` arm at **:233-234**. Every other variant gets its own prefix (`Request` → `"error reaching server to call server function: "`, :230-232, etc.). So **any** `e.to_string()` shown to a user carries framework noise. `ServerFnError::new(msg)` builds `ServerError(msg)` (`error.rs:198-202`), the enum's variants are `pub` and it is **not** `#[non_exhaustive]` (`error.rs:165-197`), so it can be matched directly.

That matters because brdgme server fns *deliberately* raise user-facing rejections through `ServerFnError::new`: `"No user named {name}"` (`friends.rs:167`), `"You cannot friend yourself"` (`friends.rs:171`), `"Request not found"` (`friends.rs:205`, `:214`), `"You are not a player in this game"` (`game/server_fns.rs:1275` in `get_restart_prefill_impl`, `:1350` in `bump_bot_turns`, `:1198` in `restart_game_with_roster`), `"Game is not finished"` (`:1268`, `:1191`), `"You have already left this game"` (`:885`). Collapsing all of those to `"Something went wrong"` would be a **regression in usefulness**. Infrastructure failures are already opaque before they leave the server: `error::internal` logs the real error and substitutes `ServerFnError::new("Internal server error")`.

So Task 1 adds **one** helper, and every task uses it:

```rust
/// Message to show a user when a server-fn call returns `Err`.
///
/// `ServerFnError`'s `Display` impl prefixes every variant with framework
/// noise - `ServerError(s)` renders as "error running server function: {s}"
/// (server_fn-0.8.13/src/error.rs:233-234) - so `e.to_string()` must never
/// reach the UI. Server fns raise deliberate user-facing rejections with
/// `ServerFnError::new(msg)`, i.e. the `ServerError` variant, and
/// `error::internal` has already replaced genuine infrastructure failures
/// with the opaque "Internal server error" before they get here, so that
/// message is safe to show verbatim. Every other variant is transport or
/// (de)serialization and collapses to the generic text.
pub fn action_error_message(e: &ServerFnError) -> String {
    match e {
        ServerFnError::ServerError(msg) => msg.clone(),
        _ => user_facing_server_error(e),
    }
}
```

The `_` arm is mandatory rather than exhaustive matching: `ServerFnError::WrappedServerError` is `#[deprecated(since = "0.8.0")]` (`server_fn-0.8.13/src/error.rs:171-178`) and naming it would trip `-D warnings`.

**Which helper for which job:** `user_facing_server_error` for a failed **read** (a resource that would not load — the user has no action to correct and the message content is never useful); `action_error_message` for a failed **write** (a dispatched action, where the server's own rejection text is the useful part).

### The reactive facts the three Leptos bugs depend on

- **`LocalResource<T>` is `Copy`** (`leptos_server-0.8.7/src/local_resource.rs:294 impl<T> Copy for LocalResource<T> {}`) and never resolves during SSR (`docs/CODING.md:87`, "Always `None` on SSR"; mechanically, `ArcLocalResource::new` substitutes a `pending()` future under `cfg(feature = "ssr")` — `local_resource.rs:64-80`). That is why the existing hoists in `App` (`app.rs:126-143`: `logout_action` :126-127, `active_games` :129-134, `current_user` :138-143) work by plain `provide_context(resource)`, and why Task 7 can do the same.
- **Resources are stale-while-revalidate: a refetch never resets `.get()` to `None`.** `ArcAsyncDerived`'s spawn loop only writes the new value when the future resolves (`reactive_graph-0.2.14/src/computed/async_derived/arc_async_derived.rs:380-389`); nothing clears `value` first. The in-repo statement of this is `app.rs:763-765`. **This matters for Task 2:** a refetch that returns *identical* data therefore produces no DOM write at all (see the next two bullets), so a refetch cannot be used as a "re-sync the UI" mechanism.
- **Attributes are built before children, and a reactive attribute's effect runs immediately.** `HtmlElement::build` (`tachys-0.2.18/src/html/element/mod.rs:349-367`) does `let attrs = self.attributes.build(&el);` at **:352** and only then `self.children.build()` at **:357**. A reactive `prop:` builds a `RenderEffect` whose doc comment reads *"Creates a new render effect, which immediately runs `fun`"* (`reactive_graph-0.2.14/src/effect/render_effect.rs:61-62`), via `tachys-0.2.18/src/reactive_graph/property.rs:36` → `html/property.rs:83-88`. **Consequence, and it is stronger than an earlier draft of this spec claimed: `prop:value` on a `<select>` is written to an element that has no `<option>` children yet, so on first build it selects nothing.** The browser then auto-selects the first option as the options are inserted. `prop:value` only takes effect on *later* runs of its effect, i.e. when a signal it tracks changes after mount. Task 8 depends on this.
- **Unchanged attribute values are not re-written on rebuild.** `impl AttributeValue for bool::rebuild` (`tachys-0.2.18/src/html/attribute/value.rs:554-563`) compares against the previous value and returns without touching the DOM when equal. And `AnyView::rebuild` (`tachys-0.2.18/src/view/any_view.rs:386-400`) rebuilds **in place** whenever the `TypeId` matches, i.e. whenever the same `match` arm is taken again — it does not recreate the subtree. Together these are why Task 2 cannot rely on a refetch to re-sync a `<select>`.
- **`<option selected>` sets `defaultSelected`, and only re-syncs an option whose "dirtiness" flag is false** (HTML spec: user interaction with a `<select>` sets the newly-selected option's dirtiness to true). So once the user has touched a select, re-writing `selected` on its options does not change what is displayed — even if the attribute value did change.
- **`docs/CODING.md:305-310` is binding and forbids the obvious alternative:** *"`<option selected>` only sets `defaultSelected` — drive the value via `prop:value` on the `<select>`. … doing it per-`<option>` fights hydration."* So Task 8 may **not** switch to per-option `selected=`, even though `friends.rs:560`/`:572` does exactly that. (Those two selects are pre-existing and out of scope; do not "fix" them either way — see Cross-package #6/#7.)
- **`StoredValue` is the non-reactive holder** for revert snapshots: `get_value` (`reactive_graph-0.2.14/src/traits.rs:766`), `set_value` (`:855`), `update_value` (`:819`). Already used in this codebase at `new_game.rs:394`, `opponent_slot.rs:64`, `app.rs:777`.
- **`crate::auth::AuthUser` is `{ id: Uuid, name: String, email: String }`** (`rust/web/src/auth/server.rs:117-122`) — `id` is what Task 6 keys the latches on.
- **A successful login refetches `current_user` client-side without a page load** — `LoginPage`'s confirm effect at `app.rs:511-517` calls `current_user.refetch(); active_games.refetch(); navigate("/", …)`. This is the premise that makes wfe F54 (Task 6) reachable at all: a logout→login round trip in one tab never reloads the page, so a one-shot latch is never re-armed.
- **`NodeRef<E>` is an `RwSignal<Option<E::Output>>` populated during element build**, and `leptos::html::Select`'s output type is `web_sys::HtmlSelectElement` (`tachys-0.2.18/src/html/element/elements.rs:380`; the `HtmlSelectElement` web-sys feature is enabled by `tachys-0.2.18/Cargo.toml:304`). An `Effect` that reads a `NodeRef` runs *after* the render pass, so by then the element has its children — that is the lever Task 8 uses.
- **`crate::components::confirm(&str) -> bool`** is the whole of `rust/web/src/components/confirm.rs` (5 lines) and is re-exported by `components/mod.rs:11`. `proposals.rs` calls it at :2007, :2023, :2051, :2063, :2133. `components/mod.rs` has both `pub mod confirm;` and `pub use confirm::*;`, and modules and functions live in different namespaces, so `crate::components::confirm("…")` unambiguously resolves to the function — proven by those five existing call sites.

### The eight files

| File | Live lines | What lives there |
|---|---|---|
| `rust/web/src/error.rs` | 16 | `internal` (:6-12), `user_facing_server_error` (:14-16). **Not in the package path list but Task 1 adds one fn here — see Non-Goals.** |
| `rust/web/src/components/game.rs` | 681 | `GameBoard`, `GameMeta` (:25-217, five `ServerAction`s at :49-53), `PlayerInfo` (:219-303), `window_key` (:305-308), `format_log_time` (:310-325), `render_log_entries` (:327-366), `GameLogs` (:368-414), `RecentGameLogs` (:416-…), `PlayerName`, `GameCommandInput` (:492-681) |
| `rust/web/src/friends.rs` | 581 | server fns (:84-351) then `FriendsPage` (:353-581) with six `ServerAction`s at :363-368 |
| `rust/web/src/settings.rs` | 572 | `SettingsPage` (:11-43), `UsernameSection` (:48-107), `ColorsSection` (:112-177), `EmailPreferencesSection` (:179-240), `EmailSection` (:242-465), `ThemeSection` (:471-572) |
| `rust/web/src/app.rs` | 924 | `shell`, `App` (:105-237; latches at :145-173 and :175-193; the three hoisted contexts at :126-143), `set_theme_client` (:246-272), `local_data_theme` (:277-282), `HomePage` (:284-…), `LoginPage` (:470-653), `GamePage` (:655-…) |
| `rust/web/src/components/layout.rs` | 316 | `SubMenuOpen` (:12-16), `next_game_id` (:20-26) + its two unit tests (:301-315, inside `mod tests` :281-316), `MainLayout` (:28-114), `SidebarMenu` (:116-279) |
| `rust/web/src/components/opponent_slot.rs` | 352 | `SlotMode` (:8-13), `OpponentSlot` (:17-28, `Default` :40-47), `OpponentSlotEditor` (:53-352) |
| `rust/web/src/components/mod.rs` | 15 | module list + re-exports; two stale comment lines at :1-2, `pub use confirm::*;` at :11 |
| `rust/web/src/new_game.rs` | 660 | pure helpers (`player_range` :18-36, `weight_text` :38-40, `prefill_to_slots` :42-57, `filter_and_sort`), `NewGameTypePage` (:94-…), `GameTypeGrid`, `NewGameSetupPage` (:191-…), `GameSetupPanel` (:232-549), `#[cfg(test)] mod tests` (:551-660) |

---

## Tech Stack

Rust 1.97.0, edition 2024 (`rust/rust-toolchain.toml`; `rust/web/Cargo.toml:5`). Leptos 0.8.20 / leptos_router 0.8.14. One crate: `web`, feature-gated — **every command in this spec passes `--features ssr`**. Let-chains, `let … else`, `Option::is_none_or`/`is_some_and` are all available and already used in these files.

## Global Constraints

- Run all cargo commands from `/home/beefsack/Development/brdgme/rust`.
- **Per-package only**: `cargo test -p web --features ssr`, `cargo clippy -p web --all-targets --features ssr -- -D warnings`. **Never** a workspace-wide build/test (AGENTS.md "Resource constraints": ~30 binaries, RAM/disk spike).
- Each task ends with clippy clean at `-D warnings` and `cargo fmt --all -- --check` clean.
- **No server function's signature, name, argument list or return type changes in this package.** Every fix is client-side observation of a result that already crosses the wire. No SQL, no migration, no `#[server]` body edit. (`error.rs`'s new pure helper is the single exception to "client-side only", and it is not a server fn.)
- **No structural first-render change.** Every element this package adds is inside a `{move || option.map(…)}` whose signal is `None` during SSR **and** on the first hydration pass, or inside a subtree that never renders on the server at all. `docs/CODING.md:139-152`: element-type/hierarchy differences panic; attribute/class differences are tolerated. If a step would change what SSR emits, it says so explicitly and explains why it is safe.
- Existing tests must keep passing unmodified **except** `ssr_pages.rs::game_page_anonymous_visitor_gets_clean_error_not_panic`, which Task 5 strengthens deliberately.
- Line numbers below are **live-file** numbers verified against the working tree on 2026-07-25 **by an adversarial review pass that re-read every one of them**. Numbering shifts as tasks land; every task after the first locates its edit by **symbol name plus quoted anchor text**, not by line number alone. Where an earlier draft of this spec had a number wrong in a way that would have broken the build, the corrected text says so inline — do not "restore" the old range.
- Run `/home/beefsack/Development/brdgme/scripts/rust-test.sh` once before the **final** commit of the package (it provisions the throwaway Postgres 18 / NATS containers). DB-backed failures in a bare run without it are pre-existing (AGENTS.md; backlog #40) — not a regression.
- **Do not run `tilt up` / `kind`** to verify anything (AGENTS.md: never on a <32GB machine). The manual checklists in this spec are written for whoever has the 32GB+ environment and are explicitly deferred, not skipped-and-claimed. Never mark a manual checklist as passed without having actually clicked through it.

---

## Disposition table

Verdicts are first-pass verification: **units 11 (`web-domain`) and 12 (`web-frontend-email`) were never verified** (the verification pass covered units 1-9), so every row below was re-derived from live source for this spec.

| # | Claim | Verdict | What the spec does, and why |
|---|---|---|---|
| **wfe F52** (major) | `GameMeta` mutation actions swallow errors | **CONFIRMED + WIDENED** | All five actions verified: `undo_action` effect `game.rs:58-63`, `concede_action` :64-69, **`end_game_action` :70-75 (new in #47 — a third destructive action the finding could not have seen)**, `force_delete_action` :80-85, and `bump_bot_action` with **no watcher at all** (declared :52, dispatched :176-179, never read). All five match `Some(Ok(()))` only. Task 1 gives all five one shared slot. |
| **wfe F56** | `GameMeta` inlines confirm dialogs | **CONFIRMED + WIDENED** | Three inline `web_sys::window().and_then(\|w\| w.confirm_with_message(…).ok()).unwrap_or(false)` blocks, not two: concede `game.rs:126-128`, **end-game `:139-141` (#47)**, force-delete `:191-193`. Byte-identical to `components/confirm.rs:1-5`. Folded into Task 1 (same handlers). |
| **wd F57** | Friends page mutation errors silently swallowed | **CONFIRMED; the finding's "re-sync the selects from the refetched overview on failure" recommendation is OVERTURNED** | `respond_action`, `unfriend_action`, `unblock_action`, `policy_action`, `visibility_action` (`friends.rs:364-368`) have success-only effects at :373-398 (policy has none at all) and no error render anywhere. Only `add_action` renders, at :426-428 — and it uses `e.to_string()`. **The finding's second recommendation cannot work and Task 2 must not implement it:** a refetch after a *rejected* change returns byte-identical data, so the `selected=selected` bool attribute compares equal and `AttributeValue for bool::rebuild` writes nothing (`tachys-0.2.18/src/html/attribute/value.rs:554-563`); the enclosing `AnyView` also rebuilds in place rather than recreating the `<select>` (`any_view.rs:386-400`), and even a changed `selected` content attribute would not move a select the user has already touched (option dirtiness). Task 2 therefore reports the error and leaves the residual visual desync recorded as Cross-package #7. |
| **wd F58** | `SetInvitePolicy` success does not refetch | **CONFIRMED as an inconsistency; user-visible impact NARROWED to ~nil** | Read all six: `add_action` :373-378, `respond_action` :379-383, `unfriend_action` :384-388, `unblock_action` :389-393, `visibility_action` :394-398 all bump `set_refresh`. `policy_action` is the **only** one without — five-to-one, so the sibling is the norm. **But the "refetch re-syncs the select" rationale is false** (see wd F57 above), and `o.invite_policy` is read nowhere except that one `<select>` (`friends.rs:559`), so the concrete consequence of the missing refetch is only that the client's cached `overview` copy of `invite_policy` is stale until some other action refetches or the page remounts. Fixed anyway, because (a) an internally inconsistent set of six effects is a maintenance trap and (b) the cached value must not lie. Add the effect; do **not** drop it from the others. Task 2. |
| **wd F59** | Restart prefill `Err` silently swallowed | **CONFIRMED; the finding's error list corrected** | `new_game.rs:270-279`: `let Some(Some(Ok(pf))) = prefill.get() else { return; };`. `get_restart_prefill` is `game/server_fns.rs:1321-1332`, a thin wrapper that raises `"Not authenticated"` (:1329) and delegates to `get_restart_prefill_impl` (`:1257-1319`), which raises `"Game not found"` (:1265), `"Game is not finished"` (:1268), `"You are not a player in this game"` (:1275) and `"Game type not found"` (:1287). **It cannot return `"Game version not found"`** — that message is `restart_game_with_roster:1204`, a different fn; an earlier draft of this spec cited `:1180-1204` (i.e. `restart_game_with_roster`) as if it were the prefill fn. All five real rejections are discarded today, leaving a default form headed `"Restarting <name>"` (`new_game.rs:399-402`). Task 9. |
| **wd F63** (nit) | Submit silent no-op with no version | **CONFIRMED** | `new_game.rs:355-357`, bare `return`, while every other guard sets `form_error` (:368-371). Reachable: `gt.versions` empty → `selected_version_id` starts `None` (:243); or the version `<select>`'s `parse::<Uuid>().ok()` at :429 yields `None`. Task 9, with a **stated assumption** on wording. |
| **wd F64** (nit) | Both-`None` outcome navigates nowhere | **CONFIRMED** | `new_game.rs:316-324` (create) and `:348` (`RestartOutcome::AlreadyRestarted { .. } => {}`) both fall through silently on a **successful** mutation. Task 9, with a **stated assumption** on wording. |
| **wd F66** (nit) | Prefill can select an unoffered player count | **CONFIRMED; recommendation narrowed to the first of its two options** | `new_game.rs:274,278` sets `player_count` to `pf.opponents.len() + 1` with no reference to `gt.player_counts`; the radios (:470-487) render only from `gt.player_counts` (:470), so `prop:checked` (:478) is false on all of them while `on_submit` still submits the stale count. The finding's second option (render the union including the prefill value) is **rejected**: it would offer the user a count the game type cannot start, moving the failure to the server. **`gt.player_counts` is the right clamp target, not `pf.player_counts`:** `RestartPrefill` does carry its own `player_counts` (`game/server_fns.rs:151`, populated from the *latest non-deprecated* version at `:1284-1287`), but `new_game.rs` never reads it — the radios come from `GameTypeInfo::player_counts` (`:138`). Clamping to what the radios render is what makes a radio check. The unused `pf.player_counts` is recorded as Cross-package #8. Task 9 extracts a pure `clamp_player_count` and tells the user. |
| **wd F73** | Fire-and-forget settings mutations; optimistic UI never reverts | **CONFIRMED for Colors + EmailPrefs; ADJUSTED for Theme** | `ColorsSection` `save_action` (`settings.rs:135`) dispatched at :145-147 after `colors.update(…)` :137-144, never read. `EmailPreferencesSection`'s three actions (:200-202) dispatched at :213, :224, :235 after `turn/invite/reminder.set(val)` (:212, :223, :234), never read. Both get revert + slot. The revert genuinely works here (unlike the friends selects, wd F57) because both controls are driven by `prop:checked`/`prop:value` over a **signal** — writing a different value into the signal re-runs the property effect on an element that already exists, so the DOM follows. **`ThemeSection` must NOT revert**: `select()` (`settings.rs:488-499`) calls `set_theme_client(slug)` first (:494), which writes `document.documentElement[data-theme]` (`app.rs:255-264`) **and a 1-year `theme` cookie** (`app.rs:265-271`, `max-age=31536000` at :267), so the choice is already effective and persisted on this device — only the *profile* sync failed. Reverting would undo a change that did succeed. It gets a non-reverting advisory line instead. Task 3. |
| **wfe F54** | Presence-ping and profile-theme latches never reset | **CONFIRMED; the finding's second option adopted** | `applied_profile_theme` (`app.rs:154`) and `presence_started` (`app.rs:179`) are `RwSignal<bool>` set true once (:158, :182) and never cleared; the ping loop breaks on logout (:185-187). Both effects re-run when `current_user` changes (they call `current_user.get()` at :156/:181), so the *only* thing blocking a restart is the latch — and a re-login *does* re-resolve `current_user` in the same tab, because `LoginPage`'s confirm effect calls `current_user.refetch()` and then client-side-navigates (`app.rs:511-517`), never reloading. Key on `Option<Uuid>` **and** clear on observed logout — id-keying alone would not re-arm a logout→login as the *same* user in the same tab. Task 6. **(The finding's own anchors — `app.rs:179`, `:185-187`, `:180-193`, `:154-173` — are correct against live source; an earlier draft of this spec "re-derived" them five lines low. Trust the numbers in this table.)** |
| **wfe F55** | `GamePage` error branch leaks raw `ServerFnError` | **CONFIRMED** | `app.rs:762`: `Err(e) => view! { <div class="error">"Error: " {e.to_string()}</div> }`. Because `game_data` is `Resource::new_blocking` (`app.rs:700-708`, with the locally-generated `ServerFnError::new("Invalid Game ID")` at :705) inside a `<Transition>` (`:756`, which renders children directly on SSR — `docs/CODING.md:135-137`), this text reaches the **server-rendered HTML**, e.g. `Error: error running server function: Not authenticated` for an anonymous visitor. Uses `user_facing_server_error`, not `action_error_message`: this is a failed read. Task 5. |
| **wfe F57** | `friend_request_count` recreated per navigation | **CONFIRMED** | `layout.rs:135-136` creates it locally in `SidebarMenu`; the comment at :126-129 documents that every page wraps its own `<MainLayout>` so the sidebar remounts per navigation, which is exactly why `active_games`/`current_user` were hoisted to `App` (`app.rs:126-143`). It also never tracks `last_update`, unlike `active_games` (`app.rs:129-133`). Task 7 hoists it, newtyped, and tracks the WS trigger. |
| **wfe F58** | Bot difficulty select can desync | **CONFIRMED (finding filed UNCERTAIN) and WIDENED — the desync starts at first render, not at resource resolution; BOTH of the finding's options are rejected and a third fix is derived** | `opponent_slot.rs:317-320` `prop:value` tracks only `slot()`; the options closure :328-346 tracks only `bot_names`. Two independent defects here, both verified from library source: **(a)** `prop:value` is applied during `attributes.build` (`tachys-0.2.18/src/html/element/mod.rs:352`) *before* `children.build` (`:357`), and a reactive `prop:` `RenderEffect` runs immediately on creation (`reactive_graph-0.2.14/src/effect/render_effect.rs:61-62`) — so on first build the property is written to an option-less `<select>` and selects nothing, and the browser then auto-selects the **first** option. The select therefore shows `"easy"` (or the server list's first entry) while `slot`'s `bot_name` says `"medium"` **from the very first paint**. **(b)** When `bot_names` resolves, the options closure re-runs; `collect_view()`'s `Vec` rebuild reuses the existing `<option>` elements and rewrites their `value` attributes in place, so the still-selected element now carries a different value (or is dropped if the new list is shorter, which then resets selectedness to the first option) — while `prop:value`'s effect does not re-run, because `slot()` did not change. Finding option 1 (*"track `bot_names` from `prop:value` too"*) **cannot** fix either half: the build-order and immediate-run facts above mean the property still lands before the options. Finding option 2 (*"render the select only once `bot_names` is available"*) fixes **(b)** only — **(a)** survives, because a select mounted later is still built attributes-first. Per-`<option>` `selected=` is forbidden by `docs/CODING.md:305-310`. Task 8 therefore does **both**: gate the element on the *settled* resource (kills (b)) **and** apply the value from an `Effect` over a `NodeRef` (kills (a), because Effects run after the render pass, when the options exist). |
| **wfe F59** | Logout failure gives no feedback | **CONFIRMED** | `layout.rs:120-124`: `if logout_action.value().get().is_some_and(\|r\| r.is_ok())`. Task 4. |
| **wfe F60** (nit) | `format_log_time` hardcodes en-US | **CONFIRMED; resolved by the "locale-following" branch — see stated assumption** | `components/game.rs:310-325` (doc comment :310-311, fn :312-325): comment claims "browser's local time zone via Date.toLocaleString", call is `date.to_locale_string("en-US", &options.into())` (:324) with `hour12: true` forced (:323). Time zone is genuinely local; wording/order/clock convention are not. **Do not extend the edit range to :307** — an earlier draft of this spec said `:307-325`, which eats `window_key`'s body (`:306-308`) and would not compile. **No hydration risk**: `format_log_time` is called only from `render_log_entries` (`game.rs:346`), called only from `GameLogs` (`:409`) and `RecentGameLogs` (`:446`), both gated behind `mounted` (`game.rs:386-387`, `:425-426`) which is `false` on SSR *and* on the first hydration pass by design (`:379-385`). Task 11. |
| **wfe F61** (nit) | Click-only anchors keyboard-inaccessible (3 sites) | **CONFIRMED; all three located, and they are the only three** | `layout.rs:166-171` (logout), `app.rs:603` ("I already have a login code"), `app.rs:623` ("Logging in as <email>"). Each is `<a on:click=… style="cursor:pointer">` with no `href`, `tabindex` or `role`. `grep -rn 'cursor:pointer' rust/web/src/` returns exactly these three (`app.rs:603`, `app.rs:623`, `layout.rs:170`) — nothing else in the crate uses an inline pointer cursor, which is what makes the SSR assertion in Task 10 sound. **`<button>` is rejected for all three** — see Task 10 for the per-site reasoning. Task 10. |
| **wfe F62** (nit) | `components/mod.rs` placeholder comment stale | **CONFIRMED** | `components/mod.rs:1-2`. The module lists six submodules and five glob re-exports. Task 11. |

Counts: **17 in scope — all 17 premises CONFIRMED, 0 overturned premises, 0 skipped, 0 fenced out.** Of the 17, **4 have a recommendation that was changed** (wd F73's Theme arm; wd F57's "re-sync the selects on failure", which is OVERTURNED as unimplementable; wd F66's second option, rejected; wfe F58, where *both* of the finding's options are rejected), **2 were widened by #47's new code** (wfe F52, wfe F56), and **1 was widened by mechanism** (wfe F58 — the desync starts at first paint, not at resource resolution).

The 17 map to `work-packages.md:424`'s scope list exactly: `wd F57, wd F58, wd F59, wd F63, wd F64, wd F66, wd F73, wfe F52, wfe F54, wfe F55, wfe F56, wfe F57, wfe F58, wfe F59, wfe F60, wfe F61, wfe F62`. Severity split matches `work-packages.md:426` (`1M/10m/6n`): major = wfe F52; nits = wd F63, wd F64, wd F66, wfe F60, wfe F61, wfe F62; the remaining ten are minor.

**Verdicts are not the findings' own words.** Units 11 (`web-domain`) and 12 (`web-frontend-email`) were never covered by the verification pass (it covered units 1-9), so every row above was re-derived from live source. Where a finding's *recommendation* is wrong it says so explicitly, and where a finding's *line anchors* are right and an earlier draft of this spec was wrong, it says that too.

### Stated assumptions (Lead: these are product-wording/behaviour calls I made rather than guessed silently)

> **A1 — wd F63 message.** Submitting with no version selected shows the existing `form_error` slot with `"No game version is available for this game type."` *Alternative:* disable the submit button whenever `selected_version_id` is `None`. Rejected because a permanently-dead button with no explanation is less informative than a sentence, and because the same slot already carries the sibling validation error at :368-373.
>
> **A2 — wd F64 message.** A **successful** create/restart whose outcome carries neither `game_id` nor `proposal_id` shows `"Created, but no game or invite link came back. Check your games in the menu."` It is deliberately not phrased as a failure, because the mutation succeeded. *Alternative:* make the state unrepresentable in `ProposalOutcome`/`RestartOutcome` server-side (the finding's own second option) — that is a server-fn type change, i.e. **WP-53** territory, and is recorded under Cross-package.
>
> **A3 — wd F66 tie-break direction.** `clamp_player_count` breaks distance ties **upward** (offered `[2,4,6]`, prefill `3` → `4`, not `2`). Rationale: clamping *up* leaves a visibly empty opponent slot which `on_submit`'s existing guard already refuses with a clear message (`new_game.rs:367-372`), whereas clamping *down* silently truncates a real opponent out of the roster (the resize effect at `:265-268` calls `resize_with`). A loud failure beats a silent data loss. *Alternative:* tie-break downward, or refuse to prefill at all and show only the error.
>
> **A4 — wfe F60 locale direction.** Timestamps switch to the browser's own locale **and** drop the forced `hour12: true`, so each locale uses its own clock convention. This changes visible output for non-en-US users (e.g. `11 Jul, 22:50` for en-GB). **The locale is read via `js_sys::Reflect` off the JS global, not via `web_sys`** — `js_sys::Date::to_locale_string`'s binding is `(this: &Date, locale: &str, options: &JsValue)` (`js-sys-0.3.98/src/lib.rs:6689`), so `undefined` cannot be passed through the `&str` parameter, and `web_sys::Window::navigator()` does not compile in this crate (see the web-sys feature note in Architecture). Task 11 therefore reads `globalThis.navigator.language` reflectively, exactly as `get_turnstile_response` (`app.rs:458-468`) reaches `globalThis.turnstile.getResponse`. This is deterministic — there is **no** "if it does not compile, fall back" branch. *Alternative, if the Lead judges the visual change unwanted:* keep `"en-US"` + `hour12` and change only the comment to say "US formatting, browser-local time zone". That alternative is a one-line comment edit and is not the default.
>
> **A5 — settings/theme advisory wording.** `ThemeSection`'s non-reverting line is `"Theme applied on this device, but saving it to your profile failed."`
>
> **A6 — wd F57 residual (NEW, added by the review pass).** After Task 2, a *rejected* invite-policy or game-visibility change shows an error but the `<select>` **keeps displaying the value the user picked** until the page is reloaded or `FriendsPage` remounts. Reverting it properly requires converting both selects from per-`<option>` `selected=` to a `prop:value`-over-signal binding (which `docs/CODING.md:305-310` wants anyway) — a change to markup this package otherwise only reads, and one that interacts with the build-order hazard documented for Task 8. It is recorded as **Cross-package #7** and routed to the `friends.rs` owner rather than absorbed here. Task 2's manual checklist states the residual as the *expected* outcome, so nobody mistakes it for a regression. *Alternative:* absorb the conversion into Task 2 — Lead call.

---

## Non-Goals (owned elsewhere — do NOT absorb)

- **`rust/web/src/admin.rs` is NOT in this package. LEAD RULING, honour it verbatim.** `work-packages.md:423-429` is authoritative on paths and does not list `admin.rs`; none of the 17 findings is an `admin.rs` finding. WP-37 originally routed `admin.rs` presentation-layer error rendering here and has since **recorded the correction**: `WP-37-admin-pass.md:2349-2350` ("this is no longer WP-54's … WP-54 has since explicitly refused it by LEAD RULING"), reinforced at `:45` and `:89` ("**WP-54** does not touch `admin.rs`; no coordination needed"). WP-59 records the same routing note at `WP-59-inbound-processing-quality.md:2754`. *(An earlier draft of this spec cited `WP-37:2251` and `WP-59:2494-2504`; both are unrelated passages — `WP-37:2251` is about `ProvidersSection`, `WP-59:2494-2504` is about `bump_reply`.)* **Do not open `admin.rs`. Do not add it to any file list. Do not `use` your new helper from it.** The concrete outstanding sites are all **15** hits of `grep -n 'e.to_string()' rust/web/src/admin.rs`: `:1024`, `:1041`, `:1119`, `:1132`, `:1144`, `:1156`, `:1431`, `:1444`, `:1456`, `:1468`, `:1767`, `:1780`, `:1792`, `:1804`, `:1827` — recorded under Cross-package #4 as a WP-37 follow-up / its own small package.
- **wfe F53 (Turnstile never renders after SPA navigation to `/login`) — WP-55**, BLOCKED-ON-DECISION D-16. It edits `app.rs:589-598` (the `{move || …}` block that conditionally emits the `cf-turnstile` div at :595, inside `LoginPage`) and may force a full page load for `/login`. **Task 10 edits `app.rs:549-551` (`show_code_link`), `:603` and `:623` — all outside that block — and changes no `<form>` structure and no `site_key` handling.** Landing order: **WP-54 lands first** (READY vs blocked); WP-55 rebases onto it. Do not pre-empt any part of F53 — in particular, do not "fix" the Turnstile div, do not call `turnstile.render()`, and do not convert `/login` links to hard navigations.
- **D-15 (REOPENED: the email verb `end` collides with live top-level game moves in acquire-1 / starship-catan-1 and with the email dispatcher's own `end` arm at `rust/web/src/email/commands.rs:1217`) gates WP-59's Task 14, not anything here.** WP-54 adds no email verb, no dispatcher arm and no email copy. The only `end`-adjacent thing it touches is the **UI** label prefix `"End game failed: "` in `GameMeta` (Task 1) and the existing `EndGame` *server action dispatch* — no server fn, no grammar, no verb table. **Nothing in this package is gated on D-15.** Do not "harmonise" the UI wording with whatever D-15 decides for the email verb.
- **wfe F63 (sentry snippet escaping does not cover `</script>`) — WP-60.** It edits `app.rs:53-57` (`js_string_escape`) and `theme.rs`. **Not yours.** Do not touch `js_string_escape`, `sentry_init_snippet`, or `shell()`.
- **WP-53 (domain misc server fns)** touches `friends.rs`, `settings.rs`, `game/mod.rs`, `game/server_fns.rs`, `players.rs`, `models/game.rs`, `db.rs`, `stats/viz.rs`. Its `friends.rs` work is **server-side**: wd F56 (`block_user` target existence — the `#[server(BlockUser)]` attribute is `friends.rs:229`, the fn body `:230-240`) and the other authz nits, all in the server-fn region **`friends.rs:84-351`**. Its `settings.rs` work is server-side too (the `#[server]` bodies live in `auth/server.rs`, not in `settings.rs`'s components). **WP-54 touches only `friends.rs:353-581` (`FriendsPage`) and only the component bodies in `settings.rs` (`ColorsSection` :112-177, `EmailPreferencesSection` :179-240, `EmailSection`'s four effects :276-336, `ThemeSection` :471-572).** Disjoint line ranges; either order lands. Do not change any `#[server]` fn in `friends.rs`.
- **WP-52 (stats and query performance)** also lists `friends.rs`: wd F65 (`get_friends_overview` six sequential queries, `friends.rs:95-133`) — server-side, same fence as above.
- **WP-51 (invite-mailer / notify dedup)** owns `proposals.rs` and `email/`. Task 1's `confirm()` conversion must **not** touch `proposals.rs`' five existing `crate::components::confirm` call sites; they are already correct and are only read as precedent.
- **WP-47 (D-6)** owns `game_visibility` gates. `friends.rs`' `SetGameVisibility` **server fn** and the visibility predicate are its business; WP-54 only adds an error slot and reads the existing `visibility_action` effect.
- **WP-50 (D-9, email canonicalisation)** owns wd F60 / wd F72 — trimming and lowercasing email input in `new_game.rs` and `settings.rs`. Task 9 edits `new_game.rs`'s `on_submit` for wd F63/F64 but **must not** add any trim/lowercase/empty-check to the `OpponentSlot::Email(email) => emails.push(email)` arm (`new_game.rs:374`). That is wd F62's own finding and WP-50's fix.
- **WP-49** owns `rules.rs` (including its `class="error"` + `e.to_string()` at `rules.rs:46`). **WP-52** owns `players.rs` (`:240`, `:273`, `:539`, `:795`). Leave both.
- **`components/game.rs:286`** (`PlayerInfo`'s add-friend `<span class="error">{e.to_string()}</span>`) is **not fixed by this package** even though it is in a file this package edits — see Cross-package #3 for the reasoning and the route. Do not "tidy" it while you are in the file.
- **Restyling `.error`** (adding a CSS rule for the eight `class="error"` sites). `rust/web/style/main.scss` is in no package's path list. Cross-package item #1; do not edit `main.scss`.
- **The stale `rust/web/end2end/tests/page-loads.spec.ts`.** It is already broken against live code (Cross-package #2). Do not repair it, and do not add assertions to it.
- **`rust/web/src/settings.rs:1-2`'s "email placeholder" stale module doc** is its own web-domain finding routed to another package (it appears in the `game_info/, models/, rules.rs, settings.rs, index.rs` group, not in WP-54's 17). Leave the module doc alone even though you are editing the file.

### Coordination / landing order

1. **Land WP-54 before WP-55.** WP-55 (D-16) is blocked; WP-54 is READY. Both edit `LoginPage` in `app.rs`; WP-54's edits are `show_code_link` (`:549-551`) and two anchors (`:603`, `:623`), WP-55's are the Turnstile block (`:589-598`) and possibly the route's navigation mode.
2. **WP-60 also edits `app.rs`** (`js_string_escape`, `:55-57`) and `theme.rs`. Non-overlapping with WP-54's `app.rs` edits (the load-error line at `:762`, the latches at `:145-193`, the `friend_request_count` hoist after `:143`, and `LoginPage`'s `:549-551`/`:603`/`:623`). Either order.
3. **WP-53 / WP-52 vs WP-54 in `friends.rs` and `settings.rs`:** server-fn region vs component region, no shared line. Either order. If WP-53 lands first and renames a user-facing rejection message, Task 2's manual checklist expectations shift accordingly — the checklist says "the server's message", not a literal string, for exactly that reason.
4. **`error.rs`:** WP-54 is the only READY package adding to it. If WP-37/WP-59 follow-ups later want `action_error_message`, it will already be there.
5. **Internal order is strict:** Task 1 first (it creates `action_error_message`, called by Tasks 2, 3, 4, 8 and 9, and it is the in-repo precedent later tasks point at). Task 5 does **not** need it — it uses the pre-existing `user_facing_server_error` — but still runs after Task 1 so the `app.rs` hunks stay ordered. Then Task 4 before Task 7 before Task 10 (all three edit `layout.rs`; this order keeps their hunks apart). Task 5 before Task 6 before Task 7 before Task 10 for `app.rs`.
6. **Line numbers shift as tasks land.** Every anchor in this spec is a **pre-task-1** live number. From Task 2 onward, locate each edit by the quoted anchor text plus the enclosing symbol name, and treat the line number as a sanity check only. The tasks that edit the *same* file as an earlier task are: Task 5/6/7/10 (`app.rs`), Task 4/7/10 (`layout.rs`), Task 1/11 (`components/game.rs`), Task 5/10 (`tests/ssr_pages.rs`).

---

### Task 1: add `action_error_message` and give `GameMeta` a real error slot (wfe F52 **major**; wfe F56)

**Problem (restated):** `GameMeta` owns five `ServerAction`s (`components/game.rs:49-53`). Four have `Effect`s that match `Some(Ok(()))` and drop `Some(Err(_))` on the floor (:58-63 undo, :64-69 concede, **:70-75 end-game**, :80-85 force-delete); `bump_bot_action` (declared :52, dispatched :176-179) has no watcher at all. Three of those actions are destructive — concede forfeits a live game, end-game finishes it for everyone, force-delete removes it permanently for all players. On a server error (session expired mid-session, transient 500, an authz rejection like `"You have already left this game"` at `game/server_fns.rs:885`, or `"Admin access required"` at `:1357`) nothing on the page changes: the mutation failed so there is no WS bump, so there is no refetch either, so the user's only signal is *absence of change* — indistinguishable from a slow network. The same file already does this correctly twice (`GameCommandInput`'s `error_msg` :583-591; `PlayerInfo`'s `add_friend` match :284-296, whose `Err` arm is :286). **#47 added `end_game_action` after the finding was written; it is the same defect at a new site and is in scope.**

The three confirm dialogs in the same handlers are hand-inlined copies of `components/confirm.rs` (wfe F56) and are converted in the same edit because it is the same three closures.

**Fix (re-derived):** one shared `RwSignal<Option<String>>` on `GameMeta`, written by all five effects with an action-naming prefix, rendered once under the `"Actions"` heading. New `crate::error::action_error_message` supplies the message text so the server's deliberate rejection (`"Game is already finished"`, `"You are not a player in this game"`) survives while `Display`'s framework prefix never appears.

**Edge cases:**
- The slot is cleared on *success* as well, so a retry that works removes the stale message.
- `force_delete_action`'s success arm navigates away (`:82-83`); clearing the slot first is harmless and keeps the arms uniform.
- `bump_bot_action`'s new effect gets **only** error handling plus a slot clear. Do **not** add a `trigger`/`bump_game_update` refetch to it — it has none today and adding one is a behaviour change outside the finding.
- Hydration: `meta_error` is `None` on SSR and on the first hydration pass, so `{move || meta_error.get().map(…)}` emits nothing on both sides. No structural mismatch (`docs/CODING.md:141-148`).
- `class="form-error"`, not `"error"` — `.error` has no CSS rule (see Architecture).
- `error.rs`'s new fn must **not** be `#[cfg(feature = "ssr")]`; it is called from WASM.

**Files:**
- Modify: `rust/web/src/error.rs` (append one fn)
- Modify: `rust/web/src/components/game.rs` (`GameMeta`: add slot, rewrite five effects, three confirm call sites, one render site)

**Steps:**

- [ ] Append to `rust/web/src/error.rs` (after `user_facing_server_error`):

```rust

/// Message to show a user when a server-fn call returns `Err`.
///
/// `ServerFnError`'s `Display` impl prefixes every variant with framework
/// noise - `ServerError(s)` renders as "error running server function: {s}"
/// (server_fn-0.8.13/src/error.rs:233-234) - so `e.to_string()` must never
/// reach the UI. Server fns raise deliberate user-facing rejections with
/// `ServerFnError::new(msg)`, i.e. the `ServerError` variant, and
/// `internal()` above has already replaced genuine infrastructure failures
/// with the opaque "Internal server error" before they get here, so that
/// message is safe to show verbatim. Every other variant is transport or
/// (de)serialization and collapses to the generic text.
///
/// Use this for a failed *write* (a dispatched action). For a failed *read*
/// (a resource that would not load) use `user_facing_server_error`: there
/// the message content is never actionable.
pub fn action_error_message(e: &ServerFnError) -> String {
    match e {
        ServerFnError::ServerError(msg) => msg.clone(),
        // `_` rather than exhaustive arms: `WrappedServerError` is
        // #[deprecated] in server_fn 0.8 and naming it fails -D warnings.
        _ => user_facing_server_error(e),
    }
}
```

- [ ] In `rust/web/src/components/game.rs`, insert the slot immediately after the five `ServerAction` declarations (after the `let force_delete_action = …;` line, currently :53):

```rust

    // Shared error slot for every mutation on this panel (wfe F52). One slot,
    // not one per action: the prefix names the action that failed. Written
    // only from the Effects below, so it is None during SSR and on the first
    // hydration pass - the render site emits nothing on both sides.
    let meta_error = RwSignal::new(None::<String>);
```

- [ ] Replace the whole effect block currently at **:55-85** — from the comment line `    // Trigger re-fetch after undo/concede. Local bump makes the own action` (:55) through the `    });` that closes the force-delete effect (:85), inclusive. Line 86 is blank and must survive; line 54 is blank and must survive. With:

```rust
    // Trigger re-fetch after undo/concede/end-game. Local bump makes the own
    // action refetch even if the WS is down; the trigger bump is still needed
    // for the layout header. On Err the shared slot is filled instead: a
    // failed mutation produces no WS bump, so without this the page simply
    // does not change and the user cannot tell failure from latency.
    Effect::new(move |_| {
        match undo_action.value().get() {
            Some(Ok(())) => {
                meta_error.set(None);
                trigger.set_last_update.update(|n| *n += 1);
                crate::websocket_client::bump_game_update(game_update, game_id);
            }
            Some(Err(e)) => meta_error.set(Some(format!(
                "Undo failed: {}",
                crate::error::action_error_message(&e)
            ))),
            None => {}
        }
    });
    Effect::new(move |_| {
        match concede_action.value().get() {
            Some(Ok(())) => {
                meta_error.set(None);
                trigger.set_last_update.update(|n| *n += 1);
                crate::websocket_client::bump_game_update(game_update, game_id);
            }
            Some(Err(e)) => meta_error.set(Some(format!(
                "Concede failed: {}",
                crate::error::action_error_message(&e)
            ))),
            None => {}
        }
    });
    Effect::new(move |_| {
        match end_game_action.value().get() {
            Some(Ok(())) => {
                meta_error.set(None);
                trigger.set_last_update.update(|n| *n += 1);
                crate::websocket_client::bump_game_update(game_update, game_id);
            }
            Some(Err(e)) => meta_error.set(Some(format!(
                "End game failed: {}",
                crate::error::action_error_message(&e)
            ))),
            None => {}
        }
    });
    // No refetch on success: bumping bot turns has never triggered one and
    // adding it is out of scope. Error reporting only.
    Effect::new(move |_| {
        match bump_bot_action.value().get() {
            Some(Ok(())) => meta_error.set(None),
            Some(Err(e)) => meta_error.set(Some(format!(
                "Bump bot failed: {}",
                crate::error::action_error_message(&e)
            ))),
            None => {}
        }
    });

    // Navigate away after force delete (spec D3); bump the sidebar trigger so
    // the deleted game drops out of the active-games list. On Err stay put and
    // say so - this is the most destructive action on the page.
    let navigate_after_delete = use_navigate();
    Effect::new(move |_| {
        match force_delete_action.value().get() {
            Some(Ok(())) => {
                meta_error.set(None);
                trigger.set_last_update.update(|n| *n += 1);
                navigate_after_delete("/", NavigateOptions::default());
            }
            Some(Err(e)) => meta_error.set(Some(format!(
                "Delete failed: {}",
                crate::error::action_error_message(&e)
            ))),
            None => {}
        }
    });
```

- [ ] Render the slot. Find `<h3>"Actions"</h3>` (currently :110) and insert directly after it:

```rust
                        {move || meta_error.get().map(|e| view! {
                            <div class="form-error">{e}</div>
                        })}
```

- [ ] Convert the concede confirm (the six lines **:126-131**, inside the `can_concede` `<Show>` whose anchor `<a href="#" on:click=move |ev| {` is :124). Replace

```rust
                                    let confirmed = web_sys::window()
                                        .and_then(|w| w.confirm_with_message("Are you sure you want to concede?").ok())
                                        .unwrap_or(false);
                                    if confirmed {
                                        concede_action.dispatch(ConcedeGame { game_id });
                                    }
```

with

```rust
                                    if crate::components::confirm("Are you sure you want to concede?") {
                                        concede_action.dispatch(ConcedeGame { game_id });
                                    }
```

- [ ] Convert the end-game confirm (the six lines **:139-144**) the same way:

```rust
                                    if crate::components::confirm("End this game?") {
                                        end_game_action.dispatch(EndGame { game_id });
                                    }
```

- [ ] Convert the force-delete confirm (the six lines **:191-196**) the same way:

```rust
                                    if crate::components::confirm("Permanently delete this game for all players? This cannot be undone.") {
                                        force_delete_action.dispatch(ForceDeleteGame { game_id });
                                    }
```

- [ ] Confirm no `web_sys::window()` call remains in `GameMeta`: `grep -n "confirm_with_message" rust/web/src/components/game.rs` must return **nothing**. Before this task `grep -n "web_sys" rust/web/src/components/game.rs` returns four hits — the `use web_sys::wasm_bindgen::JsCast;` import (:9), the three `web_sys::window()` confirm calls (:126, :139, :191) — plus `GameCommandInput`'s `dyn_ref::<web_sys::HtmlElement>` (**:525**). Afterwards it must return exactly **two**: :9 and the `dyn_ref` site. The `JsCast` import is still required by that `dyn_ref` (:520-527), so **leave the import**. (Task 11 adds one more `web_sys` hit; run this check before Task 11, or expect three.)

**Verification checkpoint:**

- [ ] `cd /home/beefsack/Development/brdgme/rust && cargo clippy -p web --all-targets --features ssr -- -D warnings` — clean.
- [ ] `cd /home/beefsack/Development/brdgme/rust && cargo fmt --all -- --check` — clean.
- [ ] `cd /home/beefsack/Development/brdgme/rust && cargo test -p web --features ssr` — all pass (the game-page SSR tests `game_page_anonymous_visitor_gets_clean_error_not_panic` (`tests/ssr_pages.rs:323-358`), `game_page_logged_in_player_renders_game` (`:361-401`) and `game_page_player_names_link_to_profiles_for_human_opponents` (`:404-436`) exercise `GameMeta`'s render path; a `view!` mistake shows up there as a panic or a missing marker). **These three are DB-backed `#[sqlx::test]`s** — if the throwaway Postgres is not up, run them under `/home/beefsack/Development/brdgme/scripts/rust-test.sh` rather than reporting a failure (backlog #40).

**Test plan.** Layers, honestly:

- **(b) SSR test — the render path only.** No new test. `game_page_logged_in_player_renders_game` (`tests/ssr_pages.rs:361-401`, marker `"mock render"` at :400) and `game_page_player_names_link_to_profiles_for_human_opponents` (`:404-436`, same marker at :435) already render `GameMeta` server-side and assert a clean 200 with no `panicked at` (`assert_clean_html_body`, `:168-182`, panic check at :178-181). The slot renders nothing on SSR by design, so there is nothing new to assert; these tests are the regression guard for the `view!` edit itself.
- **(a) unit test — not applicable.** `action_error_message`'s `ServerError` arm is trivially assertable but the value it returns is a `String` clone; a test would assert the language of the match arm. **Skip deliberately** — no `#[cfg(test)] mod tests` is added to `error.rs`. (If a reviewer wants one, `assert_eq!(action_error_message(&ServerFnError::new("nope")), "nope")` plus `assert_eq!(action_error_message(&ServerFnError::Request("x".into())), "Something went wrong, please try again")` are the two cases — but they add no coverage of anything that can drift.)
- **(c) Playwright — no.** The e2e spec is already stale (Cross-package #2) and provoking a server error from the browser needs an injected fault.
- **(d) manual checklist — the real coverage.** Deferred to the 32GB+ environment; do NOT claim it from a constrained machine.

  1. `tilt up`; log in; open a game where it is your turn.
  2. Open devtools → Network → set "Offline" (or block `/api/**`).
  3. Sub-menu → "Concede" → OK in the confirm.
     **Expect:** a red line under "Actions" reading `Concede failed: Something went wrong, please try again`. Board unchanged. No console error.
  4. Turn the network back on, click "Concede" → Cancel in the confirm. **Expect:** the red line persists (nothing dispatched) — this is correct; it clears on the next *completed* action.
  5. Concede for real → OK. **Expect:** the red line disappears and the board refetches to the conceded state.
  6. As an admin on a **finished** game whose `can_concede` is false, use the browser console to dispatch a concede against it (or offline-test as in 3) — any server rejection must appear verbatim after `Concede failed: `, e.g. `Concede failed: Game is already finished` (that literal is `game/server_fns.rs:875`; the other reachable ones are `"Not authenticated"` :866, `"Game not found"` :871, `"You are not a player in this game"` :882, `"You have already left this game"` :885), with **no** `error running server function` anywhere in the visible text.
  7. Repeat 3 with **"End game"** → expect `End game failed: …`.
  8. Repeat 3 with **"Delete game (admin)"** → expect `Delete failed: …` and that you are **still on the game page** (no navigation).
  9. Repeat 3 with **"Undo"** → `Undo failed: …`; and with **"Bump bot to play"** (admin, bot's turn) → `Bump bot failed: …`.
  10. Confirm all three dialogs still appear with their original wording, and that Cancel still dispatches nothing.

- [ ] **Commit:** `feat(web): observe GameMeta mutation results and share one error slot` — body notes wfe F52 (major) + wfe F56, that `end_game_action` (#47) is the third destructive action covered, and that `crate::error::action_error_message` is the pattern the rest of WP-54 copies.

---

### Task 2: friends page — shared error slot for all six mutations, plus the missing policy refetch (wd F57, wd F58)

**Problem (restated):** `FriendsPage` owns six `ServerAction`s (`friends.rs:363-368`). Only `add_action`'s error is rendered (:426-428), and it renders `e.to_string()` — so a "no such user" reads as *"error running server function: No user named bob"*. `respond_action` (Accept / Decline / Decline-and-block), `unfriend_action`, `unblock_action`, `policy_action` and `visibility_action` errors go nowhere: a failed Decline leaves the request sitting in the list with no explanation, a failed policy change leaves the `<select>` showing a value that was never saved. Separately, five of the six mutations bump `set_refresh` on success (:373-398) and `policy_action` is the only one that does not (wd F58), so the client's cached `overview` copy of `invite_policy` goes stale after a successful change.

**Fix (re-derived):** one `RwSignal<Option<String>>` for the page, one uniform `match` per action (success → clear + refresh; error → prefixed message), plus the missing `policy_action` refresh effect.

**What the fix deliberately does NOT do, and why — read this before writing code.** Both the wd F57 finding ("re-sync the selects from the refetched overview on failure") and an earlier draft of this spec claimed that bumping `set_refresh` on the *failure* arm re-syncs the two `<select>`s. **That is false, and implementing it would add a wasted round trip plus a comment that lies.** Three verified reasons:

1. A rejected mutation changes nothing server-side, so the refetch returns **identical** data. The `selected` value is a plain `bool` computed eagerly inside the arm (`let selected = o.invite_policy == *slug;`, `friends.rs:559`, `:571`), and `impl AttributeValue for bool::rebuild` returns without touching the DOM when the new value equals the old (`tachys-0.2.18/src/html/attribute/value.rs:554-563`).
2. The enclosing `{move || match overview.get() { … }}` returns `AnyView`s. Taking the **same** arm again means the same `TypeId`, and `AnyView::rebuild` then rebuilds **in place** (`tachys-0.2.18/src/view/any_view.rs:386-400`) — it does not recreate the `<select>`. Likewise `collect_view()`'s `Vec` rebuild reuses the existing `<option>` elements.
3. Even a genuinely *changed* `selected` content attribute would not move a select the user has already clicked: `selected` sets `defaultSelected`, and per HTML the attribute only reassigns selectedness while the option's dirtiness flag is false — user interaction sets it true.

So: **bump `set_refresh` on the success arm only, for all six actions.** The residual (a rejected select change keeps showing the unsaved pick until reload/remount) is assumption **A6** and Cross-package **#7**; the manual checklist below asserts the residual as expected behaviour so it is never mistaken for a regression.

**Edge cases:**
- Six actions, six effects, one shared slot. Keep them in the existing declaration order so the diff reads cleanly.
- `add_action`'s existing inline error render (:426-428) is **removed**; its message moves into the shared slot. Do not leave two error surfaces.
- `add_action` keeps its extra success work: `set_add_name.set(String::new())` (currently :375) must survive into the new effect.
- The page-level load error at `:406` (`Some(Err(e)) => … "Error: " {e.to_string()}`) is a **read** failure and also leaks the prefix. It is in scope as the same defect in the same view (see Cross-package #3 for why it is folded in rather than deferred): switch it to `user_facing_server_error`. Keep its `class="error"` and its `<p>` element type — restyling is out of scope and the element type must not change.
- Slot placement: **outside** the `overview.get()` match, directly after `<h1>"Friends"</h1>` (:403), so it survives a refetch and is visible regardless of which section's action failed. Do not put it inside the `Some(Ok(o))` arm.
- Hydration: `mutation_error` is `None` on SSR and on the first hydration pass, so the added `{move || … .map(…)}` emits nothing on both sides.

**Files:**
- Modify: `rust/web/src/friends.rs` (`FriendsPage` only — lines 353-581; **do not touch the server fns above :351**)

**Steps:**

- [ ] After the six action declarations (which end at `friends.rs:368`) and the `add_name` signal (:370), insert — i.e. between :370 and the comment currently on :372:

```rust

    // One error slot for every mutation on this page (wd F57). The prefix
    // names the action that failed. Written only from the Effects below, so
    // it is None during SSR and on the first hydration pass.
    let mutation_error = RwSignal::new(None::<String>);
```

- [ ] Replace the whole effect block currently at **:372-398** — from the comment line `    // Any successful mutation refetches the overview.` (:372) through the `    });` that closes the `visibility_action` effect (:398), inclusive. Line 371 is blank and must survive; line 399 is blank and must survive. With:

```rust
    // Every mutation refetches the overview on success, and reports failure
    // into the shared slot instead of silently doing nothing (wd F57). No
    // refetch on the failure arm: a rejected mutation returns identical data,
    // so tachys writes nothing to the DOM and it would re-sync nothing (see
    // the Task 2 preamble in the spec) - it would just be a wasted request.
    Effect::new(move |_| {
        match add_action.value().get() {
            Some(Ok(())) => {
                mutation_error.set(None);
                set_add_name.set(String::new());
                set_refresh.update(|n| *n += 1);
            }
            Some(Err(e)) => mutation_error.set(Some(crate::error::action_error_message(&e))),
            None => {}
        }
    });
    Effect::new(move |_| {
        match respond_action.value().get() {
            Some(Ok(())) => {
                mutation_error.set(None);
                set_refresh.update(|n| *n += 1);
            }
            Some(Err(e)) => mutation_error.set(Some(format!(
                "Could not respond to that request: {}",
                crate::error::action_error_message(&e)
            ))),
            None => {}
        }
    });
    Effect::new(move |_| {
        match unfriend_action.value().get() {
            Some(Ok(())) => {
                mutation_error.set(None);
                set_refresh.update(|n| *n += 1);
            }
            Some(Err(e)) => mutation_error.set(Some(format!(
                "Unfriend failed: {}",
                crate::error::action_error_message(&e)
            ))),
            None => {}
        }
    });
    Effect::new(move |_| {
        match unblock_action.value().get() {
            Some(Ok(())) => {
                mutation_error.set(None);
                set_refresh.update(|n| *n += 1);
            }
            Some(Err(e)) => mutation_error.set(Some(format!(
                "Unblock failed: {}",
                crate::error::action_error_message(&e)
            ))),
            None => {}
        }
    });
    // wd F58: this effect was missing entirely, so after a successful policy
    // change the client's cached `overview.invite_policy` stayed stale -
    // unlike all five siblings, which refetch.
    Effect::new(move |_| {
        match policy_action.value().get() {
            Some(Ok(())) => {
                mutation_error.set(None);
                set_refresh.update(|n| *n += 1);
            }
            // No refetch here: see the comment at the top of this block. The
            // select keeps showing the rejected pick until the page reloads
            // or this component remounts (spec assumption A6 /
            // cross-package #7) - that is a known, recorded residual.
            Some(Err(e)) => mutation_error.set(Some(format!(
                "Could not save the invite policy: {}",
                crate::error::action_error_message(&e)
            ))),
            None => {}
        }
    });
    Effect::new(move |_| {
        match visibility_action.value().get() {
            Some(Ok(())) => {
                mutation_error.set(None);
                set_refresh.update(|n| *n += 1);
            }
            Some(Err(e)) => mutation_error.set(Some(format!(
                "Could not save the visibility setting: {}",
                crate::error::action_error_message(&e)
            ))),
            None => {}
        }
    });
```

- [ ] Render the slot. Replace `<h1>"Friends"</h1>` (currently **:403**; it is the only occurrence in the file) with:

```rust
                <h1>"Friends"</h1>
                {move || mutation_error.get().map(|e| view! {
                    <div class="form-error">{e}</div>
                })}
```

- [ ] Fix the page load error (currently :406). Replace

```rust
                    Some(Err(e)) => view! { <p class="error">"Error: " {e.to_string()}</p> }.into_any(),
```

with

```rust
                    Some(Err(e)) => view! {
                        <p class="error">{crate::error::user_facing_server_error(&e)}</p>
                    }.into_any(),
```

- [ ] Delete `add_action`'s now-duplicate inline error render (currently :426-428):

```rust
                            {move || add_action.value().get().and_then(|r| r.err()).map(|e| view! {
                                <p class="error">{e.to_string()}</p>
                            })}
```

- [ ] `grep -n "to_string()" rust/web/src/friends.rs` must return **nothing at all**. Before this task it returns exactly two hits, both in `FriendsPage` (:406 and :427) and both removed above; there are no `to_string()` calls in the server-fn region. (An earlier draft of this spec said "the remaining hits must be inside `#[server]` bodies" — there are none.)

**Verification checkpoint:**

- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean; `cargo fmt --all -- --check` clean.
- [ ] `cargo test -p web --features ssr` — all pass.

**Test plan.**

- **(b) SSR test — thin but real.** No new test needed; the friends page has no existing `ssr_pages.rs` coverage and adding a full one is out of scope. **What you must do instead:** confirm no regression by hand-running the router once — there is no cheap SSR assertion here because both new surfaces (the slot and the refetch) are `None`/inert on the server. Explicitly: **no SSR test for this task**, and that is a limitation, not an oversight.
- **(d) manual checklist** (32GB+ environment; deferred):
  1. Log in as user A, `/friends`. Send a request to a nonexistent username `zzz-nobody`.
     **Expect:** a red line under the "Friends" heading reading exactly `No user named zzz-nobody` (the literal is `friends.rs:167`) — **not** `error running server function: No user named zzz-nobody`.
  2. Send a request to yourself. **Expect:** `You cannot friend yourself` (`friends.rs:171`).
  3. As user B with an incoming request from A: go offline, click **Decline**.
     **Expect:** `Could not respond to that request: Something went wrong, please try again`; the request stays in the list. Go online, Decline again → the red line clears and the row disappears.
  4. Offline, click **Decline and block** → OK. Expect the same prefixed message.
  5. Offline, **Unfriend** an existing friend → OK. **Expect:** `Unfriend failed: …`, friend still listed.
  6. Offline, **Unblock** a blocked user. **Expect:** `Unblock failed: …`.
  7. Offline, change **"Who can invite me to games"** from `open` to `none`.
     **Expect:** `Could not save the invite policy: Something went wrong, please try again`. **Expect ALSO — and this is intentional, not a bug:** the select **keeps showing `none`**. It only returns to `open` on a page reload or after navigating away and back. This is spec assumption **A6** / Cross-package **#7**. Do **not** "fix" it here; do not add a failure-arm `set_refresh` bump to try to fix it (that has been verified not to work — see the Task 2 preamble).
  8. Reload `/friends`. **Expect:** the select shows `open` again (the server never saved `none`).
  9. Online, change the invite policy to `none`. **Expect:** no error line, and reloading the page shows `none` (wd F58: before this task the client's cached overview stayed stale until some other action refetched).
  10. Repeat 7-9 for **"Who can see my games"** (`visibility_action`) — it already refetched on success, so 9 is a no-regression check and 7 is the same recorded residual.
  11. Kill the DB (or block `/api/get_friends_overview`) and reload `/friends`. **Expect:** `Something went wrong, please try again` — no `error running server function`.

- [ ] **Commit:** `fix(web): surface all friends-page mutation errors and refetch after policy change` — body cites wd F57, wd F58, and that it copies Task 1's slot pattern.

---

### Task 3: settings page — revert optimistic state and report errors (wd F73)

**Problem (restated):** three sections write local signals *before* dispatching and never read the result.

- `ColorsSection` (`settings.rs:112-177`): `pick` (:136-148) mutates `colors` (:137-144) then dispatches `SetPrefColors` (:145-147). `save_action` (:135) — its `value()` is never read.
- `EmailPreferencesSection` (:179-240): each checkbox does `turn.set(val)` (:212) / `invite.set(val)` (:223) / `reminder.set(val)` (:234) and then dispatches (:213, :224, :235). None of the three action values (declared :200-202) is read.
- `ThemeSection` (:471-572): `select()` (:488-499) calls `set_theme_client(slug)` (:494) then `current_theme.set(slug)` (:495) then dispatches `SetTheme` (:497). `set_theme_action` (:475) — value never read.

So on a session-expiry or transient 500 the page shows a saved state that was never persisted, silently, with no revert.

**Fix (re-derived), and where it diverges from the finding:** Colors and email-prefs get a `StoredValue` snapshot taken at dispatch time, an `Effect` that restores it on `Err`, and a section-level error slot. **`ThemeSection` does not revert** — `set_theme_client` (`app.rs:246-272`) has already written `document.documentElement[data-theme]` (`:255-264`) *and* a `theme=<slug>; path=/; max-age=31536000` cookie (`:265-271`, the literal at :267), so the choice is genuinely applied and will survive a reload via `THEME_BOOT_SCRIPT` (`app.rs:21`). Only the account-level sync failed (and it is only dispatched at all when logged in — `settings.rs:496`). Reverting the visible theme would undo something that worked; the honest signal is an advisory line (assumption **A5**).

**Why the revert works here but not on the friends selects (Task 2):** both controls read their value from a **signal** through `prop:checked=move || turn.get()` (`settings.rs:209`, :220, :231) and `prop:value=move || colors.get().get(i)…` (`:160`). Writing a *different* value into that signal re-runs the property's `RenderEffect` on an element that already exists and already has its children, so the DOM follows. The friends selects instead compute a static `bool` for a per-`<option>` `selected=` attribute, which is why the same trick does not work there.

This does **not** contradict `docs/CODING.md:297-303` ("Fields that are just a choice among valid options … save immediately on change, fire-and-forget — no loading state, no page-wide 'unsaved changes' banner"). No loading state and no dirty banner are added. That rule governs the *save trigger*, not whether failures are reported.

**Edge cases:**
- Concurrent dispatches: `ServerAction::value()` holds only the latest result, and `reactive_graph 0.2.14`'s out-of-order guard is dead code (see Cross-package #5, `reactive_graph-0.2.14/src/actions/action.rs:269` snapshot / `:288` `is_latest` check, with `dispatched` never incremented). So a rapid double-click can revert to the snapshot from the *second* click rather than the first. This is acceptable for a three-colour picker and three checkboxes, and reproducing it needs sub-100ms clicking against a failing server. **Do not build a request-sequence mechanism** — out of scope.
- Each of `EmailPreferencesSection`'s three toggles needs its **own** snapshot (they are independent booleans) but shares one error slot.
- The `initialized` latch effects (`settings.rs:126-133`, `:189-198`) are untouched: they adopt the stored prefs once and are not implicated. In particular, do **not** reuse `initialized` for the snapshots.
- `StoredValue`, `RwSignal`, `Effect` and `event_target_checked` are all already in scope via `use leptos::prelude::*;` (`settings.rs:5`) — `event_target_checked` is already called at :211/:222/:233. No new imports.
- The four `EmailSection` effects at :276-336 each do `error.set(Some(e.to_string()))` (**:285, :301, :316, :331** — verified exact) — same `Display`-prefix leak, in a section already wired for errors. **In scope as a rider** (same file, same class, and the helper exists precisely for it); switch all four to `action_error_message`. See Cross-package #3.
- Hydration: all three new slot signals are `None` on SSR and first hydrate.

**Files:**
- Modify: `rust/web/src/settings.rs` (`ColorsSection`, `EmailPreferencesSection`, `ThemeSection`, plus the four `EmailSection` message lines)

**Steps:**

- [ ] `ColorsSection`: replace **:135-148** — from `    let save_action = ServerAction::<crate::auth::SetPrefColors>::new();` (:135) through the `    };` that closes `pick` (:148), inclusive — with:

```rust
    let save_action = ServerAction::<crate::auth::SetPrefColors>::new();
    // The UI updates optimistically, so a rejected save has to be undone
    // (wd F73). Snapshot is non-reactive: nothing should re-render on it.
    let error = RwSignal::new(None::<String>);
    let before_pick = StoredValue::new(Vec::<String>::new());
    Effect::new(move |_| {
        match save_action.value().get() {
            Some(Ok(())) => error.set(None),
            Some(Err(e)) => {
                colors.set(before_pick.get_value());
                error.set(Some(format!(
                    "Could not save your colours: {}",
                    crate::error::action_error_message(&e)
                )));
            }
            None => {}
        }
    });
    let pick = move |i: usize, val: String| {
        before_pick.set_value(colors.get_untracked());
        colors.update(|c| {
            if let Some(j) = c.iter().position(|x| *x == val)
                && j != i
            {
                c[j] = c[i].clone();
            }
            c[i] = val;
        });
        save_action.dispatch(crate::auth::SetPrefColors {
            colors: colors.get_untracked(),
        });
    };
```

- [ ] `ColorsSection` render: replace `<h2>"Preferred colours"</h2>` (currently :151, the only occurrence in the file) with:

```rust
        <h2>"Preferred colours"</h2>
        {move || error.get().map(|e| view! { <div class="form-error">{e}</div> })}
```

- [ ] `EmailPreferencesSection`: replace the three action declarations (currently **:200-202**) with:

```rust
    let turn_action = ServerAction::<crate::auth::SetEmailTurnEnabled>::new();
    let invite_action = ServerAction::<crate::auth::SetEmailInviteEnabled>::new();
    let reminder_action = ServerAction::<crate::auth::SetEmailReminderEnabled>::new();

    // Each toggle flips its signal before dispatching, so a rejected save has
    // to be undone (wd F73). One shared error slot, one snapshot per toggle.
    let error = RwSignal::new(None::<String>);
    let before_turn = StoredValue::new(true);
    let before_invite = StoredValue::new(true);
    let before_reminder = StoredValue::new(true);
    Effect::new(move |_| {
        match turn_action.value().get() {
            Some(Ok(())) => error.set(None),
            Some(Err(e)) => {
                turn.set(before_turn.get_value());
                error.set(Some(format!(
                    "Could not save turn notifications: {}",
                    crate::error::action_error_message(&e)
                )));
            }
            None => {}
        }
    });
    Effect::new(move |_| {
        match invite_action.value().get() {
            Some(Ok(())) => error.set(None),
            Some(Err(e)) => {
                invite.set(before_invite.get_value());
                error.set(Some(format!(
                    "Could not save invite notifications: {}",
                    crate::error::action_error_message(&e)
                )));
            }
            None => {}
        }
    });
    Effect::new(move |_| {
        match reminder_action.value().get() {
            Some(Ok(())) => error.set(None),
            Some(Err(e)) => {
                reminder.set(before_reminder.get_value());
                error.set(Some(format!(
                    "Could not save reminder notifications: {}",
                    crate::error::action_error_message(&e)
                )));
            }
            None => {}
        }
    });
```

- [ ] `EmailPreferencesSection` render: replace `<h2>"Email notifications"</h2>` (currently :205) with:

```rust
        <h2>"Email notifications"</h2>
        {move || error.get().map(|e| view! { <div class="form-error">{e}</div> })}
```

  and add the snapshot line to each of the three `on:change` handlers — for the turn checkbox (currently :210-214), replace

```rust
                on:change=move |ev| {
                    let val = event_target_checked(&ev);
                    turn.set(val);
                    turn_action.dispatch(crate::auth::SetEmailTurnEnabled { enabled: val });
                }
```

  with

```rust
                on:change=move |ev| {
                    let val = event_target_checked(&ev);
                    before_turn.set_value(turn.get_untracked());
                    turn.set(val);
                    turn_action.dispatch(crate::auth::SetEmailTurnEnabled { enabled: val });
                }
```

  and identically for invite (`before_invite` / `invite`, currently :221-225) and reminder (`before_reminder` / `reminder`, currently :232-236).

- [ ] `ThemeSection`: after `    let set_theme_action = ServerAction::<crate::auth::SetTheme>::new();` (currently **:475**) insert:

```rust

    // Profile sync only: `set_theme_client` in `select` below has already
    // written <html data-theme> and the year-long `theme` cookie (app.rs
    // :255-271), so the choice IS applied and IS persisted on this device.
    // Do not revert it - only the account-level sync failed (wd F73,
    // adjusted). The generic message is used deliberately: the server text
    // adds nothing a user can act on for a background profile write.
    let sync_error = RwSignal::new(None::<String>);
    Effect::new(move |_| {
        match set_theme_action.value().get() {
            Some(Ok(())) => sync_error.set(None),
            Some(Err(_)) => sync_error.set(Some(
                "Theme applied on this device, but saving it to your profile failed.".to_string(),
            )),
            None => {}
        }
    });
```

- [ ] `ThemeSection` render: replace `<h2>"Theme"</h2>` (currently :532) with:

```rust
        <h2>"Theme"</h2>
        {move || sync_error.get().map(|e| view! { <div class="form-error">{e}</div> })}
```

- [ ] `EmailSection` rider: replace **all four** occurrences of `error.set(Some(e.to_string()));` (currently :285, :301, :316, :331 — all four inside `Err(e) => { … }` arms of the four action effects at :276-290, :291-306, :307-321, :322-336) with

```rust
                    error.set(Some(crate::error::action_error_message(&e)));
```

  (`grep -c "action_error_message" rust/web/src/settings.rs` must be **exactly 8** afterwards: 1 in `ColorsSection` + 3 in `EmailPreferencesSection` + 4 in `EmailSection`. `ThemeSection` uses a hard-coded advisory string and contributes **0**. An earlier draft of this spec said "exactly 10 … + 2 in nothing else", which is arithmetically incoherent — 8 is the number. Recount with `grep -n` and confirm each site is one of the eight. `grep -n "e.to_string()" rust/web/src/settings.rs` must return nothing; before this task it returns exactly :285, :301, :316, :331.)

- [ ] Confirm `settings.rs:1-2`'s "email placeholder" module doc is **unchanged** (Non-Goals).

**Verification checkpoint:**

- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean; `cargo fmt --all -- --check` clean.
- [ ] `cargo test -p web --features ssr` — all pass.

**Test plan.**

- **(b) SSR test — no.** All three sections' new surfaces are `None` server-side, and `/settings` has no existing SSR test to strengthen. No new SSR test; stated as a gap.
- **(d) manual checklist** (deferred):
  1. Log in, `/settings`. Note the three colour selects, e.g. `Green / Red / Blue`.
  2. Go offline. Change "1st choice" to `Yellow`.
     **Expect:** the select **snaps back to `Green`** and a red line under "Preferred colours" reads `Could not save your colours: Something went wrong, please try again`.
  3. Go online, change "1st choice" to `Yellow`. **Expect:** the red line clears; the swap holds; reload shows `Yellow` first.
  4. Offline. Untick "Turn notifications".
     **Expect:** the checkbox **re-ticks itself** and `Could not save turn notifications: Something went wrong, please try again` appears under "Email notifications".
  5. Repeat 4 for "Invite notifications" and "Reminder notifications", checking the message names the right one.
  6. Offline. Click a theme tile.
     **Expect:** the theme **does change** (this is correct), the tile shows `.selected`, and a red line under "Theme" reads `Theme applied on this device, but saving it to your profile failed.` Reload the page: the theme is still applied (cookie). Log in on another browser: the theme is **not** synced — matching what the message said.
  7. Online, click another theme tile. **Expect:** the advisory line clears.
  8. Offline, "Add email address" with a valid address. **Expect:** the existing `form-error` line reads `Something went wrong, please try again` — **no** `error running server function`.
  9. Online, add an address that already belongs to another account. **Expect:** the server's own message verbatim, prefix-free.

- [ ] **Commit:** `fix(web): revert optimistic settings state and report save failures` — body cites wd F73 and records that `ThemeSection` deliberately does not revert (the cookie write already succeeded).

---

### Task 4: logout failure feedback (wfe F59)

**Problem (restated):** `SidebarMenu`'s effect (`layout.rs:120-124`) navigates to `/login` only when `logout_action.value()` is `Ok`. An `Err` leaves the user apparently logged in, on the same page, with no signal. The sidebar is on every page, so this is the most broadly visible fire-and-forget in the app.

**Fix (re-derived):** match both arms; on `Err` fill a slot rendered inside the logged-in block, right next to the "logout" link so it is adjacent to what the user clicked.

**Edge cases:**
- The slot lives inside the `<div hidden=move || !logged_in()>` block (`layout.rs:154-175`), specifically inside its `{move || { … view! { … } }}` content closure (:155-174). That closure renders unconditionally on SSR (only `hidden` toggles — the comment at :151-153 says so), so the slot render site must be a `{move || … .map(…)}` returning `None` on SSR — it is. Adding a fifth node to that fragment changes the fragment's tuple type but not the SSR/hydrate *structure*, because the added node emits nothing on both sides.
- Do **not** clear the slot on success: on success the effect navigates away and `SidebarMenu` is remounted, so there is nothing to clear. Setting it to `None` first is harmless; include it for uniformity with Task 1.
- `logout_action` is `ServerAction::<crate::auth::Logout>` created in `App` (`app.rs:126`) and provided via context (`:127`). Its value type is `Result<(), ServerFnError>` — `is_ok()` today; match on it now.
- `RwSignal` is in scope in `layout.rs` via `use leptos::prelude::*;` (:3). No new imports.

**Files:**
- Modify: `rust/web/src/components/layout.rs` (`SidebarMenu` only)

**Steps:**

- [ ] Replace `SidebarMenu`'s opening statements — **:118-124** (the fn signature on :117 is untouched):

```rust
    let logout_action = expect_context::<ServerAction<crate::auth::Logout>>();
    let navigate = use_navigate();
    Effect::new(move |_| {
        if logout_action.value().get().is_some_and(|r| r.is_ok()) {
            navigate("/login", NavigateOptions::default());
        }
    });
```

  with:

```rust
    let logout_action = expect_context::<ServerAction<crate::auth::Logout>>();
    let navigate = use_navigate();
    // wfe F59: a failed logout used to leave the user apparently signed in
    // with no signal at all. The sidebar is on every page, so this slot is
    // the app-wide one.
    let logout_error = RwSignal::new(None::<String>);
    Effect::new(move |_| {
        match logout_action.value().get() {
            Some(Ok(())) => {
                logout_error.set(None);
                navigate("/login", NavigateOptions::default());
            }
            Some(Err(e)) => logout_error.set(Some(format!(
                "Logout failed: {}",
                crate::error::action_error_message(&e)
            ))),
            None => {}
        }
    });
```

- [ ] Render the slot. Inside the logged-in block's inner `view!` (currently **:163-173**), immediately after the `")"` text node on :172 and before the `}` that closes that `view!` on :173, add a sibling node. Replace

```rust
                            <a
                                on:click=move |_| {
                                    logout_action.dispatch(crate::auth::Logout {});
                                }
                                style="cursor:pointer"
                            >"logout"</a>
                            ")"
                        }
```

  with

```rust
                            <a
                                on:click=move |_| {
                                    logout_action.dispatch(crate::auth::Logout {});
                                }
                                style="cursor:pointer"
                            >"logout"</a>
                            ")"
                            {move || logout_error.get().map(|e| view! {
                                <div class="form-error">{e}</div>
                            })}
                        }
```

  (Task 10 rewrites this same anchor for a11y — that is why Task 4 lands first and Task 10 second.)

**Verification checkpoint:**

- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean; `cargo fmt --all -- --check` clean.
- [ ] `cargo test -p web --features ssr` — all pass, including `layout.rs`'s two inline unit tests (`next_game_id_picks_longest_waiting_my_turn_game` :301-309 and `next_game_id_none_when_no_game_is_my_turn` :311-315, inside `mod tests` :281-316) which must be untouched.

**Test plan.**

- **(b) SSR test — no new test.** Every existing `ssr_pages.rs` page test renders `SidebarMenu`; a `view!` error surfaces there. The slot itself is invisible on SSR.
- **(d) manual checklist** (deferred):
  1. Log in. Open any page. Go offline.
  2. Click "(logout)".
     **Expect:** a red line under the username row reading `Logout failed: Something went wrong, please try again`. You remain on the page and remain logged in (sidebar still shows your name).
  3. Go online. Click "(logout)". **Expect:** navigation to `/login`, sidebar shows "Login".
  4. Confirm no console error in either case.

- [ ] **Commit:** `fix(web): report logout failures in the sidebar` — cites wfe F59.

---

### Task 5: `GamePage` shows a generic load error, not raw `ServerFnError` (wfe F55)

**Problem (restated):** `app.rs:762` renders `"Error: " {e.to_string()}` for any `get_game_details` failure. Because `game_data` is `Resource::new_blocking` (`app.rs:700-708`) inside a `<Transition>` (`:756-758`), and `<Transition>` renders children directly during SSR (`docs/CODING.md:135-137`), the string reaches the **server-rendered HTML**: an anonymous visitor to `/games/<id>` currently gets `Error: error running server function: Not authenticated` in the page source. `GameCommandInput` in the same feature explicitly documents the opposite policy (`components/game.rs:588`: *"Transport/server fault: never leak the raw ServerFnError text"*).

**Fix (re-derived):** `user_facing_server_error`, not `action_error_message` — this is a failed **read**, and `get_game_details`' rejections (`"Not authenticated"` `game/server_fns.rs:251`, `"Game not found"` `:256`) are not actionable in a way that a distinct message would help with. The only case worth distinguishing is the locally-generated `"Invalid Game ID"` (`app.rs:705`, and again in the `logs` resource at `:722`), which means a malformed URL — but that is generated client-side from the same `ServerFnError::new`, so distinguishing it would mean string-matching the message. **Do not do that**; one message.

**Edge cases:**
- Keeps `class="error"` and the same `<div>` element type, so SSR/hydrate structure is unchanged (only the text differs, which is a text-node change inside the same element — and both sides produce the *same* text, since both run the same branch). Note the replacement drops the separate `"Error: "` text node and emits a single literal, which is fine: the `<div>` remains one element with one text child on both sides.
- Because the arm binds nothing now, it must be written `Err(_) =>`, not `Err(e) =>` — an unused `e` trips `-D warnings`.
- `"Failed to load this game."` — a period, sentence case, matching `GameLogs`' `"Failed to load logs."` (`components/game.rs:406`) rather than `GameCommandInput`'s imperative style.

**Files:**
- Modify: `rust/web/src/app.rs` (one line inside `GamePage`)
- Modify: `rust/web/tests/ssr_pages.rs` (strengthen one existing test)

**Steps:**

- [ ] In `GamePage`, replace (currently `app.rs:762`)

```rust
                        Err(e) => view! { <div class="error">"Error: " {e.to_string()}</div> }.into_any(),
```

  with

```rust
                        // Never leak the raw ServerFnError text - its Display
                        // impl prefixes "error running server function: " and
                        // this branch renders into the SSR HTML (wfe F55).
                        Err(_) => view! {
                            <div class="error">"Failed to load this game."</div>
                        }.into_any(),
```

- [ ] Strengthen `game_page_anonymous_visitor_gets_clean_error_not_panic` (`rust/web/tests/ssr_pages.rs:323-358`; `#[sqlx::test]` attribute on :322). After the existing `assert!(!body.to_lowercase().contains("panicked at"), "body: {body}");` on **:357** (the last statement in the test), add:

```rust
    // wfe F55: the Err branch of GamePage renders into the SSR HTML (blocking
    // resource inside a Transition), so `e.to_string()`'s framework prefix
    // used to ship to the browser. Guard both directions.
    assert!(
        !body.contains("error running server function"),
        "raw ServerFnError Display text reached the SSR HTML: {body}"
    );
    assert!(
        body.contains("Failed to load this game."),
        "expected the generic game-load error in the SSR body: {body}"
    );
```

  **Both assertions are mandatory. Prove the premise first, then make the change** — this replaces the "if it fails, delete the assertion" hedge an earlier draft carried, which would have let the implementer silently discard the only automated coverage in the package:

  1. **Before** editing `app.rs`, add *only* the first assertion inverted, as a throwaway reproduction check:
     `assert!(body.contains("error running server function"), "wfe F55 premise: {body}");`
     Run `cargo test -p web --features ssr game_page_anonymous_visitor_gets_clean_error_not_panic`. It **must pass** — that is wfe F55 reproduced: the blocking resource's `Err` branch really is inlined into the SSR HTML for this route.
  2. Delete the throwaway line, make the `app.rs` change, add both real assertions, re-run. Both must pass.
  3. **If step 1 fails**, the wfe F55 premise (`Transition` + `Resource::new_blocking` puts this text in the initial HTML) does not hold in this Leptos build. **STOP and report to the Lead.** Do not proceed by weakening the test: the `app.rs` change is still correct and should still land, but the disposition-table justification for wfe F55 would need rewriting and only the Lead can make that call.

  **Do not weaken or delete the first assertion under any circumstances** — it is a valid guard either way, since the serialized resource payload carries the JSON-encoded `ServerFnError`, never its `Display` form.

**Verification checkpoint:**

- [ ] `cargo test -p web --features ssr game_page_anonymous_visitor_gets_clean_error_not_panic` — passes (this is the DB-backed layer; if the throwaway Postgres is not up, run it under `/home/beefsack/Development/brdgme/scripts/rust-test.sh` instead of reporting a failure — backlog #40).
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean; `cargo fmt --all -- --check` clean.

**Test plan.**

- **(b) SSR test — this is the right layer and it is written above.** The anonymous-visitor path deterministically produces the `Err` branch server-side, which makes wfe F55 the one item in this package with genuine automated coverage.
- **(d) manual checklist** (deferred, 2 steps):
  1. Logged out, open `/games/<a real game id>`. **Expect:** `Failed to load this game.` and nothing resembling `error running server function`. View source and confirm the same.
  2. Open `/games/not-a-uuid`. **Expect:** the same message (the route matches, `Uuid::from_str` fails, `"Invalid Game ID"` is generated locally and collapsed).

- [ ] **Commit:** `fix(web): generic game-load error instead of raw ServerFnError` — cites wfe F55 and `server_fn-0.8.13/src/error.rs:233-234`.

---

### Task 6: re-arm the presence-ping and profile-theme latches after logout (wfe F54)

**Problem (restated):** `App` has two one-shot `RwSignal<bool>` latches.

> **Line numbers here were wrong in an earlier draft (five lines low across the board). These are verified against live `app.rs` on 2026-07-25, and they agree with the wfe F54 finding's own anchors.**

`applied_profile_theme` is declared at **`app.rs:154`** (doc comment :145-153), guarded at :156-157 and set at :158: fetches the logged-in user's stored theme once per page load; its `spawn_local` body is :159-171 and the `Effect` closes at :173. `presence_started` is declared at **`app.rs:179`** (comment :175-178), guarded at :181 and set at :182: spawns the 5-minute presence ping loop (:183-191), which breaks itself when `current_user` is no longer logged in (:185-187); that `Effect` closes at :193.

Both effects **do** re-run when `current_user` changes — they call `current_user.get()` at :156 and :181 — so the latch is the only thing preventing a restart. And a re-login in the same tab genuinely does re-resolve `current_user`: `LoginPage`'s confirm effect calls `current_user.refetch()` then client-side-navigates to `/` (`app.rs:511-517`), with no page load. After a logout→login in the same tab: the ping loop has exited and never respawns (the re-authenticated session reports no presence until a full reload), and a second user's stored theme is never applied.

**Fix (re-derived):** key each latch on the user id it did its work for, **and** clear it on an observed logged-out state. Id-keying alone is insufficient: logging out and back in as the *same* user leaves the latch equal to that id, so the ping loop would still not respawn. Clearing on logout alone is also insufficient if the resource transitions user-A→user-B without an intervening `Ok(None)`. Do both.

**Edge cases:**
- `current_user.get()` returns `Option<Result<Option<AuthUser>, ServerFnError>>`, and its `T` is `Clone`, so `Some(Ok(Some(user)))` binds an **owned** `AuthUser` — `user.id` is a plain `Copy` `Uuid` read. The outer `None` (the resource has never resolved; it tracks `logout_action.version()`, `app.rs:138-142`) and the `Some(Err(_))` case must both be treated as "no information": leave the latch alone. Only `Some(Ok(None))` — a *resolved* anonymous state — clears it. **Note:** because resources are stale-while-revalidate (see Architecture), a refetch does **not** transiently produce `None`; the old value stays until the new one lands. That only makes the `None` arm rarer, not wrong.
- The latch is read with `get_untracked()` and written with `set`; the effect does not subscribe to it, so there is no self-triggering loop. Guard the clear with an `is_some()` check anyway, so a page that is anonymous throughout does not enqueue a notification on every `current_user` change.
- Effects are inert during SSR (`docs/hydration.md:80-104`), so nothing here affects rendered HTML.
- The ping loop's `spawn_local` closure captures nothing new; its own break condition (`:185-187`) already handles the logout that re-arms the latch, so there is no risk of two loops running: the old loop exits at its next `current_user.get_untracked()` check, and the new one only starts after a subsequent login.
  **One tolerated overlap:** the old loop may still be sleeping in `TimeoutFuture::new(PRESENCE_PING_INTERVAL_MS)` when a re-login starts a second loop, so for up to 5 minutes two loops can both be alive; the old one then hits its break and exits. `ping_active()` is idempotent (it stamps a "last active" time), so a duplicate ping in that window is harmless. Do not add a cancellation token for it.

**Files:**
- Modify: `rust/web/src/app.rs` (`App` — the two latch blocks only)

**Steps:**

- [ ] Replace the profile-theme block — **`app.rs:145-173`**, from the comment line `    // Profile theme sync: once the current user resolves to logged-in for` (:145) through the `    });` that closes the `Effect` (:173), inclusive. **Do NOT start at :141**: :138-142 are the `current_user` `LocalResource` and :143 is `provide_context(current_user);` — an earlier draft of this spec said `:141-172`, which deletes part of that resource and `provide_context`, and orphans the `});` on :173. The build would fail. Line 144 (blank) and line 174 (blank) must both survive. Replacement:

```rust
    // Profile theme sync: once the current user resolves to logged-in, fetch
    // their stored theme (if any) and apply it - the profile wins over
    // whatever was showing pre-login (system default or a locally-set-but-
    // unsaved theme). If the profile has no stored preference, instead push
    // the local choice (if any) up to the profile, so the local choice syncs
    // to the account and follows the user to new devices. No-ops for
    // anonymous visitors. Runs only on hydrate (Effects are inert during
    // SSR), so `set_theme_client`'s/`web_sys` calls are safe here.
    //
    // wfe F54: the latch holds the user id it ran for, and is cleared on a
    // resolved logged-out state, so logging out and back in - as the same
    // user or a different one - re-runs the sync. A plain bool never reset,
    // so a second user in the same tab never got their stored theme.
    let applied_profile_theme = RwSignal::new(None::<Uuid>);
    Effect::new(move |_| {
        match current_user.get() {
            // Resolved anonymous: re-arm. Guarded so an always-anonymous
            // page does not notify on every resource change.
            Some(Ok(None)) => {
                if applied_profile_theme.get_untracked().is_some() {
                    applied_profile_theme.set(None);
                }
            }
            Some(Ok(Some(user))) => {
                if applied_profile_theme.get_untracked() == Some(user.id) {
                    return;
                }
                applied_profile_theme.set(Some(user.id));
                leptos::task::spawn_local(async move {
                    match crate::auth::get_user_theme().await {
                        Ok(Some(theme)) => set_theme_client(Some(&theme)),
                        Ok(None) => {
                            if let Some(local) = local_data_theme()
                                && crate::theme::is_known_slug(&local)
                            {
                                let _ = crate::auth::set_theme(Some(local)).await;
                            }
                        }
                        Err(_) => {}
                    }
                });
            }
            // Still loading, or the fetch failed: no information, leave the
            // latch as it is.
            None | Some(Err(_)) => {}
        }
    });
```

- [ ] Replace the presence-ping block — **`app.rs:175-193`**, from the comment line `    // Presence ping: while logged in with any page open, tell the server we're` (:175) through the `    });` that closes the `Effect` (:193), inclusive. Line 174 is the blank line separating it from the previous block and must survive (do not start at :174). Replacement:

```rust
    // Presence ping: while logged in with any page open, tell the server we're
    // active every 5 min. No Page Visibility gating - an open page counts.
    // Runs only on hydrate (Effects are inert during SSR). The loop breaks once
    // the user is no longer logged in, so it can't outlive the session.
    //
    // wfe F54: same id-keyed latch as the theme sync above. With a plain bool
    // the loop broke on logout and never respawned, so a logout -> login in
    // the same tab reported no presence at all until a full reload. The old
    // loop may still be sleeping when a new one starts; it exits at its next
    // check and `ping_active` is idempotent, so the overlap is harmless.
    let presence_started = RwSignal::new(None::<Uuid>);
    Effect::new(move |_| {
        match current_user.get() {
            Some(Ok(None)) => {
                if presence_started.get_untracked().is_some() {
                    presence_started.set(None);
                }
            }
            Some(Ok(Some(user))) => {
                if presence_started.get_untracked() == Some(user.id) {
                    return;
                }
                presence_started.set(Some(user.id));
                leptos::task::spawn_local(async move {
                    loop {
                        if !matches!(current_user.get_untracked(), Some(Ok(Some(_)))) {
                            break;
                        }
                        let _ = crate::auth::ping_active().await;
                        gloo_timers::future::TimeoutFuture::new(PRESENCE_PING_INTERVAL_MS).await;
                    }
                });
            }
            None | Some(Err(_)) => {}
        }
    });
```

- [ ] `Uuid` is already imported in `app.rs` — `use uuid::Uuid;` on **:9** (used by `provide_context(RwSignal::<Option<(Uuid, u64)>>::new(None))` at :114 and by `GamePage`). Confirm with `grep -n "^use uuid" rust/web/src/app.rs` (expect `9:use uuid::Uuid;`); add nothing. `crate::theme::is_known_slug` (`theme.rs:120`), `local_data_theme` (`app.rs:277`), `set_theme_client` (`app.rs:246`), `crate::auth::get_user_theme` (`auth/server.rs:576`), `crate::auth::set_theme` (`:553`) and `crate::auth::ping_active` (`:591`) all already exist and are already called from these two blocks — the replacement text below changes only the latch type and the match shape, never a call.

**Verification checkpoint:**

- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean; `cargo fmt --all -- --check` clean.
- [ ] `cargo test -p web --features ssr` — all pass.

**Test plan.**

- **(a)/(b)/(c) — none possible.** These are two `Effect`s in `App` that never run on the server (`docs/hydration.md:80-104`: Effects are inert during SSR), so an SSR test can observe nothing, and there is no pure function to extract without inverting the control flow of the whole component. **This task has no automated coverage. Say so; do not fake it with an SSR test that asserts an unrelated marker.** The one thing `cargo test -p web --features ssr` does guard is that `App` still compiles and every existing page test still renders cleanly — run it.
- **(d) manual checklist** (deferred — this IS the coverage):
  1. Log in as user A. In `/settings` pick a distinctive theme (e.g. "Solarized Dark") so A has a stored profile theme.
  2. Log in as user B in the same browser (different session) and give B a clearly different stored theme (e.g. "Solarized Light").
  3. Fresh tab, log in as A. **Expect:** A's theme applies. Devtools → Network: a `ping_active` request within a second of login.
  4. **Without reloading**, click "(logout)", then log in as **B** in the same tab.
     **Expect (fails before this task, passes after):** the page switches to **B's** theme, and a **new** `ping_active` request fires. Before the fix neither happened.
  5. Repeat 4 logging back in as **A** (same user as before the logout). **Expect:** a new `ping_active` fires — this is the case that id-keying alone would not fix, so it is the assertion that proves the logout-clear is present.
  6. Leave the tab open >5 minutes after step 5 and confirm a second `ping_active` (the loop is genuinely running, not a single call).
  7. Log out and stay on `/login` for >5 minutes. **Expect:** no `ping_active` requests at all.

- [ ] **Commit:** `fix(web): re-arm presence-ping and profile-theme sync after logout` — cites wfe F54.

---

### Task 7: hoist `friend_request_count` into `App` (wfe F57)

**Problem (restated):** `SidebarMenu` creates `friend_request_count` locally (`layout.rs:135-136`). The comment just above it (:126-129) documents that *every page wraps its own `<MainLayout>`*, so the sidebar remounts on every route change, and that `active_games`/`current_user` were hoisted into `App` (`app.rs:126-143`) for exactly that reason. `friend_request_count` was left behind: every navigation refires `get_incoming_friend_request_count` and the `(N new)` badge vanishes until it resolves. It also never tracks `last_update`, unlike `active_games` (`app.rs:129-133`), so an incoming friend request does not update the badge until the next navigation.

**Fix (re-derived):** create it in `App` alongside its two siblings, track `last_update` for liveness, and provide it through context. **Wrap it in a newtype.** Context is keyed by type, and `LocalResource<Result<usize, ServerFnError>>` is a structurally generic type that a future feature could easily collide with; the repo already newtypes contexts for exactly this reason (`CommandInputText`, `components/game.rs:12-16`, whose doc comment says *"Newtype so the context can't collide with other RwSignal<String> providers"*; also `WebSocketTrigger` `websocket_client.rs:5`, `ProposalUpdate` `:11`, `SubMenuOpen` `layout.rs:12-16`). `LocalResource<T>` is `Copy` (`leptos_server-0.8.7/src/local_resource.rs:294`), so a `#[derive(Clone, Copy)]` newtype is free.

**Why the two existing sibling resources are NOT newtyped, and why that is not a reason to skip it here:** `active_games` and `current_user` are provided and consumed as bare `LocalResource<…>` (provided `app.rs:134`/`:143`, consumed `layout.rs:130`/`:131-132`, `app.rs:289-290`/`:472-476`, `settings.rs:13-14`, `ThemeSection` `:473-474`). Their type parameters (`SidebarGames`, `Option<AuthUser>`) are domain-specific enough to be collision-proof; `Result<usize, ServerFnError>` is not. Do **not** "consistency-fix" the two existing ones — that is out of scope and would touch eight `expect_context::<LocalResource<…>>` call sites (`settings.rs:14`, `:474`, `app.rs:290`, `:473`, `layout.rs:53`, `:130`, `:132`, `admin.rs:1001` — and `admin.rs` is forbidden by the Non-Goals LEAD RULING, which by itself makes the sweep impossible inside this package).

**Note that `app.rs:725` already provides a bare `LocalResource<Result<Vec<GameLogEntry>, ServerFnError>>`** for the game logs, consumed at `components/game.rs:375-377` and `:420-422`. That is a second un-newtyped generic resource in the same context tree. It does not collide with `Result<usize, …>`, so it is not a blocker — noted so the implementer does not think the new newtype is redundant with it.

**Edge cases:**
- `LocalResource` never resolves on SSR, so the badge is absent from server HTML before and after — no hydration change.
- Put the declaration **after** `provide_context(current_user);` (`app.rs:143`) so the three sibling resources sit together, and provide it immediately. It must be **above `<Router>`** (the `view!` starts at :213) — that is the whole point of the hoist, and putting it inside the router would make `expect_context` panic in `SidebarMenu`.
- The newtype needs to be visible from both `app.rs` and `layout.rs`. Declare it in `layout.rs` (next to `SubMenuOpen` at :12-16, the existing precedent for a layout-scoped context type) and reference it from `app.rs` as `crate::components::layout::FriendRequestCount` — matching how `app.rs` already reaches into that module (`components/game.rs:90` does the same for `SubMenuOpen`; `friends.rs:355`, `new_game.rs:95`/`:192`, `players.rs:217`/`:515`/`:762` all path into `crate::components::layout::`). `LocalResource` and `ServerFnError` are already in scope in `layout.rs` via `use leptos::prelude::*;` (:3), so the newtype needs no new imports.
- `crate::friends::get_incoming_friend_request_count` (`friends.rs:135-144`) takes no arguments and is currently passed as a bare fn to `LocalResource::new` (`layout.rs:136`). The hoisted version must instead be an `async move` block that tracks `last_update` first, exactly like `active_games` (`app.rs:129-133`). `last_update` is the `ReadSignal<u64>` destructured at `app.rs:109` and is in scope at the insertion point.

**Files:**
- Modify: `rust/web/src/components/layout.rs` (add the newtype; replace the local resource with `expect_context`)
- Modify: `rust/web/src/app.rs` (`App` — create + provide)

**Steps:**

- [ ] In `rust/web/src/components/layout.rs`, after the `SubMenuOpen` struct (currently :12-16), add:

```rust

/// The sidebar's incoming-friend-request count. Created in `App` and provided
/// via context - like `active_games`/`current_user` - so it survives the
/// per-page `<MainLayout>` remount instead of refetching and blanking the
/// `(N new)` badge on every navigation (wfe F57). Newtype so the context
/// can't collide with another `LocalResource<Result<usize, ServerFnError>>`
/// provider.
#[derive(Clone, Copy)]
pub struct FriendRequestCount(pub LocalResource<Result<usize, ServerFnError>>);
```

- [ ] In `SidebarMenu`, replace (currently :135-136)

```rust
    let friend_request_count: LocalResource<Result<usize, ServerFnError>> =
        LocalResource::new(crate::friends::get_incoming_friend_request_count);
```

  with

```rust
    let friend_request_count = expect_context::<FriendRequestCount>().0;
```

  The badge render site (currently :184-192, reading `friend_request_count.get()` at :185-186) is unchanged. `expect_context` is already in scope (used at :118, :130, :132).

- [ ] In `rust/web/src/app.rs`'s `App`, immediately after `    provide_context(current_user);` (currently **:143**; :144 is blank) add:

```rust

    // Hoisted for the same reason as `active_games`/`current_user` above: the
    // sidebar remounts on every navigation, and a local resource there
    // refetched and blanked the "(N new)" badge each time (wfe F57).
    // Tracking `last_update` also makes the badge live, matching
    // `active_games`.
    let friend_request_count: LocalResource<Result<usize, ServerFnError>> =
        LocalResource::new(move || async move {
            let _ = last_update.get();
            crate::friends::get_incoming_friend_request_count().await
        });
    provide_context(crate::components::layout::FriendRequestCount(
        friend_request_count,
    ));
```

- [ ] Confirm no other module constructs this resource: `grep -rn "get_incoming_friend_request_count" rust/web/src/` must show exactly **three** lines afterwards — the `#[server(GetIncomingFriendRequestCount, "/api")]` attribute (`friends.rs:135`), the fn signature (`:136`), the `internal(...)` context string (`:142`) — plus the single new call in `app.rs`. Before this task the same grep also shows `layout.rs:136`; after it, `layout.rs` must contain no hit at all.

**Verification checkpoint:**

- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean; `cargo fmt --all -- --check` clean.
- [ ] `cargo test -p web --features ssr` — all pass. **This is a real guard for this task**: `expect_context` **panics** if the context is missing, and every page test in `ssr_pages.rs` renders `SidebarMenu` through `MainLayout`. If the `provide_context` call is misplaced (e.g. inside `<Router>` — the `view!` at `app.rs:213`, `<Router>` at :216 — instead of above it), those tests fail with a panic in the SSR body, which `assert_clean_html_body` catches via its `"panicked at"` assertion (`tests/ssr_pages.rs:178-181`). Most of these are DB-backed `#[sqlx::test]`s; use `scripts/rust-test.sh` if no throwaway Postgres is running.

**Test plan.**

- **(b) SSR test — no new test, but strong existing coverage.** As above: `expect_context` panicking is exactly what `assert_clean_html_body`'s `panicked at` check catches (`tests/ssr_pages.rs:178-181`), across ~20 existing page tests (`home_page_anonymous` :234, `home_page_logged_in_renders_index_shell` :246, `login_page_anonymous` :259, `new_game_type_page_anonymous` :292, `players_page_*` :665 onward, `deep_dive_page_*` :1037 onward, `history_page_renders_games_placings_and_pagination` :1171, …). Run the full `cargo test -p web --features ssr` and treat any panic in a page body as this task's failure.
- **(d) manual checklist** (deferred):
  1. As user B, send user A a friend request.
  2. As user A, load any page. **Expect:** the sidebar shows `Friends (1 new)`.
  3. Navigate `/` → `/settings` → `/friends` → `/` using sidebar links (client-side navigation, not reloads).
     **Expect (fails before, passes after):** the `(1 new)` badge stays visible the whole time, never flickering to absent. Devtools → Network: **one** `get_incoming_friend_request_count` request for the whole sequence, not one per navigation.
  4. With user A's page open and idle, have user B send a second request. **Expect:** within a WS round trip the badge becomes `(2 new)` with no navigation. (Before this task it only changed on navigation.)
  5. As A, accept both requests on `/friends`. **Expect:** the badge disappears (the WS bump refetches).

- [ ] **Commit:** `refactor(web): hoist friend-request count into App like its siblings` — cites wfe F57.

---

### Task 8: stop the bot-difficulty select desyncing (wfe F58)

**Problem (restated) — TWO defects, not one. An earlier draft of this spec fixed only the second.**

`OpponentSlotEditor`'s Bot branch (`opponent_slot.rs:313-349`) has a `<select>` (:315-347) whose `prop:value` closure (:317-320) tracks only `slot()`, and whose `<option>` children closure (:328-346) tracks only `bot_names`.

- **(a) `prop:value` never applies on first build, so the select starts wrong.** `HtmlElement::build` runs `let attrs = self.attributes.build(&el);` at `tachys-0.2.18/src/html/element/mod.rs:352` and only then `self.children.build()` at `:357`. A reactive `prop:` builds a `RenderEffect`, and `RenderEffect::new`'s own doc comment is *"Creates a new render effect, which **immediately** runs `fun`"* (`reactive_graph-0.2.14/src/effect/render_effect.rs:61-62`; path: `tachys-0.2.18/src/reactive_graph/property.rs:36-48` → `html/property.rs:83-88`). So `select.value = "medium"` is executed against a `<select>` with **zero** `<option>` children — a no-op — and the browser then makes the **first** option selected as the children are inserted. Result: the moment the user clicks "Bot", the control displays the *first* difficulty in the list while `slot`'s `bot_name` is `"medium"` (set by `set_mode`, `:75`). If the fallback list happens to be showing, that is `"easy"`.
- **(b) When `bot_names` resolves, the option list is rewritten under the still-selected element.** `collect_view()`'s `Vec` rebuild reuses the existing `<option>` elements and rewrites their `value=` attributes in place; the DOM's selectedness lives on the element, not the value, so the selected element now carries a different value. (If the new list is *shorter* than the selected index, that element is removed and selectedness resets to the first option.) Meanwhile the `prop:value` `RenderEffect` does not re-run, because `slot()` did not change.

Either way the visible selection diverges from `slot`'s `bot_name`, and `on_submit` submits the state value, not what the user sees.

**Fix (re-derived), and why all three obvious fixes are wrong:**

- *Make `prop:value` also track `bot_names`.* **Wrong**, and it fixes neither half. Attributes build before children (:352 vs :357) and the effect runs immediately, so the property lands before the options on every pass, including the post-resolution one.
- *Drive selection with per-`<option>` `selected=`.* **Forbidden.** `docs/CODING.md:305-310`: *"`<option selected>` only sets `defaultSelected` — drive the value via `prop:value` on the `<select>`. … doing it per-`<option>` fights hydration."* (It also would not survive a user interaction — see the option-dirtiness note in Architecture.)
- *Gate the `<select>` on the settled resource and stop there* (the finding's second recommendation, and what an earlier draft of this spec did). **Fixes (b) only.** A select mounted later is still built attributes-first, so **(a) survives** and the control still opens on the wrong value.

So the fix is **both halves**:

1. **Gate the element on the *settled* `bot_names`**, so its option list is fixed for the element's whole lifetime — this removes (b) entirely. The gate returns `None` until `bot_names.get()` is `Some(_)`; the `Err`/empty case then falls back to the hardcoded list. `bot_names` is created once per page (`new_game.rs:235`) and never refetched, so the `None → Some` transition happens exactly once and the select is built exactly once.
2. **Apply the value from an `Effect` over a `NodeRef`**, which removes (a). Effects run *after* the render pass, so by the time the effect body executes the `<select>` has its `<option>` children. The effect tracks the `NodeRef` and `slot()`, so it re-applies on mount and on every later state change. `prop:value` is **kept** as well: it is what `docs/CODING.md:305-310` mandates, it is correct for post-mount `slot()` changes, and it never fights the effect (both write the same string).

**Edge cases:**
- **Brief empty window.** If the user clicks the "Bot" radio before `get_available_bots` resolves, the difficulty control is momentarily absent. The slot state already defaults to `bot_name: "medium"` (`opponent_slot.rs:75`), so submitting during that window is valid. It is sub-second in practice because the resource starts fetching at page load (`new_game.rs:235`, at `GameSetupPanel` construction). Do not add a spinner or a disabled placeholder select — a placeholder `<select>` would reintroduce the option-rewrite it is there to avoid.
- **`"medium"` may not be in the server's list.** If `get_available_bots` returns a list without `"medium"`, `el.set_value("medium")` selects nothing and the browser keeps the first option displayed — the same class of desync, now unfixable from the client because the state value is simply not offered. **Do not paper over it by silently rewriting `slot`'s `bot_name`** (that would submit a difficulty the user never chose). It is recorded as Cross-package **#9** and routed as a server/seed-data question. In the dev DB the `bots` table is seeded with `easy`/`medium`/`hard` (`rust/web/migrations/013_bot_efficacy.sql:41-45`, per the WP-41 review), so this does not bite today.
- **Hydration: no impact.** The Bot branch is inside `<Show when=move || mode() == SlotMode::Bot>` (:313), and `mode()` (:67) derives from `OpponentSlot`, whose `Default` is `Player` (`opponent_slot.rs:40-47`). The only things that set a slot to `Bot` are a user click (`set_mode`, :69-87) and the restart prefill effect (`new_game.rs:270-279`), which reads a `LocalResource` — `None` on SSR and inert on the first hydration pass. So this subtree never renders on the server, and the new `NodeRef` is `None` there. Verified, not assumed.
- The `on:change` handler (:321-326) is unchanged.
- Keep `prop:value`'s `_ => "medium".to_string()` fallback arm: the closure still has to be total over `OpponentSlot`. The new effect needs the same total match — factor it into one closure so the two cannot drift.
- `NodeRef` and `Effect` are in scope via `use leptos::prelude::*;` (`opponent_slot.rs:2`). `leptos::html::Select` must be named explicitly (there is no `use leptos::html;` in this file — `new_game.rs`/`settings.rs` write `leptos::html::Input` inline the same way, e.g. `opponent_slot.rs` has no precedent but `settings.rs:254` does: `NodeRef::<leptos::html::Input>::new()`).
- **Rider (same file, same defect class as Task 1's helper):** `search_error` at :129 does `format!("Search failed: {e}")` — `Display` on a `ServerFnError`, so a failed typeahead reads `Search failed: error running server function: Internal server error`. Fix it with the helper.

**Files:**
- Modify: `rust/web/src/components/opponent_slot.rs`

**Steps:**

- [ ] Add the settled-list helper, the desired-value closure, the `NodeRef` and the post-mount apply effect. After `    let (search_seq, set_search_seq) = signal(0u32);` (currently **:134**; :135 is blank, :136 begins `view! {`) insert:

```rust

    // wfe F58, half 1: the <select> below must be built exactly once, with an
    // option list that never changes afterwards. When `bot_names` resolves,
    // `collect_view()`'s Vec rebuild rewrites the existing <option> elements'
    // `value` attributes in place while the DOM's selectedness stays on the
    // same element - so the visible choice silently diverges from state. The
    // `prop:value` RenderEffect does not re-run (it tracks `slot()`, not
    // `bot_names`), and making it track `bot_names` would not help: attributes
    // are built before children (tachys html/element/mod.rs:352 vs :357) and a
    // reactive prop's RenderEffect runs immediately on creation
    // (reactive_graph effect/render_effect.rs:61-62). docs/CODING.md also
    // forbids per-<option> `selected=`. So: gate the element on the *settled*
    // resource, and its option list is fixed for its whole lifetime.
    //
    // `None` while the resource is still loading; after that the list is
    // fixed. `bot_names` is created once per page (new_game.rs:235) and never
    // refetched, so this transitions exactly once.
    let bot_name_options = move || -> Option<Vec<String>> {
        let settled = bot_names.get()?;
        Some(match settled {
            Ok(b) if !b.is_empty() => b,
            _ => vec![
                "easy".to_string(),
                "medium".to_string(),
                "hard".to_string(),
            ],
        })
    };

    // The single source of truth for what the difficulty select should show.
    // Used by both `prop:value` and the post-mount effect below, so the two
    // cannot drift.
    let bot_name_value = move || match slot() {
        OpponentSlot::Bot { bot_name, .. } => bot_name,
        _ => "medium".to_string(),
    };

    // wfe F58, half 2: `prop:value` alone can never select anything on first
    // build. The property is written during `attributes.build`
    // (tachys html/element/mod.rs:352) BEFORE `children.build` (:357), and the
    // reactive prop's RenderEffect runs immediately, so it targets a <select>
    // with no <option> children - a no-op - after which the browser selects
    // the FIRST option as the children are inserted. Effects, unlike
    // RenderEffects created during build, run after the render pass, so by
    // the time this body executes the element has its options. It re-runs on
    // mount (the NodeRef is a signal) and on every later `slot()` change.
    let bot_select = NodeRef::<leptos::html::Select>::new();
    Effect::new(move |_| {
        // NodeRef::get() is Option - never unwrap it (docs/CODING.md:63-65).
        // None on SSR and before mount.
        let Some(el) = bot_select.get() else {
            return;
        };
        el.set_value(&bot_name_value());
    });
```

- [ ] Replace the entire Bot branch — **:313-349**, from `            <Show when=move || mode() == SlotMode::Bot>` (:313) through its `            </Show>` (:349), inclusive. Line 350 is the enclosing `</div>` and :351-352 close the `view!` and the fn; leave all three. With:

```rust
            <Show when=move || mode() == SlotMode::Bot>
                <div class="form-control">
                    {move || bot_name_options().map(|names| view! {
                        <select
                            aria-label="Bot difficulty"
                            node_ref=bot_select
                            prop:value=bot_name_value
                            on:change=move |ev| {
                                let val = event_target_value(&ev);
                                if let OpponentSlot::Bot { name, .. } = get.get_untracked() {
                                    set.run(OpponentSlot::Bot { name, bot_name: val });
                                }
                            }
                        >
                            {names
                                .into_iter()
                                .map(|n| {
                                    let text = n.clone();
                                    view! {
                                        <option value=n>{text}</option>
                                    }
                                })
                                .collect_view()}
                        </select>
                    })}
                </div>
            </Show>
```

  Three things to note:

  - The option list is now built from a **plain `Vec`**, not from a closure over `bot_names` — that is the whole point of half 1. **Do not reintroduce a reactive closure inside the `<select>`.**
  - `bot_name_value` is used in two places (the `Effect` above and `prop:value` here) and that is legal without cloning: it is a closure whose only capture is `slot`, itself a closure whose only capture is `get: Signal<OpponentSlot>` — and `Signal<T>` is `Copy`, so both closures derive `Copy`. Likewise `NodeRef` and the `LocalResource` in `bot_name_options` are `Copy`.
  - `node_ref=bot_select` sits alongside `prop:value` as an attribute. Its position among the attributes does **not** matter: the `NodeRef` is populated during `attributes.build` either way, and the `Effect` that reads it runs after the whole render pass regardless.

- [ ] Rider: replace `search_error`'s message (currently **:129**)

```rust
            Some((tag, Err(e))) if *tag == current => Some(format!("Search failed: {e}")),
```

  with

```rust
            Some((tag, Err(e))) if *tag == current => Some(format!(
                "Search failed: {}",
                crate::error::action_error_message(&e)
            )),
```

  **It is `&e`, not `e` — this is settled, not a guess.** `search_action.value()` is an `RwSignal<Option<(String, Result<Vec<UserSearchResult>, ServerFnError>)>>` and `.get()` returns that `Option` **by value** (proof: the sibling arm at :116 returns its bound `results` as the fn's owned `Vec<UserSearchResult>` return value — `search_results` is `move || -> Vec<UserSearchResult>`, :110). So `e` binds an **owned** `ServerFnError` and `action_error_message(&ServerFnError)` needs the reference. (`*tag == current` still compiles because `*tag` derefs `String` to `str` and `impl PartialEq<String> for str` exists.) An earlier draft of this spec left this as "let the compiler decide" — do not.

**Verification checkpoint:**

- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean; `cargo fmt --all -- --check` clean.
- [ ] `cargo test -p web --features ssr` — all pass, including `new_game_type_page_anonymous` (`tests/ssr_pages.rs:292-296`), which renders `/games/new` — note that page is `NewGameTypePage`, the type *grid*; `OpponentSlotEditor` lives in `NewGameSetupPage`/`GameSetupPanel` (`/games/new/:type`), for which there is **no** SSR page test. The Bot subtree never renders on the server anyway, so this checkpoint proves only that the crate still compiles and the sibling pages still render.

**Test plan.**

- **(a) unit test — not applicable.** `bot_name_options` closes over the `bot_names` resource handle; extracting it as a pure fn would mean passing `Option<Result<Vec<String>, ServerFnError>>` in, which tests the two-line `match` and nothing about the bug (which is a DOM/effect-ordering bug). **Skip deliberately.**
- **(b) SSR test — no.** The subtree never renders on the server (proven above).
- **(d) manual checklist** (deferred — this is the only layer that can see the bug):
  1. **Make defect (a) visible.** `get_available_bots` must return a list whose **first** entry is not `medium`, otherwise the wrong-on-first-paint bug is invisible. The dev DB seeds `bots` with `easy`/`medium`/`hard` (`rust/web/migrations/013_bot_efficacy.sql:41-45`), which already satisfies this: the first entry is `easy`, not `medium`. Confirm the actual order in devtools (`/api/get_available_bots`) or via the `bots` table's ordering column before relying on it. If the first entry *is* `medium`, temporarily reorder it in the dev DB.
  2. `/games/new/<some game>`, network **not** throttled. Click the "Bot" radio on Opponent 1 and look at the difficulty select immediately.
     **Before the fix:** it shows the list's **first** entry (`easy`) even though state is `medium` — defect (a).
     **Expect after the fix:** it shows `medium`.
  3. **Make defect (b) visible.** Throttle the network to "Slow 3G" so `bot_names` resolves visibly late, reload, and click "Bot" **immediately**.
     **Before the fix:** the select appears instantly with the fallback list, then its labels/values change under the same selected element as the server list lands.
     **Expect after the fix:** the difficulty select is **absent** for a moment (the settled-resource gate), then appears already showing `medium`. This brief absence is intended (edge cases above); it is not a regression.
  4. Without touching the select, submit the form. **Expect:** the created game's bot is the **medium** bot — matching what the select displays. Verify on the game page's player list (`(bot: <name>)`, `components/game.rs:255-259`).
  5. Change the select to another difficulty, then switch the radio to "Email" and back to "Bot". **Expect:** the slot resets to `medium` (existing `set_mode` behaviour, `opponent_slot.rs:73-76`) and the select shows `medium`, not a stale value. (This is the case that proves the `NodeRef` effect re-runs on `slot()` changes, not just on mount.)
  6. Add a second opponent, set it to "Bot" too, give the two different difficulties, and submit. **Expect:** both bots come out as displayed — i.e. the per-slot `NodeRef`s do not cross-talk (each `OpponentSlotEditor` instance creates its own).
  7. Block `/api/search_users` and type 3+ characters in a Player slot.
     **Expect:** `Search failed: Something went wrong, please try again` — **not** `Search failed: error running server function: …`.

- [ ] **Commit:** `fix(web): build the bot-difficulty select once and apply its value after mount` — cites wfe F58 (both halves: the never-applied build-time `prop:value` and the option-list rewrite), the tachys build-order fact (`html/element/mod.rs:352` vs `:357`), `RenderEffect::new`'s immediate first run, and the CODING.md `<option selected>` prohibition.

---

### Task 9: new-game setup form — prefill errors, count clamping, and no more silent submits (wd F59, wd F63, wd F64, wd F66)

**Problem (restated), four defects in `GameSetupPanel` (`new_game.rs:232-548`):**

- **wd F59:** `let Some(Some(Ok(pf))) = prefill.get() else { return; };` (:271-273) discards every `Err` from `get_restart_prefill`. Its rejections are all meaningful: `"Not authenticated"` (`game/server_fns.rs:1329`, in the `#[server]` wrapper `:1321-1332`), `"Game not found"` (`:1265`), `"Game is not finished"` (`:1268`), `"You are not a player in this game"` (`:1275`), `"Game type not found"` (`:1287`) — the last four in `get_restart_prefill_impl` (`:1257-1319`). **It does not return `"Game version not found"`** — that is `restart_game_with_roster:1204`, a different fn. A user following a stale or unentitled `?restart=<id>` link gets a **blank default setup form headed "Restarting <name>"** (:398-403, the `format!` at :400) with no hint that the prefill failed.
- **wd F66:** the same effect sets `player_count` to `pf.opponents.len() + 1` (:274, :278) with no reference to `gt.player_counts`. The radios render only from `gt.player_counts` (:470-487, `prop:checked` at :478), so if the original game's count is no longer offered, **no radio is checked** while `on_submit` still submits the stale count.
- **wd F63:** `let Some(version_id) = selected_version_id.get_untracked() else { return; };` (:355-357) — a bare `return`, unlike every other guard in the same function which sets `form_error` (:367-372). Reachable when `gt.versions` is empty (`selected_version_id` starts `None`, :243) or when the version `<select>`'s `parse::<Uuid>().ok()` yields `None` (:429).
- **wd F64:** the create-success effect navigates only when `outcome.game_id` or `outcome.proposal_id` is `Some` (:316-324); the `RestartOutcome::Created(po)` arm has the same gap (:330-336) and `RestartOutcome::AlreadyRestarted { .. } => {}` (:348) does nothing at all. A **successful** mutation with both ids `None` leaves the user on the form with the button re-enabled, indistinguishable from having clicked nothing.

**Fix (re-derived):** all four route into the one `form_error` slot that already exists and is already rendered (**:532-536**). wd F66 additionally gets a **pure function**, `clamp_player_count`, which is the point of the change: the clamp rule is the only genuinely testable logic in this whole package, and `new_game.rs` already has a `#[cfg(test)] mod tests` (**:551-660**) that does `use super::*;` (:553) and holds five sibling pure-fn tests (`prefill_to_slots_maps_humans_and_bots` :573, `player_range_formats` :601, `weight_text_formats` :609, `filter_by_player_count` :615, `filter_by_text_is_case_insensitive_substring` :631, `sort_variants` :641 — six, in fact) to put it in.

Clamp rule (assumption **A3**): exact match wins; otherwise the nearest offered count; distance ties break **upward**. Rationale in A3 — clamping up produces a visibly empty opponent slot which `on_submit`'s existing guard refuses with a clear message (:367-372), whereas clamping down silently drops a real opponent (`resize_with` truncates, :265-268).

**Clamp against `gt.player_counts`, not `pf.player_counts`.** `RestartPrefill` does carry a `player_counts: Vec<i32>` field (`game/server_fns.rs:151`, filled from the *latest non-deprecated* version at `:1284-1287`), but `new_game.rs` never reads it — every radio comes from `GameTypeInfo::player_counts` (`:138`, read at `new_game.rs:470`). Clamping to what the radios actually render is what makes a radio check. The unused field is Cross-package **#8**.

**Edge cases:**
- `counts` empty → return `wanted` unchanged; there is nothing to clamp to and no radio will render either way.
- Distance arithmetic is done in `i64` so a hostile/absurd `player_counts` value cannot overflow `i32::abs` (which panics on `i32::MIN`). The values come from game-version metadata and are small, but the widening is free.
- The prefill effect must set `form_error` **and still apply the prefill** when it clamps — a partially-usable form beats a blank one.
- Setting `form_error` from the prefill effect and then clearing it in `on_submit` (`set_form_error.set(None)` at :378) is correct: the warning is about the prefill, and once the user submits they have seen it.
- The `form_error` slot is **already rendered** at :532-536 with `class="form-error"`; there is a second `form-error` div for `server_error()` at :537-545 which already uses `user_facing_server_error` (:541). Do not add a third render site and do not touch the `server_error` block.
- `prefill` returns `Option<Option<Result<…>>>`: outer `None` = still loading, inner `None` = not a restart at all. Both must stay non-actions.
- The prefill error uses `action_error_message` (the server's message is the useful part: *"You are not a player in this game"* tells the user precisely what is wrong), not `user_facing_server_error`.
- **Do not** add trimming, lowercasing or empty-checking to the `OpponentSlot::Email(email)` arm (:374) — that is wd F62, owned by WP-50 (Non-Goals).

**Files:**
- Modify: `rust/web/src/new_game.rs` (one new pure fn + its tests; the prefill effect; both success effects; `on_submit`'s first guard)

**Steps:**

- [ ] Add the pure function immediately after `prefill_to_slots` (which ends with its `}` on **:57**; :58 is blank):

```rust

/// Picks which offered player count to use when a restart prefill asks for a
/// count the game type no longer offers (wd F66). Exact match wins; failing
/// that, the nearest offered count, with distance ties broken **upward**:
/// clamping up leaves an obviously-empty opponent slot, which `on_submit`
/// already refuses with a clear message, whereas clamping down would
/// silently truncate a real opponent out of the roster. Returns `wanted`
/// unchanged when the type offers no counts at all - there is nothing to
/// clamp to, and no radio renders either way.
///
/// Distance is computed in i64 so `abs()` cannot panic on a pathological
/// value (`i32::MIN.abs()` panics).
fn clamp_player_count(counts: &[i32], wanted: i32) -> i32 {
    counts
        .iter()
        .copied()
        .min_by_key(|&c| {
            let distance = (i64::from(c) - i64::from(wanted)).abs();
            (distance, -i64::from(c))
        })
        .unwrap_or(wanted)
}
```

- [ ] Replace the prefill effect — **:270-279**, from `    Effect::new(move |_| {` (:270) through `    });` (:279), inclusive — with:

```rust
    // wd F59: `Err` used to be discarded here, leaving a blank default form
    // headed "Restarting <name>". wd F66: the requested count used to be
    // applied verbatim, so a count the game type no longer offers left every
    // radio unchecked while the form still submitted the stale value.
    let gt_counts = StoredValue::new(gt.player_counts.clone());
    Effect::new(move |_| {
        let Some(Some(result)) = prefill.get() else {
            return;
        };
        let pf = match result {
            Ok(pf) => pf,
            Err(e) => {
                set_form_error.set(Some(format!(
                    "Could not load the previous game's setup: {}",
                    crate::error::action_error_message(&e)
                )));
                return;
            }
        };
        let wanted = (pf.opponents.len() + 1) as i32;
        let count = gt_counts.with_value(|counts| clamp_player_count(counts, wanted));
        if count != wanted {
            set_form_error.set(Some(format!(
                "The previous game had {wanted} players, which this game does not offer. \
                 Set up for {count} instead."
            )));
        }
        let slots = prefill_to_slots(&pf.opponents);
        set_selected_version_id.set(Some(pf.version_id));
        set_opponent_slots.set(slots);
        set_player_count.set(count);
    });
```

  **Placement note — verified, no conditional.** `gt` is the component's owned `GameTypeInfo` parameter (`new_game.rs:233`) and is moved into a `StoredValue` at **:394** (`let gt = StoredValue::new(gt);`), which is **124 lines after** this effect. Between :233 and :394 `gt` is only read by reference (`gt.versions.first()` :243, `gt.player_counts.first()` :244), so `gt.player_counts.clone()` at :270 compiles and leaves `gt` intact for the :394 move. `gt_counts` clones just the `Vec<i32>`; `StoredValue<T>` is `Copy` regardless of `T`, so capturing it in the `move` closure is free. There is **no** "if the borrow does not compile" branch — an earlier draft of this spec carried one, and it is unnecessary.

- [ ] Replace the create-success effect — **:316-324**, from `    Effect::new(move |_| {` (:316) through `    });` (:324), inclusive. Leave `let navigate_create = navigate.clone();` (:315) in place. With:

```rust
    Effect::new(move |_| {
        if let Some(Ok(outcome)) = create_action.value().get() {
            if let Some(gid) = outcome.game_id {
                navigate_create(&format!("/games/{}", gid), NavigateOptions::default());
            } else if let Some(pid) = outcome.proposal_id {
                navigate_create(&format!("/invites/{}", pid), NavigateOptions::default());
            } else {
                // wd F64: a successful mutation that carries neither id used
                // to leave the user on the form with the button re-enabled,
                // indistinguishable from nothing having happened.
                set_form_error.set(Some(
                    "Created, but no game or invite link came back. \
                     Check your games in the menu."
                        .to_string(),
                ));
            }
        }
    });
```

- [ ] In the restart-success effect (`:327-351`), replace the final no-op arm (currently **:348**). Do **not** touch the two `AlreadyRestarted` arms that *do* navigate (`:337-341` `game_id: Some(g)`, `:342-347` `proposal_id: Some(p)`) — arm order is load-bearing, the catch-all must stay last.

```rust
                RestartOutcome::AlreadyRestarted { .. } => {}
```

  with

```rust
                // wd F64: same both-None case as the create path above.
                RestartOutcome::AlreadyRestarted { .. } => {
                    set_form_error.set(Some(
                        "This game was already restarted, but no link came back. \
                         Check your games in the menu."
                            .to_string(),
                    ));
                }
```

  and, in the `RestartOutcome::Created(po)` arm (currently **:330-336**), add the same `else` branch:

```rust
                RestartOutcome::Created(po) => {
                    if let Some(gid) = po.game_id {
                        navigate_restart(&format!("/games/{gid}"), NavigateOptions::default());
                    } else if let Some(pid) = po.proposal_id {
                        navigate_restart(&format!("/invites/{pid}"), NavigateOptions::default());
                    } else {
                        set_form_error.set(Some(
                            "Created, but no game or invite link came back. \
                             Check your games in the menu."
                                .to_string(),
                        ));
                    }
                }
```

- [ ] Replace `on_submit`'s first guard (currently :355-357)

```rust
        let Some(version_id) = selected_version_id.get_untracked() else {
            return;
        };
```

  with

```rust
        // wd F63: this used to be a bare `return`, so clicking the button did
        // literally nothing when no version was selected - unlike every other
        // validation path below, which sets `form_error`.
        let Some(version_id) = selected_version_id.get_untracked() else {
            set_form_error.set(Some(
                "No game version is available for this game type.".to_string(),
            ));
            return;
        };
```

- [ ] Add the clamp tests inside the existing `#[cfg(test)] mod tests` (**:551-660**), after `player_range_formats` (`:600-606`; :607 is blank, `weight_text_formats` starts at :608). The module already has `use super::*;` (:553), so `clamp_player_count` resolves with no extra import; the four tests need no fixture (the existing `gt(name, counts, weight)` helper at :557-571 is for `filter_and_sort` and is not used here):

```rust

    #[test]
    fn clamp_player_count_keeps_offered_counts() {
        assert_eq!(clamp_player_count(&[2, 3, 4], 3), 3);
        assert_eq!(clamp_player_count(&[2, 3, 4], 2), 2);
        assert_eq!(clamp_player_count(&[2, 3, 4], 4), 4);
    }

    #[test]
    fn clamp_player_count_clamps_above_max_and_below_min() {
        // Above the offered maximum.
        assert_eq!(clamp_player_count(&[2, 3, 4], 6), 4);
        // Below the offered minimum.
        assert_eq!(clamp_player_count(&[3, 4, 5], 2), 3);
        // Single offered count absorbs anything.
        assert_eq!(clamp_player_count(&[2], 9), 2);
    }

    #[test]
    fn clamp_player_count_handles_non_contiguous_sets_and_breaks_ties_upward() {
        // 3 is equidistant from 2 and 4; A3 says pick the higher, so the
        // shortfall shows up as an empty slot rather than a dropped opponent.
        assert_eq!(clamp_player_count(&[2, 4, 6], 3), 4);
        assert_eq!(clamp_player_count(&[2, 4, 6], 5), 6);
        // Unambiguous nearest inside a gap.
        assert_eq!(clamp_player_count(&[2, 6], 3), 2);
        assert_eq!(clamp_player_count(&[2, 6], 5), 6);
    }

    #[test]
    fn clamp_player_count_passes_through_when_nothing_is_offered() {
        assert_eq!(clamp_player_count(&[], 7), 7);
    }
```

**Verification checkpoint:**

- [ ] `cd /home/beefsack/Development/brdgme/rust && cargo test -p web --features ssr clamp_player_count` — **four tests pass**. This is the only new automated coverage in the package; it must be green before moving on.
- [ ] `cargo test -p web --features ssr` — all pass, including the six pre-existing `new_game.rs` pure-fn tests (`prefill_to_slots_maps_humans_and_bots`, `player_range_formats`, `weight_text_formats`, `filter_by_player_count`, `filter_by_text_is_case_insensitive_substring`, `sort_variants`) and `new_game_type_page_anonymous` (`ssr_pages.rs:292`), `restart_game_on_finished_game_succeeds` (`:520`), `restart_game_with_roster_uses_passed_version` (`:579`).
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean; `cargo fmt --all -- --check` clean.

**Test plan.**

- **(a) unit test — written above, and the extraction is the point.** wd F66 is the one item here whose logic is separable from the reactive graph, so it gets a real pure function and four tests covering: in-set, above max, below min, single-count, non-contiguous with an upward tie-break, unambiguous gap, and empty.
- **(b) SSR test — no new test.** Every one of the four defects is a client-side reactive path: the prefill effect, two action-result effects, and a submit handler. None runs on the server. Existing `ssr_pages.rs` coverage (`new_game_type_page_anonymous` :292, `restart_game_on_finished_game_succeeds` :520, `restart_game_with_roster_uses_passed_version` :579) guards the render and the server fn, not these paths.
- **(d) manual checklist** (deferred):
  1. **wd F59:** as user A, open `/games/new/<type>?restart=<the id of a game A is NOT in>`.
     **Expect:** the heading still says `Restarting <type>` (unchanged) **and** a red `form-error` line reads `Could not load the previous game's setup: You are not a player in this game`. Before the fix: a silent blank form.
  2. Repeat with `?restart=<a game that is not finished>` → `…: Game is not finished` (`game/server_fns.rs:1268`). With `?restart=<random uuid>` → `…: Game not found` (`:1265`). **Do not** expect `Game version not found` — `get_restart_prefill` cannot produce it.
  3. **wd F66:** pick a game type with a non-contiguous or narrow count set. Finish a game with N players, then change/choose a type whose `player_counts` excludes N and open its `?restart=<that game id>`.
     **Expect:** a radio **is** checked (the clamped count), and a red line reads `The previous game had N players, which this game does not offer. Set up for M instead.` Before the fix: no radio checked, no message, and submitting sent N.
  4. In the clamped-up case, confirm the extra opponent slot is empty and that clicking "Restart game" shows the existing `Choose a player for each Player slot…` error rather than submitting.
  5. **wd F63:** hard to reach naturally. In the dev DB, remove all `game_versions` rows for one game type (or point the type at zero versions) and open its setup page. Click "Start game".
     **Expect:** `No game version is available for this game type.` Before the fix: nothing at all happened.
  6. **wd F64:** requires a server response with both ids `None`. Use devtools to override the `create_proposal` response body to `{"Ok":{"game_id":null,"proposal_id":null}}` (match the real wire shape).
     **Expect:** `Created, but no game or invite link came back. Check your games in the menu.` Before the fix: the button just re-enabled.
  7. Confirm the happy paths are untouched: a normal create navigates to `/games/<id>` or `/invites/<id>`; a normal restart of a finished game prefills the roster and the correct count with **no** error line.

- [ ] **Commit:** `fix(web): surface new-game prefill and submit failures, clamp prefill player count` — cites wd F59, wd F63, wd F64, wd F66 and records assumptions A1-A3.

---

### Task 10: make the three click-only anchors keyboard-accessible (wfe F61)

**Problem (restated):** three `<a>` elements have `on:click` and `style="cursor:pointer"` but no `href`, `tabindex` or `role`, so they are not in the tab order and cannot be activated from the keyboard:

- `components/layout.rs:166-171` — "logout" (dispatches `Logout`); the `style="cursor:pointer"` is on :170
- `app.rs:603` — "I already have a login code" (sets `show_code_input` true)
- `app.rs:623` — "Logging in as \<email\>" (sets `show_code_input` false)

**Fix (re-derived), chosen per site by reading what each does:** all three become `<a href="#" on:click=… ev.prevent_default()>`, which is the codebase's own dominant idiom — `components/game.rs:116-119` (Undo), `:124-132` (Concede), `:137-145` (End game), `:176-179` (Bump bot), `:189-197` (Delete), `:288-294` (Add friend), `friends.rs:447`, `:452`, `:457`, `:489`, `:543`, `opponent_slot.rs:244-253`, `:273-284`, and `GameCommandInput`'s suggestion links (`components/game.rs:624`). `href="#"` puts the element in the tab order and makes Enter activate it, which is the whole of the defect.

**`<button>` is rejected for all three, with a reason per site:**
- **`app.rs:603`** is inside `<form on:submit=on_email_submit>` (**:574**). A `<button>` there defaults to `type="submit"`, so getting it right requires `type="button"` *and* it would then be styled as a form button, visually breaking the `.login .hasCode` small-text link (`main.scss:116-118`, `font-size: 0.8em`).
- **`app.rs:623`** sits inline inside a sentence (`"Logging in as " <a>{email}</a>`). A `<button>` mid-sentence needs a full `appearance: none; background: none; border: none; padding: 0; font: inherit; color: …; text-decoration: underline` reset to stop looking like a button — i.e. a new CSS rule, and `main.scss` is out of scope (Non-Goals).
- **`layout.rs:166-171`** sits inline between literal `" ("` (:165) and `")"` (:172) text nodes in the sidebar. Same inline-styling problem.

**Styling is preserved for free:** `main.scss:17-21` is `a, a:hover, a:link, a:visited { text-decoration: underline; color: var(--mk-blue); cursor: pointer; }`. The bare `a` selector already matches an href-less anchor, so these three are *already* styled — and `cursor: pointer` is already in that rule (:20), which makes the inline `style="cursor:pointer"` redundant at all three sites. **Delete the inline style** at each: it is the only marker distinguishing these anchors, and its removal is what the SSR test below asserts on.

**Edge cases:**
- `ev.prevent_default()` is mandatory at all three: without it the browser appends `#` to the URL, and under `leptos_router` a hash-only change is a same-document navigation that would close the sidebar menu (`layout.rs:141-144` resets `set_open` on every `location.pathname` change; `MainLayout` has the mirror effect for the sub-menu at `:43-46`).
- `app.rs:603`'s handler is currently the named closure `show_code_link` (**`app.rs:549-551`**), typed `move |_|`. It must take the event and prevent default; change it in place rather than inlining, so the `on:click=show_code_link` site stays a one-word attribute. Its parameter type must be written out: `move |ev: leptos::ev::MouseEvent|`. `leptos::ev` is `tachys::html::event` (`leptos-0.8.20/src/lib.rs:323`) and `MouseEvent` is available there (the `MouseEvent` web-sys feature is enabled by `tachys-0.2.18/Cargo.toml:230`); the file already uses the same style for `leptos::ev::SubmitEvent` at `app.rs:519` and `:530`, and `components/game.rs:612` writes `move |ev: leptos::ev::MouseEvent|` verbatim.
- `GameCommandInput`'s type-anywhere keydown listener (`components/game.rs:512-542`) deliberately leaves Space alone whenever something other than `<body>` is focused, *"so Tab-focused links/buttons keep their normal keyboard behaviour"* (:509-511, the check at :536-538). Adding these three to the tab order therefore does not interact badly with it: anchors are activated by Enter, not Space, so nothing is swallowed.
- **SSR structural safety:** the element type stays `<a>`; only attributes change (an `href` gained, a `style` dropped). `docs/CODING.md:146-148`: attribute differences are tolerated — and in any case SSR and client render identically here. The `app.rs:623` step also re-indents the enclosing `<div>` onto multiple lines; that changes whitespace text nodes inside the `<div>`, which the `view!` macro elides, so it is not a structural change.

**Files:**
- Modify: `rust/web/src/components/layout.rs` (one anchor)
- Modify: `rust/web/src/app.rs` (`LoginPage` — one closure + two anchors)
- Modify: `rust/web/tests/ssr_pages.rs` (two assertions)

**Steps:**

- [ ] `layout.rs`: replace the logout anchor (as left by Task 4)

```rust
                            <a
                                on:click=move |_| {
                                    logout_action.dispatch(crate::auth::Logout {});
                                }
                                style="cursor:pointer"
                            >"logout"</a>
```

  with

```rust
                            // href="#" + prevent_default is this codebase's
                            // click-link idiom and is what puts the element in
                            // the tab order (wfe F61). `cursor: pointer` comes
                            // from the global `a` rule in main.scss, so the
                            // inline style is redundant.
                            <a
                                href="#"
                                on:click=move |ev| {
                                    ev.prevent_default();
                                    logout_action.dispatch(crate::auth::Logout {});
                                }
                            >"logout"</a>
```

- [ ] `app.rs`: replace `show_code_link` (currently **:549-551**)

```rust
    let show_code_link = move |_| {
        set_show_code_input.set(true);
    };
```

  with

```rust
    // Takes the event so the anchor below can be a real, focusable
    // `href="#"` link without the browser appending a hash (wfe F61).
    let show_code_link = move |ev: leptos::ev::MouseEvent| {
        ev.prevent_default();
        set_show_code_input.set(true);
    };
```

- [ ] `app.rs`: replace the "I already have a login code" anchor (currently :603)

```rust
                            <a on:click=show_code_link style="cursor:pointer">"I already have a login code"</a>
```

  with

```rust
                            <a href="#" on:click=show_code_link>"I already have a login code"</a>
```

- [ ] `app.rs`: replace the "Logging in as" anchor (currently :623)

```rust
                        <div>"Logging in as " <a on:click=move |_| set_show_code_input.set(false) style="cursor:pointer">{email.get()}</a></div>
```

  with

```rust
                        <div>
                            "Logging in as "
                            <a
                                href="#"
                                on:click=move |ev| {
                                    ev.prevent_default();
                                    set_show_code_input.set(false);
                                }
                            >{email.get()}</a>
                        </div>
```

- [ ] `grep -rn "cursor:pointer" rust/web/src/` must return **nothing**. Before this task it returns exactly three lines: `app.rs:603`, `app.rs:623`, `components/layout.rs:170`. (Note `components/game.rs:264` has `style="cursor: help;"` on an `<abbr>` — different string, different element, leave it.)

- [ ] Add the SSR assertions. In `login_page_anonymous` (`tests/ssr_pages.rs:259-268`, `#[sqlx::test]` on :258), after the existing `assert_clean_html_body(…)` call (which ends at :267 — `body` is still in scope), add:

```rust
    // wfe F61: the click-only anchors on this page were not focusable. Their
    // inline `style="cursor:pointer"` is the marker that they lacked an href;
    // the codebase has no other inline cursor style.
    assert!(
        !body.contains("cursor:pointer"),
        "a click-only anchor without href is still present: {body}"
    );
    assert!(
        body.contains("I already have a login code"),
        "expected the code-entry link in the login page body: {body}"
    );
```

  And in `home_page_logged_in_renders_index_shell` (`tests/ssr_pages.rs:246-256`, `#[sqlx::test]` on :245), after its `assert_clean_html_body(…)` on :255:

```rust
    // wfe F61: the sidebar logout link is rendered on EVERY page - the
    // wrapper div's `hidden` attribute toggles, not its children (see the
    // comment at components/layout.rs:151-153) - so this is where the layout
    // half of the fix is observable server-side. The logged-in test is used
    // rather than the anonymous one only because the marker reads naturally
    // here; either would catch a regression.
    assert!(body.contains("logout"), "expected the sidebar logout link: {body}");
    assert!(
        !body.contains("cursor:pointer"),
        "the sidebar logout anchor still has no href: {body}"
    );
```

**Verification checkpoint:**

- [ ] `cargo test -p web --features ssr login_page_anonymous` — passes.
- [ ] `cargo test -p web --features ssr home_page_logged_in_renders_index_shell` — passes.
- [ ] `cargo test -p web --features ssr` — all pass.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean; `cargo fmt --all -- --check` clean.

**Test plan.**

- **(b) SSR test — the right layer, written above.** Two of the three anchors are in server-rendered HTML: `app.rs:603` because `<Show when=move || !show_code_input.get()>` (`app.rs:571`) is `true` on SSR (`show_code_input` starts `false`, `:478`), and the sidebar logout because its `hidden`-toggled wrapper still renders its content. **`app.rs:623` is NOT server-rendered** — it is inside `<Show when=move || show_code_input.get()>` (`:620`), which is `false` on SSR — so the SSR assertions cover two of three sites; the third is covered only by manual step 2. Say so; do not claim three. The absence of `cursor:pointer` is still a stable, meaningful and cheap guard, because a partial fix leaves the string behind at whichever site was missed *if* that site renders — and the crate-wide `grep` step above covers the third deterministically.
- **(c) Playwright — no.** A `page.keyboard.press("Tab")` assertion would be ideal but the e2e spec is already stale against live code (Cross-package #2) and the suite must stay under ~1 min.
- **(d) manual checklist** (deferred, keyboard-only):
  1. `/login`, click nothing. Press **Tab** repeatedly. **Expect:** focus reaches "I already have a login code" with a visible focus ring; **Enter** switches to the code-entry view. Before the fix Tab skipped it entirely.
  2. In the code view with an email already entered, Tab to "Logging in as \<email\>" and press **Enter**. **Expect:** back to the email view.
  3. Confirm both links still look identical (blue, underlined, pointer cursor on hover) and that the URL does **not** gain a trailing `#`.
  4. Logged in on any page, open the sidebar, Tab to "(logout)", press **Enter**. **Expect:** logout and navigation to `/login`; no `#` in the URL; the mobile menu does not close-then-reopen.

- [ ] **Commit:** `fix(web): make the three click-only anchors keyboard-accessible` — cites wfe F61 and names the three sites.

---

### Task 11: log timestamps follow the browser locale; delete the stale placeholder comment (wfe F60, wfe F62)

**Problem (restated):**

- **wfe F60:** `format_log_time` (`components/game.rs:312-325`, its two-line comment at `:310-311`) is introduced by the comment *"Formats in the browser's local time zone via Date.toLocaleString"* but calls `date.to_locale_string("en-US", &options.into())` (:324) with `hour12: true` forced (:323). The time zone genuinely is local (that is `Date`'s behaviour, not the locale argument's), but the month wording, field order and clock convention are pinned to US English for every user.
- **wfe F62:** `components/mod.rs:1-2` reads *"Components module - placeholder for UI components / This will be expanded in later milestones"*. The module declares six submodules (`confirm`, `form`, `game`, `layout`, `opponent_slot`, `spinner`, :4-9) and five glob re-exports (:11-15 — `game` is deliberately not re-exported).

**Fix (re-derived):** read the browser's own locale and pass it as the locale string, and drop the forced `hour12` so each locale uses its own convention (assumption **A4**). Delete the two comment lines.

**Why this cannot cause a hydration mismatch (verified, not assumed):** `format_log_time` is called only from `render_log_entries` (`game.rs:346`), which is called only from `GameLogs` (`:409`) and `RecentGameLogs` (`:446`). Both gate their entire output behind `mounted.get()` (`:386-387` and `:425-426`), a signal set by an `Effect` — so both render **nothing** on the server and **nothing** on the client's first hydration pass, by explicit design (the comment at `:379-385` says so, and `docs/hydration.md:75-79` names this very function as the worked example). Timestamps only ever appear after mount, on the client, from a `LocalResource`. There is no server-produced string to disagree with.

**Edge cases — and the one thing an earlier draft of this spec got wrong:**
- `js_sys::Date::to_locale_string`'s binding is `pub fn to_locale_string(this: &Date, locale: &str, options: &JsValue) -> JsString` (`js-sys-0.3.98/src/lib.rs:6689`). **So `undefined` cannot be passed for the locale** — the parameter is `&str`. The locale must be read as a real string.
- **`web_sys::Window::navigator()` does NOT compile in this crate.** `Window::navigator` is gated on the `Navigator` web-sys feature, and nothing in the `ssr` dependency graph enables it (see the web-sys feature note in Architecture: `rust/web/Cargo.toml:77` does not list it, `tachys-0.2.18/Cargo.toml:193-312` does not, `leptos-use`'s `use_window`/`use_web_lock` features that would are not enabled, and `whoami`'s web-sys dep is `optional` + `cfg(target_arch = "wasm32")`-gated so it contributes nothing to a native `--features ssr` build). An earlier draft of this spec wrote `w.navigator()` and then hedged *"if it is not available the compile will fail — take the alternative"*. **That hedge is removed: the code below uses `js_sys::Reflect` and compiles as written**, exactly like `get_turnstile_response` (`app.rs:458-468`) reaching `globalThis.turnstile.getResponse` through `Reflect`.
- `js_sys::global()` (`js-sys-0.3.98/src/lib.rs:13578`) returns the JS global object on both wasm and native builds; `JsValue::as_string()` (`wasm-bindgen-0.2.121/src/lib.rs:372`) returns `Option<String>`, so every step is fallible-by-`Option` and the fallback is a plain `unwrap_or_else`. Nothing panics. The function is unreachable on the server anyway (proven above), but it must still compile there — and it does, because `js_sys` compiles for native targets.
- `js_sys` is a direct dependency (`rust/web/Cargo.toml:78`, `js-sys = "0.3"`), and `js_sys::Reflect` is already used in this very function (:321) and in `app.rs:462-463`. No new dependency, no new import (the file calls `js_sys::` by full path).
- Keep `window_key` (`game.rs:305-308`, comment on :305, fn `:306-308`) and the 10-minute bucketing untouched. **The edit range below starts at :310, not :307** — :307 is `window_key`'s body.

**Files:**
- Modify: `rust/web/src/components/game.rs` (`format_log_time` only)
- Modify: `rust/web/src/components/mod.rs` (delete two lines)

**Steps:**

- [ ] Replace `format_log_time` together with its two-line comment — **`components/game.rs:310-325`**, from `// Formats in the browser's local time zone via Date.toLocaleString, e.g. "Jul 11, 10:50 AM".` (:310) through the `}` that closes the fn (:325), inclusive. **Do NOT start at :307** — lines :305-308 are `window_key`'s comment and body, and :309 is blank; an earlier draft of this spec said `:307-325`, which deletes `window_key`'s `dt.assume_utc()…` body line and its closing brace and would not compile. Line 309 (blank) and line 326 (blank) must both survive. Replacement:

```rust
// Formats in the browser's local time zone AND the browser's own locale via
// Date.toLocaleString, e.g. "Jul 11, 10:50 AM" (en-US) or "11 Jul, 22:50"
// (en-GB). The locale comes from navigator.language rather than being pinned
// to en-US (wfe F60); `hour12` is left to the locale's own convention.
// Only runs client-side: render_log_entries is reached exclusively through
// GameLogs/RecentGameLogs, both of which gate all output behind their
// `mounted` signal, so neither SSR nor the first hydration pass ever calls
// this. No hydration mismatch is possible.
//
// navigator.language is read through js_sys::Reflect off the JS global, not
// through web_sys::Window::navigator(): the `Navigator` web-sys feature is
// not enabled anywhere in this crate's dependency graph, so that method does
// not exist here. Same technique as get_turnstile_response in app.rs.
fn browser_locale() -> String {
    js_sys::Reflect::get(&js_sys::global(), &"navigator".into())
        .ok()
        .and_then(|nav| js_sys::Reflect::get(&nav, &"language".into()).ok())
        .and_then(|lang| lang.as_string())
        .filter(|l| !l.is_empty())
        .unwrap_or_else(|| "en-US".to_string())
}

fn format_log_time(window: i64) -> String {
    let date = js_sys::Date::new(&((window * 600_000) as f64).into());
    let options = js_sys::Object::new();
    for (key, value) in [
        ("month", "short"),
        ("day", "numeric"),
        ("hour", "numeric"),
        ("minute", "2-digit"),
    ] {
        let _ = js_sys::Reflect::set(&options, &key.into(), &value.into());
    }
    date.to_locale_string(&browser_locale(), &options.into())
        .into()
}
```

  Note exactly what changed inside `format_log_time`: the `let _ = js_sys::Reflect::set(&options, &"hour12".into(), &true.into());` line (old :323) is **deleted**, and the locale argument is `&browser_locale()` instead of `"en-US"`. Everything else is byte-identical to the original.

  **Type notes so this compiles first time:** `js_sys::global()` returns `js_sys::Object`, which derefs to `JsValue`, so `Reflect::get(&js_sys::global(), …)` type-checks against `Reflect::get(target: &JsValue, key: &JsValue)`. `"navigator".into()` and `"language".into()` produce `JsValue` (the same `&str -> JsValue` conversion already used at :321 and `app.rs:462`). `Reflect::get` returns `Result<JsValue, JsValue>`, hence the `.ok()`. `JsValue::as_string()` returns `Option<String>`.

- [ ] There is **no fallback branch for this step.** If the build fails here, do not silently revert to `"en-US"` — STOP and report to the Lead, quoting the compiler error. (Assumption **A4** records the `"en-US"`-plus-comment-fix alternative as a *Lead-selectable* option, not as an implementer escape hatch.)

- [ ] Delete `rust/web/src/components/mod.rs:1-2`:

```rust
// Components module - placeholder for UI components
// This will be expanded in later milestones
```

  and the blank line that followed them (:3), so the file now begins with `pub mod confirm;` and is **12** lines long. Do **not** add a replacement doc comment — the module is a plain re-export barrel and needs none, matching the other barrel modules in the crate. Do not reorder or add to the `pub mod` / `pub use` lists.

**Verification checkpoint:**

- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean; `cargo fmt --all -- --check` clean.
- [ ] `cargo test -p web --features ssr` — all pass.
- [ ] `sed -n '1,3p' rust/web/src/components/mod.rs` shows `pub mod confirm;` on line 1.

**Test plan.**

- **(a) unit test — genuinely not possible, and the extraction was considered.** `format_log_time` and the new `browser_locale` are `js_sys` end to end: `js_sys::Date`, `js_sys::Object`, `js_sys::Reflect`, `js_sys::global`, `Date::to_locale_string`. Under `cargo test -p web --features ssr` (a native `x86_64` target) there is no JS runtime, so a test would either not link or panic in the binding. Splitting out a pure part is not available either: the only non-JS logic is the `window * 600_000` millisecond conversion and the four-entry option table, neither of which is where the defect was. `window_key` (`:306-308`) is the pure half and already exists, untouched here. **This item has no automated coverage. Do not add a test that asserts the option table's contents — it would lock in trivia and prove nothing. In particular do not test `browser_locale()`'s `"en-US"` fallback: on a native target every `Reflect` call is a stub and the test would assert the stub, not the behaviour.**
- **(b) SSR test — no, and provably so:** the function is unreachable on the server (the `mounted` gate).
- **(d) manual checklist** (deferred):
  1. Log in, open a game with several log entries spanning >10 minutes. **Expect:** each 10-minute group is headed `- Jul 11, 10:50 AM -` (or your locale's equivalent).
  2. Set the browser's preferred language to English (United Kingdom) (Chrome: Settings → Languages, move en-GB to the top) and hard-reload the game page.
     **Expect after the fix:** the group heading changes to the en-GB form, e.g. `- 11 Jul, 22:50 -`. Before the fix it stayed US-formatted.
  3. Set it to a non-English locale (e.g. Deutsch) and reload. **Expect:** the month abbreviation localises (`11. Juli` / `11 Juli`, per the browser) and the times are still correct.
  4. Confirm the times themselves are unchanged in every locale (same instants, local time zone) — compare one entry against the game's `logged_at` in the DB plus your UTC offset.
  5. Reload the page repeatedly and confirm **no hydration error in the console** (this is the check that the `mounted` gate reasoning holds in practice).
  6. `wfe F62`: nothing to click. Confirm `rust/web/src/components/mod.rs` starts at `pub mod confirm;`.

- [ ] **Commit:** `fix(web): follow the browser locale in log timestamps; drop stale components comment` — cites wfe F60 (recording that the locale is read via `js_sys::Reflect` because the `Navigator` web-sys feature is not enabled) and wfe F62.

- [ ] **Final package gate (run once, after this last task):** `/home/beefsack/Development/brdgme/scripts/rust-test.sh`. It provisions throwaway Postgres 18 (port 15432) and NATS 2.11 (14222), sets `DATABASE_URL`/`NATS_URL`/`SQLX_OFFLINE=true`/`RUST_MIN_STACK`, runs the migrations, then `scripts/rust-ci-commands.sh` (fmt check, clippy workspace-minus-web, clippy web with ssr, `cargo sqlx prepare --check`, workspace tests minus web, `cargo test -p web --features ssr`). AGENTS.md requires this passes before committing any Rust change; DB-backed failures in a bare run **without** this script are pre-existing (backlog #40).

---

## Cross-package / newly discovered

Not fixed by this package. Evidence recorded; routed per `work-packages.md`.

1. **`class="error"` has no CSS rule, so "error" text is not visually distinguished.** `grep -n error /home/beefsack/Development/brdgme/rust/web/style/main.scss` returns exactly two hits: `.game-main .game-command-input .command-error` (:373-376) and `.form-error` (:715-717). Meanwhile `class="error"` is used at **22** sites (`grep -rn 'class="error"' rust/web/src/`): `app.rs:607`, `:610`, `:646`, `:762`; `friends.rs:406`, `:427`; `components/layout.rs:197`; `components/game.rs:286`; `new_game.rs:106`, `:212`; `rules.rs:46`; `players.rs:240`, `:273`, `:539`, `:795`; `admin.rs:1041`, `:1166`, `:1478`, `:1558`, `:1815`, `:1827`, `:1926`. All of them render in the body colour, including the login page's "Failed to send login email" and "Invalid or expired code". **This package works around it** by rendering every *new* slot with `class="form-error"` — after WP-54 lands, 21 sites remain (Task 2 deletes `friends.rs:427`). *(An earlier draft of this spec said "eight" in one place and "~15" here; the number is 22 today.)* **Route:** `rust/web/style/main.scss` is in no package's path list. Needs either a one-line `.error { color: var(--mk-red); }` addition in a small styling package, or a sweep converting the sites to `form-error`. Lead call; do **not** fold into WP-54 (it would edit a file outside the package's paths and change the appearance of six other packages' pages).
2. **`rust/web/end2end/tests/page-loads.spec.ts` is already broken against live code (re-verified 2026-07-25).** It navigates to `/games` and expects a `"New Game"` heading, but `/games` is now an unrouted 404 — asserted by `ssr_pages.rs:303-320` (`games_route_is_unused_returns_not_found`) and by the commented-out route at `app.rs:220-221`. It then does `page.locator(".form-row", { hasText: "Opponent 1" }).locator("select").selectOption("bot")` and clicks a `"Create Game"` button: `grep -rn "form-row\|Create Game" rust/web/src rust/web/style` finds neither string in any rendered markup (the only hits are a doc comment at `components/form.rs:3` and a SQL comment at `db.rs:1125`). Opponent slots are `.form-field.opponent-slot` with **radio** inputs (`opponent_slot.rs:137-171`, radios from :144), and the submit value is `"Start game"` (`new_game.rs:525`). It also expects a `"Welcome to brdg.me"` heading on `/`, where the anonymous marker in the SSR tests is `"Lo-fi board games by email and web"` (`ssr_pages.rs:241`). So the one Playwright spec cannot pass in its current form. **Route:** e2e-suite repair; no package owns `end2end/`. Recommend the Lead file it as its own small package. WP-54 deliberately adds no Playwright assertions because of this.
3. **`e.to_string()` on a `ServerFnError` at seven more sites inside WP-54's own files, none of them named by any of the 17 findings.** `settings.rs:285`, `:301`, `:316`, `:331` (`EmailSection`'s four action effects), `components/game.rs:286` (`PlayerInfo`'s `add_friend`, whose nested `ServerAction` is declared at `:281`), `friends.rs:406` (page load) and `:427` (`add_action`), `opponent_slot.rs:129` (`search_error`). **Disposition:** because these are in this package's files, are the same defect class as wfe F55, and would otherwise leave the file internally inconsistent immediately after introducing `action_error_message`, they are folded in as explicit riders — `settings.rs` × 4 in Task 3, `friends.rs:406` and `:427` in Task 2, `opponent_slot.rs:129` in Task 8. **`components/game.rs:286` (`PlayerInfo`'s add-friend error) is NOT fixed**: it is inside a per-player nested `ServerAction` in a component this package otherwise only reads, and switching it changes what a user sees on the game page for a case (`"No user named …"` `friends.rs:167`, `"You cannot friend yourself"` `:171`) where the raw message is already close to right. It is also the one `class="error"` site inside this package's own files that Tasks 2/3/8 leave behind. **Route:** flag to the Lead as a one-line follow-up (`{e.to_string()}` -> `{crate::error::action_error_message(&e)}`; the arm binds an owned `ServerFnError`, so it is `&e`) rather than absorbing it silently.
4. **`admin.rs` presentation-layer error rendering is unowned.** **Excluded from WP-54 by LEAD RULING** because `work-packages.md:423-429` is authoritative on paths and does not list `admin.rs`. WP-37 has already recorded the corrected routing at `WP-37-admin-pass.md:2349-2350` ("this is no longer WP-54's … WP-54 has since explicitly refused it by LEAD RULING"), restated at `:45` and `:89`; WP-59 notes the same at `WP-59-inbound-processing-quality.md:2754`. *(An earlier draft of this spec cited `WP-37:2251` and `WP-59:2494-2504` — both unrelated passages.)* Concrete sites: all **15** hits of `grep -n 'e.to_string()' rust/web/src/admin.rs` — `:1024` (`let msg = e.to_string();`), `:1041` and `:1827` (`view! { <p class="error">{e.to_string()}</p> }`), and `:1119`, `:1132`, `:1144`, `:1156`, `:1431`, `:1444`, `:1456`, `:1468`, `:1767`, `:1780`, `:1792`, `:1804` (`Err(e) => error.set(Some(e.to_string()))` / `Err(e.to_string())`). Every one prints `error running server function: …` to an admin. **Route:** its own small package, or a WP-37 follow-up sequenced **after WP-54 Task 1** so it reuses `crate::error::action_error_message` instead of inventing a second helper (exactly what `WP-37:2350` asks for). Each site is then a one-line change.
5. **`reactive_graph 0.2.14`'s `Action` out-of-order guard is dead code** — already recorded by `WP-37-admin-pass.md:2347`, and re-verified here: `ArcAction::dispatch` snapshots `let current_version = self.dispatched.get_value();` (`reactive_graph-0.2.14/src/actions/action.rs:269`) and gates the write on `let is_latest = dispatched.get_value() <= current_version;` (`:288`, identically `:321`/`:340` in `dispatch_local`); `dispatched` is only ever read, never incremented, so `is_latest` is always true. **Relevance to WP-54:** Task 3's revert snapshots inherit this — two rapid failing dispatches can revert to the second snapshot rather than the first. Noted in Task 3's edge cases and deliberately not worked around. **Route:** already routed to WP-43 (web cargo deps); no action here.
6. **`friends.rs`'s two `<select>`s drive selection with per-`<option>` `selected=`** (`:555-562` invite policy, the `selected` bool at `:559`; `:567-574` game visibility, at `:571`), which `docs/CODING.md:305-310` explicitly rules out in favour of `prop:value`. **Correction to an earlier draft of this spec:** the draft claimed this "happens to work only because the whole select is re-created from scratch on every `overview` refetch". It is **not** re-created — `AnyView::rebuild` rebuilds in place whenever the same match arm is taken (`tachys-0.2.18/src/view/any_view.rs:386-400`) and `collect_view()`'s `Vec` rebuild reuses the `<option>` elements. The `selected=` attribute works only on the *first* build of a freshly mounted `FriendsPage`, and never re-syncs after that. **Not fixed here** — outside all 17 findings; converting it is item #7. **Route:** consistency nit for the `friends.rs` owner (WP-53 already touches the file), paired with #7.

7. **NEW (found by this review pass): a rejected invite-policy or game-visibility change leaves the `<select>` displaying the unsaved value, and nothing in this package can revert it.** This is the half of wd F57's recommendation that cannot be implemented as written — see the wd F57 disposition row and Task 2's preamble for the three verified reasons (identical refetched data + `bool::rebuild`'s equality skip at `tachys-0.2.18/src/html/attribute/value.rs:554-563`; in-place `AnyView::rebuild`; `<option selected>` not reassigning a dirtied option). The correct fix is to convert both selects to a `prop:value`-over-`RwSignal<String>` binding — seeded from `overview` by an `Effect`, written by `on:change`, reset by the failure arm of the policy/visibility effects — which simultaneously closes #6 and satisfies `docs/CODING.md:305-310`. **It must be `Effect`-driven, not a static `prop:value`**, because a `prop:` written during `attributes.build` lands before the `<option>` children exist and selects nothing (`tachys-0.2.18/src/html/element/mod.rs:352` vs `:357`; `reactive_graph-0.2.14/src/effect/render_effect.rs:61-62`) — the same trap Task 8 documents. **Route:** no package currently owns `FriendsPage`'s markup except WP-54, so this needs a **Lead decision**: either absorb it into Task 2 (assumption **A6**'s alternative) or file it as its own item against the `friends.rs` owner. It is neither silently fixed nor baked into any test — Task 2's checklist step 7 states the residual as the expected outcome.

8. **NEW: `RestartPrefill::player_counts` is dead on the client.** The field exists (`game/server_fns.rs:151`) and the server populates it from the *latest non-deprecated* game version (`:1284-1287`), but `new_game.rs` never reads it: `grep -n player_counts rust/web/src/new_game.rs` returns only `GameTypeInfo`-derived uses (`:71`, `:171`, `:244`, `:405`, `:470`) plus the test fixture (`:561`). So the prefill's own count set is computed, serialised and discarded — and if the latest version's counts differ from `GameTypeInfo::player_counts` the two disagree with no reconciliation. Task 9 deliberately clamps against `gt.player_counts`, i.e. what the radios actually render. **Route:** `game/server_fns.rs` belongs to **WP-53** — either drop the field or make the client use it. Not a correctness bug today.

9. **NEW: nothing guarantees the slot's default `bot_name` is present in `get_available_bots`' list.** `set_mode` hard-codes `bot_name: "medium"` (`opponent_slot.rs:75`) and the fallback option list hard-codes `easy`/`medium`/`hard` (`:331-335`), while the real list comes from the DB. If `"medium"` is ever absent from the returned list, Task 8's post-mount `el.set_value("medium")` selects nothing and the browser keeps the first option displayed — the same desync class, and unfixable client-side without silently rewriting the user's state (which Task 8 explicitly forbids). The dev DB seeds exactly `easy`/`medium`/`hard` (`rust/web/migrations/013_bot_efficacy.sql:41-45`, per the WP-41 review), so it does not bite today. **Route:** the `get_available_bots` / `bots`-table owner; nearest is **WP-53** (`game/server_fns.rs`). If no package owns it, **no owner - Lead to file.** The durable fix is for the default difficulty to come from the same source as the list.

10. **Inherited from WP-41, routed INTO this package as a note only - nothing to do.** Five pure predicate helpers in `db.rs` carry `#[cfg(feature = "ssr")]` even though they are pure and their doc comments read as shared logic: `active_within_window` (gate `db.rs:2001`, fn `:2002`), `can_remove_email` (`:2909`/`:2910`), `can_switch_to_email` (`:2916`/`:2917`), `is_expired_unverified` (`:2923`/`:2924`) and `cap_digest` (`:2938`/`:2939`) — unlike `validate_username` (`:849`), which is genuinely ungated and *is* called from the client-side settings form (its doc comment at `:848` says so). **Re-verified against live `db.rs` by this review pass: all five gates are present at exactly those lines, and `validate_username` has none.** Every current caller of the gated five is server-side (`auth/server.rs:653`, `:887`, `:1363`, `game/import.rs:178`, `email/commands.rs:458`, `:483`, plus `db.rs` itself), so **nothing is broken today and WP-54 changes nothing here.** It is recorded because any future work that wants to reuse these predicates from a WASM component — this package's own territory — must remove the gates first. Routed by `WP-41-db-quality-pass.md:1988` ("Routed to WP-54 (frontend UX) as a note, to be actioned only if a client-side caller actually appears") and accepted verbatim by the unit-4b Lead ruling on WP-41 in `planning/specs-LOG.md` ("New cross-package items accepted as routed: 5 `ssr`-gated pure predicates … -> WP-54 (note only)"). **Action for the implementer: none. Do not open `db.rs`.**

---

## Snapshot drift (verified 2026-07-25 against snapshot commit `f8763a5`)

`diff -u /home/beefsack/Development/brdgme-review-snapshot/rust/web/src/<file> /home/beefsack/Development/brdgme/rust/web/src/<file>`, per file:

| File | Result |
|---|---|
| `friends.rs` | **identical** — no drift |
| `new_game.rs` | **identical** |
| `settings.rs` | **identical** |
| `app.rs` | **identical** |
| `components/layout.rs` | **identical** |
| `components/opponent_slot.rs` | **identical** |
| `components/mod.rs` | **identical** |
| `error.rs` | **identical** (added to the table by the review pass — Task 1 edits it) |
| `tests/ssr_pages.rs` | **identical** (added by the review pass — Tasks 5 and 10 edit it) |
| `style/main.scss` | **identical** (added by the review pass — read-only here, but Cross-package #1 cites it) |
| `components/game.rs` | **DRIFTED** — 660 -> **681** lines, 35 lines matching `^[+-]` in `diff -u` (33 real changes plus the two `---`/`+++` header lines) |

`components/game.rs` drift comes from **exactly one commit**, `1f665b0` *"feat(ui): concede/end-game buttons and replaced-player display (#47)"* — `git log --oneline -- rust/web/src/components/game.rs` shows it as the only post-snapshot commit touching the file. *(An earlier draft of this spec attributed the drift to five commits, `0243472`/`1f665b0`/`3b7252f`/`998a081`/`ecfc17a`; the other four are part of #47 but did not touch `components/game.rs`.)* The hunks are: the `EndGame` import (:1-4), `can_concede`/`can_end_game` replacing `is_2player` (:31-32), `end_game_action` (:51), its success effect (:70-75), the concede `<Show>` condition changing from `!is_finished && is_2player` to `can_concede` (:122), the "End game" action link with a third inline confirm (:135-147), `profile_link=player.user_id.is_some()` replacing `!player.is_bot` (:253), and the `is_replaced` suffix in `PlayerInfo` (:255-259).

**Corrected:** the `Place:`/`Form:`/`FormStrip` block at `:269-277` is **NOT drift** — it is byte-identical in the snapshot. An earlier draft of this spec listed it as a #47 hunk at ":268-278"; `diff -u` shows no change there. It is still read-only for this package either way.

**Stale citations in the findings, corrected in the disposition table above:**

| Finding cites | Live |
|---|---|
| wfe F52 "game.rs:56", "56-61" undo, "62-67" concede, "72-77" force-delete, "four ServerActions" | :58-63 undo, :64-69 concede, :80-85 force-delete — and **five** actions, because #47 added `end_game_action` (:51) and its effect (:70-75) |
| wfe F52 "GameCommandInput error_msg (game.rs:562-570)" | :583-591 (rendered :657-659) |
| wfe F52 "PlayerInfo add_friend (game.rs:263-275)" | the nested `ServerAction` is :281, the `match` :284-296, the `Err` arm :286 |
| wfe F56 "game.rs:118" concede confirm, "170-172" force-delete confirm, "two inline blocks" | :126-128, :191-193 — and **three** blocks, the new one at :139-141 |
| wfe F60 "game.rs:303" | :312 (`fn format_log_time`), its comment :310-311, locale arg at :324, forced `hour12` at :323 |
| wfe F55 "game.rs:567" (`"never leak the raw ServerFnError text"`) | :588 |

Every other cited line in the 17 findings matches live, because the other seven files have no drift. **In particular the wfe F54 anchors (`app.rs:179`, `:185-187`, `:180-193`, `:154-173`) and the wfe F61 anchor (`layout.rs:166-171`) and the wd F73 anchors (`settings.rs:145-148`, `:210-237`, `:496-498`, `:62-69`) are all correct as the findings wrote them** — an earlier draft of this spec "re-derived" several of them incorrectly. All line numbers in this spec are **live** numbers, re-verified 2026-07-25.

## Verification protocol reminders

- Commands run from `/home/beefsack/Development/brdgme/rust`, always `-p web --features ssr`.
- Per `superpowers:verification-before-completion`: never write "tests pass" or "verified" without the command output in front of you. The manual checklists in Tasks 1-4 and 6-11 need a 32GB+ Tilt environment; if you do not have one, report them as **not run** and say which they are. Do not paraphrase them as done.
