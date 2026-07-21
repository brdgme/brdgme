# Session Orchestration: #22b-d Email, #24 Game Invites, #25 Rules Rendering

Date: 2026-07-20. Orchestrator session implementing backlog items #22b-d, #24,
#25 autonomously. This is the plan + running decisions log. Append-only during
the session; the user reviews the decisions sections at the end.

## User mandate (verbatim intent)

- Implement #22b-d, #24, #25. Work fully autonomously; do not stop to ask
  questions. Make obvious/straightforward decisions. For large/difficult/
  impactful decisions, work around them low-risk and do as much as possible;
  redoing work later is acceptable - functional-first is the goal.
- #22b-d: review legacy `~/Development/brdg.me` for inbound email handling +
  outbound rendering style (learnings only, not gospel - it is years old;
  prefer modern approaches). A Rust email templating library for consistent
  cross-client styling may be referenced in specs/plans - investigate.
  Emails should support themes: render using the theme selected in the
  recipient's profile. Most settings should be customisable via email commands
  (display name, preferred colours, theme); ideally everything the web client
  can do, can be done purely via email commands. See legacy `scommand` for the
  old command set. Ignore infra config; focus on implementation + automated
  testing.
- #24: make sensible straightforward decisions; defer only very big decisions;
  do as much work as possible.
- #25: also render basic + advanced strategy for the user (same or separate
  pages - undecided), in addition to rules.
- Log every decision made and every decision deferred to the user.

## Specs (authoritative design records)

- #22: `docs/superpowers/specs/2026-07-05-22-email-via-resend-design.md`
- #24: `docs/superpowers/specs/2026-07-04-24-game-invites-design.md`
- #25: `docs/superpowers/specs/2026-07-05-25-rules-rendering-design.md`
- #26 theming (done): `docs/superpowers/specs/2026-07-05-26-theming-design.md`,
  plan `2026-07-13-26-theming-semantic-colors.md`
- #43 bot efficacy (done): added per-game DATA_DOCS.md / BASIC_STRATEGY.md /
  ADVANCED_STRATEGY.md and `game_versions` strategy columns / game-service
  endpoints - the source for #25's strategy rendering.

## Verification conventions (per AGENTS.md)

- Target single packages: `cargo ... -p web` etc. Never workspace-wide builds.
- web crate has NO default features: always `--features ssr` for check/clippy/
  test on `-p web`.
- Before committing Rust: `cargo fmt --all -- --check`,
  `cargo clippy -p web --all-targets --features ssr -- -D warnings`,
  `cargo clippy --workspace --exclude web --all-targets -- -D warnings`.
- DB-backed tests fail in a plain local/agent run (no Postgres) - known
  pre-existing condition (#40), not a regression. Leads: prioritise pure-logic
  unit tests that run without a DB (reply parsing, quote stripping, command
  parsing, markup/theming rendering); write `#[sqlx::test]` integration tests
  for CI; verify locally with fmt + clippy + the non-DB tests. Do NOT chase the
  DB-test failures.
- Do not poll CI. Commit per unit; defer pushing to the final cleanup unit.

## Lead sequence (serial; one subagent at a time)

### Survey Leads (write context-handover docs under docs/superpowers/handovers/)
- S1 #22 email survey -> `handovers/2026-07-20-email-22-handover.md`
- S2 #25 rules/strategy survey -> `handovers/2026-07-20-rules-25-handover.md`
- S3 #24 invites survey -> `handovers/2026-07-20-invites-24-handover.md`

### Implementation Leads (each: own tests + own commit)
- #25 web rendering (rules + basic/advanced strategy): backend render fn +
  query/strategy-fetch, then UI/routing. (~2 Leads)
- #22b email foundation:
  - (a) themed `email_render` module (mrml chrome + palette-aware board) +
    `users.last_seen_at` activity tracking + the active-web suppression check
    (decision 20) + outbound notification wiring + recipient classes.
  - (b) inbound webhook (svix, metadata-only + Received-emails body fetch
    behind a trait) + reply parsing (quote-strip) + idempotency.
  - (c) server commands (concede/undo/restart/un-subscribe) + SETTINGS
    commands (name/colours/theme/emails) + response emails.
  - (d) game CREATION by email (decision 17) on the command framework.
  (~4 Leads)
- #22c turn reminders: sweep task (honours suppression + opt-out). (1 Lead)
- #22d multi-email: verification + switching + auth + digest. (1-2 Leads)
- #24 core: proposal tables + flows + UI. (~2 Leads)
- #24 email: invite emails + accept/decline-by-email + nudge/expiry (honours
  suppression). (1-2 Leads)
- #25 email: `rules`/strategy reply commands + "view rules" link in mails.
  (1 Lead, folds into the command set)
- Final cleanup/verification Lead: fmt/clippy pass, push, summary.

---

## DECISIONS MADE (Orchestrator / Leads) - review at end of session

1. **Implementation order.** #25 web rendering first (independent of email),
   then #22b email foundation (the base for all email features), then #22c,
   #22d, #24 (core then email), then #25 email integration. Rationale: #25 web
   has no email dependency and delivers value early; #22b's `email_render`
   module is a prerequisite for #22c/#22d/#24-email/#25-email.
2. **#24 open decisions resolved to the spec defaults** (user: make sensible
   straightforward decisions): (a) declining is terminal per invitee; (b)
   auto-start when the last pending invitee accepts; (c) solo-vs-bots games
   bypass the proposal and create the game directly. Owner may still start
   early with per-slot drop/replace-with-bot policy.
3. **#25 strategy rendering.** Extend #25 beyond rules to also render basic +
   advanced strategy (per user). Default UI: a single `/rules/{version_id}`
   page with three sections (Rules, Basic Strategy, Advanced Strategy), each
   through the same render-time-specialization pipeline (pulldown-cmark +
   `brdgme_markup` fence interception). Final same-page-vs-sections choice may
   be refined by the #25 survey/implementation; functional-first.
4. **Email theming.** Outbound email HTML renders using the recipient's
   selected profile theme (`users.theme`, system-default fallback), reusing the
   #26 semantic-colour palette. Exact mechanism (inline CSS vs MJML) decided by
   the #22 survey (see open item O1).
5. **Email command set expansion.** Beyond the spec's `concede`/`undo`/
   `restart`/`unsubscribe`/`subscribe`/`rules`, add settings commands so most
   web settings are reachable by email: at minimum display name, preferred
   colours, theme, turn-emails on/off. The #22 survey enumerates the legacy
   `scommand` set + current web settings surface and proposes the grammar.
6. **Infra out of scope.** No k8s/SealedSecret/MX/DNS/Resend-dashboard work.
   Webhook svix signing secret + Resend key read from env with a dev/log
   fallback (matching 22a's pattern). Focus is implementation + automated tests.
7. **Verification stance.** Pure-logic unit tests run locally; DB-backed
   `#[sqlx::test]` tests are written for CI (local DB unavailable - known #40
   condition). Every Lead runs fmt + clippy + non-DB tests before committing.
8. **#22 survey resolutions** (from `handovers/2026-07-20-email-22-handover.md`):
   - Templating: `mrml` v6.0.1 (MJML -> email-safe HTML) for the chrome, with
     the board render inlined via `mj-raw`; multipart via Resend `text`/`html`.
     Added SSR-gated alongside `svix` + `mail-parser`. (Survey-recommended,
     confirmed maintained.)
   - Theming-in-email: resolve recipient `users.theme` slug -> palette via the
     #26 `themes()` + `slugify` (NULL/system -> brdgme light); render the board
     with a palette-aware transform -> concrete inline colours. Text part is
     unthemed by design.
   - Next free migration: **014** (`014_email_play.sql`).
   - **Resend inbound correction (survey finding):** inbound webhooks are
     metadata-only; the body is fetched via the Received-emails API using
     `email_id`, and the Reply-To routing token is in the `to`/`received_for`
     metadata (routing precedes body fetch). Leads must abstract the body-fetch
     behind a trait so reply parsing is unit-testable without live Resend.
9. **Game creation by email:** deferred to a stretch unit late in the session
   (attempt only if budget allows). Core this round = plays + settings +
   server-commands + reminders + multi-email. The command framework is built
   extensible so creation slots in later. (User wants "everything by email";
   creation is the largest single command and separable - functional-first.)
10. **Settings-by-email auth (v1):** authorised by From matching a verified
    `user_emails` row (resolves to owning user, own-settings only). Settings
    changes are low-severity (no game-state or data-leak impact), so verified-
    From is an acceptable v1 backstop matching the survey recommendation. See
    deferred item D1 for the hardening follow-up.
11. **`rules`/strategy reply command:** implemented in the #25 email unit (its
    committed scope), not as a #22 placeholder. #22's command parser reserves
    the keyword; #25 wires the rendered output.
12. **Infra (Resend receiving API method, `play.brdg.me` MX, webhook
    registration):** OUT of scope per user. Code reads config from env with a
    dev/log fallback and is structured against the documented Resend receiving
    API; provisioning is the user's later task.
13. **22c reminders (v1):** one reminder per turn; threshold env-configurable
    (`TURN_REMINDER_AFTER`, default 24h); reset on every `is_turn` transition.
14. **#25 survey resolutions** (from `handovers/2026-07-20-rules-25-handover.md`):
    - Strategy is fetched LIVE from the game-service V2 endpoints
      (`Request::BasicStrategy`/`AdvancedStrategy`), gated by
      `interface_version` read from DB - there are NO strategy DB columns
      (migration 013 added only `interface_version`). Rules come from DB
      (`game_versions.rules`). #25 needs NO migration.
    - UI: ONE `/rules/{version_id}` page, three anchored sections
      (Rules / Basic Strategy / Advanced Strategy); absent doc -> section
      omitted.
    - Shared render fn `render_doc(markdown, players, style)`; synthetic
      players = `max(player_counts)`, palette = `theme::PLAYER_COLOR_NAMES`
      (8 colours - the spec's 7-colour palette is STALE), semantic-colour path
      so colours follow the viewer's theme; out-of-range `{{player N}}`
      validated upfront -> loud `Err` (the markup lib silently tolerates it).
    - Authoring doc is `docs/authoring/RULES_AUTHORING.md` (not `docs/RULES.md`).
    - Web render uses the SEMANTIC path (`transform_semantic` + `html_class`),
      not the spec's concrete `transform` + `html`.
15. **#25 data-docs section:** keep to the THREE human-facing sections (Rules,
    Basic Strategy, Advanced Strategy). The auto-generated data-docs glossary
    is a technical bot contract, NOT rendered for humans. (User asked for
    rules + basic + advanced strategy only.)
16. **#25 strategy fetch verification:** strategy handlers are static
    `include_str!` ignoring their `game`/`player` args, so an empty `game`
    should work; the #25 implementation Lead verifies this and, only if it
    fails, mints a throwaway state via `Request::New`. (Implementation detail,
    not a user decision.)

### User clarifications received 2026-07-20 (mid-session)

17. **Game creation by email IS in scope this round** (user). Promoted from
    stretch (supersedes decision 9). The email command set gains a game-
    creation command (legacy `new`-style): pick game type + opponents by name +
    bot slots, creating a game (or, once #24 lands, a proposal). Built
    extensibly on the #22 command framework.
18. **Rules by email in scope** (user) - confirms decision 11 (#25 owns it).
19. **Settings-by-email auth: verified sender is fine** (user). Resolves D1 -
    verified-From auth (decision 10) is accepted; NO per-user settings token
    needed.
20. **Suppress proactive game emails when the user is active on the web**
    (user, new requirement): do NOT send a game email to a user who has an
    active web session that was active within the past hour. Mechanism:
    track `users.last_seen_at` (updated by auth middleware on authenticated
    requests and on websocket activity); the email-send path skips proactive
    notifications when `last_seen_at > now() - EMAIL_SUPPRESS_IF_ACTIVE_WITHIN`
    (env, default `1 hour`). Suppression applies to PROACTIVE mails only (turn
    notifications, elimination, game-finished, reminders, invites, digests) -
    NOT to inbound command-response emails (those must always go, or email play
    breaks). Unsubscribed users are still skipped as before.

21. **#24 survey resolutions** (from `handovers/2026-07-20-invites-24-handover.md`):
    - Schema: `game_proposals` + `game_proposal_players` per spec; migration
      **015** (#22=014, #25=none; sequence holds).
    - A started proposal builds a `CreateGameSeed` and routes through the
      existing `create_game_from_service` choke point unchanged; add
      `all_accepted: bool` to `CreateGameOpts` so started games set
      `has_accepted=true` for all.
    - `restart_game_impl` creates a proposal (fixes the restart-drops-bots bug);
      solo-vs-bots restart bypasses.
    - Email seam: a no-op-default `InviteMailer` trait wired into core (called
      at create/decline/cancel/start) so #24 compiles/works in-app pre-email;
      the post-#22b unit fills it in. (Accepted over an AppState fn-pointer.)
    - Start-early below min `player_counts`: BLOCK with a clear validation
      message (owner adds bots or drops to a valid count) - not auto-fill bots.
    - The 2026-07-19 new-game-page rebuild HAS landed (`rust/web/src/new_game.rs`,
      `game_types.blurb`/`weight`); invite UI bases on `new_game.rs`.
22. **#25 backend DONE** (commit `a834d28`): `rust/web/src/rules.rs` -
    `render_doc` (fence scan + upfront `{{player N}}` validation + pulldown-cmark
    prose + semantic markup pipeline), `synthetic_players` (max(player_counts),
    8-colour `PLAYER_COLOR_NAMES`), `fetch_strategy` (V2-gated), `RenderedDocs`,
    `get_rendered_rules` server fn; db.rs `find_game_version_rules` +
    `find_game_version_render_meta` (plain queries). `pulldown-cmark 0.13.4`
    added SSR-only. Empty `game` string WORKS for V2 strategy endpoints (no
    `Request::New` minting needed). `SQLX_OFFLINE=true` required for clippy/test
    on `-p web`. 10 pure-logic tests pass locally; 1 `#[sqlx::test]` compiles
    (runs in CI). User confirmed all default decisions (mrml, settings-by-email
    breadth, active-web suppression) on 2026-07-20.

23. **#22b split into 5 small units** (keep each Lead under budget): a1 migration
    014 + themed `email_render` module (mrml); a2 `users.last_seen_at` activity
    tracking + active-web suppression + outbound notification wiring (recipient
    classes, opt-out); b inbound webhook (svix + body-fetch trait + idempotency)
    + reply parsing; c command dispatcher (game + server + SETTINGS commands) +
    response emails; d game creation by email. Migration 014 ALSO carries
    `users.last_seen_at timestamptz NULL` (null = never active -> send).

24. **COMMIT RULE CHANGE (user, mid-session):** NO committing and NO pushing at
    ANY level for the rest of this session. Maintain
    `~/Development/brdgme-commit-plan.md` (outside the repo) listing every
    commit to perform in a separate commit session (message + files). The 9
    earlier commits this session are already local/unpushed (plan Section A);
    the 4 untracked orchestration docs are Section B; all later units leave
    changes UNCOMMITTED and report files + message -> appended to plan Section C.
    Lead briefs now forbid `git commit`/`git push` and require a file list.
25. **#22b c1 (`deb0742`) + c2 (`abb2d1e`) done:** command dispatcher (concede/
    undo/restart/subscribe/unsubscribe + `rules` + `help` + fall-through to game
    moves) wired into the game-reply loop; settings-by-email (name/colours/theme/
    emails on-off + `settings` summary) in-game AND via a standalone verified-From
    path. The `rules` email command is implemented (#25's reply-command scope is
    DONE; only the "View rules" LINK in notification/invite mails remains).
26. **#22b unit d (game creation by email) DONE** (uncommitted; commit-plan C1):
    `new <gametype> <opponent>...` + `list` commands. `new` resolves the game
    type by name/slug, classifies opponents as bots (easy/medium/hard or
    `bot:<name>`) or humans (by username), validates against `player_counts`,
    creates via `create_game_from_service`, triggers bot turns + turn emails.
    Wired into the standalone verified-From path AND game replies. FOLLOW-UP:
    after #24, convert human-opponent `new` to a proposal; email `new` currently
    bypasses `check_invite_policy_tx`.
27. **#22c turn reminders DONE** (uncommitted; commit-plan C2): tokio sweep
    (`spawn_turn_reminder_sweep` via `spawn_periodic_sweeps`), `FOR UPDATE SKIP
    LOCKED`, candidate = is_turn AND NOT is_eliminated AND reminder NULL AND
    is_turn_at older than `TURN_REMINDER_AFTER` (default 24h), mark-on-success,
    reset `turn_reminder_sent_at` on every is_turn transition (concede/undo/
    command). Sweep interval `TURN_REMINDER_SWEEP_INTERVAL` (default 15m). Three
    db.rs turn-update queries converted macro->plain (dead `.sqlx` entries to
    prune).
28. **#22d-a multi-email backend DONE** (uncommitted; commit-plan C3): add
    (confirmation code via reused login-code machinery) / confirm (verified_at) /
    make-active (one-tx primary switch + capped-20 turn digest, bypasses
    suppression but honours opt-out/bot) / remove (primary protected); login
    resolves ANY verified address; signup creates a verified primary;
    `spawn_unverified_email_sweep` deletes unverified addresses >24h. Errors are
    stable `ServerFnError` message strings + pure predicates (codebase
    convention). Invite-by-email primaries stamped verified.
29. **Bot naming on slot replacement (user):** when replacing a human slot
    with a bot, keep the user's name + " (bot: {difficulty})" suffix.
30. **WS proposal updates broadcast to all clients (user confirmed):** same
    as game updates.
31. **Invite sweep redesign (user, 2026-07-21):** 24h reminder + 48h
    auto-decline replaces the spec's 3d nudge + 14d expiry. One reminder per
    proposal (`nudged_at` on `game_proposals`, migration 016), not per player.
32. **Skip owner email on proposal expiry (user confirmed).**
33. **Nudge email reuses invite email verbatim (autonomous):** no distinct
    "Reminder:" subject prefix.
34. **`emails confirm <code>` targets most recently created unverified
    address (autonomous).**
35. **`emails active`/`emails use` skips turn-digest on the email command
    path (autonomous):** the web settings UI still sends the digest.
36. **Migration 016: `nudged_at` on `game_proposals` (autonomous):** one
    nudge per proposal, not per player.
37. **Expiry notification: `notify_cancelled` emails accepted invitees only,
    not owner (autonomous).**
38. **InviteMailer notification reply address uses
    `i-{proposal_id}@play.brdg.me` (autonomous):** replies gracefully no-op.

## PROGRESS (completed units)

- S1 #22 survey, S2 #25 survey, S3 #24 survey - done (handover docs on disk).
- #25 backend render - done (`a834d28`).
- #25 UI + routing (RulesPage, `/rules/{version_id}`, links) - done (`2377ba8`).
- #22b a1 themed email_render + migration 014 - done (`7ed9951`).
- #22b a2a outbound plumbing (activity/suppression/send/tokens/recipient) - done (`85167e2`).
- #22b a2b proactive notifications (turn/elim/finished, all call sites) - done (`c30d489`).
- #22b b1 inbound parsing/routing/svix - done (`e3b69c6`).
- #22b b2 webhook + play-by-email loop - done (`8450974`).
- #22b c1 command dispatcher (server cmds/rules/help) - done (`deb0742`).
- #22b c2 settings-by-email + standalone path - done (`abb2d1e`).
- (Above 9 commits are local/unpushed - see commit-plan Section A.)
- #22b d game creation by email (`new`/`list`) - done, UNCOMMITTED (plan C1).
- #22c turn reminders sweep - done, UNCOMMITTED (plan C2).
- #22d-a multi-email backend - done, UNCOMMITTED (plan C3).
- #22d-b settings UI email management + `emails add/confirm/active/use/remove`
  commands - done (2026-07-21 session).
- #24 core backend (migration 015, proposals.rs, all_accepted, broadcast,
  restart refactor) - done (2026-07-21 session; SURPRISE: was already
  implemented by a prior session, not reflected in the handover's remaining
  work).
- #24 core UI (InvitePage, `/invites/:id` route, dashboard pending invites,
  new_game.rs repoint to create_proposal) - done (2026-07-21 session).
- #24 email (RealInviteMailer replacing NoopInviteMailer, webhook Invite(token)
  arm for accept/decline-by-email) - done (2026-07-21 session).
- #24 sweeps (24h reminder, 48h auto-decline, 14d expiry) - done (2026-07-21
  session; user design change: originally 3d nudge + 14d expiry, changed to
  24h reminder + 48h auto-decline).
- #25 email ("View rules" link in notification + invite mails) - done
  (2026-07-21 session).
- Cleanup (.sqlx prune, BACKLOG update, handover doc deletion) - done
  (2026-07-21 session).
- All later units are UNCOMMITTED per the commit-rule change; see commit-plan
  Section C.

## REMAINING WORK

- All units complete. Handover docs deleted per user request (temporary
  session docs). See commit-plan for the full uncommitted-changes list.

## CONFIG NOTES FOR PROD (out of scope this session - infra)

- `PUBLIC_BASE_URL` must be set to `https://brdg.me` (defaults to
  `http://localhost:3000`); used for browser links in emails.
- `RESEND_WEBHOOK_SECRET` (svix signing secret) + Resend receiving API key +
  `play.brdg.me` MX + webhook registration - all provisioned later by the user.
- `EMAIL_SUPPRESS_IF_ACTIVE_WITHIN` (default `1 hour`) tunes active-web
  suppression; `TURN_REMINDER_AFTER` (22c) tunes reminders.

## IMPLEMENTATION DECISIONS WORTH REVIEW (made autonomously)

- `send_rendered_email` takes `Option<&Resend>` (not `&AppState` - AppState is
  not a Leptos context; individual items are provided). Works in both server-fn
  and bot-consumer paths.
- Activity-write throttle is per-process (~60s, in-memory); multi-replica allows
  one extra stamp per window - acceptable v1 approximation.
- `is_first_message` for email threading approximated as "game has no logs yet"
  (no per-recipient mail-sent column) - documented in code.

## DECISIONS DEFERRED TO THE USER - review at end of session

(populated as Leads surface large/difficult/impactful choices)

- _none currently outstanding_ (D1 resolved by decision 19; D2 resolved by
  decision 17; #24 open questions resolved by decision 21).

## OPEN ITEMS / KNOWN ISSUES (carried forward through Lead briefs)

- DB tests fail locally without Postgres (#40) - expected, do not chase.
- `THEME_BOOT_SCRIPT` follow-up (#26) - out of scope this session.
- lords-of-vegas-1 non-deterministic shuffle - out of scope.
