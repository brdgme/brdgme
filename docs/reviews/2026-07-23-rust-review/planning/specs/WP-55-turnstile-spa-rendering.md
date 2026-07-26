# WP-55: Turnstile SPA rendering

**Findings:** wfe F53 (major).
**Decision:** D-16 **OVERRULED** - make `/login` a normal, unrouted navigation
that forces a **full page load**, so Turnstile's automatic rendering just
works. Do **not** call Turnstile's `render()` from an effect.

**Landing order:** **WP-54 first** (it rewrites the same `SidebarMenu` logout
effect); **WP-37 and WP-38 first** (both own `rust/web/src/admin.rs`). See
`planning/landing-order.md` section 6.6.

> **Read the named functions before editing. If one does not match what this
> spec describes, STOP and report rather than improvising.** This code is under
> concurrent edit; no line numbers are cited on purpose. Any line number below
> is approximate - verify before trusting it.

## 1. Problem

- **wfe F53** - the `cf-turnstile` div in `rust/web/src/app.rs::LoginPage`
  relies on Turnstile's *implicit* rendering, which scans the DOM once when
  `api.js` (loaded `async defer` in `app.rs::shell`'s head) executes. Any
  client-side navigation to `/login` inserts that div long after the scan, so
  no widget appears, `app.rs::get_turnstile_response` returns `""`, and
  `auth/server.rs::login` rejects whenever `TURNSTILE_SECRET_KEY` is set. The
  user is stuck until a hard refresh.

## 2. Why it's wrong

**wfe F53 is correct as written** (verified live: `api.js` is in `shell`'s
head; the div is emitted from a `{move || …}` block gated on the `site_key`
`Resource` inside `LoginPage`). Do not revert it. **Its first recommended fix -
an explicit `turnstile.render()` from an Effect/NodeRef - was OVERRULED by
D-16.** Its second ("force a full-page load for /login") is this WP.

## 3. Required end state

Every navigation to `/login` becomes a browser page load. There are exactly
**five** sites; `grep -rn '"/login"' rust/web/src/` returns all of them and
nothing else (confirmed live - no sixth site, and no server-side redirect to
`/login` anywhere in the crate).

### 3a. `rust/web/src/app.rs` - the shared hard-navigation helper

The crate has **no** existing `location().set_href` call, so add one helper
beside `set_theme_client` / `local_data_theme` in `app.rs`, used by 3d-3f:

```rust
/// Assigns `window.location`, forcing a full page load rather than a
/// client-side route change. `/login` needs this: Turnstile's implicit
/// rendering only scans the DOM when `api.js` runs (wfe F53 / D-16). No-op
/// under SSR, where `web_sys::window()` is `None`.
pub(crate) fn hard_navigate(path: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let _ = window.location().set_href(path);
}
```

`web-sys` is a **non-optional** dependency of `rust/web` with the `"Window"`
and `"Location"` features already enabled (`rust/web/Cargo.toml`), so **no
`#[cfg(feature = …)]` and no SSR guard is needed** - the `let Some(…) else` is
the guard, as `set_theme_client` already does it, and all three call sites are
inside `Effect::new`, which is inert during SSR.

### 3b/3c. The two `<A href="/login">` anchors

- `rust/web/src/app.rs::HomePage` - `<A href="/login" attr:class="index-cta">
  "Start a game"</A>` gains `attr:rel="external"`. (`<A>` has no `rel` prop;
  `attr:` spreading onto `<A>` is already proven here by the `attr:class`.)
- `rust/web/src/components/layout.rs::SidebarMenu` - `<A href="/login">
  "Login"</A>`, in the `<div hidden=logged_in>` block, becomes a plain
  `<a href="/login" rel="external">"Login"</a>`. `<A>`'s only extra behaviour
  is `aria-current` marking, irrelevant here.

`rel="external"` is what opts out, and is required for **both** forms:
interception is a **window-level** click listener in `leptos_router` 0.8.14
walking `composed_path()` for any `HtmlAnchorElement`, so a plain `<a>` alone
is not enough; `leptos_router-0.8.14/src/location/mod.rs` splits `rel` on
whitespace and returns early on an `external` token.

### 3d. `rust/web/src/components/layout.rs::SidebarMenu` - post-logout

In the logout `Effect` (after WP-54 has landed, its `Some(Ok(()))` arm),
replace `navigate("/login", NavigateOptions::default());` with
`crate::app::hard_navigate("/login");`, and drop `SidebarMenu`'s now-unused
`let navigate = use_navigate();`. **Keep both file-level `leptos_router`
imports** (`NavigateOptions`, `use_navigate`) - `MainLayout` in the same file
still navigates to `/games/{id}`. WP-54's `Some(Err(_))` arm is untouched.

### 3e. `rust/web/src/settings.rs::SettingsPage` - anonymous redirect

Replace the `navigate("/login", NavigateOptions::default());` in the
`matches!(current_user.get(), Some(Ok(None)))` effect with
`crate::app::hard_navigate("/login");`. Nothing else in this file navigates:
delete the `let navigate = use_navigate();` binding and the whole
`use leptos_router::{NavigateOptions, hooks::use_navigate};` line, and add
`hard_navigate` to the existing `use crate::app::{local_data_theme,
set_theme_client};`. Leave the stale module doc alone (owned elsewhere).

### 3f. `rust/web/src/admin.rs::AdminPage` - anonymous redirect

Same replacement in the `Some(Ok(None))` effect. `AdminPage` currently does
`let navigate = use_navigate(); let navigate2 = navigate.clone();` to feed two
effects; afterwards only the non-admin bounce to `"/"` needs one, so drop the
clone and leave a single binding for it. Keep the function-local
`use leptos_router::{NavigateOptions, hooks::use_navigate};`. Do **not** touch
the `"/"` bounce itself - WP-37 rewrites that effect.

## 4. Non-goals

- **CANCELLED (D-16 overruled):** calling `turnstile.render()` from an
  Effect/NodeRef, explicit-rendering mode, or any `render()` call at all.
- Do **not** make other links external. Only `/login`.
- Do **not** touch the `api.js` `<script>` tag in `app.rs::shell`, the
  `cf-turnstile` div, the `site_key` `Resource`, or `get_turnstile_response` -
  no read evidence says any is wrong once `/login` is a full load.
- Leave every other `use_navigate` site as a SPA navigation (`new_game.rs`,
  `components/game.rs`, `proposals.rs`, `LoginPage`'s post-confirm navigate,
  `admin.rs`'s `"/"` bounce).

## 5. Regression test cases

`app.rs`, `layout.rs` and `admin.rs` have `#[cfg(test)]` modules but they cover
pure helpers only; `settings.rs` has none. The router harness is
`rust/web/tests/ssr_pages.rs` (`build_router`, `make_user`, `login_cookie`,
`get`) - add there, next to `home_page_anonymous`:

- `GET /` anonymous: the body contains `rel="external"` **twice** (the
  `index-cta` link and the sidebar Login link) and `href="/login"` twice.
  Assert substrings, not a full tag literal - attribute order is not guaranteed.
- `GET /` with a `login_cookie`: both are hidden by attribute, not removed, so
  the counts are unchanged.

**Not testable here:** whether the browser really performs a full page load,
and whether the widget then renders, cannot be asserted from an SSR-only test -
there is no wasm/browser harness in `rust/web/tests/`. Manual verification, in
a build with `TURNSTILE_SITE_KEY`/`TURNSTILE_SECRET_KEY` set - in each case the
widget must appear and submitting an email without refreshing must succeed:

1. Logged out, click "Start a game" on `/` - the network tab must show a
   document request, not an XHR.
2. Log in, then log out from the sidebar.
3. Logged out, hit `/settings` and then `/admin` directly.

## 6. Riders

None - wfe F53 is the package's only finding.
