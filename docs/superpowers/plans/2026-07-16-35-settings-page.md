# 35: Settings Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the standalone `/theme` page with a logged-in-only `/settings` page (username, preferred colours, reworked theme picker, email placeholder), plus the D2/D3 username rules (regex + case-insensitive uniqueness, petname defaults at signup, one-off rename migration).

**Architecture:** New `rust/web/src/settings.rs` Leptos page module wrapped in `MainLayout`; new `FormField`/`ColorChip` components; new server fns `get_settings`/`set_username`/`set_pref_colors` in `auth/server.rs` backed by plain (non-macro) sqlx helpers in `db.rs`; a `ThemeCategory` split in `brdgme_color`; one SQL migration for username rules; SCSS additions in `main.scss`.

**Tech Stack:** Rust 2024, Leptos 0.8 (ssr + hydrate features), sqlx 0.8 (Postgres), SCSS via cargo-leptos, `petname = "=3.1.0"` (new, ssr-only).

## Global Constraints

- Canonical player palette, in order: `Green, Red, Blue, Orange, Purple, Brown, Cyan, Pink` (8 colours; matches `brdgme_color::Palette::player_colors()` order).
- Username rules (D2): must match `^[a-zA-Z0-9_-]{1,16}$`, unique case-insensitively (`CREATE UNIQUE INDEX ... ON users (lower(name))`).
- Username help copy, verbatim: `1-16 characters: letters, numbers, - and _. Must be unique.`
- Username taken error copy, verbatim: `That name is taken`.
- Email placeholder copy, verbatim: `Additional email addresses are coming soon.`
- New dependency: `petname = { version = "=3.1.0", default-features = false, features = ["default-words", "default-rng"], optional = true }`, enabled only by the `ssr` feature (`"dep:petname"`). No other new dependencies.
- Migrations live in `rust/web/migrations/`; `008_user_admin.sql` exists (uncommitted); the new migration is `009_username_rules.sql`.
- All NEW sqlx queries must be plain (non-macro) `sqlx::query`/`sqlx::query_as` so no `.sqlx` regeneration is needed (see `db.rs::get_user_theme` convention).
- DB integration tests (`#[sqlx::test]`) currently fail with PoolTimedOut (backlog #40). Do NOT gate task completion on them. Verify with unit tests (filtered) + `SQLX_OFFLINE=true cargo check -p web --features ssr -j 2`.
- Run only ONE cargo build/test at a time, always with `-j 2`.
- The working tree has unrelated uncommitted changes. Commit with explicit file paths only (`git add <path> <path>`), NEVER `git add -A` / `git add .`.
- The `/theme` route and `ThemeSettingsPage` are deleted entirely (no redirect). Anonymous users keep their theme cookie but get no theme UI.
- Save model: username has an explicit Save button (can fail validation); colours and theme apply immediately (fire-and-forget). No page-wide dirty state.

---

### Task 1: Migration 009 - username rules (rename violators, unique index)

**Files:**
- Create: `rust/web/migrations/009_username_rules.sql`

**Interfaces:**
- Consumes: existing `users` table (`id uuid`, `name text`, `created_at`, `updated_at`).
- Produces: unique index `users_name_lower_key` on `lower(name)`; all `users.name` values match `^[a-zA-Z0-9_-]{1,16}$` and are case-insensitively unique. Task 3's `generate_unique_username` and Task 4's `set_user_name` rely on this index existing.

Rename strategy for case-insensitive duplicates: the earliest-created holder (tie-break by id) keeps the name; later holders are renamed. Renames use adjective-animal petname-style names from inline word lists (max 6+1+6 = 13 chars, always regex-valid), retried until unique. This is the SQL-side one-off from spec D3; word lists here are intentionally small (pre-beta user count is tiny) and independent of the `petname` crate used at signup (Task 3).

- [ ] **Step 1: Write the migration**

```sql
-- D2/D3 of docs/superpowers/specs/2026-07-11-35-user-settings-design.md:
-- usernames must match ^[a-zA-Z0-9_-]{1,16}$ and be unique
-- case-insensitively. One-off: regenerate any existing names that violate
-- that (invalid chars, too long, or case-insensitive duplicates - the
-- earliest-created holder keeps a duplicated name), then add the unique
-- index. Replacement names are petname-style adjective-animal pairs from
-- small inline word lists (max 13 chars, always charset-valid); signup-time
-- generation uses the petname crate instead (rust/web/src/db.rs).
DO $$
DECLARE
    adjectives text[] := ARRAY[
        'big','tiny','brave','calm','clever','eager','fuzzy','gentle',
        'happy','jolly','keen','lucky','merry','nimble','proud','quick',
        'shiny','sunny','swift','witty'];
    animals text[] := ARRAY[
        'walrus','otter','badger','crane','dingo','eagle','ferret','gecko',
        'heron','ibis','jackal','koala','lemur','marmot','newt','ocelot',
        'panda','quokka','raven','stoat','toucan','urchin','viper','wombat',
        'yak','zebra'];
    u record;
    candidate text;
BEGIN
    FOR u IN
        SELECT us.id FROM users us
        WHERE us.name !~ '^[a-zA-Z0-9_-]{1,16}$'
           OR EXISTS (
               SELECT 1 FROM users other
               WHERE other.id <> us.id
                 AND lower(other.name) = lower(us.name)
                 AND (other.created_at < us.created_at
                      OR (other.created_at = us.created_at AND other.id < us.id))
           )
        ORDER BY us.created_at, us.id
    LOOP
        LOOP
            candidate := adjectives[1 + floor(random() * array_length(adjectives, 1))::int]
                         || '-' ||
                         animals[1 + floor(random() * array_length(animals, 1))::int];
            EXIT WHEN NOT EXISTS (
                SELECT 1 FROM users WHERE lower(name) = lower(candidate)
            );
        END LOOP;
        UPDATE users SET name = candidate, updated_at = NOW() WHERE id = u.id;
    END LOOP;
END $$;

CREATE UNIQUE INDEX users_name_lower_key ON public.users (lower(name));
```

- [ ] **Step 2: Sanity-check the migration compiles at the SQL level**

DB integration tests are broken (backlog #40), so full verification is not possible. Confirm the ssr build (which embeds `migrations/` via `sqlx::migrate!`) still compiles:

Run: `SQLX_OFFLINE=true cargo check -p web --features ssr -j 2`
Expected: `Finished` with no errors.

- [ ] **Step 3: Commit**

```bash
git add rust/web/migrations/009_username_rules.sql
git commit -m "feat: migration for username rules (charset, length, case-insensitive uniqueness)"
```

---

### Task 2: Split ThemeCategory::DeutanProtan into Deutan and Protan

**Files:**
- Modify: `rust/lib/color/src/palette.rs:3168-3243` (enum + `themes()` registry)
- Modify: `rust/web/src/theme.rs:62-90` (`grouped_themes` category list) and its tests (`grouped_themes_category_order_and_sorting`, `rust/web/src/theme.rs:235-344`)
- Modify: `rust/web/src/app.rs:308-317` (`ThemeSettingsPage` heading match - still exists at this point; deleted in Task 6)

**Interfaces:**
- Consumes: `brdgme_color::ThemeCategory`, `themes()`.
- Produces: `ThemeCategory::{Default, Light, Dark, Deutan, Protan, Tritan}` (the `DeutanProtan` variant no longer exists). `grouped_themes()` returns groups in that order. Task 6's picker match arms use exactly these six variants with headings `"Light"`, `"Dark"`, `"Deuteranopia"`, `"Protanopia"`, `"Tritanopia"` (Default: no heading).

- [ ] **Step 1: Update the failing tests first (web theme grouping test)**

In `rust/web/src/theme.rs` tests, replace the `expected_order` list and the `deutan_protan_group` assertions inside `grouped_themes_category_order_and_sorting`:

```rust
        let mut expected_order = vec![
            ThemeCategory::Default,
            ThemeCategory::Light,
            ThemeCategory::Dark,
            ThemeCategory::Deutan,
            ThemeCategory::Protan,
            ThemeCategory::Tritan,
        ];
```

and replace the whole `deutan_protan_group` block (the `let deutan_protan_group = ...` binding and its four `assert!`s plus the tritanopia-exclusion assert) with:

```rust
        let deutan_group = groups
            .iter()
            .find(|(c, _)| *c == ThemeCategory::Deutan)
            .expect("Deutan category must be present")
            .1
            .clone();
        assert!(
            deutan_group
                .iter()
                .any(|(slug, _)| *slug == "brdgme-light-deuteranopia")
        );
        assert!(
            deutan_group
                .iter()
                .any(|(slug, _)| *slug == "brdgme-dark-deuteranopia")
        );
        assert!(
            deutan_group
                .iter()
                .all(|(slug, _)| slug.contains("deuteranopia"))
        );

        let protan_group = groups
            .iter()
            .find(|(c, _)| *c == ThemeCategory::Protan)
            .expect("Protan category must be present")
            .1
            .clone();
        assert!(
            protan_group
                .iter()
                .any(|(slug, _)| *slug == "brdgme-light-protanopia")
        );
        assert!(
            protan_group
                .iter()
                .any(|(slug, _)| *slug == "brdgme-dark-protanopia")
        );
        assert!(
            protan_group
                .iter()
                .all(|(slug, _)| slug.contains("protanopia"))
        );
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `SQLX_OFFLINE=true cargo test -p web --features ssr --lib theme::tests -j 2`
Expected: FAIL to compile with `no variant or associated item named 'Deutan' found for enum 'ThemeCategory'`.

- [ ] **Step 3: Split the enum in brdgme_color**

In `rust/lib/color/src/palette.rs`, replace the `DeutanProtan` variant (and its doc comment) with:

```rust
    /// Deuteranopia-targeted themes. Displayed as "Deuteranopia".
    Deutan,
    /// Protanopia-targeted themes. Displayed as "Protanopia".
    Protan,
```

Update the enum's outer doc comment sentence `none of the five overlap in practice for this theme set (see 'DeutanProtan''s doc comment)` to `none of the six overlap in practice for this theme set`.

In `themes()`, change the `use` line to:

```rust
    use ThemeCategory::{Dark, Default as DefaultCat, Deutan, Light, Protan, Tritan};
```

and retag the four entries:

```rust
        ("brdgme light deuteranopia", Deutan, &LIGHT_DEUTERANOPIA),
        ("brdgme light protanopia", Protan, &LIGHT_PROTANOPIA),
        ("brdgme light tritanopia", Tritan, &LIGHT_TRITANOPIA),
        ("brdgme dark deuteranopia", Deutan, &DARK_DEUTERANOPIA),
        ("brdgme dark protanopia", Protan, &DARK_PROTANOPIA),
        ("brdgme dark tritanopia", Tritan, &DARK_TRITANOPIA),
```

(The two `modus ... tritanopia` entries stay `Tritan`.)

- [ ] **Step 4: Update grouped_themes and the picker heading match**

In `rust/web/src/theme.rs::grouped_themes`, replace the `categories` array (and its doc comment's category list) with:

```rust
    let categories = [
        ThemeCategory::Default,
        ThemeCategory::Light,
        ThemeCategory::Dark,
        ThemeCategory::Deutan,
        ThemeCategory::Protan,
        ThemeCategory::Tritan,
    ];
```

In `rust/web/src/app.rs::ThemeSettingsPage`, replace the heading match arm

```rust
                        brdgme_color::ThemeCategory::DeutanProtan => {
                            Some("Deuteranopia / Protanopia")
                        }
```

with

```rust
                        brdgme_color::ThemeCategory::Deutan => Some("Deuteranopia"),
                        brdgme_color::ThemeCategory::Protan => Some("Protanopia"),
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p brdgme_color -j 2`
Expected: PASS (`test result: ok`).

Then run: `SQLX_OFFLINE=true cargo test -p web --features ssr --lib theme::tests -j 2`
Expected: PASS, including `grouped_themes_category_order_and_sorting`.

- [ ] **Step 6: Commit**

```bash
git add rust/lib/color/src/palette.rs rust/web/src/theme.rs rust/web/src/app.rs
git commit -m "feat: split ThemeCategory::DeutanProtan into Deutan and Protan"
```

---

### Task 3: Theme tile swatch redesign (8 colour chips)

**Files:**
- Modify: `rust/web/src/theme.rs` (`SAMPLE_MARKUP`, `build_sample_html`, delete `sample_players`/`sample_player_style`, tests)
- Modify: `rust/web/src/app.rs:272-295` (`tile()` - drop per-tile player vars)

**Interfaces:**
- Consumes: `brdgme_markup::{from_string, transform_semantic, html_class}` (existing).
- Produces: `theme::SAMPLE_HTML` renders one line of 8 chips (`mk-bg-{slot}` background + `mk-fg-c-{slot}` contrast text, e.g. `<span class="mk-bg-green"><span class="mk-fg-c-green"> Green </span></span>`). `sample_players()` and `sample_player_style()` no longer exist - Task 6's picker must not reference them.

- [ ] **Step 1: Update the sample test to the new expectations**

Replace `sample_html_renders_expected_pieces` in `rust/web/src/theme.rs`:

```rust
    #[test]
    fn sample_html_renders_expected_pieces() {
        let html = &*SAMPLE_HTML;
        for slot in [
            "green", "red", "blue", "orange", "purple", "brown", "cyan", "pink",
        ] {
            assert!(html.contains(&format!("mk-bg-{slot}")), "missing bg {slot}");
            assert!(
                html.contains(&format!("mk-fg-c-{slot}")),
                "missing contrast fg {slot}"
            );
        }
        assert!(html.contains(" Green "), "chip text padded with spaces");
        assert!(!html.contains("<b>"), "no Bold in the sample");
        assert!(!html.contains("&lt;"), "no player names in the sample");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `SQLX_OFFLINE=true cargo test -p web --features ssr --lib theme::tests::sample_html_renders_expected_pieces -j 2`
Expected: FAIL (`missing bg orange` or similar).

- [ ] **Step 3: Rewrite the sample markup and drop the player sample helpers**

In `rust/web/src/theme.rs`:

Delete `sample_players()` (lines 194-202) and `sample_player_style()` (lines 204-208) entirely.

Replace `SAMPLE_MARKUP` and `build_sample_html`:

```rust
/// One chip per palette slot: colour name with a space of padding either
/// side, slot colour as background, contrast colour as text.
const SAMPLE_MARKUP: &str = "{{bg green}}{{fg green | contrast}} Green {{/fg}}{{/bg}} \
{{bg red}}{{fg red | contrast}} Red {{/fg}}{{/bg}} \
{{bg blue}}{{fg blue | contrast}} Blue {{/fg}}{{/bg}} \
{{bg orange}}{{fg orange | contrast}} Orange {{/fg}}{{/bg}} \
{{bg purple}}{{fg purple | contrast}} Purple {{/fg}}{{/bg}} \
{{bg brown}}{{fg brown | contrast}} Brown {{/fg}}{{/bg}} \
{{bg cyan}}{{fg cyan | contrast}} Cyan {{/fg}}{{/bg}} \
{{bg pink}}{{fg pink | contrast}} Pink {{/fg}}{{/bg}}";

fn build_sample_html() -> String {
    let (nodes, _) = brdgme_markup::from_string(SAMPLE_MARKUP).unwrap_or_default();
    let tnodes = brdgme_markup::transform_semantic(&nodes, &[]);
    brdgme_markup::html_class(&tnodes)
}
```

Update `SAMPLE_HTML`'s doc comment to: `/// One line of 8 colour chips (name on its own slot colour, contrast text), rendered once via html_class/transform_semantic; shown on every theme preview tile.`

- [ ] **Step 4: Fix tile() in app.rs to stop using sample_player_style**

In `rust/web/src/app.rs::ThemeSettingsPage::tile`, delete the line `let player_style = crate::theme::sample_player_style();` and replace the tile's `style` attribute with a plain string:

```rust
                style="background-color: var(--mk-background); color: var(--mk-foreground);"
```

(i.e. drop the `format!` wrapper and the `{}`/`player_style` argument.)

- [ ] **Step 5: Run tests to verify they pass**

Run: `SQLX_OFFLINE=true cargo test -p web --features ssr --lib theme::tests -j 2`
Expected: PASS, all theme tests including the rewritten sample test.

- [ ] **Step 6: Commit**

```bash
git add rust/web/src/theme.rs rust/web/src/app.rs
git commit -m "feat: theme tile swatch shows 8 palette colour chips"
```

---

### Task 4: Username backend - validation, petname signup defaults

**Files:**
- Modify: `rust/web/Cargo.toml` (add `petname` optional dep + `"dep:petname"` in `[features] ssr`)
- Modify: `rust/web/src/db.rs` (add `validate_username`, `generate_unique_username`, `PLAYER_COLOR_NAMES`; use them in `create_game_with_users_tx`)
- Modify: `rust/web/src/auth/server.rs` (`confirm_login_inner` uses petname default; update its DB test)
- Test: unit tests in `rust/web/src/db.rs`

**Interfaces:**
- Consumes: migration 009's `users_name_lower_key` index (uniqueness); `petname::petname(2, "-") -> Option<String>`.
- Produces:
  - `pub fn validate_username(name: &str) -> bool` in `db.rs` (NOT `#[cfg(feature = "ssr")]`-gated; pure, shared with Task 5's server fn).
  - `pub async fn generate_unique_username(conn: &mut sqlx::PgConnection) -> Result<String>` in `db.rs` (ssr-gated).
  - `pub const PLAYER_COLOR_NAMES: [&str; 8]` in `rust/web/src/theme.rs` = `["Green", "Red", "Blue", "Orange", "Purple", "Brown", "Cyan", "Pink"]` (theme.rs so the hydrate build can use it; Task 5 validation and Task 7 selects consume it).

- [ ] **Step 1: Add the dependency**

In `rust/web/Cargo.toml`, after the `resend-rs` line add:

```toml
# Default-username generation (D3) - server-only
petname = { version = "=3.1.0", default-features = false, features = ["default-words", "default-rng"], optional = true }
```

In the `[features] ssr = [` list, add `"dep:petname",` alongside the other `dep:` entries.

- [ ] **Step 2: Write failing unit tests**

In `rust/web/src/db.rs`'s `#[cfg(all(test, feature = "ssr"))] mod tests`, add:

```rust
    // --- validate_username ---

    #[test]
    fn validate_username_accepts_valid_names() {
        for name in ["Sam", "big-scary-walrus", "a", "user_1", "ABCDEFGHIJKLMNOP"] {
            assert!(validate_username(name), "{name} should be valid");
        }
    }

    #[test]
    fn validate_username_rejects_invalid_names() {
        for name in ["", "seventeen-letters!", "with space", "émile", "toolongtoolongtoo", "a.b"] {
            assert!(!validate_username(name), "{name} should be invalid");
        }
    }

    #[test]
    fn petname_output_charset_is_username_safe() {
        // Length can exceed 16 (generate_unique_username retries those away);
        // the charset itself must always pass.
        for _ in 0..20 {
            let name = petname::petname(2, "-").expect("petname generates");
            assert!(
                name.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
                "unexpected char in {name}"
            );
        }
    }
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `SQLX_OFFLINE=true cargo test -p web --features ssr --lib db::tests::validate_username -j 2`
Expected: FAIL to compile with `cannot find function 'validate_username'`.

- [ ] **Step 4: Implement validate_username and generate_unique_username**

In `rust/web/src/db.rs`, directly above `normalize_pref_color`, add:

```rust
/// D2 username rules (docs/superpowers/specs/2026-07-11-35-user-settings-design.md):
/// `^[a-zA-Z0-9_-]{1,16}$`. Uniqueness is enforced separately by the
/// `users_name_lower_key` index (migration 009). Pure and ungated so the
/// client-side form and server fns share one definition.
pub fn validate_username(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 16
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Generates a default username: a 2-word petname (e.g. "scary-walrus"),
/// regenerated until it satisfies D2 (length; the crate's charset is already
/// safe) and is case-insensitively unused. Long words make regeneration
/// expected and cheap. The uuid fallback is unreachable in practice but keeps
/// this total. Takes a connection so it can run inside callers' transactions.
#[cfg(feature = "ssr")]
pub async fn generate_unique_username(conn: &mut sqlx::PgConnection) -> Result<String> {
    for _ in 0..100 {
        let Some(candidate) = petname::petname(2, "-") else {
            continue;
        };
        if !validate_username(&candidate) {
            continue;
        }
        let taken: Option<(bool,)> =
            sqlx::query_as("SELECT true FROM users WHERE lower(name) = lower($1)")
                .bind(&candidate)
                .fetch_optional(&mut *conn)
                .await?;
        if taken.is_none() {
            return Ok(candidate);
        }
    }
    Ok(format!(
        "user-{}",
        &Uuid::new_v4().simple().to_string()[..11]
    ))
}
```

In `rust/web/src/theme.rs`, below `THEME_SLUGS`, add:

```rust
/// Canonical palette colour names, in palette order - the values stored in
/// `users.pref_colors`/`game_players.color`. Matches
/// `brdgme_color::Palette::player_colors()` slot order.
pub const PLAYER_COLOR_NAMES: [&str; 8] = [
    "Green", "Red", "Blue", "Orange", "Purple", "Brown", "Cyan", "Pink",
];
```

- [ ] **Step 5: Wire petnames into both signup paths**

In `rust/web/src/db.rs::create_game_with_users_tx` (email-opponent branch, ~line 778-790), replace:

```rust
            let new_user_id = Uuid::new_v4();
            let username = email.split('@').next().unwrap_or("user").to_string();
```

with:

```rust
            let new_user_id = Uuid::new_v4();
            let username = generate_unique_username(&mut *tx).await?;
```

Also in the same function replace the hardcoded palette (~line 821-823):

```rust
    let palette = [
        "Green", "Red", "Blue", "Orange", "Purple", "Brown", "Cyan", "Pink",
    ];
```

with:

```rust
    let palette = crate::theme::PLAYER_COLOR_NAMES;
```

In `rust/web/src/auth/server.rs::confirm_login_inner` (~line 351-352), replace:

```rust
        let new_user_id = Uuid::new_v4();
        let username = email.split('@').next().unwrap_or("user").to_string();
```

with:

```rust
        let new_user_id = Uuid::new_v4();
        let username = crate::db::generate_unique_username(&mut *tx)
            .await
            .map_err(internal("confirm_login: generate username"))?;
```

In the DB test `confirm_login_creates_user_exactly_once` (`rust/web/src/auth/server.rs`), replace:

```rust
        assert_eq!(confirmed.user.name, "brand-new", "username from localpart");
```

with:

```rust
        assert!(
            crate::db::validate_username(&confirmed.user.name),
            "default username satisfies D2: {}",
            confirmed.user.name
        );
        assert_ne!(
            confirmed.user.name, "brand-new",
            "username no longer derived from email localpart"
        );
```

(This test is `#[sqlx::test]` and currently cannot run - backlog #40 - but must be kept correct.)

- [ ] **Step 6: Run tests and check to verify they pass**

Run: `SQLX_OFFLINE=true cargo test -p web --features ssr --lib db::tests::validate_username -j 2`
Expected: PASS (2 tests).

Run: `SQLX_OFFLINE=true cargo test -p web --features ssr --lib db::tests::petname_output_charset_is_username_safe -j 2`
Expected: PASS.

Run: `SQLX_OFFLINE=true cargo check -p web --features ssr -j 2`
Expected: `Finished` with no errors.

- [ ] **Step 7: Commit**

```bash
git add rust/web/Cargo.toml Cargo.lock rust/web/src/db.rs rust/web/src/theme.rs rust/web/src/auth/server.rs
git commit -m "feat: petname default usernames and D2 username validation"
```

(`Cargo.lock` lives at the workspace root `/home/beefsack/Development/brdgme/Cargo.lock` if present, otherwise `rust/Cargo.lock` - `git status` after the build will show which one changed; add that path.)

---

### Task 5: Server fns - get_settings, set_username, set_pref_colors

**Files:**
- Modify: `rust/web/src/db.rs` (add `set_user_name`, `get_user_pref_colors`, `set_user_pref_colors` next to `get_user_theme`/`set_user_theme` at ~line 1430)
- Modify: `rust/web/src/auth/server.rs` (add `SettingsData`, `get_settings`, `set_username`, `set_pref_colors`)
- Test: unit test for pref-colour validation in `rust/web/src/auth/server.rs`

**Interfaces:**
- Consumes: `crate::db::validate_username` (Task 4), `crate::theme::PLAYER_COLOR_NAMES` (Task 4), `normalize_pref_color` (existing, `db.rs`), `get_current_user` (existing).
- Produces (Task 7 consumes exactly these):
  - `pub struct SettingsData { pub name: String, pub email: String, pub pref_colors: Vec<String> }` (Clone, Debug, Serialize, Deserialize) in `auth/server.rs`, re-exported like `AuthUser` via `crate::auth::*`.
  - `#[server(GetSettings, "/api")] pub async fn get_settings() -> Result<SettingsData, ServerFnError>` - `pref_colors` always length 3 (defaults `["Green", "Red", "Blue"]` when unset), legacy names normalized.
  - `#[server(SetUsername, "/api")] pub async fn set_username(name: String) -> Result<Option<String>, ServerFnError>` - `Ok(None)` success; `Ok(Some(msg))` field error (`"That name is taken"` or the format message below).
  - `#[server(SetPrefColors, "/api")] pub async fn set_pref_colors(colors: Vec<String>) -> Result<(), ServerFnError>`. Dispatchable as `ServerAction::<crate::auth::SetPrefColors>` with field `colors`.
  - `db.rs`: `pub async fn set_user_name(pool: &PgPool, user_id: Uuid, name: &str) -> Result<bool>` (false = unique violation), `pub async fn get_user_pref_colors(pool: &PgPool, user_id: Uuid) -> Result<Vec<String>>` (normalized, may be empty), `pub async fn set_user_pref_colors(pool: &PgPool, user_id: Uuid, colors: &[String]) -> Result<()>`, all ssr-gated.
  - `pub fn validate_pref_colors(colors: &[String]) -> bool` in `auth/server.rs` (pure: exactly 3, all in `PLAYER_COLOR_NAMES`, all distinct).

- [ ] **Step 1: Write failing unit test for pref-colour validation**

In `rust/web/src/auth/server.rs`'s `mod tests`, add:

```rust
    #[test]
    fn validate_pref_colors_rules() {
        let ok = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(validate_pref_colors(&ok(&["Green", "Red", "Blue"])));
        assert!(validate_pref_colors(&ok(&["Pink", "Cyan", "Brown"])));
        assert!(!validate_pref_colors(&ok(&["Green", "Red"])), "must be 3");
        assert!(
            !validate_pref_colors(&ok(&["Green", "Green", "Blue"])),
            "must be distinct"
        );
        assert!(
            !validate_pref_colors(&ok(&["Green", "Red", "Amber"])),
            "legacy names are normalized on read, not accepted on write"
        );
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `SQLX_OFFLINE=true cargo test -p web --features ssr --lib auth::server::tests::validate_pref_colors_rules -j 2`
Expected: FAIL to compile with `cannot find function 'validate_pref_colors'`.

- [ ] **Step 3: Add the db.rs helpers**

In `rust/web/src/db.rs`, directly below `set_user_theme` (~line 1447), add (all plain non-macro queries - `users.name` writes and `pref_colors` reads/writes here must not add `.sqlx` cache entries):

```rust
/// Renames a user. Returns `Ok(false)` when the name is already taken
/// case-insensitively (unique violation on `users_name_lower_key`); the
/// caller turns that into a field error. Plain query for the same reason as
/// `get_user_theme`.
#[cfg(feature = "ssr")]
pub async fn set_user_name(pool: &PgPool, user_id: Uuid, name: &str) -> Result<bool> {
    let res = sqlx::query("UPDATE users SET name = $1, updated_at = NOW() WHERE id = $2")
        .bind(name)
        .bind(user_id)
        .execute(pool)
        .await;
    match res {
        Ok(_) => Ok(true),
        Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23505") => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// The user's stored colour preferences, legacy names ("Amber", "BlueGrey")
/// normalized onto the current palette. May be empty (never set) - the
/// settings server fn applies the palette-order default.
#[cfg(feature = "ssr")]
pub async fn get_user_pref_colors(pool: &PgPool, user_id: Uuid) -> Result<Vec<String>> {
    let row: Option<(Vec<String>,)> =
        sqlx::query_as("SELECT pref_colors FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;
    Ok(row
        .map(|(colors,)| colors.iter().map(|c| normalize_pref_color(c)).collect())
        .unwrap_or_default())
}

#[cfg(feature = "ssr")]
pub async fn set_user_pref_colors(pool: &PgPool, user_id: Uuid, colors: &[String]) -> Result<()> {
    sqlx::query("UPDATE users SET pref_colors = $1, updated_at = NOW() WHERE id = $2")
        .bind(colors)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}
```

- [ ] **Step 4: Add the server fns**

In `rust/web/src/auth/server.rs`, below `get_user_theme` (~line 491), add:

```rust
/// Everything the settings page needs in one round trip. `pref_colors` is
/// always exactly 3 entries: stored prefs normalized, or the palette-order
/// default (Green, Red, Blue) when unset - behaviour-neutral since identical
/// prefs resolve by rank with random tiebreak (see db.rs::choose_colors).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsData {
    pub name: String,
    pub email: String,
    pub pref_colors: Vec<String>,
}

#[server(GetSettings, "/api")]
pub async fn get_settings() -> Result<SettingsData, ServerFnError> {
    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let mut pref_colors = crate::db::get_user_pref_colors(&pool, user.id)
        .await
        .map_err(internal("get_settings: load pref colors"))?;
    if pref_colors.len() != 3 {
        pref_colors = vec!["Green".to_string(), "Red".to_string(), "Blue".to_string()];
    }

    Ok(SettingsData {
        name: user.name,
        email: user.email,
        pref_colors,
    })
}

/// Renames the caller. `Ok(None)` on success; `Ok(Some(message))` is a field
/// error to render inline (validation or uniqueness) - not a ServerFnError,
/// so the form can distinguish expected rejections from transport failures.
#[server(SetUsername, "/api")]
pub async fn set_username(name: String) -> Result<Option<String>, ServerFnError> {
    if !crate::db::validate_username(&name) {
        return Ok(Some(
            "1-16 characters: letters, numbers, - and _. Must be unique.".to_string(),
        ));
    }

    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    match crate::db::set_user_name(&pool, user.id, &name)
        .await
        .map_err(internal("set_username: update"))?
    {
        true => Ok(None),
        false => Ok(Some("That name is taken".to_string())),
    }
}

/// Exactly 3 distinct canonical palette colour names, in preference order.
/// Pure so it is unit-testable; `set_pref_colors` is the only caller.
pub fn validate_pref_colors(colors: &[String]) -> bool {
    colors.len() == 3
        && colors
            .iter()
            .all(|c| crate::theme::PLAYER_COLOR_NAMES.contains(&c.as_str()))
        && colors[0] != colors[1]
        && colors[0] != colors[2]
        && colors[1] != colors[2]
}

#[server(SetPrefColors, "/api")]
pub async fn set_pref_colors(colors: Vec<String>) -> Result<(), ServerFnError> {
    if !validate_pref_colors(&colors) {
        return Err(ServerFnError::new("Invalid colour preferences"));
    }

    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    crate::db::set_user_pref_colors(&pool, user.id, &colors)
        .await
        .map_err(internal("set_pref_colors: update"))
}
```

Note: `validate_pref_colors` must NOT be `#[cfg(feature = "ssr")]`-gated (it is pure), but it references only `crate::theme::PLAYER_COLOR_NAMES` which is ungated - fine in both builds.

- [ ] **Step 5: Run tests and check to verify they pass**

Run: `SQLX_OFFLINE=true cargo test -p web --features ssr --lib auth::server::tests::validate_pref_colors_rules -j 2`
Expected: PASS.

Run: `SQLX_OFFLINE=true cargo check -p web --features ssr -j 2`
Expected: `Finished` with no errors.

- [ ] **Step 6: Commit**

```bash
git add rust/web/src/db.rs rust/web/src/auth/server.rs
git commit -m "feat: settings server fns (get_settings, set_username, set_pref_colors)"
```

---

### Task 6: FormField and ColorChip components + form SCSS

**Files:**
- Create: `rust/web/src/components/form.rs`
- Modify: `rust/web/src/components/mod.rs`
- Modify: `rust/web/style/main.scss` (append form + chip styles)

**Interfaces:**
- Consumes: `crate::theme::slot_from_color_name` (existing; maps `"Green"` -> `"green"`, legacy `"Amber"` -> `"orange"` etc.), markup classes `mk-bg-{slot}`/`mk-fg-c-{slot}` from `THEME_STYLE_CSS` (already emitted globally).
- Produces (Task 7 consumes exactly these):
  - `#[component] pub fn FormField(label: String, help: Option<String>, error: Signal<Option<String>>, children: Children) -> impl IntoView` (`label`/`help` are `#[prop(into)]`, `help`/`error` optional).
  - `#[component] pub fn ColorChip(color: Signal<String>) -> impl IntoView` (`#[prop(into)]`; `color` is a canonical colour name like `"Green"`).
  - CSS classes: `.form-field`, `.form-label`, `.form-control`, `.form-help`, `.form-error`, `.form-actions`, `.color-chip`.

- [ ] **Step 1: Create the components**

Write `rust/web/src/components/form.rs`:

```rust
use leptos::prelude::*;

/// The reusable form-row template (see the 2026-07-16 settings spec):
/// bold block label above the control, optional muted help line, optional
/// red error line. CSS lives in main.scss under `.form-*`.
#[component]
pub fn FormField(
    #[prop(into)] label: String,
    #[prop(optional, into)] help: Option<String>,
    #[prop(optional, into)] error: Signal<Option<String>>,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="form-field">
            <label class="form-label">{label}</label>
            <div class="form-control">{children()}</div>
            {help.map(|h| view! { <div class="form-help">{h}</div> })}
            {move || error.get().map(|e| view! { <div class="form-error">{e}</div> })}
        </div>
    }
}

/// A colour swatch: the canonical colour name (e.g. "Green") padded with one
/// space either side, slot colour as background, contrast colour as text -
/// reuses the `mk-bg-*`/`mk-fg-c-*` markup classes so it previews in the
/// live theme.
#[component]
pub fn ColorChip(#[prop(into)] color: Signal<String>) -> impl IntoView {
    view! {
        <span class=move || {
            let slot = crate::theme::slot_from_color_name(&color.get());
            format!("color-chip mk-bg-{slot} mk-fg-c-{slot}")
        }>
            {move || format!(" {} ", color.get())}
        </span>
    }
}
```

In `rust/web/src/components/mod.rs` add:

```rust
pub mod form;
```

and below the existing `pub use layout::*;` add:

```rust
pub use form::*;
```

- [ ] **Step 2: Add the SCSS**

Append to `rust/web/style/main.scss`:

```scss
/* Reusable form template (settings spec 2026-07-16) */
.form-field {
  margin-bottom: 1em;
}

.form-label {
  font-weight: 700;
  display: block;
}

.form-help {
  font-size: 0.8em;
  color: var(--mk-grey);
}

.form-error {
  color: var(--mk-red);
}

.color-chip {
  margin-left: 0.5em;
  white-space: pre;
}
```

(`white-space: pre` preserves the single-space padding either side of the name; `.form-actions` and `.form-control` need no rules yet - they exist as structural hooks.)

- [ ] **Step 3: Verify it compiles**

Run: `SQLX_OFFLINE=true cargo check -p web --features ssr -j 2`
Expected: `Finished` with no errors.

- [ ] **Step 4: Commit**

```bash
git add rust/web/src/components/form.rs rust/web/src/components/mod.rs rust/web/style/main.scss
git commit -m "feat: FormField and ColorChip components with form CSS template"
```

---

### Task 7: Settings page with reworked theme picker; delete /theme

**Files:**
- Create: `rust/web/src/settings.rs`
- Modify: `rust/web/src/lib.rs` (add `pub mod settings;` after `pub mod theme;`)
- Modify: `rust/web/src/app.rs` (route swap; delete `ThemeSettingsPage`; make `set_theme_client`/`local_data_theme` `pub(crate)`)
- Modify: `rust/web/src/components/layout.rs:163` (sidebar link)
- Modify: `rust/web/style/main.scss` (`.settings`, `.theme-category`, `.theme-tiles`, selected-label styles; delete the `.theme-category-heading` flex hack)

**Interfaces:**
- Consumes: `crate::theme::{grouped_themes, SAMPLE_HTML, is_known_slug}`; `brdgme_color::ThemeCategory` (six variants, Task 2); `crate::app::{set_theme_client, local_data_theme}` (made `pub(crate)` here); `crate::auth::SetTheme` (existing); `MainLayout`.
- Produces: `#[component] pub fn SettingsPage() -> impl IntoView` at route `/settings`, containing only the Theme section at this point (Task 8 adds the other sections above it). A `current_theme: RwSignal<Option<String>>` drives `.selected` highlighting (None = System).

- [ ] **Step 1: Make the theme client helpers reusable**

In `rust/web/src/app.rs`, change the visibility of the two helpers (keep bodies and doc comments unchanged):

```rust
pub(crate) fn set_theme_client(slug: Option<&str>) {
```

```rust
pub(crate) fn local_data_theme() -> Option<String> {
```

- [ ] **Step 2: Create settings.rs with the reworked picker**

Write `rust/web/src/settings.rs`:

```rust
//! The /settings page: username, preferred colours, theme picker, email
//! placeholder. Logged-in only - anonymous visitors are sent to /login.
//! See docs/superpowers/specs/2026-07-16-35-settings-page-design.md.

use leptos::prelude::*;
use leptos_router::{NavigateOptions, hooks::use_navigate};

use crate::app::{local_data_theme, set_theme_client};
use crate::components::MainLayout;

#[component]
pub fn SettingsPage() -> impl IntoView {
    let current_user =
        expect_context::<LocalResource<Result<Option<crate::auth::AuthUser>, ServerFnError>>>();

    // Logged-in only: once the user resource resolves to anonymous, bounce
    // to /login. SSR/hydration render normally (resource is None there).
    let navigate = use_navigate();
    Effect::new(move |_| {
        if matches!(current_user.get(), Some(Ok(None))) {
            navigate("/login", NavigateOptions::default());
        }
    });

    view! {
        <MainLayout>
            <div class="settings">
                <h1>"Settings"</h1>
                <ThemeSection/>
            </div>
        </MainLayout>
    }
}

/// The theme picker: one block per category (h3 heading + wrapping tile
/// row), selected tile's label highlighted like "your turn". Applies
/// immediately on click; profile sync is fire-and-forget for logged-in
/// users (same pattern as the old /theme page).
#[component]
fn ThemeSection() -> impl IntoView {
    let current_user =
        expect_context::<LocalResource<Result<Option<crate::auth::AuthUser>, ServerFnError>>>();
    let set_theme_action = ServerAction::<crate::auth::SetTheme>::new();

    // Drives the .selected highlight; None = System. Initialized from the
    // <html data-theme> attribute on hydrate (Effects are inert during SSR,
    // so SSR renders no selection - class-only change, no structural
    // mismatch).
    let current_theme = RwSignal::new(None::<String>);
    Effect::new(move |_| {
        current_theme.set(local_data_theme());
    });

    // Handles are Copy, so this is callable from any number of move
    // closures without Rc.
    fn select(
        slug: Option<String>,
        current_theme: RwSignal<Option<String>>,
        current_user: LocalResource<Result<Option<crate::auth::AuthUser>, ServerFnError>>,
        set_theme_action: ServerAction<crate::auth::SetTheme>,
    ) {
        set_theme_client(slug.as_deref());
        current_theme.set(slug.clone());
        if matches!(current_user.get_untracked(), Some(Ok(Some(_)))) {
            set_theme_action.dispatch(crate::auth::SetTheme { theme: slug });
        }
    }

    fn tile(
        slug: &'static str,
        name: &'static str,
        current_theme: RwSignal<Option<String>>,
        current_user: LocalResource<Result<Option<crate::auth::AuthUser>, ServerFnError>>,
        set_theme_action: ServerAction<crate::auth::SetTheme>,
    ) -> impl IntoView {
        let sample_html = crate::theme::SAMPLE_HTML.clone();
        let on_click =
            move |_| select(Some(slug.to_string()), current_theme, current_user, set_theme_action);
        view! {
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
                <div class="theme-tile-sample" inner_html=sample_html></div>
            </div>
        }
    }

    view! {
        <h2>"Theme"</h2>
        <div class="theme-category">
            <div class="theme-tiles">
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
                {crate::theme::grouped_themes()
                    .into_iter()
                    .filter(|(c, _)| *c == brdgme_color::ThemeCategory::Default)
                    .flat_map(|(_, group)| group)
                    .map(|(slug, name)| tile(slug, name, current_theme, current_user, set_theme_action))
                    .collect_view()}
            </div>
        </div>
        {crate::theme::grouped_themes().into_iter().filter_map(|(category, group)| {
            let heading = match category {
                brdgme_color::ThemeCategory::Default => None,
                brdgme_color::ThemeCategory::Light => Some("Light"),
                brdgme_color::ThemeCategory::Dark => Some("Dark"),
                brdgme_color::ThemeCategory::Deutan => Some("Deuteranopia"),
                brdgme_color::ThemeCategory::Protan => Some("Protanopia"),
                brdgme_color::ThemeCategory::Tritan => Some("Tritanopia"),
            }?;
            Some(view! {
                <div class="theme-category">
                    <h3>{heading}</h3>
                    <div class="theme-tiles">
                        {group.into_iter().map(|(slug, name)| {
                            tile(slug, name, current_theme, current_user, set_theme_action)
                        }).collect_view()}
                    </div>
                </div>
            })
        }).collect_view()}
    }
}
```

In `rust/web/src/lib.rs`, after `pub mod theme;` add:

```rust
pub mod settings;
```

- [ ] **Step 3: Swap the route and delete ThemeSettingsPage**

In `rust/web/src/app.rs`:

Replace the route line

```rust
                <Route path=StaticSegment("theme") view=ThemeSettingsPage/>
```

with

```rust
                <Route path=StaticSegment("settings") view=crate::settings::SettingsPage/>
```

Delete the entire `#[component] fn ThemeSettingsPage() -> impl IntoView { ... }` function (the block from `#[component]` above `fn ThemeSettingsPage` through its closing brace - after Tasks 2/3 it spans roughly lines 250-328).

In `rust/web/src/components/layout.rs`, replace:

```rust
            <div><A href="/theme">"Theme"</A></div>
```

with:

```rust
            <div><A href="/settings">"Settings"</A></div>
```

- [ ] **Step 4: Update the SCSS**

In `rust/web/style/main.scss`, replace the whole `/* Theme picker */` block (from the `/* Theme picker */` comment through the `.theme-category-heading` rule at the end of the file, i.e. the `.theme-grid`, `.theme-tile`, `.theme-tile .theme-tile-label`, `.theme-tile .theme-tile-sample`, `.theme-category-heading` rules) with:

```scss
/* Settings page */
.settings {
  max-width: 40em;
  padding: 0 1em;
}

.settings input,
.settings select {
  max-width: 100%;
}

/* Theme picker */
.theme-category {
  margin-bottom: 1em;
}

.theme-tiles {
  display: flex;
  flex-wrap: wrap;
  gap: 1em;
}

.theme-tile {
  width: 14em;
  border: 1px solid var(--mk-soften-foreground-90);
  border-radius: 0.63em;
  padding: 0.63em;
  cursor: pointer;
}

.theme-tile .theme-tile-label {
  font-weight: 700;
  margin-bottom: 0.4em;
}

/* Same treatment as "active game / your turn" (.layout-game.my-turn). */
.theme-tile .theme-tile-label.selected {
  background-color: var(--mk-soften-orange-86);
  font-weight: 700;
}
```

- [ ] **Step 5: Verify build and tests**

Run: `SQLX_OFFLINE=true cargo check -p web --features ssr -j 2`
Expected: `Finished` with no errors (in particular, no remaining references to `ThemeSettingsPage`, `theme-grid`, or `sample_player_style`).

Run: `SQLX_OFFLINE=true cargo test -p web --features ssr --lib theme::tests -j 2`
Expected: PASS.

Run: `SQLX_OFFLINE=true cargo test -p web --features ssr --lib app::tests -j 2`
Expected: PASS (boot-script slug test unaffected).

- [ ] **Step 6: Commit**

```bash
git add rust/web/src/settings.rs rust/web/src/lib.rs rust/web/src/app.rs rust/web/src/components/layout.rs rust/web/style/main.scss
git commit -m "feat: /settings page with reworked theme picker, delete /theme"
```

---

### Task 8: Username, preferred colours, and email sections

**Files:**
- Modify: `rust/web/src/settings.rs` (add `UsernameSection`, `ColorsSection`, `EmailSection`; wire into `SettingsPage`)

**Interfaces:**
- Consumes: `crate::auth::{get_settings, SettingsData, SetUsername, SetPrefColors}` (Task 5), `crate::components::{FormField, ColorChip}` (Task 6), `crate::theme::PLAYER_COLOR_NAMES` (Task 4).
- Produces: the complete settings page, section order: Username, Preferred colours, Theme, Email addresses.

- [ ] **Step 1: Hoist a shared settings resource and add the sections**

In `rust/web/src/settings.rs`, update `SettingsPage`'s body to load settings once and pass the resource down, with the sections in spec order:

```rust
#[component]
pub fn SettingsPage() -> impl IntoView {
    let current_user =
        expect_context::<LocalResource<Result<Option<crate::auth::AuthUser>, ServerFnError>>>();

    // Logged-in only: once the user resource resolves to anonymous, bounce
    // to /login. SSR/hydration render normally (resource is None there).
    let navigate = use_navigate();
    Effect::new(move |_| {
        if matches!(current_user.get(), Some(Ok(None))) {
            navigate("/login", NavigateOptions::default());
        }
    });

    // One round trip for everything the page prefills (name, email, colour
    // prefs). LocalResource, matching current_user/active_games: fetched on
    // the client after hydration.
    let settings: LocalResource<Result<crate::auth::SettingsData, ServerFnError>> =
        LocalResource::new(crate::auth::get_settings);

    view! {
        <MainLayout>
            <div class="settings">
                <h1>"Settings"</h1>
                <UsernameSection settings=settings/>
                <ColorsSection settings=settings/>
                <ThemeSection/>
                <EmailSection settings=settings/>
            </div>
        </MainLayout>
    }
}
```

Add the three section components (below `SettingsPage`, above `ThemeSection`):

```rust
/// Explicit-save username form. Server-side rejections (format or "That
/// name is taken") come back as Ok(Some(message)) from set_username and
/// render as a field error; transport errors render a generic one.
#[component]
fn UsernameSection(
    settings: LocalResource<Result<crate::auth::SettingsData, ServerFnError>>,
) -> impl IntoView {
    use crate::components::FormField;
    use leptos::html;

    let name_input = NodeRef::<html::Input>::new();
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

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if let Some(el) = name_input.get() {
            save_action.dispatch(el.value());
        }
    };

    view! {
        <h2>"Username"</h2>
        <form on:submit=on_submit>
            <FormField
                label="Username"
                help="1-16 characters: letters, numbers, - and _. Must be unique."
                error=Signal::derive(move || error.get())
            >
                <input
                    type="text"
                    node_ref=name_input
                    pattern="[a-zA-Z0-9_-]{1,16}"
                    required
                    prop:value=move || {
                        settings.get().and_then(|r| r.ok()).map(|s| s.name).unwrap_or_default()
                    }
                />
            </FormField>
            <div class="form-actions">
                <input type="submit" value="Save" disabled=move || save_action.pending().get()/>
                <Show when=move || {
                    save_action.value().get().is_some_and(|r| matches!(r, Ok(None)))
                        && !save_action.pending().get()
                }>
                    <span class="form-help">"Saved."</span>
                </Show>
            </div>
        </form>
    }
}

/// Exactly three ordered selects over the 8-colour palette; picking a colour
/// already used in another box swaps the two, so the trio is always valid.
/// Saves immediately on change (fire-and-forget, like the theme tiles).
#[component]
fn ColorsSection(
    settings: LocalResource<Result<crate::auth::SettingsData, ServerFnError>>,
) -> impl IntoView {
    use crate::components::{ColorChip, FormField};

    let colors = RwSignal::new(vec![
        "Green".to_string(),
        "Red".to_string(),
        "Blue".to_string(),
    ]);
    // Adopt the stored prefs exactly once; after that the signal is the
    // source of truth (get_settings always returns a valid trio).
    let initialized = RwSignal::new(false);
    Effect::new(move |_| {
        if let Some(Ok(s)) = settings.get()
            && !initialized.get_untracked()
        {
            initialized.set(true);
            colors.set(s.pref_colors);
        }
    });

    let save_action = ServerAction::<crate::auth::SetPrefColors>::new();
    let pick = move |i: usize, val: String| {
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

    view! {
        <h2>"Preferred colours"</h2>
        {["1st choice", "2nd choice", "3rd choice"]
            .into_iter()
            .enumerate()
            .map(|(i, label)| {
                view! {
                    <FormField label=label>
                        <select on:change=move |ev| pick(i, event_target_value(&ev))>
                            {crate::theme::PLAYER_COLOR_NAMES
                                .into_iter()
                                .map(|name| {
                                    view! {
                                        <option
                                            value=name
                                            selected=move || {
                                                colors.get().get(i).map(|c| c == name).unwrap_or(false)
                                            }
                                        >{name}</option>
                                    }
                                })
                                .collect_view()}
                        </select>
                        <ColorChip color=Signal::derive(move || {
                            colors.get().get(i).cloned().unwrap_or_default()
                        })/>
                    </FormField>
                }
            })
            .collect_view()}
    }
}

/// Placeholder until #22d (multi-email management) lands: current login
/// email read-only plus a muted coming-soon note.
#[component]
fn EmailSection(
    settings: LocalResource<Result<crate::auth::SettingsData, ServerFnError>>,
) -> impl IntoView {
    use crate::components::FormField;

    view! {
        <h2>"Email addresses"</h2>
        <FormField label="Login email">
            <div>{move || {
                settings.get().and_then(|r| r.ok()).map(|s| s.email).unwrap_or_default()
            }}</div>
        </FormField>
        <div class="form-help">"Additional email addresses are coming soon."</div>
    }
}
```

- [ ] **Step 2: Verify build**

Run: `SQLX_OFFLINE=true cargo check -p web --features ssr -j 2`
Expected: `Finished` with no errors.

- [ ] **Step 3: Run the web unit-test suites touched by this feature**

Run: `SQLX_OFFLINE=true cargo test -p web --features ssr --lib theme::tests db::tests::validate_username auth::server::tests::validate_pref_colors_rules -j 2`
Expected: PASS (all listed filters).

- [ ] **Step 4: Commit**

```bash
git add rust/web/src/settings.rs
git commit -m "feat: username, preferred colours and email sections on /settings"
```

---

### Task 9: Final verification sweep

**Files:**
- Modify: none expected (fix-ups only if the sweep finds issues).

**Interfaces:**
- Consumes: everything above.
- Produces: green build + unit tests across both crates; formatted code.

- [ ] **Step 1: Format**

Run: `cargo fmt -p web -p brdgme_color`
Expected: exits 0. If it reformats files, re-run the Task-relevant test commands below before committing the formatting under the touched paths only:

```bash
git add rust/web/src rust/lib/color/src
git commit -m "style: cargo fmt"
```

(Skip the commit if `git status` shows no changes under those paths.)

- [ ] **Step 2: Full unit verification (one cargo at a time)**

Run: `cargo test -p brdgme_color -j 2`
Expected: PASS.

Run: `SQLX_OFFLINE=true cargo check -p web --features ssr -j 2`
Expected: `Finished`, no warnings introduced by this work.

Run: `SQLX_OFFLINE=true cargo test -p web --features ssr --lib theme:: -j 2`
Expected: PASS.

Run: `SQLX_OFFLINE=true cargo test -p web --features ssr --lib app:: -j 2`
Expected: PASS.

(`#[sqlx::test]` DB tests will fail with PoolTimedOut - backlog #40; that is a known pre-existing failure, not a gate for this plan.)

- [ ] **Step 3: Grep for leftovers**

```bash
grep -rn "ThemeSettingsPage\|DeutanProtan\|sample_player_style\|theme-grid\|theme-category-heading\|/theme\"" /home/beefsack/Development/brdgme/rust/web/src /home/beefsack/Development/brdgme/rust/lib/color/src /home/beefsack/Development/brdgme/rust/web/style
```

Expected: no matches (a `docs/` or comment match is acceptable; code/style matches are not - fix and amend the relevant commit's follow-up).

---

## Self-review notes (already applied)

- Spec coverage: route/nav (T7), page structure + save model (T7/T8), theme picker rework incl. category split, selected highlight, swatch redesign (T2/T3/T7), form template (T6), responsive (`.settings` cap + `max-width:100%` inputs, T7), components/server fns list (T5/T6/T7), CSS adds/deletes (T6/T7), D2 (T1/T4/T5), D3 (T1 migration + T4 signup), D4 already landed (`choose_colors`) - T4 only re-points the palette constant.
- Ambiguities resolved: (1) spec says category headings are `<h2>` but the page also uses `<h2>` per section - categories use `<h3>` under the `<h2>"Theme"` section; (2) migration renames use SQL-inline word lists (the `petname` crate is unavailable in SQL), earliest-created holder keeps a duplicated name; (3) `.selected` styles the label bar (per spec wording), not the whole tile.
- Type consistency: `set_username -> Result<Option<String>, ServerFnError>` used identically in T5 and T8; `SettingsData` fields match between T5 and T8; `PLAYER_COLOR_NAMES` defined in T4 (theme.rs) and consumed in T4/T5/T8; `ThemeCategory::{Deutan, Protan}` names match T2/T7.
