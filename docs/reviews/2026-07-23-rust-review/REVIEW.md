# brdgme Rust review - consolidated report

Snapshot under review: `f8763a5ba9c0ce3d0e85d61db7133d19a26ed313`
(worktree `rust/`). Review-only: no code was changed. This document is the
index and synthesis; the per-unit findings files under `findings/` hold the
full finding text and are the authoritative detail. For units 1-9 an
independent verification pass is recorded under `findings/verification/`;
this report uses the verification-corrected tallies and verdicts for those
units.

## 1. Executive summary

### Scope

All Rust code in `rust/` was reviewed in 13 units:

1. lib-game (parser combinator + core traits)
2. lib-support (cmd, game_client, markup, color, cost, rand_bot)
3-8. the 27 game crates in six batches (a-f)
9. web-server (auth, crypto, admin, db.rs, NATS, websockets)
10. web-domain (game pipeline, server fns, proposals, stats, social)
11. web-frontend-email (email subtree + Leptos frontend)
12. bot-operator-tools (bot, k8s operator, fuzz/repl tools)
13. dependencies (40 Cargo.toml, Cargo.lock, deny.toml)

Method: each unit was reviewed against the five charter values
(correctness, quality, simplicity, consistency, dependencies). Units 1-9
(originally reviewed by a prior Kimi K3 session) then received an
independent line-by-line verification with per-finding verdicts
(CONFIRMED / ADJUSTED / REJECTED / UNVERIFIABLE) and corrected severity
tallies. Units 10-13 were reviewed and lead-verified in a single session.
Known non-issues were excluded by charter: the custom serialisable command
parser combinator, the duplicated `impl Parser for CommandSpec`, and
DB-backed tests that fail in plain local runs.

### Overall verdict against the five charter values

- Correctness: mostly strong, with a concentrated set of real defects.
  The dangerous class is a family of char-count-used-as-byte-index panics
  in the hand-rolled parser and markup layers, reachable from ordinary
  user input (3 criticals in lib-game, 1 in lib-support, 1 in red7-1).
  The web layer has no request-path panics but carries concurrency/
  atomicity gaps and privacy gates that were built but never wired.
- Quality: generally high. Parameterised SQL throughout, disciplined
  optimistic locking on the move path, good hidden-info hygiene in most
  game crates, strong test suites in the core libraries. The recurring
  quality drag is fail-open / error-swallowing posture and lifecycle
  edges (bot-turn wedges, unsupervised consumers) that nobody wired
  end-to-end.
- Simplicity: good at the function level; the systemic simplicity cost is
  duplication - the finish/placings epilogue copy-pasted across almost
  every game crate, and 108 near-identical game binaries.
- Consistency: good within crates; the cross-crate gaps are typed-vs-spec
  parser impl drift, docs-vs-code divergence (RULES.md contradicting the
  implementation in many crates), and unification gaps in dependencies.
- Dependencies: core stack currency is genuinely good (tokio, serde, axum,
  leptos, sqlx, aes-gcm all current; known-bad crates absent from the
  lock). The problems are structural: no `[workspace.dependencies]`, a
  sqlx 0.8/0.9 split, oversized feature sets (sentry pulling actix + ureq),
  and a handful of unmaintained/duplicated crates (term_size, serde_yaml,
  combine, warp-beside-axum).

Bottom line: a healthy, well-tested codebase with no systemic rot. The
critical work is a small, sharply-defined set - eliminate the char/byte
panic class, wire the privacy gates, and give the bot-turn/undo/concede
paths the same concurrency discipline the move path already has.

### Headline findings

All 10 criticals:

- lib-game F1/F2/F3 - `Space::parse`, `Token::parse`, `Enum::parse` panic
  on multi-byte input (char count sliced as byte index); reachable
  server-side per command and client-side on every WASM keystroke; F3 via
  non-ASCII player names.
- lib-support F1 - markup `slice()` byte-indexes char offsets; any
  multi-byte char in a `{{canvas}}` layer panics or corrupts output.
- games-batch-b F16 - Alhambra `take()` mints duplicate cards
  (money-duplication exploit from crafted input).
- games-batch-d F34 - Modern Art infinite busy-loop when all hands empty
  after a settle (legally reachable round 4; hang + unbounded log growth;
  Go-inherited).
- games-batch-e F29 - red7-1 CardParser non-ASCII byte-slice panic from
  any current player's command text.
- web-domain - `undo_game` on a finished game corrupts ratings permanently
  (no is_finished guard; reverts state but never rewinds rating_change,
  and the idempotency guard then blocks re-rating the real outcome).
- web-frontend-email (2) - settings route authenticated solely by a
  spoofable `From` header, and email-management commands over that path
  enable full account takeover (redirect all turn emails and game tokens).

Major themes (detailed in section 3): request-reachable panics from the
char/byte class and from unvalidated deserialized state; privacy/visibility
gates built but not wired; TOCTOU/concurrency guards missing on undo/
concede/auth; unvalidated bot slots wedging games; fail-open / error-
swallowing; NATS delivery semantics with no recovery path; Go-port parity
vs official-rules divergences; and dependency-unification gaps.

## 2. Rolled-up tallies

Verification-corrected tallies for units 1-9; heading-verified counts for
units 10-13 (see note below the table). Rejected findings are excluded
(games-batch-d F13, web-server F30).

| # | Unit | Critical | Major | Minor | Nit | Total |
|---|------|:---:|:---:|:---:|:---:|:---:|
| 1 | lib-game | 3 | 4 | 8 | 5 | 20 |
| 2 | lib-support | 1 | 5 | 23 | 16 | 45 |
| 3 | games-batch-a | 0 | 6 | 6 | 8 | 20 |
| 4 | games-batch-b | 1 | 5 | 13 | 16 | 35 |
| 5 | games-batch-c | 0 | 4 | 14 | 16 | 34 |
| 6 | games-batch-d | 1 | 6 | 16 | 22 | 45 |
| 7 | games-batch-e | 1 | 5 | 18 | 22 | 46 |
| 8 | games-batch-f | 0 | 2 | 22 | 34 | 58 |
| 9 | web-server | 0 | 8 | 36 | 22 | 66 |
| 10 | web-domain | 1 | 12 | 37 | 30 | 80 |
| 11 | web-frontend-email | 2 | 13 | 30 | 18 | 63 |
| 12 | bot-operator-tools | 0 | 4 | 16 | 11 | 31 |
| 13 | dependencies | 0 | 4 | 18 | 5 | 27 |
| | **Grand total** | **10** | **78** | **257** | **225** | **570** |

Note: the units 10-13 findings files' own per-unit tally sections
undercount the finding headings actually present in their bodies
(web-domain 80 vs 78 stated, web-frontend-email 63 vs 60,
bot-operator-tools 31 vs 30, dependencies 27 vs 26). The counts above
reflect the actual finding headings, verified during triage (see
`planning/work-packages.md` coverage check and the W5/W6 ID audits in
`planning/raw/`).

Excluded rejected findings (2): games-batch-d F13 (jaipur "8 vs 11 camels"
- the market camels are conjured in `start_round`, so 8+3 = 11 in play and
its recommended fix would have created 14 camels); web-server F30
(`update_bot_provider` "omits updated_at" - the `bot_providers` table has
no `updated_at` column, so the recommended fix would be a runtime SQL
error).

## 3. Cross-cutting themes

### Request-reachable panics (char/byte confusion + deserialized-state trust)

The single most dangerous pattern. Two sub-classes:

- Char-count used as byte index in hand-rolled parsing. lib-game F1/F2/F3
  (criticals), F16; lib-support F1 (critical), F2; games-batch-e F29
  (critical, red7). Reachable from raw command text server-side and from
  the WASM suggest engine on every keystroke (NBSP from iOS autocorrect is
  a live trigger). The core libraries have zero non-ASCII test coverage,
  exactly where the panics live.
- Unvalidated deserialized `Game` state. The requester layer
  (`lib/cmd/src/requester/gamer.rs`) deserializes stored/crafted state
  verbatim, so unchecked indexing and drain paths panic across nearly
  every game crate: games-batch-e F18/F36/F22, games-batch-f
  F45/F46/F4/F9/F26/F34/F44/F53/F55, games-batch-c F25/F10. This is a
  systemic fix at the requester/serde boundary, not per-crate.

### Privacy / visibility gates built but not wired

- web-domain: `get_game_details` and all stats endpoints ignore
  `game_visibility`; `is_game_visible_to_user` is effectively dead code
  (any authed user sees any game; anonymous stats name private users).
- web-domain: `get_proposal` serialises every invitee's `email_token`
  (the inbound-email auth credential) to any authenticated viewer.
- games-batch-f F1/F13 - pub_state hidden-info leaks (zombie-dice cup draw
  order NEW vs Go; for-sale selling-phase secret plays).
- games-batch-a F15 - starship Sensor peek data present in PlayerState
  JSON but never rendered.
- Export bundle includes private logs (web-domain).

### TOCTOU / concurrency guards missing

- web-domain critical + majors: `undo_game` and `concede_game` skip the
  optimistic-locking discipline the move path has (`db::undo_game` /
  `db::concede_game` lack guards - one shared root cause in db.rs).
- web-frontend-email: the email concede/undo paths hit the same db.rs root
  cause, plus a `FOR UPDATE SKIP LOCKED` that is a no-op under autocommit
  `fetch_all` (concurrent replicas double-send).
- web-server: F1 attempt-cap race on login codes, F38 zero-change defeats
  the rating idempotency guard, F39 opposite-direction friend requests.
- bot-operator-tools: ack-after-all-work with no `AckKind::Progress`
  heartbeat yields duplicate turn processing; operator finalizer merge-
  patch race on stale watch-cache data.

### Bot-slot validation and bot-turn delivery

- Client-supplied bot slots are unvalidated at 4 entry points (web-domain
  `create_proposal` / `add_proposal_player` / `restart_core`, plus the
  email `new` command) - a bogus bot name creates an unrecoverably wedged
  game. One shared validation choke point is recommended.
- The bot-turn NATS pipeline has no recovery path for any wedge mode:
  UserError acked without re-publish, retry exhaustion, lost publish after
  DB commit, consumer spawned once with no restart (web-domain majors +
  web-server F27/F53). web-server F27: games reference bots by NAME, so a
  rename/delete/disable silently skips-and-acks and deadlocks in-flight
  games permanently. The ack-heartbeat gap recurs in bot-operator-tools.

### Fail-open / error-swallowing

The most-repeated quality pattern across the web units. web-server:
hardcoded fallback encryption key used with one warn line (F16), Turnstile
fails open (F8), Resend failure silent + quota burn (F7), transient DB
error triggers mass logout (F5). web-domain / web-frontend-email:
fire-and-forget mutations in friends/settings/GameMeta/logout/mailers/
sweeps swallow errors. Game crates: draw/round logs dropped
(games-batch-e F19/F37), write-only stats subsystems
(games-batch-c F11/F12).

### Go-port parity vs official rules

A large cross-cutting decision batch: many crates faithfully reproduce
their Go origin while diverging from official rules, and RULES.md
sometimes documents the Go behaviour and sometimes contradicts the code.
The modern-art cluster (games-batch-d F34/F35/F36/F37/F43), seven-wonders
deviations (games-batch-b F3/F6/F7), splendor tie-break (F29), red7
empty-winning-set (games-batch-e F30), player-count caps
(games-batch-c/f), and the games-batch-f parity batch
(F2/F14/F15/F24/F43/F50/F54) all need one project-level "port parity vs
official rules" policy plus per-game adjudication. Verification refuted two
premises here (sushi-go pudding tiebreak and cathedral flood-fill are both
correct per in-crate RULES.md), establishing "documented in-crate wins".

### Boilerplate duplication

The finish/placings epilogue is copy-pasted across almost every game crate
(6x in seven-wonders, alhambra; 5x splendor, texas-holdem; and so on), and
27 crates x 4 near-identical binaries = 108 boilerplate files
(dependencies unit). web-server duplicates the admin gate 15x verbatim.

### Dependency unification and hygiene

No `[workspace.dependencies]` (serde copied across 36 crates, already
drifting); sqlx 0.8 (web) vs 0.9 (bot/operator); sentry default features
drag actix-web + ureq into every server build; term_size
(RUSTSEC-2020-0163), serde_yaml (archived), combine (dormant), warp beside
axum. deny.toml exists but is toothless at warn level with 4 stale
advisory ignores for crates absent from the lock.

## 4. Per-unit sections

Each unit links its findings file and, for units 1-9, its verification
report. Finding IDs are per-unit (each unit numbers from F1).

### Unit 1 - lib-game (3c / 4M / 8m / 5n)

High-quality crate with strong tests (~70 suggest tests, typed/spec parity
guards) and clean rng/game/errors/chain modules. One systematic defect
class dominates: char/byte unit confusion produces all three criticals
(F1/F2/F3), a major (F4), and a nit (F16). Secondary class: typed-vs-spec
impl divergence (F8/F13/F14) not covered by the parity tests; F7 shows the
OneOf furthest-error ranking is dead code (all offsets provably 0).
Findings: `findings/lib-game.md`. Verification:
`findings/verification/lib-game.md` (20/20 CONFIRMED).

### Unit 2 - lib-support (1c / 5M / 23m / 16n)

Quality varies by crate. game_client and cost are near-clean; markup
carries the worst defects (F1 critical byte/char slice; F2 panic-on-
overflow; F3 silent truncation of malformed markup); color's issue is a
regex + lazy_static footprint serving a dead parse API (F12); cmd is
panic-heavy in dev tools with one production-path unwrap (F19 warp
handler) and an unbounded game_client timeout (F31). Findings:
`findings/lib-support.md`. Verification:
`findings/verification/lib-support.md` (43 CONFIRMED, 2 ADJUSTED wording
only).

### Unit 3 - games-batch-a: roll-through-the-ages-2, starship-catan-1 (0c / 6M / 6m / 8n)

rtta-2 is a careful, heavily-annotated Go port with one real logic bug (F1
phase re-match after keep_skulls) plus RULES.md-vs-code contradictions.
starship-catan-1 is structurally sound but has three economy/rules bugs
reachable by legal play (F11 cannon surcharge, F12 sacrifice-to-skip, F13
no affordability check), a reachable i32 overflow (F14), and a feature-
breaking render gap (F15 Sensor peek). Findings:
`findings/games-batch-a.md`. Verification:
`findings/verification/games-batch-a.md` (19 CONFIRMED, 1 ADJUSTED).

### Unit 4 - games-batch-b: seven-wonders-1, alhambra-1, splendor-2 (1c / 5M / 13m / 16n)

Alhambra carries the worst defects: F16 (critical) mints duplicate cards,
plus F17/F18 index/scoring majors. seven-wonders has a permanent soft-lock
(F2 DrawDiscard resolver), a scoring omission (F1 Halicarnassus VP), and
undocumented official-rules deviations (F3). splendor-2 is clean apart from
Go-parity quirks. Findings: `findings/games-batch-b.md`. Verification:
`findings/verification/games-batch-b.md` (31 CONFIRMED, 4 ADJUSTED; F9
upgraded nit->minor - its "unreachable" claim was false).

### Unit 5 - games-batch-c: texas-holdem-2, acquire-1, cathedral-2, sushizock-2 (0c / 4M / 14m / 16n)

No criticals. Majors: F7 (acquire never offers 6 players - the headline
count), F8 (2p dummy die never rolls 6), F22 (cathedral `Box::leak` per
parser construction - a traffic-driven memory leak in the long-running
service), F29 (sushizock i32::MIN overflow panic in default builds).
Findings: `findings/games-batch-c.md`. Verification:
`findings/verification/games-batch-c.md` (33 CONFIRMED, 1 ADJUSTED; F23
downgraded minor->nit - RULES.md documents the behaviour).

### Unit 6 - games-batch-d: lords-of-vegas-1, jaipur-2, sushi-go-2, modern-art-2 (1c / 6M / 16m / 22n)

modern-art-2 is the problem crate: F34 (critical) infinite busy-loop, F35
deadlock, and a scoring cluster (F36/F37) all inherited verbatim from Go.
lords-of-vegas-1 is an explicitly partial port with structural gaps (F1
`unimplemented!()` one wiring line from reachable, F4 render underflow
reachable in ordinary 5-6p play). jaipur-2 F14 is a genuine scoring defect
(no bonus token for 6/7-card sales). Weakest-verified unit: one rejected
finding (F13), one refuted premise (F26), two majors moved in opposite
directions. Findings: `findings/games-batch-d.md`. Verification:
`findings/verification/games-batch-d.md` (38 CONFIRMED, 7 ADJUSTED, 1
REJECTED).

### Unit 7 - games-batch-e: love-letter-2, age-of-war-2, lost-cities-1/-2, red7-1 (1c / 5M / 18m / 22n)

Solid ports with strong hidden-info hygiene. F29 (critical) red7 non-ASCII
CardParser panic from player command text. Majors: F17 (lost-cities-2 3p
stats hardcoded to players 0/1), F18/F36 (unchecked `hands[player]` index
panics via crafted PlayerRender), F30 (red7 zero-rule-fulfilling player
treated as winning). Verification called this the strongest batch.
Findings: `findings/games-batch-e.md`. Verification:
`findings/verification/games-batch-e.md` (42 CONFIRMED, 4 ADJUSTED; F37
downgraded major->minor for internal consistency).

### Unit 8 - games-batch-f: zombie-dice-2, battleship-2, for-sale-2, category-5-2, greed-2, farkle-2, tic-tac-toe-2, no-thanks-2, liars-dice-2 (0c / 2M / 22m / 34n)

Nine smaller ports in good shape - no criticals, no panic reachable from
crafted command input. Both majors are pub_state hidden-info leaks: F1
(zombie-dice cup draw order, NEW vs Go) and F13 (for-sale selling-phase
secret plays). Dominant secondary class is the systemic all-pub
Deserialize state-trust shape. Findings: `findings/games-batch-f.md`.
Verification: `findings/verification/games-batch-f.md` (54 CONFIRMED, 4
ADJUSTED, tally unchanged).

### Unit 9 - web-server (0c / 8M / 36m / 22n)

The security- and operations-critical unit. Code quality is generally high
(parameterised SQL, no request-path panics, fail-closed admin gating,
strong auth state-machine tests). Majors cluster in concurrency/atomicity
(F1 attempt-cap race, F2 email-squatting DoS), fail-open posture (F16
hardcoded fallback encryption key, F52 missing Secure cookie flag), and
unwired lifecycle edges (F27 bot rename deadlocks games, F34 undo doesn't
clear ratings, F53 unsupervised bot consumer). Findings:
`findings/web-server.md`. Verification:
`findings/verification/web-server.md` (57 CONFIRMED, 8 ADJUSTED, 1
REJECTED, 1 UNVERIFIABLE; F1 crit->major, F27 minor->major, F18
major->minor, F58 minor->nit).

### Unit 10 - web-domain (1c / 12M / 37m / 30n)

~14.2k LOC of web domain logic. Core paths are solid (optimistic locking on
moves, FOR UPDATE in restart/proposals, authz on nearly all server fns);
defects cluster at the edges. The critical is `undo_game` corrupting
ratings on a finished game. Majors: bot-turn NATS pipeline has no recovery
for any wedge mode (4 majors), undo/concede skipped the move path's
concurrency discipline, `game_visibility` privacy model never wired to read
endpoints, `email_token` serialised to all proposal viewers, unvalidated
bot slots x3, auto-decline keyed on wrong timestamp, rules-page version
picked by `ORDER BY name`. Findings: `findings/web-domain.md`
(lead-verified in session; no separate verification report).

### Unit 11 - web-frontend-email (2c / 13M / 30m / 18n)

~9.7k LOC email subtree + Leptos frontend. The rendering/threading/escaping
layer and hydration discipline are strong. Two criticals compose into
account takeover: settings route authenticated by spoofable `From`, and
email-management commands over that path. Majors: idempotency marker
inserted before processing (post-marker failures permanently dropped),
mark-before-do in every sweep, `FOR UPDATE SKIP LOCKED` no-op under
autocommit, unvalidated `bot:` opponents, concede/undo TOCTOU + duplication
of the server fns, forgeable From composing with web-domain's email_token
leak. Findings: `findings/web-frontend-email.md` (lead-verified in
session).

### Unit 12 - bot-operator-tools (0c / 4M / 16m / 11n)

~2.7k LOC; zero criticals and a long clean list. Dominant class is
lifecycle/robustness gaps in long-running async work: F-class majors are a
reachable `unreachable!()` in the bot retry loop (panics the task, leaves
message unacked), no ack-deadline extension for long turns (duplicate
command submission), hand-rolled finalizer handling in the operator (stale
watch-cache clobber), and the fuzzer hanging forever on worker failure
(live Sender never disconnects). Findings: `findings/bot-operator-tools.md`
(lead-verified in session).

### Unit 13 - dependencies (0c / 4M / 18m / 5n)

Core stack currency is genuinely good; the problems are structural. Four
majors: no `[workspace.dependencies]` (shared versions copy-pasted across
40 manifests, already drifting); sqlx 0.8/0.9 split (both stacks in the
lock, two type-mapping behaviours against one DB); sentry default features
drag actix-web + ureq into every server build; term_size 0.3.2 unmaintained
(RUSTSEC-2020-0163, direct dep of every game binary). Findings:
`findings/dependencies.md` (lead-verified in session).

## 5. Verification summary (units 1-9)

An independent pass re-checked every finding in units 1-9 against the
snapshot. Verdicts across all 9 units (371 findings examined):

| Verdict | Count |
|---------|:---:|
| CONFIRMED | 337 |
| ADJUSTED | 31 |
| REJECTED | 2 |
| UNVERIFIABLE | 1 |

The original review was highly accurate - 337/371 confirmed exactly,
including subtle char/byte arithmetic traces and cross-crate reachability
claims. ADJUSTED verdicts were mostly wording/count refinements that left
severity intact.

Notable severity changes:

- Upgrades: games-batch-b F9 (nit->minor, "unreachable" claim was false);
  games-batch-d F4 (minor->major, render underflow reachable in ordinary
  5-6p play); web-server F27 (minor->major, bot rename deadlocks in-flight
  games - resolved from UNCERTAIN).
- Downgrades: games-batch-c F23 (minor->nit, documented in RULES.md);
  games-batch-d F15 (major->minor, no in-repo rules source), F26/F36
  (premises documented in-crate); games-batch-e F37 (major->minor,
  internal consistency); web-server F1 (critical->major, race is a bounded
  multiplier not unbounded brute force), F18 (major->minor, admin-only
  display), F58 (minor->nit).
- Rejections: games-batch-d F13 and web-server F30 (both excluded from
  tallies; both had recommendations that would introduce a bug or a SQL
  error).

Invalid / flawed recommendations caught (important - see below). At least
these, and web-server carried a dense cluster of them:

- games-batch-d F13 - recommended `Camel => 11` would create 14 camels.
- games-batch-e F45 - "move binary-only deps to `[dev-dependencies]`" is
  unsound: dev-dependencies do not apply to `src/bin/` targets; correct fix
  is optional deps + required-features on `[[bin]]` or a separate bin
  crate.
- games-batch-f F18 - the `#[serde(default)] phase` migration is unsound
  (Phase defaults to Buying, so live mid-Selling games would deserialize
  wrong); use `Option<Phase>` with fallback.
- web-server F30 - rejected outright (fix is a runtime SQL error);
  plus flawed recommendations on F6 (would under-count the send cap), F1
  (off-by-one, `>` not `>=`), F4 (would lock out existing users), F34
  (double-counts ratings), F48 (breaks an existing test), F52 (forces
  Secure on the dev overlay), F66 (breaks non-ssr `cargo test`).

Implication: recommendations in the findings files are a starting
direction, not validated fixes. Several were shown to be wrong or harmful.
Every recommendation must be re-validated at fix time before implementation.

## 6. Requires discussion / design decision

These findings are not mechanical fixes - they need a semantics call,
product judgment, or migration plan. This list feeds a later backlog/spec
effort. Grouped; 34 items.

### Platform / infrastructure (16)

1. Email `From`-header authentication redesign (web-frontend-email
   criticals + forgeable-From major): per-user secret settings tokens, drop
   the `None` fallthrough, whether to require Resend SPF/DKIM verdicts,
   whether account-security commands belong on the email path at all.
2. Sweep/webhook delivery semantics (web-frontend-email, web-domain):
   at-least-once (mark after success, 5xx for svix retry, claim-then-send
   in sweeps) vs the current at-most-once mark-before-do; sync-vs-enqueue
   for the webhook given the svix 15s timeout.
3. Undo-vs-ratings semantics (web-server F34, web-domain critical): may a
   finished game be undone at all, and if so do ratings rewind atomically
   or recompute (recompute alone double-counts)?
4. concede/undo TOCTOU (web-domain, web-frontend-email): fix once in
   `db::undo_game` / `db::concede_game`, and decide whether to also unify
   the duplicated email vs server-fn paths.
5. Bot-turn wedge recovery + NATS delivery (web-server F27, web-domain,
   bot-operator-tools): per-error re-publish vs a reconciliation sweep;
   reference bots by id (new migration) vs warn-on-rename; ack-heartbeat vs
   raising ack_wait; what the worker does with an unresolvable bot.
6. `game_visibility` scope (web-domain): which read endpoints the setting
   gates (index/feeds only vs game details, stats, history); anonymize vs
   filter for stats.
7. Export bundle privacy (web-domain): accept private logs/hidden state in
   bundles, or add `--redact-private`.
8. Unvalidated bot slots - single shared validation choke point across the
   4 entry points (web-domain x3 + email x1).
9. Email canonicalization policy (web-domain): trim+lowercase at boundaries
   vs enforcing lowercased storage globally (touches the unique constraint
   and existing rows).
10. Unsubscribe RFC 8058 compliance (web-frontend-email): build an HTTPS
    one-click endpoint (Gmail/Yahoo bulk rules) vs mailto-only with the
    Post header dropped.
11. Reminder preference semantics (web-frontend-email): which flag governs
    reminders (`reminder_emails_enabled` vs `turn_emails_enabled`).
12. Fail-open posture (web-server F8/F16): explicit policy on Turnstile and
    the encryption key failing open vs startup refusal / dev opt-in.
13. `/ws` unauthenticated site-wide firehose (web-server F59):
    accept-and-document at current scale vs per-connection subscription
    filtering and/or session requirement.
14. Auth edges (web-server): session-token expiry / "log out everywhere"
    (F11), email-squatting remediation semantics (F2), send-cap windowed
    accounting (F6), atomic attempt-cap redesign (F1).
15. Reserved email verbs vs game move grammars (web-frontend-email):
    document the reservation or add an escape prefix.
16. Turnstile rendering after client-side nav to /login
    (web-frontend-email): explicit `render()` vs forcing full-page load.

### Dependencies / build (9)

17. sqlx 0.8/0.9 unification: web is pinned by tower-sessions-sqlx-store;
    wait for an sqlx-0.9-compatible release vs vendor the trivial session
    store, then move sqlx to workspace deps.
18. sentry feature trim: which feature set to keep; verify actix/ureq drop
    out via cargo tree while preserving the deliberate native-tls transport.
19. `[workspace.dependencies]` / `[workspace.package]` / `[workspace.lints]`
    migration: 40-manifest touch; natural umbrella for several other fixes.
20. 108 boilerplate game binaries: a `brdgme_game_bins!(Game)` macro vs one
    generic parameterised bin crate; affects every game manifest and where
    tokio/fuzz deps live.
21. serde_yaml migration (archived): bot + lib/game_client must move
    together; fork (serde_yaml_ng / serde-yml / saphyr) vs switching the
    surface to JSON.
22. warp -> axum consolidation in lib/cmd: small surface but touches all 28
    game binaries' HTTP layer.
23. deny.toml hardening: flip multiple-versions to deny only after
    enumerating current duplicates in skip/skip-tree; clear the 4 stale
    ignores.
24. combine dependency: accept as recorded risk vs migrate brdgme_markup to
    winnow / in-house combinator when next touched.
25. lib/cost consolidation: fold into seven-wonders-1 vs port splendor-2
    onto it (the half-shared status quo is the worst option).

### Game port-parity vs official rules (9)

26. Modern Art cluster (games-batch-d): F34/F35 round-4 end semantics, F36
    all-purchases payout (RULES.md documents it - may canonize a Go
    defect), F37 zero-card artists awarded, F43 tie-break.
27. seven-wonders deviations (games-batch-b F3/F6/F7): discard coins,
    sacrifice-to-discard, both wonder sides dealt (no Go source in snapshot;
    F7 fix also perturbs RNG draw ordering).
28. splendor prestige tie-break (games-batch-b F29): most cards (Go, test-
    locked) vs official fewest cards.
29. red7 empty-winning-set (games-batch-e F30): adopt official "cannot win"
    (needs a defined all-empty outcome) or document the deviation.
30. Player-count caps vs official: texas-holdem 8 vs 9 (games-batch-c F2),
    category-5 8 vs 10 (games-batch-f F24), lords-of-vegas 2-6 vs 2-4
    (games-batch-d F12), no-thanks 3-5 edition (games-batch-f F50).
31. acquire edition behaviours (games-batch-c): random start player (F13),
    full-hand redraw (F14), bag-exhaustion mid-turn end (F15) - no Go port
    to match.
32. jaipur (games-batch-d): next-round starter needs the rulebook quote
    (F15), camel token in tie-break (F16).
33. pub_state redaction design (games-batch-f F1/F13): zombie-dice cup
    order and for-sale bids - counts-only/canonicalized vs per-player
    private field; Go parity vs privacy.
34. rtta-2 fidelity policy (games-batch-a F1): the crate deliberately
    preserves other Go quirks; this one produces objectively wrong state
    and diverges from its own next-path test, but the fix diverges from Go.

Smaller design questions also raised but foldable into the above: lib-game
parser design (OneOf offset propagation F7, typed-vs-spec `expected`
divergence F13, case-folding convention F17); lib-support Color parse API
delete vs keep (F12) and word_wrap space handling (F8); starship peek
rendering and BTreeSet state-shape (games-batch-a F15/F20).
