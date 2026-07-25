# specs-LOG - Lead session, spec-writing unit 1

Date: 2026-07-25. Lead: orchestrate Lead role. Workers: model fable (user
override).

## Plan

Write implementation specs for the first READY work packages in
BACKLOG.md order. Strict backlog order of READY packages:
WP-44, WP-01, WP-14, WP-25, WP-36, WP-68, WP-39 (WP-68 sits at backlog
item 10, ahead of WP-39 at 11; the briefing expected WP-39 sixth, so
this unit covers all seven to satisfy both readings).

Output: planning/specs/WP-NN-<slug>.md, one per package, per
superpowers:writing-plans conventions (header, tasks, bite-size steps,
test plans, no placeholders). No code changes anywhere.

Worker sequence (serial):
1. W0: REVIEW.md tally correction (grand 10c/78M/257m/225n = 570;
   web-domain 80 = 1/12/37/30, web-frontend-email 63 = 2/13/30/18,
   bot-operator-tools 31, dependencies 27 - confirm last two splits from
   planning/raw/ w5/w6 notes) + footnote. Also: recon of repo test/build
   conventions (docs/CODING.md etc.) written to
   planning/specs/notes-conventions.md for later workers.
2. W1: WP-44 spec (proposals integrity + email_token leak)
3. W2: WP-01 spec (char/byte panic elimination)
4. W3: WP-14 spec (alhambra-1 core fixes)
5. W4: WP-25 spec (modern-art-2 liveness)
6. W5: WP-36 spec (crypto/deploy hardening)
7. W6: WP-68 spec (term_size replacement)
8. W7: WP-39 spec (bot consumer supervision)

Each spec worker must: read the package entry in work-packages.md, the
findings bodies (using planning/raw/ ID mappings for units 10-13), any
verification/ annotations, then read the LIVE repo source
(/home/beefsack/Development/brdgme/rust/...) and re-derive/validate every
fix from the actual code (findings' recommendations proven unreliable).
Note snapshot-vs-live drift in the spec. Lead reviews each draft against
the quality bar; revision pass if gaps.

## Entries

- [planned] Plan written (unit 1). No workers dispatched yet.
- [W0 done] REVIEW.md tallies corrected (grand 10/78/257/225=570; rows
  10-13 = 80/63/31/27 with confirmed splits 0/4/16/11 and 0/4/18/5 from
  w6 notes; footnote added; section-4 headers updated; only one 563
  existed). Lead spot-checked table + footnote - correct.
  notes-conventions.md written (116 lines): cargo -p per crate, web needs
  --features ssr, scripts/rust-test.sh CI suite, AGENTS.md exists (no
  CLAUDE.md), k8s at repo root.
- [dispatch W1] WP-44 spec (proposals integrity + email_token leak).
- [W1 done] specs/WP-44-proposals-integrity-email-token-leak.md written.
  10 tasks, all 11 findings dispatched. No snapshot drift (file
  byte-identical to f8763a5). Overturned rec: wd F30 - "not declined"
  fallback unsound once F29 lands; target must be accepted. Lead
  spot-checked struct/SELECT/consumer claims against live source - all
  confirmed. ACCEPTED without revision.
- [dispatch W2] WP-01 spec (char/byte panic elimination).
- [W2 done] specs/WP-01-char-byte-panic-elimination.md written. 7 tasks
  (6 code + docs convention). No snapshot drift. Overturned/adjusted
  recs: lg F2 (chars().zip() alternative rejected - UniCase folding),
  lg F3 (byte-len-of-lowered-string insufficient - to_lowercase changes
  lengths; re-derived per-char compare returning (input_bytes,
  value_bytes)). Key insight: lib/game is byte-unit end-to-end,
  lib/markup is char-unit - fixes respect each. Lead spot-checked
  Space/Token/shared_prefix/Enum/slice/CardParser live source incl. the
  v_str lowercasing in Enum - all claims confirmed. ACCEPTED.
- [dispatch W3] WP-14 spec (alhambra-1 core fixes).
- [W3 done] specs/WP-14-alhambra-core-fixes.md written. 10 tasks. No
  snapshot drift. F18 impact partially overturned: truncation/
  nondeterminism only reachable on grids violating grid_is_valid
  invariants (unreachable in play) - live scores were NOT wrong; fix
  lands for robustness (break-into-success-branch + sorted iteration,
  serialization-neutral; BTreeMap rejected). F16/F17 recs confirmed.
  Lead spot-checked take()/place()/wall-walk live source - claims and
  fix semantics confirmed. ACCEPTED.
- [dispatch W4] WP-25 spec (modern-art-2 liveness).
- [W4 done] specs/WP-25-modern-art-liveness.md written. 5 tasks. No
  snapshot drift. F34+F35 fixed via one shared advance_past_empty_hands
  invariant; busy-loop repro re-derived (forced play in round 4,
  infinite while + log growth, not recursion); timeout-bounded (2s)
  regression test so CI fails fast. Overturned: F42's pub_state-None
  option rejected (sentinel is documented API; render-side bid>0 gate
  instead); F34's original rec correct-but-insufficient (missed F35
  path). Epilogue hoist in command() required by new finish paths.
  Lead spot-checked round_cards/end_round/settle loop/test fixtures -
  confirmed. ACCEPTED.
- [dispatch W5] WP-36 spec (crypto and deploy hardening).
- [W5 done] specs/WP-36-crypto-deploy-hardening.md written. 6 tasks.
  Drift: 4 primary files identical to snapshot; admin.rs drifted (#47)
  but crypto call sites re-verified live. Triage's F52 design note
  validated (dev overlay unpatched + non-localhost plain HTTP =>
  default-secure-in-code + SECURE_COOKIE=false dev opt-out in new
  k8s/dev/web-patch.yaml, Tiltfile, .env.template; prod needs no
  manifest change). Overturned: F17 AAD declined (bot/src/crypto.rs
  shares ciphertext format; create-flow encrypts pre-id; prod
  ciphertexts would break) - zeroize half implemented. F54
  unverifiable resolved via cargo tree (dual backend real). F55 WS
  half fixed via CancellationToken/TaskTracker; background half
  deferred to WP-38 scope. No prod kubectl in spec. Lead spot-checked
  session.rs/main.rs/websocket.rs/kustomization/Tiltfile/crypto.rs -
  confirmed. ACCEPTED.
- [dispatch W6] WP-68 spec (term_size replacement).
- [W6 done] specs/WP-68-term-size-replacement.md written. 1 task. No
  snapshot drift (one stale line citation in dp F13 corrected:
  deny.toml entry is at lines 31-34, not 19-27). terminal_size 0.4
  chosen (no existing lock entry; rustix 1.x already present). Both
  findings same defect; non-tty zero-width behavior preserved via
  map_or(0,..). Lead spot-checked repl.rs:186 / Cargo.toml:16 /
  deny.toml:34 - confirmed. ACCEPTED.
- [dispatch W7] WP-39 spec (bot consumer supervision mechanical).
- [W7 done] specs/WP-39-bot-consumer-supervision.md written. 8 tasks.
  Drift: game/mod.rs +1 line at :7 (#47 pub mod placing) - spec cites
  LIVE lines; main.rs/nats.rs/bot main.rs identical to snapshot.
  Overturned: bo F8 healthz DB-check DECLINED (livenessProbe only, no
  readinessProbe/Service; DB probe would crashloop); bo F5 TaskTracker
  suggestion replaced with single Semaphore (bounds concurrency AND
  drains on SIGTERM, no new dep); ws F53 crash-the-pod alternative
  rejected (in-process supervisor + backoff + restart metric).
  Coordination with WP-36 main.rs edits recorded (disjoint regions,
  supervisor NOT wired into WS shutdown token). WP-38/D-5 boundary
  scrupulously fenced (only test-code uses Nak). Lead spot-checked
  main.rs spawn block, Conflict arm fan-out, Ok(()) stream-end, bot
  unreachable!()/continue paths, drift - all confirmed. ACCEPTED.

## Completion

Unit COMPLETE 2026-07-25. All 7 specs written and Lead-reviewed with
live-source spot-checks; zero revision passes needed (every draft met
the quality bar first pass):

1. specs/WP-44-proposals-integrity-email-token-leak.md
2. specs/WP-01-char-byte-panic-elimination.md
3. specs/WP-14-alhambra-core-fixes.md
4. specs/WP-25-modern-art-liveness.md
5. specs/WP-36-crypto-deploy-hardening.md
6. specs/WP-68-term-size-replacement.md
7. specs/WP-39-bot-consumer-supervision.md

(7 not 6: strict backlog order puts READY WP-68 at item 10 ahead of
WP-39 at item 11; the briefing expected WP-39 - both covered.)

Also done: REVIEW.md tally correction (570 grand total + footnote) and
specs/notes-conventions.md (shared recon for future spec workers).

REMAINING for the next spec-writing Lead: 32 of 39 READY packages, in
backlog order: WP-06, WP-07 (Phase 2); WP-13, WP-15, WP-22, WP-23,
WP-28, WP-19, WP-21, WP-03, WP-41, WP-37, WP-59 (Phase 3); WP-29
(Phase 4, sequence after WP-30); WP-54, WP-51, WP-60, WP-52, WP-53,
WP-61, WP-62, WP-63, WP-08, WP-27, WP-24, WP-18, WP-31, WP-32, WP-33,
WP-43 (Phase 5); WP-65 (Phase 6, after WP-64).

Notable overturned/adjusted original recommendations (all documented in
the specs' disposition tables): wd F30, lg F2, lg F3, b F18 (impact),
d F42 (half), ws F17 AAD, bo F8, bo F5 (mechanism), ws F53 (mechanism),
d F34 (insufficient alone). Cross-spec coordination points: WP-36 Task 5
and WP-39 Tasks 1-2 both touch web main.rs (disjoint regions, noted in
both); WP-39 notes WP-36's stale "supervision is WP-38" label.

# specs-LOG - Lead session, spec-writing unit 2

Date: 2026-07-25. Lead: orchestrate Lead role. Workers: model fable
(user override). Budget: hard 150k tokens; 5 packages max.

## Plan

Next 5 READY packages in backlog order: WP-06, WP-07, WP-13, WP-15,
WP-22. Serial workers, one spec each, same conventions as unit 1
(read work-packages.md entry + findings bodies + verification
annotations, re-derive every fix from LIVE source, note drift,
disposition table for original recs, superpowers:writing-plans
structure, test plan per docs/CODING.md + specs/notes-conventions.md).
Lead reviews each draft with live-source spot-checks.

Findings sources: WP-06/WP-07 = findings/lib-support.md (ls F*) +
dependencies.md (dp F10); WP-13 = games-batch-a.md (a F11-F20);
WP-15 = games-batch-b.md (b F1-F15 subset); WP-22 = games-batch-d.md
(d F1-F11 subset). Verification annotations live under
findings/verification/.

## Entries

- [planned] Plan written. No workers dispatched yet.
- [dispatch W1] WP-06 spec (lib cmd tools and http).
- [W1 done] specs/WP-06-lib-cmd-tools-http.md written. 5 tasks. No
  snapshot drift (14 files identical to f8763a5). Dispositions: 9
  confirmed, 3 adjusted, 0 overturned. Adjusted: ls F20 (Status
  re-request chosen; stack-stored renders cannot fix :load), ls F21
  (delete cli/Response, KEEP Request in brdgme_cmd - platform-side
  wire type, avoids churning rand_bot lib.rs that WP-07 edits),
  ls F30 (push-on-play already exists at repl.rs:152; only stack
  seeding changes). Refinements: RequestError::Parse Display gains
  {source}; cli.rs stdout-write panics stay as expect (process-
  boundary exception). Adds #[cfg(test)] test_game.rs (TestGame +
  BrokenRenderGame) since game crates can't be dev-deps (cycle).
  Lead spot-checked http.rs:17/54, cli.rs unwraps, gamer.rs
  renders/handle_*_render unwraps (67/74/164/177), repl.rs prompt/
  undo seed/panic strings, bot_cli.rs, local.rs (no status check),
  error.rs (Stdin msg matches test), api.rs serde(default) +
  GameResponseError #[from] serde_json, Gamer trait vs TestGame impl
  (start/command/status/command_spec sigs, rules() etc. defaulted,
  GameError::PlayerCount fields, invalid_input) - all confirmed.
  ACCEPTED without revision.
- [dispatch W2] WP-07 spec (game_client and rand_bot).
- [W2 done] specs/WP-07-game-client-rand-bot.md written. 4 tasks. No
  snapshot drift (5 files identical to f8763a5; WP-06 will create
  disjoint rand_bot drift if it lands first - noted). Dispositions:
  7 confirmed, 3 adjusted, 1 confirmed-as-accept (ls F37). Key
  mechanism overturn inside ls F31 (adjusted): RequestBuilder::timeout
  REPLACES the client-level timeout (would silently override web 10s /
  bot 60s KEDA cold-start allowance); fix is a tokio::time::timeout
  ceiling (90s, min-semantics) on RetryConfig, body read bounded too,
  ceiling timeouts retryable. ls F41 adjusted ("" join here; share-
  with-fuzz half deferred to WP-63). ls F43 adjusted (Player empty ->
  unwrap_or_default, not placeholder string; cli unwraps -> expect,
  no in-band error channel). Full caller audit: 9 call sites, exactly
  one breaking (web/src/game/mod.rs execute_command `?` into
  ExecuteCommandError, needs one-line map_err(anyhow::Error::from)).
  Unlisted Int min>max panic left untouched (scope discipline,
  flagged). Lead spot-checked send_with_retry predicate/no-timeout,
  request_with_config anyhow, fetch_game_data sequential + Advanced-
  Strategy move, operator Client::new(), web/bot client timeouts,
  ExecuteCommandError From<anyhow> only, rand_bot OneOf/Player/join/
  cli unwraps + chrono dep, BotCommand From<String> impl - all
  confirmed. ACCEPTED without revision.
- [dispatch W3] WP-13 spec (starship-catan-1).
- [W3 done] specs/WP-13-starship-catan-fixes.md written. 9 tasks, all
  10 findings dispatched (a F17 + a F20 share Task 9). No snapshot
  drift. Dispositions: 8 confirmed, 1 refined (a F18: message
  unreachable via command() - can_buy/can_sell gate the verb; test
  calls can_trade directly), 1 SKIPPED (a F20: BTreeMap<usize,bool>
  serializes as JSON object, BTreeSet as array - would break live
  mid-flight saved states; comment lands instead). Adjustments:
  a F14 parser cap Int::bounded(1,99) over checked arithmetic (one
  choke point vs four sites; suggest already caps at min+4); a F15
  peek render gated on viewer==current_player (unguarded render would
  hand peek to opponent - player_state() clones peeking into BOTH
  states; JSON exposure flagged to WP-10/D-33). a F17 start_card
  field removal justified serde-compatible (no deny_unknown_fields,
  never read) with one-off compat spot check + revert instruction.
  Lead spot-checked cannon_transaction booster bug, can_lose_module
  ||, TradeAndBuild buy branch missing astro check vs Flight branch,
  trade() unconditional debit, direction error token, end_flight
  insert(0,..), render.rs viewer-in-turn-row + unused _peeking,
  Int::positive parsers, Int::bounded + suggest min+4 in lib-game,
  SectorCard variant fields (Pirate/Trade/Colony start_card), Game
  field visibility for tests - all confirmed. ACCEPTED without
  revision.
- [dispatch W4] WP-15 spec (seven-wonders-1 mechanical fixes).

# specs-LOG - Lead session, spec-writing unit 2 RESUME (unit 2b)

Date: 2026-07-25. Prior unit-2 Lead died (credit outage) after
dispatching W4. Assessment: WP-06/WP-07/WP-13 specs written AND
Lead-reviewed/accepted (entries above). W4's output
specs/WP-15-seven-wonders-mechanical.md exists (written 03:17, after
last LOG write 03:01) but was NEVER Lead-reviewed. WP-22 not started.
Resume plan: this Lead reviews WP-15 critically with live-source
spot-checks, then dispatches one worker for WP-22, reviews, closes the
unit (5 packages: WP-06, WP-07, WP-13, WP-15, WP-22).

## Entries (unit 2b)

- [resume] Assessment logged. Beginning WP-15 review.
- [W4 done / reviewed by unit-2b Lead] specs/WP-15-seven-wonders-mechanical.md
  ACCEPTED after review. 9 tasks, all 9 findings dispatched. No snapshot
  drift claimed and structure matches accepted specs. Lead spot-checked
  live source: end_hand auto-discard DISCARD_COINS at lib.rs:195;
  player_vp catch-all dropping DrawDiscard (701-725, vp self-shadow fix
  correct); card.rs Halicarnassus B vp 2/1/0 + A stage-2 vp 0;
  post_build_hook queue guard 410-412; take_from_discard ownership
  rejection 921-923 / remove(0) 932; check_hand_complete 255-257;
  resolve_deal unwrap_or_default 418-431; Action enum 28-40; choose_build
  808-851 / choose_deal 892-898; execute_build no re-validation at
  execute time (sabotage test sound, call sites 320/347); military log
  raw index 770-777; start_hand 177-180 + execute_actions reset 295;
  command.rs gating 17-19/21-37/39/54-58; player_state 984;
  command_spec 1177 (no turn assert); PORTING_NOTES.md:63-64 claim;
  lib/cost early return 181-184; can_afford_cost empty-deal 467 and
  coin filter 507; test helpers/imports (pub use card::*, Status,
  brdgme_markup dep, to_string "{{player N}}", Log.content, whose_turn
  provided method); Giza A Stone / Ore Vein 1 + Foundry 2 Ore; Tree Farm
  Wood; Haven cost. All confirmed. Overturned/adjusted per spec: b F2
  queue-time rec replaced by prune_resolvers() at fire time; b F9
  "unreachable" analysis overturned by verification, serde-compat
  deal_coins shape; b F14 gap list corrected (MimicGuild already
  tested). Lead ADDED during review: flag that military_conflicts
  battles RIGHT neighbor only (official rules: both) - unfindinged
  deviation, noted in Task 8 test comment + cross-package section,
  routed to WP-16. ACCEPTED with that addition.
- [dispatch W5] WP-22 spec (lords-of-vegas-1).
- [W5 done] specs/WP-22-lords-of-vegas-fixes.md written. 9 tasks, all 10
  in-scope findings dispatched (d F5 -> WP-09/D-36, d F12 -> WP-26, both
  fenced). No snapshot drift. Dispositions: 6 confirmed, 4 adjusted:
  d F1 (rec correct, but arms untestable via command() - extracted
  private dispatch() helper; `..` patterns drop the allow attr and the
  Gamble player shadow); d F2 (transient BTreeSet pop_first BFS +
  sorted TILES-key Vec; BTreeMap re-typing of serialized Board
  REJECTED - shape caution, transient locals suffice; re-derived that
  the BFS HashSet is per-call so even same-process calls diverge ->
  reliable 100-iter red test); d F4 major per verification (casino-tile
  supply enforced in build(); dice/token build-side checks rejected as
  dead code - max 2 lots/player, no draw mechanism; saturating_sub on
  all three render sites mandatory for legacy states); d F6 (std
  LazyLock over OnceLock-getter/once_cell, dep removed); d F10 (pub
  BLOCK_WIDTH import over duplicate constant). Zero serialized-shape
  changes anywhere. Lead spot-checked live source: command()
  unimplemented arms 182-186 + allow attr 172 + player shadow;
  InvalidInput struct style; casino_at HashSet queue 242-250; casinos()
  TILES.keys() 278; resolve_boss_ties silent reroll 330-332 + recursion
  337; reroll_at Some(die); boss_tiles owner-Some; render.rs 80/85/117
  subtractions + 154-155 literal 3 + line-13 import; CASINO_TILES=9 /
  PLAYER_DICE=12 / PLAYER_OWNER_TOKENS=10; build() guard insertion
  point 281-286; Game/PubState/Player field literals vs tests;
  Command variant field names; Casino Vega/Display/color (Sphinx
  Orange, Pioneer Brown); tile.rs lazy_static 23-25 + private TileMap
  alias (lint-safe in pub LazyLock); Cargo.toml lazy_static:15,
  serde_json:18, dev-deps; card.rs shuffled_deck min insert pos 38;
  lib.rs:118 GameEnd unreachable; GameRng::seed_from_u64 INHERENT (no
  trait import needed in board.rs tests); markup transform() falls back
  to "Player N" on empty slice (Task 4 test safe); plain/transform sigs;
  existing test_board_casino_at_works order [A1,A2,A5] == BTreeSet BFS
  pop order (stays green); RULES.md 48-54/85-89/98-105. All confirmed.
  ACCEPTED without revision.

## Completion (unit 2 / 2b)

Unit COMPLETE 2026-07-25. Five packages done for unit 2 overall:
1. specs/WP-06-lib-cmd-tools-http.md (unit 2, reviewed+accepted)
2. specs/WP-07-game-client-rand-bot.md (unit 2, reviewed+accepted)
3. specs/WP-13-starship-catan-fixes.md (unit 2, reviewed+accepted)
4. specs/WP-15-seven-wonders-mechanical.md (written by unit-2 W4;
   reviewed+accepted by unit-2b Lead, with one addition: flagged the
   unfindinged military_conflicts right-neighbor-only deviation to
   WP-16)
5. specs/WP-22-lords-of-vegas-fixes.md (unit 2b, reviewed+accepted)

REMAINING for the next spec-writing Lead: 27 of 39 READY packages, in
backlog order: WP-23, WP-28, WP-19, WP-21, WP-03, WP-41, WP-37, WP-59
(Phase 3); WP-29 (Phase 4, sequence after WP-30); WP-54, WP-51, WP-60,
WP-52, WP-53, WP-61, WP-62, WP-63, WP-08, WP-27, WP-24, WP-18, WP-31,
WP-32, WP-33, WP-43 (Phase 5); WP-65 (Phase 6, after WP-64).

Cross-spec coordination added this unit: WP-15 flags the unfindinged
seven-wonders military right-neighbor-only deviation to WP-16; WP-22
Task 5/6 pre-empt WP-65's lazy_static/serde_json sweep for this crate;
WP-22 leaves d F5 to WP-09 (D-36) and d F12 to WP-26.

# specs-LOG - Lead session, spec-writing unit 3

Date: 2026-07-25. Lead: orchestrate Lead role. Workers: model opus
(user override for this session - NOT fable). Budget: hard 150k tokens;
5 packages.

## Plan

Next 5 READY packages in backlog order (per unit-2/2b Completion list):
WP-23 (jaipur-2), WP-28 (lost-cities-1/-2 shared), WP-19 (acquire-1),
WP-21 (cathedral-2 + sushizock-2), WP-03 (lib-game parser mechanical).

Serial workers, one spec each. Same conventions as units 1-2: read the
work-packages.md entry, findings bodies, verification annotations, then
re-derive every fix against LIVE source; note snapshot drift; disposition
table for every original recommendation; superpowers:writing-plans
structure; test plan per docs/CODING.md + specs/notes-conventions.md.
Exemplar: specs/WP-22-lords-of-vegas-fixes.md. Lead reviews each draft
with live-source spot-checks; revision pass if gaps. No code changes.

Findings sources: WP-23 = findings/games-batch-d.md (d F14, F17-F20,
F22) + verification/games-batch-d.md; WP-28 = findings/games-batch-e.md
(e F17/F19/F20/F23/F24/F26/F27/F37/F38/F41-F44); WP-19 + WP-21 =
findings/games-batch-c.md (c F7-F11/F16-F21; c F22-F34 subset);
WP-03 = findings/lib-game.md (lg F5/F6/F8-F12/F15/F18/F20) +
games-batch-c.md c F31.

Coordination notes carried forward: WP-21 c F23 is comment-only
(verification refuted the premise - do NOT change the flood-fill);
WP-03 lg F9 fix discharges c F31; WP-19 must fence off WP-20's
BLOCKED items (c F2/F12/F13/F14/F15); WP-23 must NOT act on d F13
(rejected: Camel => 11 would create 14 camels) and must fence WP-26's
BLOCKED d F15/F16/F23-F25; WP-28 must fence WP-30's e F21/F25/F30/
F39/F40. WP-65/WP-64 own workspace-wide manifest hygiene.

## Entries (unit 3)

- [planned] Plan written. No workers dispatched yet.
- [dispatch W1] WP-23 spec (jaipur-2).
- [W1 done] specs/WP-23-jaipur-fixes.md written (637 lines). 5 tasks, all 6
  in-scope findings dispatched (d F14 T1, d F18+d F20 T2, d F19 T3, d F22 T4,
  d F17 T5), 7 new tests (4 red-first). No snapshot drift. Baseline verified
  live: 61 tests green. Dispositions: 3 confirmed(+detail), 2 adjusted,
  1 overturned-upstream restated (d F13), 4 fenced out of scope (d F15/F16/
  F23 -> WP-26; d F21 -> WP-08 epilogue dedup).
  Key overturns: d F18 - BOTH offered mechanisms are impossible (Map's
  closure is `Fn(T) -> O` and infallible, lib/game parser/mod.rs:188-206,
  no fallible map in the library; and sell() only receives good+quantity,
  never the type list) -> re-derived a crate-local SellGoodsParser impl
  Parser with to_spec/expected delegated so CommandSpec is byte-identical.
  d F20 - "index goods[0]" REJECTED (turns a silent wrong answer into a
  panic on a player-reachable path) -> let-else returning GameError::Parse.
  d F22 - "round N of 3" alternative rejected: tied rounds replay without
  incrementing round_wins (lib.rs:632-636) so a derived counter drifts.
  d F14 - uses existing MAX_TRADE_BONUS const, not a literal 5.
  Worker empirically reproduced F14 and F18 with a throwaway test (deleted;
  tree clean, no code changes). F18 is WORSE than filed: `sell dia gold lea`
  SUCCEEDS, selling 3 diamonds for 19 pts + a bonus, un-undoable.
  NEWLY DISCOVERED (nit, not fixed): render.rs never uses N::Player;
  common_rows hardcodes "Player 0 is in the lead." (render.rs:175-181)
  while every lib.rs log uses N::Player. Lead ROUTED it to WP-26 (owns the
  other jaipur display-policy item d F23, already edits render.rs, and is
  BLOCKED-ON-DECISION - a player-visible display call belongs there); spec
  cross-package section amended by the Lead to say so and to forbid folding
  it into WP-23.
  Lead spot-checked live source: MAX_TRADE_BONUS lib.rs:24; bonuses
  get_mut(&quantity) in sell() 519-521 (6/7 find nothing => no bonus);
  sell_parser p2 goods.first()+goods.len() and unwrap_or(Good::Diamond)
  command.rs:76-85; Map struct/impl Fn(T)->O infallible mod.rs:188-220;
  Parser trait sig (parse/expected/to_spec) mod.rs:21-28; Many<TP,DP> +
  some_spaced min:Some(1) mod.rs:290-317 (so `Many<Enum<Good>, Space>` in
  the spec's struct field is well-formed); render.rs common_rows literals
  and 3u8.saturating_sub; end_round tied-replay branch lib.rs:625-640.
  All confirmed. ACCEPTED without revision (Lead added the F26 routing).
- [dispatch W2] WP-28 spec (lost-cities-1/-2 shared fixes).
- [W2 done] specs/WP-28-lost-cities-shared-fixes.md written (730 lines).
  10 tasks, all 13 in-scope findings dispatched (e F17/F19/F20/F23/F24/F26/
  F27/F37/F38/F41/F42/F43/F44). No snapshot drift (both crates diff-clean vs
  f8763a5). Baseline measured: both crates 7 lib tests + game_contract green.
  -1 vs -2 divergence DIFFED, not assumed: command.rs byte-identical;
  card.rs/Cargo.toml/contract.rs differ only cosmetically; shared+identical
  fix = F19/F37 (draw logs), F20/F38 (hand sorting); shared but fix text
  differs = F26/F41 (HAND_SIZE vs hand_size(self.players)), F43 (literals vs
  named consts); -2 ONLY = F17, F23, F24; -1 ONLY = F42 (-2 has no PLAYERS
  const). -2's renderer is a genuine rewrite (331 vs 235 lines, 2p/3p
  branch) - deliberately NOT reconciled.
  Dispositions: 6 confirmed as recommended; 1 OVERTURNED, 4 adjusted:
  e F23 OVERTURNED - the recommended `% self.players` would INTRODUCE a
  divide-by-zero panic (PubState.players deserializes to 0) where
  `% MAX_PLAYERS` was panic-free; fix reuses the already-correct clamp form
  at render.rs:51-54, and must also drop MAX_PLAYERS from render.rs:9 or
  -D warnings fails. e F27 ADJUSTED/extended - game_types conflict key is
  typeName and BOTH manifests say "Lost Cities" (operator controller.rs:
  181-196 ON CONFLICT (name)), so editing only -2 leaves a landmine; both
  blurbs get identical text. e F43+F44 ADJUSTED, widened to BOTH crates
  (identical constructs at -2 lib.rs:718-729, render.rs:264/282); F44 form
  changed to map_or(0, Vec::len) / and_then(|c| c.get(row_i)). e F42
  ADJUSTED, widened: 7 lib.rs sites + render.rs:39, not 4 (excludes
  render.rs:116 min(..,1) = last-index, and lib.rs:686's 20/8 = costs).
  e F24 confirmed but implementation re-derived (-1's 2-element score array
  + opponent() is uncopyable; generalized over leaders() with a sorted Vec
  for log determinism).
  game-version.yaml resolution: manifest text edits only in BOTH
  k8s/base/game/lost-cities-{1,2}/game-version.yaml (blurb line 9); no
  kubectl/deploy anywhere in the spec (prod mutations are Michael's call).
  Worker empirically validated F17 (red: left 3 right 2), F19, F23, F24,
  F26 (debug overflow panic), F43/F44 with scratch edits, then reverted;
  git status confirmed clean by Lead (only docs/BACKLOG.md, docs/archive/
  BACKLOG.md pre-existing + untracked docs/reviews/).
  NEWLY DISCOVERED, routed by Lead: (1) REAL user-visible defect - the same
  shared game_types row makes `player_counts` last-writer-wins between -1's
  [2] and -2's [2,3], so the new-game page may offer Lost Cities as 2p-only;
  ROUTED TO WP-62 (operator, READY; exactly its controller.rs path) - spec
  cross-package section amended by the Lead to name WP-62 and instruct its
  spec writer to take it as an added item. (2) -1's command() flattens parse
  errors to invalid_input (lib.rs:625) where -2 preserves them (lib.rs:630).
  (3) -1's score() hardcodes 20/8 where -2 uses named consts. (4) both
  crates emit "who won" twice (placings_log + game_over_log) - WP-08
  adjacent but neither crate is in WP-08's scope list. (2)-(4) recorded in
  the spec, not fixed, no tests baked in.
  Lead rulings on the worker's two open questions: (a) widening F43/F44/F42
  to both crates is APPROVED - anti-drift is this package's stated purpose
  and both crates' src are in its declared paths; (b) preserving the
  self.hands[player] panic is CORRECT - e F18/F22/F36 belong to WP-09
  (BLOCKED-ON-DECISION D-36); WP-28 must not discharge it.
  Lead spot-checked live source: status() stats: vec![player_stats(0),
  player_stats(1)] at -2 lib.rs:534 (F17 confirmed) vs placings() 491-497
  generalized and player_stats' own bounds guard 457-459; render.rs:130
  `% MAX_PLAYERS` + MAX_PLAYERS import line 9 + the correct clamp at 51-54;
  -1 render.rs:116 cmp::min(..,1); -1 const PLAYERS lib.rs:25 used at
  124/509/634; both game-version.yaml files (identical two-player blurb,
  identical typeName, -1 isDeprecated); operator controller.rs:181-196
  ON CONFLICT (name) DO UPDATE SET player_counts/weight/blurb; -2
  PubState.players + Game.players fields; -2 start() stats push loop
  512-521. All confirmed. ACCEPTED without revision.
- [dispatch W3] WP-19 spec (acquire-1 fixes).
- [W3 done] specs/WP-19-acquire-fixes.md written (841 lines). 9 tasks, all 11
  in-scope findings dispatched (c F7-F11, F16-F21). No snapshot drift.
  Baseline measured: 11 tests green (10 lib + game_contract), clippy/fmt
  clean. Dispositions: 11 confirmed-or-adjusted, 0 overturned, 0 skipped,
  4 fenced out of scope (c F2/F12/F13/F14/F15 -> WP-20).
  c F7 (6-player never offered) RESOLVED and proven separable from WP-20:
  cause is the half-open `(2..6).collect()` at lib.rs:313 while
  MAX_PLAYERS = 6 and start() accepts 6; fix `(MIN_PLAYERS..=MAX_PLAYERS)`
  -> [2,3,4,5,6]. Worker empirically started 2..6-player games and built
  every seat's player_state/command_spec; tile capacity re-derived (42 of
  108 tiles at 6p). MIN/MAX_PLAYERS, start() bounds and RULES.md untouched,
  so no WP-20 edition/player-cap decision is presupposed; contract harness
  unadvertised-count probe is 0 before and after.
  c F8 (dummy die never rolls 6) RESOLVED: `random_range(1..=5)` at
  lib.rs:902 -> `1..=6`. Test is SEEDED not statistical (ChaCha8 stream is
  portable/stable per lib/game/src/rng.rs:8-16), 1000 draws from
  seed_from_u64(42) asserting all six faces; worker measured first 6 within
  16 draws at four stream offsets, so it cannot flake.
  Adjusted: c F19 (recommended `if let Some(s) = start` still needs the
  is_none() pre-assignment + a dead None arm; `let s = *start.get_or_insert(
  col);` is one line, identical emitted tuples); c F21 (symptom worse than
  filed - 50/50 DISTINCT specs from 50 consecutive command_spec calls in ONE
  process because available_corps() builds a fresh HashSet per call; scope
  extended to the identical uncited defect at command.rs:43-52 merge parser
  via neighbouring_corps, flagged loudly as an extension); c F20 (cost worse
  than filed - command_spec is called per seat per request by renders(), so
  a 6p game pays 6+ whole-game clones per request).
  Panic hardening done properly: c F9 proven command-reachable-but-latent
  (founder free share + shares only disposable while the corp is merged
  away, after bonuses pay) -> GameError::Internal, and gamer.rs:88-107
  verified to return no game payload on Err so the already-rolled die is not
  persisted. All 10 c F10 sites individually re-derived to show 0 yields a
  correct rejection/skip/"0", never a silent wrong payout; Int::bounded(1,0)
  confirmed non-panicking against parser/mod.rs:119-175; take_shares/
  return_shares return before the entry().or_insert() lines that would
  resurrect a key and underflow.
  NEWLY DISCOVERED: (1) two panic!("must be Phase::SellOrTrade") at
  lib.rs:951 and 980 - same class as c F9, command-reachable functions,
  provably unreachable today, NOT a filed finding. Worker asked for a scope
  call; LEAD RULING: ROUTED TO WP-09 (its whole subject is this class of
  invariant panic; acquire-1 must be ADDED to WP-09's crate list) and
  explicitly NOT folded into WP-19 - absorbing unfiled fixes into a
  fixed-scope package is the drift this discipline forbids. Spec amended by
  the Lead accordingly; Task 9's panic-grep step whitelists the two sites so
  WP-09 and WP-19 do not fight. (2) RULES.md:60 documents a "Tycoon Mode
  (3+ players)" tertiary bonus that is never paid and has no mode selection
  -> routed to WP-20 (pure edition/rules question, do not silently edit the
  doc). (3) command.rs:43-52 HashSet ordering - same defect as c F21, fixed
  in Task 8 as a declared extension. (4) board.rs:3 prelude-redundant
  `use std::iter::FromIterator;` - cosmetic, clippy silent, left alone,
  route to WP-64/65 if wanted.
  Coordination point (not a defect): acquire-1 carries WP-08's copy-pasted
  placings epilogue shape (lib.rs:286-294) but is ABSENT from WP-08's crate
  list - WP-08's spec writer must decide whether it joins.
  Lead spot-checked live source: player_counts (2..6) lib.rs:313;
  random_range(1..=5) lib.rs:902; both panic!("must be Phase::SellOrTrade")
  at 951/980; command.rs:43-52 neighbouring_corps().into_iter().collect();
  rng.rs ChaCha8 portability doc 8-16; git status clean (no source changes,
  scratch test deleted). All confirmed. ACCEPTED without revision.
- [dispatch W4] WP-21 spec (cathedral-2 + sushizock-2).
- [note] WP-21 has PRE-EXISTING stray-edit material the spec Lead/writer MUST
  read before drafting: `raw/cathedral-stray-edits-notes.md` +
  `raw/cathedral-stray-edits.diff`. A stray subagent edited three cathedral-2
  files during the read-only phase; the edits were captured and REVERTED (diff
  is reference material, not applied code). Covers c F22 (adopt as-is; also
  closes c F28), c F26 (adopt, but its "mirrors Go" rationale is factually
  wrong - do not repeat it), c F25 (PARTIAL - empty-vec closes the panic but
  prefer boundary validation + Option/Result).

# specs-LOG - unit 3 RESUME (unit 3b)

Date: 2026-07-25. Prior unit-3 Lead died after dispatching W4 (WP-21) and
logging the stray-edits note; no WP-21 draft exists (specs/ has no WP-21
or WP-03 file). Assessment: WP-23, WP-28, WP-19 written AND
Lead-reviewed/accepted (entries above). Resume plan: dispatch one Worker
for WP-21, review, then one for WP-03, review, close unit 3.

NEW HARD READ-ONLY CONSTRAINTS for this and every later unit (user made
them explicit after a prior subagent violated them):
- No file may be created/modified/deleted outside planning/specs/,
  planning/raw/ and appends to planning/specs-LOG.md.
- NEVER modify any file under rust/ - not even a one-line experiment.
- NEVER run cargo (build/check/test/clippy/fmt) or any build/test command.
  Validation is by READING source only. Earlier units' "empirically
  reproduced with a scratch test" technique is now FORBIDDEN.
- NEVER run git mutations (add/commit/checkout/stash/reset/clean/rm).
Workers run on model opus (user override for this session).

## Entries (unit 3b)

- [resume] Assessment logged. Dispatching W4 (WP-21).
- [W4 done] specs/WP-21-cathedral-sushizock-fixes.md written (1082 lines). 10
  tasks, all 12 in-scope findings dispatched (T1 c F22 + c F28, T2 c F26,
  T3 c F25, T4 c F24, T5 c F23 comment-only, T6 c F27, T7 c F29, T8 c F30,
  T9 c F32, T10 c F33 + c F34 + final gate). No snapshot drift (both crates
  diff-clean vs f8763a5, empty diff -ru). Worker validated by READING ONLY -
  no cargo, no edits under rust/ (confirmed by Lead: git status shows only
  the pre-existing docs/BACKLOG.md pair + untracked docs/reviews/).
  Stray-edit judgements incorporated as directed: Edit A adopted as-is
  (Task 1) and c F28 closed as resolved-by-A with an explicit "do NOT delete
  impl Display for Loc" instruction; Edit B adopted with the "mirrors Go"
  rationale REPLACED (correct rationale: latent overflow/garbage-key hazard +
  unifies the contract with render.rs's Tiler guard; the real safety net is
  the separate !l.valid() at lib.rs:143 which must stay); Edit C REJECTED as
  insufficient - Task 3 instead does real boundary validation
  (Option-shaped piece::pieces, a players-aware Game::player_pieces,
  a range guard in command_parser as the single choke point shared by
  Gamer::command AND command_spec, GameError::internal in command,
  remaining_piece_size -> Option<i32>, and an explicit render marker because
  the Renderer/player_state signature cannot return a Result).
  Dispositions: 5 confirmed, 5 adjusted, 1 resolved-by, 1 skipped, 1 fenced.
  Overturns: c F23 code change OVERTURNED (RULES.md:59-83 documents the
  cathedral as a capturable identity INSIDE a region - blocking the walk
  would contradict the crate's own rules doc and break
  capture_returns_piece_but_never_counts_cathedral; comment only);
  c F26 rationale overturned; c F25's empty-vec option rejected; c F30's
  hoist-after-the-match alternative rejected; c F24's keep-and-test option
  rejected. c F31 fenced to WP-03/lg F9. c F32 shown RNG-stream-identical
  by reading pinned rand 0.10.2 (choose == self[random_range(..len)]), which
  matters because Game.rng is persisted.
  NEWLY DISCOVERED + LEAD RULINGS (amended into the spec by the Lead):
  (1) sushizock-2 `Player {}` parser returns a target index bounded only by
  names.len(), never self.players -> player_blue_tiles[target] panic ->
  ROUTED TO WP-09 (already owns gamer.rs + per-crate sweep; sushizock-2 must
  be ADDED to its crate list); explicitly NOT folded into Task 10.
  (2) can_play_something rebuilt the piece catalogue inside its 100-iteration
  loop - hoist is a forced consequence of Task 3, documented, no routing.
  (3) Gamer::player_state cannot reject an invalid player in ANY crate
  (lib/game/src/game.rs:52 returns Self::PlayerState, gamer.rs forwards
  unvalidated) -> ROUTED TO WP-09 as the general form of its e F18/F36
  bounds-check item; WP-06's finalized spec must NOT be retro-edited.
  Lead spot-checked LIVE source (read-only): command.rs LocChoice/loc_name/
  Box::leak block 15-34 + loc_parser 98-107 + Map still used at 62/94/118 +
  glob import line 4; loc.rs valid()/to_key() 'A'+y as u8/Display 118-122/
  all_locs row-major (so [0]="A1", [99]="J10")/parse_loc 166-181; piece.rs
  pieces panic arm; lib.rs tile_at 85-90 (no guard), can_play_piece
  !l.valid() at 143 before tile_at 146, play's pieces() + finish log
  remaining_piece_size(...).to_string(), remaining_piece_size 343-352,
  can_play_something pieces() INSIDE the loc loop, calc_placings metric,
  Gamer::start played_pieces line, command_parser(&self, player: i32)
  signature (so the guard's `player < 0` arm is well-typed), Gamer::command
  (player: usize) + command_spec + points, check_captures inner walk
  `== player` only; render.rs Tiler valid() guard 37-50 + wall_char
  + render_player_remaining_tiles(state: &PubState, p_num) pieces() at 359
  + NamedColor/N::Fg already imported + PubState.players/played_pieces pub;
  test module named `test` with fn players() at 552 and pieces(0/1) at
  1037-1038; Renderer::render -> Vec<Node> (game.rs:134-136) and cathedral's
  two impls; GameError::Internal + GameError::internal (errors.rs:20,36);
  Spec::Enum { values, exact } (command/mod.rs:20-23); Enum<T: ToString +
  Clone> + partial() + parse/expected/to_spec all via to_string()
  (parser/mod.rs:551-576, 605-681); RULES.md:59-83; cathedral Cargo.toml:14
  rand; sushizock dice_counts_all() EXISTS as a method (used by
  can_take_blue) so Task 10's helper compiles, take_blue/red 399-431,
  steal_blue/red guard-order + `len as i32 - n` at both sites, take_worst
  527-566 first-minimum strict-< loops + blue_tiles[0], roll_dice
  choose().unwrap() 150-152, roll_dice_cmd take_worst at :612, Roll arm
  without the placings block vs Take/Steal arms with it (736/757),
  command_parser is_finished -> None, test consts MICK/names(),
  test_take_worst_red_picks_minimum 1686 / _blue_when_no_red 1716.
  All confirmed. Only nits: two test line cites off by one (1715 vs 1716)
  and "lib.rs:215" for the play finish log (actually 219-221) - immaterial,
  the spec instructs locating by symbol where numbering shifts.
  ACCEPTED without revision (Lead added the two WP-09 routings).
- [dispatch W5] WP-03 spec (lib-game parser mechanical).
- [W5 done] specs/WP-03-lib-game-parser-mechanical.md written (1316+ lines).
  8 tasks, all 11 in-scope items dispatched (T1 lg F8, T2 lg F6, T3 lg F5,
  T4 lg F9 + c F31, T5 lg F10, T6 lg F18 + lg F20, T7 lg F11 + lg F12,
  T8 lg F15 + final gate). No snapshot drift (lib/game and sushizock-2 both
  diff-clean vs f8763a5). Read-only validation only; git status clean.
  CALLER AUDIT: zero breaking call sites - no public signature change, no
  serialized-shape change (Spec/Suggestion/Output untouched; consumers are
  rust/web components/game.rs:580 + lib/cmd; no TS mirror). Behavioral blast
  radius audited: the only in-tree proper-prefix Enum::partial value list is
  cathedral-2's loc enum (A1 vs A10), provably unchanged.
  WP-01 COORDINATION (table in the spec): only overlap is Enum::parse -
  WP-01 Task 3 rewrites the method and explicitly leaves the replace-vs-push
  asymmetry to WP-03; WP-03 Task 3 rewrites ONLY the final ranking if-block.
  LANDING ORDER: WP-01 first, then WP-03; Task 3 opens with a hard
  shared_prefix-signature precondition check plus a documented contingency.
  All other tasks textually disjoint.
  Dispositions: 8 confirmed, 3 adjusted, 0 overturned, 0 skipped, 5 fenced
  to WP-04 (lg F7/F13/F14/F17/F19, each with a "why it looks adjacent"
  note). Adjustments: lg F8's own recommendation is WRONG as written
  (dropping the early return would let max==0 parse unboundedly because the
  typed loop tests max AFTER pushing) -> the max check MOVES to the loop
  head as >= max, mirroring the spec impl; lg F18's optional OneOf-dedupe
  half declined (branches carry independent Doc descs; OneOf::parse has no
  dedupe to mirror); lg F5 rule choice (see ruling).
  LEAD RULING on lg F5: worker chose ranking key (matched length, then
  full-match) i.e. longest-wins, instead of the finding's full-match-first
  recommendation. APPROVED, with the spec's "would be a regression"
  framing CORRECTED by the Lead (full-match-first is today's behavior in one
  ordering - a wart, not a regression) and the adjustment re-labelled in the
  spec as a DELIBERATE user-visible policy change to prefix-overlapping
  Enum/Player resolution, with the reopen path routed to WP-04/D-38 rather
  than to a future re-edit of the task. Approved because it is
  order-independent (all lg F5 requires), cathedral's A1/A10 behavior is
  provably unchanged, and the only runtime delta (player names that prefix
  each other) resolves the way the player meant.
  c F31 DISCHARGE proven by reading sushizock-2 command.rs:38-50 +
  Many::to_spec (parser/mod.rs:415-422): the spec really carries
  max = rolled_dice.len(), and suggest.rs:109's `Spec::Many { spec, delim,
  .. }` discards it. Pinned by a lib-side test that reconstructs sushizock's
  to_spec() shape locally - no game-crate file touched. Note recorded that
  c F31's wording is loose: the die-number VALUES were already bounded by
  Spec::Int; the ITEM COUNT was not.
  NEWLY DISCOVERED (recorded, not fixed): (a) Spec::Int suggest uses
  min.unwrap_or(1), implying a floor of 1 where the parser accepts 0 and
  negatives - after Task 7 suggest is the last of the three views still
  lying -> routed WP-04; (b) sub-case: suggest filters on the raw fragment
  so typing "-" yields nothing -> WP-04; (c) Enum::parse's dedupe key is the
  lowercased value so "Red"/"red" silently collapse (undocumented) -> WP-04
  comment. (d) WP-01's spec line 35 mis-assigned lg F13/F14 to WP-03 and lg
  F18/F20 to WP-04, contradicting work-packages.md. LEAD ACTION: corrected
  that one line IN WP-01's spec (routing-label only, no task changed, edit
  annotated inline). work-packages.md is authoritative and needed no change.
  Lead spot-checked LIVE source (read-only): Cargo.toml name = brdgme_game
  (underscore) + combine = "4.6.7" at :12 with ZERO `combine` uses under
  lib/game/src (F15 confirmed); `use std::cmp::Ordering` already at
  parser/mod.rs:1 (Task 3's match compiles); Enum::parse ranking block
  626-636 verbatim as quoted + the stale comment 608-609 + the lowercase
  dedupe `searched` set + all three matched.len() arms; typed Many::parse
  early return 342-349 (max==0 || max<min) and the post-push
  `parsed.len() == max` break at 371-375 and the min check at 380-394
  (confirms the F8 adjustment); Many::to_spec propagating min/max 415-422;
  Many::expected 402-413 (left untouched per the WP-04 fence);
  suggest.rs:109 `Spec::Many { spec, delim, .. }` dropping min/max;
  suggest Spec::Int `start + 4` at :87 (F10) and min.unwrap_or(1) (newly
  discovered (a)); suggest Spec::Token arm 25-34 (no empty-token guard,
  F20) and Spec::Enum arm 35-45 (no dedupe, F18) both using to_lowercase
  (F17 fence intact); doc.rs doc_int `(min, Some(max))` -> min.unwrap_or(0)
  at :51 (F11) and the Spec::Many -> doc_many dispatch 23-29 (F12);
  sushizock-2 roll_parser Many::bounded_spaced(Int::bounded(1, max), 1, max)
  at command.rs:40-50 (c F31 discharge). All confirmed.
  ACCEPTED without revision (Lead added the F5 ruling text and the WP-01
  routing-line correction).

## Completion (unit 3 / 3b)

Unit COMPLETE 2026-07-25. Five packages done for unit 3 overall:
1. specs/WP-23-jaipur-fixes.md (unit 3, reviewed+accepted)
2. specs/WP-28-lost-cities-shared-fixes.md (unit 3, reviewed+accepted)
3. specs/WP-19-acquire-fixes.md (unit 3, reviewed+accepted)
4. specs/WP-21-cathedral-sushizock-fixes.md (unit 3b, reviewed+accepted)
5. specs/WP-03-lib-game-parser-mechanical.md (unit 3b, reviewed+accepted)

17 of 39 READY packages now specced.

REMAINING for the next spec-writing Lead: 22 of 39 READY packages, in
backlog order: WP-41, WP-37, WP-59 (Phase 3); WP-29 (Phase 4, sequence
after WP-30); WP-54, WP-51, WP-60, WP-52, WP-53, WP-61, WP-62, WP-63,
WP-08, WP-27, WP-24, WP-18, WP-31, WP-32, WP-33, WP-43 (Phase 5); WP-65
(Phase 6, after WP-64). (WP-02 sits with WP-01's markup items - it is not
on the READY list carried forward from unit 2/2b; do not add packages to
the list without checking BACKLOG.md.)

Cross-spec coordination added this unit (3b):
- WP-09 gains TWO added items from WP-21: (1) sushizock-2 must be ADDED to
  its crate list for the `Player {}` parser's unbounded `target` index
  (steal_blue/steal_red index player_*_tiles[target]); (2) the general
  statement of its e F18/F36 item - `Gamer::player_state` cannot reject an
  invalid player in ANY crate, so validate `player < game.player_count()`
  in `handle_player_render` (lib/cmd/src/requester/gamer.rs). WP-06's
  finalized spec must NOT be retro-edited for this.
- WP-04 gains three recorded-not-fixed items from WP-03: Spec::Int suggest
  implying a floor of 1; suggest filtering on the raw fragment so "-"
  suggests nothing; Enum::parse's lowercased dedupe key collapsing
  "Red"/"red". Plus WP-04 is the reopen path for WP-03 Task 3's Enum
  ranking policy if it is ever contested.
- WP-01 must land BEFORE WP-03 (WP-03 Task 3 edits lines WP-01 Task 3
  rewrites in Enum::parse; every other WP-03 task is disjoint).
- WP-01's spec Non-Goals routing line was corrected in place (F13/F14 ->
  WP-04, F18/F20 -> WP-03) to match work-packages.md.

Compliance for unit 3b: NO file under rust/ was created, modified or
deleted; NO cargo/build/check/test/clippy/fmt command was run by the Lead
or either Worker (all validation was by reading source, plus read-only
diff/grep/git status); NO git mutation was run. Writes were confined to
planning/specs/*.md and this LOG.

---

## WP-41 db.rs quality pass - spec written (Worker, 2026-07-25)

Deliverable: `planning/specs/WP-41-db-quality-pass.md` (1893 lines, 11 tasks
plus a final gate). All 16 scope items (ws F35, F36, F37, F39-F51 minus the
WP-40-owned F34/F38) re-derived from LIVE source at
`/home/beefsack/Development/brdgme/rust/web/src/db.rs`.

Snapshot drift: YES. db.rs 6380 -> 6877 lines, 13 hunks, +503/-6 vs
`brdgme-review-snapshot` (f8763a5), all from the #47 concede/end-game work.
Findings' line citations are off by up to +500; the spec carries a full
snapshot->live mapping table and uses live numbers throughout.

Dispositions: CONFIRMED 8 (F37, F39, F40, F43, F45, F46, F47, F48, F49, F50
- of which F37/F39/F40 have ADJUSTED recommendations and F43/F46 are
narrowed to their doc halves), ADJUSTED 3 (F35, F36, F51), OVERTURNED 2
(F44 code change, F51 part 1), SKIPPED-BY-DECISION 1 (F41), FENCED 1 (F42).

Overturned recommendations:
- F44 (`is_turn_at` reset): the same UPDATE also clears
  `turn_reminder_sent_at` (db.rs:1936) and the reminder sweep gates on both
  (email/sweep.rs:60-68), so "last turn activity" is coherent and
  intentional; changing it would nag a player who just acted. Doc only.
- F51 part 1 (`suggestions_exclude_blocked_and_self` over-promises): the
  fixture makes `me` a game_players row (db.rs:4197), so self-exclusion IS
  mutation-covered. No rename.
- F37's "clear finished_at" option: `undo_game` is the deliberate un-finish
  path and already NULLs it; clearing on the command path would erase a real
  finish. Spec makes `is_finished` sticky instead.
- F39's `ON CONFLICT` option: two-expression index inference plus DO NOTHING
  would swallow the auto-accept. Spec uses `pg_advisory_xact_lock` on the
  ordered pair.
- F46's "retry on 23505": all four callers are inside open transactions, so
  a retry needs SAVEPOINT plumbing in four modules for a nit. Doc only.
- F43's `Option<GameTypeUser>`: no caller reads `id`; the struct is WP-53's.

F36 sweep re-derived from live grep: 25 sites on the 14 trigger-maintained
tables (migration 001:392-446), enumerated line-by-line with replacement SQL.
EXCLUDES live :1487/:1493 (game_proposals, migration 015, no trigger) per the
triage note, plus an added warning comment so they are not swept later.

F35 re-enumerated: 26 public db.rs fns with ZERO references in the test
module (:3139-6877) or `rust/web/tests/*`. The finding's list was incomplete.
Spec adds 11 `#[sqlx::test]`s covering 24 of them (all written out verbatim),
with a documented cut rule; excludes `create_pool` (needs DATABASE_URL) and
`create_game_with_users_tx` (transitively covered by ~100 fixture games).

Routed out of WP-41:
- concede_game's 3+-player `debug_assert!` (db.rs:1315) -> WP-40 (D-3 gates
  the correct behaviour).
- The four remaining `($1 || ' seconds')::interval` sites (proposals.rs:725,
  755, 819; email/sweep.rs:65) -> WP-44 / WP-46.
- `find_active_turn_games`' dead `NULLS LAST` (is_turn_at is NOT NULL,
  001:193) -> WP-52, cleanup-if-touched only.
- db.rs module split (F42) -> new package AFTER WP-35/40/45/47/49/50/52/53
  land their db.rs edits.

Landing order: WP-41 BEFORE WP-40 (shared functions concede_game, undo_game,
apply_rating_changes - WP-40 restructures, WP-41 only deletes clauses) and
BEFORE WP-47 (which adds callers of the visibility predicate WP-41 inlines a
second copy of, with a mandatory cross-ref comment). WP-41 also lands before
WP-37/WP-53 for the `is_user_admin` signature change (verified
caller-source-compatible: `error.rs:7` `internal<E: Display>` accepts
anyhow::Error at all 19 sites, so no file outside db.rs changes).

`.sqlx` note carried in the spec: Tasks 1 and 3 change `query!` macro SQL, so
the implementer must regenerate the offline cache via the DEV.md:82-95
scratch-DB flow before the final commit or `cargo sqlx prepare --check` fails.

Compliance: NO file under rust/ was created, modified or deleted; NO
cargo/build/check/test/clippy/fmt or other build command was run (all
validation by reading source plus read-only diff/grep/sed/git status); NO git
mutation was run. Writes confined to planning/specs/WP-41-db-quality-pass.md
and this LOG.

---

## WP-37 admin.rs pass - spec written 2026-07-25

File: `planning/specs/WP-37-admin-pass.md` (2254 lines, 13 tasks).

Scope 14 (ws F18-F26, F28, F29, F31-F33). Disposition: 11 CONFIRMED as-written
+ 3 CONFIRMED-with-adjusted-recommendation (F22, F24, F31) = 14 implemented;
0 OVERTURNED; 1 FENCED (ws F27 -> WP-38/D-5); 1 SKIPPED (ws F30 rejected -
`bot_providers` has NO `updated_at` column, re-verified against
migrations/013_bot_efficacy.sql:23-34 + grep over 014-022; the only later
`bots`/`bot_providers` DDL is 022:16 `can_replace_humans`).

Adjusted recommendations (evidence in the spec's disposition table):
- ws F22: no model column exists on `llm_providers`/`bots` (only
  `bot_providers.model`), so the fix resolves the highest-priority enabled
  link model, with an optional explicit override, and DROPS the gpt-4o-mini
  fallback (keeping it preserves the false negative). No new migration.
- ws F24: verified against reactive_graph-0.2.14 action.rs:287-297 - `input()`
  clears when in_flight hits 0, so the claimed cross-attribution cannot occur;
  the real failure is a dropped result panel. Finding's fix (id inside the
  value) is correct under both readings and is what the spec does.
- ws F31: exact equality on `e.to_string()` can NEVER match
  (server_fn-0.8.13 error.rs:233 prefixes "error running server function: ").
  Spec matches the `ServerFnError::ServerError` variant against a shared
  `ADMIN_REQUIRED` const instead.

STATED ASSUMPTION requiring Lead awareness (ws F21): blank API-key field keeps
its current meaning ("keep existing"); clearing requires a new explicit
"Clear API key" checkbox which wins over typed text. Alternative
(blank == clear) rejected because it would destroy keys on routine URL edits.
Labelled as an assumption block in Task 8.

Landing order: WP-41 SHOULD precede (its `is_user_admin` -> anyhow::Result
change), but neither order breaks the build - WP-41's spec verified
caller-source-compatibility, and WP-37 Task 1 reduces admin.rs from 15 call
sites to 1. WP-38 must follow (shares admin.rs, D-5 will touch
delete_bot/update_bot). WP-54 does not include admin.rs, so ws F24/F32 are
NOT WP-54 overlaps - fenced explicitly in the spec.

Snapshot drift: NOT clean - 21 hunks, all the post-review
`bots.can_replace_humans` feature (migration 022, #47). No finding site
modified; all spec line numbers re-derived live.

Newly discovered (routed, not fixed): (1) reactive_graph's `is_latest` guard
is dead code - `dispatched` never incremented (upstream; WP-43 nearest owner);
(2) AdminPage renders raw ServerFnError Display text, `user_facing_server_error`
unused (WP-54, whose path list would need extending); (3) `bots.display_order`
has no unique index (backlog, needs migration 023); (4) `create_provider`
accepts an empty-string API key (Lead decision); (5) admin test fns will POST
to any admin-supplied http(s) URL incl. internal addresses (documented trust
boundary, no change proposed).

Compliance: NO file under rust/ was created, modified or deleted; NO
cargo/build/check/test/clippy/fmt/sqlx command was run (validation by reading
live source, the review snapshot diff, and the vendored
reactive_graph/server_fn/reqwest crate sources); NO git mutation was run.
Writes confined to planning/specs/WP-37-admin-pass.md and this LOG.

---

## WP-59 inbound processing quality — spec written 2026-07-25

Path: planning/specs/WP-59-inbound-processing-quality.md (14 tasks, 16 findings).

Dispositions: 11 CONFIRMED (F4, F6, F7, F8, F12, F13, F15, F24, F27, F28, F29),
5 ADJUSTED (F9, F14, F21, F23, F26). 0 findings fully overturned; 3 recommendations
overturned + 1 sub-recommendation skipped by decision.

Overturned recommendations:
- F21 premise "internal errors emailed verbatim and unlogged" is FALSE -
  `crate::error::internal` already logs and redacts to "Internal server error".
  Real defect found instead: `ServerFnError`'s Display prepends "error running
  server function: " to EVERY message (server_fn-0.8.13/src/error.rs:233-234), so
  restart refusals are emailed with framework noise. Fixed by a classifier, not a
  typed error enum (which would collide with WP-40 in server_fns.rs).
- F23 "join login_confirmations on code" rejected: selecting by code bypasses
  `validate_confirmation_code`'s attempt-cap bump (auth/server.rs:376-386).
  Spec iterates the pending addresses instead.
- F28 "(reply bump again for the rest)" rejected: `find_active_turn_games` has a
  fixed ORDER BY and no cursor, so a second bump re-sends the same games.
- F9 AppState field rejected in favour of a shared fn (trait object in a Clone
  struct + 3 construction sites for one env lookup).
- F14 render.rs excluded (its brdg.me literals are a Message-Id domain and the
  unsubscribe mailto, owned by WP-60/WP-58) — this package leaves render.rs alone.
- F26 scope cut in half: db.rs ALREADY has byte-identical `get_user_email_prefs`
  and the three `set_user_*_emails_enabled` helpers (with tests), so 4 of 6 sites
  need no new db.rs code. Only 2 new helpers, both plain queries -> no sqlx prepare.
- F6's "forgiving trailing noise" SKIPPED-BY-DECISION.

D-15 / F29 verb-collision check: **COLLISION FOUND (premise refuted).** `end` is a
live top-level move in acquire-1/src/command.rs:192-197 and
starship-catan-1/src/command.rs:309-313, and `dispatch_email_command`'s "end" arm
(commands.rs:1219, added post-snapshot by #47) intercepts it. Option A specced as
instructed (docs/authoring/COMMANDS.md, new "Reserved verbs on the email path"
section + a code cross-reference comment), but D-15 needs re-opening: its recorded
basis "no current collision" is false. USER DECISION REQUIRED.

Snapshot drift vs f8763a5: inbound.rs, notify.rs, render.rs CLEAN (0 lines).
commands.rs 126 diff lines and db.rs 606 diff lines, both from #47 (concede
replacement, `end`, ranked placings). All commands.rs finding citations are stale;
a stale-citation map is in the spec. All spec line numbers are live.

Assumptions the Lead may want to confirm:
1. Task 12/13's tests are conditional on an existing `StandaloneCommandCtx` /
   multi-game test fixture; the spec instructs skip-and-record rather than
   building a NATS harness for a nit.
2. Task 9 adds `INTERNAL_ERROR_MESSAGE` to `src/error.rs` (a file not in WP-59's
   declared path list) — needed to avoid a magic-literal comparison.
3. Reply domain stays a `const`, not config/env.

Newly discovered (routed, not fixed): (1) the live `end` collision (above,
D-15 + game packages); (2) ServerFnError Display prefix leaking wherever a
ServerFnError is stringified, `user_facing_server_error` under-used (WP-54, same
class WP-37 flagged for AdminPage); (3) `SELECT game_version_id FROM games` inlined
twice more at server_fns.rs:2333,:2375 (WP-40/WP-53); (4) login_confirmations
DELETE inlined twice in auth/server.rs:486,:850 (auth packages); (5) `cap_digest`
was unreachable-dead in `bump_reply` pre-fix, check its other callers; (6) no
AppState test fixture exists for the email handlers — blocks coverage for
handle_invite_reply and will block WP-57.

Landing order: WP-41 before WP-59's db.rs additions (WP-41 is finalized and its
top-of-file header + test-module append collide textually). WP-59 Task 1 before or
independent of WP-56 — designed for ZERO overlap: the F4 fix lands in
`resend_webhook` and `select_route`, and does NOT touch
`from_matches_verified_email` / `resolve_user_by_verified_from`, which are D-1's.
WP-59 before WP-57 (Task 5 shrinks handle_settings_reply_route) and before WP-40
(one-line run_restart map_err; run_concede/run_end/run_undo untouched).

Compliance: NO file under rust/ was created, modified or deleted; NO
cargo/build/check/test/clippy/fmt/sqlx command was run (validation by reading live
source, the review-snapshot diff, and the vendored server_fn/mail-parser crate
sources); NO git mutation was run. Writes confined to
planning/specs/WP-59-inbound-processing-quality.md and this LOG.

## WP-29 red7-1 cleanup — written (2026-07-25)

Spec: `planning/specs/WP-29-red7-cleanup.md` (398 lines, 5 tasks).
Scope: e F31, e F32, e F33, e F34, e F35. Snapshot drift: NONE
(`diff -ru` of the whole `game/red7-1` tree vs snapshot f8763a5 exits 0).

Dispositions: 3 CONFIRMED-as-is (F31, F33, F35 facts), 2 ADJUSTED
(F32 extended to name the deck-exhaustion game end; F34 downgraded to
comment-only), 0 OVERTURNED findings, 2 sub-recommendations REJECTED
(F33's "or drop the re-export" — `mod card` is private so Card/Suit would
become unnameable while still appearing in pub PubState fields; F34's
"return Option" — relocates the panic into three `.expect()`s, breaks a
pub API for an unreachable nit, and collides with WP-30/D-29 which owns
`leader`/`leader_with_suit`). Fenced: e F29 -> WP-01 Task 6 (confirmed it
covers `red7-1/src/command.rs:23-42`), e F30 + D-29/D-40 -> WP-30,
e F45/e F46 -> **WP-73** (not WP-08/WP-33 as the brief guessed; neither of
those lists any red7-1 path), e F9 -> WP-65, deserialized-state trust ->
WP-09 (D-36).

Sequencing: Tasks 1-3 (F33/F35/F34, code+comment) are D-29-independent and
may land while WP-30 is blocked. Task 4 (DATA_DOCS) opens with a mandatory
precondition checkpoint and carries two verbatim variants of the all-empty-
winning-set sentence (B-CURRENT vs A-OFFICIAL). Task 5 (RULES.md) is
D-29-independent by sentence-level check; `RULES.md:31-32` is left untouched
because any "cannot be leader with no qualifying cards" clause is WP-30's.

Rulings: F34 = documented invariant (callers: `leader_with_suit` <- lib.rs:234,
:325; `Game::leader` <- lib.rs:100, :156, :176 + test :592 — all guarantee a
survivor). F35 = one-line saturating fix, signature unchanged (callers
lib.rs:206, render.rs:135, test :611-613; reachable only via an unvalidated
deserialized `num_players`, which stays WP-09's problem). F33 = NOT
API-breaking: `rg "PubCard|PubSuit"` over the whole repo has exactly one
non-docs hit (the definition), no crate depends on red7-1 as a library, serde
uses the real type names, no TS mirror. No game-version.yaml / rules-version
bump needed — red7's manifest has no rules-version field and its blurb is
accurate.

Newly discovered (routed, not fixed): (1) RULES.md never documents the
empty-hand elimination (lib.rs:146-152) — no package owns it, Lead to file;
(2) red7-1 RULES.md misses several RULES_AUTHORING required sections
(Reading the Display render, worked scoring example, Strategy Tips vs the
separate *_STRATEGY.md files) — Lead ruling + new docs package;
(3) `num_players` trusted in more places than `end_points` -> WP-09.

Compliance: NO file under rust/ was created, modified or deleted (including
red7-1's DATA_DOCS.md and RULES.md — the new text lives in the spec);
NO cargo/build/check/test/clippy/fmt command was run (validation by reading
live source plus the snapshot diff); NO git mutation was run. Writes confined
to planning/specs/WP-29-red7-cleanup.md and this LOG.

---

# specs-LOG - unit 4 RESUME (unit 4b)

## Resume assessment (Lead, 2026-07-25)

The unit-4 Lead died from a session limit while dispatching its WP-54
Worker. What it left on disk:

| WP | Spec file | Bytes | Worker LOG entry | Lead ACCEPTED line | Status entering unit 4b |
|----|-----------|-------|------------------|--------------------|-------------------------|
| WP-41 | specs/WP-41-db-quality-pass.md | 127521 | YES (LOG:767) | **NO** | needs Lead review |
| WP-37 | specs/WP-37-admin-pass.md | 125998 | YES (LOG:843) | **NO** | needs Lead review |
| WP-59 | specs/WP-59-inbound-processing-quality.md | 137453 | YES (LOG:902) | **NO** | needs Lead review |
| WP-29 | specs/WP-29-red7-cleanup.md | 49970 | YES (LOG:976) | **NO** | needs Lead review |
| WP-54 | specs/WP-54-frontend-ux-error-handling.md | 144803 | **NO** | **NO** | needs Lead review + LOG entry reconstruction |

Key finding of the assessment: the unit-4 Lead logged NOTHING itself. Every
one of the four LOG sections above is a raw *Worker* report pasted under a
`## WP-nn ... spec written` heading - none carries the `ACCEPTED` /
`ACCEPTED without revision` line that units 1-3b used to mark a
Lead-reviewed draft (compare LOG:57, :203, :266, :422, :647, :719). The
WP-54 spec file is timestamped 13:34, AFTER the LOG's last write at 13:12,
confirming its Worker finished but its Lead never got to review or log it.
Cross-references inside the Worker reports that assert peer specs are
"finalized" (e.g. the WP-59 entry's "WP-41 is finalized") are Worker
claims, not Lead acceptances, and are NOT treated as such.

Consequence per the unit-4b brief: **nothing is reused as-is.** All five
drafts are treated as unreviewed and go through Lead review with
load-bearing live-source spot-checks before acceptance. No draft is
rewritten from scratch - the review is a gap hunt against the quality bar,
with a revision pass demanded where gaps exist.

## Plan (unit 4b)

Serial review Workers, one per spec, model opus, in backlog order:
WP-41, WP-37, WP-59, WP-29, WP-54. Each Worker (a) re-verifies the spec's
load-bearing claims by READING live source, (b) audits against the quality
bar, (c) applies fixes to the spec in place for gaps it can close with
evidence, (d) reports a verification table plus anything it could not
close. The Lead then spot-checks a sample of each Worker's live-source
citations itself before writing the ACCEPTED line. After the batch, if
budget allows, continue with the next READY packages in backlog order
(WP-51, WP-60, WP-52, ...), to a cap of 5 newly-authored specs.

Hard constraints carried into every Worker brief verbatim: no writes
outside planning/specs/ + planning/raw/ (+ appends to specs-LOG.md and
decisions-needed.md); NEVER modify any file under rust/; NEVER run cargo
or any build/compile/test command (validation by READING source only);
NEVER run git mutations.

## WP-41 db.rs quality pass - adversarial spec review + in-place repairs (Worker, 2026-07-25)

Reviewed the unreviewed WP-41 draft against live source. **Verdict: ACCEPT-AFTER-MY-REPAIRS.**
Full notes: `planning/raw/WP-41-lead-review-notes.md`.

- 64 citations checked, **21 wrong**. All five OVERTURNED/narrowed justifications
  (F44, F51(1), F37's clear-finished_at, F39's ON CONFLICT, F46's retry, F43's Option) were
  re-derived independently and **all hold**.
- Blocking defects found and fixed: (a) the draft claimed 5 "pure" helpers are ungated - all
  five carry `#[cfg(feature = "ssr")]`, and the false claim was about to be written into
  db.rs's module header; (b) `update_finished_at` / `update_is_turn_at` trigger lines were
  001:440-444 / 446-450, actually **001:448-452 / 454-458**, also destined for in-source
  comments; (c) Task 11's bot test could not pass - `migrations/013_bot_efficacy.sql:41-44`
  **seeds** `bots` with easy/medium/hard, so `is_empty()` fails and inserting `'hard'` raises
  23505 (same bug in Test 11); (d) the F35 count is **27** untested fns, not 26 (the list was
  right, the arithmetic wasn't); (e) `is_user_admin` has **20** external callers, not 19, one
  of which (`admin.rs:2201`) is `.await.unwrap()` and would have tripped the draft's own
  STOP-and-report gate.
- Also fixed: ~14 off-by-1-to-3 line anchors (F49 :977, F50 :1802-1807, F44's
  turn_reminder :1934, F47 :3131, F43 `#[cfg]` :56, `NULLS LAST` :3112, Task 3's assert
  :4885-4889, etc.), a mislabelled concede-vs-undo test, the drift hunk narrative, and the
  F42 package count (9 packages declare db.rs, not 8; 6 are decision-blocked, not 5).
- Removed all four placeholder hedges ("if the borrow checker complains, inline it", "if
  DELETE hits an FK, either shape is acceptable", "if Green is not in PLAYER_COLOR_NAMES",
  "read the ORDER BY before asserting") by verifying the underlying facts and rewriting
  Task 8's drift-guard test to use a per-case friend user instead of table deletes.
- Added a verified test-helper signature table to Task 11 and a "the `bots` table is seeded"
  fixture warning.
- Two new cross-package items routed: the 5 `ssr`-gated pure predicates -> **WP-54** (note
  only, nothing broken today); `friends`' two overlapping unique indexes -> **BACKLOG**,
  needs a user decision (migration required).
- **Left for the Lead / user:** `concede_game`'s release-build 3+-player mis-placing stays
  routed to WP-40 (D-3) and untested, per the draft; Task 7's advisory-lock design and
  Task 11's 25-of-27 coverage cut are scope calls left as written.
- Nothing under `rust/` was touched; no cargo/build/test command run; no git mutation.

### Lead ruling on WP-41 (unit 4b)

**ACCEPTED after the review Worker's repair pass.** specs/WP-41-db-quality-pass.md.

Lead independently spot-checked three load-bearing anchors the review Worker
nominated, by reading LIVE source:
- `rust/web/migrations/001_initial_schema.sql:448` = `CREATE OR REPLACE
  TRIGGER update_finished_at` (draft had said 440 - the draft was WRONG, the
  repair is right; 446 is the tail of the preceding `update_updated_at`
  trigger).
- `rust/web/src/db.rs:2909` = `#[cfg(feature = "ssr")]` directly above
  `pub fn can_remove_email` (draft had claimed the pure predicates were
  ungated - draft WRONG, repair right).
- `rust/web/migrations/013_bot_efficacy.sql:41-45` = `INSERT INTO bots ...
  VALUES ('easy',...),('medium',...),('hard',...)` (draft's Task 11 Test 7/11
  assumed an empty `bots` table and could never have passed - draft WRONG,
  repair right).

All three confirmed the reviewer's corrections exactly, which is sufficient
evidence that the review was done by reading source rather than asserted.
64 citations checked, 21 wrong, all repaired in place; the draft's five
OVERTURNED justifications (F44, F51 pt1, F37 clear-finished_at, F39 ON
CONFLICT, F46 retry-on-23505, F43 Option<GameTypeUser>) all survived
independent re-derivation and stand.

Lead-accepted open items (deliberately NOT resolved in the spec):
- `concede_game`'s release-build 3+-player mis-placing (db.rs:1315
  `debug_assert!` only) stays routed to WP-40 / D-3 and untested. Correct.
- Task 7's `pg_advisory_xact_lock` design and Task 11's 25-of-27 coverage cut
  are scope calls; Lead upholds both as written.
- Unverifiable-by-reading: `ALTER TABLE ... DISABLE TRIGGER` inside
  `#[sqlx::test]` and `hashtext` availability on PG18. The spec must (and
  does) leave these to the implementer's first test run.

New cross-package items accepted as routed: 5 `ssr`-gated pure predicates
with sharing-implying doc comments -> WP-54 (note only). `friends`' two
overlapping unique indexes (010:5-6 subsumed by 010:7-9) -> needs a user
decision, added to decisions-needed.md by this Lead at end of unit.

---

## WP-37 admin.rs pass - adversarial review of the unreviewed draft, 2026-07-25

Reviewed by a Worker on behalf of the Lead (the draft was left unreviewed by a
Lead that died mid-session). Full notes: `planning/raw/WP-37-lead-review-notes.md`.
Spec repaired in place: 2255 -> 2355 lines.

**Verdict: ACCEPT-AFTER-MY-REPAIRS.** The draft's reasoning holds - all seven
load-bearing judgements re-derived independently from live source and vendored
crates and every one stands (ws F30 rejection, ws F22 no-model-column + drop the
fallback, ws F24 `input()`-clears mechanism, ws F31 Display-prefix, no unique
index on `bots.display_order`, WP-54 excludes `admin.rs`, WP-41 caller
compatibility). What failed review was citation accuracy and the spec's own test
code.

~95 citations checked, **34 wrong** (nearly all off-by-1-to-9 within the correct
function; no symbol-level location was wrong, and the spec already tells the
implementer to locate by `grep -n "fn <name>"`). All repaired.

**Nine refuted claims, all repaired:** 21 hunks -> **26**; "no finding site
modified by the drift" -> **four were** (`create_bot`, `update_bot`, `list_bots`,
`BotsSection`'s closures), so before-blocks must come from the live file;
"every `admin.rs` site does `.map_err(internal(...))?`" -> **`admin.rs:2201` uses
`.await.unwrap()`** (conclusion survives via `anyhow::Error: Debug`);
**`rust/web/src/game/client.rs` does not exist** (cited twice as the mock-server
pattern - the real one is `tests/ssr_pages.rs:104-128`); `ssr_pages.rs:17-31` is
the import block; cross-package item 2's WP-54 routing is **stale - WP-54 refused
it by LEAD RULING** (`WP-54-...md:210`, :1885) and re-routed it to "its own small
package, or a WP-37 follow-up" after WP-54 Task 1; Task 2's grep step was
self-contradicting; Task 1's post-task grep count was 2, not 1; and
`crypto::load_key` cannot fail on a missing env var (it falls back to
`default_key()`).

**Five defects in the spec's own test/fix code, all repaired:** (1) Task 1's
`include_str!` regression test **self-matched its own literals** (17 vs 16) and
would have failed, and also broke the sibling grep count - now uses `concat!`
needles plus an explicit `assert_eq!(server_fns, 15)`; (2) Task 2's SSR test
**would not compile** - `build_router` is async in this repo; (3) that test's
assertion was **vacuous** (`LocalResource` does not load during SSR, so `/admin`
renders the Suspense fallback for admins and non-admins alike) - relabelled as a
panic smoke test with the redirect moved to a manual check; (4) Task 6's
single-statement reorder was **unsound for a duplicated id** (Postgres picks one
ordinal, unspecified, and `rows_affected` still matches) - explicit rejection +
test added; (5) Task 11's `Router::fallback(any(closure))` passes a `MethodRouter`
where a `Handler` is required - rewritten to the proven `route(path, post(...))`
shape.

**Left open for the Lead, not invented:** the ws F21 STATED ASSUMPTION (verified
*coherent* with the live form and update path, but still a product decision);
cross-package item 2's new home and its sequencing after WP-54 Task 1; and new
cross-package item **3b** (`reorder_bots` accepts a partial bot list and can
produce colliding orders - unreachable from the UI, folded into the same
schema-hardening backlog item as the missing unique index).

Compliance: NO file under `rust/` created, modified or deleted; NO
cargo/build/check/test/clippy/fmt/sqlx command run (all compile-level judgements
reasoned from live source and the vendored reactive_graph / server_fn / reqwest /
axum sources, with a proven in-repo fallback recorded where reading alone could
not settle it); NO git mutation (the only git-adjacent command was a read-only
`diff -u` against the snapshot directory). Writes confined to
`planning/specs/WP-37-admin-pass.md`, `planning/raw/WP-37-lead-review-notes.md`
and this LOG.

### Lead ruling on WP-37 (unit 4b)

**ACCEPTED after the review Worker's repair pass.** specs/WP-37-admin-pass.md.

Lead independently spot-checked, by reading LIVE source:
- `rust/web/src/admin.rs:2201` = `let is_admin = crate::db::is_user_admin(&pool,
  user_id).await.unwrap();` - confirmed. The draft's "all 15/19 sites use
  map_err" wording was WRONG; this site is a `.unwrap()` inside admin.rs's own
  test module, so WP-41's `anyhow::Result` change is still source-compatible
  (`.unwrap()` works on `Result<_, anyhow::Error>` too) but the draft's blanket
  claim needed the correction the reviewer made.
- `rust/web/migrations/013_bot_efficacy.sql:33` = `UNIQUE (bot_id, provider_id,
  model)` as the last entry before `);` - confirms `bot_providers` has NO
  `updated_at` column, so ws F30 is correctly SKIPPED.
- `reactive_graph-0.2.14/src/actions/action.rs:295-296` = `if
  in_flight.get_untracked() == 0 { input.update(|inp| **inp = None); }` -
  confirms the ws F24 re-derivation.
- `rust/web/src/game/client.rs` does NOT exist (the draft cited it twice as a
  mock pattern) - reviewer's refutation confirmed.

~95 citations checked, 34 wrong, all repaired. All seven load-bearing
judgements (ws F22, F24, F30, F31, display_order index, WP-54 exclusion,
WP-41 compatibility) survived independent re-derivation. Five substantive
defects in the draft's own test code were fixed: a self-matching
`include_str!` regression test that would have failed, a non-compiling SSR
test (`build_router` is async, ssr_pages.rs:1184), a VACUOUS redirect
assertion (LocalResource does not load during SSR, so the Suspense fallback
renders for admins and non-admins alike and the test would pass with the
redirect deleted), an unsound `WITH ORDINALITY` reorder for duplicated ids,
and a `Router::fallback(any(closure))` type error.

Lead rulings on the items the reviewer left open:
1. ws F21 blank-API-key semantics ("blank keeps existing"; clearing requires
   an explicit checkbox that wins over typed text). UPHELD as specced and
   escalated: added to decisions-needed.md as a product confirmation, because
   the alternative (blank == clear) silently destroys keys. The spec must
   proceed on the stated assumption; the decision item exists so the user can
   overturn it cheaply.
2. Cross-package item 2 (AdminPage renders raw `ServerFnError` Display text;
   `user_facing_server_error` under-used) - WP-54 declined admin.rs by an
   explicit LEAD RULING at WP-54 spec:210, and WP-37's own finding set does
   not cover general error copy. **Lead ruling: route to WP-38**, the other
   admin.rs package (D-5-blocked, lands after WP-37), as a
   cleanup-if-touched item. Recorded here because this Lead may not write
   work-packages.md.
3. New item 3b (`reorder_bots` accepts a partial list and can produce
   colliding orders; unreachable from the UI) - folded into the same
   schema-hardening backlog item as the missing `bots.display_order` unique
   index. Needs migration 023; added to decisions-needed.md.
4. Item 4 extension (`ApiKeyUpdate::Set("")` has the same empty-key hole as
   `create_provider`) - accepted; one fix covers both, and the spec now says
   so.

## WP-59 inbound processing quality - LEAD REVIEW 2026-07-25

Verdict: **ACCEPT-AFTER-MY-REPAIRS**, with **one item needing a USER DECISION (D-15)**.
Full notes: `planning/raw/WP-59-lead-review-notes.md`.

Verification: ~215 file:line citations checked against live source,
**83 distinct wrong (~39%)** - the worst rate of the three specs reviewed this
session. All repaired in place. Every re-derived judgement (F21 premise
refutation, F23 attempt-cap bypass, F28 no-cursor, F9 AppState rejection, F14
render.rs exclusion, F26 already-solved-in-db.rs) independently re-derived and
**upheld**.

Four defects that would have broken the build or an existing test:
1. Task 4's delete range `:227-241` ate the first two lines of
   `failure_report_header`'s doc comment (which starts at inbound.rs:**240**, not
   :242) -> `:227-239`.
2. Task 13's `bump_reply` body range `:455-476` deleted the function's closing
   brace at :475 without replacing it -> `:455-474`.
3. Task 9 inserted the classifier "after :24", inside `CommandError`'s body
   (the enum closes at :**25**).
4. Task 2's quote-stripping rule 1 ("line followed by a `>` line is an
   attribution") **regresses the existing
   `parse_reply_commands_strips_quoted_lines` test** (inbound.rs:1226-1230).
   Replaced with a block-retraction rule (retract the block since the last blank
   line iff attribution-shaped: ends with `:` or carries `<...@...>`), hand-traced
   against all 9 existing + 7 new tests; `continue`-not-`break` semantics on
   quoted lines preserved.

Plus: Task 11 missed **two** call sites of the private
`set_turn_emails_enabled` it deletes (`commands.rs:1338`, `:1349`, inside
`subscribe_unsubscribe_toggles_turn_emails`) - without them the crate does not
compile; Global Constraints updated to three sanctioned test edits. Task 12's
test rewritten: `make_standalone_ctx_deps()` (`commands.rs:2096`) **does** exist
(Assumption 1 resolved), the placeholders `nats_ctx_for_test()` /
`GameBroadcaster::default()` do not, and `expect_user_err` takes
`Option<Result<..>>` so the draft's test would not compile.

**D-15 / F29 verdict: the collision is REAL, the spec's line number was WRONG.**
`"end"` is intercepted at `commands.rs:`**`1217`** (`:1219` is the `"restart"`
arm - the draft said :1219 four times). `acquire-1/src/command.rs:192-197`
(reachable via `:68`) and `starship-catan-1/src/command.rs:309-313` both expose
`end` as a top-level player move; starship's own test at `lib.rs:2467` asserts
it. Repo-wide grep confirms **`end` is the entire collision set** - no other
reserved verb collides. D-15's recorded basis ("no current collision") is
**refuted**, D-15 is still an open *Recommendation* not a Decision, and Task 14
is now gated "do not execute until the Lead confirms D-15".

Also: F6's trailing-noise skip was mislabelled **SKIPPED-BY-DECISION** - there is
no such entry in `decisions-needed.md`; relabelled **REJECTED-WITH-REASON**.
WP-41 ordering rationale corrected: its top-of-file module doc does **not**
collide (it inserts at db.rs:1); the real collisions are the shared `mod tests`
append and **WP-41 Task 6's rewrite of `delete_expired_unverified_emails`
(:3128-3136), immediately above WP-59's insertion point**. Assumption 2 cleared:
`error.rs` is outside WP-59's declared paths but collision-free (WP-41 only reads
it; WP-37 puts `ADMIN_REQUIRED` in `admin.rs`) - now flagged in the coordination
table rather than absorbed silently. Newly discovered and routed:
`auth/server.rs:884`/`:887` has the identical `cap_digest`-after-`LIMIT`
redundancy and missing cap disclosure (owner: auth package), closing the spec's
previously open cross-package item 5.

Snapshot drift re-confirmed exactly: inbound/notify/render diff empty exit 0;
commands.rs 126; db.rs 606.

Compliance: no file under `rust/` created/modified/deleted; no cargo/build/test
command run; no git mutation run.

### Lead ruling on WP-59 (unit 4b)

**ACCEPTED after the review Worker's repair pass, with Task 14 GATED on a
reopened D-15.** specs/WP-59-inbound-processing-quality.md.

Lead independently spot-checked, by reading LIVE source:
- `rust/web/src/email/commands.rs:1215-1220` = the verb match, with
  `"end" => return run_end(ctx).await,` at **:1217** and `"restart"` at
  :1219. The draft cited :1219 for the `end` arm FOUR times - WRONG; the
  reviewer's correction is right.
- `rust/web/src/email/inbound.rs:240` = `/// Builds the header block for a
  command-failure report email. Layout (each a` - confirms the draft's
  delete range :227-241 would have eaten the first two lines of
  `failure_report_header`'s doc comment. Repair to :227-239 is right.
- `rust/web/src/email/commands.rs:1338` = `set_turn_emails_enabled(&pool,
  user_id, false)` - confirms one of the two call sites Task 11 missed for
  a function it deletes; unrepaired, the crate would not compile.
- `rust/game/acquire-1/src/command.rs:192-197` = `Doc::name_desc("end",
  "trigger the end of the game at the end of your turn", ...)` and
  `rust/game/starship-catan-1/src/command.rs:309-313` = `Doc::name_desc(
  "end", "end the flight early", Token::new("end"))`.

**D-15 VERB COLLISION: CONFIRMED REAL by the Lead's own reading.** `end` is
a live top-level game move in two shipped crates AND is intercepted by the
email dispatcher's own `end` arm at commands.rs:1217 (added post-snapshot by
#47) before the game path at :1264. D-15's recorded basis "no current
collision" is FALSE. Consequences:
- D-15 is REOPENED. Added to decisions-needed.md by this Lead.
- WP-59 Task 14 (the COMMANDS.md "Reserved verbs on the email path" section)
  is specced but carries a hard gate: do not execute until the user
  re-decides D-15. The reviewer added that gate; Lead upholds it.
- This is a live functional defect, not just a docs matter: an acquire or
  starship-catan player cannot issue `end` by email. Routed to the game
  packages and to D-15; NOT silently fixed here.
- No other reserved email verb collides - repo-wide grep over rust/game/
  returns only these `end` hits. Lead accepts that negative result.

~215 citations checked, **83 wrong (~39%)** - the worst hygiene of the three
web-side specs reviewed this unit. Every load-bearing judgement (F21, F23,
F28, F9, F14, F26) survived independent re-derivation and stands. Four
build/test-breaking defects were repaired: the inbound.rs :227-241 range, a
`bump_reply` body range that omitted the closing brace, a classifier
inserted INSIDE the `CommandError` enum (after :24 rather than :25), and
Task 2's attribution rule, which as drafted would have broken the existing
`parse_reply_commands_strips_quoted_lines` test (inbound.rs:1226-1230) while
the spec claimed all 9 pass. Task 12's test was rewritten against the
fixture that actually exists (`make_standalone_ctx_deps()` at :2096).

Refuted draft claims of note: WP-41 does NOT collide textually with WP-59's
db.rs additions (WP-41 inserts at db.rs:1, roughly 3100 lines away), so the
draft's stated landing-order rationale was wrong - the ordering preference
is retained only as a soft merge-hygiene preference, not a hard dependency.
F6's "forgiving trailing noise" was labelled SKIPPED-BY-DECISION but is NOT
a recorded user decision; relabelled REJECTED-WITH-REASON. Lead upholds the
rejection on its stated reasoning but notes it as a Worker judgement call,
not a user ruling.

Lead ruling on scope: WP-59 Task 9 adds `INTERNAL_ERROR_MESSAGE` to
`src/error.rs`, outside the package's declared path list. **Permitted** - the
alternative is a magic-literal comparison, no accepted peer spec touches
error.rs, and the delta is recorded in the spec's coordination table.

New defect accepted as routed: `auth/server.rs:884`/`:887` carries the
identical `cap_digest`-after-`LIMIT` redundancy plus missing cap disclosure
-> auth package. This closes the draft's open cross-package item 5.
`proposals.rs:904` is a 4th `internal(...)` site the Task 9 table omitted
(classification unchanged) - now listed.

---

# specs-LOG - unit 4 RESUME 2 (unit 4c)

## Resume assessment (Lead, 2026-07-25)

Second Lead on this batch died from a session limit while dispatching the
WP-29 review Worker. State found on disk:

| WP | Spec file | Bytes now | Bytes at unit-4b start | Lead ACCEPTED line | Status entering 4c |
|----|-----------|-----------|------------------------|--------------------|--------------------|
| WP-41 | specs/WP-41-db-quality-pass.md | 148645 | 127521 | **YES** (LOG:1114-1116) | REUSE AS-IS, do not re-review |
| WP-37 | specs/WP-37-admin-pass.md | 151803 | 125998 | **YES** (LOG:1217-1219) | REUSE AS-IS, do not re-review |
| WP-59 | specs/WP-59-inbound-processing-quality.md | 156122 | 137453 | **YES** (LOG:1336-1338, Task 14 GATED on reopened D-15) | REUSE AS-IS, do not re-review |
| WP-29 | specs/WP-29-red7-cleanup.md | 62965 | 49970 | **NO** | ORPHAN REPAIRED DRAFT - needs Lead review |
| WP-54 | specs/WP-54-frontend-ux-error-handling.md | 144803 | 144803 | **NO** | UNREVIEWED DRAFT - needs full adversarial review |

Evidence for the WP-29 classification: the spec grew ~13KB and its mtime is
**15:09**, after the LOG's last write at **14:59**. `planning/raw/` holds
`WP-41-lead-review-notes.md` (14:09), `WP-37-...` (14:32), `WP-59-...`
(14:57) but **no WP-29 notes file**. So the WP-29 review Worker ran and
edited the spec in place, but its report and notes were lost with the Lead.
The repaired content is structurally complete (SEQUENCING section,
disposition table, 5 tasks with per-task verification checklists,
cross-package section), but its provenance is unverified. Treated as an
unaccepted draft.

WP-54's mtime is unchanged from unit 4b (13:34), so no review Worker ever
touched it.

## Plan (unit 4c)

1. Lead reviews WP-29 directly (small spec, 467 lines) with load-bearing
   live-source spot-checks via file reads; accept or demand a revision
   Worker. Log immediately.
2. Dispatch one adversarial review Worker (model opus) for WP-54, same
   contract as units 4b's reviewers: re-verify load-bearing claims by
   READING live source, audit against the quality bar, repair in place,
   report a verification table. Lead spot-checks, then logs the ruling.
3. END THE UNIT. Do not start any further packages even if budget remains -
   the Orchestrator spawns a fresh Lead for WP-51/WP-60/WP-52/...

Hard constraints carried into every Worker brief verbatim: no writes outside
planning/specs/ + planning/raw/ (+ appends to specs-LOG.md and
decisions-needed.md); NEVER modify any file under rust/; NEVER run cargo or
any build/compile/test command (validation by READING source only); NEVER
run git mutations.

## WP-29 red7-1 cleanup - LEAD REVIEW (unit 4c, Lead-performed, no Worker)

The orphan draft left by the lost review Worker was reviewed directly by the
Lead (spec is only 467 lines, so a Worker round-trip was not worth the
budget). The draft carries internal evidence of a completed review pass - it
contains self-corrections labelled "CORRECTION to the earlier draft of this
spec (verified live)" (spec :127) and a full disposition table with re-derived
rulings - consistent with a reviewer that finished its in-place repairs and
then died before reporting.

### Lead ruling on WP-29 (unit 4c)

**ACCEPTED without revision.** specs/WP-29-red7-cleanup.md (62965 bytes,
5 tasks, 5 findings dispatched: e F31, e F32, e F33, e F34, e F35).

Load-bearing live-source spot-checks performed by the Lead by READING files
(10 checks, **10 correct, 0 wrong**):

| Spec claim | Live source | Result |
|---|---|---|
| `lib.rs:16` is `pub use card::{Card as PubCard, Suit as PubSuit};` | red7-1/src/lib.rs:16 | EXACT |
| `lib.rs:10` explicit `use crate::card::{Card, Suit, full_deck, leader, points, sort_by_suit, suit_rule};` | :10 | EXACT |
| `end_points` at `:22-24`, body `(50 - players * 5) as u32` | :22-24 | BYTE-EXACT |
| `leader_with_suit` at `:237-252`; eliminated skip `:242-248`; `player_map[l_index]` at `:251` | :237-252 | EXACT |
| `can_play` `:283-285` has `!has_played`; `can_discard` `:287-289` does NOT | :283-289 | EXACT (both are private `fn`, which the spec never mis-states) |
| `discard` draws at `:346-348`, sets `has_played` `:350`, calls `end_turn` `:351` | :346-352 | EXACT |
| `test_end_points` at `:609-614` asserting 40/35/30 | :609-614 | EXACT |
| `card::leader` `:297-317`, empty guard `:298-300`, strict `>` / `i_max > l_max` at `:311` | card.rs:297-317 | EXACT |
| RULES.md `:21-29` and `:48-50` byte-exact replacement targets; `:31-32` untouched; "color" appears only at table lines `:40`,`:42` | RULES.md (56 lines) | ALL EXACT, including the spelling-check assertion |
| DATA_DOCS.md `:31` parenthetical and `:36` final "highest card overall in the palette" line; `:34` anchor | DATA_DOCS.md :23-36 | ALL EXACT, `:36` confirmed final line |
| `rust/Cargo.toml` profiles set no `overflow-checks`/`debug-assertions` (load-bearing: makes Task 2's failing-test step actually fail) | rust/Cargo.toml:46-63 | CONFIRMED - only `debug`, `opt-level`, `inherits`, `lto`, `codegen-units`, `panic` |
| sibling convention `pub use card::*;` at lib.rs:5 with private `mod card;` at :1 | alhambra-1/src/lib.rs:1-6 | EXACT |

Judgements the Lead upholds:
- **e F33's "(or drop the re-export)" REJECTED** - correct: `mod card;` is private
  (`lib.rs:12`) while `Card`/`Suit` appear in `pub` fields of `PubState`/`PlayerState`,
  so dropping the re-export makes those types unnameable. Independently verified.
- **e F34 narrowed to COMMENT ONLY** - upheld. `player_map` cannot be empty on any
  of the four runtime paths, and `leader`/`leader_with_suit` are exactly the functions
  WP-30/D-29 rewrites; an `Option` return here would pre-empt that decision and merely
  relocate the panic into four `.expect()`s.
- **e F35 widened to a real code change (saturating), signature unchanged** - upheld.
  The `saturating_mul`-before-`saturating_sub` ordering point is correct and non-obvious:
  `50usize.saturating_sub(players * 5)` still panics for `players > usize::MAX / 5`.
- **e F32 extended** to name the second game-end condition (`start_round`'s
  `deck.len() < l * 8`, lib.rs:88-91) because the replaced sentence is itself the
  wrong-end-condition sentence. Upheld as in-scope, not scope creep.
- **D-29 gating of Tasks 4-5 with two verbatim variants (B-CURRENT / A-OFFICIAL)** -
  upheld. This is the right shape for a package sequenced after a decision-blocked
  package: Tasks 1-3 are D-29-independent and may land immediately; only the two doc
  tasks are gated.

Prerequisite ordering (stated prominently in the spec, `## SEQUENCING - READ FIRST`,
spec :5-23): WP-29 lands after WP-30, which is BLOCKED-ON-DECISION(D-29, D-40).
The spec correctly establishes that D-40 cannot affect any red7 text (red7's
`Status::Finished` emits `stats: vec![]`) and only keeps WP-30 blocked.

Cross-package items the Lead accepts as routed, plus the three that need
Lead/Orchestrator action beyond this Lead's write scope (planning/BACKLOG.md and
planning/work-packages.md are OUTSIDE the write scope of unit 4c, so these are
recorded here for the next Lead / the Orchestrator to file):
1. **NEW DOCS ITEM (unowned):** red7-1 `RULES.md` never documents the empty-hand
   elimination (`lib.rs:146-152`, which can immediately end the round via `end_turn`
   at `:150`). Out of e F32's scope. No existing package owns red7-1 doc completeness
   (WP-30's paths are `card.rs`/`lib.rs` only, work-packages.md:245).
   **Action needed: file as a new BACKLOG docs item.**
2. **NEW DOCS PACKAGE (unowned):** red7-1's `RULES.md` fails
   `docs/authoring/RULES_AUTHORING.md:13-107`'s required-sections list - missing
   Cards/Components, Rounds/Game End, Winning, Reading the Display (called "critical
   for the bot" at `:44`), Strategy Tips, and Scoring's mandated worked example.
   **Lead ruling:** out of scope for WP-29 (e F32 names only the Turn and Scoring
   sections), and it needs a live render pulled from a real game state, which a
   read-only-derived spec cannot produce. The crate does ship `BASIC_STRATEGY.md` /
   `ADVANCED_STRATEGY.md` surfaced via `Gamer::basic_strategy`/`advanced_strategy`
   (`lib.rs:540-546`); whether that satisfies the Strategy Tips requirement is a
   question for the new package, not for WP-29.
   **Action needed: new docs work package.**
3. `DATA_DOCS.md`'s `discard_pile` entry verified CORRECT against `current_rule()`
   (`lib.rs:254-259`) - recorded so the next reader does not re-flag it. No action.
4. **WP-09 path widening needed:** `end_points` is not the only site that trusts a
   deserialized `num_players` (`render.rs:135`, the `0..self.num_players` loops at
   `lib.rs:207`, `:403`, `:413`, and all per-player vector indexing). Task 2 closes
   only the arithmetic panic. Routed to **WP-09 (BLOCKED-ON-DECISION D-36)**, but
   WP-09's path list (`work-packages.md:90`) does **not** name `game/red7-1`.
   **Action needed: widen WP-09's paths to include `game/red7-1`.**

Also confirmed by the spec and accepted: e F29 is fenced to WP-01 Task 6 (do not
touch `red7-1/src/command.rs`); e F45/e F46 are WP-73's (D-20), and the package
brief's attribution of them to WP-08/WP-33 is WRONG - neither WP-08
(work-packages.md:80-86) nor WP-33 (`:265-269`) owns any red7-1 file. The spec's
correction stands.

Snapshot drift for red7-1: NONE claimed by the spec (`diff -ru` empty, exit 0
against `f8763a5`). Lead did not re-run the diff (no shell execution at Lead
tier); every live line the Lead read matched the spec's cited numbers exactly,
which is consistent with zero drift.

Compliance (Lead, unit 4c so far): no file under `rust/` created/modified/deleted;
no cargo/build/test command run; no git mutation run. All Lead verification was
by reading source files.

**Next: dispatch the WP-54 adversarial review Worker (model opus). WP-54 is the
last item in this unit; after its ruling the unit ENDS.**

## WP-54 frontend UX / error handling - adversarial review of the never-reviewed draft (Worker, 2026-07-25)

Worker verdict: **ACCEPT-AFTER-MY-REPAIRS.** Spec repaired in place, 1923 -> 2051
lines. Full notes: `planning/raw/WP-54-lead-review-notes.md`.

- **89 load-bearing citations checked, 41 wrong (46%)** - the worst hygiene of any
  spec reviewed in units 4b/4c.
- **Two build-breaking delete ranges fixed:** `components/game.rs:307-325` ->
  **`:310-325`** (`:307` is `window_key`'s only statement and `:308` its closing
  brace; the replacement supplied neither); `app.rs:141-172` -> **`:145-173`**
  (the draft's range swallowed `provide_context(current_user);` and orphaned a
  `});`), with the second range `:174-193` -> **`:175-193`**.
- **Non-compiling API removed:** Task 11 called `w.navigator()`; the `Navigator`
  web-sys feature is absent from the entire `--features ssr` graph (checked
  `rust/web/Cargo.toml:77`, `tachys-0.2.18/Cargo.toml:193-312`,
  `leptos-use-0.19.0/Cargo.toml:402,423`, `whoami-1.6.1/Cargo.toml:59-66`).
  Rewritten to read `globalThis.navigator.language` via `js_sys::Reflect`.
- **Wrong function entirely:** the wd F59 row and Task 9 cited
  `get_restart_prefill` at `game/server_fns.rs:1180-1204`; that range is
  `restart_game_with_roster`. Real prefill: `:1321-1332` ->
  `get_restart_prefill_impl` `:1257-1319`, error strings `"Not authenticated"`
  `:1329`, `"Game not found"` `:1265`, `"Game is not finished"` `:1268`,
  `"You are not a player in this game"` `:1275`, `"Game type not found"` `:1287`.
- Task 3's `grep -c action_error_message` gate said 10, actual **8**. `class="error"`
  site count 8/~15 -> **22**. Task 10's "all three anchors are server-rendered" ->
  only two (`app.rs:623` sits inside a false-on-SSR `<Show>`). Systematic anchor
  drift throughout `settings.rs` (up to 14 lines), `app.rs` (4-5), `layout.rs`,
  `friends.rs`, `new_game.rs`, `Cargo.toml`. "Effects inert during SSR -
  `CODING.md:69-153`" is not in CODING.md; it is `docs/hydration.md:80-104`.
- **In three places the FINDINGS were right and the draft's "re-derivation" was
  wrong** (wfe F54's app.rs anchors, wfe F61's `layout.rs:166`, wd F73's
  settings.rs anchors). The spec now says so explicitly so nobody reverts them.

### Lead ruling on WP-54 (unit 4c)

**ACCEPTED after the review Worker's repair pass, with a Lead scope ruling on
cross-package #7 (new D-41, ruled B).** specs/WP-54-frontend-ux-error-handling.md.

Lead independently spot-checked, by reading LIVE source (4 checks, 4 confirming
the reviewer and refuting the draft):
- `rust/web/src/components/game.rs:305-308` = the `window_key` comment, signature
  and its single statement `dt.assume_utc().unix_timestamp() / 600` at `:307`,
  closing brace `:308`, `format_log_time` starting `:310`. The draft's `:307`
  start would have destroyed `window_key`. Reviewer correct.
- `rust/web/src/app.rs:143` = `provide_context(current_user);` exactly, with the
  `current_user` `LocalResource` at `:138-142` and the profile-theme `Effect`
  closing at `:173`. Also confirms `applied_profile_theme` at `:154` (draft said
  `:150`) and `presence_started` at `:179` (draft said `:174`). Reviewer correct
  on all four.
- `rust/web/src/game/server_fns.rs:1178-1180` is a `get_current_user` /
  `"Not authenticated"` block inside a DIFFERENT function, and
  `get_restart_prefill_impl` is at `:1257` with `"Game not found"` at `:1265`.
  Confirms the draft cited the wrong function. Reviewer correct.
- `rust/web/src/email/commands.rs:1217` = `"end" => return run_end(ctx).await,`
  (re-confirmed independently this unit, see the D-15 item below).

**OVERTURNED justification the reviewer refuted, and the Lead upholds the
refutation:** wd F57's inherited "re-sync the selects from the refetched overview
on failure" cannot work. Three independent library-level reasons, each traced to
crate source: a rejected mutation returns identical data and
`AttributeValue for bool::rebuild` skips equal values; `AnyView::rebuild` rebuilds
**in place** on a matching `TypeId` so the `<select>` is never re-created (which
refutes the draft's own cross-package #6); and `<option selected>` will not
reassign a user-dirtied option. The failure-arm `set_refresh` bumps were removed,
wd F58 was narrowed to "CONFIRMED as an inconsistency, impact ~nil" (still fixed,
for cache truthfulness rather than re-sync), and manual checklist step 7 now
asserts the residual desync as EXPECTED so it is never baked in as a regression.
This is the right handling: the defect is recorded, not silently absorbed.

**Task 8 (wfe F58) - a fix that did not fix its finding, now repaired.** The draft
proposed mounting the `<select>` only once `bot_names` settled. The reviewer proved
from library source that `prop:value` never applies on first build at all:
`HtmlElement::build` runs `attributes.build` (`tachys-0.2.18/src/html/element/mod.rs:352`)
BEFORE `children.build` (`:357`), and a reactive `prop:` goes through
`RenderEffect::new`, documented as "immediately runs `fun`"
(`reactive_graph-0.2.14/src/effect/render_effect.rs:61-62`). So the value is set on
an option-less element and the browser falls back to the first option - and a
later-mounted select is still built attributes-first, so the gate alone changes
nothing. Task 8 now keeps the settled gate AND applies the value from an `Effect`
over `NodeRef::<leptos::html::Select>` (Effects run after the render pass), with
`leptos::html::Select` and `HtmlSelectElement`'s feature-enablement both verified.
The old manual step ("Before the fix: the select shows `medium`") was false and is
rewritten. **Lead assessment: this is the single most valuable catch of the unit -
the draft would have shipped a no-op fix with a test narrative asserting it worked.**

Justifications that HELD under independent re-derivation: wd F73's Theme
non-revert (strengthened with `app.rs:255-264` + cookie `:265-271`), wd F66's
rejection of the union option, wfe F54's dual-latch strategy (missing premise
supplied: `app.rs:511-517` refetches `current_user` on login with no page load),
wfe F55's helper choice, wfe F61's `<button>` rejection, Task 1's `_` arm. No
"SKIPPED-BY-DECISION" verdicts existed in this spec, so none needed relabelling.

Four placeholder hedges closed by verifying the underlying fact: `&e` vs `e` in
Task 8's rider (it is `&e`, proven by the sibling arm at `opponent_slot.rs:110`/`:116`);
Task 9's `gt_counts` borrow (compiles - `gt` moves at `:394`, 124 lines later);
Task 11's Navigator fallback (determined unavailable, code rewritten); Task 5's
"if the assertion fails, delete it" (replaced with a reproduce-first protocol plus
a STOP-and-report gate).

**WP-41-routed item was MISSING and is now present.** The unit-4b WP-41 ruling
routed "the 5 `ssr`-gated pure predicates -> WP-54 (note only)" but the WP-54
draft never recorded it. Added as cross-package #10 with all six `db.rs` anchors
re-verified live (`:2001/:2002`, `:2909/:2910`, `:2916/:2917`, `:2923/:2924`,
`:2938/:2939`; `validate_username:849` ungated), the server-side caller list, the
WP-41 routing sentence quoted, and "Action for the implementer: none. Do not open
`db.rs`."

**D-15 gate: WP-54 is NOT gated.** Verified: WP-54 adds no email verb, no
dispatcher arm and no email copy; its only `end`-adjacent touches are the UI label
`"End game failed: "` and the existing `EndGame` action dispatch. An explicit
Non-Goals bullet now says so. D-16 continues to gate WP-55, not this package.

Snapshot-vs-live drift, re-verified by the reviewer with read-only `diff -ru`:
`friends.rs`, `new_game.rs`, `settings.rs`, `app.rs`, `layout.rs`,
`opponent_slot.rs`, `components/mod.rs`, `error.rs`, `tests/ssr_pages.rs`,
`style/main.scss` all IDENTICAL. Only `components/game.rs` drifted, 660 -> 681
lines, from **one** commit (`1f665b0`), not five as the draft claimed; and the
`Place:`/`Form:`/`FormStrip` block at `:269-277` is NOT drift (byte-identical in
the snapshot). Two hunks the draft missed were added (`is_2player` -> `can_concede`
at `:122`; `profile_link=player.user_id.is_some()` at `:253`).

New defects accepted as routed:
- **#7** friends-page select stays visually desynced after a rejected change. No
  owner but WP-54. **-> new D-41** (see below).
- **#8** `RestartPrefill::player_counts` is computed and serialised but never read
  -> **WP-53**.
- **#9** nothing guarantees the hard-coded default `bot_name: "medium"` appears in
  `get_available_bots`' list -> **WP-53** (moot today:
  `migrations/013_bot_efficacy.sql:41-45` seeds easy/medium/hard).

### decisions-needed.md writes made by this Lead (unit 4c)

1. **D-15 REOPEN AMENDMENT WRITTEN - the unit-4b Lead's claimed write had never
   landed.** LOG:1363 says "D-15 is REOPENED. Added to decisions-needed.md by this
   Lead", but `decisions-needed.md:226-233` still read "Recommendation: A now (no
   current collision)" with no amendment - that Lead died before the edit. The
   amendment is now in place: the two colliding game crates
   (`acquire-1/src/command.rs:192-197`, `starship-catan-1/src/command.rs:309-313`),
   the intercepting dispatcher arm (`email/commands.rs:1217`, before the game path
   at `:1264`), the statement that "no current collision" is FALSE, the note that
   option A can no longer be adopted as written, and the hard gate on WP-59 Task 14.
   **Lesson for the next Lead: verify a dead predecessor's claimed writes, do not
   trust the LOG's assertion that a file was edited.**
2. **D-41 ADDED** (friends-page select revert after a rejected change), with the
   library-level proof that the obvious fix cannot work, options A (absorb into
   WP-54 Task 2) / B (error message only, route the binding conversion to
   `friends.rs`'s owner WP-53), and recommendation B. **Lead ruling applied so
   WP-54 is not blocked: B.** WP-54 ships the error message, records the residual
   as expected, and routes the conversion. Overriding to A touches only WP-54
   Task 2; nothing else depends on it.

## Completion (unit 4c) - UNIT ENDS HERE

Batch WP-41 / WP-37 / WP-59 / WP-29 / WP-54 is COMPLETE. All five specs are
Lead-accepted and final:

1. `specs/WP-41-db-quality-pass.md` - accepted unit 4b, REUSED AS-IS by 4c
2. `specs/WP-37-admin-pass.md` - accepted unit 4b, REUSED AS-IS by 4c
3. `specs/WP-59-inbound-processing-quality.md` - accepted unit 4b, REUSED AS-IS
   by 4c. **Task 14 HARD-GATED on the reopened D-15.**
4. `specs/WP-29-red7-cleanup.md` - orphan repaired draft, LEAD-REVIEWED and
   accepted in unit 4c (10/10 spot-checks correct). **Sequenced after WP-30,
   which is BLOCKED-ON-DECISION(D-29, D-40); Tasks 1-3 are D-29-independent and
   may land immediately, Tasks 4-5 are gated with two verbatim variants.**
5. `specs/WP-54-frontend-ux-error-handling.md` - unreviewed draft, review Worker
   + Lead ruling in unit 4c, accepted after repairs. **New D-41 ruled B.**

Running total: **22 finalized specs** (17 from units 1-3b + these 5).

Per the unit-4c brief the Lead STOPS here rather than starting further packages,
so the Orchestrator can spawn a fresh Lead with a clean budget.

**Remaining READY packages for the next Lead, in backlog order: WP-51, WP-60,
WP-52, then the rest of `planning/BACKLOG.md`'s READY list.** The next Lead should
re-read BACKLOG.md itself rather than trusting this list beyond WP-51/60/52.

Known carry-forward items for the next Lead:
- Three actions this unit could NOT take (planning/BACKLOG.md and
  planning/work-packages.md are outside unit 4c's write scope): (a) file a new
  BACKLOG docs item for red7-1 RULES.md's undocumented empty-hand elimination;
  (b) file a new docs work package for red7-1 RULES.md's `RULES_AUTHORING.md`
  non-compliance (needs a live render, so a read-only spec cannot produce it);
  (c) widen WP-09's path list (`work-packages.md:90`) to include `game/red7-1`.
- WP-54's repairs shifted its line numbers, so WP-37's cross-references to
  `WP-54:210` (from `WP-37:45`/`:2349`) and `WP-54:1885-1886` (from `WP-37:2347`)
  are now ~19 and ~115 lines off. The quoted text is intact, so nothing is
  substantively stale - fix opportunistically, do not re-review WP-37 for it.
- Pre-existing WP-37 anchor error, NOT repaired: `WP-37:45` cites
  `components/game.rs:550-562` for `GameCommandInput`'s error slot; live it is
  `:583-591`.
- Citation error rates observed this unit: WP-41 33%, WP-37 (see 4b), WP-59 39%,
  WP-54 46%, WP-29 0%. Budget a full adversarial review Worker for every
  web-side spec; the game-crate specs have been far cleaner.

Compliance (unit 4c, Lead + its one Worker): **no file under `rust/` was created,
modified or deleted; no cargo/build/check/test/clippy/fmt or any other
build command was run; no git mutation was run.** All validation was by reading
source. Files written this unit: `specs/WP-54-frontend-ux-error-handling.md`
(in-place repairs), `raw/WP-54-lead-review-notes.md`, appends to `specs-LOG.md`,
and two edits to `decisions-needed.md` (D-15 amendment, D-41 added).

---

## Unit 4d (Lead 5) - housekeeping + WP-51 / WP-60 / WP-52

Brief: (A) three carry-forward housekeeping items the unit-4c Lead could not do,
(B) specs for the next 3 READY packages in backlog order (WP-51, WP-60, WP-52),
then STOP. Workers on opus per user override. Adversarial citation review Worker
mandatory per spec (all three are web-side).

### A. Housekeeping - DONE 2026-07-25

1. **WP-09 paths widened.** `work-packages.md` WP-09 paths line now names
   `red7-1` (crate count `~12` -> `~13`), plus an "ADDED at spec time" note
   recording the exact red7-1 sites (`render.rs:135`, `lib.rs:207`/`:403`/`:413`
   `0..self.num_players` loops), that it carries no finding ID, and that WP-29
   Task 2 must not be widened for it.
2. **Two docs packages filed: WP-74 and WP-75.**
   - New `work-packages.md` section "Documentation packages filed at spec time"
     (before `## Coverage check`).
   - WP-74 red7-1 empty-hand-elimination rules documentation - READY. Sourced
     from `specs/WP-29-red7-cleanup.md:464`.
   - WP-75 red7-1 RULES.md `RULES_AUTHORING.md` compliance - READY. Sourced from
     `specs/WP-29-red7-cleanup.md:465`. Recorded with TWO explicit blockers to
     spec-writing that a future Lead must not miss: it needs a LIVE render
     capture (recipe `RULES_AUTHORING.md:56-64`, needs DB + built binary) and a
     RULING on whether the shipped `BASIC_STRATEGY.md`/`ADVANCED_STRATEGY.md`
     satisfy the mandatory Strategy Tips section. It is READY (no D-item gates
     it) but NOT spec-writable from source reading alone - do not hand it to a
     read-only spec Worker.
   - Both sequenced after WP-29 Task 5 and after WP-30 (BLOCKED-ON-DECISION
     D-29/D-40), since all of them rewrite `red7-1/RULES.md`.
   - **Coverage check kept consistent:** added a paragraph stating WP-74/WP-75
     are absent from the per-package finding-count table BY DESIGN (0 finding
     IDs each, so the 570 sum, the per-unit checks and the exactly-one-package
     invariant are unaffected), and a convention for future spec-time packages.
     Package totals line updated: **75 packages - 41 READY, 34
     BLOCKED-ON-DECISION** (was 73 - 39 - 34).
   - `BACKLOG.md`: new **Phase 7 - documentation follow-ups (lowest priority)**
     with items 74 (WP-74) and 75 (WP-75). Position numbers 74/75 continue
     Phase 6's numbering with no renumbering of any existing item.
3. **Predecessor's claimed writes VERIFIED PRESENT - both real this time.**
   - D-15 reopen amendment: `decisions-needed.md:235-253`, opening
     "**REOPENED 2026-07-25 ... The recorded basis 'no current collision' is
     FALSE.**", carrying both colliding crates
     (`acquire-1/src/command.rs:192-197`, `starship-catan-1/src/command.rs:309-313`),
     the intercepting arm `email/commands.rs:1217` vs the game path `:1264`, and
     the WP-59 Task 14 hard gate. Matches the 4c LOG claim exactly.
   - D-41: `decisions-needed.md:553-576`, "D-41. Friends-page select revert after
     a rejected change [informs WP-54, WP-53]", with the cannot-work proof,
     options A/B, recommendation B and the applied Lead ruling B. Matches.
   - Conclusion: unit 4c's LOG claims are accurate; the missing-write problem was
     specific to the unit-4b Lead.

### B. Specs - starting WP-51

Confirmed against `BACKLOG.md` directly (not just the 4c LOG): Phase 5 items
47/48/49 are WP-51 invite-mailer/notify dedup, WP-60 outbound tokens/metrics/
render, WP-52 stats/query perf pass, all READY. Order matches the brief.

### WP-51 author Worker returned (unit 4d)

Draft at `specs/WP-51-invite-mailer-notify-dedup.md`, 1310 lines, 7 tasks.
NOT YET ACCEPTED - adversarial citation review pending.

Author's self-reported verdicts: 10 CONFIRMED, 0 REJECTED, 0 NEEDS-DECISION, no
task decision-gated. Recommendations it claims to have overturned: wd F8's "log a
warn" (says `execute_command` already loads the identical snapshot at
`game/mod.rs:90-92`, so return it instead); wd F33's "no-reply address" (says a
bare no-reply would route stray replies into From-authenticated settings commands
via `inbound.rs:37-51` + `:484-486`; keeps an `i-` prefix); wd F34's "skip the
send"; wfe F41's "skip"; wfe F43's "spawn the send loop"; and BOTH of wfe F42's
offered options plus its cited call site. Claims wfe F32/F36 findings were right
where adopted and says so in the spec. Claims one clause of wfe F36's evidence
("the two gating copies have already drifted") is FALSE and that the real drift
is wfe F32/D-11, WP-46's.

Drift reported: proposals.rs / sweep.rs / notify.rs / outbound.rs / render.rs
empty diff; `game/mod.rs` +1 line; `game/server_fns.rs` and `email/commands.rs`
large #47 drift.

**Two large new-defect claims that need independent verification before the Lead
accepts anything (they assert whole missing code paths, not line errors):**
1. email-originated game moves never call `notify_game_emails`
   (`commands.rs:1264-1281`, claimed zero hits in `inbound.rs`);
2. no web game-start path notifies either (`proposals.rs:1118`, `:1360`,
   `inbound.rs:791`).
If true these are more severe than anything in WP-51's scope and need routing;
if false they must not enter the spec. Handed to the review Worker as priority 1.

---

## Unit: CRITICAL PATH extraction (Lead, 2026-07-25) - `planning/critical-path.md`

Purpose: the user stopped broad spec-writing to drive the CRITICAL items to
implementation first. This unit produced **`planning/critical-path.md`** - a lean
extraction/condensation of the existing findings and planning docs. **No new
specs, no code verification, no source changes.**

Method: two serial extraction Workers (model opus per user override), Lead
synthesis. Worker 1 swept `findings/` + `findings/verification/` for the 10
criticals and the security/data-corruption/liveness majors. Worker 2 swept
`planning/` (work-packages.md, decisions-needed.md, BACKLOG.md, specs-LOG.md,
`ls specs/`) to map every finding to its owning WP, confirm spec existence by
listing the directory, and condense the gating decisions. Neither Worker wrote
any file; neither read `rust/` source beyond `ls` to confirm three path prefixes.

Result: **48 critical-path findings (10 critical + 38 major) across 21 work
packages.**

- **Ready to implement now (11):** WP-44, WP-01, WP-14, WP-25, WP-36, WP-39,
  WP-15, WP-21, WP-06, WP-03, WP-13. Plus predecessors WP-41 (before WP-40 and
  WP-47), WP-37 (before WP-38) and WP-59 (before WP-57 and WP-40), all already
  Lead-accepted. Hard order: **WP-01 before WP-03**.
- **Spec-gapped but unblocked (1):** WP-34 (medium) - specc-able immediately.
- **Decision-blocked (10 packages / 10 decisions):** WP-56 (D-1), WP-40 (D-3,
  D-4 rides along), WP-38 (D-5), WP-45 (D-8), WP-47 (D-6 + D-13), WP-57 (D-2),
  WP-35 (D-12 + D-14), WP-10 (D-33, after D-35), WP-09 (D-36), WP-02 (D-37).
- Spec-writing left on the critical path: 4 large (WP-40, WP-38, WP-09, WP-02),
  4 medium (WP-56, WP-35, WP-34, WP-57), 3 small (WP-47, WP-45, WP-10).

Where the criticals live: 7 of the 10 are already covered by finalized specs
(WP-01 x4+lg F4, WP-14, WP-25) and are handable today. The remaining 3 sit in
two unspecced, decision-blocked packages - **WP-56 (wfe F1, wfe F17 - account
takeover via the forgeable inbound `From` header, D-1)** and **WP-40 (wd F14 -
permanent ratings corruption via undo, D-3)**. Those two decisions are the
critical path's real gate.

`WP-46` (sweep delivery, wfe F31) was deliberately excluded from the path -
duplicate outbound mail, not loss or takeover - but it shares D-2 with WP-57 so
the same answer unblocks it. `WP-02` carries only `ls F2` on the path inside a
large 10-finding markup package; noted as separable if D-37 stalls. `WP-16`
appeared in the mapping pass only as a mis-candidate for `c F29` (which belongs
to WP-21); it holds none of the 48 and is off the path.

Corrections recorded while extracting (no docs edited): `ws F1` is written
`critical` in `findings/web-server.md` but was ADJUSTED to major in
`findings/verification/web-server.md`, which is why REVIEW.md's tally is 10
criticals and this document lists it as a major. Units 10-13 findings docs do
not number their findings; the `wd`/`wfe`/`bo` F-numbers used in critical-path.md
come from `planning/raw/w5-webdomain-email.md` and `w6-botops-deps.md` and are
authoritative only there. Units 10-13 have no per-finding verification verdicts,
only in-session lead verification - recorded verbatim rather than invented.

Compliance: NO file under `rust/` was created, modified or deleted; NO
cargo/build/check/test/clippy/fmt or any other build command was run; NO git
mutation was run. All work was reading `docs/reviews/2026-07-23-rust-review/`.
Writes confined to `planning/critical-path.md` and this LOG entry.

---

## 2026-07-25 - Lead unit: record 10 decision answers + spec the critical-only gap packages

### Step A - decisions recorded (Lead, no Worker)

All 10 gating decision groups ANSWERED by the user and recorded
authoritatively BEFORE any spec work, so a session death cannot lose them.

`planning/decisions-needed.md`: added an "ANSWERED - 2026-07-25 session"
summary table near the top (10 rows, deviations flagged) plus an in-place
**ANSWERED** block on each of D-1, D-2, D-3, D-4, D-5, D-6, D-8, D-12, D-13,
D-14, D-33, D-35, D-36, D-37.

Two answers DEVIATE from the recommendation and are quoted verbatim in their
blocks:
- **D-5**: bots stay referenced BY NAME (no bot-id migration). Dangling bot
  player names are an explicitly SUPPORTED state - they no-op rather than wedge,
  and the admin page warns listing dangling names. "Disabling all bots must
  remain a valid intentional configuration."
- **D-12+D-14 (iv)**: **NO session expiry** - the user explicitly does not want
  sessions to expire. revoke-all-sessions stays in scope; the compensating
  control is that **changing an account email requires re-verification**
  (step-up confirmation to the new address), which per D-1 lives in the web UI.

`planning/work-packages.md`: status legend gained a decision-session note;
11 packages flipped to READY (WP-02, WP-09, WP-10, WP-35, WP-38, WP-40, WP-45,
WP-47, WP-49, WP-56, WP-57); WP-42 -> READY-PENDING-CONFIRMATION; WP-46 ->
BLOCKED(D-11) only; WP-11 -> BLOCKED(D-30) only; totals block updated
(52 READY / 23 BLOCKED).

Conflicts reconciled while recording:
1. **D-8 vs D-5.** D-8 option C validates bot names at 4 entry points AND at
   game start, but D-5 makes a later-missing name a supported no-op. Recorded
   rule: **validate on write, tolerate on read** - names are validated at
   creation/start for immediate feedback; a name that disappears or is disabled
   LATER must not wedge or reject, it takes D-5's no-op + admin-warning path.
   Both the WP-45 and WP-38 specs must state this explicitly.
2. **D-13 label ambiguity - FLAGGED, needs user confirmation.** The answer
   "D-6 + D-13: Option A" gates activity feeds, but D-13's literal option A is
   *accept the /ws firehose as-is*. Recorded per its stated substance (feeds
   gated with the same `is_game_visible_to_user` predicate, i.e. D-13 B/C
   shape), with an explicit FLAG; WP-42 is READY-PENDING-CONFIRMATION rather
   than READY. WP-47 (the package this unit's decisions actually gate) is
   unaffected.
3. **D-35 is broader than the question asked.** The answer arrived inside the
   D-33/WP-10 group but D-35 is by construction the global port-parity policy,
   and "official rules are authoritative; correct both the code and RULES.md" is
   *stronger than* option A and rejects option C's documented-in-crate-wins
   precedent that several already-triaged parity items were reasoned under.
   Recorded as the global default with a scope caveat: D-26..D-32 and D-34 must
   be re-read under policy A before their packages are specced.

### Step A verification - WP-34 has NO critical

Confirmed by reading `findings/verification/web-server.md` row F1: `ws F1`
(concurrent confirm requests race the per-code attempt cap) is written
`critical` in `findings/web-server.md` but was **ADJUSTED to major** by
verification ("bounded race-window multiplier over the cap of 10, not
effectively unbounded"). `work-packages.md` WP-34 severity line reads
`1M/4m/4n` - zero criticals. **WP-34 is therefore OUT of this unit's scope**
and no spec was written for it; it stays a READY, unspecced medium package for
a later security-major spec unit.

Critical-only gap packages for this unit are therefore exactly **WP-56**
(wfe F1 + wfe F17) and **WP-40** (wd F14).

### New leaner spec format in force from this unit

Replaces the previous heavyweight format. No exhaustive line citations (33-46%
of web-side spec citations verified WRONG, and the double-verification was the
budget sink); code identified by file path + function/type name; line numbers
only as hints marked "approximate, verify". No adversarial citation-review
Worker - instead each spec instructs the implementer to locate and read the
named functions first and to STOP and report on any mismatch. Target 150-400
lines. Mandatory sections for a critical package: root cause (not symptom),
"constraint going forward" as a reviewable rule, proposed docs updates
(docs/CODING.md and/or relevant docs/*.md), a complete solution (no partial
mitigations), a concrete regression-test plan (inputs, expected outcomes, test
locations, commands), and explicit non-goals.

Read-only compliance for step A: writes confined to `planning/decisions-needed.md`,
`planning/work-packages.md` and this LOG. No file under `rust/` touched, no
cargo/build/test command run, no git mutation.

### Step B1 - WP-56 spec (Worker return + Lead acceptance)

`planning/specs/WP-56-email-from-auth-redesign.md` - **500 lines**. ACCEPTED.

Covers wfe F1 + wfe F17 (both criticals) and wfe F5. Six tasks: (1) per-user
secret settings token with a new `users.settings_email_token` column and
migration `023`, (2) consult Resend's inbound SPF/DKIM verdict, (3) remove the
unrouted-`None` fallthrough, (4) delete the account-security commands, (5)
plumbing checklist, (6) the forward constraint into `docs/CODING.md`. All seven
mandatory sections present (root cause, complete solution, constraint going
forward, doc updates, regression-test plan, non-goals, finding-recommendation
audit).

Lead spot-checks against source (all CONFIRMED by reading, no exhaustive pass):
- The `s-` settings address is `format!("s-{user_id}@brdg.me")` in `inbound.rs`
  and `parse_reply_address` merely strips the `s-` prefix - the "token" is a
  guessable identifier, never a secret. This is the spec's stated root cause and
  it is correct.
- `ResendInboundData` has exactly four fields (`email_id`, `from`, `to`,
  `received_for`) - there is genuinely no SPF/DKIM verdict deserialized today.
  The spec correctly REFUSES to invent a payload field name and instead gives a
  verify-then-implement procedure plus a STOP-and-report gate. Good behaviour
  under the new format; do not "fix" this by guessing a field name later.
- `dispatch_email_command` falls through to `dispatch_settings_command`, so
  settings verbs ARE reachable on the `g-` game-token path. This makes the
  finding's own recommendation (patch the standalone settings dispatcher) a
  PROVEN-WRONG recommendation - the shared `run_settings_emails` is the correct
  edit point. Recorded in the spec's section 7; do not revert.
- Highest existing migration is `022_concede_bot_replacement.sql`, so `023` is
  the right number.
- Web replacements (`add_email_address`, `confirm_email_address`,
  `make_email_address_active`, `remove_email_address` in `auth/server.rs`, wired
  from `settings.rs`) exist, so Task 4 is not a functional regression.
- Citation density is 13 numeric refs in 500 lines, every one marked
  "approximate, verify" - format compliant.

Accepted deviations, and Lead rulings on the two items the Worker escalated:
1. **Length 500 > the 400 target.** Accepted. Three compression passes were
   applied; the overage is four solution parts plus a verbatim doc block, not
   padding. Still a 60% reduction on the old 1000-1300-line format.
2. **`emails remove` is deleted too** - the user's verbatim answer enumerates
   "add/confirm/activate". **LEAD RULING: keep the deletion.** `emails remove`
   mutates the same account-security state (dropping an address redirects mail),
   the answer says remove account-security commands "entirely", and leaving one
   arm would be exactly the partial mitigation this package forbids. The spec
   documents a one-arm revert path if the user disagrees. **Surfaced to the
   Orchestrator for user confirmation.**
3. **Cold-start gap, NOT solved in this spec and NOT to be invented by the
   implementer:** once `s-` is secret, a user with no games and no prior
   settings email has no discoverable settings address. Escalated to the
   Orchestrator/user as a product question; the spec builds no UI for it.
4. Declared paths extended beyond work-packages.md's `email/{inbound,commands}.rs`
   to include `email/outbound.rs` (the `ensure_email_token` pattern), a new
   migration, and `docs/CODING.md`. Flagged in-spec rather than absorbed
   silently - correct.

New cross-spec conflict discovered (record in the landing order): **WP-59 Task
10 (wfe F23) and the `emails confirm` half of WP-59 Task 9 (wfe F24) fix
commands WP-56 DELETES.** If WP-56 lands first, those WP-59 tasks become no-ops
and must be dropped rather than re-adding the commands.

Correction to an earlier LOG claim: specs-LOG's "no AppState test fixture exists
for the email handlers" is narrower than it reads - `tests/ssr_pages.rs::make_state`
and `tests/websocket_hygiene.rs` do build a real `AppState`. The genuine blockers
are svix signature construction plus `resend_webhook` building `ResendInbound`
inline instead of accepting the injectable `InboundEmailSource`. The spec's test
plan therefore pushes logic into pure / pool-only functions. This also softens
the WP-57 coverage warning.

### Step B2 - WP-40 spec (Worker return + Lead acceptance)

`planning/specs/WP-40-undo-concede-toctou-ratings-integrity.md` - **477 lines**.
ACCEPTED.

Covers all 8 findings (wd F14 critical, wd F15, wd F16, wfe F19, wfe F20,
wfe F22, ws F34, ws F38). Six tasks: (1) a `claim_unfinished_game_tx` claim
helper next to `StaleStateConflict`, (2) guard the three lifecycle writers,
(3) `undo_core`/`concede_core` in `game/server_fns.rs`, (4) collapse the email
copies, (5) arm the rating idempotency guard (ws F38), (6) `concede_game`'s
2-player assumption (routed in from WP-41). All seven mandatory sections
present; only 2 numeric refs in 477 lines.

Lead spot-checks against source (all CONFIRMED by reading):
- Move-path guard precedent is real: `db::update_game_command_success` with
  `WHERE id = $4 AND updated_at = $5`, returning `db::StaleStateConflict` (a
  `thiserror` unit struct) on 0 rows, surfaced as
  `ExecuteCommandError::Conflict`. Copying this is the right call.
- `*_core` precedent is real: `game::server_fns::restart_core` (`pub(crate)`),
  already consumed by both `restart_game_with_roster` and
  `email::commands::run_restart` - so `undo_core`/`concede_core` follow an
  established in-repo shape rather than inventing one.
- `db::concede_game`, `db::concede_game_replace`, `db::undo_game` and
  `db::apply_rating_changes` all exist as named; `email/commands.rs` has
  `run_concede`, `run_undo`, `run_restart` as named.
- Existing `#[sqlx::test]` db-level tests named in the plan exist
  (`undo_game_restores_state_and_clears_undo`,
  `concede_game_replace_swaps_in_bot`, the `update_game_command_success_*`
  family), so the regression plan lands in a real test module.

Accepted deviations:
1. **`db::concede_game_replace` added to the guarded set.** It post-dates the
   review snapshot (commit series #47), is the DEFAULT concede branch when a
   replacement bot exists, and writes no `games` row - so wfe F19 / wd F16 are
   only half-closed without it. This is also why the guard is a
   `SELECT ... FOR UPDATE` claim helper rather than a WHERE clause alone (the
   WHERE-clause form is still specified belt-and-braces on the two functions
   that do UPDATE `games`). Good catch; accepted.
2. **New `db::GameAlreadyFinished` error alongside `StaleStateConflict`**, so
   "already finished" and "someone moved first" get distinguishable user text on
   both the web and email surfaces. Accepted.
3. `ActingPlayer` enum in the core signature instead of the finding's
   pre-resolved game_player, avoiding a second `find_game_extended` on the web
   path. Accepted.
4. 477 lines vs the 400 target - overrun is the audit table plus verbatim doc
   text. Accepted.

Stale-note supersession recorded in the spec, as instructed: **work-packages.md's
WP-40 note "rewind via stored deltas" PREDATES D-3 and is SUPERSEDED.** D-3
option A forbids undo on a finished game, so there is no rewind and none is to be
written. ws F34's own "recompute on next finish" recommendation remains
KNOWN-UNSOUND (double-counts) and is doubly moot. The spec's non-goals open with
"NO rating rewind or recompute of any kind ... This is the single most important
line in the spec."

Live-state notes from the Worker, worth carrying forward:
- **WP-41 has NOT landed** (live `db.rs` still carries the manual
  `updated_at = NOW()` clauses), so WP-40's "if WP-41 has not landed, STOP"
  branch is currently the live one. WP-41-before-WP-40 remains a hard ordering
  requirement.
- WP-59 dependency confirmed concretely: WP-59 adds `classify_server_fn_error`
  and `crate::error::INTERNAL_ERROR_MESSAGE` and owns the single `run_restart`
  `map_err` line.
- Undo/concede errors are currently INVISIBLE in the web UI (the `GameMeta`
  effects match `Some(Ok(()))` only). That is WP-54 Task 1's work; WP-40 states
  it rather than fixing it. Without WP-54, the new conflict errors will be
  silent on the web surface - flag to the Orchestrator when sequencing.

### Unit close-out

Specs written this unit: 2 (WP-56 500 lines, WP-40 477 lines). WP-34 skipped -
verified to contain NO critical (ws F1 was ADJUSTED critical->major).

Remaining critical-path spec gaps, all security/data-corruption/liveness MAJORS,
all now decision-unblocked and awaiting a later spec unit:
WP-38 (large, D-5 MODIFIED - must state the D-8 reconciliation), WP-09 (large,
D-36 A), WP-02 (large, D-37 A + the stored-content risk assessment step),
WP-35 (medium, D-12+D-14 MODIFIED - no session expiry, email-change
re-verification), WP-34 (medium, never blocked), WP-57 (medium, D-2 A),
WP-47 (small, D-6 A), WP-45 (small, D-8 C + D-5 reconciliation), WP-10 (small,
D-33 A + D-35 official-rules-win). Off-path but D-2-adjacent: WP-46 (still
BLOCKED on D-11).

Read-only compliance for the whole unit: writes confined to
`planning/decisions-needed.md`, `planning/work-packages.md`,
`planning/specs/WP-56-email-from-auth-redesign.md`,
`planning/specs/WP-40-undo-concede-toctou-ratings-integrity.md` and this LOG.
No file under `rust/` created, modified or deleted by the Lead or either Worker;
no cargo/build/check/test/clippy/fmt command run; no git mutation.

## 2026-07-25 - Lead unit: fold in four user refinements (D-1 narrowing, D-13 /ws, parity park, landing order)

Fold-in / investigation / bookkeeping unit. **No new specs written** and no
unrelated code re-verified, per the brief. Two Workers, run serially, both on
Opus 5 per an explicit user override of the skill's Worker-model table.

### Refinement 1 + 2 - D-1 scope narrowed, `emails remove` confirmed

The user does **not** want all settings commands off the email interface. Only
account-security verbs leave: `emails add`, `emails confirm`,
`emails active`/`use`, `emails remove` (the last **confirmed yes**). **Username
(`name`), `theme`, `colors`/`colours`, bare `emails`, `emails on`/`off`,
`emails invite on|off`, `emails reminder on|off` and `settings` are KEPT** - the
user ruled they are not sensitive. The `s-` token + SPF/DKIM + `None`-fallthrough
work (WP-56 Tasks 1-3) is **unchanged**; it now exists to protect the retained
commands.

Cold start **RESOLVED**, no longer an open question: settings are managed in the
web UI, which surfaces the tokenised inbound settings address as an **opt-in
reveal**. The token must **never** appear in an email footer (a bearer secret in a
footer leaks with every forward). Building the reveal is not WP-56's.

Edits to `planning/specs/WP-56-email-from-auth-redesign.md`:
- Added a **REFINED** block under the binding-decision quote.
- Task 1's "Cold-start consequence - report, do not solve" bullet **replaced** with
  the resolved approach plus the explicit no-footer prohibition.
- Task 4 gained a **scope table** naming every verb and its DELETE/KEEP fate, and
  a sentence forbidding narrowing Tasks 1-3.
- The "One deviation ... `remove` is a one-arm revert" paragraph **withdrawn** and
  replaced with a confirmation. The "Explicitly retained" line expanded from just
  `name` to the full KEEP list.
- `docs/CODING.md` boilerplate in section 4 reworded so it does not read as a
  blanket ban on settings over email - the boundary is drawn at *credentials*.
- Non-goals gained: no web-UI reveal here; no removal of the KEEP-list verbs.
- Regression table gained `retained_settings_verbs_still_work` - a guard so a
  future agent does not "finish the job" by deleting the kept verbs.

Also recorded in `decisions-needed.md` as a **REFINED** block under D-1.

### Refinement 3 - D-13 `/ws`, with the user's direct technical question answered

Worker 1 read `rust/web/src/{websocket.rs,websocket_client.rs,router.rs,state.rs,
app.rs,nats.rs}`, `auth/{session.rs,server.rs}` and
`tests/websocket_hygiene.rs`. Findings, all source-verified:

- **(a) `/ws` is fully anonymous.** `ws_handler` (websocket.rs:82-87) is five lines
  taking only `WebSocketUpgrade` and `State<GameBroadcaster>` - no session, no
  cookie, no token, no `get_current_user`. `tests/websocket_hygiene.rs:71-81`
  *asserts* a cookie-less connect gets 101. **But identity is available:** `/ws` is
  registered at router.rs:142 **before** `.layer(session_layer)` at :155, so
  tower-sessions already wraps the route and a `Session` extractor would resolve.
  The one thing that could have blocked authentication is already correct.
- **(b) A single unfiltered site-wide firehose.** Every socket subscribes to the
  NATS **wildcards** `game.>` and `proposal.>` (websocket.rs:112, :119) and
  forwards payloads verbatim. Payloads are skinny JSON (UUIDs only - no state, no
  names), so the leak is bounded to *existence and timing* of every move and
  proposal event site-wide. No `user.>` subject exists; websocket.rs:200,228
  actively assert it stays empty. Filtering is entirely client-side
  (websocket_client.rs:37-85, app.rs:842-856), and the global
  `trigger.last_update` counter is bumped on **every** frame while keying
  `active_games` (app.rs:129-133) and `public_index` (app.rs:294-299) - so every
  site-wide event forces a server-fn refetch on every connected client. **That is a
  load bug independent of the privacy question.**
- **(c) No `sub`/`unsub` protocol exists, server or client.** The server polls
  inbound frames for pong/close but discards the payload (websocket.rs:165-172,
  comment: "we don't act on client-sent data here"). The client never binds the
  `send` handle (websocket_client.rs:51-55). **No vestige of the previous
  brdg.me's `sub`/`unsub` survives in `rust/`** - a repo-wide grep finds only
  `tracing-subscriber`, the two NATS `client.subscribe` calls, email footer
  strings and a migration comment; `rust/web/public/` has no legacy JS. So the
  user's recollection is correct about the *old* system but the protocol would be
  **new work**, not a restoration.

**Answer to the user's question:** yes, user-specific events can ride on websocket
authentication - authenticating the upgrade is small and the stack already supports
it - **but identity alone filters nothing today**, because the NATS subject scheme
carries no user dimension. Recommended design recorded under D-13: (1)
authenticate the upgrade with `Session` + `State<PgPool>`, resolving identity
**before** `ws.on_upgrade` (the connection is hijacked after, and the session
layer's response-side save pass has already run), using
`auth::session::get_user_from_session` + `validate_session_token`, **not**
`get_current_user` (a `#[server]` fn whose leptos context does not cover the plain
`/ws` route); (2) **never 401 an anonymous upgrade** - degrade to a public-only
stream, since logged-out visitors need `public_index` and the hygiene test asserts
101; (3) filter with a wildcard subscribe + per-socket membership set rather than
per-user fan-out subjects, which would force all eleven publish sites to learn the
recipient set; (4) `sub`/`unsub` is then needed **only** for public-game pages -
and the user's stated intent ("only send public-game events to a client that
actually has that public game's page open") **requires the real protocol**; a
single always-on "public games" subject would not satisfy it.

D-13's **label-ambiguity FLAG is resolved** and the item is now recorded ANSWERED
(option B shape). `work-packages.md`: **WP-42 flips
READY-PENDING-CONFIRMATION -> READY** with F59 split into Task A (auth + filter)
and Task B (`sub`/`unsub`, separable, must not block Task A); **WP-47** gains a
note that WP-42 must reuse its `is_game_visible_to_user` rather than forking it,
and that WP-47 should ideally land first. **Neither spec was written this unit**,
per the brief.

### Refinement 4 - game rules parity PARKED

The user's ruling, two parts: (1) official rules are authoritative and docs may be
corrected, but **no gameplay change without per-game sign-off**; (2) the **whole
question is parked** pending the user's own review of the game rules, because some
`RULES.md` content was **AI-generated and may be wrong** (so it is not a
trustworthy baseline for "code vs docs" adjudication) and because
**edition/variation choices are the user's to make**.

Applied:
- `decisions-needed.md` gained a **PARKED-PENDING-USER-RULES-REVIEW banner** at the
  top of Group D, and per-item markers on the headings of D-26, D-27, D-28, D-29,
  D-30, D-31, D-32, D-34 and D-35. D-35's ANSWERED block gained a park paragraph
  noting that the "docs may be corrected" half is **also** suspended for these
  items.
- **D-33 is deliberately NOT parked** (pub_state redaction, independent of rules
  parity) and **WP-10 stays READY** - its heading previously read "D-33 + D-35
  answered", which was the one collision with a global D-35 park; the heading now
  says so explicitly so no agent re-blocks it.
- `work-packages.md`: new **BLOCKED-ON-USER-RULES-REVIEW** status added to the
  legend, described as *stronger* than BLOCKED-ON-DECISION (clears only on the
  user's per-game sign-off, not on a decision answer). Applied to **WP-11, WP-12,
  WP-16, WP-20, WP-26, WP-30**, each with a note on what is parked and what is not.
  WP-20 and WP-30 keep their separate BLOCKED-ON-DECISION(D-40) tag, which is a
  stats question, not a rules question.
- **Unaffected, stated explicitly so nothing is wrongly assumed stuck:** WP-25
  (modern-art-2 d F34 critical infinite busy-loop + d F35 round-4 soft-lock - its
  own note already said it does not wait on WP-26's D items), WP-15
  (seven-wonders-1 b F1/F2/F3, incl. the reachable permanent DrawDiscard
  soft-lock), WP-10, WP-19, WP-22, WP-23, WP-29.

**Egregious-fix candidates - FLAGGED to the user only, NOT unparked, NOT specced.**
Worker 2 triaged all of D-26..D-32/D-34 against "is this really an edition
question". Five survived as plain bugs:

| Finding | Crate | Why it is not an edition choice |
|---|---|---|
| **a F1** | roll-through-the-ages-2 | `roll()` re-matches `self.phase` after `keep_skulls()` may have advanced it; an all-skull reroll cascades into `next_turn()` so the **previous** player's `roll()` decrements the **next** player's `remaining_rolls`. Cross-player state corruption, and the crate's own `test_game_keep_skulls_all_disaster_leadership` asserts the opposite for the `next`-command path. |
| **b F4** | seven-wonders-1 | `execute_actions` resolves p0..pn against live-mutated state, so p+1 trades for goods p built the same turn. **Asymmetric by player index**; every edition is simultaneous-symmetric. |
| **b F7** | seven-wonders-1 | `cities()` lists all 14 A/B entries and `start_game` takes the first `players`, so Rhodes A and Rhodes B can both be in play. Physically unreachable - 7 boards in every printing. |
| **e F30** (seat-order half only) | red7-1 | With all palettes empty, counts tie at 0 and `rank_key` ties at `(0,0)`, so the strict `>` at card.rs:311 leaves the **first non-eliminated** player as leader and the lowest-index player may discard into a rule nobody satisfies. **Tie-break by seat order** is in no edition. The "can an empty set win" half genuinely IS D-29 and stays parked. |
| **d F37** | modern-art-2 | `end_round` initialises `highest_count = -1`, so zero-card artists are awarded 2nd ($20) / 3rd ($10) whenever fewer than three artists had paintings played, and the values enter `value_board` and inflate later rounds. **Honest caveat recorded in the doc:** `modern_art.go:389-403` is identical, so a strict parity framing can claim it - which is exactly why it needs the user's word, not an agent's. |

Explicitly **not** egregious (left parked): D-28 splendor tie-break, all of D-30's
player caps, all three D-31 acquire items (the findings name later-Hasbro vs
classic 3M/AH editions explicitly), D-32 jaipur (premise uncorroborated),
D-27 F5/F6/F8, and **all eight** WP-11 items.

### Sequencing facts - new `planning/landing-order.md`

All three prior-Lead claims verified by reading:
- **WP-41 has not landed and WP-40 depends on it - VERIFIED.** `rust/web/src/db.rs`
  still carries the `updated_at = NOW()` clauses WP-41 Task 1 deletes; no module
  doc header; `git log --oneline -40` contains zero `WP-*` commits. WP-40's spec
  already says "If WP-41 has NOT landed: stop and say so."
- **WP-59 vs WP-56 - the conflict is SMALLER than reported.** Rechecked under
  refinement 1. Task 10 (wfe F23, minor) is fully dead. Task 9 **survives ~3/4
  intact**: the `error.rs` constant, `classify_server_fn_error` (which **WP-40
  consumes**) and the `run_restart` `map_err` at ~:1113 - the wfe F21 **major** -
  all target surviving code; only the `run_emails_confirm` `map_err` (wfe F24,
  minor) dies. Also 2 of Task 11's 6 inline-SQL sites die. **Net: two minor
  findings become no-ops, nothing major is lost.** Either order remains fine.
- **WP-40's new conflict errors are invisible until WP-54 - VERIFIED.** `GameMeta`'s
  five `ServerAction` effects in `components/game.rs` (~:58-85) match
  `Some(Ok(()))` and drop `Some(Err(_))`; a failed mutation produces no WS bump, so
  no refetch, so the only signal is absence of change. WP-54 Task 1's shared
  `RwSignal<Option<String>>` + `crate::error::action_error_message` is what surfaces
  them. No hard ordering - WP-40 may land first and simply ships mute on the web
  surface. Email is unaffected (it goes through `CommandError`).

`landing-order.md` also records the full email-settings verb inventory by exact
verb name, a recommended order for the cluster (WP-41 -> WP-40 -> WP-54; WP-56 and
WP-59 either order; WP-47 before WP-42), and a line-number caveat.

### Deviation from the brief - `docs/BACKLOG.md` NOT written

The brief asked for the parity park to be noted in `docs/BACKLOG.md`, but its own
HARD READ-ONLY CONSTRAINT restricts writes to
`docs/reviews/2026-07-23-rust-review/planning/`. The Lead **did not write outside
that directory** and instead recorded the exact ready-to-apply patch in
`planning/BACKLOG-note-proposed.md`, flagged to the Orchestrator. Verified while
preparing it: the 2026-07-23 review is **not referenced in `docs/BACKLOG.md` at
all** (only the unrelated, already-archived 2026-07-04 pass), so the note creates a
new anchor; highest ID in use anywhere is #52, so the new item is **#53**.
`docs/BACKLOG.md` is currently modified in the working tree - re-read before
applying.

### Unit close-out

No new specs written (correct - this was a fold-in unit). One spec amended
(WP-56), two planning files amended (`decisions-needed.md`, `work-packages.md`),
two created (`landing-order.md`, `BACKLOG-note-proposed.md`).

Read-only compliance for the whole unit: writes confined to
`planning/decisions-needed.md`, `planning/work-packages.md`,
`planning/specs/WP-56-email-from-auth-redesign.md`,
`planning/landing-order.md`, `planning/BACKLOG-note-proposed.md` and this LOG.
Both Workers wrote **no files at all** - they reported in their final messages
only. No file under `rust/` created, modified or deleted by the Lead or either
Worker; no cargo/build/check/test/clippy/fmt command run; no git mutation (shell
use was limited to read-only `ls`/`grep`/`sed`/`head`/`tail`/`wc`,
`git log --oneline`, `git diff --stat` and `git diff`).
