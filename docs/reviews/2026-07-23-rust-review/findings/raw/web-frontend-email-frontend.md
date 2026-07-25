# Raw findings: web-frontend-email W5 (components/, app.rs, lib.rs)

Scope: web/src/components/{game,opponent_slot,layout,form,mod,spinner,confirm}.rs, web/src/app.rs, web/src/lib.rs (snapshot line numbers, paths relative to rust/).

### GameMeta mutation actions swallow errors silently
- severity: major
- category: quality
- location: web/src/components/game.rs:56
- finding: The four ServerActions in GameMeta only handle success. The undo effect (lines 56-61) and concede effect (62-67) match `Some(Ok(()))` only; `Some(Err(_))` renders nothing. `bump_bot_action` has no value watcher at all, and the force-delete effect (72-77) also ignores `Err`. If Concede or "Delete game (admin)" fails server-side (auth expiry, transient 500), the user gets zero feedback - the page just does not change, and since the failed mutation produces no WS bump there is no refetch either. This is the same fire-and-forget mutation pattern flagged in prior units, and it contrasts with the components that do it right in the same files (PlayerInfo add_friend at game.rs:263-275, GameCommandInput error_msg at game.rs:562-570).
- recommendation: Render a generic error line per action (mirroring GameCommandInput's "Failed to submit command" pattern) when `action.value()` holds `Err`, at minimum for concede and force-delete which are destructive and confirmed via dialog.

### Turnstile widget likely never renders after client-side navigation to /login
- severity: major
- category: correctness
- location: web/src/app.rs:595
- finding: UNCERTAIN. The `cf-turnstile` div relies on Turnstile's implicit rendering, which scans the DOM when `api.js` executes (loaded once in `shell()` at app.rs:88). On a hard load of /login the div is in the SSR HTML and gets rendered. But on a client-side navigation to /login (e.g. the logout flow in layout.rs:120-124 navigates to "/login"), the div is inserted after the implicit scan already ran, so no widget appears, `get_turnstile_response()` (app.rs:458-468) returns "", and `login()` rejects with "CAPTCHA verification failed" whenever TURNSTILE_SECRET_KEY is set (verified in auth/server.rs:234-237: empty token fails when a secret is configured). The user is stuck unless they hard-refresh. Additionally the div is added reactively only after the `site_key` resource resolves (app.rs:589-598), so even a hard load can insert it post-scan if the resource is not serialized into the SSR HTML.
- recommendation: Call `turnstile.render()` explicitly from an Effect/NodeRef once the div mounts (Cloudflare's documented explicit-rendering mode), or force a full-page load for /login. Verify by logging out in a prod-configured build and attempting login without refresh.

### Presence-ping loop cannot restart after logout -> login in the same session
- severity: minor
- category: correctness
- location: web/src/app.rs:179
- finding: `presence_started` is set true once and never reset. The ping loop breaks when it wakes and finds the user logged out (app.rs:185-187). If a user logs out and the loop exits, then logs back in without a page reload, the Effect at 180-193 sees `presence_started == true` and never spawns a new loop, so the re-authenticated session sends no presence pings until a full reload. (If re-login happens before the next 5-minute wake the old loop happens to survive, which makes this intermittent.) `applied_profile_theme` (app.rs:154-173) has the same one-shot latch: logging in as a different user in the same tab never applies the second user's stored theme.
- recommendation: Reset both flags when `current_user` transitions to logged-out (e.g. clear them in the same Effect's else-branch), or key the latches on the user id instead of a bool.

### GamePage error branch leaks raw ServerFnError text to the user
- severity: minor
- category: quality
- location: web/src/app.rs:762
- finding: `Err(e) => view! { <div class="error">"Error: " {e.to_string()}</div> }` renders the raw `ServerFnError` string for any get_game_details failure. GameCommandInput in the same feature deliberately never does this ("Transport/server fault: never leak the raw ServerFnError text", game.rs:567) and shows a generic message instead. Raw ServerFnError text can include internal detail and is inconsistent UX.
- recommendation: Show a generic "Failed to load game" message (distinguishing only the invalid-ID case if useful) to match the established policy.

### GameMeta inlines confirm dialogs instead of using the shared confirm() helper
- severity: minor
- category: consistency
- location: web/src/components/game.rs:118
- finding: The concede handler (game.rs:118-120) and admin force-delete handler (game.rs:170-172) each inline `web_sys::window().and_then(|w| w.confirm_with_message(...).ok()).unwrap_or(false)`, which is exactly the body of `crate::components::confirm()` (components/confirm.rs:1-5) that proposals.rs already uses in five places. Duplicated logic and an extra `web_sys` call site outside the sanctioned helper.
- recommendation: Replace both inline blocks with `crate::components::confirm(...)`.

### friend_request_count resource recreated on every route change, unlike its siblings
- severity: minor
- category: consistency
- location: web/src/components/layout.rs:135
- finding: `SidebarMenu` creates `friend_request_count` as a local LocalResource. Per the comment at layout.rs:126-129 (and app.rs:120-125), every page wraps its own `<MainLayout>`, so the sidebar remounts on each navigation; `active_games` and `current_user` were hoisted into `App` specifically to avoid the resulting reset-to-None flash and duplicate fetch. `friend_request_count` was not hoisted, so every navigation refires the request and the "(N new)" badge disappears until it resolves, the exact defect the hoisting pattern fixed for the other two. It also never refetches on `last_update`, so an incoming friend request does not update the badge until navigation.
- recommendation: Hoist the resource into `App` and provide it via context like `active_games`/`current_user` (optionally tracking the WS trigger for live badge updates).

### Bot difficulty select can desync from state when bot_names resolves after render
- severity: minor
- category: correctness
- location: web/src/components/opponent_slot.rs:316
- finding: UNCERTAIN. The `<select>`'s `prop:value` closure tracks only `slot()`, while the `<option>` list is a separate reactive closure tracking `bot_names` (opponent_slot.rs:328-346). When the LocalResource resolves after the select first renders (fallback list already shown), the options are re-created; replacing option nodes can reset the DOM selection to the first option ("easy") without re-running the `prop:value` closure, so the visible selection ("easy") diverges from state ("medium") until the user touches the control. Harmless when the server list equals the fallback list, wrong difficulty submitted if it ever differs or reorders.
- recommendation: Track `bot_names` from the `prop:value` closure too (or render the select only once bot_names is available) so value is re-asserted after the option list changes.

### Logout action failure gives no feedback
- severity: minor
- category: quality
- location: web/src/components/layout.rs:120
- finding: The logout effect only handles `is_ok()`; a failed Logout server call leaves the user apparently still logged in with no error shown. Same fire-and-forget pattern as the GameMeta finding above, listed separately because it lives in the layout and affects every page.
- recommendation: Show a transient error (or retry) when `logout_action.value()` is `Err`.

### format_log_time hardcodes en-US locale despite "browser local" intent
- severity: nit
- category: quality
- location: web/src/components/game.rs:303
- finding: The comment says timestamps format "in the browser's local time zone via Date.toLocaleString", but the call is `date.to_locale_string("en-US", ...)`. Time zone is local, but date wording/order is forced to US English for all users.
- recommendation: Pass `undefined` (js_sys: `to_locale_string(&JsValue::UNDEFINED.into(), ...)` equivalent) or navigator.language if locale-following output is intended; otherwise fix the comment.

### Click-only anchors without href are keyboard-inaccessible
- severity: nit
- category: quality
- location: web/src/components/layout.rs:166
- finding: The "logout" link (layout.rs:166-171), the "I already have a login code" link (app.rs:603), and the "Logging in as <email>" link (app.rs:623) are `<a>` elements with `on:click` and `style="cursor:pointer"` but no href/tabindex/role, so they cannot be focused or activated by keyboard. (The many `href="#"` action links elsewhere at least remain focusable.)
- recommendation: Use `<button>` styled as a link, or add `href="#"` with prevent_default like the rest of the codebase.

### mod.rs placeholder comment is stale
- severity: nit
- category: quality
- location: web/src/components/mod.rs:1
- finding: "Components module - placeholder for UI components / This will be expanded in later milestones" - the module has been fully populated for some time; the comment is misleading.
- recommendation: Delete the two comment lines.

### sentry snippet escaping does not cover </script>
- severity: nit
- category: quality
- location: web/src/app.rs:55
- finding: `js_string_escape` escapes backslash and double-quote only. A DSN or SENTRY_RELEASE env value containing `</script>` (or a newline) would break out of the inline `<script inner_html=...>` block. Operator-controlled values, so not a security issue in practice, but the escaping claim ("cheap enough to apply unconditionally rather than assume") stops short of the actual injection vector for inline scripts.
- recommendation: Also replace `<` with the escape sequence backslash-u003c (standard JSON-in-script hardening), or serialize via serde_json::to_string.

## Areas reviewed and found clean

- Hydration discipline: the mounted-gate idiom in GameLogs/RecentGameLogs and HomePage, the nested `<Suspense fallback=|| ()>` wrappers (game.rs:189, app.rs:791), and `try_get_untracked` in both raf scroll handlers match the sanctioned patterns in docs/hydration.md; resources and boundaries are created unconditionally and in stable order in every component reviewed; hidden-attribute (not structural) toggling is used consistently for SSR-varying UI (layout header, sidebar login state, HomePage sections).
- GamePage architecture: blocking `Resource::new_blocking` for SSR serialization, `track_game_seq` memo isolating WS refetches per game (with unit tests), Transition-not-Suspense rationale, hoisted logs LocalResource and CommandInputText context, and command-text reset on game navigation are all coherent and correct.
- GameCommandInput: type-anywhere focus handler correctly excludes modifiers, multi-char keys, text-entry elements, and Space-on-focused-controls, and the window listener is removed in on_cleanup (no leak); submit error handling distinguishes game-rejected vs transport errors; suggestion grouping and word_prefix byte slicing are safe (space is ASCII).
- opponent_slot.rs: debounce-with-sequence plus response tagging correctly prevents stale search results/errors; the generate_bot_name race is guarded by re-reading current slot state; taken-id dedupe applied to both suggestions and search results.
- layout.rs next_game_id and app.rs count_my_turn/track_game_seq/theme-slug pinning are unit tested; THEME_BOOT_SCRIPT slug duplication is pinned by test.
- shell()/lib.rs: ssr-gated env reads, hydrate() body-dataset e2e signal, and the Sentry before-send scrubber are sound.

Severity tally: critical 0, major 2, minor 5, nit 4.
