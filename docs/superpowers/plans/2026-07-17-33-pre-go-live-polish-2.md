# Pre-Go-Live UI/UX Polish Batch 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the nine 2026-07-17 jank entries in `docs/pre-go-live-polish.md` (the second #33 batch), each as an independent, individually-committed task on master.

**Architecture:** Nine tasks, one per polish-doc entry, all in `rust/web` (Leptos 0.8 SSR+hydrate app, Axum server). Ordered so CSS-only and low-risk tasks land first; the two tasks that both modify `GameCommandInput` (typed invalid-command errors, then input-survives-updates) are sequenced back to back at the end, with the most delicate task (input clearing, which needs its root cause captured in a test before the fix) last. Root causes below were confirmed by reading the current code, not guessed:

- Entry "sub menu still missing" (Task 1): the 2026-07-11 `SubMenuOpen` wiring and icon button DID ship (`layout.rs` lines 33-46, 81-86), but the CSS rule that re-shows `.header-sub-menu` inside the 60em media block never existed - the base rule's own comment ("see the 60em media block below", `main.scss` line 143-145) promises an override that was never written. Never shipped, not a regression.
- Entry "settings page scrolls the whole page" (Task 2): `.layout .layout-body .content` has `height: 100%` but no `overflow-y`, so tall content overflows the layout and the body scrolls.
- Entry "stale username" (Task 6): `get_settings` returns `user.name` from `get_current_user`, which reads the `SessionUser` cached in the tower-sessions session at login time (`auth/session.rs`); `set_username` updates the DB but never the session, so every later `get_settings` serves the login-time name.
- Entry "command input sometimes clears itself" (Task 9): `game_update` is a single global `RwSignal<Option<(Uuid, u64)>>` (`websocket_client.rs`). `GamePage`'s `seq_for_this_game` memo maps it to `Some(seq)` only when the update is for the open game, `None` otherwise - so an update for a DIFFERENT game flips the memo from `Some(n)` back to `None`, which re-keys `game_data`, refetches, and re-runs the `<Transition>` closure, remounting `GameCommandInput` and resetting its local `command` signal to `""`. Michael's gut feeling (sidebar-game updates) is exactly right, and this also explains "sometimes": it only strikes after the open game has had at least one update in the session. A legitimate update for the open game (e.g. an opponent move) also remounts the input and clears typed text.
- Entry "ELO rating change not shown" (Task 7): `game_players.rating_change` already exists in the schema (`001_initial_schema.sql` line 200), is already selected by `find_game_extended` (`db.rs` line 383), and is already on the `GamePlayer` model (`models/game.rs` line 67) - and the legacy presentation CSS (`.rating-change`/`-up`/`-down`/`-none`, `main.scss` lines 428-442) was already ported. Only the `PlayerViewData` plumbing and the Leptos markup are missing. Legacy presentation (recovered from git history, `web/src/components/game/show.tsx` at `ba975b5^`): `Rating: <rating> (<icon><abs(change)>)` where icon is U+2197 (up, green), U+2198 (down, red), or `-` (zero, blue), rendered only when `rating_change` is non-null.

**Tech Stack:** Rust 2024 edition, Leptos 0.8 (SSR + hydrate) + leptos_router, Axum, sqlx/Postgres, SCSS compiled by cargo-leptos.

## Global Constraints

- ASCII-only source edits: no em dashes, no smart quotes, no ellipsis character. Functional Unicode glyphs go in as Rust escapes (`"\u{2197}"`), matching the existing `"\u{22ee}"` style in `layout.rs`.
- Single-package cargo only: `cargo check -p web --features ssr --no-default-features` etc. Never workspace-wide builds (AGENTS.md resource constraints).
- Light local verification per AGENTS.md: `cargo fmt --all` plus `cargo check -p web --features ssr --no-default-features` per task; push and let CI run the full test/clippy suites. Bare `cargo check -p web` (no feature flags) is known-broken - never use it.
- DB-backed tests fail in a plain local run by design (backlog #40) - do not chase or report those failures. Only run the targeted pure-function tests named in the tasks.
- SCSS is compiled by cargo-leptos at build time - `cargo check` does not validate it. SCSS-only changes get an eyeball review of the diff locally; real validation is the CI build + beta deploy.
- Never start `tilt`/kind on a machine with less than 32GB RAM. Manual verification steps marked "(beta)" are for Michael on the deployed beta after CI/ArgoCD, not local blockers.
- Do not install host packages; all tooling comes from the devenv/nix shell.
- Commits go directly to master, message style `web #33: <what>` (see `git log`), ending with the trailer `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`.
- User-facing branding is always "brdg.me", never "brdgme".

---

### Task 1: Show the header sub menu button below 60em

**Files:**
- Modify: `rust/web/style/main.scss` (the `@media only screen and (max-width: 60em)` block, lines 468-508)

**Interfaces:** none - CSS only. The Rust side (`SubMenuOpen` context, `.header-sub-menu` icon button with `on:click`, `GameMeta`'s `.open`/underlay wiring) already works; only the display override is missing.

- [x] **Step 1: Add the missing display rule**

In `rust/web/style/main.scss`, inside the existing `@media only screen and (max-width: 60em)` block, after the `.game-meta-close-underlay` rule (the one ending `background-color: color-mix(in srgb, var(--mk-background) 63%, transparent);`), add:

```scss
  /* Re-show the header sub menu button here: hidden by default (between
     60em and 80em the game meta panel is still on screen), only useful
     once the panel actually collapses. */
  .layout .layout-header .header-sub-menu {
    display: inline-block;
  }
```

The base rule at the top of the file (`.layout .layout-header .header-sub-menu { display: none; }`) stays as-is - equal specificity, the media-block rule appears later in the file so it wins below 60em.

- [x] **Step 2: Verify**

Eyeball the diff (SCSS is not checked by cargo). Run `cd rust && cargo fmt --all` (no-op expected) to keep the tree clean.

(beta) On a game page below 60em: the U+22EE button appears right-aligned in the title bar and opens the game meta panel; between 60em and 80em it stays hidden.

- [x] **Step 3: Commit**

```bash
git add rust/web/style/main.scss
git commit -m "$(cat <<'EOF'
web #33: show header sub menu button below 60em

The 2026-07-11 SubMenuOpen wiring and icon button shipped, but the CSS
rule re-showing .header-sub-menu inside the 60em media block was never
written - the base display:none rule's comment promised an override that
did not exist, so the button never appeared at any width.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Contain page scrolling to the main content pane

**Files:**
- Modify: `rust/web/style/main.scss` (`.layout` line 109-114, `.layout .layout-body .content` line 185-188)

**Interfaces:** none - CSS only.

This is the standard app-shell pattern the polish entry asks for: a full-viewport (`100dvh`) flex container with `overflow-y: auto` on the main pane only, body never scrolling. Game pages already fit this (their `.game-container` is `height: 100%` with internal `overflow-y: auto` panes, so `.content` never overflows there) - this change makes settings and other tall content pages scroll inside `.content` while the sidebar stays put. The login page is `position: fixed` outside `.layout` and is unaffected; `body { overflow: hidden }` is deliberately NOT set so a shorter-than-form viewport on the login page can still scroll.

- [x] **Step 1: Bound the layout to the viewport and scroll the content pane**

In `rust/web/style/main.scss`, find:

```scss
.layout {
  display: flex;
  width: 100%;
  height: 100%;
  flex-direction: column;
}
```

Replace with:

```scss
.layout {
  display: flex;
  width: 100%;
  /* App-shell: the layout is exactly the viewport, and only .content
     scrolls (below). height: 100% is the fallback for browsers without
     dvh support. */
  height: 100%;
  height: 100dvh;
  flex-direction: column;
}
```

Then find:

```scss
.layout .layout-body .content {
  height: 100%;
  flex: 1;
}
```

Replace with:

```scss
.layout .layout-body .content {
  height: 100%;
  flex: 1;
  overflow-y: auto;
}
```

- [x] **Step 2: Verify**

Eyeball the diff. Run `cd rust && cargo fmt --all` (no-op expected).

(beta) Settings page: only the main content area scrolls, the sidebar stays static, the body has no scrollbar. Game pages: board/logs/meta panes scroll exactly as before.

- [x] **Step 3: Commit**

```bash
git add rust/web/style/main.scss
git commit -m "$(cat <<'EOF'
web #33: contain page scrolling to the main content pane

App-shell pattern: .layout is bounded to 100dvh and .content gets
overflow-y: auto, so tall pages (settings) scroll inside the content
pane instead of scrolling the whole body, sidebar included. Game pages
already fit inside the viewport and are unaffected.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Widen non-game content pages to a centered ~1220px

**Files:**
- Modify: `rust/web/style/main.scss` (new `.content-page` rule; shrink the `.settings` rule, lines 510-519)
- Modify: `rust/web/src/settings.rs` (`SettingsPage`, line 33)
- Modify: `rust/web/src/app.rs` (`HomePage` ~line 251, `GamesPage` ~line 521, `DashboardPage` ~line 706)

**Interfaces:**
- Produces: `.content-page` CSS class - the shared wrapper for every non-game content page. Game pages (`GamePage`) do NOT get it.

- [x] **Step 1: Add the shared class and drop the settings-specific width**

In `rust/web/style/main.scss`, find:

```scss
/* Settings page */
.settings {
  max-width: 40em;
  padding: 0 1em;
}
```

Replace with:

```scss
/* Shared wrapper for non-game content pages: centered, ~1220px max
   (Wikipedia 1220px, MUI lg 1200px - squarely within best practice).
   Wide enough for 3 theme-tile columns (3 x 14em + gaps = 44em = 704px)
   on the settings page. Game pages don't use it. */
.content-page {
  max-width: 1220px;
  margin: 0 auto;
  padding: 0 1em;
  box-sizing: border-box;
}

/* Settings page */
```

(The `.settings input, .settings select { max-width: 100%; }` rule that follows stays unchanged.)

- [x] **Step 2: Apply the class to the non-game pages**

In `rust/web/src/settings.rs`, change:

```rust
            <div class="settings">
```

to:

```rust
            <div class="settings content-page">
```

In `rust/web/src/app.rs`, `HomePage`: wrap the layout children in the new class:

```rust
        <MainLayout>
            <div class="content-page">
                <h1>"Welcome to brdg.me"</h1>
                <p>"Lo-fi board games by email and web."</p>
                <A href="/dashboard">"Go to Dashboard"</A>
            </div>
        </MainLayout>
```

`DashboardPage`: same treatment - wrap everything between `<MainLayout>` and `</MainLayout>` in `<div class="content-page">...</div>`.

`GamesPage` (~line 521): change `<div class="new-game">` to `<div class="new-game content-page">`.

`GamePage` is left alone.

- [x] **Step 3: Verify**

Run: `cd rust && cargo fmt --all && cargo check -p web --features ssr --no-default-features`
Expected: `Finished` with no errors.

(beta) Settings page shows 3 theme tile columns; content is centered with even margins on a wide monitor; game pages unchanged.

- [x] **Step 4: Commit**

```bash
git add rust/web/style/main.scss rust/web/src/settings.rs rust/web/src/app.rs
git commit -m "$(cat <<'EOF'
web #33: widen non-game content pages to centered 1220px

New shared .content-page wrapper (max-width 1220px, centered) applied to
settings, home, dashboard, and new-game pages, replacing the settings
page's 40em cap that could not fit 3 theme columns. Game pages
unaffected.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: Indicate selected theme with a border, not a name highlight

**Files:**
- Modify: `rust/web/src/settings.rs` (`ThemeSection`'s `tile` helper and the System tile, lines 247-276)
- Modify: `rust/web/style/main.scss` (theme tile rules, lines 532-553)

**Interfaces:** none new - moves the existing `selected` class from `.theme-tile-label` to `.theme-tile`.

- [x] **Step 1: Move the class in the Rust view**

In `rust/web/src/settings.rs`, in the `tile` function, change:

```rust
            <div
                class="theme-tile"
                data-theme=slug
                style="background-color: var(--mk-background); color: var(--mk-foreground);"
                on:click=on_click
            >
                <div
                    class="theme-tile-label"
                    class:selected=move || current_theme.get().as_deref() == Some(slug)
                >{name}</div>
```

to:

```rust
            <div
                class="theme-tile"
                class:selected=move || current_theme.get().as_deref() == Some(slug)
                data-theme=slug
                style="background-color: var(--mk-background); color: var(--mk-foreground);"
                on:click=on_click
            >
                <div class="theme-tile-label">{name}</div>
```

And the System tile in the `view!` block, change:

```rust
                <div
                    class="theme-tile"
                    style="background-color: var(--mk-background); color: var(--mk-foreground);"
                    on:click=move |_| select(None, current_theme, current_user, set_theme_action)
                >
                    <div
                        class="theme-tile-label"
                        class:selected=move || current_theme.get().is_none()
                    >"System"</div>
                </div>
```

to:

```rust
                <div
                    class="theme-tile"
                    class:selected=move || current_theme.get().is_none()
                    style="background-color: var(--mk-background); color: var(--mk-foreground);"
                    on:click=move |_| select(None, current_theme, current_user, set_theme_action)
                >
                    <div class="theme-tile-label">"System"</div>
                </div>
```

Also update the `ThemeSection` doc comment line "selected tile's label highlighted like \"your turn\"" to "selected tile outlined with a thicker highlight border".

- [x] **Step 2: Swap the CSS**

In `rust/web/style/main.scss`, find and delete:

```scss
/* Same treatment as "active game / your turn" (.layout-game.my-turn). */
.theme-tile .theme-tile-label.selected {
  background-color: var(--mk-soften-orange-86);
  font-weight: 700;
}
```

And after the `.theme-tile` rule (the one with `border: 1px solid var(--mk-soften-foreground-90);` and `padding: 0.63em;`), add:

```scss
/* Selected theme: thicker border in the highlight colour; padding
   compensates the extra 2px so tiles don't shift. */
.theme-tile.selected {
  border: 3px solid var(--mk-orange);
  padding: calc(0.63em - 2px);
}
```

- [x] **Step 3: Verify**

Run: `cd rust && cargo fmt --all && cargo check -p web --features ssr --no-default-features`
Expected: `Finished` with no errors.

(beta) Clicking a theme tile moves a 3px orange border to that tile; no tile grows/shifts; the theme name is never highlighted.

- [x] **Step 4: Commit**

```bash
git add rust/web/src/settings.rs rust/web/style/main.scss
git commit -m "$(cat <<'EOF'
web #33: indicate selected theme with border, not name highlight

The selected class moves from the tile label to the tile itself, styled
as a 3px highlight-colour border with padding compensation instead of a
background highlight on the theme name.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 5: Theme colour preview as solid swatch blocks

**Files:**
- Modify: `rust/web/src/theme.rs` (`SAMPLE_MARKUP` ~line 202, `sample_html_renders_expected_pieces` test ~line 416)
- Modify: `rust/web/style/main.scss` (`.theme-tile .theme-tile-sample`, line 545)

**Interfaces:**
- `SAMPLE_HTML` keeps its type (`LazyLock<String>`) and consumers (`settings.rs` `ThemeSection`) - only the content changes.

The 10 swatches are the accent colours in `NamedColor::ALL` order (`rust/lib/color/src/palette.rs`): row 1 Red, Green, Blue, Yellow, Purple; row 2 Cyan, Pink, Orange, Brown, Grey. Foreground/Background are excluded (the tile background/text already show them). Each swatch is 5 spaces with only a `bg` tag - no text, no `fg`.

- [x] **Step 1: Update the test first**

In `rust/web/src/theme.rs`, replace the `sample_html_renders_expected_pieces` test with:

```rust
    #[test]
    fn sample_html_renders_expected_pieces() {
        let html = &*SAMPLE_HTML;
        // The 10 accent colours from NamedColor::ALL, in order; Foreground
        // and Background are excluded (the tile itself shows them).
        for slot in [
            "red", "green", "blue", "yellow", "purple", "cyan", "pink", "orange", "brown", "grey",
        ] {
            assert!(html.contains(&format!("mk-bg-{slot}")), "missing bg {slot}");
        }
        assert!(
            !html.contains("mk-bg-foreground") && !html.contains("mk-bg-background"),
            "foreground/background must be excluded"
        );
        assert!(html.contains("     "), "swatches are 5-space blocks");
        assert!(!html.contains("mk-fg-"), "no text/fg in the sample");
        assert!(!html.contains("Green"), "no colour names in the sample");
        assert_eq!(
            html.matches('\n').count(),
            1,
            "exactly two rows separated by one newline"
        );
    }
```

- [x] **Step 2: Run it to confirm it fails**

Run: `cd rust && cargo test -p web --features ssr --lib sample_html_renders_expected_pieces`
Expected: FAIL (the current sample has fg tags, colour names, no yellow/grey, and no newline).

- [x] **Step 3: Replace the sample markup**

In `rust/web/src/theme.rs`, replace the `SAMPLE_MARKUP` constant and its doc comment:

```rust
/// Two rows of five 5-space background swatches - the 10 accent colours in
/// `NamedColor::ALL` order (Foreground/Background excluded; the tile itself
/// already shows them). No text: the swatch blocks read purely as colour.
const SAMPLE_MARKUP: &str = "{{bg red}}     {{/bg}}{{bg green}}     {{/bg}}\
{{bg blue}}     {{/bg}}{{bg yellow}}     {{/bg}}{{bg purple}}     {{/bg}}\n\
{{bg cyan}}     {{/bg}}{{bg pink}}     {{/bg}}{{bg orange}}     {{/bg}}\
{{bg brown}}     {{/bg}}{{bg grey}}     {{/bg}}";
```

Also update the `SAMPLE_HTML` doc comment ("One line of 8 colour chips...") to match: "Two rows of five solid colour swatches (see `SAMPLE_MARKUP`), rendered once via `html_class`/`transform_semantic`; shown on every theme preview tile."

- [x] **Step 4: Make the swatches render packed**

Space-only spans collapse in normal HTML whitespace handling, so the sample container needs `white-space: pre`; `line-height: 1` plus `display: inline-block` on the spans removes the vertical line-box gap between the two rows. In `rust/web/style/main.scss`, replace:

```scss
.theme-tile .theme-tile-sample {
  font-size: 0.8em;
}
```

with:

```scss
.theme-tile .theme-tile-sample {
  font-size: 0.8em;
  /* Swatches are runs of spaces - keep them, and pack the two rows with
     no vertical gap. */
  white-space: pre;
  line-height: 1;
}

.theme-tile .theme-tile-sample span {
  display: inline-block;
}
```

- [x] **Step 5: Run the test to confirm it passes**

Run: `cd rust && cargo test -p web --features ssr --lib sample_html_renders_expected_pieces`
Expected: PASS.

- [x] **Step 6: Verify the crate still checks**

Run: `cd rust && cargo fmt --all && cargo check -p web --features ssr --no-default-features`
Expected: `Finished` with no errors.

(beta) Each theme tile shows two packed rows of five solid colour blocks, no gaps and no text, in the order Red Green Blue Yellow Purple / Cyan Pink Orange Brown Grey.

- [x] **Step 7: Commit**

```bash
git add rust/web/src/theme.rs rust/web/style/main.scss
git commit -m "$(cat <<'EOF'
web #33: theme preview as solid swatch blocks

Replaces the coloured-name chips with 10 text-free 5-space background
swatches in NamedColor::ALL accent order, packed in 2 rows of 5 with no
gaps. Foreground/Background excluded - the tile itself shows them.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 6: Username change must not serve a stale cached value

**Files:**
- Modify: `rust/web/src/db.rs` (new `get_user_name` helper next to `set_user_name`, ~line 1503)
- Modify: `rust/web/src/auth/server.rs` (`get_settings` ~line 507, `set_username` ~line 531)

**Interfaces:**
- Produces: `pub async fn get_user_name(pool: &PgPool, user_id: Uuid) -> Result<String>` in `db.rs` (`#[cfg(feature = "ssr")]`, plain non-macro query like `get_user_theme` so no sqlx offline prep is needed).

**Root cause (read, not guessed):** the session (`SessionUser` in `auth/session.rs`) caches `name` at login; `get_current_user` returns it verbatim, and `get_settings` passes it through as `SettingsData.name`. `set_username` updates only the `users` row. The settings page's `LocalResource` is recreated per mount, so it does refetch on every visit - but the refetch returns the stale session copy. Fix both layers: `get_settings` reads the name from the DB (source of truth - covers other devices/sessions too), and `set_username` refreshes the session copy so `get_current_user` is also correct immediately.

- [x] **Step 1: Add the DB helper**

In `rust/web/src/db.rs`, directly above `set_user_name`, add:

```rust
/// The user's current name straight from the `users` table - the session's
/// cached copy can be stale after a rename. Plain query for the same reason
/// as `get_user_theme`.
#[cfg(feature = "ssr")]
pub async fn get_user_name(pool: &PgPool, user_id: Uuid) -> Result<String> {
    let row: (String,) = sqlx::query_as("SELECT name FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}
```

- [x] **Step 2: Read the fresh name in `get_settings`**

In `rust/web/src/auth/server.rs`, in `get_settings`, change:

```rust
    Ok(SettingsData {
        name: user.name,
        email: user.email,
        pref_colors,
    })
```

to:

```rust
    Ok(SettingsData {
        // From the DB, not the session-cached AuthUser - the session copy
        // is stale after a rename (see set_username's session refresh).
        name: crate::db::get_user_name(&pool, user.id)
            .await
            .map_err(internal("get_settings: load name"))?,
        email: user.email,
        pref_colors,
    })
```

- [x] **Step 3: Refresh the session copy in `set_username`**

In `set_username`, change the success arm:

```rust
    match crate::db::set_user_name(&pool, user.id, &name)
        .await
        .map_err(internal("set_username: update"))?
    {
        true => Ok(None),
        false => Ok(Some("That name is taken".to_string())),
    }
```

to:

```rust
    match crate::db::set_user_name(&pool, user.id, &name)
        .await
        .map_err(internal("set_username: update"))?
    {
        true => {
            // The session caches the name from login time; refresh it so
            // get_current_user (and anything else reading the session) sees
            // the new name immediately. Best-effort: the DB rename already
            // succeeded, and get_settings reads the DB directly anyway.
            let session: Session = extract()
                .await
                .map_err(internal("set_username: extract session"))?;
            if let Some(mut session_user) = get_user_from_session(&session).await {
                session_user.name = name;
                let _ = session.insert(SESSION_USER_KEY, session_user).await;
            }
            Ok(None)
        }
        false => Ok(Some("That name is taken".to_string())),
    }
```

`Session`, `extract`, `get_user_from_session`, and `SESSION_USER_KEY` are already used by `get_current_user` in the same file - add any of them missing from the existing `use` lines rather than fully qualifying.

Note: other concurrently-logged-in sessions of the same user keep their old cached name until their next login, but their settings page now shows the fresh DB value - which is exactly the polish entry's requirement ("the settings form always shows the current value").

- [x] **Step 4: Verify**

Run: `cd rust && cargo fmt --all && cargo check -p web --features ssr --no-default-features`
Expected: `Finished` with no errors. (The `#[sqlx::test]` suites need a DB and fail locally by design - do not run them; CI covers them.)

(beta) Change the username, navigate away, return to settings: the form shows the new name.

- [x] **Step 5: Commit**

```bash
git add rust/web/src/db.rs rust/web/src/auth/server.rs
git commit -m "$(cat <<'EOF'
web #33: fix stale username on return to settings

get_settings served the SessionUser name cached at login; set_username
updated only the DB. Now get_settings reads the name from the users
table (source of truth) and set_username also refreshes the session copy
so get_current_user is immediately correct.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 7: Show the ELO rating change at game end

**Files:**
- Modify: `rust/web/src/game/server_fns.rs` (`PlayerViewData` ~line 56, its construction in `get_game_details` ~line 180)
- Modify: `rust/web/src/components/game.rs` (`PlayerInfo`, lines 138-151)

**Interfaces:**
- Produces: `PlayerViewData.rating_change: Option<i32>` (plumbed from `p.game_player.rating_change`, already selected by `find_game_extended`).

**Legacy presentation to replicate exactly** (recovered from git history: `web/src/components/game/show.tsx` at commit `ba975b5^`, the commit before "repo #31: delete legacy stack (WP1)"): `Rating: <rating> (<icon><abs(change)>)`, e.g. `Rating: 1216 (^16)` where `^` here stands for U+2197. Icon: U+2197 in `.rating-change-up` (green) for positive, U+2198 in `.rating-change-down` (red) for negative, literal `-` in `.rating-change-none` (blue) for zero. The number is always `abs(change)` - no `+`/`-` sign. Shown whenever `rating_change` is non-null (in practice: rated game finished; bot games never get one), no separate is_finished gate. The `.game-meta .rating-change` CSS (font-weight 700 + the three colour classes) already exists in `main.scss` lines 428-442, ported 1:1 from the legacy `game.less`.

- [x] **Step 1: Plumb the field**

In `rust/web/src/game/server_fns.rs`, add to `PlayerViewData`:

```rust
    pub rating: i32,
    /// ELO change applied when the game finished; `None` until then (and
    /// always `None` for unrated/bot games).
    pub rating_change: Option<i32>,
```

And in `get_game_details`'s construction:

```rust
                rating: p.game_type_user.rating,
                rating_change: p.game_player.rating_change,
```

- [x] **Step 2: Render it in `PlayerInfo`**

In `rust/web/src/components/game.rs`, replace `PlayerInfo`:

```rust
#[component]
fn PlayerInfo(player: PlayerViewData) -> impl IntoView {
    // Legacy presentation: "Rating: 1216 (<icon>16)" - icon U+2197 (up,
    // green) / U+2198 (down, red) / "-" (zero, blue), number always the
    // absolute value. Only rendered once a rating change exists.
    let rating_change = player.rating_change.map(|amount| {
        let (class, icon) = match amount {
            a if a > 0 => ("rating-change-up", "\u{2197}"),
            a if a < 0 => ("rating-change-down", "\u{2198}"),
            _ => ("rating-change-none", "-"),
        };
        view! {
            <span>
                " ("
                <span class="rating-change">
                    <span class=class>{icon}</span>
                    {amount.abs()}
                </span>
                ")"
            </span>
        }
    });
    view! {
        <div class="player-info">
            <div class:brdgme-is-turn=player.is_turn>
                <PlayerName name=player.name color=player.color />
            </div>
            <div style="margin-left: 1em;">
                <div>
                    <abbr title="ELO rating" style="cursor: help;">"Rating"</abbr>
                    ": " {player.rating} {rating_change}
                </div>
                <div>"Points: " {player.points}</div>
            </div>
        </div>
    }
}
```

- [x] **Step 3: Verify**

Run: `cd rust && cargo fmt --all && cargo check -p web --features ssr --no-default-features`
Expected: `Finished` with no errors.

(beta) Finish a rated game (or open an already-finished one between humans): each player's meta row reads `Rating: <n> (<arrow><abs change>)` with green/red/blue styling; unfinished and bot games show plain `Rating: <n>`.

- [x] **Step 4: Commit**

```bash
git add rust/web/src/game/server_fns.rs rust/web/src/components/game.rs
git commit -m "$(cat <<'EOF'
web #33: show ELO rating change at game end

Plumbs game_players.rating_change (already stored and queried) through
PlayerViewData and renders it next to the rating in the legacy brdg.me
format: "(<arrow><abs change>)" with the existing rating-change-up/
down/none colour classes.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 8: Typed invalid-command errors instead of raw ServerFnError / HTTP 500

**Files:**
- Modify: `rust/web/src/game/mod.rs` (`ExecuteCommandError` enum line 62, `execute_command`'s `UserError`/`remaining_input` arms ~lines 118-132)
- Modify: `rust/web/src/game/server_fns.rs` (`submit_command`, ~line 196)
- Modify: `rust/web/src/components/game.rs` (`GameCommandInput`'s success effect ~line 372 and `error_msg` ~line 402)

**Interfaces:**
- Produces: `ExecuteCommandError::UserError(String)` variant.
- Produces: `submit_command(game_id: Uuid, command: String) -> Result<Option<String>, ServerFnError>` - `Ok(None)` = success, `Ok(Some(msg))` = the game rejected the command (user input error, render inline), `Err` = transport/server fault. Same expected-rejection pattern as `set_username`. Task 9 builds on the `GameCommandInput` changes made here - do not restructure them there.

**Design note:** the polish entry asks for a 4xx "or ideally a typed/expected server fn error the client renders directly". This repo's established pattern for expected user-input rejections is `Ok(Some(message))` (see `set_username`), which removes both the HTTP 500 and the "error running server function" leakage in one move and needs no custom `ServerFnError` codec; it responds 200 rather than 4xx, which satisfies the entry's core requirement (user input error must not be a server fault). If a literal 4xx is ever required, that is a Leptos custom-error-type follow-up, out of scope here.

- [x] **Step 1: Add the `UserError` variant**

In `rust/web/src/game/mod.rs`, change:

```rust
pub enum ExecuteCommandError {
    #[error("stale state conflict")]
    Conflict,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
```

to:

```rust
pub enum ExecuteCommandError {
    #[error("stale state conflict")]
    Conflict,
    /// The game rejected the command (e.g. "expected buy or done") - user
    /// input error, not a server fault. submit_command renders it inline.
    #[error("{0}")]
    UserError(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
```

- [x] **Step 2: Return it from `execute_command` for game-rejected input**

In `execute_command`, change:

```rust
        Response::UserError { message } => return Err(anyhow::anyhow!("{}", message).into()),
```

to:

```rust
        Response::UserError { message } => return Err(ExecuteCommandError::UserError(message)),
```

and change:

```rust
    if !remaining_input.trim().is_empty() {
        return Err(anyhow::anyhow!("Unexpected input: {}", remaining_input).into());
    }
```

to:

```rust
    if !remaining_input.trim().is_empty() {
        return Err(ExecuteCommandError::UserError(format!(
            "Unexpected input: {}",
            remaining_input.trim()
        )));
    }
```

Leave "Not your turn" / "Game is already finished" / "Game not found" as `Other` - they are race/URL-tampering cases, not command-syntax feedback, and the bot/e-mail paths that also consume `ExecuteCommandError` treat them as failures today. Grep `ExecuteCommandError` consumers in `game/mod.rs` (the bot consumer around lines 289-320 and the undo/concede/restart paths) and confirm none match exhaustively on the enum in a way the new variant breaks (`Conflict` is matched specifically; everything else funnels into generic error handling) - adjust only if the compiler objects.

- [x] **Step 3: Map it in `submit_command`**

In `rust/web/src/game/server_fns.rs`, change `submit_command`'s signature and tail:

```rust
/// Ok(None) = success. Ok(Some(message)) = the game rejected the command -
/// expected user-input feedback rendered inline by the command input (same
/// pattern as set_username), NOT a transport/server error.
#[server(SubmitCommand, "/api")]
pub async fn submit_command(
    game_id: Uuid,
    command: String,
) -> Result<Option<String>, ServerFnError> {
```

and replace the final `super::execute_command(...)` expression with:

```rust
    match super::execute_command(
        &pool,
        &http_client,
        &broadcaster,
        &jetstream,
        game_id,
        position as usize,
        command,
    )
    .await
    {
        Ok(()) => Ok(None),
        Err(crate::game::ExecuteCommandError::UserError(msg)) => Ok(Some(msg)),
        Err(e) => Err(ServerFnError::new(e.to_string())),
    }
```

(`ExecuteCommandError` may need re-exporting or a `use` inside the ssr-gated fn body depending on its current visibility - it is `pub` in `game/mod.rs`.)

- [x] **Step 4: Render it in `GameCommandInput`**

In `rust/web/src/components/game.rs`, the success effect currently matches any `Ok`:

```rust
        if let Some(Ok(_)) = submit_action.value().get() {
```

Change it so only real success clears the input:

```rust
        if let Some(Ok(None)) = submit_action.value().get() {
```

And replace `error_msg`:

```rust
    let error_msg = move || {
        submit_action.value().get().and_then(|r| match r {
            Err(e) => Some(e.to_string()),
            Ok(_) => None,
        })
    };
```

with:

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

The typed text stays in the input on `Ok(Some(_))` because the clear effect no longer fires - matching the entry's expectation that the user can correct and resubmit.

- [x] **Step 5: Verify**

Run: `cd rust && cargo fmt --all && cargo check -p web --features ssr --no-default-features`
Expected: `Finished` with no errors.

(beta) Submit an invalid command (e.g. gibberish, or `buy` out of phase in Acquire): the error area shows `Invalid command: <game message>`, the input keeps the text, and the network tab shows a 200 on `submit_command` - no 500, no "error running server function".

- [x] **Step 6: Commit**

```bash
git add rust/web/src/game/mod.rs rust/web/src/game/server_fns.rs rust/web/src/components/game.rs
git commit -m "$(cat <<'EOF'
web #33: typed invalid-command errors, no more HTTP 500

Game-rejected commands (Response::UserError, trailing unparsed input) get
a dedicated ExecuteCommandError::UserError variant that submit_command
returns as Ok(Some(message)) - the set_username expected-rejection
pattern - rendered as "Invalid command: <msg>" in the command input's
error area. Transport faults show a generic message instead of leaking
raw ServerFnError text, and user input errors no longer produce 500s.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 9: Command input must survive background game updates

**Files:**
- Modify: `rust/web/src/app.rs` (`GamePage`: `seq_for_this_game` memo ~line 741, new `track_game_seq` helper + tests, new hoisted command-text signal)
- Modify: `rust/web/src/components/game.rs` (`GameCommandInput`: read the hoisted signal via context; new `CommandInputText` newtype)

**Interfaces:**
- Consumes: Task 8's `GameCommandInput` shape (`Ok(None)`-gated clear effect).
- Produces: `pub struct CommandInputText(pub RwSignal<String>)` in `components/game.rs`, provided by `GamePage` via context.
- Produces: `fn track_game_seq(prev: Option<(Option<Uuid>, Option<u64>)>, current_id: Option<Uuid>, update: Option<(Uuid, u64)>) -> (Option<Uuid>, Option<u64>)` (private, pure, unit-tested) in `app.rs`.

**Root cause (two layers, both confirmed in code - see Architecture):**
1. `seq_for_this_game` flips `Some(n) -> None` when a WS update arrives for a *different* game, re-keying `game_data` and remounting the whole game view (this is the "sidebar game update clears my input" case Michael observed).
2. Even a legitimate update for the open game remounts `GameCommandInput` (it is created inside the `<Transition>` closure), resetting its local `command` signal.

Fix both: (1) make the memo retain the last seq seen for the open game instead of collapsing to `None`, and (2) hoist the command text into `GamePage` so remounts re-read the typed value (the same hoist-above-the-closure pattern already used for the `logs` resource).

- [x] **Step 1: Reproduce with a failing test (the logic-level reproduction)**

The full manual reproduction needs a running stack and a second actor (see Step 6); the memo bug itself is pure logic, so capture it in a unit test first. In `rust/web/src/app.rs`, add to the existing `mod tests` block:

```rust
    #[test]
    fn track_game_seq_retains_seq_on_other_game_updates() {
        let this_game = Uuid::new_v4();
        let other_game = Uuid::new_v4();
        // An update for this game sets the seq...
        let state = track_game_seq(None, Some(this_game), Some((this_game, 3)));
        assert_eq!(state, (Some(this_game), Some(3)));
        // ...and an update for a DIFFERENT game must keep it (the old memo
        // collapsed to None here, re-keying game_data and remounting the
        // game view mid-typing).
        let state = track_game_seq(Some(state), Some(this_game), Some((other_game, 4)));
        assert_eq!(state, (Some(this_game), Some(3)));
    }

    #[test]
    fn track_game_seq_resets_when_viewed_game_changes() {
        let game_a = Uuid::new_v4();
        let game_b = Uuid::new_v4();
        let state = track_game_seq(None, Some(game_a), Some((game_a, 7)));
        // Navigating to another game must not carry game A's seq over.
        let state = track_game_seq(Some(state), Some(game_b), Some((game_a, 7)));
        assert_eq!(state, (Some(game_b), None));
    }
```

- [x] **Step 2: Run to confirm they fail**

Run: `cd rust && cargo test -p web --features ssr --lib track_game_seq`
Expected: compile error - `track_game_seq` does not exist yet.

- [x] **Step 3: Implement the helper and rewire the memo**

In `rust/web/src/app.rs`, next to `count_my_turn`, add:

```rust
/// State for GamePage's per-game WS-sequence memo: `(viewed game, last seq
/// seen for it)`. Updates for other games keep the previous seq - the old
/// closure returned None for them, which re-keyed the game resource and
/// remounted the game view (clearing the command input mid-typing).
/// Changing the viewed game resets the seq.
fn track_game_seq(
    prev: Option<(Option<Uuid>, Option<u64>)>,
    current_id: Option<Uuid>,
    update: Option<(Uuid, u64)>,
) -> (Option<Uuid>, Option<u64>) {
    let prev_seq = match prev {
        Some((prev_id, seq)) if prev_id == current_id => seq,
        _ => None,
    };
    let seq = match update {
        Some((id, seq)) if Some(id) == current_id => Some(seq),
        _ => prev_seq,
    };
    (current_id, seq)
}
```

In `GamePage`, replace:

```rust
    // Per-game sequence number, isolated from other games' WS updates so this
    // page's resources don't refetch when a different game changes.
    let seq_for_this_game = Memo::new(move |_| {
        let current_id = game_id();
        game_update
            .get()
            .and_then(|(id, seq)| (Some(id) == current_id).then_some(seq))
    });
```

with:

```rust
    // Per-game sequence state, isolated from other games' WS updates so this
    // page's resources don't refetch when a different game changes. Holds
    // (game_id, last seq for it) so an update for another game leaves the
    // memo value unchanged (PartialEq dedupe) instead of flipping to None.
    let seq_for_this_game = Memo::new(move |prev: Option<&(Option<Uuid>, Option<u64>)>| {
        track_game_seq(prev.copied(), game_id(), game_update.get())
    });
```

The memo's value now already contains the game id, so simplify `game_data`'s key from `(game_id(), seq_for_this_game.get())` to:

```rust
    let game_data = Resource::new_blocking(
        move || seq_for_this_game.get(),
        |(id, _)| async move {
            match id {
                Some(id) => get_game_details(id).await,
                None => Err(ServerFnError::new("Invalid Game ID")),
            }
        },
    );
```

The `logs` `LocalResource` closure keeps its `let _ = seq_for_this_game.get();` subscription (now also dedup-stable) and its own `game_id()` read - no change needed there.

- [x] **Step 4: Hoist the command text above the remounting closure**

In `rust/web/src/components/game.rs`, add near the top (after the `use` lines):

```rust
/// The game command input's text, owned by `GamePage` (above the
/// `<Transition>` closure that remounts `GameCommandInput` on every game
/// refetch) so typed input survives background updates. Newtype so the
/// context can't collide with other RwSignal<String> providers.
#[derive(Clone, Copy)]
pub struct CommandInputText(pub RwSignal<String>);
```

In `GameCommandInput`, replace:

```rust
    let (command, set_command) = signal(String::new());
```

with:

```rust
    let command = expect_context::<CommandInputText>().0;
```

and replace every `set_command.set(...)` in the component with `command.set(...)` (three sites: the clear-on-success effect, the `on:input` handler, the suggestion-click handler). `command.get()`/`command.get_untracked()` reads stay as they are (`RwSignal` supports both).

In `rust/web/src/app.rs`, in `GamePage`, right after `provide_context(logs);`, add:

```rust
    // Hoisted for the same reason as `logs` above: the <Transition> closure
    // remounts GameCommandInput on every game_data refetch, and a local
    // signal there would reset typed-but-unsent text to "" each time.
    let command_text = crate::components::game::CommandInputText(RwSignal::new(String::new()));
    provide_context(command_text);

    // Typed text must not leak between games when navigating game-to-game
    // (the route component is reused, so nothing else resets it).
    Effect::new(move |prev: Option<Option<Uuid>>| {
        let id = game_id();
        if let Some(prev_id) = prev
            && prev_id != id
        {
            command_text.0.set(String::new());
        }
        id
    });
```

- [x] **Step 5: Run the tests and check both**

Run: `cd rust && cargo test -p web --features ssr --lib track_game_seq`
Expected: 2 passed.

Run: `cd rust && cargo fmt --all && cargo check -p web --features ssr --no-default-features`
Expected: `Finished` with no errors.

- [ ] **Step 6: Manual reproduction/verification (beta)**

Before-fix reproduction (also the after-fix check), needs two games and a second actor:
1. Create a bot game (game B) and a second game where it is your turn (game A). Open game A and start typing in the command input without submitting.
2. Trigger an update in game B (make your move there in another tab, or have the bot/second account move).
3. Before this fix: game A's input clears the moment game B's WS update arrives (once game A has had at least one update that session). After: the typed text stays.
4. Also verify the open-game case: while typing in a game where another player/bot then moves, the board updates but the typed text survives.
5. Regression check: submit a valid command - input clears and refocuses; navigate game A -> game B - the input is empty (no text leak); undo/concede still refetch.

- [x] **Step 7: Commit**

```bash
git add rust/web/src/app.rs rust/web/src/components/game.rs
git commit -m "$(cat <<'EOF'
web #33: command input survives background game updates

Two confirmed causes: (1) GamePage's per-game WS memo collapsed to None
whenever an update arrived for a DIFFERENT game, re-keying game_data and
remounting the whole game view mid-typing - it now tracks (game_id, last
seq) via the pure track_game_seq helper so other games' updates are
no-ops; (2) GameCommandInput's text lived in a local signal inside the
<Transition> closure, so even legitimate refetches of the open game reset
it - hoisted to GamePage via the CommandInputText context (same pattern
as the logs resource), cleared on game-to-game navigation.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
EOF
)"
```

---

## Self-Review Notes

- Spec coverage: all nine 2026-07-17 entries have a task (entry 1 -> Task 2, entry 2 -> Task 3, entry 3 -> Task 4, entry 4 -> Task 5, entry 5 -> Task 6, entry 6 -> Task 7, entry 7 -> Task 9, entry 8 -> Task 1, entry 9 -> Task 8).
- Task 9 deliberately lands after Task 8: both edit `GameCommandInput`, and Task 9's `Ok(None)` clear-effect assumption comes from Task 8.
- The 4xx-vs-Ok(Some) decision in Task 8 is a documented deviation from the entry's "ideally" wording, matching the repo's `set_username` pattern; flag to Michael if a literal 4xx status is required.
- Zero rating change renders as `(-0)` - byte-for-byte legacy parity (`Math.abs(0)` after a `-` icon). Deliberate; change only if Michael objects.
