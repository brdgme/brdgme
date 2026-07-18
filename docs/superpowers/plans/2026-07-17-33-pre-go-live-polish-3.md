# Pre-Go-Live Polish Batch 3 (#33) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the three remaining unresolved entries in `docs/pre-go-live-polish.md`: game-navigation loading spinner, command input disable-while-submitting, and autocomplete-click refocus.

**Architecture:** All changes are in the Leptos web client (`rust/web`). Task 1 extracts the login form's spinner into a reusable `Spinner` component and keys `GamePage`'s `<Transition>` on the game id so navigating to a different game remounts it (showing the spinner fallback) while seq-only websocket/command refetches keep the stale board visible. Tasks 2 and 3 are small changes inside `GameCommandInput`.

**Tech Stack:** Rust, Leptos 0.8 (SSR + hydrate), SCSS.

## Global Constraints

- One commit per task (user requirement - each task is independently committable).
- The spinner must NOT appear on websocket or command-submit rerenders of the currently visible game - only on initial load or navigation to a different game (`docs/pre-go-live-polish.md` entry "2026-07-17: No loading indicator when navigating to a game").
- `game_data` is keyed on `(game_id, last seq)` via `track_game_seq` (`rust/web/src/app.rs`); the spinner condition must distinguish a game_id change (show spinner) from a seq-only refetch (don't). Do not change `track_game_seq`.
- On command-submit error: re-enable input and button, KEEP the submitted text (entry "2026-07-11: Command input stays enabled while a command is submitting").
- Each task finishes by adding a `- **Resolved:** ...` line to its entry in `docs/pre-go-live-polish.md` (repo convention - see resolved entries in that file) and committing.
- Verification gate for every task (run from `/home/beefsack/Development/brdgme/rust`):
  - `cargo clippy -p web --all-targets --features ssr -- -D warnings`
  - `cargo test -p web --features ssr`
  (These are the exact CI commands from `.github/workflows/ci.yml`.)

## Background for implementers (zero-context primer)

- `rust/web` is a Leptos app. `rust/web/src/app.rs` holds the pages; `rust/web/src/components/` holds shared components (`mod.rs` re-exports `form::*` and `layout::*`; `game.rs` is imported explicitly).
- The login page spinner markup lives at `rust/web/src/app.rs:386-392`: a `div.spinner` with three `div.bounce1/2/3` children, styled by `.spinner` rules in `rust/web/style/main.scss:30-70`. Note `.spinner` carries `margin: 100px auto 0` (login-specific top offset).
- `GamePage` (`rust/web/src/app.rs:727`) renders the game inside `<Transition fallback=|| view! { <div></div> }>`. Transition semantics: the fallback shows only before the FIRST load of the resource; on any refetch the previously rendered children stay visible. Because the route component is reused when navigating game-to-game, today the OLD game's board stays visible (no loading indication) until the new game's data arrives.
- `game_data` is a `Resource::new_blocking` keyed on `seq_for_this_game`, a `Memo<(Option<Uuid>, Option<u64>)>` = `(game_id, last seq)`. A websocket update or command submit for the open game bumps the seq (same game_id); navigating to a different game changes the game_id. This is the distinction the spinner must honor.
- `GameCommandInput` (`rust/web/src/components/game.rs:340`) owns the command form. Its text lives in a `CommandInputText` context signal hoisted to `GamePage`. `submit_action` is a `ServerAction::<SubmitCommand>`; `submit_action.value()` is `Option<Result<Option<String>, ServerFnError>>` where `Ok(None)` = success, `Ok(Some(msg))` = game rejected the command, `Err(_)` = transport fault. On success an existing `Effect` clears the text and refocuses. The text signal is never cleared on error, so "keep the text on error" is already true - only the disabling is missing.
- The autocomplete suggestions are `<a href="#">` links rendered by `make_link` inside `GameCommandInput` (`rust/web/src/components/game.rs:459-468`); their `on_click` sets the command text but never focuses `input_ref`.
- There is no DOM/browser test harness in this repo; view-layer behavior is verified with clippy + the existing unit test suite + manual browser checks. Only pure helpers get unit tests (e.g. `track_game_seq`, `count_my_turn` in `app.rs`). None of these three tasks introduces new pure logic, so no new unit tests are added; do not write DOM tests.

Manual verification uses the dev environment. Start it per the repo's dev tooling (`cargo leptos watch` in `rust/web`, or the project's usual dev-up command if one is running already). Two games and a bot opponent make websocket updates easy to trigger.

---

### Task 1: Reusable Spinner + loading indicator when navigating to a game

**Files:**
- Create: `rust/web/src/components/spinner.rs`
- Modify: `rust/web/src/components/mod.rs`
- Modify: `rust/web/src/app.rs` (LoginPage ~line 386, GamePage ~line 815)
- Modify: `rust/web/style/main.scss` (near the `.spinner` rules, ~line 30)
- Modify: `docs/pre-go-live-polish.md` (entry "2026-07-17: No loading indicator when navigating to a game")

**Interfaces:**
- Produces: `Spinner` component (`#[component] pub fn Spinner() -> impl IntoView`), re-exported from `crate::components`. `.game-loading` CSS class for centering a spinner in a fill-height pane.
- Consumes: existing `.spinner` CSS, existing `GamePage` structure (`seq_for_this_game`, `game_data`, `Transition`).

- [ ] **Step 1: Create the Spinner component**

Create `rust/web/src/components/spinner.rs`:

```rust
use leptos::prelude::*;

/// The three-dot bounce spinner (styles: `.spinner` in main.scss). Markup
/// extracted from the login form so any page loading server data can reuse it.
#[component]
pub fn Spinner() -> impl IntoView {
    view! {
        <div class="spinner">
            <div class="bounce1"></div>
            <div class="bounce2"></div>
            <div class="bounce3"></div>
        </div>
    }
}
```

In `rust/web/src/components/mod.rs` add the module and re-export:

```rust
pub mod form;
pub mod game;
pub mod layout;
pub mod spinner;

pub use form::*;
pub use layout::*;
pub use spinner::*;
```

- [ ] **Step 2: Use Spinner in the login form**

In `rust/web/src/app.rs` (LoginPage, ~line 386) replace:

```rust
                        <Show when=move || login_action.pending().get()>
                            <div class="spinner">
                                <div class="bounce1"></div>
                                <div class="bounce2"></div>
                                <div class="bounce3"></div>
                            </div>
                        </Show>
```

with:

```rust
                        <Show when=move || login_action.pending().get()>
                            <crate::components::Spinner/>
                        </Show>
```

- [ ] **Step 3: Key GamePage's Transition on the game id and use a spinner fallback**

In `GamePage` (`rust/web/src/app.rs`), after the `seq_for_this_game` memo, add a memo of just the game id (Memo dedupes via PartialEq, so seq-only updates never touch it):

```rust
    // Just the game id, deduped: changes only when navigating to a
    // different game, never on seq-only WS/command refetches. Keying the
    // <Transition> below on this remounts it per game, so its spinner
    // fallback shows while a newly navigated-to game loads, while
    // refetches of the current game keep the stale board (no spinner).
    let current_game = Memo::new(move |_| game_id());
```

Then wrap the existing `<Transition>` block in a closure that tracks `current_game`. The current view block is:

```rust
    view! {
        <MainLayout has_sub_menu=Signal::from(true)>
            <Transition fallback=|| view! { <div></div> }>
                {move || {
                    let base = game_data.get();
                    // ... existing body unchanged ...
                }}
            </Transition>
        </MainLayout>
    }
```

Change it to (the inner `{move || { ... }}` closure body is byte-for-byte unchanged):

```rust
    view! {
        <MainLayout has_sub_menu=Signal::from(true)>
            {move || {
                current_game.track();
                view! {
                    <Transition fallback=|| view! {
                        <div class="game-loading"><crate::components::Spinner/></div>
                    }>
                        {move || {
                            let base = game_data.get();
                            // ... existing body unchanged ...
                        }}
                    </Transition>
                }
            }}
        </MainLayout>
    }
```

- [ ] **Step 4: Add the centering CSS**

In `rust/web/style/main.scss`, after the `.spinner` keyframes block (~line 70), add:

```scss
/* Fill-height centered wrapper for the spinner (game area loading state). */
.game-loading {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.game-loading .spinner {
  margin: 0;
}
```

(The `margin: 0` override is needed because `.spinner` carries the login form's `margin: 100px auto 0`.)

- [ ] **Step 5: Verify (automated)**

Run from `/home/beefsack/Development/brdgme/rust`:

```bash
cargo clippy -p web --all-targets --features ssr -- -D warnings
cargo test -p web --features ssr
```

Expected: no warnings, all tests pass (including the existing `track_game_seq_*` tests, which must be untouched).

- [ ] **Step 6: Verify (manual, dev server)**

With the dev server running and two active games available:

1. Hard-refresh a game page: centered spinner in the game area until the board renders. Login form still shows the spinner on email submit.
2. Click from game A to game B in the sidebar: spinner replaces game A's board until game B renders.
3. In an open game, submit a command: NO spinner - the board stays visible and swaps in place.
4. Trigger a websocket update for the open game (bot move / second account): NO spinner, board updates in place. Typed text in the command input survives (regression check on the 2026-07-17 command-input fix).
5. Trigger an update for a DIFFERENT game while viewing one: nothing visible changes on the open game.

- [ ] **Step 7: Update polish doc and commit**

Append to the "2026-07-17: No loading indicator when navigating to a game" entry in `docs/pre-go-live-polish.md`:

```markdown
- **Resolved:** Reusable `Spinner` component extracted from the login form
  (`rust/web/src/components/spinner.rs`); `GamePage`'s `<Transition>` is
  now remounted via a deduped `current_game` memo (game id only), so its
  centered-spinner fallback shows on initial load and game-to-game
  navigation, while seq-only WS/command refetches keep the stale board.
```

```bash
git add rust/web/src/components/spinner.rs rust/web/src/components/mod.rs rust/web/src/app.rs rust/web/style/main.scss docs/pre-go-live-polish.md
git commit -m "feat #33: loading spinner when navigating to a game"
```

---

### Task 2: Disable command input and send button while submitting

**Files:**
- Modify: `rust/web/src/components/game.rs` (GameCommandInput, ~lines 399-411 and 499-516)
- Modify: `docs/pre-go-live-polish.md` (entry "2026-07-11: Command input stays enabled while a command is submitting")

**Interfaces:**
- Consumes: `submit_action` (`ServerAction::<SubmitCommand>`) - `pending()` signal, `value()` of type `Option<Result<Option<String>, ServerFnError>>`.
- Produces: nothing new for other tasks.

- [ ] **Step 1: Disable input and button while pending**

In `rust/web/src/components/game.rs` (GameCommandInput view, ~lines 503-515) change the form to:

```rust
                <form on:submit=on_submit>
                    <input
                        type="text"
                        placeholder="Enter command..."
                        autocomplete="off"
                        autocapitalize="none"
                        spellcheck="false"
                        node_ref=input_ref
                        prop:value=command
                        disabled=move || submit_action.pending().get()
                        on:input=move |ev| command.set(event_target_value(&ev))
                    />
                    <input
                        type="submit"
                        value="Send"
                        disabled=move || submit_action.pending().get()
                    />
                </form>
```

Nothing else is needed for the on-success / on-error behavior: `pending()` returning to `false` re-enables both, the existing success `Effect` (~line 402) already clears the text, and the text signal is never cleared on error so the submitted text stays for correction.

- [ ] **Step 2: Refocus the input after an error**

Disabling the input drops browser focus, and the existing refocus effect only fires on success (`Ok(None)`). Extend it so error outcomes also refocus (keeping the success-only side effects gated):

Replace the effect at ~line 402:

```rust
    // Clear command, refocus input, and trigger re-fetch on successful submit.
    // Local bump makes the own action refetch even if the WS is down; the
    // trigger bump is still needed for the layout header.
    Effect::new(move |_| {
        if let Some(Ok(None)) = submit_action.value().get() {
            command.set(String::new());
            trigger.set_last_update.update(|n| *n += 1);
            crate::websocket_client::bump_game_update(game_update, game_id);
            if let Some(el) = input_ref.get() {
                let _ = el.focus();
            }
        }
    });
```

with:

```rust
    // On any submit outcome, refocus the input (disabling it while pending
    // drops focus). On success additionally clear the text and trigger a
    // re-fetch - the local bump makes the own action refetch even if the WS
    // is down; the trigger bump is still needed for the layout header. On
    // error the text signal is left alone so the user can correct it.
    Effect::new(move |_| {
        let Some(result) = submit_action.value().get() else {
            return;
        };
        if matches!(result, Ok(None)) {
            command.set(String::new());
            trigger.set_last_update.update(|n| *n += 1);
            crate::websocket_client::bump_game_update(game_update, game_id);
        }
        if let Some(el) = input_ref.get() {
            let _ = el.focus();
        }
    });
```

- [ ] **Step 3: Verify (automated)**

Run from `/home/beefsack/Development/brdgme/rust`:

```bash
cargo clippy -p web --all-targets --features ssr -- -D warnings
cargo test -p web --features ssr
```

Expected: no warnings, all tests pass.

- [ ] **Step 4: Verify (manual, dev server)**

1. Submit a valid command: input and Send grey out while pending, then the input is cleared, re-enabled, and focused.
2. Submit an invalid command (e.g. gibberish): while pending both are disabled; on the `Invalid command: ...` error both re-enable, the typed text is still in the input, and the input is focused.
3. Rapid double-press of Send / double-Enter: only one submission goes through (the second press hits a disabled control).

- [ ] **Step 5: Update polish doc and commit**

Append to the "2026-07-11: Command input stays enabled while a command is submitting" entry in `docs/pre-go-live-polish.md`:

```markdown
- **Resolved:** Input and Send button now `disabled` while
  `submit_action.pending()`; the submit-outcome effect refocuses the input
  on both success and error (disabling drops focus), still clearing the
  text only on success so errors keep the submitted text for correction.
```

```bash
git add rust/web/src/components/game.rs docs/pre-go-live-polish.md
git commit -m "fix #33: disable command input and send button while submitting"
```

---

### Task 3: Autocomplete click refocuses the command input

**Files:**
- Modify: `rust/web/src/components/game.rs` (GameCommandInput `make_link`, ~lines 459-468)
- Modify: `docs/pre-go-live-polish.md` (entry "2026-07-11: Autocomplete click doesn't focus the command input")

**Interfaces:**
- Consumes: `input_ref` (`NodeRef<leptos::html::Input>`, already in scope; `NodeRef` is `Copy` so the click closure can capture it).
- Produces: nothing new.

- [ ] **Step 1: Focus the input in the suggestion click handler**

In `rust/web/src/components/game.rs`, inside `make_link` (~line 459), change the `on_click` closure from:

```rust
                                    let on_click = move |ev: leptos::ev::MouseEvent| {
                                        ev.prevent_default();
                                        let current = command.get_untracked();
                                        let prefix = word_prefix(&current);
                                        command.set(format!("{}{} ", prefix, value2));
                                    };
```

to:

```rust
                                    let on_click = move |ev: leptos::ev::MouseEvent| {
                                        ev.prevent_default();
                                        let current = command.get_untracked();
                                        let prefix = word_prefix(&current);
                                        command.set(format!("{}{} ", prefix, value2));
                                        // The click focuses the <a>, so the
                                        // type-anywhere guard won't help here -
                                        // refocus the input directly.
                                        if let Some(el) = input_ref.get_untracked() {
                                            let _ = el.focus();
                                        }
                                    };
```

- [ ] **Step 2: Verify (automated)**

Run from `/home/beefsack/Development/brdgme/rust`:

```bash
cargo clippy -p web --all-targets --features ssr -- -D warnings
cargo test -p web --features ssr
```

Expected: no warnings, all tests pass.

- [ ] **Step 3: Verify (manual, dev server)**

1. In a game where it is your turn, click an autocomplete word above the command input: the word is inserted (existing behavior) AND the caret is in the command input - typing immediately continues the command without clicking.
2. Click a word in a grouped suggestion block (multiple values under one description) and confirm the same.

- [ ] **Step 4: Update polish doc and commit**

Append to the "2026-07-11: Autocomplete click doesn't focus the command input" entry in `docs/pre-go-live-polish.md`:

```markdown
- **Resolved:** The suggestion `on_click` handler now calls
  `input_ref.focus()` after inserting the word, so typing continues in the
  command input immediately.
```

```bash
git add rust/web/src/components/game.rs docs/pre-go-live-polish.md
git commit -m "fix #33: autocomplete click refocuses the command input"
```
