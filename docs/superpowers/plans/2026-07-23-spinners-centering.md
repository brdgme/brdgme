# Loading spinners + game render centering (Implementation Plan)

Research + planning doc only. No source was modified. All paths relative to
repo root. Crate is `rust/web` (Leptos SSR + WASM, Axum backend). SCSS lives at
`rust/web/style/main.scss` (NOT `src/main.scss`).

Two small, independent UX fixes:

- **Spinners:** show the existing three-dot `Spinner` while the create-game,
  restart-game, and start-proposal actions are in flight. Today those buttons
  only `disabled` while pending, with no visual feedback that work is happening.
- **Centering:** vertically center the game board inside `.game-render` so short
  boards sit in the middle of the play area instead of glued to the top, while
  tall boards stay top-aligned and scrollable.

Requirements (verbatim intent):
- R1 Show a loading spinner on the **create game** action (bot-only game; the
  new-game form's submit button).
- R2 Show a loading spinner on the **restart game** action (bot-only restart;
  the same submit button when `?restart=` is present).
- R3 Show a loading spinner on the **start proposal** action (the "Start game"
  button on the pending/invite page).
- R4 Spinners go on create / restart / start ONLY. NOT on "submit command" and
  NOT on "undo" (those are fast, frequent, and a spinner there would be noise).
- R5 Center the game board within `.game-render` ONLY. Do NOT rework the whole
  `.game-main` layout.
- R6 The vertical-height/viewport issue (the play area being taller/shorter than
  expected) is OUT OF SCOPE - the user is investigating that separately. Do not
  touch container heights here.

Resolved decisions (do not re-litigate; these came from the user):
- D1 Spinners on create/restart/start actions ONLY (matches R4). No spinner on
  submit-command or undo.
- D2 Center the board within `.game-render` only; do not restructure `.game-main`
  (matches R5).
- D3 Vertical height issue excluded from this spec (matches R6).
- D4 Reuse the existing `crate::components::Spinner` component - do NOT add a new
  spinner widget. Provide an inline (margin-reset) presentation via CSS only.

---

## 1. Current behaviour

### The Spinner component (`rust/web/src/components/spinner.rs:6`)

```rust
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

- Exported via `rust/web/src/components/mod.rs:9,15` (`pub mod spinner;` +
  `pub use spinner::*;`). Call sites use `crate::components::Spinner`.
- No props. The class is hard-coded to `spinner`.

### Spinner CSS (`rust/web/style/main.scss:30-87`)

```scss
.spinner {
  display: inline-block;
  margin: 100px auto 0;   /* <-- large top margin: the inline gotcha */
  width: 100px;
  text-align: center;
}
.spinner > div { width: 18px; height: 18px; ... }   /* three bouncing dots */
/* ... bounce1/bounce2 delays + sk-bouncedelay keyframes ... */

/* Fill-height centered wrapper for the spinner (game area loading state). */
.game-loading { height: 100%; display: flex; align-items: center; justify-content: center; }
.game-loading .spinner { margin: 0; }   /* <-- existing precedent: scope-based margin reset */
```

- The standalone `.spinner` carries `margin: 100px auto 0` - fine for a
  full-area loading state, far too large next to a button.
- `.game-loading .spinner { margin: 0 }` (`:85-87`) is the established pattern
  for resetting that margin in a specific container. The inline variant (S1)
  follows this exact pattern, scoped to `.form-actions`.

### Existing inline-spinner usage pattern (`rust/web/src/app.rs:580-582`)

```rust
<Show when=move || login_action.pending().get()>
    <crate::components::Spinner/>
</Show>
```

- The login form disables its inputs on `pending()` AND shows a `<Spinner/>`
  inside a `<Show when=pending>`. This is the pattern to copy for R1-R3.
- Other `Spinner` call sites: `app.rs:730,740` (wrapped in `.game-loading`),
  `rules.rs:45`. None are inside `.form-actions`, so the S1 CSS scope is safe.

### Create / restart actions (`rust/web/src/new_game.rs`)

- `create_action` (`:281`): `Action::new(...)` calling
  `crate::proposals::create_proposal(...)`.
- `restart_action` (`:294`): `Action::new(...)` calling
  `restart_game_with_roster(...)`.
- A SINGLE shared submit input renders both actions (`:522-528`):

```rust
<div class="form-actions">
    <input
        type="submit"
        value=if restart.is_some() { "Restart game" } else { "Start game" }
        disabled=move || create_action.pending().get() || restart_action.pending().get()
    />
</div>
```

- NO spinner today. The button just greys out via `disabled`.
- Note: create/restart only hit the (slow) game service when the roster is
  bot-only. With human invitees they create a proposal (fast, DB-only) and the
  effect at `:316-324` navigates to `/invites/{id}`. The spinner is still
  correct in both cases - it shows for the duration of the pending action.

### Start proposal action (`rust/web/src/proposals.rs`)

- `start_action = ServerAction::<StartProposal>::new()` (`:1827`).
- Start button (`:2119-2126`), inside a `.form-actions` div:

```rust
<div class="form-actions">
    <button
        type="button"
        disabled=move || start_action.pending().get()
        on:click=move |_| { start_action.dispatch(StartProposal { proposal_id }); }
    >"Start game"</button>
    " "
    <a ... >"Cancel invite"</a>
</div>
```

- NO spinner today; only `disabled` while pending.

### `.form-actions` (shared button row, `main.scss:716-721`)

```scss
.form-actions { margin-bottom: 1em; display: flex; gap: 0.5em; align-items: center; }
```

- Both the new-game submit (`new_game.rs:522`) and the proposal start button
  (`proposals.rs:2119`) live in a `.form-actions` flex row with
  `align-items: center`. A spinner placed as a sibling of the button will
  vertically center automatically; only the `.spinner` top margin needs
  resetting (S1).

### GameBoard + game-render (`rust/web/src/components/game.rs:18-23`)

```rust
#[component]
pub fn GameBoard(html: String, player_style: String) -> impl IntoView {
    view! {
        <div class="game-render" style=player_style><pre inner_html=html></pre></div>
    }
}
```

### `.game-main` / `.game-render` CSS (`main.scss:274-292`)

```scss
.game-main {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  border-right: 1px solid var(--mk-soften-foreground-90);
  flex-direction: column;
}
.game-main .game-render {
  flex: 100;          /* <-- stretches .game-render to fill the column ... */
  text-align: center;
  width: 100%;
  overflow-y: auto;
}
.game-main .game-render > pre {
  margin: 1em;        /* <-- ...so the <pre> sits at the top, not centered */
}
```

- `.game-main` is a centered column flex container, but `.game-render` has
  `flex: 100`, so it grows to fill the column. The `<pre>` board then sits at
  the top of that stretched box (block flow), not centered.
- `.game-render` is REUSED outside `.game-main`: the rules/index board renderer
  emits `<div class="game-render">...` at `rust/web/src/rules.rs:143` (and is
  asserted in `rules.rs:379,398,419`). All centering CSS MUST therefore be
  scoped to `.game-main .game-render` so the rules/index pages are untouched.
  (The existing rules at `:283`/`:290` are already scoped this way - follow
  suit.)

---

## 2. Implementation units

Each unit is one concern, sized well under the 150k budget. S1 is a prerequisite
for S2 and S3 (they render the spinner that S1 styles). S4 is fully independent
and can land in any order. Every unit ends with `cargo fmt` + `cargo clippy`
green and its own commit.

### S1. Inline spinner variant (CSS only)

- **Goal:** make `<crate::components::Spinner/>` look right next to an action
  button by resetting its large standalone top margin, without touching the
  component or any other spinner usage.
- **Files:** `rust/web/style/main.scss`.
- **Change:** add a scoped margin reset that mirrors the existing
  `.game-loading .spinner { margin: 0 }` precedent (`:85-87`), targeting the
  button row both new spinners live in:

  ```scss
  /* Inline spinner beside action buttons (new-game submit, proposal start).
     Resets the large standalone top margin; .form-actions align-items:center
     handles vertical alignment. */
  .form-actions .spinner {
    margin: 0;
  }
  ```

  - Place it adjacent to the `.form-actions` block (`:716-721`) or next to the
    existing `.game-loading .spinner` reset - either is fine; keep it grouped
    with related rules.
  - Keep `width: 100px` (the three 18px dots stay centered in it). OPTIONAL
    refinement if the footprint looks too wide next to a button: add
    `width: auto;` so the box shrinks to the dots. This is a visual nicety, not
    required for correctness - the dev Lead may apply it if it looks better.
- **Acceptance:** a `<Spinner/>` rendered inside a `.form-actions` row has no
  100px top gap and aligns with the button; the standalone login spinner
  (`app.rs:581`, NOT in `.form-actions`) and the `.game-loading` spinners
  (`app.rs:730,740`) are visually unchanged.
- **Tests:** none (pure CSS). Verified by S2/S3 + the Playwright hard-load smoke
  (zero console errors) per `docs/CODING.md` 11.6.
- **Depends on:** nothing.

### S2. Spinner on create / restart submit (new_game.rs)

- **Goal:** R1 + R2 - show the spinner while the shared create/restart submit
  action is pending.
- **Files:** `rust/web/src/new_game.rs`.
- **Change:** inside the existing `<div class="form-actions">` (`:522-528`),
  add a `<Show>` AFTER the submit `<input>` that renders the spinner when either
  action is pending - using the SAME predicate as the button's `disabled`:

  ```rust
  <div class="form-actions">
      <input
          type="submit"
          value=if restart.is_some() { "Restart game" } else { "Start game" }
          disabled=move || create_action.pending().get() || restart_action.pending().get()
      />
      <Show when=move || create_action.pending().get() || restart_action.pending().get()>
          <crate::components::Spinner/>
      </Show>
  </div>
  ```

  - One spinner covers BOTH actions because there is one shared submit button;
    the `||` predicate matches the existing `disabled` condition exactly so the
    spinner appears iff the button is disabled-by-pending.
  - Do NOT add spinners anywhere else in this form (no per-slot spinners).
- **Acceptance:** clicking "Start game" (create) or "Restart game" greys the
  button AND shows the bouncing-dot spinner beside it until the action resolves;
  on success the existing navigate effects (`:316-324` create, `:326+` restart)
  still fire. No spinner appears on ordinary form edits.
- **Tests:** SSR page test for the new-game route stays 200 / no-panic
  (`rust/web/tests/ssr_pages.rs`); Playwright hard-load smoke (zero console
  errors) per `docs/CODING.md` 11.6.
- **Depends on:** S1.

### S3. Spinner on start-proposal button (proposals.rs)

- **Goal:** R3 - show the spinner while the "Start game" proposal action is
  pending.
- **Files:** `rust/web/src/proposals.rs`.
- **Change:** inside the existing `<div class="form-actions">` (`:2119-2134`),
  add a `<Show>` next to the start `<button>` using the same predicate as its
  `disabled`:

  ```rust
  <div class="form-actions">
      <button
          type="button"
          disabled=move || start_action.pending().get()
          on:click=move |_| { start_action.dispatch(StartProposal { proposal_id }); }
      >"Start game"</button>
      <Show when=move || start_action.pending().get()>
          <crate::components::Spinner/>
      </Show>
      " "
      <a href="#" on:click=...>"Cancel invite"</a>
  </div>
  ```

  - Place the spinner between the button and the "Cancel invite" link (the
    existing `" "` text separator can stay or move - keep the row readable).
  - Only the START action gets a spinner here. Do NOT add spinners to
  add-player, remove, cancel, accept, or decline controls (per D1/R4).
- **Acceptance:** clicking "Start game" greys the button AND shows the spinner
  until `start_action` resolves; the existing navigate-to-game effect on a
  successful start still fires. Cancel/add/remove show no spinner.
- **Tests:** SSR page test `/invites/:id` stays 200 / no-panic
  (`rust/web/tests/ssr_pages.rs`); Playwright hard-load smoke (zero console
  errors) per `docs/CODING.md` 11.6.
- **Depends on:** S1.

### S4. Game render vertical centering (CSS only)

- **Goal:** R5 - center the board `<pre>` vertically within `.game-render` when
  the board is shorter than the play area; keep tall boards top-aligned and
  scrollable. Scope strictly to `.game-main .game-render`.
- **Files:** `rust/web/style/main.scss`.
- **Change:** make `.game-render` a column flex container and give the `<pre>`
  auto margins (verified in a mock by the user):

  ```scss
  .game-main .game-render {
    flex: 100;
    text-align: center;
    width: 100%;
    overflow-y: auto;
    display: flex;            /* NEW */
    flex-direction: column;   /* NEW */
  }
  .game-main .game-render > pre {
    margin: auto;             /* was: margin: 1em */
  }
  ```

  - Modify the two EXISTING rules in place (`:283-288` and `:290-292`). Do not
    add duplicate selectors elsewhere.
  - **Why `margin: auto` and not `justify-content: center`:** with a flex column
    + `overflow-y: auto`, `justify-content: center` pushes an overflowing child's
    top above the scroll origin, clipping it unreachable. `margin: auto` on the
    child centers it when there is free space, and collapses to 0 when the child
    overflows - so tall boards align to the top and scroll normally. This is the
    standard flexbox overflow-centering fix.
  - **Spacing trade-off:** the old `margin: 1em` guaranteed a 1em gutter around
    the board. `margin: auto` absorbs all free space (centering), and collapses
    to 0 when the board is tall, so a tall board can reach the container edge.
    If the original 1em breathing room should be preserved, add `padding: 1em`
    to `.game-main .game-render` (the scroll container) - but note padding-bottom
    on a scroll container can be swallowed in some browsers; the dev Lead should
    eyeball this in the mock and pick whichever looks right. The centering
    behavior itself is the requirement; the exact gutter is a visual call.
  - Horizontal centering: `margin: auto` (left/right) on the `<pre>` flex item
    also centers it horizontally; combined with the existing `text-align: center`
    on `.game-render`, the board text stays centered. No regression expected.
- **Acceptance:** a short board renders vertically centered in the play area; a
  tall board starts at the top and scrolls (no clipped top). The rules/index
  page boards (`rules.rs:143`, NOT under `.game-main`) are visually unchanged.
- **Tests:** none (pure CSS). The SSR game-page test stays green
  (`rust/web/tests/ssr_pages.rs`, which references the inlined
  `<div class="game-render">` markup at `:395`); Playwright hard-load smoke
  (zero console errors) per `docs/CODING.md` 11.6.
- **Depends on:** nothing (independent of S1-S3).

Suggested commit order: S1, then S2 + S3 (may land together), then S4 (any
time). Push deferred to a final cleanup unit per the orchestrate handover rules.

---

## 3. Known issues / gotchas (carry forward to every Lead)

- **SCSS path is `rust/web/style/main.scss`**, not `src/main.scss`.
- **Scope every CSS change.** `.spinner` is reused by the login form and the
  `.game-loading` game-area state; `.game-render` is reused by the rules/index
  board renderer (`rules.rs:143`). S1 scopes to `.form-actions .spinner`; S4
  scopes to `.game-main .game-render`. Do not write unscoped `.spinner` or
  `.game-render` rules.
- **`.spinner` default `margin: 100px auto 0` is too large inline** - that is
  exactly what S1 resets. Do not "fix" it by editing the base `.spinner` rule
  (that would break the standalone loading states); add the scoped reset.
- **Hydration safety** (`docs/hydration.md`, `docs/CODING.md` "Leptos: SSR and
  Hydration"): the `<Show when=pending>` spinner is hydration-safe because
  `Action`/`ServerAction` `.pending()` is `false` during SSR and on first
  hydrate, so the `<Show>` branch is consistently empty on both sides - the same
  pattern already shipped in the login form (`app.rs:580`). Do NOT swap element
  STRUCTURE on async data elsewhere; the spinner `<Show>` only adds/removes a
  leaf node gated on a client-side pending signal. Keep the button itself always
  present (it already is) and toggle only its `disabled` attribute (already the
  case) - never conditionally render the button.
- **One shared submit button in new_game.rs.** There is a single `<input
  type="submit">` for both create and restart (`:522-528`); use the combined
  `create_action.pending() || restart_action.pending()` predicate for BOTH the
  existing `disabled` and the new spinner `<Show>` so they stay in lockstep.
- **Spinners ONLY on create/restart/start (D1/R4).** Resist adding spinners to
  submit-command, undo, add-player, remove, cancel, accept, or decline. Those
  are fast/frequent and a spinner there is noise.
- **Do NOT touch container heights (D3/R6).** The vertical-height/viewport
  concern is being investigated separately by the user. S4 changes flex
  direction + margins only; it must not alter `.game-main`/`.game-render`
  heights or the `flex: 100` sizing.
- **SQLX_OFFLINE=true for clippy/check.** Canonical gates (`docs/DEV.md`):
  `cargo fmt --all -- --check`;
  `cargo clippy -p web --all-targets --features ssr -- -D warnings`;
  `cargo clippy --workspace --exclude web --all-targets -- -D warnings`.
  Target `-p web` only; never run workspace-wide builds (RAM/disk spike).
- **No panics** in components (`docs/CODING.md`): these changes add no
  `Option::unwrap` / `NodeRef::get()` unwraps - keep it that way.
- **Pre-existing DB-test failures** in plain local runs are a known condition
  (backlog #40), not a regression from this work. These units add no DB tests.
- **Org is `brdgme`** (not `beefsack`) for any image/URL references.
