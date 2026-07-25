# Raw findings: web-frontend-email W4 (email/render.rs, email/outbound.rs, theme.rs)

Scope: full read of web/src/email/render.rs (553 LOC), web/src/email/outbound.rs (355 LOC), web/src/theme.rs (465 LOC). Supporting reads: lib/markup/src/html.rs (escaping), email/notify.rs url builders, parse_duration call sites in sweep.rs. The known-deliberate mj-raw/<pre>/font-size structure, concrete hex colours, unthemed plain part, and fallback_html path were not flagged per domain context.

### ensure_email_token has a lost-update race that can invalidate an already-emailed reply address
- severity: minor
- category: correctness
- location: web/src/email/outbound.rs:110
- finding: `ensure_email_token` does SELECT-then-UPDATE with no atomicity. Two concurrent sends for the same `game_players` row (e.g. a turn notification and a reminder sweep firing together) can both observe `email_token IS NULL`, generate different tokens, and each send an email carrying its own `g-{token}@brdg.me` reply address. The second UPDATE overwrites the first token, so replies to the first email's address no longer resolve to the player - the reply is silently dead. Low probability, but the failure mode is silent lost player commands.
- recommendation: Make it atomic: `UPDATE game_players SET email_token = COALESCE(email_token, $1), updated_at = NOW() WHERE id = $2 RETURNING email_token` and return the returned value, dropping the SELECT.

### ensure_email_token returns an unpersisted token for a nonexistent game_player_id
- severity: minor
- category: correctness
- location: web/src/email/outbound.rs:116
- finding: When the SELECT returns no row (slot does not exist), the function falls through, generates a token, runs an UPDATE that matches 0 rows, and returns `Ok(token)` anyway. The caller then puts a reply address into an email whose token exists nowhere in the DB, so inbound replies to it can never match. The row-existence signal (`fetch_optional` returning `None`) is checked for the token but not for existence.
- recommendation: Return an error (or `Ok(None)`) when `row` is `None`; only generate a token when the row exists with a NULL token. The RETURNING form above also fixes this (0 rows returned => error).

### game_emails_sent_total counts failed and unattempted-success sends alike
- severity: minor
- category: correctness
- location: web/src/email/outbound.rs:65
- finding: The `game_emails_sent_total` counter is incremented before the Resend call, so a send that fails (function returns `false`, caller may not mark as sent - cross-ref: sweep.rs mark-as-sent decision keys off this return) still increments the "sent" metric. During a Resend outage the metric shows normal send volume while nothing is delivered, masking exactly the incident the metric exists to surface.
- recommendation: Increment on the `Ok(_)` arm only, and optionally add a `game_emails_failed_total` counter on the `Err` arm. If attempt-counting is intended, rename to `..._attempts_total`.

### List-Unsubscribe-Post: One-Click without an HTTPS URI violates RFC 8058
- severity: minor
- category: correctness
- location: web/src/email/render.rs:235
- finding: `List-Unsubscribe-Post: List-Unsubscribe=One-Click` is emitted while `List-Unsubscribe` contains only a `mailto:` URI. RFC 8058 (which Gmail/Yahoo bulk-sender rules reference) requires the one-click POST target to be an HTTPS URI in the List-Unsubscribe header; a mailto-only header plus the Post header is non-compliant. Best case the Post header is ignored; worst case it counts against bulk-sender compliance checks.
- recommendation: Either drop the `List-Unsubscribe-Post` header until an HTTPS one-click unsubscribe endpoint exists, or add an `<https://brdg.me/unsubscribe?...>` URI alongside the mailto and implement the POST handler.

### mrml parse/render failure is silently swallowed
- severity: minor
- category: quality
- location: web/src/email/render.rs:181
- finding: `mrml::parse(&mjml).ok()` and `.render(...).ok()` discard the error before falling back to `fallback_html`. The fallback itself is deliberate, but there is no log/metric, so if body content ever starts breaking mrml parsing (all game emails degrading to the bare-<pre> fallback), nothing surfaces it. Given the file's own history of a subtle Gmail rendering incident, losing this signal is a real observability gap.
- recommendation: Log at `tracing::warn!` (with the error, not the body) in an `.inspect_err(...)`/match before falling back, and/or increment a fallback counter.

### render_block silently renders malformed markup as empty
- severity: minor
- category: quality
- location: web/src/email/render.rs:72
- finding: `brdgme_markup::from_string(markup).unwrap_or_default()` maps a parse failure to zero nodes, so a malformed board/header/digest line renders as an empty block in both HTML and text with no diagnostic. A game whose renderer emits bad markup would ship emails with a missing board and nothing in the logs. Contrast rules.rs:139, which propagates the same error as `RenderError::Markup`.
- recommendation: Log a warning with the block kind on parse failure (keeping the empty-render fallback so the email still goes out), matching the observability recommendation above.

### URLs interpolated into href attributes without escaping
- severity: nit
- category: correctness
- location: web/src/email/render.rs:154
- finding: `browser_url`/`rules_url` are interpolated raw into `<a href="{url}">` (and into the MJML string). Today both are server-built from `public_base_url()` + a UUID (notify.rs:43-50), so there is no injection path; but the renderer's contract does not require that, and a future caller passing a URL containing `"` or `&` would break the attribute or produce invalid HTML. UNCERTAIN whether any future call site would ever carry user-influenced URLs.
- recommendation: Attribute-escape (`&` -> `&amp;`, `"` -> `&quot;`) the URLs at interpolation, or document the trusted-URL precondition on `EmailContent`.

### parse_duration lives in outbound.rs but is sweep configuration parsing
- severity: nit
- category: consistency
- location: web/src/email/outbound.rs:13
- finding: `parse_duration` is a generic env-var duration parser used only by sweep.rs (5 call sites, all sweep config). It has nothing to do with outbound send plumbing; its placement makes outbound.rs's "single send choke point" module description inaccurate and makes the function harder to find.
- recommendation: Move it to sweep.rs (or a config module) next to its only consumers.

### random_pref_colors hand-rolls Fisher-Yates with modulo over rand
- severity: nit
- category: simplicity
- location: web/src/theme.rs:72
- finding: The manual shuffle via `rand::random::<u32>() as usize % (i + 1)` re-implements what `rand` provides (`SliceRandom::shuffle` / `choose_multiple` or `random_range`). The modulo bias is negligible at n<=8, so this is purely a simplicity/intent issue, not a correctness one.
- recommendation: `use rand::seq::SliceRandom; colors.shuffle(&mut rand::rng());` then truncate (adjust to the rand 0.9 API in use).

## Areas reviewed and found clean

- HTML escaping of interpolated markup content: all header/digest/board/you_can/footer text flows through `brdgme_markup::html`, whose `escape` handles `&`, `<`, `>` on every `TNode::Text` (lib/markup/src/html.rs:20-25); player names arrive as text nodes and are escaped, so user-supplied names cannot inject HTML into the body or break out of the `<mj-raw>` block. Subject goes only to the Resend subject field, never into HTML.
- Email header injection: header values are fixed strings or `<{thread_id}@brdg.me>` where thread_id is a caller-built `game-{uuid}`/`proposal-{uuid}`/`settings-{uuid}`; no user-controlled text reaches header values. `reply_to` is `g-{token}@` with a 32-char alphanumeric token; `to` is the DB-verified primary address. Threading semantics (Message-Id on first, In-Reply-To+References after, none when de-threaded) are correct and well tested (render.rs:447-552).
- Palette resolution: `palette_for_slug` matches slugs via the same `slugify` that `theme_slugs_match_brdgme_color_themes` pins against `THEME_SLUGS`, so email and web resolve identically; NULL/"system"/unknown -> LIGHT is the documented, tested behaviour. `player_for_slot` legacy Amber/BlueGrey mapping matches `slot_from_color_name` and is tested against DRACULA.
- outbound send path: dev no-API-key logging branch, EMAIL_FROM fallback, per-header fold-in, and error-logged `false` return are consistent with auth's send_login_email; return semantics for the sweep mark-as-sent decision are a prior-unit cross-ref. `should_email_recipient` truth table is correct and tested; the sweep-side turn_emails_enabled vs reminder_emails_enabled gate mismatch is a prior unit's finding, not re-reviewed.
- suppress_for_web_presence: correct None/Some split, fail-open documented, DB-backed test covers active/stale/never/no-user.
- fetch_email_recipient SQL: LEFT JOINs give bots/addressless slots `email: None`; COALESCE on turn_emails_enabled handles the no-user row.
- theme.rs registry: THEME_SLUGS/`themes()` sync is test-pinned; `grouped_themes` category order and per-group sorting exhaustively tested; `build_theme_style_css` :root/light + prefers-color-scheme dark + per-slug `[data-theme]` blocks are consistent with the documented cookie contract; CHROME_SOFTENS contrast floor tested across every theme; `player_style_vars` only ever receives fixed slot tokens from `slot_from_color_name` so no CSS injection path; SAMPLE_MARKUP/`SAMPLE_HTML` static and thoroughly asserted.
- parse_duration parsing logic itself (digits-then-unit, saturating multiply, trailing-junk rejection) is correct and tested.

Severity tally: critical 0, major 0, minor 6, nit 3.
