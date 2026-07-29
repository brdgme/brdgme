# 2026-07-23 Rust review - summary

A comprehensive review of all Rust code in `rust/` (snapshot
`f8763a5ba9c0ce3d0e85d61db7133d19a26ed313`), split into 13 units and 570
findings (10 critical / 78 major / 257 minor / 225 nit), followed by a
remediation program of ~85 work packages. Remediation is COMPLETE: every
landed work package shipped and all Tier-3 verification checklists finished
on 2026-07-29. This is the only retained file from the review directory;
full per-finding and per-work-package detail lives in git history under this
directory's commit range (see the final line).

## Headline findings

- lib-game: three critical char-count-used-as-byte-index panics in the
  hand-rolled parser (`Space`/`Token`/`Enum`), reachable server-side per
  command and client-side on every WASM keystroke; secondary typed-vs-spec
  parser drift. Zero non-ASCII test coverage exactly where the panics lived.
- lib-support: markup `slice()` byte-indexed char offsets (critical panic /
  corruption on any multi-byte char); color carried a dead regex/lazy_static
  parse API; cmd had dev-tool panics and an unbounded game_client timeout.
- games batches (a-f): one critical per of alhambra (duplicate-card mint),
  modern-art (infinite busy-loop), red7-1 (non-ASCII CardParser panic);
  systemic unvalidated-deserialized-state indexing across ~15 crates;
  pub_state hidden-info leaks (zombie-dice cup order, for-sale bids);
  copy-pasted finish/placings epilogue and Go-port-vs-official-rules drift.
- web server: no request-path panics and fail-closed admin gating, but
  concurrency/atomicity gaps (login attempt-cap race), fail-open posture
  (hardcoded fallback encryption key, Turnstile fails open), and unwired
  lifecycle edges (bot rename deadlocks in-flight games, unsupervised
  consumer).
- web domain: critical `undo_game`-on-finished-game permanent rating
  corruption; bot-turn NATS pipeline with no recovery for any wedge mode;
  undo/concede skipping the move path's optimistic-locking discipline;
  `game_visibility` privacy model and `email_token` leak never wired.
- web frontend / email: two criticals composing into account takeover -
  settings route authenticated by a spoofable `From` header, plus
  account-management commands over that path; idempotency markers inserted
  before processing; `FOR UPDATE SKIP LOCKED` no-op under autocommit.
- bot / operator / tools: lifecycle/robustness gaps in long-running async
  work - reachable `unreachable!()` in the bot retry loop, no ack-deadline
  extension for long turns, hand-rolled operator finalizer races, fuzzer
  hanging forever on worker failure.
- dependencies: core stack currency genuinely good; structural problems - no
  `[workspace.dependencies]`, a sqlx 0.8/0.9 split, sentry dragging
  actix-web + ureq into every build, and unmaintained/duplicated crates
  (term_size RUSTSEC-2020-0163, archived serde_yaml, combine, warp-beside-axum).

Cross-cutting themes: request-reachable panics (char/byte confusion +
deserialized-state trust); privacy/visibility gates built but not wired;
missing TOCTOU/concurrency guards on undo/concede/auth; unvalidated bot
slots wedging games; fail-open / error-swallowing; NATS delivery semantics
with no recovery path; Go-port parity vs official-rules divergences; and
dependency-unification gaps.

## What was fixed

Core libraries:
- WP-01 - char/byte panic elimination across lib-game, lib-support, red7-1.
- WP-02 - markup robustness and dedup (D-37 literal-brace escape).
- WP-03 - lib-game parser mechanical fixes.
- WP-04 - lib-game parser design items (D-38).
- WP-05 - lib/color dead parse API deleted, regex/lazy_static dropped (D-39).
- WP-06 - lib/cmd tools and http hardening.
- WP-07 - game_client error enum, timeout, rand_bot panic-free.
- WP-08 - finish/placings epilogue dedup across 11 crates (+WP-08b riders).
- WP-09a/b - deserialized-state trust: requester boundary check + per-game
  `Gamer::validate` hook in 14 crates (D-36; folded in WP-80 ttt-2).

Game crates:
- WP-10 - pub_state hidden-info redaction: zombie-dice cup, for-sale bids,
  starship Sensor peek (D-33).
- WP-13 - starship-catan-1 fixes.
- WP-14 - alhambra-1 core fixes (duplicate-card critical).
- WP-15 - seven-wonders-1 mechanical fixes.
- WP-17 - splendor-2 ported onto lib/cost; generic get/set added (D-25).
- WP-18 - texas-holdem-2 cleanup.
- WP-19 - acquire-1 fixes.
- WP-21 - cathedral-2 (Box::leak memory leak) + sushizock-2 (overflow).
- WP-22 - lords-of-vegas-1 fixes.
- WP-23 - jaipur-2 fixes.
- WP-24 - sushi-go-2 fixes.
- WP-25 - modern-art-2 liveness (infinite busy-loop critical) + cleanup.
- WP-27 - love-letter-2 + age-of-war-2 fixes.
- WP-28 - lost-cities-1/-2 shared fixes.
- WP-29 - red7-1 cleanup (partial; DATA_DOCS task parked to WP-30).
- WP-31 - zombie-dice-2 + battleship-2 fixes.
- WP-32 - for-sale-2 + category-5-2 fixes.
- WP-33 - small-crate cleanup (greed/farkle/ttt/no-thanks/liars-dice).
- WP-81 - dead per-game stats machinery removed (D-40).
- WP-83 - parity fixes released from the rules park (a F1, b F7, e F30).

Web server:
- WP-34 - auth races + session mechanical, Turnstile client.
- WP-35 - auth edge semantics + fail-closed posture (D-12/D-14).
- WP-36 - crypto and deploy hardening.
- WP-37 - admin.rs pass.
- WP-38 - bot-turn wedge recovery: sweep + retry alert + Progress heartbeat (D-05).
- WP-39 - bot consumer supervision.
- WP-41 - db.rs quality pass.
- WP-43 - web cargo deps (metadata cap, feature trims).
- WP-68 - term_size -> terminal_size (dropped RUSTSEC-2020-0163).
- WP-82 - db.rs module split (superseded WP-78).

Web domain:
- WP-40 - undo/concede TOCTOU + ratings integrity; finished-game undo guard (D-03/D-04).
- WP-44 - proposals integrity + email_token leak closed.
- WP-45 - bot-slot validation choke point at all entry points (D-08).
- WP-46 - sweep delivery semantics, at-least-once (D-02/D-11).
- WP-47 - game_visibility gates + stats anonymization (D-06/D-13).
- WP-48 - export/import made admin-only, full bundle (D-07 overruled).
- WP-49 - rules + game-info pages public, version ordering fixed.
- WP-50 - email canonicalization at boundaries + migration (D-09).
- WP-51 - invite-mailer + notify dedup.
- WP-52 - stats + query performance pass.
- WP-53 - domain misc server fns.
- WP-79 - game-service HTTP call hoisted out of the FOR UPDATE transaction.

Web frontend / email:
- WP-54 - frontend UX error handling.
- WP-55 - Turnstile SPA rendering via hard navigation (D-16).
- WP-56 - email From-auth redesign (per-user tokens) + SPF/DKIM classification (D-01).
- WP-57 - inbound webhook delivery semantics, at-least-once (D-02).
- WP-58 - one-click unsubscribe RFC 8058 (D-10).
- WP-59 - inbound processing quality.
- WP-60 - outbound tokens, metrics, render hardening.
- WP-76 - notify_game_emails wired for email-originated moves + game-start paths.
- WP-77 - default bot_name derived client-side from available bots.
- WP-84 - /ws migrated to two SSE streams, WebSockets deleted (D-44..D-52).

Bot / operator / tools:
- WP-61 - bot service quality (merge-patch, crypto nonce, markup resolve).
- WP-62 - operator (finalizer helper, game_types resolution guard).
- WP-63 - fuzz tool (hang-forever fix, timing, parallelism).

Dependencies / build:
- WP-64 - workspace dependencies/package/lints migration (D-19).
- WP-65 - workspace hygiene + weekly deps-currency job.
- WP-66 - sqlx unified 0.9 workspace-wide; session store vendored (D-17).
- WP-67 - sentry trimmed to explicit features, no functionality lost (D-18).
- WP-69 - deny.toml hardened: bans + sources deny, stale ignores cleared (D-23).
- WP-70 - serde_yaml -> serde_yaml_ng, byte-identical (D-21; backend half open).
- WP-71 - warp -> axum in lib/cmd (D-22).
- WP-72 - combine accepted as recorded risk in deny.toml (D-24).
- WP-73 - 108 game binaries collapsed to macro-free brdgme_game_bin (D-20/D-41/D-43).

## Decisions and policies (and where they now live)

| Decision | New home |
|---|---|
| Deprecated-game-crate policy (A2) | docs/CODING.md (Game Services) |
| No breaking stored-state-shape changes (A7) | docs/CODING.md (Game Services) |
| Finished game not undoable (D-03) | docs/CODING.md (Game Services) |
| Strategy-doc split BASIC/ADVANCED (N-4) | docs/CODING.md (Game Services) |
| Macro wariness / brdgme_game_bin macro-free (D-20) | docs/CODING.md (General Principles + Game Services) |
| Email canonicalization (D-09) | docs/CODING.md (Inbound and Webhook Authentication) |
| Fail closed in prod / Turnstile reject (D-12) | docs/CODING.md |
| No session expiry + email-change re-verification (D-14) | docs/CODING.md |
| Refuse startup on missing DATABASE_ENCRYPTION_KEY | docs/CODING.md (Database) |
| pub_state redaction (D-33) | docs/ARCHITECTURE.md (Game Interface Contract) |
| Deserialized-state not trusted / Gamer::validate (D-36) | docs/ARCHITECTURE.md |
| SSE topology (D-44/45/47/48/49/50/52) | docs/decisions/SSE_TOPOLOGY.md |
| Bots referenced by name / validate-on-write tolerate-on-read (D-05/D-08) | docs/decisions/BOT_REFERENCES_BY_NAME.md |
| Port parity (D-35) | docs/decisions/PORT_PARITY.md |
| Dependency trim must not lose functionality (D-18) | docs/decisions/SENTRY_SAAS_EXCEPTION.md |
| Fuzzer throughput evaluation (D-43/D-51) | docs/fuzz-throughput-evaluation.md |
| Disk-pressure stop-and-report rule | AGENTS.md (Resource constraints) |

Already documented elsewhere (no migration needed): no-global-install
(AGENTS.md / docs/CODING.md / docs/DEV.md), migration immutability
(AGENTS.md), dependency-currency (docs/DEPENDENCY-CURRENCY.md), command-parser
dedup (docs/decisions/COMMAND_PARSER_SPEC_DEDUP.md).

## Parked / deferred

Deliberate product holds, not unfinished remediation:

- WP-11/12/16/20/26/30 - per-game rules adjudication, blocked on a rules
  review (BACKLOG #53).
- D-26..D-32 / D-34 - unruled per-game parity items; the global policy
  (official rules authoritative, no gameplay change without per-game
  sign-off) is recorded, the detailed per-rule list is retained only in git
  history (BACKLOG #53).
- WP-74/75 - red7-1 rules / strategy-doc content, queued behind the same
  park (BACKLOG #53).
- WP-85 - email escape-hatch verb set; D-15 settled dispatch (game parser
  first, platform fallback) but the reserved verb membership is deliberately
  undecided (D-55) (BACKLOG #56).
- dp-F14 backend half - unsafe-libyaml 0.2.11 still in Cargo.lock via
  serde_yaml_ng after the WP-70 front half landed (BACKLOG #57).
- Non-default build targets not CI-gated - non-ssr `cargo test -p web` never
  compiled; wasm clippy `-D warnings` fails on pre-existing lints (BACKLOG #58).

---

Full review artifacts - per-unit findings, verification reports, specs,
Tier-3 checklists, the decision record D-01..D-56, and the execution
tracker - are retained in git history under this directory's commit range
(`f0589894..868094a6`).
