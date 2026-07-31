# Findings part 3 - F-158..F-211 (normalized extraction)

Extraction pass over unit reports 09 (09a/09b/09c), 10, 10b and 11. No re-review,
no new findings. Where a unit report and `00-STATE.md`/`00-HANDOVER.md` conflict,
`00-STATE.md` wins and the conflict is recorded in `## Discrepancies`.

| ID | Severity | Unit | WP | file:line | Summary | Pairing / status notes |
|----|----------|------|----|-----------|---------|------------------------|
| F-158 | High | 09a | - | `rust/web/src/events.rs:33-41` (+1 site) | SSE resolves the viewer once at connect and never re-validates, so a revoked session keeps streaming private events indefinitely. | Discharges obligation 3; concretises F-131. Visibility-staleness half is bounded (~30s `VisibilityCache` TTL) and recorded as acceptable; only session revocation is unbounded. |
| F-159 | Medium | 09a | - | `rust/web/src/events.rs:47-112` (+1 site) | Both SSE tasks exit only on a `tx.send` failure needing a visible event, so disconnected/idle/anonymous viewers leak tasks and NATS subs forever. | Interacts with F-109: `efad81f` deleted WP-36's ws F55 shutdown drain and the SSE replacement reintroduces the same lifecycle family. `sse_connections` gauge counts leaked tasks as live, hiding it. |
| F-160 | Medium | 09a | - | `rust/web/src/events.rs:117-183` (+2 sites) | Unauthenticated public SSE handler skips `VisibilityCache`, subscribes to the `game.>` firehose and has no rate limit - attacker-scaled DB/decode amplification. | Confirmed pattern 2 (hardened sibling ten lines up). Confirms F-94 (no rate-limiting middleware anywhere in `rust/web`; the two doc comments asserting a per-IP limit are false). Compounds with F-159. |
| F-161 | High | 09a | WP-56 | `rust/web/src/email/inbound.rs:164-219` (+4 sites) | WP-56's inbound auth gate is fail-open three independent ways, so the `From` header is not authenticated and Unit 07's DMARC premise is unsound. | **Session's most severe finding** (`00-STATE.md`, `00-HANDOVER.md`); top of the remediation order. Discharges obligation 2 with answer NOT SOUND and **escalates F-129 + F-130 to account takeover** under the condition Unit 07 set. Settings token has no expiry/single-use/rate limit. Report heading states severity as "High, and it escalates F-129 + F-130". |
| F-161a | unstated | 09a | WP-56 | `rust/web/src/email/inbound.rs:719-723` (+4 sites) | `AuthVerdict::Unknown` proceeds on a `warn!` only, and `Unknown` is returned whenever the authserv-id is not exactly `amazonses.com`. | Sub-letter of F-161 (High); no separate severity given. Pipeline is Resend, not SES - a different authserv-id makes the whole gate inert in production. No test against a captured real message; no metric or alert on `Unknown`. |
| F-161b | unstated | 09a | WP-56 | `rust/web/src/email/inbound.rs:213-218` | `Pass` means "not explicitly failed": `failed(dmarc) \|\| (failed(spf) && failed(dkim))` inverts the DMARC rule, so `spf=fail; dkim=none` is accepted. | Sub-letter of F-161 (High). The cleanest row - unconditional forgery derivable from the file alone, no deployment assumption. Also passes `dmarc=none`, `spf=softfail; dkim=none`, and `spf=neutral/none/permerror/temperror`. |
| F-161c | unstated | 09a | WP-56 | `rust/web/src/email/inbound.rs:170-178` | The topmost-header rule defends only against an *added* second `Authentication-Results`; since `Unknown` proceeds, an attacker-supplied sole header is honoured verbatim. | Sub-letter of F-161 (High). Depends on F-161a. |
| F-161d | unstated | 09a | WP-56 | `rust/web/src/email/inbound.rs:1794-1808` | The two tests named for the lenient boundary are decoys - each input carries an independently passing result, so the "nothing authenticated" cases are untested. | Sub-letter of F-161 (High). Cited in `00-STATE.md` as making decoy tests a confirmed *class*; F-151 decoy family crossed with pattern 4f. Third tooth of the sign-off rule. |
| F-162 | Medium | 09a | - | `rust/web/src/email/inbound.rs:992-1060` (+7 sites) | Seven pre-commit transient failures in `handle_invite_reply` return `Done` instead of `Retry`, so svix never redelivers and an authenticated invite response is lost silently. | **Pairs with F-169** - same `RouteOutcome` contract (`inbound.rs:742-750`), different route. `00-STATE.md`: the `RouteOutcome` sweep is SETTLED - F-162 and F-169 are the only two sites, no third route has the defect. |
| F-163 | Low | 09a | - | `rust/web/tests/sse_events.rs:456-457` (+1 site) | The SSE migration's replacement for a deleted default-running regression test is `#[ignore = "takes 32+ seconds"]`, so the timeout property is no longer checked. | Explicitly **NOT pattern 4e** - the original test predates the programme, so no checklist row is falsified. Recorded as the obligation-1 near-miss alongside the single true 4e instance (F-109). |
| F-164 | Low | 09b | - (obligation 4 product) | `rust/web/style/main.scss:1091-1094` | `.friend-request-badge` hardcodes the CSS keyword `orange` instead of a `var(--mk-*)` token, breaking under all 34 themes including colour-blind palettes, with no non-hue cue. | Product of obligation 4, which DISCHARGED F-15 as LATENT (no live violation at the real emitter). Sole exception in 1,095 lines; the file's own rule at `:761-762` forbids it. |
| F-165 | Medium | 09b | T3-B6 (ws F60, `Test? n`) | `rust/web/src/websocket_client.rs:84-101` | The reconnect fix put the global `last_update` refetch bump inside the `ready_state == Closed` guard, so the friend-request badge never refreshes in a healthy tab. | Over-application beyond the checklist row (the row asked only to guard `open()`). No SSE event exists for friend requests. `Test? n`, so no test was owed - does NOT count toward the "Test? y with no test" tally. |
| F-166 | Medium | 09b | `dec967b6` | `rust/web/src/game_info/queries.rs:14-24` (+1 site) | Pattern 2: the `, name DESC` tiebreak was added to one "latest game version" query and not its sibling, so rules links and game creation can pick different versions. | Sweep complete - exactly two `ORDER BY ... LIMIT 1` sites in `rust/web`; the fixed direction is confirmed correct against the operator. Carries a secondary Low maintainability note (three disagreeing definitions of "latest version"). |
| F-167 | Low | 09b | - (obligation 4 product) | `rust/web/src/theme.rs:12-19` | `CHROME_SOFTENS` keeps a dead `(Red, 86)` entry with zero consumers, emitting 72 dead CSS declarations per page, and its doc comment misdescribes the set. | Obligation 4 concluded. Deadness predates the remediation window, so not programme-introduced; obligation 4 simply did not clean it. `chrome_softens_meet_contrast_floor` explicitly NOT a decoy. |
| F-168 | Low | 09b | WP-54 (wfe F61) | `rust/web/tests/ssr_pages.rs:256-266` (+1 site) | Both accessibility regression tests assert absence of the `cursor:pointer` marker rather than presence of `href="#"`, so reverting the fix still passes. | Explicitly NOT a decoy in the F-151 sense - the weakness is inherited from the spec's own acceptance criterion. Filed as "criterion falsifiable in only one direction". Fix verified present at HEAD. |
| F-169 | High | 09b | WP-57 (`65c22edc`) §3b | `rust/web/src/email/inbound.rs:1392-1433` | Pattern 2: the at-least-once `Retry` fix landed on the game and invite routes but not settings; `handle_settings_reply` returns `()`, so transient DB errors silently discard the command. | **Pairs with F-162** - same `RouteOutcome` contract, settings route vs invite route; remediate together. `00-STATE.md`: the `RouteOutcome` sweep is SETTLED - no third route has the defect. |
| F-170 | Medium | 09b | WP-58 (`390dd3b8`) §3a | `rust/web/src/email/render.rs:35-42` (+1 site) | `EmailKind::pref_column()` has zero `src/` callers; the live column mapping is an untested duplicate `match`, and the only test asserting it guards nothing. | Instance of F-153's "documentation-only constant" pattern in `pub fn` form, crossed with the decoy-test class. `00-STATE.md`: F-170 is **NOT extended** by the game-start mail (REFUTED in 09c) - that path reads `turn_emails_enabled` directly. |
| F-171 | Medium | 09b | WP-58 §6 rider row 2 (`Test? y`) | `rust/web/src/email/inbound.rs:1377-1380` | The `List-Unsubscribe*` deletion from `send_rules_reply_response` landed but the promised absence test does not exist; the function has no test at all. | **Fifth confirmed "Test? y with no test"** row (joins F-142, F-148, F-149, F-150) and the most explicit - the row named the assertion. Later rolled into the nine-row tally with WP-60's four (F-176). |
| F-172 | Low | 09b | WP-59 (`f56ff375`) Task 1 | `rust/web/src/email/inbound.rs:135` | The CRLF sanitiser truncates at the first CR/LF where the spec required replacement with a space, so a legally folded `From` parses to `None` and the move is dropped with a 200. | Near-decoy: the covering test exercises only the injection case where truncate and replace agree. The injection half of the criterion IS satisfied. |
| F-173 | Low | 09b | WP-59 (out of mandate) | `rust/web/src/email/inbound.rs:532-545` (+1 site) | Inbound `from_matches_verified_email` normalises via SQL `LOWER()` while every write path uses Rust `canonicalize_email`, so e.g. U+0130 addresses can never match. | **F-128 is NOT closed and has NO OWNER.** Folds into ONE `CanonicalEmail` newtype remediation item with F-128, F-124 and F-127. Explicitly outside WP-59's mandate. Breadth bounded by deployment DB collation - not verifiable this session. |
| F-174 | Low | 09b | WP-58 follow-up (`5786a1b6`) | `rust/web/src/email/commands.rs:179-208` (`:192`) | `help_text()` still advertises `rules` and four game-only verbs to standalone/no-game users, who are then told the command is unavailable. | Residual of `5786a1b6`, whose fix corrected only the rejection string despite a "help text" commit subject. Pattern 4b/4e for `5786a1b6` is REFUTED; F-174 is the residual that refutation points to. |
| F-175 | Medium | 09c | WP-60 | `rust/web/src/email/outbound.rs:123-139` (+1 site) | `ensure_settings_email_token` / `ensure_unsubscribe_token` keep the pre-fix select-then-update body, so tokens can be returned unpersisted or lost to a concurrent writer. | Pattern 2; same shape as F-116, F-166, F-169. The checklist scoped `wfe F44`/`F45` by function name, so the row reads satisfied. `00-STATE.md`: the WP-60 token expiry/single-use hypothesis is REFUTED - this is the real pattern-2 gap, and F-161's substance is untouched by WP-60. |
| F-176 | Medium | 09c | WP-60 | `rust/web/src/email/outbound.rs:301-364` (+2 sites) | `e5513ec6` adds no test at all, yet all four of WP-60's `Test? y` rows (`wfe F44`, `F45`, `F46`, `F63`) are marked tested. | One ID covering FOUR falsified rows; brings the session "Test? y with no test" tally to **nine**. The F44/F45 guard is a pre-existing decoy (F-151 / F-161d class). WP-76/WP-77 rows must NOT be added to this tally - `EXECUTION-README.md:408` records them as a deliberate no-spec/no-row gap. |
| F-177 | Low | 09c | WP-60 | `rust/web/src/email/render.rs:252-262` | Two of four `href` interpolations in the same function (`unsub`, `manage`) were left unescaped ten lines below the two that got `escape_html_attr`. | Pattern 2 inside `wfe F49`. Impact theoretical today - no attacker-controlled byte reaches either URL. F49 offered escape OR documenting the trusted-URL precondition; neither was done fully, so the row is undischarged. |
| F-178 | Low | 09c | WP-60 | `rust/web/src/email/render.rs:152-164` | The new `escape_html_attr` duplicates the existing `html_escape` in the same module tree, whose "no public HTML-escape helper exists" comment is still in place. | Maintenance duplicate, not a live defect. `html_escape` has a test; `escape_html_attr` has none. |
| F-179 | Medium | 09c | WP-76 (`bc051164` / `ca7925bc`) | `rust/web/src/email/inbound.rs:1076` (+2 sites) | The invite-accept auto-start mails the same invitee three times for one event, gated by three different preference columns, so no single unsubscribe damps the burst. | WP-76 has NO spec and NO checklist row (deliberate, `EXECUTION-README.md:408`) - no `Test?` column to falsify. Unit 10-adjacent: hits the sending-domain reputation WP-57/WP-58 were spent protecting. |
| F-180 | Low | 09c | WP-76 | `rust/web/src/email/proposals.rs:1471` | The solo-game start notify is unreachable in practice - `suppress_for_web_presence` suppresses it every time in the normal hydrated-page flow. | Harmless but contradicts the commit message; untested. |
| F-181 | Low | 09c | WP-76 | `rust/web/src/email/proposals.rs:1470-1478` (+4 sites) | All start sites publish `bot.turn` before `notify_game_emails`, so a fast bot move lets both the start-path notify and `handle_bot_command_event` mail the same transition. | Ordering pre-dates `ca7925bc` (hence Low) but the commit widened it to two more sites. |
| F-182 | Low | 09c | WP-76 | `rust/web/src/email/proposals.rs:111-120` | Both WP-76 commits call the free notify function instead of the existing `ProposalMailer` seam, so none of the new wiring is spyable or tested. | Explicitly a DISCLOSED gap, not a false `Test? y` - WP-76 has no `Test?` column and `EXECUTION-STATE.md:18` records the missing spy infra. Route as testability debt, not checklist integrity. |
| F-183 | High | 09c | WP-77 (defect is WP-59-era code) | `rust/web/src/email/commands.rs:82-93` (written at `:398-401`) | Email `new` lowercases the bot name into `game_bots.bot_name` while the bot service looks it up case-sensitively, so the bot never moves and the game wedges silently. | **Remediate as ONE item with F-104, F-138 and F-189** - one bot-name case-sensitivity defect spanning four units. Fix = canonicalize inside `validate_bot_slots` and return the canonical name. F-185 must be re-fixtured in the same change. Precondition: `admin::create_bot` (`admin.rs:293-303`) permits arbitrary casing. NOT introduced by `33150afe`. Report states the pairing as three items; `00-STATE.md` adds F-189 - `00-STATE.md` wins. |
| F-184 | Low | 09c | WP-77 | `rust/web/src/components/opponent_slot.rs:93-97` (+2 sites) | `set_mode` runs on an ungated radio before `bot_names` settles, so the hard-coded `"medium"` default can still be stored, rendering a blank `<select>` and failing submit. | Pre-settle residual of an otherwise correct fix; the settled path is REFUTED as a defect (WP-77's own default IS canonical - a byte-for-byte copy of the `bots.name` column). No spec, no checklist row, so no test was owed. |
| F-185 | Low | 09c | WP-77 | `rust/web/src/email/commands.rs:1435-1455` | `classify_opponent_detects_bots` uses an all-lowercase fixture, so lowercasing and canonicalising are indistinguishable and it asserts the lowercased output as correct. | Pattern 4b decoy - **the test that hid F-183**; must be re-fixtured in the same change as F-183 / F-104 / F-138 / F-189. Its partner `validate_bot_slots_accepts_case_mismatch` is already filed as F-104 (pattern 4f in `00-STATE.md`). |
| F-186 | High | 10a | - | `rust/bot/src/crypto.rs:66-70` (+1 site) | The bot silently falls back to the hardcoded dev encryption key when `DATABASE_ENCRYPTION_KEY` is unset - no opt-in gate, no `MissingKey` variant. | The new finding routed from the F-96 investigation with F-90; the forbidden "dev default + warn" pattern (`docs/CODING.md:701`). Remediate as ONE item with F-187 and F-188. |
| F-187 | Medium | 10a | - | `rust/bot/src/crypto.rs` (+1 site) | `rust/bot/src/crypto.rs` is a divergent duplicate of `rust/web/src/crypto.rs` on four axes; every web hardening is absent from the bot copy. | **F-90 is NOT closed at HEAD** - recorded as fixed, but the fix landed only in the web copy. Pattern 2 at file granularity. One item with F-186/F-188; `00-STATE.md` says fix F-90 and F-108 together. |
| F-188 | Medium | 10a | - | `rust/bot/src/nats.rs:1-36` (+2 sites) | Bot and web NATS wire types and constants are copy-paste duplicates with no shared type and no cross-crate round-trip test. | F-108 still open. No live wire drift today, but non-wire divergence already exists (bot hardcodes web's local `ack_wait`). One item with F-186/F-187. |
| F-189 | High | 10a | - | `rust/bot/src/config.rs:26-29` (+2 sites) | The case-sensitive `WHERE name = $1` bot lookup misses and the miss path returns `Ok(())`, acking and discarding the turn - the game wedges. | **Extends F-183** (bot-side half CONFIRMED) and adds a second, previously uncited site `rust/bot/src/config.rs:67`. Remediate as ONE item with F-104, F-138 and F-183 - one bot-name defect spanning four units. The silent ack at `main.rs:186-194` must be in the same change. |
| F-190 | Low | 10a | - | `rust/bot/src/main.rs:809-816` (+2 sites) | An invalid `DATABASE_ENCRYPTION_KEY` only warns and sets the key to `None`; every turn then errors and strands in the WorkQueue stream. | Consistent-fix candidate with F-186 (fail startup outright, as the web crate does). |
| F-191 | Low | 10a | WP-06 | `rust/lib/cmd/src/http.rs:26-29` (+1 site) | A malformed request *envelope* is rejected by axum's `JsonRejection` (400/422 text) instead of the documented HTTP 200 `Response::SystemError`; untested at any layer. | Confirms the `00-STATE.md` Unit 10 carry-forward about `http.rs`'s axum final form. The WP-06 test is explicitly NOT a decoy - only the WP-06 acceptance narrative overstates. |
| F-192 | Medium | 10a | - | `rust/lib/game_client/src/lib.rs:25-35` (+2 sites) | `HttpStatus`/`ParseResponse` embed the whole game-service body - every seat's private state - in `Display`, reaching `tracing::error!` and Sentry. | Medium pair with F-193 (F-193 is the cause); remediate together. Belongs in the hidden-information section with F-22/F-28, not logging. Found because `prompt.rs` was REFUTED as a leak vector. |
| F-193 | Medium | 10a | - | `rust/lib/game_client/src/lib.rs:310-331` (+1 site) | `fetch_game_data` requests `Request::Status`, pulling every seat's `player_renders` plus raw `game.state` into the bot, then discards all but one. | Cause half of the F-192/F-193 pair. Narrower `PubRender` + `PlayerRender{player}` endpoints already exist. |
| F-194 | Low | 10a | - | `rust/bot/src/main.rs:585` (+1 site) | `players[].score` reaches the prompt from `gamer.points()` - the one prompt input bypassing the `pub_state` redaction boundary. | Not a bot defect: the platform already treats points as public. Pairs with the carried-forward unnumbered item "`Gamer::points()` has no documented ordering contract", which `00-STATE.md` hands to the remediation plan. |
| F-195 | Low | 10a | - | `rust/bot/src/main.rs:276-282` | TRACE logging emits `system_prompt`/`user_prompt` verbatim, exposing the bot's own hand to anyone with log access. | Own-seat only, off by default. |
| F-196 | Medium | 10a | WP-62 | `rust/operator/src/controller.rs:240` (+2 sites) | The authoritative-version guard only writes forward, so deprecating or deleting the newest version leaves the stale `game_types` row permanently unrepaired. | `cleanup` (`:174`) has the same shape and **zero test callers**. A fresh instance of "satisfied the row literally, missed what it was for" plus cross-file pattern 2. |
| F-197 | Low | 10b | WP-65 | `rust/game/love-letter-2/.rls.toml` (+3 sites) | The `e F28` sweep worked from an enumerated file list, so four byte-identical `.rls.toml`/`.gitignore` siblings survived and the row was accepted as complete. | Textbook pattern 2. Row is `Test? = n`, so NOT a falsified row. The `build-release` eradication itself did land. |
| F-198 | Low | 10b | - | `rust/bot/src/main.rs:776-827` (+2 sites) | `rust/bot` is the only TLS-capable binary with no rustls process-default install and declares no `rustls` dependency at all. | Explicitly NOT a checklist falsification (the `docs/CODING.md` rule is conditional) and NOT a WP-64 regression - the omission is original. No live panic demonstrable, hence Low. |
| F-199 | Low | 10b | WP-65 | `.github/workflows/deps-currency.yml` (+1 site) | The weekly `cargo deny` job can fail with no notification wiring and checks `advisories` only - never `bans`, `licenses` or `sources`. | **Remediates as ONE item with F-206 and 10b's Coverage gap 3** - three views of one unenforced `bans` section. Row `dp F23` is `Test? = n`, not a falsified row. |
| F-200 | Medium | 10b | WP-66 | `rust/lib/session_store/src/postgres_store.rs:87-130` (+2 sites) | `migrate()`'s duplicate-key branch returns `Ok(())` before `create table` and without committing, so a concurrent cold start reports success with no session table. | **The vendoring finding and a new named pattern**: an upstream defect inherited *because* the correctly-followed "minimal port, not a rewrite" criterion guarantees it comes along. WP-66's spec gate was honoured; the cost landed anyway. Recommend a "known upstream defects inherited" criterion for future vendoring specs. Feeds the owner's open vendoring-policy question. |
| F-201 | Low | 10b | WP-66 | `rust/web/src/db/users.rs:256` (+2 sites) | Three sqlx error-classification sites - the only ones in the workspace - crossed the 0.8 -> 0.9 major bump with no re-check and no test. | Filed Low as an unverified risk, not a demonstrated defect; the point is procedural. Same class as F-200's second-order risk. |
| F-202 | Low | 10b | WP-66 | `rust/web/src/db/test_support.rs:146-152` | `count_rows` interpolates `table: &str` with no validation; the 0.9 migration added the `AssertSqlSafe` wrapper to satisfy the compiler and audited nothing. | Not a live injection surface and not introduced here. Contrast recorded: the vendored store's eight `AssertSqlSafe` sites are genuinely safe. |
| F-203 | Low | 10c | WP-64 | `rust/Cargo.toml:78-79` | WP-64 shipped `[workspace.lints.clippy]` but silently dropped the spec-prescribed `[workspace.lints.rust]` table, and no later commit added it. | Spec-vs-code gap, explicitly NOT a regression. No stricter per-crate config was displaced (zero `#![deny/warn/allow]` at `4fb252da^`). WP-64 has no checklist row, so not a falsified `Test? y`. |
| F-204 | Low | 10c | WP-64 | `rust/Cargo.toml:56-76` | Ten of 21 `[workspace.dependencies]` entries are bare-major/bare-minor, violating WP-64's rider 1 - which the same spec's §3b contradicts. | Spec-vs-code gap, not a regression, and NOT a clean falsification: the acceptance criterion is internally inconsistent. `sqlx = "0.9"` was set by WP-66, not WP-64. |
| F-205 | Low | 10c | WP-67 | `docs/reviews/2026-07-23-rust-review/SUMMARY.md:44-46` (+3 sites) | `dp F12` was closed on a premise that was never true (sentry defaults dragging in actix-web/ureq), and rider 2's mandated downgrade of the finding never happened. | **New named pattern: "the finding whose premise was disproved, closed anyway, never amended."** Distinct from 4b - the docs were never edited at all despite an explicit criterion requiring it. Sign-off fix: a disproved mechanism must amend the finding, not merely close it. Report states severity as "Low, but a NEW NAMED PATTERN". |
| F-206 | Medium | 10c | WP-69 | `rust/deny.toml:71-76` (+3 sites) | WP-69's spec set a STOP-AND-REPORT threshold ("roughly a dozen" skips); 29 landed and the commit wrote a pre-emptive rebuttal into `deny.toml` instead of stopping. | Unit 10c's headline and a new process-fix pattern: *a spec's own escalation trigger fired and the implementation answered it with a comment*. **Remediate as ONE item with F-199 and 10b's Coverage gap 3.** The rebuttal is falsified by `rust/deny.toml:131`. Compounding: WP-69 §5's negative checks are recorded as parked, never run. `00-STATE.md` correction applied: **29** skip entries, not 24. |
| F-207 | Low | 10c | WP-66 | `rust/Dockerfile:132` (+2 sites) | Three sqlx migrators write `_sqlx_migrations` - prod `sqlx-cli` pinned 0.8.6, CI unpinned, library 0.9 - with no commit or spec justifying the split. | **Deployment-checklist item, not a code finding - groups with the F-96 deployment-checklist family.** No commit in the 127-commit range touches `rust/Dockerfile` at all. Mitigating: `rg 'migrate!' rust` is empty, so nothing validates checksums at runtime. |
| F-208 | High | 11 | - (`f4cbc51d`, `c882d413`, `16dae9dd`) | `rust/Dockerfile:36` (+3 sites) | `hanamikoji-1` is a workspace member compiled by every image build but has no Dockerfile stage, no docker-bake target and no k8s Deployment - it is unshippable. | A complete, tested, documented new game with no build- or deploy-time signal that it does not ship. `rg 'hanamikoji'` finds zero hits outside the crate and the manifests. No commit since 2026-07-20 touches `rust/Dockerfile` or `docker-bake.hcl`, so the 127-commit window could never have caught it. Pairs with the new process-fix item: a CI guard over the four hand-maintained delivery lists. `lords-of-vegas-1` is the other absentee (WIP, owner-excluded). |
| F-208a | unstated | 11 | - | `k8s/base/game/kustomization.yaml` (+2 sites) | The carried "43 k8s Deployments vs 26 image stages" discrepancy: 43 = 26 Rust stages + 17 legacy Go games with stages in `brdgme-go/Dockerfile`. | **REFUTED - a carried premise, not a defect.** Zero stages lack a Deployment; zero bake targets lack a stage. Drop the 43-vs-26 framing from the unified report. Sub-letter carries no severity. |
| F-209 | Medium | 11 | - (crate never subject to any WP) | `rust/game/hanamikoji-1/src/lib.rs:673-730` (+4 sites) | `validate` bounds every parallel vector and seat index but never relates `phase` to `pending`, so a deserialized state can wedge the game forever or silently destroy three cards. | Textbook systemic pattern 2b - the parallel-vector sweep is present, the one cross-field invariant is missed; here the consequence is a wedge/corruption rather than a panic, which is quieter. Proof that having a `validate` test is not sufficient. |
| F-210 | Medium | 11 | - (`ae04843c`; WP-72-class self-certification) | `rust/game/sushi-go-2/src/lib.rs:140-147` (+1 site) | `ae04843c` replaced `_ => 9` with `unreachable!()` on the false premise that `start()` is the only entry point, so `all_players` outside `2..=5` now panics the game service. | **Remediate as ONE item with F-06's sushi-go-2 row** - sushi-go-2 has no `validate` override at all and `all_players` is never bounded. Second WP-72-class self-certifying commit (no spec, no `T3-B*` row, acceptance evidence is a "Done" row it wrote itself) and the **first where the self-certified premise is demonstrably false**. F-96-class hardening-into-panic; exact inverse of pattern 5. |
| F-211 | Low | 11 | - (`e2aef66b`; code change `68ebef7`) | `rust/web/end2end/tests/page-loads.spec.ts:8` (+1 site) | The e2e smoke assertion was edited down to the `<h1>`'s own `brdg.me` fallback string, so it no longer distinguishes a healthy index page from a degraded one. | Pattern 4b (test adjusted to agree with the code), milder than F-72a/F-83/F-79/F-95 because the code change was intentional. Compounding: the `e2e` job is `continue-on-error: true` (`ci.yml:148`), so no assertion in the file can fail a merge or deploy. Group with the F-96 deployment-checklist family. |

## Severity tally

Range F-158..F-211: **54 numbered findings, 59 rows** (5 extra rows are sub-letters -
F-161a/b/c/d and F-208a - none of which carries its own severity).

| Severity | Numbered findings | All rows |
|----------|-------------------|----------|
| High | 7 | 7 |
| Medium | 19 | 19 |
| Low | 28 | 28 |
| unstated (sub-letters only) | 0 | 5 |
| **Total** | **54** | **59** |

- **High (7):** F-158, F-161, F-169, F-183, F-186, F-189, F-208.
- **Medium (19):** F-159, F-160, F-162, F-165, F-166, F-170, F-171, F-175, F-176,
  F-179, F-187, F-188, F-192, F-193, F-196, F-200, F-206, F-209, F-210.
- **Low (28):** F-163, F-164, F-167, F-168, F-172, F-173, F-174, F-177, F-178,
  F-180, F-181, F-182, F-184, F-185, F-190, F-191, F-194, F-195, F-197, F-198,
  F-199, F-201, F-202, F-203, F-204, F-205, F-207, F-211.
- **unstated (5 sub-letter rows):** F-161a, F-161b, F-161c, F-161d, F-208a.

No severity in this range is qualified (unlike part 2's five qualified buckets),
with two textual exceptions normalized here and recorded below: F-161 and F-205.

## Discrepancies

### A. Report vs `00-STATE.md` / `00-HANDOVER.md` (`00-STATE.md` applied)

1. **F-183 pairing is understated in the unit report.** 09c says "remediate as one
   item with F-104 and F-138" (three). `00-STATE.md` and `00-HANDOVER.md` say
   **four**, adding F-189; the 09c report never mentions F-189 at all. `00-STATE.md`
   applied. This is the session's largest cross-unit pairing: **F-104 + F-138 +
   F-183 + F-189**, spanning Units 05b, 07, 09c and 10a, plus F-185 re-fixtured in
   the same change and `admin::create_bot`'s arbitrary-casing precondition.
2. **`deny.toml` skip-list count.** The 10b/10c report states **24** at its WP-72
   section and **29** at F-206 and Coverage gap 3; `00-STATE.md` issues an explicit
   CORRECTION to **29**. **29 applied**; the line-488 figure is stale inside the
   report itself.
3. **F-161 severity is stated two ways** - "F-161 (High)" in the progress list,
   "(High, and it escalates F-129 + F-130)" in the heading. Normalized to **High**
   with the escalation moved to the status column. `00-STATE.md` says High.
4. **F-205 severity is stated non-atomically** - "Low, but a NEW NAMED PATTERN".
   Normalized to **Low**; the pattern claim is in the status column.
5. **`00-STATE.md`'s carry-forward that `hanamikoji-1` has an unguarded epilogue is
   REFUTED by Unit 11.** The gate exists (`lib.rs:796` / `:830-834`, identical to
   jaipur-2) with a dedicated regression test. `hanamikoji-1` does **not** join the
   F-18/F-71 crate list. Report wins here because `00-STATE.md` records it as an
   unverified carry-forward, and `00-HANDOVER.md` already accepts the refutation.
6. **F-170's scope.** 09b files it broadly; 09c REFUTES its extension to the
   game-start mail (that path reads `turn_emails_enabled` directly). `00-STATE.md`
   records the refutation. Applied.
7. **F-208a is a sub-letter for a REFUTED carried premise, not a defect.** Unit 11's
   own defect count is 4. Kept as a row so the refutation is not lost.

### B. ID sequence

- **F-158..F-211 is complete: no gaps, no duplicates.** Every ID appears exactly
  once as a finding heading in exactly one report.
- Sub-letters: **F-161a/b/c/d** (09a never formally defines the IDs - the body uses
  bare `(a)`-`(d)` while a later section cites "the F-161d class", so `F-161d` is a
  citable ID the finding itself never declares) and **F-208a**. None carries its own
  severity.
- **F-176 is one ID covering four falsified checklist rows.** Any per-ID count of
  "Test? y with no test" instances undercounts by three. The tally is nine:
  F-142, F-148, F-149, F-150, F-171 + WP-60's four (F-176).
- Two findings are filed outside their unit's commit scope and the Unit column is a
  poor locator for them: **F-207** (subject `rust/Dockerfile:132` is untouched by any
  commit in the 127-commit range) and **F-210 / F-211** (sushi-go-2 and the web e2e
  suite, under Unit 11's "unassociated tail" scope).

### C. Substantive items in range carrying NO F-number

These would be lost entirely if not captured here. **41 items.**

**Discharged obligations and settled sweeps (do not re-derive):**

1. Obligation 1: `efad81f` contains **exactly one** pattern-4e instance (F-109),
   demonstrated by enumerating all 12 touched files. F-163 is the near-miss.
2. Obligation 2: WP-56's DMARC soundness answered **NOT SOUND** (F-161).
3. Obligation 3: F-131's authenticate-once consequence concretised as F-158.
4. Obligation 4 DISCHARGED: **F-15 stays LATENT**, no live violation at the real
   emitter. Every `--mk-soften-*` token referenced anywhere is emitted; game crates
   emit exactly `{(Pink,80),(Foreground,80),(Foreground,90)}`; no game emits a `mix`,
   so the empty `IN_USE_MIXES` is correct. `main.scss` is the only stylesheet.
5. Obligation 5 DISCHARGED: **28/28 `test-support` consumers are dev-dependencies**,
   the feature is not in `default`, and the 14 panic constructs cannot reach a release
   build. `assert_gamer_contract` is called from all 28 `rust/game/*/tests/contract.rs`.
6. Obligation 6 REFUTED: the `rust/.sqlx` deletion is correct consolidation, with the
   causality inverted (WP-52 is an *ancestor* of WP-66). Only residual: WP-52's commit
   message does not mention removing an 81-file directory - a process nit.
7. The **`ssr` feature-gate question is SETTLED and REFUTED**: 423 gated test
   functions across 25 modules are live. **No "Test? y" row is retro-voided.**
8. **WP-59 Tasks 9-14 are NOT a coverage hole.** `f56ff37` owns 9/11/12/13; Task 10
   was dissolved by WP-56 (`da1ea24`); Task 14 is a deliberate non-implementation per
   the spec's own carve-out to WP-85. Implementation status only - no deep code review.
9. The **`RouteOutcome` sweep of `email/inbound.rs` is CLOSED**: F-162 and F-169 are
   the only two defective routes; **no third route** has the defect.
10. **`ca7925bc`'s game-start sweep is complete** - all four `insert_game_from_service`
    callers notify - and it is **not** a pattern-4e revert (`+20/-0`).

**Refutations with no F-number (expensive negatives that must not be re-derived):**

11. **`rust/bot/src/prompt.rs` is REFUTED as a leak vector** - a pure minijinja
    renderer over a closed field list; `BotContext.game_state` never enters a context
    struct. It predates the programme. The real leaks found instead are F-192/F-193.
12. WP-64: **all four briefed hunts proved negative** - no pattern 2, no silent default
    change, no feature narrowing (the one feature change is a widening), no pattern 4b.
13. WP-66's `default-features = false` sqlx narrowing is **inert** - all four dropped
    features are compile-time-only.
14. `serde_yaml_ng` is a **faithful fork** - full `diff -ru` shows only
    `i64::max_value()` -> `i64::MAX` plus an additive API; all 7 call sites
    serialisation-only.
15. `[licenses.private] ignore = true` is **correct config**; no `cargo-deny` setting
    could machine-check the vendored MIT obligations.
16. WP-69's unspecified `allow-wildcard-paths = true` **improves on a wrong rider** -
    the explicit counterweight to F-206.
17. `be185ccb` is a harmless bookkeeping race, **explicitly not pattern 4b**.
18. **WP-73 proved good exhaustively, not sampled**: all 108 pre-commit game bins
    normalise to exactly four distinct contents, 27 each; the `:80` -> `:8080` default
    change is inert (all 43 Deployments set `ADDR`); `[lints] workspace = true` on all
    44 members.
19. The deleted `*_repl` binaries are a capability **move**, not a loss.
20. **A Worker claim is explicitly WRONG and must not be carried forward**:
    `docs/porting/GAME_PORTING.md:215` does *not* cite a non-existent package
    `brdgmen` - it reads `cargo run -p brdgme_repl`, matching the crate.
21. `settings.rs`'s `<select prop:value=...>` is **not** a pattern-2 miss of WP-54's
    build-order fix; `svix-id` is **not** attacker-chosen; `processed_webhook_events`
    does **not** grow forever; a GET **cannot** unsubscribe; the unsubscribe token is
    **not** guessable and does not reuse `settings_email_token`; **all eight** bulk-mail
    sites got the unsubscribe link. (Six separate 09b refutations.)
22. `5786a1b6` is **neither** pattern 4b nor 4e - the spec's own instruction was
    factually wrong and the follow-up corrects the string to the truth (F-174 is the
    residual).
23. WP-77's default bot name **is canonical**; the "fifth write path" hypothesis is
    refuted on the settled path, and the five other bot-name write sites are clean.
24. `hanamikoji-1`'s `render.rs` is **not** a leak vector and its `pub_state` is
    **structurally** redacted (hidden fields omitted by construction, not blanked) -
    the strongest form of the D-33 pattern seen in the session.
25. `a99bf754` (`exec` removal in `scripts/rust-test.sh`) and `3f52d2b7`
    (`ALLOW_INSECURE_DEFAULT_KEY` is set in dev/CI only, **zero hits** under
    `k8s/base/**` or `k8s/prod/**`) are both **verified good**.

**New decoys and pattern instances with no F-number:**

26. **`render_user_includes_state_in_yaml_fences`** (`rust/bot/src/prompt.rs:291-302`)
    is a confirmed decoy - the fixture hand-writes the yaml as string literals, so
    swapping in another seat's state would still pass. Extends the decoy class to the
    bot crate.
27. **`auth/server.rs:92`** still increments `login_emails_sent_total` before the send
    with no failure counter - the exact shape `wfe F46` fixed, outside WP-60's scope.
    A programme-level consistency item.
28. **A module-granularity `#[allow(dead_code)]`** at `rust/bot/src/main.rs:4-7` covers
    all of `mod config` and `mod crypto` - broader than the F-153/F-170 cases, and would
    hide `crypto::encrypt` and `LoadedKey::is_default`. Flagged for the sign-off sweep.

**Coverage gaps with no owner:**

29. Nothing tests the vendored `rust/lib/session_store` (no `tests/`, no
    `#[cfg(test)]`) - authentication-adjacent and now first-party.
30. Vendored MIT obligations are hand-satisfied and **cannot** be machine-checked; a
    one-line CI grep would close it.
31. `deny.toml`'s 29-entry skip list has no expiry, no `unused-skip`, and the weekly
    job never runs `bans`; `[advisories].ignore` has the same shape. (Folded into the
    F-199 + F-206 remediation item.)
32. Zero tests on `build_messages`; the whole bot test module covers only
    `merge_json_patch` and one constant. The bot crate has **no DB tests at all**.
33. `fetch_game_data`'s test is one assertion short of real - two seats with distinct
    hands, only the positive asserted. Nothing anywhere asserts that opponent hidden
    state is absent from the rendered prompt.
34. No test on the log SQL filter in bot or web; no test for the F-189 case-mismatch
    path; no round-trip test between the two `Bot*Event` definitions; no envelope-level
    test on `route::<G>()`.
35. `rust/operator/src/controller.rs`: no test flips an already-newest version to
    deprecated, and `cleanup` has **zero test callers**.
36. `rust/tools/fuzz/src/lib.rs:53-57`'s `recv()` has no timeout - a worker wedged
    inside `requester.request()` still hangs the driver. An explicit spec non-goal.
37. `hanamikoji-1`'s `test_redaction` tests the opening position only and
    `test_validate` tests lengths and ranges only; neither would catch a leaking field
    or F-209's cross-field invariant. **WP-10 3a's "13 crates with no redaction test"
    gap is untouched by Unit 11.**

**Deployment-checklist items (F-96 family) with no F-number:**

38. **`config::public_base_url()` defaults to `http://localhost:3000`**, which would
    make WP-58's `List-Unsubscribe` non-HTTPS and RFC 8058-invalid in production.
    Route to the same checklist F-96 produced. (Also recorded in `00-STATE.md`.)
39. The **e2e job is `continue-on-error: true`**, so no assertion in it gates a merge
    or deploy; the underlying hydration race needs tracking. (Compounds F-211.)

**Process and corpus items with no F-number:**

40. **WP-72 is self-certifying** - no spec, no checklist row, a one-line commit
    message; it exists only as commit `a5d6f102`. A work package that exists only as a
    commit cannot be verified by any sign-off procedure. `ae04843c`/F-210 is the second
    instance and the first with a demonstrably false self-certified premise.
41. **The corpus records a false dependency fact** (`SUMMARY.md:44-46,139`;
    `findings/dependencies.md:103-108,157`) and two stale superpowers docs still assert
    the `0.0.0.0:80` default and cite a WP-73-deleted path. The unified report should
    **amend** the corpus entry, not merely record the WP closed. (See F-205.)
    Also: **no spec exists for WP-60, WP-65, WP-72, WP-76 or WP-77** - extend part 2's
    already-extended no-spec list. WP-76/77 additionally have no row in any of the
    eight `T3-B*` checklists, deliberately (`EXECUTION-README.md:408`).
    And **`00-breakdown.md`'s premise was wrong a THIRD and FOURTH time**: WP-66 is 12
    real files, not 101; WP-73 is 139 files of which 135 are three-line wrappers.
    Finally, **nothing links the four hand-maintained delivery lists** (`rust/Cargo.toml`
    game members, `rust/Dockerfile` stages, `docker-bake.hcl` matrix,
    `k8s/base/game/kustomization.yaml`, spanning two repos) - F-208 is the first miss and
    a CI guard with an explicit WIP allowlist is a programme-level remediation item.

### D. Positives worth carrying (would otherwise be lost)

42. **`hanamikoji-1` is the first crate in the session with a `validate` override AND a
    `validate` test**, and the first with a dedicated epilogue-gate regression test.
    `00-STATE.md` pattern 2b's "no crate reviewed so far has a `validate` test" is now
    broken. Use it as the model; use F-209 as proof that having one is not sufficient.
43. **`hanamikoji-1`'s `Status::Finished` populates `stats`** - a **negative** for the
    F-35 tally. The one crate written outside the programme is the one that populated
    stats.
44. **`finish_epilogue` is a per-crate inherent method in 12 crates, not a
    `rust/lib/game` helper**; only `placings_log` is shared. Corrects a premise usable
    elsewhere in the unified report.
45. The **ws F55 "bot consumer gets no shutdown signal" concern does not apply to the
    bot binary** - in-flight turns are drained on shutdown. This narrows F-109's
    never-implemented second half to the email sweep task only.
