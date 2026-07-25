# Consolidation notes: units 7-9

Source: findings/{games-batch-e,games-batch-f,web-server}.md plus their
verification reports. F-IDs are the verification-report IDs (per-unit
namespaces; "U7 F1" != "U8 F1").

## Unit 7: games-batch-e

Crates: love-letter-2, age-of-war-2, lost-cities-2, red7-1, lost-cities-1,
plus the shared boilerplate-binary section.

### Tallies

- Verification-corrected: 1 critical / 5 major / 18 minor / 22 nit (46 findings).
- Original per-finding count differed: 1 critical / 6 major / 17 minor / 22 nit
  (F37 downgraded major -> minor). Findings file carries no header tally.
- Rejected: none. All 46 survive.

### Verification

- Verdicts: 42 CONFIRMED, 4 ADJUSTED (F9, F28, F37, F45), 0 REJECTED,
  0 UNVERIFIABLE.
- F37 (lost-cities-1 draw logs dropped) major -> minor: identical defect to
  lost-cities-2's F19 which the same review rated minor; log-only loss, state
  correct. Internal-consistency alignment, not a fact error.
- F9 (mod test naming) premise inverted: `mod test` is the game-crate majority
  (17 vs 10); recast as workspace-wide inconsistency, nit stands.
- F28 (.rls.toml "malformed") detail wrong: file is well-formed
  `build_lib = true`; the quoted "truetarget" was a cat-concatenation artifact.
  Stale-file substance stands.
- RECOMMENDATION INVALID: F45 (binary-only deps as lib `[dependencies]`) —
  "Cargo `[dev-dependencies]` do NOT apply to src/bin/ targets — the proposed
  move would break every game binary. Correct fix: optional deps +
  required-features on [[bin]], or a separate bin crate." Also the
  transitive-build impact claim is currently vacuous (no in-repo library
  consumers of game crates); the tokio "full" feature trim is the realizable win.
- CODING.md scope note (F2/F10 framing): the no-panic rule (docs/CODING.md:46-49)
  literally scopes to server handlers/DB/Leptos; applying it to game crates is
  extension by analogy (defensible: game code runs in the game-service handler).
- Verifier called this the strongest batch so far: accurate line numbers
  throughout, headline critical and both request-reachable panics reproduced
  end-to-end.

### Headlines

Critical:
- F29 — red7-1 CardParser non-ASCII byte-slice panic — guard counts chars but
  slices bytes; `play r€` from any current player panics the game-service
  request; reachability traced end-to-end, no catch_unwind.
  game/red7-1/src/command.rs:31.

Notable majors (all 5):
- F17 — lost-cities-2 finished stats hardcoded to players 0/1 — 3-player
  finished games silently omit player 2's stats. game/lost-cities-2/src/lib.rs:534.
- F18 — lost-cities-2 player_state() unchecked `hands[player]` index — crafted
  PlayerRender request (server-side envelope) panics the handler; kept major
  not critical since index is not player command text.
  game/lost-cities-2/src/lib.rs:570.
- F36 — lost-cities-1 same player_state() unchecked-index defect (hands always
  len 2; player >= 2 panics). game/lost-cities-1/src/lib.rs:566.
- F30 — red7-1 zero-rule-fulfilling player treated as winning — under Green/
  Violet rules with all-empty winning sets, first non-eliminated player is
  "leader"; officially cannot win. Survives `done`, allows illegal discards,
  0-point round wins. game/red7-1/src/card.rs:297.
- F1 — love-letter-2 command() duplicates the ~20-line finish/response wrap-up
  in all 8 arms (~140 lines copy-paste). game/love-letter-2/src/lib.rs:698.

### Unit state

Five card-game crates (~7.6k LOC); Go originals exist only for love-letter and
age-of-war (both verified line-by-line, no rules divergence); lost-cities-1/-2
and red7-1 judged against official rulebooks. Overall solid ports with strong
hidden-info hygiene; dominant problem classes are latent/request-reachable
panic paths (unchecked indexing, unreachable!, unwrap clusters) and drift
introduced by lost-cities-2's 3-player generalization (F17, F23, F24, F27).
lost-cities-1 is deliberately deprecated-but-deployed; its duplication with -2
is by design.

### Theme evidence

- Request-reachable panics: F29 (critical, player command text); F18, F36
  (crafted PlayerRender envelope); F2, F3, F10, F22 (unreachable!/unwrap on
  runtime paths, corrupt-state reachable); F26, F41 (underflows, latent).
- Deserialized-state trust (no validation on `Game: Deserialize`): F22
  (players outside 2..=3 panics), F2.
- Go-port parity vs official rules: F30 (red7 empty-winning-set, undocumented
  deviation); F25 (discard piles top-card-only vs inspectable); F4, F5, F6,
  F7, F8 preserved Go quirks (some undocumented in PORTING_NOTES).
- Boilerplate duplication: F1, F13 (placings-log tail triplicated), F15
  (clan_conquered logic duplicated lib/render).
- Dependency hygiene: F45 (binary-only deps as lib deps, systemic across ~27
  game crates; fix must not use dev-dependencies), F46 (port 80 default).
- Docs/code divergence: F20/F38 (hand documented sorted, is not), F31
  (DATA_DOCS tie-break fictional), F32 (RULES.md gaps), F27 (k8s blurb says
  2-player for a 2-3 player game).
- Error-swallowing / lost logs: F19, F37 (final draw of every round unlogged).
- Dead/write-only state: F21, F39, F40 (Stats.investments/expeditions).
- Missing finished-game gating: F5 (love-letter accepts commands post-finish),
  F14 (age-of-war duplicate placings logs, amplification new vs Go).

### Discussion candidates

- F30 (red7 empty-winning-set): rules-semantics call — adopt official
  "cannot win" behavior (needs defined all-empty outcome) or document the
  deviation as deliberate.
- F25 (lost-cities discard piles top-card-only): product call — expose full
  piles (card-counting is part of the physical game) or document the
  simplification.
- F45 (boilerplate dep placement): remediation design — optional deps +
  required-features vs a separate bin crate, workspace-wide.
- F5/F14 (post-finish command acceptance, preserved Go quirk): decide whether
  crates should enforce finished-game rejection or keep parity.
- F24 (lost-cities-2 dropped winner announcement): accept regression or
  restore.

## Unit 8: games-batch-f

Crates: zombie-dice-2, battleship-2, for-sale-2, category-5-2, greed-2,
farkle-2, tic-tac-toe-2, no-thanks-2, liars-dice-2 (~8.0k LOC).

### Tallies

- Verification-corrected: 0 critical / 2 major / 22 minor / 34 nit
  (58 findings) — identical to the original header tally; no severity changes.
- Rejected: none. All 58 survive.

### Verification

- Verdicts: 54 CONFIRMED, 4 ADJUSTED (F18, F28, F48, F56), 0 REJECTED,
  0 UNVERIFIABLE.
- RECOMMENDATION INVALID: F18 (for-sale phase inference) — "the recommended
  `#[serde(default)] phase: Phase` migration is unsound: Phase's #[default] is
  Buying, so live games mid-Selling would deserialize as Buying and break
  whose-turn/can_play. Sound fix: Option<Phase> with current_phase() fallback
  or post-deserialize fixup."
- F28 (category-5 points() lower-is-better) verification question discharged:
  ELO is placings-driven (web/src/db.rs:1536-1548 uses `place`, never
  points()), so lowest-wins games rate correctly today; negating points()
  would be a display regression, not a fix.
- F48 (tic-tac-toe casing) premise partly wrong: the "is X / is O" label is
  uppercase; real split is log+label uppercase vs board+RULES.md lowercase;
  any fix must touch the exact-render test.
- F56 (liars-dice "fourty") reachability corrected: NOT practically
  unreachable — the uncapped bid parser (F57) makes quantities >= 40 ordinary
  input.
- F50 (no-thanks 3-5 cap) softened: the 2004 original edition is officially
  3-5; "3-7" is later editions.
- Minor detail fixes: F34's lib.rs:522 citation wrong (points() cannot panic);
  F39 is also Go parity; F53 undercounts unchecked sites; recommendation
  caveats on F6, F11, F38.

### Headlines

Critical: none.

Majors (both hidden-information leaks in pub_state):
- F1 — zombie-dice-2 cup draw order leaked to all players — pub_state exposes
  the shuffled cup in draw order; take_dice drains from the front, so the vec
  head IS the next draw; bots/API clients get perfect foreknowledge. Leak is
  NEW vs Go (Go PubState returned nil); DATA_DOCS.md falsely claims no hidden
  info. game/zombie-dice-2/src/lib.rs:194,443.
- F13 — for-sale-2 selling-phase secret plays leaked via PubState.bids —
  simultaneous secret selection exposed to everyone in the JSON contract
  (HTML renderer hides it, so invisible in UI). Go has the identical leak.
  game/for-sale-2/src/lib.rs:258,411-412.

Most notable minors:
- F45 — tic-tac-toe-2 `1 - start_player` usize underflow on crafted state;
  release wraps silently (no overflow-checks anywhere in the workspace).
  game/tic-tac-toe-2/src/render.rs:34.
- F46 — crafted `players` count drives unbounded allocation/iteration; the
  requester deserializes Game verbatim (lib/cmd/src/requester/gamer.rs:28-45).
  Systemic; the fix belongs in the requester/validation layer.
- F25 — category-5-2 draw_cards unbounded recursion -> stack overflow if n
  exceeds deck+discard (preserved Go hazard, pub fn no guard).
  game/category-5-2/src/lib.rs:270-280.
- F24 — category-5-2 player cap 8 contradicts Go/official 2-10 AND its own
  RULES.md. game/category-5-2/src/lib.rs:21.
- F49 — no-thanks-2 vacuous test asserts nothing (loops 0..0).
  game/no-thanks-2/src/lib.rs:392-399.

### Unit state

Nine smaller ports in good shape: no criticals, no panic reachable from
crafted command input in any crate, and liars-dice-2 even fixes two real Go
bugs. The dominant problem classes are pub_state hidden-information leaks
(the two majors) and the systemic all-pub Deserialize state-trust shape
(crafted stored state can panic index/drain paths in nearly every crate).
Many findings are preserved-Go-quirk cross-references feeding the
project-wide port-parity decision list.

### Theme evidence

- Privacy/hidden-info leaks in pub_state: F1 (new vs Go), F13 (inherited from
  Go); contrast no-thanks-2 and liars-dice-2 verified leak-free.
- Deserialized-state trust / crafted stored state panics: F4, F9, F19, F26,
  F29, F34, F44, F45, F46, F53, F55 — flagged systemic, fix belongs in
  requester/serde layer (F46 names lib/cmd requester explicitly).
- Go-port parity vs official rules (cross-reference decision list): F2
  (zombie shotgun refill), F14 (for-sale floor vs round-up), F15 (deck/chip
  setup), F24 (player cap 8 vs 10), F43 (simplified Farkle), F50 (no-thanks
  3-5, edition-dependent), F54 (liars-dice turn after challenge).
- Bot-slot/player-count validation: F24 (cap vs docs), F46 (unbounded
  players), F50.
- Boilerplate duplication: F7, F35 (duplicated finish/placings blocks — same
  shape as unit 7's F1/F13), F38 (scoring table duplicated lib/render), F52
  (run-grouping duplicated), F22 (highest_bid duplicated with different
  sentinel).
- Docs/code divergence: F16 (RULES.md cheque deck factually wrong), F24, F37
  (E1 color), F48 (casing).
- Recursion hazards: F5 (zombie bust chain, theoretical), F25 (category-5,
  guardable).
- Test-quality gaps: F41, F49 (vacuous), F58 (liars-dice redaction/wild-1/
  full-game untested).
- Binary-only-deps systemic issue present in all nine crates (tracked in
  findings/dependencies.md, not re-flagged).

### Discussion candidates

- F1 (zombie cup order): redaction design — counts-only vs canonicalized
  order in PubState; also DATA_DOCS correction.
- F13 (for-sale bids): redaction semantics — zero the shared pub_state during
  Selling and re-add own play per player_state, vs a separate private field;
  Go parity vs privacy call.
- Port-parity vs official rules batch decision: F2, F14, F15, F24, F43, F50,
  F54 all need one project-level ruling (align with official or document the
  ported variant).
- F18 (for-sale phase field): migration design — Option<Phase> fallback vs
  post-deserialize fixup (the obvious serde-default approach is unsound).
- F28 residue: whether points() for lowest-wins games should carry
  display/bot-prompt semantics (ratings already safe).

## Unit 9: web-server

Scope: web crate server infra — main/router/state/config, db.rs, nats.rs,
error.rs, crypto.rs, admin.rs, auth/, websocket(+client), bin/import_game,
manifest Cargo.toml (~11.3k effective LOC).

### Tallies

- Verification-corrected: 0 critical / 8 major / 36 minor / 22 nit
  (66 surviving of 67).
- Original (Lead recount; no header tally in findings file): 1 critical /
  7 major / 37 minor / 22 nit. Net changes: F1 critical -> major, F18
  major -> minor, F27 minor -> major, F58 minor -> nit, F30 rejected.
- REJECTED and excluded from tally: F30 "update_bot_provider omits
  updated_at = now()" — bot_providers has NO updated_at column at all
  (migration 013:23-34, no later ALTER); nothing is omitted and the
  recommended fix would be a runtime SQL error.

### Verification

- Verdicts: 57 CONFIRMED, 8 ADJUSTED (F1, F18, F24, F27, F35, F36, F51, F58),
  1 REJECTED (F30), 1 UNVERIFIABLE (F67 crates.io version currency —
  external basis, in-repo facts match).
- Severity changes:
  - F1 critical -> major: confirm-cap race real (no lock/tx on validate
    path) but failed-guess increments do commit, so concurrency is a bounded
    multiplier over the cap of 10, not unbounded brute force vs 1e6 keyspace.
  - F18 major -> minor: reorder_bots non-atomicity is admin-only display
    ordering, self-repairable.
  - F27 minor -> major: bot rename/delete UNCERTAIN resolved —
    bot/src/main.rs:171-187 silently skips and ACKS the turn when the bot
    name is missing/disabled, so in-flight games deadlock permanently.
  - F58 minor -> nit: consumer acks well under the 5-min ack_wait (10s HTTP
    timeout, bounded retries); duplicate-delivery risk theoretical.
- Other notable adjustments:
  - F35 (test-coverage gap, major stands): two sub-claims wrong —
    choose_colors and the ELO helpers ARE tested; drop from the finding.
  - F36 (updated_at sweep, minor stands) carries a LIVE HAZARD: 2 of 18
    listed lines target game_proposals (migration 015, NO trigger) — a sweep
    following the line list verbatim would silently stop maintaining
    game_proposals.updated_at.
  - F24 (admin Action race): per vendored reactive_graph 0.2.14 the claimed
    cross-attribution cannot occur (input() clears on completion); adjacent
    real defect found — the `dispatched` guard counter is never incremented.
    Recommendation (key off value() alone) valid either way.
  - F51 part 1 wrong (fixture does cover self-exclusion); parts 2-3 stand.
  - F34 strengthened: undo server path (game/server_fns.rs:731-784) has no
    is_finished guard, so reachability is stronger than the finding hedged.
  - F56 UNCERTAIN resolved: consumer acks all poison classes; stranding
    limited to 3x-transient failures; stays minor.
- RECOMMENDATIONS FLAGGED INVALID/FLAWED (unusually many for one unit):
  - F30 rejected outright (fix would be a runtime SQL error).
  - F6: "reset sent_count on code rotation" would UNDER-count the global 24h
    cap (hourly rotation = 120 real sends/day vs the 50 cap); only the
    windowed-counter alternative is sound.
  - F1: increment-before-compare must use `>` not `>=` or a correct 10th
    attempt after 9 failures is rejected.
  - F4: uniform-rejection alternative would lock out existing verified users
    on blocked domains.
  - F34: "recompute on next finish" alone double-counts unless
    game_type_users is also rewound.
  - F48: suggested silent-Ok breaks an existing test assertion (db.rs:3317).
  - F52: setting SECURE_COOKIE in k8s base/ forces it on the dev overlay;
    prefer default-secure-in-code with explicit opt-out or prod-only patch.
  - F66: making futures-util optional breaks non-ssr `cargo test` (two test
    files have ungated top-level uses); needs dev-dependency or [[test]]
    required-features.
  - F60/F14: reactive/context scope details (ready_state must be read
    get_untracked in non-reactive listeners; expect_context must be called in
    server-fn scope).

### Headlines

Critical: none after verification (F1 downgraded).

Majors (all 8):
- F1 — Concurrent confirm requests bypass the per-code attempt cap —
  check-then-act across unlocked statements on the 6-digit login code;
  bounded-multiplier brute-force window; successful guess = full account
  login. web/src/auth/server.rs:354-390.
- F2 — Email-squatting DoS — any logged-in user can attach any address
  unverified; the real owner's signup is then rejected despite presenting a
  valid code; no cap, no expiry. web/src/auth/server.rs:789-831,428-436.
- F16 — Hardcoded public fallback encryption key silently used when
  DATABASE_ENCRYPTION_KEY unset — one warn line, keeps serving; all stored
  LLM API keys effectively plaintext-equivalent. web/src/crypto.rs:42-57.
- F27 — Bot rename/delete/disable permanently deadlocks in-flight games —
  worker silently skips and acks turns for unresolvable bot names (games
  reference bots by NAME). web/src/admin.rs:159-207 + bot/src/main.rs:171-187.
- F34 — undo_game does not clear rating state — re-finished game keeps the
  voided result's ratings and the new outcome is never rated (idempotency
  guard sees stale rating_change); undo path has no is_finished guard.
  web/src/db.rs:1407-1463,1536-1680.
- F35 — ~20 public DB functions have no test (CODING.md requires them);
  concede_game 3+ player constraint is debug_assert-only. web/src/db.rs:2961.
- F52 — Session cookies lack the Secure flag in production — SECURE_COOKIE
  never set in any k8s manifest; browsers will send the session token over
  plaintext HTTP. web/src/auth/session.rs:32-36.
- F53 — Bot command consumer unsupervised — single spawn, JoinHandle dropped;
  exit/panic = silent permanent bot outage while /healthz stays green.
  web/src/main.rs:55-74.

### Unit state

The security- and operations-critical unit: auth flow, crypto, admin surface,
6.4k-line db.rs, NATS eventing, websockets. Code quality is generally high
(parameterized SQL throughout, no request-path panics, fail-closed admin
gating, strong auth state-machine tests), but the majors cluster in
concurrency/atomicity gaps (F1, F18, F19, F38, F39), fail-open/silent-failure
posture (F16, F8, F7, F53, F56), and lifecycle edges nobody wired end-to-end
(F27, F34, F52). Verification found the review highly accurate (66/67) but
weak on severity calibration at the extremes and unusually rich in flawed
recommended fixes.

### Theme evidence

- Auth/crypto weaknesses: F1 (attempt-cap race), F2 (email squatting), F3
  (no session rotation), F4 (enumeration via blocked-domain carve-out), F9
  (no email normalization), F11 (tokens never expire), F12 (modulo bias/
  plaintext codes), F16 (fallback key), F52 (Secure flag).
- TOCTOU/concurrency guards missing: F1, F18/F19 (display_order), F24
  (Action race, mechanism disputed), F38 (zero-change defeats idempotency
  guard), F39 (opposite-direction friend-request 23505), F46 (username race,
  mitigated).
- Error-swallowing / fail-open / silent failure: F5 (transient DB error mass
  logout), F7 (Resend failure silent + quota burn), F8 (Turnstile fails
  open), F10 (send-cap refusal discarded), F15 (logout swallows error), F16,
  F27 (silent skip+ack), F29 (rows_affected ignored), F53, F56 (no DLQ/
  advisory handling).
- Unsupervised/unreconciled infrastructure: F53, F55 (shutdown misses WS/
  background tasks), F57 (NATS config drift never reconciled), F54 (no rustls
  CryptoProvider installed, dual providers in lockfile).
- Privacy/visibility gates not wired: F59 (/ws unauthenticated site-wide
  firehose — metadata leak + O(NxM) fan-out).
- Bot-slot lifecycle: F27, F22 (test_provider hardcodes gpt-4o-mini), F21
  (cannot clear API key), F26 (one undecryptable key kills admin page).
- Boilerplate duplication: F28 (admin gate x15 verbatim), F32 (Action-value
  unwrap idiom x10), F42 (db.rs grab-bag).
- Dependency hygiene / missing feature declarations: F64 (gloo-net unused),
  F65 (tokio net/time undeclared, works via unification), F66 (futures-util
  non-optional), F67 (currency spot-check, unverifiable).
- Docs/schema divergence: F36 (trigger-vs-manual updated_at inconsistency,
  with the game_proposals sweep hazard), F30's rejection itself (review
  assumed a column that does not exist).

### Discussion candidates

- F34 (undo vs ratings): product/engine ruling — can finished games be
  undone, and if so should ratings rewind or recompute; recompute-only
  double-counts.
- F27 (bots referenced by name): schema/design decision — reference by id
  (new migration) vs warn-on-rename/delete; also what the worker should do
  with unresolvable bots (currently silent ack).
- F1 + edge throttling: whether Cloudflare edge rules are strict enough to
  rely on, and the atomic-validation redesign (increment-then-compare with
  correct off-by-one).
- F8/F16 fail-open posture: explicit policy decision on failing open vs
  closed for Turnstile and the encryption key (startup refusal vs dev
  opt-in).
- F59 (/ws firehose): accept-and-document at current scale vs per-connection
  subscription filtering and/or session requirement.
- F11 (token expiry): confirm the accepted-design status; "log out
  everywhere" path is a product decision.
- F6 (send-cap accounting): windowed counter design (the only sound option).
- F44 (is_turn_at semantics): "turn started" vs "last activity" — drives the
  22-day digest ordering.
- F2 fix semantics: honoring a valid code as ownership proof (delete vs
  reassign the squatter's row) touches account-linking semantics.
