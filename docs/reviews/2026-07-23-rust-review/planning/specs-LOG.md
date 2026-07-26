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

---

## 2026-07-25 - Unit: Tier 2 / Tier 3 execution-plan SURVEY (Lead)

Deliverable: `planning/tier2-tier3-plan.md`. Read-only unit - no spec writing,
no source edits, no cargo/git mutation. Writes confined to `planning/`.

### Lead pre-work (from planning docs only, no source read)

Derived the candidate Tier 2 roster from `work-packages.md` severity tallies
cross-checked against `critical-path.md` and the `planning/specs/` directory
listing (25 spec files exist: WP-01, 03, 06, 07, 13, 14, 15, 19, 21, 22, 23, 25,
28, 29, 36, 37, 39, 40, 41, 44, 51, 54, 56, 59 **and WP-68**, which the
Orchestrator brief omitted).

Candidate Tier 2 = has >=1 major, no spec, not BLOCKED-ON-USER-RULES-REVIEW:
- READY (13): WP-02, 08, 09, 10, 34, 35, 38, 45, 47, 49, 57, 62, 63
- BLOCKED-ON-DECISION (8, list-and-skip): WP-04 (D-38), 05 (D-39), 46 (D-11),
  55 (D-16), 58 (D-10), 64 (D-19), 66 (D-17), 67 (D-18)

Arithmetic finding: the brief's "Tier 3 = ~480 findings" over-counts. 257m + 225n
= 482 total, but 99m + 96n are already inside the 25 finalized specs and 30m + 11n
sit in the six BLOCKED-ON-USER-RULES-REVIEW packages. Remaining unspecced,
unparked = **~130m + ~118n = ~248**. Recorded in the plan.

Worker 1 dispatched: extract post-verification major finding IDs for the 11
packages whose majors are not enumerated in `critical-path.md`, and cross-check
the no-major (Tier 3) classification against `findings/verification/`.

### Worker 1 returned - major-ID extraction + severity cross-check (read-only, wrote nothing)

Majors for the 11 packages whose majors are not listed in `critical-path.md`
(all counts reconcile with the `Severity:` tallies in `work-packages.md`):
WP-02 = ls F2 + ls F3; WP-04 = lg F7; WP-05 = ls F12; WP-08 = e F1;
WP-46 = wd F28 + wfe F30 + wfe F31; WP-49 = wd F67; WP-62 = bo F18;
WP-63 = bo F26; WP-64 = dp F1; WP-66 = dp F6; WP-67 = dp F12.

Cross-check of the 21 zero-major (Tier 3) packages: **zero majors confirmed in
all 21**; no verification UPGRADE touches them (the only upgrade in the whole
review is d F4 minor->major, already inside the specced WP-22). No REJECTED
finding sits in any of them.

REJECTED sweep across all 9 verification files: exactly **two** - games-batch-d
F13 and web-server F30 - both already excluded from package scope. No package
shrinks.

New hazards surfaced by Worker 1 (carried into the plan's gotchas):
1. **lib-support finding numbering diverges.** Raw `findings/lib-support.md` has
   46 findings; `findings/verification/lib-support.md` has 45 - raw F10 (ANSI
   escaping) is absent from verification, so every raw ls number >= 10 is +1
   against verification. `work-packages.md` uses **verification numbering**
   (only reconciles that way for WP-02 and WP-05). Resolving an `ls F10+` ID
   against the raw file reads the WRONG finding.
2. `dependencies.md` states 26 findings but contains 27 headings;
   `bot-operator-tools.md` states 30 but contains 31. Sequential numbering is
   sound (anchored by dp F6/F12/F20, bo F18/F25/F26/F28).
3. ws F67 (WP-43) is UNVERIFIABLE, not rejected - needs network to check.

### Worker 2 returned - cross-package absorption audit of the 25 finalized specs

Key result: **no unspecced package is more than incidentally covered**; the
finalized specs mostly *fence* the unspecced packages out rather than doing
their work. Three real partials only:
- **WP-38**: WP-39 already shipped the visibility half of ws F56 plus the
  supervised restart loop. WP-38's residue is ws F27, wd F1/F2/F3/F5, bo F2 -
  "what gets acked, when, with which AckKind". Do NOT re-do WP-39's work.
  WP-36's spec text is STALE where it attributes consumer supervision to WP-38.
- **WP-65**: WP-22 Task 5 already removed lords-of-vegas-1's lazy_static.
- **WP-72**: WP-03 Task 8 deletes `combine` from `lib/game/Cargo.toml`, leaving
  only the `lib/markup` half of dp F15 (inference, not stated in-spec).

Work ROUTED IN to unspecced packages by LEAD RULING in finalized specs (these
widen scope beyond `work-packages.md` and must appear in the Tier 2/3 rosters):
WP-09 gains acquire-1 (WP-19:838) and sushizock-2 (WP-21:1079) to its crate list
plus the `Gamer::player_state` totality gap; WP-10 gains starship-catan-1's
`peeking` JSON exposure (WP-13); WP-62 gains `upsert_game_type_and_version`
last-writer-wins on `game_types.player_counts` (WP-28:727); WP-35 gains the web
email-change/re-verification UI (WP-56 removed the email path) and two WP-59
riders; WP-04 gains two items from WP-03:1315/1317; WP-08 gains acquire-1,
starship-catan-1 and lost-cities double-placings sites; WP-43, WP-50, WP-52,
WP-53, WP-57, WP-58, WP-60, WP-64, WP-69, WP-70, WP-71 all gain riders.

Two items have **NO owner** and need the Orchestrator/user to file them:
1. **Email-originated game moves never call `notify_game_emails`** - other
   players get no turn email (major functional gap). WP-51 explicitly forbids
   folding it into WP-59 or WP-40 and proposes a new spec-time package.
2. `get_available_bots` default `bot_name: "medium"` not guaranteed
   (WP-54:2007, "If no package owns it, no owner - Lead to file").
Plus the **db.rs module split (ws F42)** is a deferred future package that must
land after WP-35/40/45/47/49/50/52/53/59.
Plus **D-15 is reopened** by WP-59 (email `end` verb collides with acquire-1's
and starship-catan-1's top-level `end` move).

### Unit close-out - `planning/tier2-tier3-plan.md` written

Contents: (0) scope arithmetic + one plan decision; (1) Tier 2 roster - 13
dispatchable + 8 decision-blocked packages, partial-coverage table, routed-in
scope, unowned items, 6 batches T2-B1..T2-B6; (2) Tier 3 roster - 16
dispatchable packages in 8 batches T3-B1..T3-B8 + 7 decision-blocked; (3)
prevention-package inputs (5 named root causes + 2 optional, each with its
source spec/finding); (4) execution order; (5) 13 gotchas carried forward.

**Plan decision recorded (flag to the user if disputed):** a Tier 2 spec covers
its WHOLE package (majors in detail + the package's own minor/nit riders as an
appendix), and Tier 3 covers only zero-major packages. The brief's split would
have cut 20 of 21 Tier 2 packages in half and put two sessions in one file.

Deliverables planned for future batches: Tier 2 -> `planning/specs/WP-nn-*.md`
(~1 page each); Tier 3 -> `planning/checklists/` (one crate checklist per
batch); prevention -> `planning/CODING-amendment-proposed.md` (a PROPOSAL,
since writes outside `planning/` are forbidden - precedent
`planning/BACKLOG-note-proposed.md`).

Read-only compliance for the whole unit: the Lead wrote only
`planning/tier2-tier3-plan.md` and this LOG. Both Workers wrote **no files at
all** and reported in their final messages only. No file under `rust/` was
read, created, modified or deleted by the Lead or either Worker; no
cargo/build/check/test/clippy/fmt command run; no git mutation. Shell use was
limited to read-only `wc`, `tail` and appends to this LOG.

---

## Unit: WP-34 + WP-35 Tier 2 specs (Worker, 2026-07-25)

Written (only these two files, plus this LOG append):

- `planning/specs/WP-34-auth-races-session-mechanical.md` (~155 lines) -
  ws F1 (major, atomic `UPDATE ... RETURNING *` with `attempts >` NOT `>=`),
  F3 `cycle_id`, F5 propagate DB error, F6 windowed cap via a NEW
  append-only `login_email_sends` table + migration `023` (reset-on-rotation
  is explicitly rejected as under-counting), F10 propagate the cap refusal,
  F12/F13/F14/F15 nits.
- `planning/specs/WP-35-auth-edge-semantics-fail-open.md` (~165 lines) -
  ws F2 (claim-stealing in `confirm_login_inner`), F16 (fail-closed
  `load_key` + eager startup validation), F4/F7/F8/F11, plus the three
  routed-in items.

**Landing order asserted in WP-35:** WP-41 -> WP-36 -> WP-34 -> WP-35.
WP-36 is a NEW constraint this unit found: WP-36 Task 4 changes
`crypto::load_key` to return `Zeroizing<[u8; 32]>`, and WP-35 rewrites the
same function's fallback semantics. WP-34 before WP-35 because both edit
`verify_turnstile_token` and `request_confirmation_code`.

**Findings rejected / reclassified (with evidence):**

1. **ws F4's uniform-rejection alternative - REJECTED** (already flagged by
   the verification file and settled by D-14 (ii) = A). Kept as a
   comment-only change plus a test pinning the asymmetry.
2. **ws F11's expiry/GC half - REJECTED** by D-14 (iv). Only revoke-all is
   specced; the spec instructs that the db.rs test asserting a 40-day token
   still validates must stay green.
3. **ws F6 option 1 (reset `sent_count` on rotation) - REJECTED** as
   under-counting. Also re-derived: the PER-EMAIL cap is NOT affected,
   because `request_confirmation_code` guards it with `code_valid &&`, so it
   never spans a rotation. Only the global sum is wrong.
4. **ws F1's recommendation carries an off-by-one** - `>` not `>=`. A
   dedicated test (`attempts = 9` correct code -> Ok; `attempts = 10` ->
   Err) is specced to pin it.

**Routed-in item (a) re-derived DOWN - the Lead should note this.** The web
email-change + re-verification flow **already exists in live code** and is
already compliant: `add_email_address` (unverified insert + mailed code),
`confirm_email_address`, `make_email_address_active` (refuses with
`SetPrimaryOutcome::Unverified`), the 24h expiry sweep
(`email/sweep.rs::spawn_unverified_email_sweep` +
`db::delete_expired_unverified_emails`, `UNVERIFIED_EMAIL_EXPIRY`), and the
UI in `settings.rs::EmailSection`. WP-56 removed only the email-side verbs.
So (a) collapses to: a regression test pinning the
verify-before-activate invariant, plus ws F2 so a squatted address cannot
block the true owner. **Open question for the Lead/user:** D-14's wording
says "confirmation link"; the live flow uses a 6-digit code. The specs treat
link-vs-code as cosmetic and mark it a non-goal. If the user wants an actual
link, that is new UI work and needs its own package.

**Also found:** `db::delete_login_confirmation` does NOT exist today and
WP-41 does NOT add it (grepped). WP-35 therefore specs adding
`delete_login_confirmation` + `delete_login_confirmation_tx` (two variants -
one call site is inside a transaction, one on the pool) and
`invalidate_all_auth_tokens` to `db.rs`, which is why WP-41 must land first.

Read-only compliance: nothing under `rust/` was written; no cargo/git
mutation run. All validation was by reading live source at
`/home/beefsack/Development/brdgme/rust/web/src/{auth/server.rs,
auth/session.rs,crypto.rs,main.rs,db.rs,settings.rs,email/sweep.rs}`.

---

## 2026-07-25 - Tier 2 batch T2-B1 Lead - WP-34, WP-35 ACCEPTED

Worker 1 (opus) returned two specs; Lead sanity-checked shape and accepted
both without a second verification pass (per the no-adversarial-pass rule).

- `specs/WP-34-auth-races-session-mechanical.md` (166 lines)
- `specs/WP-35-auth-edge-semantics-fail-open.md` (215 lines)

Both exceed the ~120-line target. Lead judged the excess to be content
density (six mandatory sections + rider table + per-finding test cases), not
padding, and accepted rather than spend budget re-cutting. Future Worker
briefs in this unit reiterate the cap harder.

Substantive outcomes the Orchestrator should carry forward:

1. **New landing-order constraint discovered: WP-36 must precede WP-35.**
   WP-36 Task 4 changes `crypto::load_key` to return `Zeroizing<[u8; 32]>`
   while WP-35 rewrites the same function's fallback semantics. Full asserted
   order: **WP-41 -> WP-36 -> WP-34 -> WP-35**. `planning/landing-order.md`
   does not yet record this.
2. **Routed-in item (a) re-derives DOWN.** The web email-change +
   re-verification flow already exists and is compliant in live code
   (`add_email_address` -> `confirm_email_address` ->
   `make_email_address_active` refusing unverified, plus the 24h unverified
   sweep and `settings.rs::EmailSection`). It collapses to a regression test
   plus ws F2. **Open question for the user:** D-14 says "confirmation link"
   but the live flow uses a 6-digit code. Spec marks link-vs-code cosmetic and
   a NON-GOAL; if the user actually wants a link it needs its own package.
3. `db::delete_login_confirmation` **does not exist** and WP-41 does not add
   it. WP-35 therefore specs adding it, a `_tx` variant, and
   `invalidate_all_auth_tokens`.
4. Rejections recorded with evidence: ws F4's uniform-rejection alternative
   (locks out verified users on blocked domains; D-14(ii)=A); ws F11's
   expiry/GC half (D-14(iv) forbids session expiry - revoke-all only, and the
   db.rs 40-day-token test must stay green); ws F6 option 1 (under-counts).
   ws F1's `>=` off-by-one is pinned by a dedicated test.
5. Worker re-derivation: the per-email send cap is NOT affected by the
   rotation issue, because `request_confirmation_code` guards it with
   `code_valid &&` so it never spans a rotation.

Read-only compliance: writes confined to `planning/specs/` and this LOG.
Nothing under `rust/` read-modified; no cargo/git commands.

## 2026-07-25 - Worker: WP-47 + WP-45 specs (Tier 2)

Drafted `specs/WP-47-game-visibility-gates.md` (120 lines) and
`specs/WP-45-bot-slot-validation.md` (119 lines). Both under the 120-line cap.
Sources: decisions-needed.md D-5/D-6/D-8/D-13 blocks only (grepped, not read
whole); findings/web-domain.md #17/#27/#45 and findings/web-frontend-email.md
#18 (neither file has a verification file - lead-verified raw findings are all
there is); live source under `rust/web/src/`; WP-41 spec grepped for
`is_game_visible_to_user` only.

Findings verified against LIVE source, all three upheld:
- **wd F17 correct as written.** `get_game_details` uses the viewer's
  `game_players` row only to pick a render perspective; `None` falls through to
  the full spectator render. `get_game_logs` in the same file *does* hard-reject
  non-players, which is the contrast that proves the omission.
- **wd F45 correct as written.** `get_player_game_type_stats` and
  `get_player_history` never call `get_current_user`; `opponents_by_game` and
  `head_to_head` in `stats/queries.rs` select `users.id`/`users.name` with no
  `game_visibility` clause.
- **wd F27 / wfe F18 correct as written.** Three web entry points pass
  client `BotSlot` straight to insert; `classify_opponent`'s
  `strip_prefix("bot:")` branch returns before the `bot_names` check that
  guards bare tokens. Nothing rejected.

`is_game_visible_to_user` reconciliation: the LIVE tree has exactly ONE
definition (`db.rs`, `(pool, game_id, viewer_id) -> Result<bool>`), plus
callers in `friend_recent_visible_game` and two tests. WP-41 Task 8 has NOT
landed yet - its "second copy" is an inlined SQL copy inside
`friend_recent_visible_game` (with cross-reference comment + drift-guard test),
not a second Rust fn. WP-47's spec therefore forbids a third encoding: it adds
only *callers* plus a no-new-SQL dispatcher
`is_game_visible_to_viewer(pool, game_id, Option<Uuid>)`. The canonical
signature already satisfies WP-42 (PgPool + Uuid, no leptos, no
`get_current_user`) so the spec forbids changing it.

Design call needing Lead awareness: D-6 says "anonymize private users" in
stats, which is a **per-user** rule, whereas `is_game_visible_to_user` is
**per-game**. The spec adds ONE derived batched helper
`db::visible_user_ids(pool, &[Uuid], Option<Uuid>)` whose WHERE clause is the
per-player clause lifted from the canonical predicate, with a drift-guard test
asserting the two agree (for a viewer who is not a player of the game). A
per-game gate would have meant excluding games from aggregates, which D-6
explicitly forbids.

Two consequential ripples the spec mandates: `stats::HeadToHead::user_id` must
become `Option<Uuid>` (masked rows get `None` + "Anonymous"), and
`players.rs`'s head-to-head table must render plain text instead of an `<A>`
when `user_id` is `None` - so WP-47 touches `players.rs`, which is not in its
work-packages.md path list.

WP-45: D-8 option C reads as five checks, not four - the four write entry
points plus `start_proposal_tx`. `restart_core` covers both
`restart_game_with_roster` and the email restart path, so one check there
serves two callers. Shared validator specced as
`db::validate_bot_slots(executor, &[BotSlot]) -> Result<Option<String>>`,
mirroring the existing `roster_error` Option-message shape; must be generic
over executor because `start_proposal_tx` holds only a `&mut PgConnection`.
Case-insensitive match required (the email path lowercases its token).

Open question for the Lead: restart rebuilds bot slots from a finished game's
*persisted* bots, so under "validate on write" a restart is rejected when a bot
has since been disabled. I specced that as intended creation-time feedback per
D-8, but it is the one place where write-validation touches previously-stored
data and may deserve a confirmation.

Read-only compliance: writes confined to `planning/specs/` and this LOG.
Nothing under `rust/` modified; no cargo/git/build commands run.

---

## 2026-07-25 - Tier 2 batch T2-B2 (part 1) Lead - WP-47, WP-45 ACCEPTED

Worker 2 (opus) returned two specs, both inside the 120-line cap. Lead
sanity-checked shape and accepted; no second verification pass.

- `specs/WP-47-game-visibility-gates.md` (120 lines)
- `specs/WP-45-bot-slot-validation.md` (119 lines)

Substantive outcomes:

1. **The WP-41 "second copy" of `is_game_visible_to_user` is a false alarm.**
   Live tree has exactly ONE Rust definition, in `rust/web/src/db.rs`, with
   signature `(&PgPool, Uuid, Uuid) -> Result<bool>`. WP-41 Task 8 has not
   landed; what the plan called a second copy is an inlined SQL copy inside
   `friend_recent_visible_game`, not a second function. WP-47's spec forbids a
   third encoding, adds callers plus a no-new-SQL dispatcher
   `is_game_visible_to_viewer(pool, game_id, Option<Uuid>)`, and FREEZES the
   canonical signature so WP-42 can call it with only a pool and a UUID.
2. **No finding rejected.** wd F17, wd F45, wd F27 and wfe F18 were each
   re-derived against live source and UPHELD. Evidence recorded in-spec:
   F17 - `get_game_details` uses the viewer's player row only to pick a render
   perspective, unlike `get_game_logs` which does reject non-players.
   wfe F18 - `classify_opponent`'s `strip_prefix("bot:")` branch returns
   before the `bot_names` check.
3. **WP-47 scope widens beyond its work-packages.md path list.** D-6's
   "anonymize private users" is a PER-USER rule while the canonical predicate
   is PER-GAME, so WP-47 specs one derived batched helper `db::visible_user_ids`
   with a drift-guard test, and this forces `stats::HeadToHead::user_id` to
   `Option<Uuid>` plus a small render change in `rust/web/src/players.rs`.
   `players.rs` was NOT in WP-47's declared paths - recorded here so it is not
   double-owned by a Tier 3 web package.
4. **Open question for the Orchestrator/user (WP-45):** restart rebuilds bot
   slots from a FINISHED game's persisted bots, so write-validation rejects a
   restart when a bot was disabled after the original game. Worker specced
   this as intended creation-time feedback per D-8 (validate on write), but it
   is the one place where write-validation touches already-stored data. If the
   user wants restarts to survive bot deprecation, D-8's answer needs a carve-out.

Read-only compliance: writes confined to `planning/specs/` and this LOG.

## WP-49 rules and game-info pages (worker, 2026-07-25)

Wrote `planning/specs/WP-49-rules-and-game-info-pages.md` (121 lines, under the
120-line target by content but 121 with the trailing table row; format calibrated
on WP-47/WP-45). Scope: wd F67 major + 7 riders (F68, F69, F70, F71, F76, F79,
F80) + the lead-routed `rules.rs` error-surfacing item.

Findings validated against LIVE source (`rust/web/src/rules.rs`,
`game_info/mod.rs`, `game_info/queries.rs`, `db.rs`). No verification file exists
for `web-domain.md` (confirmed: `findings/verification/` has no `web-domain*`),
so the raw findings file was used. **No finding rejected** - F67, F69 and F76 all
verified correct as written in live code and explicitly marked so in the spec.

Routed-in item resolved: the plan's `rules.rs:46` is the `RulesPage`
`Some(Err(e))` arm rendering `{e.to_string()}` raw. It is NOT an error swallow -
`get_rendered_rules` already routes infra failures through `crate::error::internal`
(opaque). The leak is the non-`internal` errors (`RenderError` authoring detail,
"Game version not found", "Not authenticated"). Fix uses the existing
`crate::error::user_facing_server_error` helper (convention from `new_game.rs`).

Cross-package hazard recorded in the spec: F69 adds `is_public`/`is_deprecated`
filters to `find_game_version_rules` and `find_game_version_render_meta`, which
WP-41's Test 9 (`game_and_version_lookups`) asserts against. The spec instructs
the implementer to re-read that test after the change and STOP if ambiguous.
WP-41 -> WP-49 landing order stated in the header.

Read-only compliance: writes confined to `planning/specs/` and this LOG. No
`rust/` file touched, no cargo/git command run.

---

## 2026-07-25 - Tier 2 batch T2-B2 (part 2) Lead - WP-49 ACCEPTED; T2-B1+T2-B2 CLOSED

Worker 3 (opus) returned `specs/WP-49-rules-and-game-info-pages.md` (121
lines). Accepted; no second verification pass.

1. **The routed-in `rules.rs` error-surfacing item is NOT an error swallow.**
   Live `get_rendered_rules` already routes DB and game-service failures
   through `crate::error::internal` (logs, returns opaque "Internal server
   error"). The real defect is in `RulesPage`: its `Some(Err(e))` arm
   interpolates `{e.to_string()}` straight into the page, so every error that
   does not pass through `internal` (`RenderError` authoring-detail strings,
   "Game version not found", "Not authenticated") is rendered raw. The spec
   adopts the existing helper `crate::error::user_facing_server_error`, already
   used in `new_game.rs`. The plan's `rules.rs:46` hint was approximate.
2. **No finding rejected.** F67 (`ORDER BY name` in
   `game_info_rules_version_id`), F69 (no `is_public` filter in
   `find_game_version_rules` / `find_game_version_render_meta`) and F76 (dead
   `pub use queries::*;` - zero `use crate::game_info::*` in the crate) each
   verified correct against live source and marked explicitly so.
   `findings/verification/` has no `web-domain` file; the raw findings stand.
3. **Cross-package hazard:** F69's `is_public = true AND is_deprecated = false`
   filters land on two `db.rs` functions that **WP-41's Test 9
   (`game_and_version_lookups`) asserts against**, so WP-41's fixtures may need
   updating. The spec instructs the implementer to re-read that test and STOP
   if ambiguous rather than guess.
4. **Conditional revert note:** F80's switch of `RulesPage` from
   `LocalResource` to `Resource::new_blocking` is sound ONLY because F68
   removes the auth gate. If the public-content posture (D-6) is ever
   reversed, F80 must be reverted with it.

### Unit close-out - T2-B1 and T2-B2 complete

Five specs written, all accepted: WP-34, WP-35, WP-47, WP-45, WP-49. No
package in either batch turned out to be empty. Nothing under `rust/` was
written; no cargo/build/test/clippy/fmt run; no git mutation. Lead and all
three Workers wrote only inside `planning/`.

Items escalated to the Orchestrator: (a) new landing constraint
WP-41 -> WP-36 -> WP-34 -> WP-35, not yet in `landing-order.md`; (b) D-14
says "confirmation link" but live auth uses a 6-digit code - user call;
(c) D-8 may need a carve-out so game restarts survive bot deprecation;
(d) WP-47 pulls `rust/web/src/players.rs` into scope, outside its declared
path list.

---

## 2026-07-25 - WP-41: db.rs quality pass - COMPLETE

Commit: `baa5fc6` on `master`.

All 11 tasks implemented per spec. Full `scripts/rust-test.sh` passes (exit 0,
462 web tests green). Two test-fixture bugs discovered and fixed during
verification: (1) `make_game_with_players` shuffles positions, so
`whose_turn: &[0]` does not guarantee the creator is on turn - tests now
explicitly UPDATE `is_turn` after creation; (2) the `update_is_turn_at`
trigger overwrites `is_turn_at` on false->true in the same statement, so
backdating requires a second UPDATE.

Findings discharged: F35 (major, 25/27 covered), F36, F37, F39, F40, F41
(comment), F43, F44, F45, F46, F47, F48, F49, F50, F51(2+3).
Findings NOT touched per spec: F34/F38 (WP-40), F42 split (deferred),
F51(1) (overturned).

Files changed: `rust/web/src/db.rs` (+1397/-125), `rust/web/.sqlx/` (cache
regenerated). No other files.

---

## 2026-07-25 - T2-B3/T2-B4 Lead (batches 3 and 4)

Fresh Lead. A previous attempt at this unit died from a session limit before
writing anything; nothing of B3/B4 had landed. Read-only: nothing under
`rust/` written, no cargo/git run.

### Worker 1 - WP-38 bot-turn wedge recovery - ACCEPTED

`specs/WP-38-bot-turn-wedge-recovery.md`, 175 lines (over the ~120 target;
6 findings across 4 files plus the D-5 preamble - accepted as proportionate,
not padded).

All six findings verified against LIVE source and **all six confirmed as
written**, none already fixed: wd F1/F2/F3/F5 (the three ack arms in
`run_bot_command_consumer`, the `Conflict` exhaustion `return Ok(())`,
`publish_bot_turns`' two `warn!`-only failure arms, and no `.term()` anywhere
in `web/src`); ws F27 (its UNCERTAIN resolved - `run_bot_turn` skips and
returns `Ok(())` when `config::load_bot_config` is `None` and `bots` is
non-empty); bo F2 (UNCERTAIN resolved - `ack_wait = 5 min` in
`nats::ensure_stream_and_consumers`). WP-39 has **not** landed in the live
tree yet, so its scope is fenced by spec text only.

Two finding recommendations DECLINED in-spec with rationale:
1. **ws F27's structural fix** (bots by id via a migration) - rejected by D-5,
   which is explicit that bots stay by NAME and there is no migration. Only
   the warn/surface half is specced.
2. **wd F3's "surface publish failures as Err"** - declined: `execute_command`
   has already committed, so leaving the `bot.command` unacked re-runs the
   whole command on redelivery against advanced state. The reconciliation
   sweep is the recovery path instead.

Lead rulings made on the Worker's two open questions:
- (a) The sweep gains a `jetstream` param on
  `email::sweep::spawn_periodic_sweeps` and `game::publish_bot_turns` becomes
  `pub(crate)`. ACCEPTED - the sweep module is already the home of every
  periodic job, and the alternative (a second scheduler in `game/`) is worse.
- (b) The 15-minute sweep threshold and 60s `AckKind::Progress` cadence are
  Worker judgement calls, not from D-5. ACCEPTED as defaults; flagged to the
  Orchestrator as tunable, not load-bearing.

New landing constraint added to the spec header: **WP-46 also owns
`web/src/email/sweep.rs`**. WP-38's edit there is additive, so either order
works, but the second to land must rebase rather than fork the scaffolding.

### Worker 2 - WP-57 inbound webhook delivery semantics - ACCEPTED

`specs/WP-57-inbound-webhook-delivery-semantics.md`, 140 lines.

All three findings (wfe F2 major, F10 minor, F16 nit) confirmed against LIVE
code, none already fixed, none incorrect: `mark_event_processed` still runs
immediately after signature verification and *before* payload
deserialization; all three route handlers still return `()`;
`resend_webhook`'s tail is still an unconditional `StatusCode::OK`;
`verify_webhook` still has three `HeaderValue::from_str(...).unwrap()`.
`inbound.rs` (2014 lines) matches WP-59's architecture description - the
concurrent critical-fix agent has not touched it.

**Test-fixture gap confirmed.** `rust/web/src/email/` has exactly one
`#[cfg(test)]` module outside `inbound.rs` (in `outbound.rs`), and
`inbound.rs`'s own `mod tests` is pure unit tests. No `AppState`/webhook
fixture exists. The spec directs a new `rust/web/tests/inbound_webhook.rs`
copied from `ssr_pages.rs`'s `make_state` + `#[sqlx::test]` + `build_router`
+ `oneshot` pattern, signing bodies with `svix::webhooks::Webhook::sign`
(confirmed present in svix 1.98) and leaving `RESEND_API_KEY` unset as the
transient-failure trigger - so no Resend HTTP double is needed.

Lead rulings:
- **ACCEPTED** the Worker's addition (not in the finding): at-least-once
  retries re-execute non-idempotent work (game commands, invite responses),
  so `Retry`/5xx is restricted strictly to failures occurring *before* any
  state mutation. Post-dispatch and reply-send failures stay 200-and-marked.
  Without this, D-2's literal "5xx on transient failure" would double-apply
  moves. This is the correct reading of D-2 and must not be simplified away.
- **Cross-package coupling flagged:** WP-57 widens WP-59's new
  `fetch_inbound_text(state, email_id) -> Option<String>` to a
  `Result`-shaped return so a failed fetch is distinguishable from an empty
  body. WP-59 owns that function; WP-57 changes its shape only, not its body.
  Both implementers need to know. Reinforces WP-59 -> WP-57 ordering.

### T2-B3 CLOSED - both specs written and accepted.

### Worker 3 - WP-10 pub_state hidden-info redaction - ACCEPTED

`specs/WP-10-pub-state-hidden-info-redaction.md`, 129 lines. Covers three
items: f F1, f F13, and the routed-in starship-catan-1 `peeking` item.

All three confirmed live, none already fixed, none incorrect:
- **f F1 (zombie-dice-2)** - `pub_state()` still does `cup: self.cup.clone()`,
  `take_dice` still drains from the front, shuffle only at turn start/refill.
  `render_cup` already collapses to per-colour counts, so a counts-only
  `PubState` costs nothing visually. `DATA_DOCS.md`'s "no hidden information
  per player" claim is still wrong and is in scope.
- **f F13 (for-sale-2)** - `pub_state()` still clones `bids`. The Selling
  render arm reads the viewer's own play from `pub_state.bids[p]`, so
  redaction *requires* adding a `bid: i32` to `PlayerState` and repointing the
  renderer. The Buying arm and `highest_bid` stay on the public `bids` - open
  auction, legitimately public.
- **starship-catan-1 (routed in from WP-13)** - CONFIRMED still exposed at the
  JSON level: `player_state()` sets `peeking: self.peeking.clone()`
  unconditionally for both seats. **WP-13 has not landed** (`render()` still
  takes `_peeking` unused), so neither the render guard nor the data fix
  exists. WP-10's fix is a one-line `player == self.current_player` guard and
  does not collide with WP-13 Task 5.

Lead rulings:
- **ACCEPTED counts over sorted.** D-33 option A's text names both "counts"
  and "canonicalized"; the spec uses
  `PubState::cup_counts: Vec<(Colour, usize)>` in fixed Green/Yellow/Red
  order. Information-equivalent to a sorted vec but a cleaner API. It is a
  `PubState` API-shape change visible to bot clients, NOT a persisted-state
  change.
- Test-module naming differs per crate (`mod test` in zombie-dice-2 and
  for-sale-2, `mod tests` in starship-catan-1) and two existing tests
  (`test_pub_state_captures_rendered_fields`,
  `test_pub_state_redacts_hands_and_cheques`) assert the leaky equality
  directly. The spec names both - they must be updated, not deleted.

**This spec sets the redaction shape for every game crate.** Later crates copy
it.

### LEAD RULING - WP-09 is SPLIT into WP-09a and WP-09b

19 findings across ~17 crates is too large for one Tier 2 spec. The Tier 2
plan anticipated this. The split is:
- **WP-09a** - requester-boundary half: the two majors (e F18, e F36), the
  systemic `Gamer::player_state` totality gap, and the routed-in acquire-1 /
  sushizock-2 panics. **Lands FIRST.**
- **WP-09b** - per-crate defensive sweep of the remaining 17 minor/nit
  findings plus red7-1's `num_players` trust. Lands after 09a.

`work-packages.md` needs updating to record the split and to add
`rust/game/acquire-1`, `rust/game/sushizock-2` and `rust/lib/game/src/game.rs`
to WP-09's path list. Flagged to the Orchestrator; not done by this Lead
(work-packages.md is not this unit's file).

### Worker 4 - WP-09a deserialized-state boundary - ACCEPTED

`specs/WP-09a-deserialized-state-boundary.md`, 166 lines (over the ~120 hint;
four routed-in items each need their own Problem/Why/End-state entry -
accepted as proportionate).

**Design answer on the `player_state` totality question: boundary check in
`gamer.rs` only, trait signature UNCHANGED.** Rationale (Worker read every
caller): `gamer.rs::renders` iterates `0..game.player_count()` and
`lib/game/src/bot.rs` picks from `game.whose_turn()`, so both are already
bounded; the only unchecked callers are `handle_play` and
`handle_player_render`, which take the index straight off the deserialized
`Request` envelope. Changing `player_state -> Result` would touch ~30 crates
for zero additional safety. **ACCEPTED.**

D-36's second half (the validate hook) lands as a defaulted no-op
`Gamer::validate(&self) -> Result<(), GameError>` in
`rust/lib/game/src/game.rs`, called in `Requester::request` after each
`serde_json::from_str` (all four game-carrying variants). No crate implements
it in this package - that is deliberate; WP-09b and later crate packages fill
it in.

All four routed-in items confirmed present in LIVE code by reading:
`self.hands[player].clone()` in both lost-cities crates' `player_state`; both
`panic!("must be Phase::SellOrTrade")` arms in acquire-1's
`next_player_sell_trade` and `end_sell_trade_phase`; sushizock-2's
`steal_blue`/`steal_red` guard `player == target` and emptiness but never
`target < self.players`. Test modules named in the spec are all real
(`api.rs` `mod tests`, sushizock-2 and lost-cities `mod test`, acquire-1
`mod tests`, `game.rs` `mod tests`); the `gamer.rs` test module does not exist
and the spec says to create it.

**New cross-package constraint:** WP-21 Task 10 refactors
`steal_blue`/`steal_red` into a shared helper *after* WP-09a lands. That task
must carry the new `target < self.players` guard forward, not drop it.
Escalated to the Orchestrator.

### Worker 5 - WP-09b game-crate state-trust sweep - ACCEPTED

`specs/WP-09b-game-crate-state-trust-sweep.md`, 138 lines. Table-shaped:
one row per item, 18 rows.

**All 18 of 18 items confirmed present in LIVE code.** None already fixed,
none judged incorrect, nothing left UNVERIFIED.

The Worker read every affected `pub struct Game` and corrected field names
that a reader would otherwise guess wrong: for-sale-2 indexes on
`bidding_player` (not `current_player`); no-thanks-2 on `currently_moving`;
red7-1 has `scored_cards` (not `scores`); category-5-2's `board` is a fixed
`[Vec<Card>; ROWS]`; age-of-war-2 has no per-player vectors at all, only
castle-indexed ones.

**Lead ratifies the two-tier pattern:**
- **Tier 1** - a per-crate `Gamer::validate` impl filling WP-09a's new
  defaulted hook (14 of 15 crates), leaving the panicking index /
  `unreachable!()` sites themselves untouched as defence-in-depth.
- **Tier 2** - an in-place guard for the four sites reachable *without* a
  deserialized `Game`: lords-of-vegas-1 `Loc::parse_str`, love-letter-2
  `assert_target`, age-of-war-2 `line_action`, for-sale-2's two `split_off`,
  no-thanks-2's renderer `unwrap`.
This makes WP-09b strictly dependent on WP-09a landing first. Correct call -
it is what makes the sweep a one-line-per-crate job instead of a rewrite.

Two verification corrections carried into the table as binding:
- **greed-2 f F34's `points()` citation is a MIS-CITATION** - do not touch it.
- **no-thanks-2 f F53 UNDERCOUNTS** - `chips[p]` and `final_scores[p]` are
  also unchecked.

Test conventions read per crate, not assumed: `mod tests` in
lords-of-vegas-1 / tic-tac-toe-2 / red7-1, `mod test` in eleven others,
modern-art-2 and lost-cities-2 have both. No crate needs a new test module.

### T2-B4 CLOSED - three specs written and accepted (WP-10, WP-09a, WP-09b).

### Unit close-out - T2-B3 and T2-B4 complete

Five specs written, all accepted: WP-38, WP-57, WP-10, WP-09a, WP-09b (WP-09
split into two). No package in either batch turned out to be empty; every one
of the 27 findings across the two batches was confirmed present in live code
and none was already fixed. Nothing under `rust/` was written; no
cargo/build/test/clippy/fmt run; no git mutation. Lead and all five Workers
wrote only inside `planning/`.

Escalated to the Orchestrator - see the additions to `landing-order.md`:
(a) WP-09 split into 09a/09b - `work-packages.md` needs the split recorded
    and `rust/game/acquire-1`, `rust/game/sushizock-2`,
    `rust/lib/game/src/game.rs` added to its path list;
(b) WP-21 Task 10 must carry WP-09a's new sushizock-2 `target` guard forward;
(c) WP-57 widens WP-59's `fetch_inbound_text` return shape;
(d) WP-38 and WP-46 both touch `web/src/email/sweep.rs`;
(e) WP-38's 15-minute sweep threshold and 60s `AckKind::Progress` cadence are
    Lead-chosen defaults, not from D-5 - tunable, not load-bearing;
(f) WP-10's `PubState::cup_counts` is a bot-client-visible API shape change.

---

## T2-B5 / T2-B6 Lead (final Tier 2 batches) - 2026-07-25

### Worker 1 returned - WP-02 markup robustness and dedup

Wrote `planning/specs/WP-02-markup-robustness-dedup.md` (139 lines; 10
findings, so slightly over the ~120 cap - accepted). All ten findings
(verification `ls F2`-`ls F11`) specced; none already fixed, none rejected.
Numbering hazard handled correctly: verification F10/F11 = raw F11/F12, raw
F10 (ANSI/plain renderer escaping) is absent from verification and was
explicitly marked out of scope.

**ESCALATION - D-37 option A refined by the Worker.** D-37's answer says
"error on non-empty rest + define an escape (`{{` or backslash) + escape on
`to_string`". A bare `{{` escape is UNSOUND: it matches the leading `{{` of
every closing tag, so a nested `markup()` would consume its own terminator.
The Worker pinned **`{{lbrace}}`** instead and recorded it as the spec's Open
question. The Orchestrator should put this to the user - it is a change to the
answered decision's letter, not its intent.

Two further Lead-accepted spec calls:
- the escape is implemented inside `parser.rs::text` (yielding one `char`)
  rather than as an eleventh `choice` alternative, so
  `from_string(to_string(n)) == n` holds exactly instead of splitting text
  into adjacent nodes;
- `from_string` keeps its `(nodes, rest)` signature to avoid churning ~10
  call sites across `web/`, `lib/cmd` and `tools/` while another agent is
  editing `rust/`.

Note for WP-06: making `from_string` hard-error on leftover input turns
`lib/cmd/src/repl.rs`'s two `from_string(...).unwrap()` calls into live panic
sites. WP-02 lists them as a non-goal; WP-06 owns them.

### Spec ACCEPTED: WP-02.

---

## WP-01 implementation progress

Date: 2026-07-25. Lead: orchestrate Lead role (WP-01 execution unit).

All 7 tasks implemented and committed as `c2637c1`:

- Task 1 (lg F1): `Space::parse` - `input.len() - input.trim_start().len()`
- Task 2 (lg F2): `Token::parse` - `input.get(..t_len)` boundary check
- Task 3 (lg F3 + lg F4): `shared_prefix` returns `(input_bytes, value_bytes)`;
  `Enum::parse` uses value-bytes for full-match detection
- Task 4 (lg F16): `Int::parse` - `char_indices` byte-length accumulation
- Task 5 (ls F1): `slice()` - char-unit slicing + `<=` boundary skip
- Task 6 (e F29): `CardParser` - `chars[0].len_utf8() + chars[1].len_utf8()`
- Task 7: docs/CODING.md non-ASCII test convention added

Verification: `scripts/rust-test.sh` passed end-to-end (107 brdgme_game,
48 brdgme_markup, 19 red7-1, 480 web, 41 ssr_pages, 5 nats_bot_eventing,
1 websocket_hygiene; 2 ignored known-flaky NATS). fmt + clippy clean.

One deviation from spec: markup `slice` tests needed `slice::<Color>(...)`
turbofish for type inference on bare `TN::text(...)` assertions (E0283).
Functionally identical to spec intent.

### Worker 2 returned - WP-08 finish/placings epilogue dedup sweep

Wrote `planning/specs/WP-08-finish-placings-epilogue-dedup.md` (150 lines; 12
findings across 13 crates - over the ~120 cap but proportionate, accepted).
All 12 specced; none already fixed, none rejected.

**Refactor shape (the spec's first decision, made and justified): identical
per-crate private `finish_epilogue(&self, logs: &mut Vec<Log>)` extract, NO new
`lib/game` API.** Worker read live epilogues in nine crates: the only common
line is `logs.push(placings_log(&placings, Some(&scores)))`, and
`brdgme_game::placings_log` already IS the shared helper. Everything above it
diverges per crate - the finished predicate (rtta uses a `self.finished` field,
not `is_finished()`), the scores expression (`player_points` /
`player_total_money` / `player_vp` / `scores()` / token sums / `player_score`)
and the placings expression (`self.placings()`, `self.calc_placings()`, or a
local `gen_placings`). A shared helper would have to take both `scores` and
`placings` - i.e. be `placings_log` again - or take closures. Lead accepts.

e F14 handled as a uniform `!was_finished && is_finished()` transition gate
(a no-op except in age-of-war-2).

Routed-in scope honoured: **acquire-1** joins (cheap hoist into its existing
trailing `.map`); **starship-catan-1** joins, coverage widening from 5 of 17
arms to all 17. **red7-1 fenced out in Non-goals.**

**Two Worker rulings the Orchestrator may overturn:**
1. **lost-cities-1/-2 get no code change.** The routed-in "double placings-log
   site" was re-derived: each crate has exactly ONE epilogue site (the `Draw`
   arm, its only finishing path). The real duplication is `end_round`'s
   `game_over_log()` plus `placings_log` both announcing the winner - and
   WP-28 Task 4 deliberately rewrites `-2`'s `game_over_log()` and asserts its
   wording, so deleting either line would contradict that package. Closed as
   "both stay" rather than left unowned.
2. starship-catan-1's widening from 5 to 17 arms is stated as
   intentional-and-safe, NOT as a bug fix - the Worker did not re-trace whether
   the 12 uncovered arms can actually reach `victory_points() >= 10`.

Incidental find: greed-2's `Score` arm has no epilogue (it cannot finish
today); the hoist closes it.

### Spec ACCEPTED: WP-08. T2-B5 CLOSED.

### Worker 3 returned - WP-62 operator

Wrote `planning/specs/WP-62-operator.md` (150 lines; two majors plus five
riders - over the ~120 cap but proportionate, accepted). Source: **raw
`findings/bot-operator-tools.md`; no `findings/verification/bot-operator-tools.md`
exists** and the spec says so.

**Routed-in second major (WP-28 lead ruling) specced. Design call: newest
non-deprecated version wins, NOT union.** Justification from live code: new
games pick a version via `rust/web/src/db.rs::find_latest_non_deprecated_game_version`,
but roster validation reads the shared type row via
`find_game_type_player_counts` (keyed by version id, returns
`game_types.player_counts`). A union would let validation accept a 3-player
Lost Cities roster that the actually-selected version cannot run. Fix is three
statements in `upsert_game_type_and_version` (id-only upsert, version upsert,
then a guarded `UPDATE` that writes counts/weight/blurb only when no newer
non-deprecated version exists). No migration. **`upsert_game_type_and_version`
lives in `rust/operator/src/controller.rs`, not in `rust/web`** - the package
stays single-crate. Lead accepts.

Dispositions: bo F18 specced; routed-in `game_types` major specced; bo F20-F24
specced; bo F25 BLOCKED.
- **bo F19 specced WITH A CORRECTION.** The field genuinely does not exist, but
  the finding's symptom claim is wrong: the applied `k8s/base/operator/crd.yaml`
  never had a Players printcolumn. The real defect is derive-vs-manifest drift,
  so the fix is to DELETE the printcolumn, not to surface it in status (which
  would need a CRD manifest edit outside the declared paths).
- bo F20 specced as an async wrapper around `reconcile`, because `error_policy`
  is sync and cannot await.

**OPEN QUESTION FOR MICHAEL (unchanged, escalate): what Kubernetes version does
the deployed cluster run?** bo F25's `k8s-openapi` pin cannot be chosen without
it. The rider is recorded as blocked; once answered, select the `v1_NN` feature
for the oldest targeted cluster in `rust/operator/Cargo.toml`.

Implementer caveat carried in the spec: `kube-runtime` 4.0.0 is in the lockfile
with the `runtime` feature on, but the exact `kube::runtime::finalizer` module
path was NOT confirmed against the vendored crate - the spec instructs
STOP-and-report if it differs.

### Spec ACCEPTED: WP-62.

### Worker 4 returned - WP-63 fuzz tool

Wrote `planning/specs/WP-63-fuzz-tool.md` (150 lines - over the cap for a small
package, but the dedup ruling and the bo F26 hang-test recipe needed the room;
accepted). Confirmed `findings/verification/` has NO file for either
bot-operator-tools or dependencies, so every claim was re-derived from live
source; the spec states this.

Live layout is `Cargo.toml` + `src/lib.rs` + `src/main.rs` (no `src/bin/`), and
the crate has **no test module** - the spec directs a new `#[cfg(test)] mod
tests` in `src/lib.rs` using a stub `Requester` from the already-present
`brdgme_cmd` dep, deliberately avoiding the `[dev-dependencies]`/`src/bin` trap
(e F45).

**Dedup ruling: KEEP SEPARATE from `brdgme_rand_bot::commands()`.** It is
private, returns quality-scored `Vec<BotCommand>` that the fuzzer would
immediately unwrap back to a `String`, and after WP-07 changes its join to `""`
the outputs already match. The real shared primitive is `spec_to_command`,
which the fuzz tool already calls. No landing-order dependency on WP-07
results. Consistent with WP-07's own rider table, which defers the "sharing
half" of ls F41 to here. Lead accepts.

Dispositions: bo F26, F27, F29, F30, F31 specced; **bo F28 = dp F20 specced
ONCE** (single `num_cpus` change, both IDs cited). Nothing already-fixed,
nothing rejected.

Minor open item: the bo F31 regression test needs a hand-built
`api::Response::New` fixture (Active status, `command_spec: None` render). The
spec marks it droppable-with-a-note if disproportionate; the F26 hang test is
the only mandatory one.

### Spec ACCEPTED: WP-63. T2-B6 CLOSED.

### Bookkeeping applied to `work-packages.md`

- **WP-09** heading now records the **split into WP-09a + WP-09b** (09a =
  requester boundary + new `Gamer::validate` hook, lands first; 09b = the
  18-item per-crate sweep, strictly depends on 09a), and its path list gains
  `rust/lib/game/src/game.rs`, `rust/game/acquire-1` and `rust/game/sushizock-2`.
- **WP-21** gains a note that Task 10 must carry WP-09a's new sushizock-2
  `Player{} target` bounds guard forward through its refactor.
- **WP-59** gains a note that WP-57 widens `fetch_inbound_text`'s return shape.

### UNIT CLOSE-OUT - T2-B5 and T2-B6 complete; ALL TIER 2 BATCHES ARE DONE

Four specs written and accepted: WP-02, WP-08, WP-62, WP-63. No package in
either batch was empty; all 37 findings across the four packages were confirmed
present in live code and none was already fixed. Nothing under `rust/` was
written; no cargo/build/test/clippy/fmt run; no git mutation. Lead and all four
Workers wrote only inside `planning/`.

**Escalated to the Orchestrator (decisions the user must make):**
(a) **D-37's `{{` escape token is unsound** - it collides with closing-tag
    syntax. WP-02 pins `{{lbrace}}`. Needs the user's blessing.
(b) **bo F25 needs the deployed Kubernetes version from Michael** before the
    `k8s-openapi` pin can be chosen. WP-62's rider is blocked on it.
(c) Two WP-08 Worker rulings that overrode routed-in framing:
    lost-cities-1/-2 get NO code change (the "double placings-log site"
    re-derived as `game_over_log()` + `placings_log`, and WP-28 Task 4 already
    owns that wording); starship-catan-1's 5-to-17-arm widening is stated as
    intentional-and-safe, not as a verified bug fix.

---

## TIER 3 - Lead unit T3-B1..B3 (2026-07-25)

### Worker 1 returned: T3-B1

`checklists/T3-B1-zombie-battleship-forsale-category5.md`. WP-31 (7) + WP-32
(12) = 19 findings, 20 table rows (`f F3` split across `lib.rs` and
`render.rs`). games-batch-f numbering; raw == verification, no offset. Nothing
escalated; nothing rejected; no line numbers cited. Plan corrections applied:
`f F18` uses `Option<Phase>` + explicit fallback (NOT the finding's unsound
`#[serde(default)]`), `f F28` label-only (do not negate `points()`), `f F6`
notes a transition-only guard misses mid-rolloff membership changes.

### Checklist ACCEPTED: T3-B1.

### Worker 2 returned: T3-B2

`checklists/T3-B2-small-game-crates.md`. WP-33 = 17 findings, 17 rows (4m/13n)
across greed-2 (3), farkle-2 (5), tic-tac-toe-2 (3), no-thanks-2 (3),
liars-dice-2 (3). games-batch-f numbering; raw == verification. Nothing
escalated, no spec overlap, no line numbers. Verification corrections carried:
`f F56` reframed as reachable via F57's uncapped parser; `f F48` scoped to
log+board+label+RULES.md with the exact-render test update
(`exact_render_markup_matches_the_old_board`) mandatory. `f F46` (ttt unbounded
`players`) is out of WP-33 scope - an inline note points at WP-09a/WP-09b so
the omission is not misread as a drop.

### Checklist ACCEPTED: T3-B2.

### Worker 3 returned: T3-B3

`checklists/T3-B3-splendor-libcost-holdem.md`. WP-17 (8) + WP-18 (4) = 12
findings, 12 rows: 9 in the main table (splendor-2 `b F30/F32/F34/F35`,
`lib/cost` `ls F38`, texas-holdem-2 `c F1/F3/F4/F5`) plus a separate
**BLOCKED ON D-25** table of 3 (`b F31`, `ls F39`, `dp F27` - all one
consolidation change, options A/B mutually exclusive, must land together).
Numbering documented per prefix: `b`/`c` positional raw==verification, `ls` =
VERIFICATION numbering (+1 offset vs raw for F10+), `dp` = no verification
file, lead-verified only. Nothing escalated, nothing dropped, no line numbers.
No spec overlap: `specs/WP-06-*.md` routes `ls F38`/`ls F39` out to WP-17, and
WP-15 only reads `lib/cost::can_afford_perm`; `b F33`/`c F6` are WP-08's and
were already out of scope.

### Checklist ACCEPTED: T3-B3.

### UNIT CLOSE-OUT - T3-B1, T3-B2, T3-B3 complete

Three checklist files written under `planning/checklists/`, 48 findings / 49
rows total. No specs written (correct for Tier 3). Nothing under `rust/` was
written; no cargo/build/test/clippy/fmt run; no git mutation. Lead and all
three Workers wrote only inside `planning/`.

**Escalated to the Orchestrator:**
(a) **D-25 is unanswered and gates 3 of T3-B3's 12 rows.** `work-packages.md`
    marks WP-17 BLOCKED-ON-DECISION(D-25) but `tier2-tier3-plan.md` section
    2.1 lists T3-B1..B3 as dispatchable now and omits WP-17 from its blocked
    roster (2.2) - the two documents disagree. Resolved by carving the three
    gated rows into their own table rather than dropping or shipping them.
    Recommendation stands at option A (port splendor-2 onto `lib/cost`).
(b) `f F46` (tic-tac-toe-2 unbounded `players`) is in no Tier 3 package -
    it belongs to WP-09a/WP-09b. T3-B2 carries an inline pointer only.

## Lead: T3-B4 / T3-B5 / T3-B6 unit

### Worker 4 returned: T3-B4

`checklists/T3-B4-sushigo-loveletter-ageofwar.md`. WP-24 sushi-go-2 (7) +
WP-27 love-letter-2/age-of-war-2 (8) = 15 findings, 15 rows (4m/11n), grouped
by crate then source file, no line numbers, T3-B2 house format. Nothing
decision-blocked, nothing escalated, no rows dropped. Verification corrections
applied: `d F26` downgraded to nit and made **doc-only** (the official
Gamewright rulebook DOES break ties by puddings, so `placings` and
`test_placings_pudding_tiebreaker` must NOT change); `d F31` is not a
byte-for-byte duplicate (method vs free fn) so the row keeps the free fn and
delegates. A "Not in this checklist (owned elsewhere)" section names the
adjacent spec-owned findings so the omissions are not misread as drops:
`d F33`, `e F13`, `e F14` -> WP-08; `e F10` + the love-letter
`assert_target`/`end_round`/`end_score` items -> WP-09a/WP-09b. Sequencing
warning recorded: `e F12` edits the same age-of-war `command` fn WP-08 touches.

### Checklist ACCEPTED: T3-B4.

---

## WP-39 implementation complete

Date: 2026-07-25. Commit: 347970a on master.

All 8 tasks implemented and verified via `scripts/rust-test.sh` (full pass):

- Task 1 (ws F53/wd F4): `supervise_consumer` in `web/src/nats.rs` with
  exponential backoff (1s..30s, reset after 60s stable). Wired in
  `web/src/main.rs`. 2 unit tests (paused-clock tokio).
- Task 2 (ws F56): `run_max_deliveries_advisory_listener` +
  `parse_max_deliveries_advisory` in `web/src/nats.rs`. Supervised spawn in
  main.rs. 1 unit test + 1 integration test (Nak-forced exhaustion against
  real nats-server).
- Task 3 (ws F57/F58): `stream_config_drift`/`consumer_config_drift` +
  startup warn in `ensure_stream_and_consumers`. ack_wait invariant comment.
  2 unit tests.
- Task 4 (wd F9): Conflict re-publish filtered to `event.player_position`
  in `game/mod.rs`. 1 two-bot integration test.
- Task 5 (bo F1): `unreachable!()` -> `Err(anyhow!(...))` in
  `bot/src/main.rs`.
- Task 6 (bo F3/F5): `Arc<Semaphore>` concurrency bound +
  `shutdown_signal()` + drain in `bot/src/main.rs`.
- Task 7 (bo F8): healthz doc comment declining DB-check recommendation.
- Task 8: `scripts/rust-test.sh` full pass (485 web unit, 7 nats_bot_eventing
  integration, 41 ssr_pages, 35 bot, workspace libs).

New alertable metrics: `nats_consumer_restarts_total`,
`bot_stream_max_deliveries_total` (web :9090 /metrics). New env:
`MAX_CONCURRENT_TURNS` (bot, default 8). No k8s manifest changes.

## WP-36 implementation complete

Date: 2025-07-25.

All 5 tasks implemented and verified via `scripts/rust-test.sh` (full pass,
491 web unit tests, 0 failures):

- Task 1 (ws F52): `secure_cookie()` helper in `web/src/auth/session.rs`;
  session cookie Secure by default, `SECURE_COOKIE=false` opts out. 3 unit
  tests.
- Task 2 (ws F52 deploy): `k8s/dev/web-patch.yaml` (new), dev kustomization
  patch, Tiltfile local web env, `.env.template` documentation. Prod needs
  no manifest change.
- Task 3 (ws F54): `rustls::crypto::aws_lc_rs::default_provider().install_default()`
  as first statement of web's main; optional ssr-gated dep.
- Task 4 (ws F17): `Zeroizing<[u8; 32]>` return from `load_key`/`default_key`
  in `web/src/crypto.rs`; hex buffer wiped. AAD declined (shared format with
  bot + existing prod ciphertexts). 3 unit tests.
- Task 5 (ws F55): `CancellationToken` + `TaskTracker` on `GameBroadcaster`;
  close frame on shutdown; bounded 5s drain in main. 1 integration test.

Prod picks up F52/F54/F55 on next web image deploy; no prod manifest change.

---

## WP-14 alhambra-1 core fixes - IMPLEMENTED 2026-07-25

Lead: orchestrate Lead role. All 10 tasks from
`planning/specs/WP-14-alhambra-core-fixes.md` executed serially via Workers.

Findings fixed: b F16 (critical, take() duplicate mint), b F17 (major,
place/swap index divergence), b F18 (major, wall-walk premature break),
b F21 (minor, missing tests), b F23 (nit, expect() naming), b F24 (nit,
gap-check symmetry), b F25 (nit, Debug formatting), b F26 (nit, tile_counts
dedup), b F27 (nit, column-header clamp), b F28 (nit, VecDeque/HashSet).

Files changed: `rust/game/alhambra-1/src/lib.rs`, `src/card.rs`,
`src/command.rs`, `src/render.rs`.

Tests: 33 lib tests + 1 contract test, all passing. Full pre-commit gate
(`scripts/rust-test.sh`) passed. `cargo fmt --all -- --check` and
`cargo clippy --workspace --exclude web --all-targets -- -D warnings` clean.

No serialized-shape changes. No game-rules behaviour changes. Non-goals
honoured (b F19/F20/WP-16, b F22/WP-08, Tile::walls HashMap, rot_all panic).

---

## WP-25 modern-art-2 liveness and cleanup - IMPLEMENTED 2026-07-25

Lead: orchestrate Lead role. Workers: 5 serial (one per task).

Spec: `planning/specs/WP-25-modern-art-liveness.md`. All 5 tasks completed
in order, TDD (red-green) for Tasks 1-3.

Commits (5, not pushed):
1. `7821938` fix(modern-art-2): end the round when all hands are empty at any boundary (d F34 d F35, WP-25)
2. `af2c014` fix(modern-art-2): reset auction state when a round ends (d F41, WP-25)
3. `b0babb8` fix(modern-art-2): hide the bid line until a real bid exists (d F42, WP-25)
4. `e560a75` docs(modern-art-2): correct next-turn and Double-auction rules text (d F39 d F40, WP-25)
5. `6c0c19c` refactor(modern-art-2): drop throwaway vec, unwrap, dead import (d F44 d F45 d F46, WP-25)

Findings fixed: d F34 (critical, settle busy-loop all hands empty), d F35
(major, round-4 empty-handed starter soft-lock), d F39 (minor, RULES.md
next-turn text), d F40 (minor, RULES.md Double omits Once Around), d F41
(minor, stale State::Auction on game end), d F42 (nit, "$0 by auctioneer"
render line), d F44 (nit, can_add throwaway vec), d F45 (nit, guarded
unwrap), d F46 (nit, redundant Default import).

Files changed: `rust/game/modern-art-2/src/lib.rs`, `src/render.rs`,
`RULES.md`.

Tests: 18 lib tests + 1 contract test, all passing (8 new tests added).
Full pre-commit gate (`scripts/rust-test.sh`) passed. `cargo fmt --all --
--check` and `cargo clippy -p modern-art-2 --all-targets -- -D warnings`
clean.

No serialized-shape changes. No game-rules behaviour changes. Non-goals
honoured (d F36/F37/F43/WP-26, d F38/WP-09, WP-08 epilogue dedup).

## Lead: T3-B5 / T3-B6 unit (resumed after previous Lead hit session limit post-T3-B4)

### Worker 5 returned: T3-B5 part 1 (WP-52)
Wrote `checklists/T3-B5-web-domain-stats-misc.md` with the shared batch header
plus WP-52's 13 rows (9m/4n), grouped by source file, no line numbers, T3-B4
house format. WP-53 half deliberately left to a second Worker pass; a
`<!-- WP-53 SECTION GOES HERE -->` marker sits above the shared trailing
sections. Nothing escalated. Dropped as spec-owned: `wd F45` (WP-47) and
`ws F40` (WP-41 Task 8). Sequencing debt recorded in-file: `wd F74` needs WP-41
Task 8 first; the four `stats/queries.rs` rows touching `finished_games` /
`game_history` must rebase onto WP-47's `visible_user_ids` signature change.
Also flagged a documentation defect: `specs/WP-54-frontend-ux-error-handling.md`
mislabels `wd F62` as WP-50-owned email-canonicalization work - it is WP-52's
`get_friends_overview` nit. Not corrected (WP-54 is finalized); surfaced for the
Orchestrator.

### Checklist ACCEPTED: T3-B5 part 1 (WP-52).

### Worker 6 returned: T3-B5 part 2 (WP-53)
Appended WP-53's 12 rows (3m/9n) to
`checklists/T3-B5-web-domain-stats-misc.md` in 7 file groups, updated the header
and merged the trailing sections. **Batch total 25 rows (12m/13n)** from a
27-finding scope.
- **ESCALATED: `wd F18`** (reqwest HTTP call inside the `FOR UPDATE` transaction).
  `create_game_from_service` takes `&mut tx` and performs the request plus all
  new-game inserts in one body, so hoisting the call out means splitting the
  helper and touching all four callers - Tier 2 sized, and it collides with
  WP-40/WP-45 in `restart_core`. **Needs an owner.**
- `wd F6` (`is_eliminated` wipe) was kept as Tier 3: the SQL `CASE` half is one
  line, with a sequencing note behind WP-40/WP-41.
- Dropped as spec-owned: `wd F56` - `specs/WP-41-db-quality-pass.md` already owns
  the identical `send_friend_request` read-then-insert race under its own id
  `ws F39`.
- Nothing decision-blocked. D-41 concerns only the `FriendsPage` `<select>`
  binding; WP-53's single `friends.rs` row is in the `#[server]` region.
- Two recommendations narrowed in-row: `wd F78` takes only the comment half
  (deriving `FromRow` would need new columns + a db.rs rewrite); `wd F65` notes
  `percent-encoding` is a transitive dep only - it appears in no `Cargo.toml`
  under `rust/`.
- Second ID-mislabel defect found in `specs/WP-54-frontend-ux-error-handling.md`'s
  coordination section: it calls `wd F56` the `block_user` nit (really `wd F61`)
  and `wd F65` the `get_friends_overview` nit (really `wd F62`). Its line fences
  are correct so there is no collision; not corrected (WP-54 is finalized).

### Checklist ACCEPTED: T3-B5 complete (25 rows).

### Worker 7 returned: T3-B6
Wrote `checklists/T3-B6-outbound-email-websocket.md`. 12 rows (6m/6n): WP-60 = 9
(5m/4n; `wfe F44` and `wfe F45` kept as two rows pointing at one atomic
`UPDATE..RETURNING` rewrite of `ensure_email_token`), WP-42 = 3 (`ws F60`,
`ws F61`, `ws F62`; 1m/2n). Grouped by package then source file, no line numbers,
T3-B4 house format.
- **ESCALATED with an explicit verdict: `ws F59` - WP-42 NEEDS A COMPACT TIER 2
  SPEC.** Neither half is a one-liner. Task A must thread identity from the
  pre-`on_upgrade` extractors into `handle_socket` and filter every frame through
  WP-47's `is_game_visible_to_user`, a per-frame async DB predicate whose caching
  strategy is an **open design question**. Task B is a wholly new `sub`/`unsub`
  protocol needing per-socket state plus the currently-dropped client `send`
  handle. This confirms the T3-B6 note in `tier2-tier3-plan.md` section 2.1.
  **Orchestrator decision required:** promote WP-42 to a Tier 2 compact spec.
- Constraints recorded in-file: anonymous `/ws` upgrades must keep returning 101,
  never 401 (`rust/web/tests/websocket_hygiene.rs` asserts it); WP-47 lands first
  and WP-42 reuses `is_game_visible_to_user` rather than forking it.
- Nothing contested was dropped: WP-51, WP-54 and WP-59 all explicitly disclaim
  the nine `wfe` findings to WP-60. "Owned elsewhere" records `ws F55` as already
  shipped by WP-36 and the WP-51 / `wfe F46` coordination point on
  `try_send_rendered_email`.
- Decision-blocked table empty: D-13 is answered, so the only cross-package
  constraint is sequencing, not a decision.
- Spot-check: live `websocket_client.rs` still destructures `ready_state: _`, so
  `ws F60` is confirmed unfixed.

### Checklist ACCEPTED: T3-B6 (12 rows).

### UNIT CLOSE-OUT - T3-B5 and T3-B6 complete
Both checklists written and accepted; `planning/checklists/` now holds
T3-B1..T3-B6. Remaining Tier 3 batches: T3-B7 (WP-61 + WP-43) and T3-B8 (WP-65 +
WP-74/WP-75, gated on D-19 and on WP-29/WP-30).

Items for the Orchestrator:
1. **Promote WP-42 to a Tier 2 compact spec** (`ws F59`, above). Its three
   remaining findings stay in T3-B6's table.
2. **`wd F18` needs an owner** - reqwest call inside the `FOR UPDATE` transaction
   in `create_game_from_service`; Tier 2 sized and collides with WP-40/WP-45 in
   `restart_core`.
3. Two ID-mislabel defects in the finalized `specs/WP-54-frontend-ux-error-handling.md`
   coordination text (`wd F62` attributed to WP-50; `wd F56`/`wd F65` swapped for
   `wd F61`/`wd F62`). Line fences are correct, so no collision - documentation
   only, left uncorrected.

## UNIT: T3-B7 + T3-B8 + WP-42 promotion (Lead, 2026-07-26)

### Worker 1 returned: T3-B7 checklist
`planning/checklists/T3-B7-bot-service-web-deps.md` written (142 lines).
- **17 rows (10 minor / 7 nit)**: WP-61 = 12 (`bo F4`, `F6`, `F7`, `F9`, `F10`,
  `F11`, `F12`, `F13`, `F14`, `F15`, `F16`, plus `dp F7`), WP-43 = 5 (`ws F63`
  through `ws F67`). Matches `tier2-tier3-plan.md` section 2.1's projection
  exactly. T3-B4 house format, grouped by crate then source file, no line
  numbers anywhere, implementer instruction reproduced verbatim.
- `bo F12` + `dp F7` are recorded as **two rows pointing at one edit** (drop the
  hand-rolled `rand_nonce`, use `Aes256Gcm::generate_nonce`, then delete the
  direct `getrandom` dependency) - land together.
- `ws F67` kept as a row and flagged **UNVERIFIABLE (external basis)**, not
  rejected: verification could confirm only in-repo declared versions, not
  crates.io currency.
- Escalate section empty - all 17 compress to one line. `bo F9` (nine `try_get`
  sites across `config.rs` and `main.rs`) is the widest but the rule is uniform
  per site, so it stays one row.
- Decision-blocked table empty and justified: D-17/D-18/D-19/D-21/D-23 all belong
  to WP-64/66/67/69/70, not here. `dp F7` removes a direct dependency outright
  rather than restating a version, so it does **not** wait on D-19.
- Dropped as spec-owned, recorded in "Not in this checklist": `bo F1`/`F3`/`F5`
  (WP-39), `bo F2` (WP-38), `bo F17` (WP-70, blocked on D-21 - leave
  `serde_yaml = "0.9"` alone even though `bo F16` edits the same table),
  `dp F13` (WP-68). `bo F8` is outside WP-61's declared scope.
- Two carried-forward interactions recorded in-file: WP-39 Task 6 now consumes
  the tokio `signal` feature, so `bo F16` must **not** strip it; and
  `specs/WP-36-crypto-deploy-hardening.md` also reaches `bot/src/crypto.rs`, so
  land WP-36 before `bo F11`/`bo F12` if both are in flight.
- Every `bo`/`dp` id was resolved by counting `###` headings and cross-checked
  against the package's declared paths; no mismatches. The known off-by-one
  tally lines (dependencies 26/27, bot-operator-tools 30/31) did not disturb the
  anchors dp F6/F12/F20 and bo F18/F25/F26/F28.

### Checklist ACCEPTED: T3-B7 (17 rows).

### Worker 2 returned: T3-B8 checklist
`planning/checklists/T3-B8-workspace-hygiene-red7-docs.md` written (197 lines).
- **11 rows (5 minor / 6 nit)**: WP-65 = 9 (`dp F4`, `F5`, `F9`, `F17`, `F21`,
  `F22`, `F23`, plus `e F9`, `e F28`), WP-74 = 1, WP-75 = 1. Matches
  `tier2-tier3-plan.md` section 2.1. T3-B4 house format, no line numbers,
  implementer instruction verbatim. All seven `dp` ids resolved against WP-65's
  declared paths with no mismatches (anchors dp F6/F12/F20 held despite the
  26-vs-27-heading tally error).
- **3 rows gated, each kept in the main table AND in the decision-blocked
  table** (never dropped):
  - `dp F9` (web tower-http / gloo-net / gloo-timers pins) on **D-19/WP-64** -
    it is a version-pin row, so doing it before the `[workspace.dependencies]`
    migration means writing the pin twice. The Worker correctly judged the other
    six WP-65 rows as **not** genuinely D-19-dependent (`lazy_static`->`LazyLock`,
    stale template files, CI job, test-module rename) and left them dispatchable
    with sequencing notes only.
  - `WP-74` on the WP-29-Task-5 + WP-30 sequencing gate (WP-30 is
    BLOCKED-ON-USER-RULES-REVIEW, plus D-29/D-40; D-29's outcome may change how
    elimination is described).
  - `WP-75` on the same gate plus WP-74, plus a live render capture, plus a
    ruling.
- **ESCALATED: `WP-75` is not Tier 3.** Five required sections missing outright
  plus a missing worked example = whole-document rewrite; "Reading the Display"
  needs a render captured from a live game state (DB + built binary) so it is
  **not writable from source alone** and the capture was correctly not attempted;
  and it is blocked on an unfiled ruling. A pointer row remains in the WP-74/75
  table so it is not lost. Recommended route recorded: a Tier 2-style spec after
  WP-30 clears, with the render capture as an explicit implementer step.
- **NEW OPEN QUESTION FOR THE USER, not currently in `decisions-needed.md`:** do
  the shipped `rust/game/red7-1/{BASIC_STRATEGY.md,ADVANCED_STRATEGY.md}`
  (surfaced via `Gamer::basic_strategy`/`advanced_strategy`) satisfy
  `RULES_AUTHORING.md`'s mandatory "Strategy Tips" section, given that document
  says "Always include this section"? Recorded in the checklist's
  decision-blocked table; **needs filing as a decision item.**
- Dropped as spec-owned: lords-of-vegas-1's `lazy_static` removal (`d F6`, owned
  by `specs/WP-22-lords-of-vegas-fixes.md` Task 5 - so `dp F21` owns only the
  `rust/lib/color` site), `dp F1`/`F2`/`F3` (WP-64), `dp F24`/`F25` (WP-69),
  `dp F13` (WP-68), `dp F26`/`e F45`/`e F46` (WP-73), red7-1's `e F31`-`e F35`
  (WP-29), `e F30` (WP-30, parked), `e F27` (WP-28).
- Verification corrections honoured: `e F28`'s `.rls.toml` is **not** malformed
  (the reported `build_lib = truetarget` was a `cat`-concatenation artifact);
  `e F9` was recast from a love-letter-2 defect to a workspace-wide convention
  sweep, and red7-1 keeps `mod tests` per WP-29.
- Read-only spot-checks recorded in-file: `lazy_static` still declared in both
  `lib/color` and `lords-of-vegas-1` (WP-22 Task 5 not yet landed);
  lords-of-vegas-1 has `.rls.toml` but no `build-release` (two-file delete);
  a `cargo-deny` CI job exists but is gated on a changed-files filter, so
  `dp F23`'s scheduled currency job is not a duplicate.

### Checklist ACCEPTED: T3-B8 (11 rows).

### Worker 3 returned: WP-42 promoted to a Tier 2 compact spec
`planning/specs/WP-42-websocket-auth-and-filtering.md` written (**142 lines** -
2 over the 140 ceiling, ~22 over the ~120 target; accepted
as-is because the overrun is all substance: the resolved caching design, the
newly-discovered proposal predicate, and the test-rework warning, none of which
compress. Flagged rather than trimmed).
- Structure per WP-47 house style: Problem / Why it's wrong / Required end state
  (3a `ws_handler`, 3b `handle_socket`, 3c `db.rs`, 3d Task B) / Non-goals /
  Regression test cases / Riders. **No line numbers anywhere.** Implementer
  stop-and-report instruction reproduced near the top.
- `## 2` affirms **`ws F59` is correct as written**, verified live, and confirms
  the "identity is already available" claim still holds: `/ws` is registered in
  `router.rs::build_router` **before** `.layer(session_layer)`, and
  `FromRef<AppState> for PgPool` exists in `state.rs`, so **no router or layer
  reordering is needed.**
- **OPEN DESIGN QUESTION RESOLVED - the per-frame async visibility check gets a
  bounded per-socket TTL cache**: plain `HashMap<Uuid, (bool, Instant)>`,
  ~256 entries, **30s TTL, positive and negative results cached identically**,
  no new crate. Rejected with reasons in-spec: unconditional DB hit per frame
  (converts the client-refetch amplification into a Postgres one), connect-time
  membership set (unbounded staleness over an hours-long connection, so a
  mid-connection join never streams), and positive/negative TTL asymmetry
  (premature - both risks want a short TTL). **Accepted staleness: up to 30s
  either direction. Failure mode: fail closed** - a sqlx error resolves to "not
  visible", is warned, and is not cached. Escape hatches: TTL expiry, reconnect
  (`use_websocket` already reopens on `visibilitychange`/`online`), and the
  client-side `bump_game_update` on the user's own action.
- **NEW SCOPE DISCOVERED - WP-42 gains a db.rs predicate WP-47 does not supply.**
  WP-47 provides only the game dispatcher `is_game_visible_to_viewer`; there is
  **no proposal visibility predicate anywhere in `rust/web/src/db.rs`**. The spec
  adds `is_proposal_visible_to_user(pool, proposal_id, viewer_id)` (one
  `EXISTS` over `game_proposal_players`) and rules that proposals have no public
  form, so an anonymous socket receives **no** proposal frames at all without a
  query. Game frames still call WP-47's dispatcher - not forked, SQL untouched.
- **TEST HAZARD the implementer must not paper over:** the existing
  `rust/web/tests/websocket_hygiene.rs` test
  `live_websocket_survives_idle_past_request_timeout` broadcasts a **random**
  `game_id` and asserts the anonymous socket receives it. Under fail-closed
  filtering a nonexistent game is not publicly visible, so **that assertion must
  be reworked** to broadcast for a real all-`'public'` seeded game. The spec says
  in terms: do not "fix" it by weakening the filter. The 101-for-cookie-less
  assertion itself stays.
- Task B (`sub`/`unsub`) is marked separable, sized separately, and explicitly
  must not block Task A. Filter order fixed as: participant-visible OR
  explicitly subscribed AND publicly visible - **a `sub` must not bypass the
  predicate.**
- Non-goals fence out, as instructed: `ws F60`/`F61`/`F62` (they stay rows in
  `planning/checklists/T3-B6-outbound-email-websocket.md`, not to be done
  twice), WP-47's own predicate work, `ws F55` graceful shutdown (already
  shipped by WP-36 - `begin_shutdown`/`drain_ws_tasks` and the shutdown
  `select!` arm are live; do not disturb), the `db.rs` split (`ws F42`), and any
  per-user NATS subject.

### Spec ACCEPTED: WP-42 (Tier 2 promotion complete).

### UNIT CLOSE-OUT - T3-B7, T3-B8 and the WP-42 promotion complete
`planning/checklists/` now holds T3-B1..T3-B8 - **Tier 3 checklist coverage is
complete** for all dispatchable batches. `planning/specs/` gains WP-42, making 26
finalized specs.

Items for the Orchestrator:
1. **File a new decision item:** do red7-1's shipped `BASIC_STRATEGY.md` /
   `ADVANCED_STRATEGY.md` satisfy `RULES_AUTHORING.md`'s mandatory "Strategy
   Tips" section? Blocks any WP-75 spec. Not in `decisions-needed.md` today.
2. **WP-75 needs a Tier 2-style spec, not a checklist row**, written only after
   WP-30 clears, and it must carry the live-render capture as an explicit
   implementer step (DB + built binary required; a read-only session cannot
   produce it).
3. **WP-42's spec widens `db.rs`** with `is_proposal_visible_to_user`. If the
   deferred `db.rs` module split (ws F42) is ever scheduled, it must now also
   wait on WP-42.
4. `dp F9` is the only T3-B8 row genuinely gated on **D-19**; pushing the user
   for D-19 unblocks it and WP-64/WP-65's manifest sequencing.
5. Still outstanding from earlier units: **D-11** (unblocks WP-46, 3 majors),
   **D-15** (reopened), `wd F18`'s missing owner.

## WP-15 IMPLEMENTATION (Lead, 2026-07-26)

Spec `planning/specs/WP-15-seven-wonders-mechanical.md` executed in full (all 9
tasks, findings b F1/F2/F3/F9/F10/F12/F13/F14/F15). One commit, rust-only
(`rust/game/seven-wonders-1/`). Gate: `cargo fmt --all -- --check` clean,
`cargo clippy --workspace --exclude web --all-targets -- -D warnings` clean,
`cargo test -p seven-wonders-1` = 40 lib + 1 contract green (23 baseline + 17
new). TDD red-first for every fix.

Two spec deviations, both test-arithmetic/setup (production code matches the
spec exactly):
1. Task 4 `stored_deal_paid_despite_mid_turn_divergence`: spec expected
   `coins[MICK]==1`, live is 2 - Haven's own Bonus (`coins:1`, Raw DIR_SELF)
   pays MICK 1 coin for his Clay Pit on build. Corrected the assertion to 2
   (pre-fix free-deal value would be 4; the stored deal IS paid).
2. Task 8 `deal_command_selects_between_multiple_deals`: spec's Haven setup
   yields ONE deal, not two - `can_afford_perm`'s early return
   (`lib/cost/src/lib.rs:183`) collapses "same good from either neighbor"
   before the second neighbor is explored. Rebuilt the setup around Gardens
   (Clay:2+Wood:1) with MICK self-providing one Clay (Clay Pool) and both
   neighbors holding Tree Farm (Wood OR Clay) + Clay Pool, which genuinely
   yields two deals (`{LEFT:4}` vs `{LEFT:2,RIGHT:2}`) and exercises `deal 2`
   selection. Intent (b F14 multi-deal coverage) preserved.

No docs/CODING.md or other docs/*.md updates: the spec proposes none.
PORTING_NOTES.md:63-64 DrawDiscard claim is now true at fire time per the b F2
prune; left unedited (out of scope).

### LEAD RULING - `is_proposal_visible_to_user` stays in WP-42, not WP-47
Worker 3 asked whether the new proposal predicate should be moved into WP-47.
**It stays in WP-42.** WP-47's scope is `wd F17` (game details) and `wd F45`
(stats identities); neither finding touches proposals, WP-47 is already
finalized and accepted, and WP-42 is the only consumer. Moving it would reopen a
closed spec to add a predicate with no caller in its own package. The
one-predicate-no-forks rule is still honoured: WP-42 *consumes* WP-47's
`is_game_visible_to_viewer` for game frames unchanged, and only adds a
predicate for a rule WP-47 never encoded.

Worker 3 also confirmed against live source, after drift from snapshot `f8763a5`:
`ws_handler` still takes only `WebSocketUpgrade` + `State<GameBroadcaster>`;
`handle_socket` still wildcard-subscribes and forwards unfiltered; the inbound
arm still discards payloads; `/ws` is still registered before
`.layer(session_layer)` and `/healthz` after; `FromRef<AppState> for PgPool`,
`get_user_from_session` and `validate_session_token(pool, auth_token_id)` all
exist as described; WP-36's `begin_shutdown`/`drain_ws_tasks`/shutdown arm are
live. D-13's line numbers have drifted (`ws_handler` has moved) - as expected,
which is why the spec cites none.

# specs-LOG - Lead session, FINAL UNIT (prevention package + wrap-up)

Date: 2026-07-26. Lead: orchestrate Lead role. Workers: model opus (user
override). Same HARD READ-ONLY CONSTRAINTS as unit 3b: writes only inside
`planning/`; never touch `rust/`; no cargo/build/test/clippy/fmt; no git
mutations; validation by reading only. Another agent is concurrently editing
`rust/` (critical fixes) - expect drift, never write there.

## Plan (final unit)

Four deliverables, one Worker each, serial:
- D1: `planning/CODING-md-amendment-proposal.md` - consolidated CODING.md
  amendment, 5 (maybe 6) rules derived from critical root causes, <150 lines,
  matching the live `docs/CODING.md` voice. Sources: `planning/critical-path.md`,
  specs WP-01, WP-40, WP-47, WP-57, WP-09a.
- D2: `planning/open-decisions-for-user.md` - one table of everything needing
  Michael's input (id / what's blocked / question / recommendation / source).
- D3: append "Unowned / newly discovered" section to `planning/work-packages.md`
  (5 items, new WP numbers, one line + severity each, no specs).
- D4: `planning/README.md` - map of the planning directory, <80 lines.

## Entries (final unit)

- [planned] Plan written. No workers dispatched yet.
- [dispatch W1] D1 CODING.md amendment proposal.
- [W1 done] `planning/CODING-md-amendment-proposal.md` written, 130 lines
  (cap 150), ASCII-only, SIX rules. Sixth rule added: "Deserialized state and
  wire-supplied indices are untrusted; bounds-check at the boundary, once" -
  justified by recurrence across WP-09a, WP-09b (15 crates), plus routed-in
  items from WP-19 and WP-21 (four packages, past the three-package bar), and
  not covered by the no-panic rule (that forbids the panic, this one fixes
  WHERE the check goes). Insertion point: new top-level `## Request-Path
  Invariants` between `## Rust: Error Handling` and `## Leptos: SSR and
  Hydration`. LEAD VERIFIED the insertion point against the live
  `docs/CODING.md` by grep: `## Rust: Error Handling` at :44, the `**DOM access
  in event handlers.**` closing paragraph at :63, `## Leptos: SSR and
  Hydration` at :69 - all exactly as claimed. Voice matches (bold lead-in
  paragraph + fenced example, same as the rest of the file).
  Caveats accepted, not defects: rules 2/3 paraphrase a single representative
  line for the WRONG side (the specs describe those defects in prose and quote
  no source, and the Worker is forbidden from reading `rust/` to quote
  verbatim); rule 4's sketch is the spec's ordered SQL/error sequence in
  comment form rather than invented Rust. Rules 1 and 5 are verbatim from
  WP-01 Task 1 and WP-57 section 3c. All six name the real crate/function.
  ACCEPTED without revision.
  Worker note carried forward: `planning/critical-path.md` is STALE - it still
  lists WP-40, WP-47 and WP-57 as "Spec? no" though finalized specs for all
  three exist in `planning/specs/`. Recorded here; not corrected (outside this
  unit's four deliverables).
- [dispatch W2] D2 consolidated open decisions table.
- [W2 done] `planning/open-decisions-for-user.md` written, 66 lines, ASCII-only,
  ONE table of 34 rows plus a 2-item Notes section. Every row states exactly one
  recommendation. Composition: 4 blessings on already-answered decisions (D-37,
  D-8, D-14, bo F25), 17 unanswered gating decisions (D-11, D-15, D-7, D-9,
  D-10, D-16, D-17..D-25, D-38, D-39, D-40), 1 parity-park row (D-35, covering
  D-26..D-32 + D-34), 5 separate rows for the five egregious parity bugs (a F1
  rtta-2 roll(), b F4 seven-wonders same-turn trade, b F7 both wonder sides,
  e F30 red7 empty-palette tie-break, d F37 modern-art $20/$10 with the Go
  original noted as identical), and 6 new ids (N-1 WP-38 thresholds, N-2 WP-10
  cup_counts, N-3 WP-62 newest-wins, N-4 red7 Strategy-Tips ruling, N-5 BACKLOG
  note, N-6 CODING.md amendment). All Orchestrator-supplied must-include items
  are present.
  D-25 / WP-17 DISCREPANCY RESOLVED (note 2 in the file): both documents are
  partly wrong. The authoritative reading is the one
  `checklists/T3-B3-splendor-libcost-holdem.md` already implements - D-25 gates
  only 3 of WP-17's 8 findings (b F31, ls F39, dp F27, one indivisible
  `lib/cost` consolidation) and the other 9 rows are dispatchable today. So
  `work-packages.md:218`'s package-level `BLOCKED-ON-DECISION(D-25)` label
  overstates the scope, and `tier2-tier3-plan.md` 2.1 is right that the batch
  was dispatchable but its 2.2 blocked roster should list WP-17 as PARTIALLY
  blocked. Neither source document was edited (out of this unit's scope) - the
  reconciliation lives in the decisions file.
  Excluded as already answered (checked against `decisions-needed.md`'s
  ANSWERED/REFINEMENTS index): D-1, D-2, D-3, D-4, D-5, D-6, D-12, D-13, D-33,
  D-36, plus D-41 (Lead applied ruling B; WP-54 is unblocked and an override
  only edits WP-54 Task 2).
  Undeterminable: the deployed Kubernetes version (bo F25) - the cell says so
  rather than guessing a pin.
  Lead review: read the file in full. Table is well formed, cells are one to
  two sentences, ASCII clean, recommendations are all singular and concrete.
  ACCEPTED without revision.
  Known gap, deliberate: D2 was written before D3, so the 5 new WP numbers from
  the "Unowned / newly discovered" section are not in the table. Reviewed - none
  of the five needs a user decision (they are unowned work items, and wd F18's
  refactor collision is an implementer sequencing problem, not a product
  question), so no D2 revision pass is required.
- [dispatch W3] D3 append "Unowned / newly discovered" to work-packages.md.
- [W3 done] `## Unowned / newly discovered` appended to
  `planning/work-packages.md` (file 799 -> 846 lines, 48 appended). Nothing
  pre-existing was restructured - append only, Lead verified by reading the
  tail. Max pre-existing WP number was WP-75, so the five items are WP-76..WP-80:
  - WP-76 notify_game_emails wiring gap for email-originated moves - READY, 1M.
  - WP-77 get_available_bots default bot_name - READY, 1m.
  - WP-78 db.rs module split (ws F42) - DEFERRED, 1m. Blocker list is WP-35, 40,
    42, 45, 47, 49, 50, 52, 53, 59 - the Worker correctly ADDED WP-59 from
    WP-41's own disposition (the Orchestrator's list omitted it) and WP-42 from
    the earlier LOG entry.
  - WP-79 hoist the game-service HTTP call out of the FOR UPDATE transaction
    (wd F18) - READY, 1m, with the four callers named and the WP-40/WP-45
    `restart_core` collision recorded in the entry.
  - WP-80 tic-tac-toe-2 unbounded `players` (f F46) - READY, 1m, pointed at
    WP-09a/WP-09b.
  Format matches the file: `### WP-NN title - STATUS` + Scope/Paths/Severity +
  note lines; severity uses the file's own `c/M/m/n` code vocabulary. Finding
  accounting preserved: WP-76/WP-77 carry 0 findings (spec-time discoveries,
  like WP-74/WP-75) and WP-78/79/80 re-file findings already counted under
  WP-41/WP-53/WP-09, so the 570 sum and the one-package-per-finding invariant
  are both unaffected - the section says so explicitly. No specs written.
  ACCEPTED without revision.
- [dispatch W4] D4 planning/README.md index.
- [W4 done] `planning/README.md` written, 79 lines (cap 80), ASCII-only. Covers
  the directory's purpose (findings live in `../`), a 14-row file map, the
  tiering, execution order and 6 implementer rules. Lead read it in full.
  ACCEPTED without revision.
  Worker found TWO planning files the brief did not name: `planning/BACKLOG.md`
  (the phase-ordered Phase 0-7 prioritized backlog - this, not
  `landing-order.md`, is the global ordering source) and `planning/triage-LOG.md`.
  No expected file was missing. `findings/` is in the parent, not in `planning/`.
  Inventory: 12 top-level files + `specs/` (40 WP specs + notes-conventions.md),
  `checklists/` (T3-B1..B8), `raw/`.
  FOUR tiering corrections the README now records (the brief's file counts were
  right, but `tier2-tier3-plan.md` states ROSTERS, not file counts):
  (1) the Tier 2 roster is 21 packages, not 15 - 13 dispatchable + 8
  decision-blocked; only 14 roster packages got specs, WP-09 later split into
  WP-09a/WP-09b (hence 15 files) and WP-42 was promoted from Tier 3;
  (2) the 8 decision-blocked Tier 2 packages have NO spec: WP-04, 05, 46, 55,
  58, 64, 66, 67;
  (3) the Tier 3 roster is 23 packages and the 8 checklists cover the 16
  dispatchable ones - WP-48, 50, 69, 70, 71, 72, 73 have no checklist;
  (4) `tier2-tier3-plan.md` section 3 names the CODING proposal
  `planning/CODING-amendment-proposed.md`; the real filename is
  `CODING-md-amendment-proposal.md`. Section 2.2 also omits WP-17.
  Tier 1 = 25 confirmed verbatim from section 0.1 (it includes WP-68).
  `landing-order.md` correctly characterised: NOT a global order - six sections
  of verified PAIRWISE sequencing constraints (WP-41 before WP-40; WP-56/WP-59
  overlap table; WP-40 conflicts invisible until WP-54; a recommended cluster
  order; a line-number caveat; and later Leads' chains - auth WP-41 -> WP-36 ->
  WP-34 -> WP-35, WP-37 -> WP-38, WP-59 -> WP-57, WP-09a -> WP-09b, WP-09a ->
  WP-21 T10). README states BACKLOG.md Phase 0-7 as the global order and
  landing-order.md as the override where they disagree.

## Completion (final unit)

Unit COMPLETE 2026-07-26. All four deliverables written and Lead-reviewed;
zero revision passes needed.

1. `planning/CODING-md-amendment-proposal.md` - 130 lines, 6 rules, insertion
   point verified against live `docs/CODING.md`.
2. `planning/open-decisions-for-user.md` - 66 lines, one 34-row table + 2 notes.
3. `planning/work-packages.md` - `## Unowned / newly discovered` appended,
   WP-76..WP-80, 48 lines, finding accounting unaffected.
4. `planning/README.md` - 79 lines, with four tiering corrections.

Compliance: NO file under `rust/` was created, modified or deleted; NO
cargo/build/test/clippy/fmt command was run by the Lead or any Worker; NO git
mutation was run. All writes were confined to `planning/`.

Still unresolved / carried forward:
- `planning/critical-path.md` is STALE (lists WP-40/WP-47/WP-57 as "Spec? no"
  though finalized specs exist). Not corrected - outside this unit's scope.
- `planning/tier2-tier3-plan.md` still carries the wrong CODING-proposal
  filename and omits WP-17 from its blocked roster; the corrections live in
  `README.md` and `open-decisions-for-user.md` note 2 rather than in the plan
  file itself.
- `work-packages.md:218`'s package-level `BLOCKED-ON-DECISION(D-25)` label on
  WP-17 overstates the gate (3 of 8 findings). Left as-is; documented.
- Both doc proposals (`BACKLOG-note-proposed.md`, `CODING-md-amendment-proposal.md`)
  remain UNAPPLIED to `docs/BACKLOG.md` and `docs/CODING.md`.
- 34 decisions await Michael.

## WP-21 implementation - complete (2026-07-26)

Both crates done: cathedral-2 (Tasks 1-6), sushizock-2 (Tasks 7-10). All 12
in-scope findings dispatched (c F22-F28 cathedral-2; c F29-F34 sushizock-2);
red-first confirmed for the TDD tasks. Per-crate test/clippy/fmt green for
both crates.

Gate: `cargo fmt --all -- --check` clean; `cargo clippy --workspace --exclude
web --all-targets -- -D warnings` clean. `scripts/rust-test.sh` could NOT run
- environmental (Docker failed to bind port 15432, already allocated; exit
125). Fallback `cargo test -p cathedral-2` and `-p sushizock-2` both PASS.

Single commit f547238 (8 files: cathedral-2 Cargo.toml/command.rs/lib.rs/
loc.rs/piece.rs/render.rs, sushizock-2 lib.rs, Cargo.lock). Not pushed.

---

## WP-06: lib cmd tools and http - COMPLETE

Date: 2026-07-26. Lead: orchestrate Lead role.

All 5 tasks executed per spec. Findings addressed: ls F19 (MAJOR), ls F20,
ls F21, ls F22, ls F23, ls F26, ls F27, ls F28, ls F29, ls F30, ls F44,
ls F45.

Changes (14 files):
- http.rs: extracted `route()`, SystemError instead of unwrap, 16 MiB body cap
- error.rs: Parse message includes source; new ChildExit variant
- gamer.rs: `renders` returns Result; handle_pub/player_render match not unwrap
- cli.rs: requester error -> SystemError; expect with messages on writes
- repl.rs: prompt returns Option (EOF=quit), refresh_renders on undo/load,
  empty undo stack, is_empty(), panic messages with content
- bot_cli.rs: deleted dead cli/Response, kept Request struct
- rand_bot: removed extern crate, fixed mangled comment
- api.rs: removed redundant serde(default); lock-in test
- test_game.rs: new cfg(test) Gamer impls for transport tests
- Cargo.toml: dev-deps tokio macros/rt, warp test feature

Gate: `cargo fmt --all -- --check` clean; `cargo clippy --workspace --exclude
web --all-targets -- -D warnings` clean; `scripts/rust-test.sh` ALL PASSED
(13 brdgme_cmd tests, full workspace-minus-web, web with DB containers).

Single commit a543120. Not pushed.

---

## WP-13 starship-catan-1 fixes - IMPLEMENTED 2026-07-26

Lead: orchestrate Lead role. Workers: 4 serial dispatches (Tasks 1-4,
Tasks 5-8, Task 9, final gate+commit).

All 9 tasks implemented per spec. All 10 findings dispatched:
- a F11: cannon_transaction surcharge now keys off Resource::Cannon
- a F12: can_lose_module || changed to &&
- a F13: TradeAndBuild buy branch gains astro affordability check
- a F14: buy/sell parsers capped at Int::bounded(1, 99)
- a F15: Sensor peek rendered to peeking player only (gated on current_player)
- a F16: Current turn row uses N::Player(current) not N::Player(viewer)
- a F17: dead code removed (Transaction::gain, Game::next_turn,
  Module::description, join_dice, start_card field)
- a F18: direction-mismatch error interpolates direction.string()
- a F19: last_sectors capped at 5 via LAST_SECTORS_LIMIT const
- a F20: comment-only (BTreeMap shape preserved for serde compat)

Serde spot check: old Colony JSON with start_card field deserializes cleanly
(serde ignores unknown fields, no deny_unknown_fields in crate).

Tests: 40 passed, 0 failed (39 lib + 1 contract). 12 new tests added.
Gate: `cargo fmt --all -- --check` clean; `cargo clippy --workspace --exclude
web --all-targets -- -D warnings` clean; `scripts/rust-test.sh` ALL PASSED.

Single commit 7f4f902. Not pushed.

---

## Planning-doc reconciliation pass - DONE 2026-07-26

Lead: orchestrate Lead role, no Workers. Read-only against `rust/`; writes
confined to `planning/`. Three staleness fixes so a cheap execution model is
not misled:

1. `tier2-tier3-plan.md`
   - Section 3 pointed at `planning/CODING-amendment-proposed.md`, which does
     not exist. Corrected to the real file
     `planning/CODING-md-amendment-proposal.md` and marked the prevention
     package DONE.
   - Added a STATUS UPDATE banner: Tier 2 and Tier 3 planning are COMPLETE.
   - Roster 1.1 (13 Tier 2 packages) marked COMPLETE with a new `Covered by`
     column naming each spec file. WP-09 is recorded as having split, exactly
     as the plan predicted, into `specs/WP-09a-deserialized-state-boundary.md`
     and `specs/WP-09b-game-crate-state-trust-sweep.md` (land 09a first).
   - Roster 2.1 (8 Tier 3 batches) marked COMPLETE with a `Covered by` column
     naming each checklist. T3-B5 was NOT split.
   - Rosters 1.2 and 2.2 (decision-blocked) untouched - still blocked.

2. WP-17 blocked-status disagreement resolved as **partially blocked**.
   `work-packages.md` said `BLOCKED-ON-DECISION(D-25)` at package level while
   `tier2-tier3-plan.md` 2.1 listed it as dispatchable. Authoritative reading
   is `checklists/T3-B3-splendor-libcost-holdem.md`: D-25 gates only
   **b F31, ls F39, dp F27** (the lib/cost keep-or-fold consolidation, one
   change seen from three findings units). The other 5 - **b F30, b F32,
   b F34, b F35, ls F38** - are implementable now. Both files now say
   `PARTIALLY-BLOCKED-ON-DECISION(D-25)` and name the 3 gated findings; new
   subsection `tier2-tier3-plan.md` 2.1a carries the detail.

3. `critical-path.md` NOT rewritten. Added a dated STALE header note: the
   criticals are being executed on a separate branch by a separate agent;
   Tier 2/Tier 3 planning is complete; `planning/README.md` is the current
   entry point; all status columns and line-number citations in that file are
   historical (snapshot `f8763a5`). Landed work packages named from
   `git log --oneline -30` on `master`: WP-01, WP-03, WP-06, WP-13, WP-14,
   WP-15, WP-21, WP-25, WP-36, WP-37, WP-39, WP-41, WP-44 - with this
   `specs-LOG.md` declared authoritative on any disagreement.

No `rust/` file read or written. No cargo/git mutation run. Only
`git log --oneline -30` (read-only) was executed.

---

## 2026-07-26 - Decision-recording unit (Lead)

Michael answered all 34 open decisions. This unit records them authoritatively
and clears the blocks they gate. Read-only against `rust/`; no cargo, no git
mutation. (Commit SHAs are not branch-stable here - a concurrent agent rebases
`rust/` - so this log does not cite them.)

### Entry 1 - answered-decisions record created

- **Created `planning/decisions-ANSWERED.md`.** All 34 rows in the original
  order of `open-decisions-for-user.md`, each stating the ruling plus attached
  constraints/rationale. Carries a "changed rulings" table at the top for the
  five that supersede prior text: D-7 (OVERRULED), D-8 (REFINED), D-15
  (REDESIGNED), D-16 (OVERRULED), D-37 (CORRECTED). Ends with six standing
  constraints extracted from the rulings (dependency-upgrade-first, macro
  restraint, parser simplicity, no Sentry functionality loss, lib/cost tests,
  parity park still in force per game).
- Three verification notes are marked PENDING pending a Worker: the D-16
  full-page-load mechanism, the D-20 concrete crate name, and the e F30
  evidence question.
- **Replaced `planning/open-decisions-for-user.md` with a 4-line stub**
  pointing at the new file, so nothing follows a stale open-questions doc.
- Recorded the coordinator's mid-unit correction on `d F37`: it is simply
  REJECTED, no follow-up. `suits()` already returns canonical top-to-bottom
  order (Lite Metal top, Krypto bottom), so the "value-board order vs array
  index" caveat is resolved and was dropped rather than left open.

### Entry 2 - Worker returned (D-16 / D-20 / e F30 / d F37 verification)

One Worker, read-only against `rust/` and `~/.cargo/registry/`. Results
recorded in `decisions-ANSWERED.md` under "Verification notes":

1. **D-16 mechanism CONFIRMED.** `leptos = 0.8.20`, `leptos_router = 0.8.14`
   (lock-resolved). `leptos_router-0.8.14/src/location/mod.rs` reads the DOM
   `rel` attribute, splits on space/tab, and returns early for `external` (or
   `download`) - so `rel="external"` DOES opt out of client-side routing.
   Crucially, a **plain `<a>` is NOT enough**: interception is a window-level
   click listener (`src/location/history.rs`) walking `composed_path()` for
   any `HtmlAnchorElement`, regardless of `<A>` vs `<a>`. `<A>` has no `rel`
   prop, so use `attr:rel="external"` (spreading is already proven in
   `rust/web/src/app.rs`) or a plain `<a href rel="external">`.
   **NEW GAP recorded against WP-55:** three `/login` navigations go through
   `use_navigate` and touch no anchor, so no `rel` can cover them -
   `components/layout.rs` (post-logout), `settings.rs`, `admin.rs`. WP-55
   must convert those to hard navigations too.
2. **D-20 name settled: `rust/lib/game_bin`, `[package] name =
   "brdgme_game_bin"`.** Convention is snake dirs under `lib/`/`tools/` with
   `brdgme_<snake_dir>` package names (10/10); hyphens are the game-crate
   convention. `game-bin`/`brdgme-game-bin` REJECTED. Also noted: the 4 bins
   live inside each game crate as `[[bin]]` targets today, so per-game wrapper
   bin crates are a structural change.
3. **e F30 CONDITION SATISFIED -> FIX NOW.** `red7-1/DATA_DOCS.md` documents
   the second tie-break ("then by the highest card overall in the palette")
   and official rules agree; the code never implements it. Not subjective.
   Cause: `leader()` only ever receives already-filtered winning sets, so the
   full palette is unreachable. D-29's half stays parked.
4. **d F37 rejection corroborated.** `end_round` scans `suits()` in declared
   order with a strict `>`; `suits()` returns
   `[LiteMetal, Yoko, ChristineP, KarlGitter, Krypto]`, which Michael confirms
   IS canonical top-to-bottom. Caveat dropped, as instructed.

### Entry 3 - decisions-needed.md updated

- New top-level banner **"ANSWERED - 2026-07-26 session (ALL remaining
  decisions)"** after the REFINEMENTS index: lists what closed, a
  supersession table for the five changed rulings (D-7/D-8/D-15/D-16/D-37),
  the five parity outcomes, and the five new standing constraints.
- Per-item `ANSWERED (2026-07-26)` blocks added in place for: **D-7**
  (OVERRULED, explicitly naming the superseded option A), **D-8** (REFINED -
  restart resolves to latest non-deprecated; the no-op fallback is explicitly
  NOT the restart answer), **D-9**, **D-10** (option A + the two visible
  links, headers unchanged), **D-11**, **D-14** (CONFIRMED non-goal; 6-digit
  code kept, correcting the 2026-07-25 answer's "confirmation link" wording),
  **D-15** (REDESIGNED - parser-first + small escape-hatch set; the "A-plus"
  reserved-list recommendation explicitly superseded; WP-59 Task 14 ungated
  but its content must be REWRITTEN), **D-16** (OVERRULED to option B, with
  the full `rel="external"` verification and the `use_navigate` gap),
  **D-17..D-25**, **D-37** (CORRECTED to `{{lbrace}}`), **D-38** (+ the
  parser-simplicity constraint on WP-04 generally), **D-39**, **D-40** (+
  WP-81 named, + the clean-slate rationale), **D-35** (park kept, per-game
  priority order recorded).
- **Group C gained a standing-process banner** from D-17: upgrade all
  dependencies to latest FIRST, then decide - binding on WP-64..WP-73.
- **Group D banner rewritten.** New "RESOLVED 2026-07-26" ruling table for the
  five egregious candidates ahead of the original flagging table, which is
  retained as historical context and explicitly declared superseded where the
  two disagree (notably b F4's "asymmetric by seat" reasoning and d F37's "not
  producible by any rulebook" claim). b F4 carries the user's binding
  correction (7 Wonders resources are NOT depleted by trade) plus the residual
  simultaneity question, parked not scheduled.

### Entry 4 - work-packages.md updated

- New 2026-07-26 banner near the top: all decisions answered,
  `BLOCKED-ON-DECISION` declared EXTINCT (with a note on the legend line that
  any surviving occurrences are historical narrative), the five standing
  constraints, and the parity outcomes.
- **16 packages flipped to READY**, each heading rewritten and each gaining an
  inline answer note: WP-04 (D-38 + parser-simplicity constraint), WP-05
  (D-39), WP-17 (D-25 - `PARTIALLY-BLOCKED` retired, + the lib/cost testing
  constraint), WP-46 (D-11 + the web-only-players rationale), WP-48 (D-7),
  WP-50 (D-9), WP-55 (D-16), WP-58 (D-10), WP-64 (D-19), WP-66 (D-17), WP-67
  (D-18 + no-functionality-loss as an acceptance criterion), WP-69 (D-23),
  WP-70 (D-21), WP-71 (D-22), WP-72 (D-24), WP-73 (D-20 + the verified
  `rust/lib/game_bin` / `brdgme_game_bin` name + the macro constraint).
- **Scope changes recorded:** WP-48 **SHRANK** (no `--redact-private`, no
  user-facing path; wd F7 becomes "make the export admin-only"); WP-55
  **GREW** (three `use_navigate` `/login` redirects in `layout.rs`,
  `settings.rs`, `admin.rs` must also become hard navigations - `rel` cannot
  reach them); WP-58 **GREW** (two visible links, additional to the headers).
  WP-59's Task 14 is ungated but flagged **rewrite, do not execute as
  specced** for D-15's parser-first design.
- **New section + WP-81** "dead per-game stats machinery removal", READY,
  scope `c F12` + `e F39` + `e F40` re-homed from WP-20/WP-30. Coverage
  bookkeeping written into all three package entries (WP-20 5->4, WP-30 5->3,
  WP-81 +3; the 570 sum and one-package-per-finding invariant hold).
- **Parity outcomes written into the owning packages:** WP-12 heading + note
  (`a F1` FIX NOW, other 8 parked), WP-16 heading + note (`b F7` FIX NOW;
  `b F4` re-parked with the user's binding correction and the residual
  simultaneity question), WP-26 note (`d F37` REJECTED, no follow-up), WP-30
  heading + note (`e F30` seat-order FIX NOW with the DATA_DOCS.md evidence
  and the `leader()` cause; D-29 half still parked).
- WP-62 gained the answered `bo F25` rider (k8s v1.36.0 -> pin `v1_36`,
  confirm the flag exists at fix time).
- WP-75's "REQUIRES A LEAD/USER RULING" is answered by N-4, with the
  bot-difficulty-tiering rationale that the `RULES_AUTHORING.md` amendment
  must carry.
- New standing-process banner on the "Dependencies and build" section
  (upgrade to latest FIRST), plus a totals block recording the 16 flips, the
  1 new package, the 3 scope changes and the N-1..N-4 blessings.
- WP-74's sequencing note de-referenced from the stale
  `BLOCKED-ON-DECISION(D-29, D-40)` wording.

### Entry 5 - README.md updated, unit complete

- New STATUS 2026-07-26 banner at the top: all 34 decisions closed,
  `decisions-ANSWERED.md` is the entry point and outranks every other file,
  `BLOCKED-ON-DECISION` extinct, 16 flips + WP-81, the three scope changes,
  the park kept with its three released carve-outs, the five standing
  constraints, and an explicit "specs for the newly unblocked packages are not
  yet written - that is the next unit".
- File map: `decisions-ANSWERED.md` added as the first row and marked
  authoritative; `open-decisions-for-user.md` re-described as a stub;
  `decisions-needed.md` row updated to D-1..D-41 with both answer sessions.
- Execution order step 1 rewritten from "get the decisions answered" to
  "DONE", naming the three carve-outs that may be picked up out of the park.
- Tiering corrections updated: the 8 Tier 2 and 7 Tier 3 formerly
  decision-blocked packages are now READY-but-unspecced, plus WP-17's 3 D-25
  rows, WP-81, and the three carve-outs.
- Implementer rules: N-5/N-6 recorded as approved with N-5's re-read-and-
  renumber condition, and a rule that `decisions-ANSWERED.md` outranks specs
  written under the five superseded recommendations.

**UNIT COMPLETE.** Files changed, all inside `planning/`:
`decisions-ANSWERED.md` (new), `open-decisions-for-user.md` (reduced to a
stub), `decisions-needed.md`, `work-packages.md`, `README.md`, `specs-LOG.md`.
No file under `rust/` was written; no cargo, build, test, clippy, fmt or git
mutation was run. Verified: `grep -n "BLOCKED-ON-DECISION" work-packages.md`
returns only the legend line (now annotated EXTINCT), the new banner, the
totals blocks, and one historical narrative line in WP-17's note.

---

## 2026-07-26 - Lead unit: WP-04 + WP-05 specs (D-38 / D-39 unblocked)

Lead batch covering the two packages unblocked by D-38 and D-39. Workers run
on opus, one at a time, serially. Writes confined to `planning/`; nothing
under `rust/` touched; no cargo/git mutation run at any tier.

### Worker 1 - WP-04 (lib/game parser design) - ACCEPTED

Wrote `specs/WP-04-game-parser-design.md` (176 lines - over the ~120 Tier 2
cap, accepted as the deliberately large package of this batch; dense, not
padded). Covers lg F7 (major), lg F13, lg F14 (minor), lg F17, lg F19 (nit).

Findings status re-derived against LIVE source, not the `f8763a5` snapshot:
**all five confirmed still present and correct as written.** Specifically
`impl Parser for OneOf` and the `CommandSpec::OneOf` arm are the only
non-zero-offset construction sites and are provably 0 by induction;
`CommandSpec::expected`'s `Doc` arm still returns `vec![name.clone()]` and its
`Many` arm still returns the bare inner `expected`; `suggest_spec`'s
`Spec::Token` arm still uses `to_lowercase` while `Token::parse` uses UniCase.
WP-03's fixes (progress guards, `Many` max check, suggest dedup, `Int`
saturating range) are **already applied live**, so the spec assumes them and
lists them as non-goals.

Three substantive outcomes:

1. **D-38(iii) NARROWED - recorded so nobody widens it back.** "Adopt UniCase
   in `suggest`" is correct **only for the `Spec::Token` arm**. `Enum::parse`
   folds via `shared_prefix`, which compares per-char `char::to_lowercase`, so
   converting the `Spec::Enum` / `Spec::Player` arms to UniCase would *create*
   a divergence rather than remove one. The spec restricts UniCase to the
   Token arm (`UniCase::to_folded_case()`, unicase 2.9, already a direct dep)
   and requires a comment on each remaining arm naming the parser it mirrors.
2. **THIRD `expected()` DIVERGENCE FOUND - not named in any finding.**
   `AfterSpace::to_spec()` is `Chain([Space, inner])`, typed
   `AfterSpace::expected` returns `inner.expected`, but the
   `CommandSpec::Chain` arm returns the *first* spec's, i.e. `["whitespace"]`.
   The new `expected()` parity assertion required by D-38(ii) fails on it.
   **LEAD RULING: fix it as `Chain` returning the first non-`Space` spec's
   `expected`**, with a fallback to the first spec if a chain is all-`Space`
   so the function stays total. It falls inside D-38(ii) ("align spec
   `expected()` to typed behaviour"), so no new decision is needed. The
   rejected alternative - changing typed `AfterSpace::expected` to return
   `["whitespace"]` - aligns the two by making user-facing text worse.
   Spec edited by the Lead to state this as a ruling, not an open question.
3. **Standing constraint (D-38, keep the parser obvious) HONOURED.** Offset
   propagation is one `pub(crate) fn add_offset` applied at four call sites
   (`chain_2`; the `Chain3`/`Chain4` tail calls; the `CommandSpec::Chain` arm;
   the two `Many` min-check errors). No wrapper type, no trait change, and
   **neither `OneOf` body is edited** - they already emit
   `offset: error_consumed`; the WP only makes that number non-zero. The
   Worker judged no conflict with D-38 here and the Lead agrees.

D-38(iv) (spec depth guard) recorded in Non-goals with its no-trust-boundary
reason, and as the package's single rider row. No line numbers cited; the
standard "read the named function, STOP on mismatch" warning box is present.

### Worker 2 - WP-05 (lib/color dead parse API) - ACCEPTED

Wrote `specs/WP-05-color-dead-parse-api.md` (**128 lines** final - the Worker
trimmed from 137 after the Lead's first read; all six sections and the
no-live-caller evidence survived the trim, re-verified). Covers ls F12 (major), ls F13/F14/F15 (minor),
ls F16/F17/F18 (nit). Deletions are specified **by item name only, never by
line range** - the explicit guard against the two earlier specs whose delete
ranges would have destroyed live code.

**D-39's no-live-caller precondition is SATISFIED.** `rg` over the whole
`rust/` tree for `from_hex`, `parse::<Color>`, `Color::from_str`, `named(`,
`regex`, `lazy_static` found: the only `from_hex` callers outside lib/color are
inside `rust/lib/markup/src/transform.rs`'s `#[cfg(test)] mod tests`; the only
`Color::from_str` uses are lib/color's own tests; `named()` is private and
reached only from `impl FromStr for Color`; `regex`/`lazy_static` appear in no
other manifest. Deletion endorsed.

Cleanup surface named in the spec, per the Lead's requirement:
- `lib.rs`: `Color::from_hex`, `impl FromStr for Color`, private `fn named`,
  the three now-dead `use`s, and the two tests of the deleted API (delete, do
  not port).
- `rust/lib/color/Cargo.toml`: drop `lazy_static` and `regex`. **There is no
  `[workspace.dependencies]` table in `rust/Cargo.toml`** - verified - so
  there is no second manifest edit. `Cargo.lock` is not hand-edited.
- **`rust/lib/color/src/error.rs` SURVIVES UNCHANGED.** `ColorError` is still
  returned by `NamedColor::from_str` in `palette.rs`, so it stays exported.
  (The Lead's brief speculated it might die entirely; reading the source
  disproved that.)
- Fallout: `rust/lib/markup/src/transform.rs` test-only `from_hex` calls become
  `Color { .. }` literals - same values, no behaviour change.

Two refinements worth carrying forward:
- **F14 is disposed of, but not literally "by deletion" as D-39's wording
  implies.** Deleting `named()` removes one of the three alias tables; the
  remaining two are not independent - markup's `resolve_named` **delegates to**
  `NamedColor::from_str`, so one table effectively remains and cannot diverge.
  Net effect matches D-39's intent. The spec forbids touching either survivor.
- **F15's numbers are corrected in the spec** per the verification file: 379
  `Color {` literals / ~2,000 literal lines (not ~3,000), and a `const fn rgb`
  rewrite lands the file near ~2,300 lines (not ~400). The palette rewrite is
  specified as a value-preserving scripted transform with the existing
  `gate_contrast_all_themes` / `gate_cvd_simulation` tests as the safety net.

F13 (`mono`) fix is concrete and derived from the live body: mean in `u16`
with round-to-nearest, existing `>= 128` boundary kept, boundary tests at
127/128 required. F16/F17/F18 are rider rows.

**UNIT COMPLETE.** Files changed, all inside `planning/`:
`specs/WP-04-game-parser-design.md` (new), `specs/WP-05-color-dead-parse-api.md`
(new), `specs-LOG.md`. No file under `rust/` was written; no cargo, build,
test, clippy, fmt or git mutation was run at any tier.

**For the Orchestrator:** one Lead ruling was made inline (WP-04's third
`expected()` divergence, `CommandSpec::Chain` vs `AfterSpace`) under D-38(ii);
it needs no new decision but should be noticed. Nothing else is blocked.

---

## WP-46 sweep delivery semantics (Worker, 2026-07-26)

Wrote `specs/WP-46-sweep-delivery-semantics.md` (229 lines) in WP-47 house
style. Read live `rust/web/src/email/sweep.rs`, `email/outbound.rs`,
`proposals.rs` (mailer + sweep helpers), `db.rs`
(`delete_expired_unverified_emails`), `web/migrations/014_email_play.sql`, plus
WP-51/WP-57/landing-order 6.2 for fencing. Nothing under `rust/` was written; no
cargo/git command was run.

**All 12 findings verified CORRECT against live code. None rejected.** Live
`sweep.rs` matches the snapshot the findings cite (`FOR UPDATE SKIP LOCKED` on
`fetch_all(pool)`, `send_reminder` returning `true` on both suppression paths,
`should_email_recipient` = `email.is_some() && !is_bot && turn_emails_enabled`,
`fetch_auto_decline_candidates` keyed on `gp.created_at`,
`cancel_proposal_for_expiry` reading owner *after* the UPDATE with
`.ok().flatten()`, no `LIMIT` anywhere, no `processed_webhook_events` DELETE in
the tree - only the INSERT at `inbound.rs::mark_event_processed`).

**Brief correction carried into the spec.** The Worker brief said D-2 =
"mark-before-do". `decisions-needed.md` D-2's ANSWERED block says the opposite:
mark **after** success, claim-then-send in a real transaction, never mark on a
retryable skip. Mark-before-do is the *bug*, not the fix. The spec implements
per-row `SELECT ... FOR UPDATE SKIP LOCKED` inside a transaction, send, then
mark-and-commit / rollback-on-Retry, with a three-way
`ReminderOutcome { Sent, PermanentSkip, Retry }`.

**D-11 resolution shape.** `should_email_recipient` is left alone (turn mails in
`notify.rs` depend on it; WP-60 owns `outbound.rs`). The reminder gate becomes a
local check on a new `EmailRecipient.reminder_emails_enabled` field - one
`outbound.rs` edit, flagged in the spec as the single exception to WP-60's
fence.

**Ordering constraints surfaced (appended to `landing-order.md` 6.2):**
WP-51 -> WP-46. WP-51 rewrites `send_reminder`'s body, the six
`RealInviteMailer` methods and the five `spawn_*` loops; WP-46 changes
`send_reminder`'s return type and gate, splits `send_invite` into an awaited
core plus spawn wrapper, and adds a `resend` parameter to
`spawn_invite_auto_decline_sweep` (hence to `spawn_periodic_sweeps`, which
WP-38 also parameterises). Either order compiles; second to land rebases.

WP-57 (inbound side of D-2) and WP-76 (email moves never call
`notify_game_emails`) were checked: **no collision.** WP-46 touches `inbound.rs`
not at all; the `processed_webhook_events` prune lives in `sweep.rs` + `db.rs`,
which is exactly where WP-57's non-goals section assigns it.

---

## 2026-07-26 - T2-B5 Lead (WP-46 / WP-50 / WP-58 batch)

**WP-46 ACCEPTED.** `specs/WP-46-sweep-delivery-semantics.md`, 229 lines.
Over the ~120 Tier 2 cap but judged proportionate: 12 findings, 3 majors, 3
source files, ~19 lines per finding vs WP-47's 60 lines for 2 findings. No
padding found on read-through; every section is load-bearing. Shape checks
passed: no line numbers, WP-47 header block, six sections in order, riders
table present, all 12 findings endorsed as correct with the code cited.

**Lead brief was WRONG, Worker was right - recorded so it is not re-introduced.**
The Lead brief paraphrased D-2 as "mark-before-do". That is the **bug**, not
the ruling. `decisions-needed.md` D-2 ANSWERED = at-least-once, **do not mark
`sent` on skip paths** - i.e. claim-then-send, mark only after success. The
spec implements the correct reading. `work-packages.md`'s WP-46 bullet
("Mark-before-do in every sweep, ...") is a list of **defects**, not of fixes;
it reads ambiguously and misled the brief.

Ordering constraint **WP-51 -> WP-46** added to `landing-order.md` 6.2.

---

## 2026-07-26 - WP-50 Worker (email canonicalization)

**Written:** `specs/WP-50-email-canonicalization.md`, 170 lines. Over the ~120
target after two trim passes; three moving parts (helper + 6 server boundaries
+ 2 client boundaries + migration) and ~20 lines are verbatim SQL/Rust quoted
on purpose - the implementer is a cheap model and the `RAISE EXCEPTION` block
must not be improvised. Flagged for the Lead rather than cut further.

**All four findings (ws F9, wd F37, F60, F72) confirmed correct against live
source.** None rejected.

**Lead brief fact CORRECTED (record so it is not re-introduced).** The brief,
following the ws F9 verification row, says `user_emails.email` is a "text
primary key (migration 005)". It is not. Migration `005_login_confirmations.sql`
creates `login_confirmations (email TEXT PRIMARY KEY, ...)`. `user_emails`
(migration `001`) has `id uuid` as PK and a plain case-sensitive
`UNIQUE (email)` (`user_emails_email_key`, 001:274-275, approximate). Practical
consequence: the lowercasing UPDATE is much safer than feared. Grep of
`rust/web/migrations/*.sql` finds **no** FK referencing `user_emails.email`,
`user_emails.id` or `login_confirmations.email`; the only FK on the table is
`user_emails_user_id_fkey -> users(id)`.

**Duplicate-row disposition: ABORT, never auto-resolve.** Migration 023 runs a
`DO $$ ... RAISE EXCEPTION $$` block listing every colliding canonical address
*before* the UPDATE. Rationale: two rows differing only by case are usually
owned by two different `users` rows, and collapsing them is an account merge
(games, ratings, friendships) that no migration can do deterministically. D-9
says surface the risk "once, deliberately" - a loud, named failure is exactly
that. A pre-flight operator query is included in the spec.

`login_confirmations` gets a blanket `DELETE FROM` instead, because its PK *is*
the address so lowercasing could collide there too; rows are 1-hour ephemeral
codes the app already GCs opportunistically.

**Policy chosen:** `raw.trim().to_lowercase()` - Unicode lowercase, not ASCII,
so it agrees with Postgres `lower()` in the new functional index. The helper
does not reject empty; each caller keeps its own existing validation and runs it
after canonicalizing.

**Scope held:** the `LOWER(email) = LOWER($2)` sites in `email/inbound.rs`
become redundant but are left alone (WP-56/WP-59 own that file). The
`emails add/confirm/active/remove` call sites in `email/commands.rs`
(`find_email_owner` / `insert_unverified_email` / `mark_email_verified` /
`set_primary_email`) are deleted by WP-56 Task 4 and marked no-op.

**Ordering constraints added** to `landing-order.md` 6.4: WP-50 is independent
of WP-56/WP-59, but **WP-50 and WP-56 both add a new migration and both assume
022 is the highest** - whichever lands second must renumber.

### Lead acceptance - WP-50

**ACCEPTED.** 170 lines; over the ~120 cap but ~20 of those are verbatim
SQL/Rust the spec deliberately quotes so a cheap model does not improvise the
`RAISE EXCEPTION` block. Shape checks passed.

**A finding-verification row was WRONG and the Worker corrected it.** Recorded
so nobody re-imports the error: `findings/verification/web-server.md`'s ws F9
row says "text PK (migration 005)". That is a **conflation**. Migration
`005_login_confirmations.sql` gives **`login_confirmations`** a `TEXT PRIMARY
KEY` on `email`; **`user_emails`** has a `uuid` PK plus a plain case-sensitive
`UNIQUE (email)` from migration 001. No FK anywhere references
`user_emails.email`, `user_emails.id` or `login_confirmations.email` - the only
FK on the table is `user_emails_user_id_fkey -> users(id)`. The lowercasing
UPDATE is therefore materially safer than the verification row implies.

**For the Orchestrator's awareness (no decision requested):** the migration's
duplicate disposition is **abort, never auto-resolve** - a `DO $$ ... RAISE
EXCEPTION $$` block listing every colliding canonical address, run before the
UPDATE, with a pre-flight operator query supplied. Rationale: two rows differing
only by case normally belong to two different `users`, so collapsing them is an
account merge (games, ratings, friendships), which is not something a migration
can decide. This satisfies D-9's "surface the collision risk once, deliberately,
during the migration", but it does mean a deploy **blocks** if a collision
exists. `login_confirmations` is exempt - blanket `DELETE FROM`, since its rows
are 1-hour ephemeral codes the app already GCs.

---

## WP-58 - RFC 8058 one-click unsubscribe (Worker, 2026-07-26)

Wrote `planning/specs/WP-58-unsubscribe-rfc8058.md` (217 lines). Covers wfe F3
(major) and wfe F25 (minor) plus D-10's grown scope: an HTTPS one-click
endpoint, two visible links, a new migration and a router change.

**Both findings verified correct against live source; neither is stale.** F3's
two header sites both exist: `render_game_email` (`email/render.rs`) emits
`List-Unsubscribe`/`List-Unsubscribe-Post` unconditionally, and
`send_rules_reply_response` (`email/inbound.rs`) hand-builds the same pair in a
`BTreeMap`. F25 verified: `subscribe_toggle` is called only from the game-scoped
`dispatch_email_command`; `dispatch_standalone_server_command` special-cases
`new` and `bump` and then delegates to `dispatch_settings_standalone`, whose
rejection string is "new, list, name, colors, theme, emails on/off, settings,
help".

**F3's own recommendation is superseded, not wrong-on-facts.** It asks for
`unsubscribe@` detection in the settings fallback. D-10 chose the HTTPS
endpoint, and WP-56's spec already routes `unsubscribe@brdg.me` to the ignore
arm and tells WP-58 not to add the special case. The spec deletes the mailto URI
entirely rather than trying to honour it.

**Type-discriminator mechanism chosen: a new public
`enum EmailKind { Turn, GameEvent, Reminder, Invite }` in `email/render.rs`,
passed as an explicit 7th parameter to `render_game_email`
(`unsubscribe: Option<Unsubscribe<'_>>`, where `Unsubscribe { kind, token }`).**
No inference anywhere. Reasons: (1) `render_game_email` is the single choke
point that emits the header, so the kind must reach it regardless; (2)
`notify.rs::NotifyKind` is private and covers only 3 of the ~6 real email
families, so it cannot serve; (3) an `Option` makes the transactional case
(every `inbound.rs` reply) fall out naturally - `None` means emit neither
header nor links, which is also the fix for the duplicated second header site.
`EmailKind` carries `slug()`, `from_slug()`, `pref_column()` and `link_label()`
so the mail and the endpoint share one mapping. Column mapping follows D-11:
`Reminder -> reminder_emails_enabled`, `Turn`/`GameEvent -> turn_emails_enabled`,
`Invite -> invite_emails_enabled`.

**Second header site disposition:** `send_rules_reply_response` is NOT rerouted
through `render_game_email` (its body is bespoke pre-rendered HTML, not
`EmailContent` blocks); its two header inserts are simply deleted.

**Token design:** new per-user `users.unsubscribe_token` (new migration, next
free number) + partial unique index, lazily populated by a new
`outbound::ensure_unsubscribe_token` copied from `ensure_email_token` and
reusing the private `generate_email_token`. Deliberately **not** WP-56's
`users.settings_email_token`: the one-click URL is POSTed by Gmail's
infrastructure and GET-fetched by link scanners, so the credential is
semi-public and must authorise nothing but "set one named preference column to
`false`". The db helper only ever writes `false`, so replay cannot re-subscribe.
GET renders a confirm form and never mutates; POST is the RFC 8058 target.
Endpoint mounted **before** `session_layer` beside `/api/webhooks/resend` - the
session layer attaches a session but never redirects, so D-10's "no auth
redirect" holds, and unlike `/healthz` this handler needs Postgres anyway.

**Ordering constraints surfaced (appended to `landing-order.md` 6.5):**
WP-59 -> WP-58; WP-56 -> WP-58; WP-51/WP-46/WP-38 vs WP-58 rebase-not-fork on
`notify.rs`/`sweep.rs`; WP-58 added to the 6.4 migration-numbering collision
note (now WP-50, WP-56, WP-58); WP-58 takes a second documented exception to
WP-60's `outbound.rs` fence (one new fn), alongside WP-46's.

**No decision requested.** One thing the Lead may want to confirm: the spec
drops `List-Unsubscribe` from all inbound command replies (transactional). That
is a deliberate reduction in header coverage versus today, justified by those
mails being user-initiated replies rather than bulk mail.

### Lead acceptance - WP-58

**ACCEPTED.** 214 lines; over the ~120 cap, and the largest overrun in this
batch, but D-10's addition turned a 2-finding package into endpoint + GET/POST
split + migration + token helper + a signature change across ~13 production and
~12 test call sites. Shape checks passed; no line numbers; both findings
endorsed as correct.

**Lead ruling on the one flagged item: the `List-Unsubscribe` reduction is
CORRECT - keep it.** Dropping the header from inbound command replies is right,
not a regression. Those mails are transactional (a direct reply to a message the
user just sent), and RFC 8058 / the Gmail and Yahoo bulk-sender rules that D-10
cites apply to bulk and notification mail. Attaching a one-click unsubscribe to
a transactional reply is what invites an accidental unsubscribe. The
`Option<Unsubscribe>` parameter shape makes this fall out as `None` at those
call sites, which also disposes of the duplicated second header site in
`inbound.rs` (`send_rules_reply_response`) by deletion rather than by rerouting
bespoke pre-rendered HTML through the choke point. Both are the right calls.

**Type discriminator (D-10's explicit "how does the link know its type"
requirement) is satisfied:** public `EmailKind { Turn, GameEvent, Reminder,
Invite }` in `email/render.rs`, passed **explicitly**, never inferred, carrying
`slug()`/`from_slug()`/`pref_column()`/`link_label()` as the single shared
mapping between the mail and the endpoint. Column choice follows D-11:
Reminder -> `reminder_emails_enabled`, Turn/GameEvent -> `turn_emails_enabled`,
Invite -> `invite_emails_enabled`.

**wfe F3's recommendation is superseded, not wrong.** It asks for `unsubscribe@`
detection in the inbound settings fallback; D-10 chose the HTTPS endpoint and
WP-56 routes that recipient to the ignore arm. The spec forbids adding the
inbound case. Recorded so nobody re-adds it from the finding text.

---

## 2026-07-26 - T2-B5 Lead: UNIT COMPLETE

Three specs written and accepted, all inside `planning/`:
`specs/WP-46-sweep-delivery-semantics.md`, `specs/WP-50-email-canonicalization.md`,
`specs/WP-58-unsubscribe-rfc8058.md`, plus `landing-order.md` sections 6.2
(amended), 6.4 and 6.5, plus this log. No file under `rust/` was written; no
cargo, build, test, clippy, fmt or git mutation was run at any tier.

**Cross-batch item the Orchestrator must sequence: three packages each add a
migration and each assume `022` is the highest** - WP-50 (`canonical_emails`),
WP-56 (`users.settings_email_token`), WP-58 (`users.unsubscribe_token`).
Whichever land second and third must renumber. Recorded in `landing-order.md`
6.4/6.5.

**Two corrections to source material surfaced by this batch, both recorded
above:** `work-packages.md`'s WP-46 bullet reads as if "mark-before-do" were the
D-2 ruling when it is the defect; and `findings/verification/web-server.md`'s
ws F9 row conflates `login_confirmations`' text PK with `user_emails`, which has
a uuid PK.

## WP-48 export/import (worker session, 2026-07-26)

**Read:** `decisions-ANSWERED.md` D-7 (OVERRULED), `work-packages.md` WP-48
(scope shrank), `findings/web-domain.md` (wd F7, F10-F13 - no
`findings/verification/web-domain.md` exists), and the LIVE source:
`rust/web/src/game/export.rs`, `rust/web/src/game/import.rs`,
`rust/web/src/router.rs`, `rust/web/src/components/game.rs`,
`rust/web/src/admin.rs` (`require_admin`), `rust/web/src/db.rs`
(`is_user_admin`), `rust/web/src/bin/import_game.rs`,
`rust/web/migrations/001_initial_schema.sql`, `rust/web/tests/ssr_pages.rs`.

**Wrote:** `specs/WP-48-export-import.md` (129 lines).

**Answer to the "is there a user-facing export path to remove?" question:
NO - nothing to delete.** The sole entrypoint is
`GET /admin/games/{id}/export` (registered in `router.rs::build_router`) ->
`export.rs::admin_export_game`, which already runs
`get_user_from_session` -> `validate_session_token` -> `db::is_user_admin`
(401/401/403). The one UI link lives in `components/game.rs` behind
`<Show when=viewer_is_admin>`. No leptos server fn, no CLI export binary, no
other referrer of `build_export_bundle`/`ExportBundle` outside `import.rs` and
`bin/import_game.rs`. F7's access-control half is therefore already satisfied;
the residual work is a module-doc rewrite only.

**Live code contradicted a finding:** wd F13's recommendation to "leave
`last_turn_at` at the column default/NULL" is impossible -
`game_players.is_turn_at`/`last_turn_at` are `timestamp NOT NULL` with no
default (migration 001). Spec substitutes `bundle.game.updated_at` for both.
Also confirmed all `update_*_updated_at`/`is_turn_at`/`last_turn_at` triggers
are BEFORE **UPDATE** only, so F12's explicit-INSERT fix is valid.

### 2026-07-26 - Lead (WP-48/WP-55 batch): WP-48 ACCEPTED

Sanity-checked the load-bearing claim by reading live source directly
(read-only): `rust/web/src/router.rs` registers exactly one export route,
`GET /admin/games/{id}/export` -> `game/export.rs::admin_export_game`, and that
handler's first three checks are `get_user_from_session` (401),
`validate_session_token` (401), `db::is_user_admin` (403), in that order,
before `build_export_bundle` is ever called. Matches the Worker's trace
verbatim. **No user-facing export path exists; nothing to delete.**
`specs/WP-48-export-import.md` accepted as landed (129 lines, house style).
Next: WP-55.

### 2026-07-26 - Lead (WP-48/WP-55 batch): WP-55 ACCEPTED

Sanity-checked the load-bearing claims by reading live source directly
(read-only): `grep -rn '"/login"' rust/web/src/` returns **exactly five** hits
and no more - `components/layout.rs` (logout `navigate`, and the `<A>` nav
link), `admin.rs` (anonymous redirect `navigate`), `settings.rs` (anonymous
redirect `navigate`), `app.rs` (the `index-cta` `<A ... attr:class>`). No sixth
site, no server-side `/login` redirect. `web-sys` in `rust/web/Cargo.toml` is a
**non-optional** dependency carrying both `"Window"` and `"Location"` features,
so the specced `hard_navigate` needs no `#[cfg]`; `app.rs::set_theme_client` is
the existing `let Some(window) = web_sys::window() else { return; }` idiom the
helper copies. All three `use_navigate` sites confirmed.

`specs/WP-55-turnstile-spa-rendering.md` accepted as landed (160 lines - over
the ~120 cap, allowed: the package's scope grew to five call sites, and the
text is dense with no padding). `landing-order.md` gained **section 6.6**
recording WP-54 -> WP-55 (same `SidebarMenu` logout effect) and
WP-37 -> WP-38 -> WP-55 (`admin.rs`), with `app.rs`/`settings.rs` verified
disjoint.

**Batch complete.** Both newly-unblocked packages in this batch (WP-48, WP-55)
now have Tier 2 specs.

## WP-55 Turnstile SPA rendering (T2-B6 Worker, 2026-07-26)
Read: `WP-48`/`WP-47` specs (style model), `decisions-ANSWERED.md` D-16 row +
the "D-16 mechanism" verification note, `work-packages.md` WP-55 entry,
`findings/web-frontend-email.md` (the wfe F53 entry - there is **no**
`findings/verification/web-frontend-email.md`, so the raw finding stands
unverified by a second pass; I re-verified it against live source myself).
Live source read: `rust/web/src/app.rs` (`shell`, `HomePage`, `LoginPage`,
`set_theme_client`, `get_turnstile_response`), `components/layout.rs`,
`settings.rs`, `admin.rs`, `rust/web/Cargo.toml`, `rust/web/tests/ssr_pages.rs`.
Wrote: `specs/WP-55-turnstile-spa-rendering.md` (148 lines) and a new
`landing-order.md` section 6.6.
- **No fourth `use_navigate`-to-`/login` site.** `grep -rn '"/login"'` over
  `rust/web/src/` returns exactly five hits: two `<A>` anchors (`app.rs`
  `index-cta`, `layout.rs` nav) and the three known `use_navigate` calls
  (`layout.rs` post-logout, `settings.rs`, `admin.rs`). No server-side
  redirect to `/login` anywhere in the crate.
- **Hard-navigation idiom:** the crate has **no** existing
  `location().set_href` call. Spec'd a new `pub(crate) fn hard_navigate` in
  `app.rs` beside `set_theme_client`, copying that fn's `let Some(window) =
  web_sys::window() else { return; }` SSR guard. `web-sys` is non-optional
  with `"Window"` + `"Location"` already enabled - no cfg gating needed.
- **Collision: REAL.** WP-54 Task 4 rewrites the *same* `SidebarMenu` logout
  effect WP-55 edits, and WP-54 already fences "do not convert `/login` links
  to hard navigations" - recorded as WP-54 -> WP-55 in 6.6, plus
  WP-37 -> WP-38 -> WP-55 for `admin.rs` (WP-37 rewrites the adjacent `"/"`
  bounce in the same `AdminPage` statement block). `app.rs` and `settings.rs`
  edits are disjoint from WP-54's.

---

## 2026-07-26 - db.rs module split unit (Lead)

Unit: spec the split of `rust/web/src/db.rs` (review finding `ws F42`),
escalated by Michael from DEFERRED to high priority ("becoming problematic
due to its size and complexity"). Must land as a hard predecessor for the
remaining web cluster, since most remaining web WPs write into `db.rs`.

Scope fences carried into every Worker brief: writes confined to
`planning/`; `rust/` is READ ONLY; no cargo/git-mutating commands; identify
code by **function name only**, never by line range (33-46% of line-number
citations in earlier specs were wrong).

- **Numbering.** `WP-78 db.rs module split - DEFERRED` already exists in the
  "Unowned / newly discovered" section of `work-packages.md`, and
  `landing-order.md` 6.4 already references `WP-50 -> WP-78`. Per the
  Orchestrator brief the split gets a proper owned number continuing the
  sequence: **WP-82** (WP-81 is the current highest). WP-78's entry will be
  marked SUPERSEDED BY WP-82 rather than deleted, so the existing 6.4
  cross-reference still resolves.
- **Measured before speccing.** WP-41 has landed (+1397/-125) so the review's
  size numbers are stale. Live `rust/web/src/db.rs` is **8149 lines**.
- Worker 1 dispatched: full symbol/coupling/caller inventory of `db.rs`, to
  `raw/db-split-inventory.md`, including the split-axis recommendation, the
  central-layer symbol list, the `.sqlx` cache question, the `mod.rs` vs
  `foo.rs`+`foo/` convention question, and test-module binding.
- Created `architecture-observations.md` (header + append rules). It is the
  parking file for the deferred architectural review; later Leads append a
  `##` section each. Seeded from Worker 1's section H once it returns.
- Web-cluster packages that touch `web/src/db.rs` per `work-packages.md`:
  **WP-35, WP-40, WP-45, WP-47, WP-49, WP-50, WP-52, WP-53, WP-59** (paths
  list `db.rs`), plus **WP-42** which consumes WP-47's `db.rs` predicate.
  This is the same set WP-78's old entry named.
- **ORDERING INVERSION - flag to Orchestrator.** The old `WP-78` entry says
  the split must land *AFTER* all of those. Michael's instruction reverses
  it: the split is a **hard predecessor** for the remaining web cluster, so
  those packages rebase onto the new module layout instead of the split
  rebasing onto ten moved-file edits. `landing-order.md` 6.4's
  `WP-50 -> WP-78` constraint is therefore **reversed** to
  `WP-82 -> WP-50`. Recording this explicitly so nobody "restores" the old
  order from the stale WP-78 note.
- **Worker 1 returned.** `raw/db-split-inventory.md` (633 lines) written and
  verified present. Key measured facts (all post-WP-41, live tree):
  - **8149 lines total**, but the single `#[cfg(all(test, feature = "ssr"))]
    mod tests` is **4838 of them (59%)**. Production is only **~3311 lines**.
    This materially changes the framing: the file is big mostly because of
    its test module.
  - 107 top-level production fns (90 `pub async fn`), 9 structs, 3 enums,
    2 impls, 3 consts, 1 type alias. Largest: `create_game_with_users_tx`
    (~205), `apply_rating_changes` (~151), `update_game_command_success`
    (~115), `find_game_extended` (~113).
  - **28 intra-file call edges. Domain axis crosses 6 module pairs;
    operation-kind axis crosses ~19.** Domain axis wins decisively.
  - **293 external `db::` refs across 22 files, but only 5 `use` lines** -
    everything else is fully-qualified `crate::db::foo(...)`. `pub use`
    re-exports from `db/mod.rs` keep **100%** of callers compiling.
  - **`.sqlx` cache is NOT affected.** Entry keys are `db_name/describe/
    hash/query`; no path or module field, filename is the query-text sha256.
    A pure move needs no `cargo sqlx prepare`.
  - `mod.rs` style is **7/7** in `web/src/` - no `foo.rs` + `foo/` pairs.
    So: `db/mod.rs` + `db/*.rs`, delete `db.rs`.
  - Only **3** private items are directly tested (`choose_colors`,
    `elo_rating_change`, `apply_rating_changes`). The real test-split risk
    is ~12 shared fixture helpers (`make_user`, `make_game_with_players`, ...).
  - **`validate_username` is the only ungated item** (shared with the client
    build). A module-level `ssr` gate on its new home would break the client.
  - `Result` is `anyhow::Result` (Lead-verified: `use anyhow::Result;` in
    db.rs). Worker 1's "unverified" note on this is now resolved.
- Lead rulings handed to Worker 2 (so the implementer does not invent them):
  keep **per-item** `#[cfg(feature = "ssr")]` gates, never module-level;
  `db/mod.rs` uses `mod x; pub use x::*;` per submodule; tests move with
  their code into per-module `#[cfg(all(test, feature = "ssr"))] mod tests`
  with the shared fixtures hoisted to one `db/test_support.rs`.
- Lead writes landed (verified by re-reading each file):
  - `architecture-observations.md` - seeded with Worker 1's section H:
    oversized fns, the 4838-line test module, the four `*_conn`/`*_tx`
    duplicate pairs, the inverted `db.rs -> crate::game::server_fns`
    dependency, no transaction/repository boundary,
    `is_user_recently_active` swallowing errors.
  - `work-packages.md` - **WP-78 marked SUPERSEDED BY WP-82** (retained so
    `landing-order.md` 6.4's reference resolves, with both of its now-wrong
    claims called out), and a new **`## Escalated by the user 2026-07-26`**
    section holding the full **WP-82** entry: READY, ws F42, 0 added to the
    570 sum, measured numbers, domain axis, no-`sqlx-prepare` note, and the
    hard-predecessor list.
  - `landing-order.md` - new **section 7**: WP-82 is a hard predecessor for
    WP-35, WP-40, WP-42, WP-45, WP-47, WP-49, WP-50, WP-52, WP-53, WP-59;
    7.3 explicitly **withdraws and reverses** 6.4's `WP-50 -> WP-78`; 7.4
    lists what is unaffected.
- Worker 2 dispatched to draft `specs/WP-82-db-module-split.md` in WP-47
  house style, with the Lead rulings above stated as decided.
- **Worker 2 returned; spec ACCEPTED by the Lead.**
  `specs/WP-82-db-module-split.md`, 291 lines (prose well under the ~120
  cap; the 12-row module table and its symbol lists carry the length).
  Verified present, ASCII-clean, and **zero line-number code citations** -
  line numbers appear only as measurements in section 1.
- Lead sanity checks performed by reading source directly:
  - Confirmed the `db.rs` `//!` block contains exactly the three parts the
    spec says it does: the `updated_at` trigger convention (14 triggered
    tables, the untriggered `bots`/`llm_providers`/`game_proposals*`, the
    three conditional triggers), the "# Module map" section - which
    literally says *"This file is deliberately one module (a split is
    tracked as review finding ws F42, deferred ...)"* and so **must** be
    rewritten - and the `ssr`-gating note with the `validate_username`
    carve-out.
  - Confirmed `use anyhow::Result;`.
- Worker 2's own additional rulings, accepted: `active_within_window` stays
  in `db/users.rs`; `cap_digest` plus its `cap_digest_*` tests go to
  `db/common.rs`, not `emails.rs`; test fixtures hoist to
  `db/test_support.rs` (`pub(crate)`, not re-exported); formerly-private
  cross-module callees become `pub(crate)` and nothing widens to `pub`.
- `README.md` file map gained a row for `architecture-observations.md`.
- **Unit complete.** Deliverables, all verified on disk:
  `specs/WP-82-db-module-split.md`, `raw/db-split-inventory.md`,
  `architecture-observations.md`, plus edits to `work-packages.md`
  (WP-78 SUPERSEDED + new WP-82 entry), `landing-order.md` (section 7),
  `README.md`.
- **For the Orchestrator:** the only open item is scheduling. WP-82 now
  gates ten web packages, several of which are on the critical path
  (WP-40, WP-47). It is a pure move and should be cheap, but nothing in
  that cluster should start until it lands.

---

## 2026-07-26 - dependency cluster spec unit (Lead)

Unit: specs for WP-64, WP-66, WP-67, WP-69, WP-70, WP-71, WP-72 - the seven
packages unblocked by D-19, D-17, D-18, D-23, D-21, D-22, D-24.

**Grouping decision (Lead, up front):** seven packages, five spec files, three
Workers. These are mechanical dependency packages; one spec per package would
have been padding.

- Worker 1 -> `specs/WP-64-workspace-tables.md`
- Worker 2 -> `specs/WP-66-sqlx-unification.md`,
  `specs/WP-67-sentry-feature-trim.md`,
  `specs/WP-69-deny-toml-hardening.md` (**WP-72 folded in**)
- Worker 3 -> `specs/WP-70-serde-yaml-ng.md`, `specs/WP-71-warp-to-axum.md`

**WP-72 gets no file.** D-24 reduces the whole package to "record `combine` 4.6
as an accepted risk in `deny.toml`". That is one section of the WP-69 spec.
Recorded in `landing-order.md` 8.1 so nobody hunts for `WP-72-*.md`.

**Worker 1 returned:** `WP-64-workspace-tables.md`, 149 lines. Accepted after
one Lead correction (below). Verified live against the tree: 41 manifests (root
+ 40 members), zero `[workspace.dependencies]`/`workspace = true` usage, zero
`[lints]` tables, and **zero crate-level `#![deny]`/`#![warn]`/`#![allow]`
attributes anywhere under `rust/`** - so `[workspace.lints]` has nothing to
preserve and nothing to conflict with. Findings' manifest counts were slightly
stale (tokio/rand are 32 not 33). `authors` is absent from `bot`, `web`,
`operator` and will be gained by inheritance. Hoist list is the 24 keys used by
2+ members. `dp F9` ownership resolved in spec section 3d: the three keys are
`web`-only so they never enter the root table, WP-64 does not perform the
downgrade, and the pin-back-vs-stay-latest tension is flagged for **escalation
to Michael** rather than silently picked.

**Lead correction applied to WP-64:** the draft's regression section called for
`cargo build --workspace --all-targets` and `cargo clippy --workspace
--all-targets`. `AGENTS.md` "Resource constraints" **forbids workspace-wide
cargo runs** on this host (links ~30 binaries, spikes RAM/disk). Replaced with
per-crate `cargo check -p <crate>` during the sweep, the CI clippy split
(`--workspace --exclude web`, then `-p web --features ssr`), and
`scripts/rust-test.sh` as the single sanctioned full gate. **This constraint
was added verbatim to Workers 2 and 3's briefs.**

**`landing-order.md` gained section 8** (dependency cluster), covering: the
binding upgrade-to-latest precondition on WP-64..WP-73; WP-64 first; WP-66/67
unordered between themselves; WP-69 last with only the stale-advisory-ignore
clearance ungated; WP-70 as a two-crate atomic change (bot + lib/game_client
move together); and a **new constraint, WP-06 -> WP-71**, since both touch
`rust/lib/cmd/src/http.rs`. WP-06's spec already records the handoff
(SystemError mapping + 16 MiB body cap must survive the axum port); sequencing
them adjacently satisfies D-22's "touch the surface once" without merging them,
and avoids writing the ls F19 fix twice.

---

## 2026-07-26 - WP-66 / WP-67 / WP-69(+WP-72) specs written by dependency-cluster Worker 2

Three Tier 2 specs written to `planning/specs/`: `WP-66-sqlx-unification.md`
(134 lines), `WP-67-sentry-feature-trim.md` (135), and
`WP-69-deny-toml-hardening.md` (151, WP-72 folded in as section 3d per the
Lead's ruling that D-24 reduces WP-72 to a `deny.toml` comment). All three open
with the binding "Step 0 - upgrade to latest FIRST" section from D-17 and state
what the spec collapses to if the upgrade resolves the issue. All three state
their landing-order position (WP-64 first; WP-66/67 unordered between
themselves; WP-69 last, with only its stale-ignore clearance ungated) and use
only the sanctioned per-crate cargo forms plus the CI clippy split and
`scripts/rust-test.sh`.

**Verified against live files (no cargo run; reading only):**

- `rust/web/Cargo.toml` sqlx 0.8 (`runtime-tokio-rustls`, postgres, uuid,
  migrate, optional), `rust/bot/Cargo.toml` sqlx 0.9 (`runtime-tokio`,
  `tls-rustls`, postgres, uuid, time, json), `rust/operator/Cargo.toml` sqlx
  0.9 (same minus time/json; dev-deps add macros+migrate). Feature divergence
  is real, so WP-66 must reconcile features, not just versions - the spec says
  so and gives the intersection.
- `rust/Cargo.lock` genuinely carries `sqlx 0.8.6` **and** `sqlx 0.9.0`. dp F6
  is correct as written, cause guess included.
- Session store call site named concretely:
  `rust/web/src/auth/session.rs::create_session_layer` (`PostgresStore::new` +
  `.migrate()` + `SessionManagerLayer` with secure/SameSite=Lax/
  `Expiry::OnInactivity(30 days)`). There is **no**
  `continuously_delete_expired` sweeper in web today - noted so a vendoring
  port neither adds nor drops one.
- Vendoring is concrete and small: upstream
  `tower-sessions-sqlx-store 0.15.0` is MIT, and only `src/lib.rs` (50 lines)
  + `src/postgres_store.rs` (266 lines) are needed; MySQL/SQLite stores are
  dropped. Schema is `tower_sessions`.`session`, created by the store's own
  `migrate()`, not by `rust/web/migrations/` - must stay byte-identical.
  Proposed home `rust/lib/session_store/` (`brdgme_session_store`), path dep.
- Currency check (crates.io, 2026-07-26): `tower-sessions-sqlx-store` newest is
  still **0.15.0** and it requires `sqlx = "0.8.0"` and tower-sessions 0.14,
  while `tower-sessions` itself is now **0.15.0**. So Step 0 most likely does
  **not** resolve dp F6 and the vendor branch is the live one - the spec still
  makes the implementer re-check rather than assume.
- `rust/deny.toml` is the only deny config (no repo-root copy). Current values
  confirmed: `[bans] multiple-versions = "warn"`, `wildcards = "allow"`, empty
  `skip`/`skip-tree`; `[sources] unknown-registry`/`unknown-git` both `"warn"`.
- **The 4 advisory ignores are genuinely stale.** `diesel` and `encoding` both
  have grep count **0** in `rust/Cargo.lock`, and the `members` array in
  `rust/Cargo.toml` (40 members) contains no `api` crate. dp F25 confirmed.
- `wildcards = "deny"` confirmed free: zero `= "*"` / `version = "*"` reqs in
  any manifest under `rust/`.
- Ignore accounting reconciled with `WP-68-term-size-replacement.md`: WP-68
  owns RUSTSEC-2020-0163 only, WP-69 owns the 4 diesel/encoding entries, 2
  (paste, proc-macro-error2) remain. No double-counting.

**Where the findings are stale or wrong:**

- **dp F12 (sentry) is probably wrong on its central mechanism.** In sentry
  0.48.5 `default = [backtrace, contexts, debug-images, panic, transport,
  release-health]` and `transport = [reqwest, native-tls]`. **`actix` and
  `ureq` are NOT default features**, so the claim that default features "drag
  actix-web 4 and ureq 3 into every server build" does not follow from the
  manifest. Both `sentry-actix` and `ureq` nonetheless appear under the
  `sentry` package in `rust/Cargo.lock`, while other unused optionals of the
  same crate (`curl`, `sentry-log`, `sentry-slog`, `sentry-anyhow`,
  `sentry-opentelemetry`, `embedded-svc`) do **not** - so the lock is at least
  partly feature-pruned and the contradiction is unresolved. **Unresolved -
  flagged for the Lead.** The spec therefore makes measurement
  (`cargo tree -p bot -i actix-web`, `-i ureq`, plus web/ssr and a game bin)
  the first implementation action, and says explicitly that if those come back
  empty then dp F12's build-bloat claim is false, it should be downgraded from
  major, and the package reduces to spelling the current defaults explicitly.
  I could not run cargo to settle this (read-only brief).
- Under D-18's no-functionality-loss constraint the honest trim list is exactly
  the current defaults spelled out - `["backtrace", "contexts",
  "debug-images", "panic", "release-health", "reqwest", "native-tls"]`.
  `debug-images` (native symbolication) and `release-health` (session health)
  are real functionality and are kept, contra the findings' suggested list
  which omitted `release-health`.
- Sentry usage enumerated from live call sites: `sentry::init` +
  `ClientOptions{release, send_default_pii:false, traces_sample_rate:0.1}` in
  `web/src/main.rs::init_sentry`, `bot/src/main.rs::main`,
  `lib/cmd/src/http.rs::serve`; `TransactionContext::continue_from_headers` +
  `start_transaction` in `lib/cmd/src/http.rs` and `bot/src/main.rs`;
  `configure_scope` in `web/src/router.rs`, `web/src/game/mod.rs`,
  `lib/game_client/src/lib.rs`; `sentry_tracing::layer()` in web and bot;
  `sentry_tower::{NewSentryLayer, SentryHttpLayer}` in `web/src/router.rs`.
  All four crates declare an identical `sentry = "0.48"` default-feature set,
  so the WP-64 hoist is clean and WP-67 is a one-line root edit after it.
  `sentry-tower` (web only, `features = ["http"]`) and `sentry-tracing` are
  separate crates and untouched.
- **dp F8/dp F19 confirmed as monitor items, as work-packages says.** WP-66
  removes only the sqlx-0.8-driver copies; rand 0.8/0.9 also arrive via
  nkeys/nuid, leptos, governor, sentry-core and tungstenite. The one
  first-party action is `bot`'s direct `getrandom = "0.3"` (bump to 0.4 or drop
  for `aes-gcm::generate_nonce`), carried as a rider.
- **dp F18's timing is wrong** (flipping `multiple-versions` to deny now would
  fail CI immediately); D-23 already corrects it and the spec records that.

**Unresolved / for the Lead:**

1. The dp F12 lock-vs-features contradiction above. If the Lead can authorise
   one `cargo tree -p bot -i actix-web` run, WP-67's severity and scope can be
   settled before implementation instead of during it.
2. WP-66 will almost certainly land on the vendor branch, which adds a 41st
   workspace member (`rust/lib/session_store`). That member must inherit
   WP-64's `[workspace.package]` and `[lints]` tables, so WP-64 -> WP-66 is a
   hard ordering, and WP-64's "40 members" assertions become 41 afterwards.

**Worker 2 returned** (Lead review, accepted all three, no corrections needed):

- `specs/WP-66-sqlx-unification.md`, 165 lines
- `specs/WP-67-sentry-feature-trim.md`, 154 lines
- `specs/WP-69-deny-toml-hardening.md`, 167 lines (WP-72 is its section 3d)

Substantive findings from Worker 2's live verification:

- **WP-66:** `rust/Cargo.lock` genuinely carries sqlx 0.8.6 **and** 0.9.0.
  `web` is held at 0.8 by `tower-sessions-sqlx-store 0.15.0`, whose manifest
  requires `sqlx = "0.8.0"` and `tower-sessions 0.14`. On current crates.io
  evidence 0.15.0 is newest, so **Step 0 probably does NOT resolve dp F6 and
  the vendor branch is the live one** - but the spec still makes the check the
  first action and re-verifies at implementation time. Feature sets diverge as
  well as versions (`web` uses the 0.8 spelling `runtime-tokio-rustls`, which
  must be respelled as `runtime-tokio` + `tls-rustls`, not carried over).
  Vendor target is `rust/lib/session_store/` (`brdgme_session_store`), minimal
  port of two files, MIT attribution preserved, DDL byte-identical so existing
  session rows survive. Call site is `create_session_layer` in
  `rust/web/src/auth/session.rs`. dp F8/dp F19 confirmed as re-audit-only:
  rand 0.8/0.9 also arrive via nkeys/nuid, leptos, governor, sentry-core and
  tungstenite, none of which WP-66 touches.
- **WP-67:** all four sentry declarations are bare `"0.48"` with defaults and
  **no crate sets `default-features = false`** - so WP-64's hoist is clean and
  this becomes a one-line root edit. **Worker 2 caught a real problem with the
  finding:** in sentry 0.48.5 `actix` and `ureq` are *not* default features
  (`default = [backtrace, contexts, debug-images, panic, transport,
  release-health]`, `transport = [reqwest, native-tls]`), yet the lock lists
  `sentry-actix` and `ureq`. The spec therefore makes **measurement the first
  action** (`cargo tree -i actix-web` / `-i ureq` per crate, recorded in the
  PR) and says plainly that if they come back empty, **dp F12's build-bloat
  claim is false and the finding is downgraded from major**. No deletion until
  measured. The no-functionality-loss constraint is discharged by a six-point
  end-to-end check against a real DSN (panic capture, backtrace frames,
  tracing breadcrumbs, release/environment/server-name contexts, the
  `sentry_tower` router layers, and **distributed-trace continuation via
  `continue_from_headers`**), each field compared against an event from an
  untrimmed build. `debug-images` and `release-health` are explicitly retained
  because dropping them loses functionality; `native-tls` is non-negotiable.
- **WP-69:** the 4 stale ignores are **confirmed genuinely stale** - zero
  `diesel`/`encoding` entries in `rust/Cargo.lock`, no `api` member in the
  40-member array. Ignore accounting reconciled with WP-68: 7 today, 2 after
  both land (paste, proc-macro-error2). `wildcards = "deny"` verified free
  (zero wildcard reqs in any manifest). Config file is `rust/deny.toml`; there
  is no repo-root one. The spec forbids blanket skips and requires every
  skip entry to carry cause + exit condition. Section 3c records that a
  WP-66 vendored path dep trips neither `[sources]` nor `[licenses]`.

## 2026-07-26 - WP-64 spec written by dependency-cluster Worker 1

- Wrote `planning/specs/WP-64-workspace-tables.md` (120 lines, Tier 2, WP-47
  house style). Findings dp F1 / dp F2 / dp F3, decision D-19 option A.
- Verified by reading live manifests only (no cargo, no writes under `rust/`):
  root `rust/Cargo.toml` has none of the three tables and lists 40 members;
  41 `Cargo.toml` files total; zero `[lints]`/`[workspace.lints]` tables and
  zero crate-level `#![deny]`/`#![warn]`/`#![allow]` attributes anywhere under
  `rust/`, so `[workspace.lints]` has nothing to preserve or conflict with.
- All three findings are correct as written. Only correction: dp F1's counts
  are slightly stale vs live (tokio 32 and rand 32 manifests, not 33; serde 36,
  serde_json 19, thiserror 9 confirmed). Mixed version spellings confirmed
  (tokio `1.52.3` x27 + `1` x8, serde_json in three spellings, etc.).
- Verified dp F2's metadata claims exactly: all 40 members carry
  `version = "0.1.0"` / `publish = false` / `edition = "2024"`; `authors` in 37,
  absent from `bot`, `web`, `operator`. No `license`, `repository` or
  `rust-version` field exists anywhere - spec says do NOT add them
  (`rust-toolchain.toml` already pins channel 1.97.0).
- Decided the hoist list explicitly rather than giving a rule: the 24 keys used
  by 2+ member manifests, enumerated in the spec. `path = ...` (`brdgme_*`) and
  single-consumer deps stay put.
- `dp F9` resolution: the three keys (`tower-http`, `gloo-net`, `gloo-timers`)
  are `web`-only so they do NOT enter `[workspace.dependencies]`; they stay in
  `rust/web/Cargo.toml`. WP-64 owns writing the value once and recording the
  `cargo tree -d` result; it does NOT perform the downgrade. WP-65's T3-B8 row
  is closed out by whatever WP-64 records.
- UNRESOLVED TENSION flagged in the spec, deliberately not decided: dp F9 wants
  a pin *back* to tower-http 0.6 / gloo-net 0.6 / gloo-timers 0.3 to dedupe,
  which contradicts the standing "stay on latest" strategy. Step 0 may dissolve
  it (if leptos/reqwest/kube-client move up); if duplicates persist the
  implementer must escalate to Michael - WASM bundle size vs latest-first - and
  must not pick silently.
- Section 5 originally cited bare `cargo build --workspace`; corrected to the
  per-crate sweep plus `scripts/rust-ci-commands.sh` clippy split and
  `scripts/rust-test.sh` as final gate, per AGENTS.md "Resource constraints"
  which forbids workspace-wide cargo runs.

**LOG CORRECTION (Lead), same session.** Two entries above are inaccurate:

1. `WP-64-workspace-tables.md` is **120 lines**, not 149. Worker 1 rewrote and
   trimmed the file to the cap after the Lead's mid-flight edits, so the final
   on-disk version supersedes them. Verified: no duplicated text, one
   build-command block at section 5.
2. The workspace-wide-cargo correction was **Worker 1's own**, caught in its
   self-review, not solely the Lead's. The Lead independently made the same
   edit; both converged. The final text cites `scripts/rust-ci-commands.sh`
   for the CI clippy split (confirmed to exist alongside `rust-test.sh`,
   `setup-kind-cluster.sh`, `render-compare` under
   `/home/beefsack/Development/brdgme/scripts/`), which is more precise than
   the Lead's inline spelling of the two clippy invocations.

The constraint was still propagated verbatim into Workers 2 and 3's briefs, so
all five specs in this cluster carry it.

**Worker 3 returned** (Lead review, accepted both, no corrections needed):

- `specs/WP-70-serde-yaml-ng.md`, 116 lines
- `specs/WP-71-warp-to-axum.md`, 180 lines

Substantive findings from Worker 3's live verification:

- **WP-70 is genuinely tiny.** Exactly two call sites, both **serialise-only**:
  `rust/bot/src/prompt.rs::spec_to_yaml` and
  `rust/lib/game_client/src/lib.rs::json_to_yaml`, each calling
  `serde_yaml::to_string` on a `serde_json::Value`. **No `from_str`, no
  `Value`, no `Mapping`, no deserialisation anywhere in the workspace** - so
  the drop-in risk is near zero and `to_string` is the only API to port. Both
  manifests declare `serde_yaml = "0.9"` plain, non-optional, no features.
  Step 0 explicitly cannot help (archived at `0.9.34+deprecated`, no newer
  version). Acceptance criterion is **byte-identical YAML** before/after, since
  the bot's system prompt documents the shape. The spec also requires
  recording whether `serde_yaml_ng` still drags the archived `unsafe-libyaml`,
  rather than claiming dp F14 closed if it does. bo F17's "or emit as JSON"
  half is called out as rejected by D-21 so nobody "improves" it.
- **WP-71: WP-06 Task 1 is ALREADY LIVE.** See `landing-order.md` 8.5. The
  8.3 gate is satisfied today; the spec still makes the implementer re-verify
  and STOP on mismatch.
- **WP-71 caught a real deployment risk the findings missed.** warp's
  `warp::post()` matches POST on **any path**, and game-service URIs come from
  the `game_versions.uri` database column - operator-configured, unknown at
  compile time. A naive axum port to `.route("/", post(...))` would **silently
  break every deployed game version whose URI carries a path**. The spec
  mandates `Router::new().fallback(...)` with a method guard and adds an
  end-to-end check that POSTs to a non-root path.
- **Sentry call made, not hedged:** keep the hand-rolled
  `continue_from_headers` -> `start_transaction` -> `set_span` ->
  `finish()` code; do **not** adopt `sentry_tower::{NewSentryLayer,
  SentryHttpLayer}`. Reason: it adds a dependency to a crate compiled into 28
  binaries and renames transactions by route path - here a single catch-all -
  losing the explicit `"game.request"`/`"http.server"` naming, for no gain.
  Distributed trace continuation must keep working;
  `lib/game_client::send_with_retry` injects `sentry-trace`/`baggage` via
  `span.iter_headers()`.
- **One accepted behaviour change recorded:** warp's `content_length_limit`
  *required* a `Content-Length` header (411 without it); axum's
  `DefaultBodyLimit` does not. Chunked requests are accepted again and only
  413 above the cap. That is a relaxation of a WP-06 side effect, not a loss
  of the cap.
- Honest scoping enforced: axum 0.8.9 is already in the lock, and warp 0.4
  already shares hyper 1 / http 1.x with axum, so this is a **dedupe of one
  framework layer**, not removal of a second HTTP stack. The spec forbids
  overselling it in the PR text.
- Two pre-existing issues recorded as riders, explicitly not fixed by WP-71:
  `env_logger` is an ungated dep in `lib/cmd` used only by the gated
  `http.rs::serve` (flagged for WP-65), and `sentry::configure_scope` sets the
  span on a shared hub with no per-request `Hub::run` (same in warp today, so
  not a regression).

## Dependency cluster unit CLOSED - 2026-07-26

Five spec files for seven packages, three Workers:

| File | Lines | Packages |
|---|---|---|
| `specs/WP-64-workspace-tables.md` | 120 | WP-64 |
| `specs/WP-66-sqlx-unification.md` | 165 | WP-66 |
| `specs/WP-67-sentry-feature-trim.md` | 154 | WP-67 |
| `specs/WP-69-deny-toml-hardening.md` | 167 | WP-69 **+ WP-72** |
| `specs/WP-70-serde-yaml-ng.md` | 116 | WP-70 |
| `specs/WP-71-warp-to-axum.md` | 180 | WP-71 |

WP-72 has no file by design. `landing-order.md` gained sections 8.0-8.5.
Every spec opens with the binding Step 0 (upgrade to latest first) and states
what it collapses to if the upgrade resolves the issue. Every spec uses the
per-crate cargo forms and `scripts/rust-test.sh`; none prescribes a bare
workspace-wide build. No file under `rust/` was written by this unit.

**Open items for the Orchestrator:** (1) `dp F9` pin-back-vs-latest-first is
escalated to Michael, unresolved by design; (2) dp F12's actix/ureq bloat claim
is unverified and may be false - WP-67 makes measurement the gate and may
downgrade the finding from major; (3) WP-66's vendor branch looks likely to be
the live one, which creates a new workspace member `rust/lib/session_store/`.

## 2026-07-26 - WP-70 / WP-71 specs written by dependency-cluster Worker 3

Files: `specs/WP-70-serde-yaml-ng.md` (98 lines), `specs/WP-71-warp-to-axum.md`
(152 lines - **over the 120-line brief cap**; three compression passes were run
and the residue is all mandated content: WP-06's five behaviours, the
sentry-tower ruling, the `serve` blast radius, the `http-server` feature delta,
the three ported tests and the four-part end-to-end. Cutting further would have
dropped an instruction, so this is flagged to the Lead rather than silently
trimmed. Siblings WP-66/WP-67 are 140/142 lines.). Both match the WP-66/WP-67
shape (Step 0 opener, landing-order header line, no line-number citations).

Verified against live files, not the snapshot:

- `serde_yaml` has exactly two consumers, both plain non-optional `"0.9"`:
  `rust/bot/Cargo.toml` and `rust/lib/game_client/Cargo.toml`. Exactly two call
  sites, `rust/bot/src/prompt.rs::spec_to_yaml` and
  `rust/lib/game_client/src/lib.rs::json_to_yaml`, and **both are
  serialise-only** (`to_string` on a `serde_json::Value`). No `from_str`, no
  `Value`, no `Mapping`, no deserialisation anywhere in `rust/`. The
  `serde_yaml_ng` swap therefore touches one API only. Neither output leaves the
  process: both end up interpolated into the bot's LLM prompt, so D-21's
  file-format concern is about the prompt contract, not an ops artefact.
- **WP-06 Task 1 HAS ALREADY LANDED** in `rust/lib/cmd/src/http.rs`. The live
  file has private `route::<G>()`, `content_length_limit(MAX_CONTENT_LENGTH)` at
  16 MiB, the `unwrap_or_else -> Response::SystemError` mapping, no
  `impl Reject`, and the three named tests. `lib/cmd/src/lib.rs` has
  `#[cfg(test)] mod test_game`. WP-71's spec describes that post-WP-06 state and
  still tells the implementer to re-verify before starting.
- `lib/cmd` gating live: `default = ["http-server"]`,
  `http-server = ["warp", "tokio", "sentry"]`. `env_logger` is NOT gated despite
  being used only inside the gated `http.rs::serve` - logged as a WP-65 rider,
  not fixed.
- `axum 0.8.9`, `tower 0.5.3` and `sentry-tower 0.48.5` are already in the lock;
  `tower-http` is already duplicated 0.6.11/0.7.0 independently of this work.
  The port needs no `tower-http` (axum's `DefaultBodyLimit` suffices).

Where the findings were stale or wrong:

- `bo F17`'s recommendation offers "maintained fork **or** JSON". JSON is
  rejected by D-21; the spec says so explicitly so nobody re-litigates it.
- `dp F16`'s recommendation calls the lib/cmd surface "a couple of routes" - it
  is one catch-all POST handler. More importantly, dp F16's own caveat (the
  saving is one framework layer, not a second HTTP stack) is **correct** and the
  spec forbids overselling it.
- `ls F34`'s rationale says "only bot shares it", citing bot's Cargo.toml - it
  omits that game_client is itself the second consumer. `dp F14` has the
  complete picture.

Decisions taken inside the specs:

- **Hand-rolled sentry transaction kept over `sentry-tower` layers** in WP-71.
  Adding `sentry-tower` to a crate compiled into 28 binaries, and letting the
  layer name transactions by route path (a single catch-all here), would lose
  the explicit `"game.request"`/`"http.server"` naming for no gain.
- **New hard constraint discovered and written in: the axum router must be a
  catch-all (`Router::fallback`), not `route("/")`.** warp's `warp::post()`
  matched POST on any path, and game-service URIs come from the
  `game_versions.uri` database column, so paths are operator-configured and
  unknown at compile time. `route("/", post(..))` would silently break every
  deployed game version whose URI carries a path. This was not in any finding.
- Recorded as an accepted behaviour change: WP-06's warp `content_length_limit`
  requires a `Content-Length` header (411 without it); axum's `DefaultBodyLimit`
  does not. The port relaxes that side effect while keeping the 16 MiB cap.

Unresolved / for the Lead:

- Whether `serde_yaml_ng` still depends on the archived `unsafe-libyaml 0.2.11`
  could not be determined offline. If it does, the backend half of `dp F14` is
  not closed by WP-70. The spec makes recording this a regression step rather
  than assuming either way.
- The exact `serde_yaml_ng` version is written as `<latest>` - Step 0 resolves
  it.
- Pre-existing, not fixed: `sentry::configure_scope` in `http.rs` sets the span
  on a shared hub with no per-request `Hub::run`, so concurrent requests can
  bleed scope. Identical under warp today, so not a port regression; logged as a
  WP-71 rider only.

**Addendum from Worker 2's final report (Lead, same session).** Two items not
captured in the Worker 2 entry above:

1. **The vendor branch adds a 41st workspace member**, which upgrades
   `WP-64 -> WP-66` from convenience to a **hard** ordering and makes
   `WP-64-workspace-tables.md`'s "40 members" regression assertions stale once
   WP-66 vendors. Recorded as `landing-order.md` 8.6.
2. Worker 2 reported its own line counts as 134/135/151; `wc -l` on disk gives
   165/154/167. The on-disk numbers are authoritative. Worker 2 flagged that
   all three overshoot the ~120 cap by 12-30%, said it compressed twice, and
   judged further cuts would remove load-bearing content. **Lead agrees and
   accepted them as-is** - the overshoot is concentrated in the branch logic
   (WP-66's two branches), the measurement-first protocol (WP-67) and the
   ignore-accounting table (WP-69), all of which a cheap executing model needs
   spelled out.

Worker 2 also confirmed it ran no cargo commands and wrote nothing under
`rust/`; its conclusions came from reading live manifests, `rust/Cargo.lock`,
`rust/deny.toml`, and the vendored registry copies of `sentry-0.48.5` and
`tower-sessions-sqlx-store-0.15.0`. The store's port surface is concrete:
MIT, `src/lib.rs` ~50 lines + `src/postgres_store.rs` ~266 lines.

---

## HANDOVER - dependency cluster Lead, 2026-07-26

**Status: the unit is COMPLETE. Nothing is partially drafted, nothing is
unwritten, no Worker was running when the stop order arrived.** All three
Workers had already returned and all writes were verified on disk before this
entry. This handover exists for continuity only; a successor picks up nothing
half-finished from this unit.

### Per-package status - all seven

| Package | Status | File |
|---|---|---|
| WP-64 | **DONE** | `specs/WP-64-workspace-tables.md` (120 lines) |
| WP-66 | **DONE** | `specs/WP-66-sqlx-unification.md` (165) |
| WP-67 | **DONE** | `specs/WP-67-sentry-feature-trim.md` (154) |
| WP-69 | **DONE** | `specs/WP-69-deny-toml-hardening.md` (167) |
| WP-70 | **DONE** | `specs/WP-70-serde-yaml-ng.md` (116) |
| WP-71 | **DONE** | `specs/WP-71-warp-to-axum.md` (180) |
| WP-72 | **DONE, no file by design** | section 3d of the WP-69 spec |

Also written: `landing-order.md` sections 8.0-8.6 (new), appends to
`architecture-observations.md` (now 101 lines). No file under `rust/` was
written or modified by this unit; verified by `git status`.

### Grouping decisions already made - do not redo

- **Five files for seven packages, three Workers.** One spec per package would
  have been padding on mechanical dependency work.
- **WP-72 gets no file.** D-24 reduces it to "record `combine` 4.6 as an
  accepted risk in `deny.toml`" - one section of WP-69. Flagged at the top of
  the WP-69 spec and in `landing-order.md` 8.1 so nobody hunts for
  `WP-72-*.md`.
- Worker split: W1 = WP-64; W2 = WP-66 + WP-67 + WP-69(+72); W3 = WP-70 + WP-71.

### What a successor would otherwise have to rediscover

**Live manifest state (read, not run - no cargo was executed by this unit):**
- 41 `Cargo.toml` files: root + 40 members. Zero `[workspace.dependencies]`,
  zero `workspace = true`, zero `[lints]` tables, and **zero crate-level
  `#![deny]`/`#![warn]`/`#![allow]` attributes anywhere under `rust/`** - so
  `[workspace.lints]` has nothing to preserve or collide with.
- No `license`, `repository` or `rust-version` field exists in any manifest.
  The channel is pinned in `rust-toolchain.toml` (1.97.0).
- `authors` is absent from `bot`, `web`, `operator`; they gain it by
  inheritance. 24 dependency keys are used by 2+ members (the hoist list).
- dp F1's counts were stale: tokio and rand appear in 32 manifests, not 33.
- `rust/Cargo.lock` genuinely carries **sqlx 0.8.6 and 0.9.0**. `web` is held
  at 0.8 by `tower-sessions-sqlx-store 0.15.0`, whose manifest requires
  `sqlx = "0.8.0"` and `tower-sessions 0.14`. Feature sets also diverge: `web`
  uses the 0.8 spelling `runtime-tokio-rustls` (must be respelled as
  `runtime-tokio` + `tls-rustls`), `bot` adds `time` + `json`.
- All four sentry declarations are bare `"0.48"` with defaults; **no crate sets
  `default-features = false`**. `sentry-tower` is web-only (`features =
  ["http"]`); `sentry-tracing` is web + bot.
- `serde_yaml = "0.9"` in exactly two manifests (`bot`, `lib/game_client`),
  plain, non-optional, no features. Exactly two call sites, **both
  serialise-only**: `bot/src/prompt.rs::spec_to_yaml` and
  `lib/game_client/src/lib.rs::json_to_yaml`, each `to_string` on a
  `serde_json::Value`. No deserialisation anywhere in the workspace.
- `rust/deny.toml` is the config file; there is **no repo-root `deny.toml`**.
  7 advisory ignores today; the 4 diesel/encoding ones are **confirmed stale**
  (`diesel` and `encoding` grep to 0 in the lock, no `api` member). 2 remain
  after WP-68 + WP-69. Zero wildcard reqs in any manifest, so
  `wildcards = "deny"` is free.
- `lib/cmd`: `http-server = ["warp", "tokio", "sentry"]`, default-on.
  `env_logger` is **not** gated despite being used only by the gated
  `http.rs::serve` - noted as a WP-65 rider, not fixed.

**Did the full-upgrade-first step resolve anything?** **Unknown - it was never
run.** This unit wrote specs only and executed no cargo. Every spec makes the
upgrade its Step 0 and states what it collapses to if the upgrade resolves the
issue. Best available evidence, from reading registry metadata only:
- **WP-66: the upgrade probably does NOT resolve it.** crates.io shows
  `tower-sessions-sqlx-store` newest = 0.15.0 (still sqlx 0.8, still
  tower-sessions 0.14) while `tower-sessions` itself has moved to 0.15.0. The
  **vendor branch is almost certainly live.** Port surface is small: MIT,
  `src/lib.rs` ~50 lines + `src/postgres_store.rs` ~266 lines. Schema
  `tower_sessions`.`session`, created by the store's own `migrate()`, called
  from `create_session_layer` in `rust/web/src/auth/session.rs`.
- **WP-70: the upgrade cannot help** - `serde_yaml` is archived at
  `0.9.34+deprecated`; there is no newer version.
- **WP-71: the upgrade cannot help** - warp-vs-axum is a framework choice, not
  a version skew.

**Live source state:** **WP-06 Task 1 has ALREADY LANDED** in
`rust/lib/cmd/src/http.rs` (private `route::<G>()`, 16 MiB
`MAX_CONTENT_LENGTH`, `unwrap_or_else(... SystemError)`, no `impl Reject`, its
three tests present). So WP-71's gate is satisfied today. Recorded in
`landing-order.md` 8.5, with a STOP-on-mismatch instruction because the tree is
under concurrent edit.

### Open items handed to the Orchestrator

1. **`dp F9`** - pin-back-to-dedupe vs stay-latest is a genuine contradiction.
   **Escalated to Michael by design, deliberately not decided.**
2. **dp F12 may be false and may downgrade from major.** In sentry 0.48.5
   `actix` and `ureq` are NOT default features, yet both are in the lock.
   WP-67 makes one `cargo tree -i` run the gate before any deletion.
3. **WP-66's vendor branch adds a 41st member**, hardening `WP-64 -> WP-66`
   and stalings WP-64's "40 members" assertions (see 8.6).
4. **WP-71 risk the findings missed:** warp matches POST on *any* path and
   game-service URIs come from the `game_versions.uri` DB column, so a naive
   port to `.route("/")` would silently break every deployed game version whose
   URI carries a path. The spec mandates a catch-all fallback.

---

## Batch 5 - WP-73 game binary consolidation (Lead session, 2026-07-26)

**Goal:** one Tier 2 spec at `planning/specs/WP-73-game-binary-consolidation.md`.
D-20 answered: generic bin crate `rust/lib/game_bin` / `brdgme_game_bin`
parameterised over `Gamer`, plus thin per-game wrappers. NOT the macro option.

**Plan:** W1 = read-only inventory of the real per-game bin shape ->
`planning/raw/wp73-game-bin-inventory.md`. W2 = draft the spec from that
inventory. Lead sanity-checks and lands.

- W1 DISPATCHED. Brief covers: true shape + divergence across >=5 game crates,
  `lib/cmd` entry-point signatures and feature gates, per-game Cargo.toml
  `[[bin]]` stanzas and bin-only deps, whether fuzz resists the generic
  treatment, workspace members wiring, DOWNSTREAM CONSUMERS of the binary
  names (deploy risk), and the four findings dp F11 / dp F26 / e F45 / e F46.

**Lead-side facts gathered directly (read-only) while W1 runs:**
- `rust/Cargo.toml` members are an EXPLICIT list (no globs), 27 game crates +
  7 `lib/` + 3 `tools/` + bot/web/operator = 40. So `lib/game_bin` needs an
  explicit `members` entry, and any NEW per-game wrapper crate would too.
  Cheapest design therefore keeps the wrappers as `[[bin]]` targets INSIDE the
  existing game crates - no new members, no k8s/Docker churn.
- **HARD CONSTRAINT found by the Lead: `rust/Dockerfile` hardcodes the binary
  file name `<snake_game>_http` in ~26 per-game distroless stages** (e.g.
  `COPY --from=builder /app/target/release/acquire_1_http .` +
  `CMD ["./acquire_1_http"]`). Renaming or collapsing the `_http` bin target
  breaks every game image. Either keep the `_http` target name or update the
  Dockerfile in the same commit.
- Apparent gap: the Dockerfile has stages for 26 games; `lords-of-vegas-1`
  appears to have NO stage. Flagged for W1 to confirm, not acted on.
- `rust/Dockerfile` builds games via `cargo build --release --workspace
  --exclude web`, i.e. it links ALL ~108 bins including cli/repl/fuzz. Cutting
  bin count is a real build-time win - worth stating as the motivation.
- k8s `k8s/base/game/<game>/deployment.yaml` sets `ADDR=0.0.0.0:8080` and no
  `command:`, so it relies purely on the image `CMD`. No k8s change needed if
  the Dockerfile CMD is preserved.
- Ordering: `landing-order.md` 8.4 puts WP-73 after WP-64. WP-71 (warp->axum)
  rewrites `lib/cmd/src/http.rs::serve`, which is exactly what the generic bin
  calls - so WP-73 should also follow WP-71 or explicitly not depend on the
  signature. To be settled in the spec.

**W1 RETURNED.** `planning/raw/wp73-game-bin-inventory.md` written (~308 lines).
Lead sanity-check: its Dockerfile, `[[bin]]`-absence, explicit-members and
missing-`lords-of-vegas-1` claims were independently confirmed by the Lead
before W1 returned. Accepted.

Decisive facts:
- **27 crates x 4 bins = 108 files, ZERO structural deviation.** Normalised-md5
  bucketing: cli 27/27 identical, fuzz 27/27, http 27/27, repl 27/27. The only
  textual variation is rustfmt import ORDER of `use <crate>::Game;`. No crate
  is missing or has an extra bin. This is the best possible case for WP-73.
- Sizes: cli 13 ln, http 13 ln, repl 8 ln, fuzz 5 ln.
- Bounds: `G: Gamer + Debug + Clone + Serialize + DeserializeOwned + 'static`
  (from `requester::gamer::new` and `http::serve`).
- `lib/cmd` features: `default = ["http-server"]`,
  `http-server = ["warp","tokio","sentry"]`, `test-support`.
- All 27 manifests are byte-identical in `[dependencies]` bar 4 additive
  outliers (acquire-1 thiserror; lords-of-vegas-1 thiserror+lazy_static;
  seven-wonders-1 brdgme_cost; cathedral-2 has NO rand).
- All 27 keep a `test-support` dev-dep for `tests/contract.rs` - must survive.
- **No in-repo crate depends on a game crate as a library** (0 hits). So
  e F45's transitive-cost argument is vacuous; the realisable win is the
  27x tokio-"full" compile, not consumer bloat.
- `tools/fuzz` is a hand-rolled fuzzer, NOT afl/libfuzzer - `fuzz_gamer::<G>()`
  is a plain generic fn. Fuzz does NOT resist the generic treatment.
- Findings: **dp F11 CORRECT, dp F26 CORRECT, e F46 CORRECT** (and sharper than
  written - distroless stages run `USER 65532` so the port-80 default is
  unusable in the shipped image; only k8s `ADDR=0.0.0.0:8080` saves it).
  **e F45 facts correct, recommendation INVALID - reconfirmed** (dev-deps do
  not link into `src/bin`); `findings/verification/games-batch-e.md` already
  records it ADJUSTED.

**Design the Lead settled from this (handed to W2):** because the bins are
identical, `brdgme_game_bin` exposes exactly four generic fns
(`cli_main/repl_main/fuzz_main/http_main`, each `<G: ...>`), every `src/bin`
file collapses to one 3-line `fn main()`, **the bin FILE NAMES are unchanged**
(Dockerfile safety), game manifests drop `brdgme_cmd`/`brdgme_fuzz`/`tokio`
from `[dependencies]` and gain `brdgme_game_bin`, and `#[tokio::main]` moves
into `game_bin` so tokio-"full" x27 dies (dp F11). e F46's port default becomes
a one-line change in one place. NO MACRO.

- W2 DISPATCHED to draft `specs/WP-73-game-binary-consolidation.md`, ~120 ln.

**W2 RETURNED and the spec is ACCEPTED.**
`planning/specs/WP-73-game-binary-consolidation.md`, 200 lines.

Lead sanity-check found and FIXED two things before landing it:
1. **Real defect.** The draft wrote the four entry points as
   `cli_main::<G>()` etc. in a *declaration* position. Turbofish in a `fn`
   definition does not compile, and a cheap model would have copied it
   verbatim. Rewritten to `pub fn cli_main<G: ...>()` plus an explicit
   "Syntax, do not get this wrong" note, and the bound now offers the
   one-`GameBin`-supertrait alternative so it is not repeated four times.
2. **Cap.** 218 -> 188 by W2, now 200 after the Lead's two additions. Over the
   ~120 Tier 2 cap, so an explicit Lead-accepted justification was added at the
   top rather than leaving it silent: 3a is a deployment-breaking constraint
   that must not be compressed, and the package spans 27 crates.

Everything else verified sound: crate-relative dep paths (`../cmd`, `../game`,
`../../tools/fuzz`) are right for `rust/lib/game_bin`; the
`grep -rn '0.0.0.0:80"' rust/` check does NOT false-positive on `0.0.0.0:8080`
because of the trailing quote; `[dev-dependencies]` test-support preservation is
called out; no macros anywhere.

**Spec's key decisions:**
- Wrappers stay as `src/bin/*.rs` files INSIDE the existing game crates - NOT
  new wrapper crates. Consequence: workspace `members` gains exactly one line,
  and Dockerfile / docker-bake.hcl / Tiltfile / k8s need NO change.
- All 108 bin FILE NAMES are frozen (section 3a) because `rust/Dockerfile`
  copies `target/release/<snake>_http` by flat filename.
- `#[tokio::main]` on a private async inner fn inside `game_bin`; game crates
  lose `brdgme_cmd`, `brdgme_fuzz` and `tokio`-"full" from `[dependencies]`.
- `tokio = { features = ["macros","rt-multi-thread"] }`, `"full"` banned.
- e F46's port default moves to `0.0.0.0:8080` in the one shared place.

**OPEN - needs Michael (recorded as a non-goal, NOT decided):** `tools/fuzz` and
`tools/repl` are already generic out-of-process drivers, so the 27 `_fuzz` and
27 `_repl` bins are arguably 54 deletable files. WP-73 deliberately keeps them.

Architecture observations appended for the deferred review (duplicate
in-process vs out-of-process fuzz/repl paths; `lords-of-vegas-1` undeployed;
44 k8s game dirs vs 26 live crates; `lib/cmd`'s default-on `http-server`).

**Batch 5 COMPLETE.**

---

## Batch 6 Lead (2026-07-26) - WP-81 + WP-17 + WP-83 parity fixes

Lead brief: three separate deliverables, serial Workers, opus, read-only outside
`planning/`. Context read: ORCHESTRATOR-HANDOVER, README, decisions-ANSWERED
(D-25/D-35/D-40), specs-LOG tail, WP-73 spec as style reference.

### Deliverable 1 - WP-81 stats deletions (D-40 option B)

- W1 DISPATCHED to confirm-then-draft `specs/WP-81-stats-deletions.md`.
  Brief: read live acquire-1 `to_brdgme_stats` + prove zero callers by grep;
  identify exactly which lost-cities-1/-2 `Stats` fields are never read
  (distinguishing written-but-never-read from never-touched); identify the one
  increment that counts the wrong thing; read `findings/verification/` batch-c
  and batch-e; STOP and report if any of the three claims is false rather than
  improvising. Target ~60-90 lines, no padding.

**Lead-side fact for deliverable 2 (gathered read-only while W1 ran):**
`rust/lib/cost/src/lib.rs` ALREADY has a substantial `#[cfg(test)] mod tests`
(~290 of its ~493 lines): add/inv/sub/pos_neg/can_afford/take/drop/keys/
to_keys/is_zero/trim/sum/from_keys plus 10 `can_afford_perm` cases. So D-25's
"suitable automated testing" constraint is NOT "build a suite from nothing" -
it binds on (a) the new generic `get`/`set`, (b) equivalence coverage proving
the splendor port is behaviour-preserving. The spec must say this so nobody
either skips tests or rewrites the existing ones.

Also noted: `Cost::new()` sits in the `K: Hash + Eq + Clone` impl block while
`Default` needs only `Hash + Eq` - that is exactly `ls F38`, and it is a T3-B3
checklist row, NOT part of WP-17's D-25 three. Do not pull it in.

**W1 RETURNED. `specs/WP-81-stats-deletions.md` (123 lines) ACCEPTED as written.**

Lead sanity-check, done independently read-only before acceptance - all four
load-bearing claims confirmed:
- `grep -rn to_brdgme_stats rust/` -> exactly ONE hit, its own definition in
  `game/acquire-1/src/stats.rs`. Zero callers CONFIRMED.
- `game/acquire-1/src/lib.rs` has `mod stats;` and `use crate::stats::Stats;`.
- lost-cities-1 AND -2 both declare `pub investments: usize` in `struct Stats`,
  both have `self.stats[player].expeditions += 1;`, and both have
  `fn player_stats` (so `Stats` must SURVIVE in those two crates - the spec is
  right to delete only two fields, not the struct).

**All three findings CONFIRMED CORRECT** (c F12, e F39, e F40). Nothing
disproved. Sharpening the Worker added, worth recording:
- `e F39` and `e F40` are the SAME code seen twice - `expeditions` is written
  by exactly one increment and never read, so deleting the field deletes the
  wrong increment. There is no separate third edit.
- What the increment actually counts: it fires when the player's whole
  `expeditions[player]` played-card vec is empty, and that vec resets per
  round, so it counts ROUNDS IN WHICH THE PLAYER PLAYED A CARD, not
  expeditions started.
- acquire-1's `Stats` becomes ENTIRELY unused, so `src/stats.rs` is deleted as
  a file, not left as a husk.

**REAL CROSS-PACKAGE COLLISION FOUND (new ordering fact, was not in
landing-order.md):** WP-19 Task 5 fixes `c F11` ("Trades stat reports merges")
with a one-token edit INSIDE `stats.rs::to_brdgme_stats`, and even adds "the
crate's first stats.rs test". WP-81 DELETES that whole file. Verified against
`specs/WP-19-acquire-fixes.md` lines 15/425/468/791. Resolution recorded in
WP-81 section 4 and propagated to `landing-order.md`: land WP-81 first, drop
WP-19 Task 5; whichever lands second must NOT resurrect `stats.rs`.

Spec also correctly states the non-design-statement framing (clean-slate
revisit, do not substitute option A) and the scope guard (no gameplay/scoring
change, no `RULES.md` - WP-20/WP-30 rules halves stay parked).

**Deliverable 1 LANDED.** Planning files updated by the Lead:
- `landing-order.md` gained **section 9 - WP-81 vs WP-19** (full resolution).
- `work-packages.md` WP-19 entry now marks `c F11` **SUPERSEDED by WP-81**,
  stays listed (not reassigned) so the 570 sum and one-package-per-finding
  invariant hold.

### Deliverable 2 - WP-17 `lib/cost` (D-25 option A)

- W2 DISPATCHED to draft `specs/WP-17-lib-cost.md`, ~100-120 lines.
  Brief: add generic `get`/`set` to `lib/cost`; replace splendor-2's
  `src/cost.rs` with `pub type Cost = brdgme_cost::Cost<Resource>;` plus the
  RETAINED crate-local gold-joker `can_afford` (splendor-specific, MUST NOT go
  in the shared lib); add the `brdgme_cost` dep. Scope is EXACTLY `b F31`,
  `ls F39`, `dp F27` - the other five (`b F30`, `b F32`, `b F34`, `b F35`,
  `ls F38`) stay in `checklists/T3-B3-splendor-libcost-holdem.md` and the split
  must be stated in the spec. Given the Lead fact that `lib/cost` already has a
  ~290-line test module, the D-25 testing constraint was scoped to (a) full
  coverage of the new `get`/`set` incl. missing-key/zero/overwrite, (b)
  equivalence coverage proving the port is behaviour-preserving plus direct
  tests on the crate-local gold-joker `can_afford`; existing lib tests must NOT
  be churned. W2 told to independently CHECK the checklist's "semantically
  equivalent" claim for from_resources/add/inv/sub/sum/can_afford and STOP if
  any function differs. Also told to check `seven-wonders-1` (the existing
  `brdgme_cost` consumer) does not regress.

**W2 RETURNED. `specs/WP-17-lib-cost.md` ACCEPTED** after two Lead fixes.

Lead sanity-check, done independently read-only:
- `rust/lib/cost/Cargo.toml` package name is `brdgme_cost`. CONFIRMED.
- Only ONE crate depends on it today: `game/seven-wonders-1/Cargo.toml`.
  `dp F27` CONFIRMED. splendor-2's `[dependencies]` has no `brdgme_cost`.
- Read `game/splendor-2/src/cost.rs` in full and CHECKED the equivalence claim
  myself rather than trusting the checklist:
  - splendor `Cost::can_afford` = `self.sub(other).0.values().all(|v| v >= 0)`;
    lib `can_afford` = `sub(other).pos_neg().1.is_empty()` and `pos_neg` files
    into `neg` exactly when `v < 0`. **Same predicate. EQUIVALENT.**
  - `from_resources(&[Resource])` maps onto lib `from_keys(impl IntoIterator)`;
    `add`/`inv`/`sub`/`sum` are line-for-line the same algorithm.
  - Both are newtype tuple structs over `HashMap<_, i32>`, so serde shape is
    identical and persisted states survive. CONFIRMED.
  Nothing disproved; `b F31`, `ls F39`, `dp F27` all CORRECT.
- The gold-joker free fn `can_afford(a, c)` really is Splendor-specific (it
  subtracts `Resource::Gold` against a summed shortfall). Correctly kept
  crate-local; the spec says loudly it must NEVER move into the lib.

**Lead fix 1 - REAL TRAP the draft would have set for a cheap model.** The
draft said to delete "the now-unused `use std::collections::HashMap`" from
`cost.rs`. But the retained `#[cfg(test)] mod tests` (which MUST keep
`test_can_afford`, the gold-joker test - no lib counterpart) builds fixtures
with `Cost(HashMap::from([..]))`. Blanket-deleting the import breaks the test
build. Rewritten to "move the import into the test module, do not drop it".

**Lead fix 2 - cap.** 172 lines, over the ~120 Tier 2 cap, so an explicit
Lead-accepted justification was added at the top rather than left silent:
section 4 is Michael's binding D-25 testing constraint and section 1 carries
the equivalence proof. Final 185 lines.

**Spec's key decisions:**
- `get(&self, k: &K) -> i32` takes `&K` (generic lib must not force non-`Copy`
  keys to be cloned for a read) - hence splendor call sites become `.get(&x)`.
  `set(&mut self, k: K, v: i32)` unchanged in shape.
- Both go in the EXISTING `impl<K: Hash + Eq + Clone>` block, not a new one,
  so they do not collide with `ls F38`'s later bound rearrangement.
- `cost.rs` survives as a module (type alias + gold-joker fn) so importers'
  `use crate::cost::{self, Cost}` needs no change.
- Testing constraint discharged as: (a) full new `get`/`set` coverage
  (missing key, present, explicit zero, insert, overwrite incl. negative,
  `set(k,0)` interaction with `trim`/`keys`/`sum`); (b) per-test equivalence
  check before deleting any splendor test, gold-joker `test_can_afford`
  RETAINED and extended to 6 cases, plus a serde round-trip of a serialized
  splendor `Game` to pin persisted-state compatibility. Existing lib tests
  must not be churned.
- Non-goals restate the five T3-B3 findings by ID and "no seven-wonders-1
  change" (`cargo test -p seven-wonders-1` is the no-regression gate).

**Deliverable 2 COMPLETE.**

### Deliverable 3 - WP-83 three released parity fixes

- W3 DISPATCHED to draft `specs/WP-83-parity-fixes-released.md`, ONE spec with
  three sections, ~110-130 lines total. Fixes: `a F1` roll-through-the-ages
  (`roll()` re-reads phase after `keep_skulls()` may have advanced it, so a
  previous player's roll decrements the NEXT player's `remaining_rolls`);
  `b F7` seven-wonders (`cities()` lists all 14 A/B entries so both sides of one
  PHYSICAL wonder board can be in play - ensure one of each physical board);
  `e F30` red7 seat-order tie-break when all palettes are empty.
  Brief requires: top-of-spec framing that all three were individually RELEASED
  from the parity park by D-35 and are FIX NOW (so nobody re-parks them); that
  `b F4` is re-parked and `d F37` is REJECTED-not-a-bug and neither is in scope;
  that `e F30`'s release evidence - red7-1's own `DATA_DOCS.md` documenting the
  "highest card overall in the palette" tie-break - be QUOTED in the spec; per
  section a confirmed-against-live-code statement, the fix, and a concrete test
  case; a scope guard forbidding any `RULES.md` edit or widening into the parked
  rules questions in the same crates; STOP-and-report if any finding does not
  reproduce.

**W3 RETURNED. `specs/WP-83-parity-fixes-released.md` ACCEPTED** (183 lines
after one Lead addition). All three findings CONFIRMED CORRECT; none disproved.

Lead sanity-check, done independently read-only:
- **Crate-name correction the Worker caught and the Lead confirmed:** the crate
  is **`roll-through-the-ages-2`**. There is NO `roll-through-the-ages-1` in the
  tree (`ls -d game/roll-through-the-ages-*` -> one dir). Package names verified
  from all three manifests: `roll-through-the-ages-2`, `seven-wonders-1`,
  `red7-1`.
- `a F1`: confirmed `game/roll-through-the-ages-2/src/lib.rs` has BOTH a
  `logs.extend(self.keep_skulls());` inside `keep_skulls`' own caller AND the
  one in `fn roll` immediately followed by `self.remaining_rolls -= 1;`. The
  stale-phase re-match is real.
- `b F7`: confirmed `pub fn cities() -> Vec<City>` in
  `game/seven-wonders-1/src/card.rs` and `all_cities.shuffle(&mut rng)` in
  `lib.rs::start_game`.
- `e F30`: **release evidence verified verbatim.** `game/red7-1/DATA_DOCS.md`
  contains exactly "Ties within a rule are broken by the highest card in the
  winning set, then by the highest card overall in the palette." The second
  clause is unimplemented - `card::leader(palettes: &[Vec<Card>])` is fed
  `rule_fn(&self.palettes[p])` (the WINNING SET, not the palette) by
  `Game::leader_with_suit`, so it cannot apply it. This is a code/doc mismatch
  inside red7-1's own data docs, NOT an open rules question. **That is why
  D-35 released it. Do not re-park it.**

**Lead addition:** a cap note. 177 -> 183 lines, over the ~120 Tier 2 cap, but
it is three independent fixes in three crates in one file (~59 lines each),
so the note records that and says the sections are self-contained and can land
as three commits.

**Key fixes as specced:**
- `a F1`: capture `phase_before` prior to `keep_skulls()` and skip the
  post-roll `match` entirely if the phase changed - `roll_phase()` already
  resets `remaining_rolls` for the new player. Two named failure modes: the
  non-Leadership cascade decrements the NEXT player to 1, and the Leadership
  path silently consumes the `ExtraRoll`.
- `b F7`: group the 14 entries into 7 physical boards by stripping the
  `" A"`/`" B"` name suffix (pairing is carried ONLY in `name: String` - there
  is no `Side` enum), shuffle BOARDS, then pick one side each. **Must use
  `BTreeMap`, not `HashMap`** - grouping order must be deterministic or the
  seeded RNG stops reproducing games.
- `e F30`: change `card::leader` to take `(winning_set, full_palette)` pairs
  and compare the full documented key (set len, max rank in set, max rank in
  palette), still returning the winning set. Keeps strict `>` and seat order
  as final fallback - reachable only when every palette is literally empty,
  since card ranks are unique.

**Deliverable 3 COMPLETE. BATCH 6 COMPLETE.**

Batch 6 summary: 3 specs written and landed
(`WP-81-stats-deletions.md` 123 ln, `WP-17-lib-cost.md` 178 ln,
`WP-83-parity-fixes-released.md` 183 ln). Six findings confirmed correct
(c F12, e F39, e F40, b F31, ls F39, dp F27) plus three parity (a F1, b F7,
e F30). Zero findings disproved. One new cross-package ordering constraint
discovered and recorded (WP-81 before WP-19; `c F11` superseded).

**CORRECTION to the WP-83 entry above.** W3 continued after the Lead's cap note
landed and trimmed the body itself: final file is **155 lines**, not 183 (~50
lines per fix). It kept the Lead's cap note. Re-verified: `phase_before`,
`BTreeMap` and the DATA_DOCS quote all still present after the trim. Spec
remains ACCEPTED.

**Extra observations W3 flagged and deliberately did NOT touch** (correct
behaviour under the scope guard) - appended to
`architecture-observations.md` where structural, recorded here in full:
- `roll-through-the-ages-2::keep_skulls` has an `if self.players == 1 { return }`
  branch that is DEAD (`player_counts()` is `[2,3,4]`). Documented in-crate as
  deliberate Go-source fidelity, so left alone.
- `red7-1::card::leader` returns `(0, vec![])` for an EMPTY `palettes` slice - a
  silent fallback that would mask an all-eliminated state. Not in scope for
  `e F30`; flagged only.
- `seven-wonders-1` uses stringly-typed names for wonder STAGES too
  (`"Rhodes A Wonder Stage 1"`), keyed into the card map by string - the same
  weakness as the A/B side encoding, one level deeper.

---

## Batch 7 - WP-73 amendment for D-41 / D-42 (Lead unit)

**Dispatched W1** (opus): Part A dependency sweep gating the D-41 deletion of
the 27 `_fuzz` + 27 `_repl` per-game bins. Brief: confirm by reading that
`rust/tools/fuzz` and `rust/tools/repl` are generic out-of-process drivers
shelling out to a game's `_cli`, then sweep the WHOLE repo (Dockerfile,
docker-bake.hcl, Tiltfile, k8s, CI, scripts, docs, test harnesses, Go side)
for any reference to the per-game `_fuzz`/`_repl` bins. Instruction: if ANY
dependency exists, report BLOCKED - do not spec the deletion. Output ->
`raw/wp73-fuzz-repl-dependency-sweep.md`.

**W1 RETURNED - verdict CLEAN.** `raw/wp73-fuzz-repl-dependency-sweep.md` written.
Premise confirmed by source read: `tools/fuzz/src/main.rs` and
`tools/repl/src/main.rs` are 3-line wrappers around
`brdgme_cmd::requester::parse_args`, which accepts only `local <path>` and builds
a `LocalRequester` spawning that path as a child process per JSON request.
Nothing hardcodes a `_fuzz`/`_repl` target. Zero references in `rust/Dockerfile`,
`docker-bake.hcl`, `Tiltfile`, `k8s/`, `.github/`, `scripts/`, `infra/`,
`devenv.nix`, `brdgme-go/`; no justfile/Makefile exists in the repo. Game bins
are auto-discovered from `src/bin/`, so no `[[bin]]` stanzas to remove.
Only fallout: the 27 `brdgme_fuzz` manifest lines become unused, `fuzz_gamer` in
`rust/tools/fuzz/src/lib.rs` becomes dead (all 27 callers deleted), and
`docs/porting/GAME_PORTING.md` documents the removed bins. **Accepted cost flagged
for Michael:** `LocalRequester` spawns one process per API request, so
out-of-process fuzzing is materially slower than the in-process `fuzz_gamer` path.
LEAD: sweep accepted. Deletion authorised.

**Dispatched W2** (opus): Part B - amend
`specs/WP-73-game-binary-consolidation.md` in place for D-41 (two entry points
`cli_main`/`http_main`; add the 54-file deletion section; `brdgme_fuzz` leaves
both the game crates and `game_bin`) and D-42 (`lords-of-vegas-1` gets the
consolidation, stays undeployed). Frozen-filename rule extended explicitly to
`_cli` as well as `_http`. Spec must get SHORTER (target 160-180 ln).
`work-packages.md` WP-73 entry to be updated; `landing-order.md` 8.4 only if
sequencing actually changed.

**W2 RETURNED. LEAD ACCEPTED with three Lead edits.**
`specs/WP-73-game-binary-consolidation.md` now 221 lines (was 199). Two entry
points (`cli_main`, `http_main`); new 3d is the D-41 deletion with the sweep as
evidence plus three follow-ons (`fuzz_gamer` deleted / `fuzz()` kept,
`GAME_PORTING.md` updated, accepted `LocalRequester` slowdown); 3a's frozen-name
rule now explicitly covers `_cli` as well as `_http`; `brdgme_fuzz` removed from
`game_bin`'s deps; the "FLAGGED FOR MICHAEL" non-goal deleted; D-42 stated
explicitly in section 4; verification counts corrected to 108-before/54-after.
Macro-free statements strengthened, not weakened. Workspace section unchanged
(one added member, 41-member note to WP-64 intact).

Lead edits on top of W2:
1. Verification grep `fuzz_gamer` was scoped `rust/ docs/` -> would always hit
   `docs/reviews/`. Rescoped to `rust/ docs/porting/` with an explicit warning.
2. `landing-order.md` 8.4: W2 correctly left sequencing alone (unchanged, still
   after WP-64) but flagged a NEW file overlap - WP-73 deletes `fuzz_gamer` from
   `rust/tools/fuzz/src/lib.rs` while WP-63 rewrites `fuzz()` in the same file.
   Recorded as a re-read requirement, not a hard ordering constraint.
3. `work-packages.md` WP-73 "Structural note" was stale and wrong on two counts
   (claimed `[[bin]]` stanzas exist; claimed the spec creates per-game bin
   crates). Corrected to match the spec: auto-discovered bins, wrappers stay
   in-crate, one workspace member added, filenames frozen.

**Deviation accepted:** brief targeted 160-180 lines, landed 221. W2 compressed
sections 1/2/3b/3c/6/7 twice; the remainder is Lead-accepted load-bearing
content (3a deployment constraint, the four finding verdicts, the new 3d
deletion). Not worth cutting correctness to hit a line count.

**BATCH 7 COMPLETE.**

## 2026-07-26 - Lead: WS -> SSE evaluation unit

- Unit brief: evaluate migrating WebSockets to Server-Sent Events. Deliverable
  `planning/ws-to-sse-evaluation.md`, supporting inventory
  `planning/raw/websocket-inventory.md`. Investigation/evaluation, not a fix spec.
- Read for context: `ORCHESTRATOR-HANDOVER.md`, `decisions-session3.md`,
  `specs/WP-42-websocket-auth-and-filtering.md`.
- Worker 1 DISPATCHED: live WebSocket ground-truth inventory (server, client,
  fan-out, infra, tests). Crux question assigned: verify whether ANY
  client->server application traffic exists.
- Worker 1 RETURNED and ACCEPTED: `planning/raw/websocket-inventory.md` (389 lines).
  Key finding: **NO client->server application traffic exists.** `handle_socket`'s
  inbound arm is `Some(Ok(_)) => {}` (value discarded); client drops leptos-use's
  `send` handle via `..`; heartbeat is disabled (`(), DummyEncoder`). Reverse
  direction carries only browser auto-pongs and close frames.
  Other key facts: axum 0.8.9 `ws` feature, `/ws` route, NO handshake auth; two
  payload shapes `{"game_id":..}` / `{"proposal_id":..}`, text frames only, no
  binary; per-socket NATS Core wildcard subs `game.>` + `proposal.>` with zero
  server-side filtering; 15 publish sites; Cilium/Envoy Gateway (no nginx - that
  config is orphaned), DO LB idle timeout 120s, Cloudflare `websockets = on`;
  HTTP protocol version NOT determinable from repo config.
- Worker 2 DISPATCHED: draft `planning/ws-to-sse-evaluation.md`.
- Worker 2 RETURNED and ACCEPTED: `planning/ws-to-sse-evaluation.md` (314 lines,
  within the 320 cap). Lead sanity-check: spot-verified `rust/web/Cargo.toml`
  dependency claims (axum 0.8.9 `["ws","macros"]` default-features-on;
  `leptos_axum 0.8.10`; `leptos-use 0.19` default-features = false with
  `use_websocket, use_event_listener, use_document`; `codee 0.3.5` already
  present) - all correct as written. UNKNOWNs are explicitly marked
  (browser-leg HTTP version, Cloudflare SSE buffering, axum-prometheus latency
  recording for long-lived streams). No line-number citations. No files under
  `rust/` touched.
- Verdict: migrate to SSE, recommended but not urgent; sequence after WP-47 and
  after WP-42's transport-independent predicate work.
- UNIT COMPLETE.

## Unit: SSE migration (D-44..D-47) - Lead session 4

- Michael has COMMITTED to SSE (D-44). This unit has three serial deliverables:
  (1) resolve D-46 connection topology -> `sse-topology-decision.md`;
  (2) write `specs/WP-84-sse-migration.md`;
  (3) rework `specs/WP-42-websocket-auth-and-filtering.md` + reorder
  `landing-order.md` / `work-packages.md`.
- Workers spawned on opus per Michael's override.
- DISPATCHED Worker 1: topology investigation (HTTP/2 browser-leg resolution,
  Option C reconnect cost, two-stream evaluation, third-shape search) ->
  `sse-topology-decision.md`.
- Worker 1 RETURNED: `sse-topology-decision.md` (234 lines). ACCEPTED by Lead.
  Key findings:
  - `infra/cloudflare.tf` sets only `ssl = "strict"` and `websockets = "on"`.
    No `http2`/`http3`/`zero_rtt`/`min_tls_version` zone setting anywhere, and no
    `cloudflare_zone_settings_override`. So browser-leg protocol is dashboard
    default and NOT provable from the repo. Indirect evidence:
    `docs/superpowers/plans/2026-07-10-28-wp4-cloudflare-pre-golive.md` states
    expected `HTTP/2 200` responses through the CF edge, but its checkboxes are
    unticked - an expectation, not an observation. Michael must run
    `curl -sI https://brdg.me | head -1`.
  - Origin remains HTTP/1.1-only (axum `http2` feature enabled nowhere).
  - Dev is permanently h1 (both Tilt modes are plain HTTP, no TLS, no ALPN),
    so any multi-stream design must be sized for the ~6-per-origin cap in dev.
  - Option C reconnect cost quantified as SMALL: no client state on the wire
    (D-45), and the only reopen trigger is game-page navigation, which already
    refetches `game_data`/`logs`/`mark_read`. Incremental cost is one extra
    `get_sidebar_games` POST plus loss of the per-connection TTL cache.
  - RECOMMENDATION (conditional): two streams IF h2 confirmed on the browser leg
    - `GET /events` (private, identity-scoped, never swapped) and
    `GET /events/public?game=<uuid>` (unauthenticated, swapped on navigation).
    Reason is NOT reconnect cost but that `/events/public` needs no auth and no
    visibility predicate at all, plus decoupling the private stream from
    navigation for future chat/notifications. If h2 is NOT confirmed: one stream
    (Option C). Hard cap: never 3 held streams.
  - Rejected: side-channel resubscribe POST (needs replica affinity; `replicas: 2`
    with no session affinity in `httproutes.yaml`). Held in reserve: a single
    all-public-games stream with client-side filtering - noted as essentially
    what production does today, since `handle_socket` currently filters nothing.
- DISPATCHING Worker 2: `specs/WP-84-sse-migration.md` on the two-stream design
  with a documented collapse-to-one-stream fallback.
- Worker 2 RETURNED: `specs/WP-84-sse-migration.md` (~300 lines). ACCEPTED with a
  Lead note: it overruns the ~120-line Tier 2 cap and the ~170 the Lead
  authorised. Kept at length deliberately - it is a transport migration covering
  routes, auth, shutdown, metrics, client, infra, deletions and a conditional
  fallback shape, and the content is verified fact rather than padding. Header
  justification rewritten by the Lead to say so.
  Key resolutions in the spec:
  - **`axum_prometheus` UNKNOWN RESOLVED: no distortion.** Read vendored
    `axum-prometheus 0.10.0` `src/lib.rs`: `Traffic::on_response` records
    `data.start.elapsed()` when the INNER SERVICE FUTURE resolves (response
    headers), not on body completion, so a multi-hour SSE stream is recorded as a
    sub-millisecond request. `PrometheusMetricLayer::pair()` passes `None` for the
    body-size recorder, so no per-chunk histogram. One real effect to document:
    `axum_http_requests_pending` IS held for the stream's lifetime (its `Pending`
    `Arc` is cloned into the streaming body), so it becomes a free live-stream
    gauge but stops meaning "handler still running".
  - **Cloudflare rate-limit rule verified live:** exactly one ruleset
    (`cloudflare_ruleset.rate_limit`, phase `http_ratelimit`), one rule `api_per_ip`,
    expression `starts_with(http.request.uri.path, "/api/")`, block, period 10,
    60 req/period. The prior evaluation's claim that `/ws` is exempt is CONFIRMED.
  - Only idle timeout in the repo is `k8s/base/gateway/gateway.yaml`'s
    `do-loadbalancer-http-idle-timeout-seconds: "120"`; axum `KeepAlive::new()`
    verified at 15s. Cloudflare edge idle behaviour for `text/event-stream`
    remains UNKNOWN from repo state - spec directs the implementer to verify.
  - **Rollout: SIDE-BY-SIDE, not a cutover**, in three commits. Reason found by
    the Worker: `/pkg/` assets are edge-cached `immutable`, so a browser holding
    an old wasm bundle keeps requesting `/ws` after deploy; a flag-day cutover
    breaks those clients until reload.
  - Shutdown: `GameBroadcaster::shutdown` + the stream's `select!` arm MUST stay
    (graceful shutdown hangs forever without it). `TaskTracker`/`drain_ws_tasks`/
    the 5s timeout are deletion CANDIDATES only - still UNKNOWN, spec requires a
    real-listener proof before deleting.
  - Client API gap found: leptos-use `use_event_source` has NO `on_message_raw`;
    the port must drive bumps from an `Effect` on the `message` signal branching
    on `event_type`, and verify duplicate consecutive frames still fire.
  - Deletions confirmed unused elsewhere: axum `ws` feature (`rust/bot` and
    `rust/operator` declare plain `axum = "0.8"`), `tokio-tungstenite` dev-dep.
- DISPATCHING Worker 3: rework WP-42 in place + reorder `landing-order.md` and
  `work-packages.md`.
- Worker 3 RETURNED and ACCEPTED. Three files edited in place:
  - `specs/WP-42-websocket-auth-and-filtering.md` (143 -> 244 lines). Retitled
    "realtime visibility predicates and per-connection filter cache"; filename
    kept so cross-references resolve. Header carries a fate table:
    §3a pre-upgrade auth SUPERSEDED (do not build, replaced by WP-84 §3c);
    §3b TTL cache SURVIVES (reframed per-connection, must not depend on any
    `axum::extract::ws` type, lives in its own module); §3c `db.rs` predicates
    SURVIVE and are now the bulk of the package; §3d Task B ELIMINATED.
    New §3e DECIDES the open question: **WP-42 makes NO edit to
    `rust/web/src/websocket.rs`.** Reasoning is not "throwaway" but
    "unbuildable" - the filter needs `viewer: Option<Uuid>`, and on the WS path
    the only source of that is the superseded pre-upgrade dance; wiring it with
    `viewer = None` would fail-closed and hide private games from their own
    participants. Consequence accepted explicitly: `/ws` stays unfiltered until
    WP-84 step 2, which is the status quo, not a new regression. Flag to the
    Orchestrator if WP-84 slips.
    §5 test strategy DECIDED: unit-test the predicate and the cache only; write
    NO new tests against the WebSocket transport; defer end-to-end filtering
    assertions to WP-84 §9's SSE tests. The previous instruction to rework
    `live_websocket_survives_idle_past_request_timeout` is WITHDRAWN.
  - `landing-order.md`: section 4 cluster line updated; §7.1 table gains a
    WP-84 row; §7.4 updated; new **section 10** records the SSE pivot ordering
    WP-82 -> WP-47 -> WP-42 (predicate only) -> WP-84, plus WP-84's HTTP/2
    BLOCKER (WP-42 is NOT gated on it).
  - `work-packages.md`: WP-42 entry rescoped; new WP-84 entry added in house
    style with the blocker, rollout, consumes-not-authors list and UNKNOWNs.
    Package-count totals earlier in the file were NOT recomputed - marked stale
    inline rather than guessed.
- WP-47 DEPENDENCY CLAIM VERIFIED by the Lead directly against
  `specs/WP-47-game-visibility-gates.md` §3a: WP-47 does create
  `is_game_visible_to_viewer(pool, game_id, viewer: Option<Uuid>)` as a thin
  dispatcher over `is_game_publicly_visible` / `is_game_visible_to_user`.
  The claim HOLDS. WP-47 also itself depends on WP-41 (landed) and WP-82.
- Architectural observations appended to `architecture-observations.md`:
  no HTTP protocol version is expressed anywhere in the repo; dev is
  permanently h1; `httproutes.yaml` has no session affinity against
  `replicas: 2`; the client `(id, seq)` trigger design is idempotent under
  duplicate delivery.
- UNIT COMPLETE.

## Batch 7 - D-43 fuzz throughput evaluation + WP-73 re-amendment (Lead, 2026-07-26)

- Worker 1 (fuzz throughput evaluation) RETURNED and ACCEPTED. Wrote
  `planning/fuzz-throughput-evaluation.md` (221 lines). The Lead independently
  read `rust/tools/fuzz/src/lib.rs`, `rust/lib/cmd/src/requester/gamer.rs` and
  `rust/lib/cmd/src/requester/local.rs` and CONFIRMS every load-bearing claim.
- **Headline, and it inverts the premise of D-43:** the in-process fuzz path is
  **NOT** free of serialisation. `requester::gamer::GameRequester` implements the
  same `api::Request`/`api::Response` contract as the out-of-process path and
  only removes the transport. `api::Request::Play` carries game state as a JSON
  `String`, so per move the in-process path already does
  `serde_json::from_str::<G>`, `GameResponse::from_gamer` ->
  `serde_json::to_string`, plus `renders()` building pub_state JSON, every
  player's state JSON, and N+1 `brdgme_markup::to_string` renders - all of which
  the fuzz loop discards except the acting player's `command_spec` and the
  opaque state string.
- Per move = exactly 1 request. So out-of-process would be 1 process spawn per
  move x `num_cpus` threads, plus a SECOND JSON layer (the state string is
  escaped again inside the outer Request/Response). Directionally strictly worse.
  Magnitude UNKNOWN - nothing was measured, no cargo in this session.
- `fuzz()` is **already parallel** - `num_cpus::get()` threads, own `Requester`,
  own `Fuzzer`, own `ThreadRng` each; the `Arc<Mutex<F>>` is touched once per
  thread at startup. No shared mutable hot-loop state. "Add parallelism" is NOT
  an available win. No `Rc`/`RefCell`/`Cell` anywhere under `rust/game/*/src/`.
- Recommendation ACCEPTED by the Lead: (1) keep fuzz in-process, do not adopt
  `LocalRequester` for fuzz; (2) adopt generic `fuzz_main::<G>()` in
  `brdgme_game_bin` - genuinely speed-neutral, same monomorphisation; (3) the
  real throughput project is separate - drive `Gamer` directly, keep the game
  live in memory, delete serde+markup from the hot loop; NEEDS MICHAEL'S
  DECISION because it trades away incidental render/serialise-panic coverage;
  (4) a single all-games binary is convenience, not throughput - defer.
- Two free wins recorded in `Fuzzer::command`: a full `PlayerRender` clone to
  take one field, and a `state.to_string()` full state clone, both per move.
- Worker 2 (WP-73 re-amendment) RETURNED and ACCEPTED. Lead re-read all three
  edited files in full and confirms internal consistency; a grep for the old
  counts (`54 file`, `54 after`, `54 wrappers`, `TWO entry points`,
  `Accepted cost`, `delete fuzz_gamer`) returns only two deliberate historical
  back-references. Files changed:
  - `specs/WP-73-game-binary-consolidation.md` (222 -> 253 lines). Header now
    cites D-41 + D-43. Counts corrected THROUGHOUT: 108 before / **81** after
    (27 `_cli` + 27 `_http` + 27 `_fuzz`), **27** deletions not 54, in section 1,
    3a, 3d, the preamble and every row of section 7 Verification. 3b restores
    `fuzz_main` as a THIRD entry point (`brdgme_fuzz::fuzz_gamer::<G>()`,
    non-async, no tokio) and reverses the "`brdgme_fuzz` is NOT a dependency"
    line - `game_bin` now takes it. 3c gains the `_fuzz` wrapper example. 3d
    rewritten as `_repl`-only with D-43's reason; the "delete `fuzz_gamer`" and
    "accepted LocalRequester cost" follow-ons are gone. `GAME_PORTING.md`
    instruction inverted: KEEP the fuzz layout entry and
    `cargo run --bin <name>_N_fuzz`, remove only the repl entry. Section 7 now
    asserts `fuzz_gamer` grep is **non-zero** after, with "zero hits means the
    fuzz path was deleted in error". New non-goal fences off evaluation 4(d).
  - `work-packages.md`: WP-73 entry - D-43 added to the header, D-41 bullet
    rewritten to `_repl`-only/27 deletions, new D-43 bullet, THREE entry points,
    `brdgme_fuzz` IS a `game_bin` dep, `rust/tools/fuzz/src/lib.rs` dropped from
    the Paths list.
  - `landing-order.md` 8.4: the "NEW file overlap WP-73 x WP-63" bullet is now
    **WITHDRAWN** - WP-73 never touches `rust/tools/fuzz/src/lib.rs` post-D-43.
    Records the reverse consequence: WP-63's `bo F29` reasoning stays TRUE.
  - `specs/WP-63-fuzz-tool.md` needed NO edit - verified.
- Lead recorded a stated crate-graph cost in WP-73 3b rather than hiding it:
  `brdgme_game_bin` -> `brdgme_fuzz` -> `brdgme_cmd`/`brdgme_rand_bot`/
  `num_cpus`/`rand`/`anyhow`, so `_cli` and `_http` now transitively link the
  fuzz subtree. Judged acceptable (compile time and image size, not runtime
  surface); fallback recorded is a `fuzz` cargo feature gate if image size
  regresses measurably.
- `decisions-session3.md` D-43 gained a **RESOLVED** subsection closing its open
  question, correcting its own premise (both paths serialise), and recording the
  one thing still needing Michael: the 4(d) render-coverage tradeoff.
- `architecture-observations.md` gained a "Fuzz / requester layering" section:
  the `Requester` abstraction forces a JSON boundary even with no transport;
  `renders()` is unconditional and eager with no way to ask for less;
  `LocalRequester` spawns per request not per session and `cli.rs` is a one-shot
  by construction; no `Rc`/`RefCell`/`Cell` anywhere under `rust/game/*/src/`.
- UNIT COMPLETE.

---

## 2026-07-26 - Lead unit: finalise WP-84 on the two-stream multi-topic design

Brief: fold D-48/D-49/D-50 into `specs/WP-84-sse-migration.md`, remove all
conditionality (single-stream fallback DELETED - browser leg measured `HTTP/2 200`),
adopt the repeatable `?topic=game:<uuid>` param with N topics from day one, name
`event:` fields meaningfully on the private stream, re-check the whole spec for
internal consistency, and mark D-46 RESOLVED in `sse-topology-decision.md`.

- Worker 1 DISPATCHED: verify how axum 0.8.9 (as locked in `rust/Cargo.lock`)
  handles a REPEATED query key, by reading real crate source. This is the unit's
  one genuine unknown - flagged in D-50 as "from general knowledge, NOT source".
- Worker 1 RETURNED - **axum repeated-query-param question RESOLVED by reading
  real crate source** (tarballs extracted from `~/.cargo/registry/cache/`; there
  is no `vendor/` dir and no `registry/src/`). Chain read:
  `axum-0.8.9/src/extract/query.rs` -> `serde_urlencoded-0.7.1/src/de.rs` ->
  `serde_core-1.0.228/src/de/value.rs` (`MapDeserializer`, `PairDeserializer`)
  and `de/impls.rs` -> `form_urlencoded-1.2.2/src/lib.rs` (`Parse::next`).
  - **`Query<Vec<(String, String)>>` WORKS** and preserves every repeated pair in
    query order. `deserialize_seq` -> `visitor.visit_seq(self.inner)`;
    `MapDeserializer::next_element_seed` hands each pair to `PairDeserializer`.
    serde_urlencoded's own doctest covers this exact type.
  - `Query<HashMap<String, String>>` collapses duplicates, **last wins**
    (`visit_map` loops `insert`).
  - `Query<HashMap<String, Vec<String>>>` and `struct { topic: Vec<String> }`
    both **FAIL with 400** - serde_urlencoded's `Part` value deserializer
    forwards `seq` to `deserialize_any`, which only ever visits a string.
  - `axum-extra` is **absent from `rust/Cargo.lock`** entirely (not even
    transitive). Latest is 0.11.0 per the local index cache; its repeated-key
    semantics are UNKNOWN-by-read (source not on disk).
  - `serde_qs` is in the lock at **0.15.0 but only transitively** (leptos 0.8.20,
    server_fn 0.8.13); latest is 1.1.2, so adding it directly at latest would put
    two majors in the tree. **Not needed.**
  - **No house pattern:** `grep -rn "extract::Query\|RawQuery\|axum_extra"` over
    `rust/` returns ZERO hits. `rust/web/src` never parses a query string
    server-side. Only client-side `use_query_map()` in `new_game.rs`/`players.rs`.
  - **CONCLUSION: D-50's flagged concern is real for the HashMap/struct forms but
    the fix needs NO new dependency.** Use `Query<Vec<(String, String)>>` and
    filter for key == "topic".
  - Lead correction to the Worker's closing line: it said to put the extractor on
    `ws_handler`; that is wrong - `/ws` is being deleted. The extractor goes on
    the new `/events/public` handler in `rust/web/src/events.rs`.
- Worker 2 DISPATCHED: apply all edits to WP-84 + the three companion docs.
- Worker 2 RETURNED and **ACCEPTED by the Lead** after reading the changed
  sections directly. `specs/WP-84-sse-migration.md` is now ~406 lines, one
  settled design with no conditionality.
  - BLOCKER block -> SETTLED block citing the measured `HTTP/2 200` and D-48.
  - Old §8 (single-stream fallback) DELETED; §9 -> §8, §10 -> §9. Lead verified
    by grep: no `§9`/`§10`/`section 8` references survive in the file, and the
    only surviving `?game=` string is the client-side instruction "Nothing may
    still emit `?game=<uuid>`", which is correct.
  - §3a carries the topic URL, the no-`[]` rationale, the never-three-streams
    hard cap, and the explicit "dev is permanently HTTP/1.1, any FUTURE increase
    in held-stream count must be re-checked against dev, not just prod".
  - §3c gained "Topic parsing and validation" (collection from day one, only
    `game:`, **cap N at 16**, 400 on unknown prefix / malformed uuid / no `:` /
    zero topics / over cap) and a dense VERIFIED subsection recording the axum
    finding with a four-row table of which `Query` forms work and which 400.
  - §3d fan-out is now **set membership over N ids**, not equality against one.
  - §3e event naming upgraded to a hard requirement with the D-49 rationale.
  - §6 upgraded "prefer" to a ruling: **`/events/public` must stay UNMATCHED by
    any rate rule**; any rule is scoped to `/events` and matches establishment
    only.
  - §8 tests gained an **N>1 topics both deliver** case (explicitly flagged as
    the test that catches a HashMap-collapsing or struct-based extractor - without
    it the wrong extractor passes every other test) and a 400-rejection case.
- Companion docs updated: `sse-topology-decision.md` gained a RESOLVED banner at
  the top with confirm-items 1/3/4 marked ANSWERED and the body left intact as
  the surviving record of WHY; `landing-order.md` §10.4 retitled to "blocker is
  DISCHARGED"; `work-packages.md` WP-84 heading is now plain `READY`.
- Worker 2 flagged two things for the Lead:
  1. `specs/WP-42-...md` carries stale cross-refs to the deleted §8 shape and to
     `WP-84 §9`. **Worker 3 DISPATCHED** to sweep them.
  2. **The cap of 16 is the Worker's proposal within D-50's stated intent, NOT a
     Michael ruling.** Escalate.
- Still genuinely UNRESOLVED and deliberately left as prove-before-deleting:
  §3g's `ws_tasks: TaskTracker` deletion (needs the real-listener graceful-
  shutdown proof) and §6's Cloudflare edge idle behaviour for `text/event-stream`.
- Worker 3 RETURNED and ACCEPTED. Edited **only** `specs/WP-42-...md`: the §3d
  sentence citing "the widened single-stream `?game=` param under WP-84 §8" is
  rewritten to the settled two-stream design (D-48/D-49/D-50), and four `WP-84 §9`
  test references corrected to `§8`. Matched on text, not on the line numbers in
  the brief. Verified every other WP-84 section citation in WP-42 against the live
  heading list (§2, §3a, §3c, §3d, §3g, §5, §7 - all correct). `specs-LOG.md`
  untouched by instruction (append-only history; stale refs in it are correct
  history). Coverage check: `grep -rlw SSE` over `planning/` returns only 9 files,
  all accounted for.
- Lead applied two supersession notes itself (planning-file writes, in scope):
  - `ws-to-sse-evaluation.md` gained a **SUPERSEDED IN ITS RECOMMENDATION** banner.
    Its recommendation was Option C (one stream) with `/events/public` "held in
    reserve" - the exact opposite of what D-48 settled. Body kept: the crate-source
    findings and UNKNOWN markers are why the file exists.
  - `architecture-observations.md`'s pre-D-46 bullet "the browser leg is not
    determinable from config, any design assuming HTTP/2 multiplexing is
    unverified" gained an **ANSWERED** note. The config claim stands; the open
    question does not. Reframed the surviving architectural point: a
    production-relevant protocol fact is discoverable only by hitting the live
    edge, because the zone's protocol settings live in the Cloudflare dashboard
    and not in `infra/cloudflare.tf` (which sets only `ssl` and `websockets`).
  - `raw/websocket-inventory.md` deliberately NOT touched - it is a raw snapshot
    and is still factually accurate about repo state.
- UNIT COMPLETE.

---

## 2026-07-26 - Lead unit: apply approved doc changes + rewrite the handover

**WIDER WRITE PERMISSION.** Michael explicitly authorised writes to
`docs/CODING.md` and `docs/BACKLOG.md` for this unit only, in addition to
`planning/`. Everything else unchanged: never touch `rust/`, no cargo/git,
reads only for validation. Workers on model opus (Michael override).

Four deliverables:
1. Apply `CODING-md-amendment-proposal.md` -> new `## Request-Path Invariants`
   section in `docs/CODING.md`, between `## Rust: Error Handling` and
   `## Leptos: SSR and Hydration`. Re-verify the anchors live.
2. Apply `BACKLOG-note-proposed.md` -> `docs/BACKLOG.md` item #53 (verify #53
   is still free).
3. NEW `docs/BACKLOG.md` item for the maximum-performance fuzzer, per D-51 -
   Michael asked that the discoveries be persisted outside the archival review
   directory.
4. Rewrite `ORCHESTRATOR-HANDOVER.md` for the next session.

- [dispatch W1] Deliverables 1-3 (the three doc edits), one Worker.
  Lead-directed factual correction carried in the brief: the proposed #53 row's
  clause "Five egregious cases (a F1, b F4, b F7, e F30, d F37) are flagged
  ... as candidates for immediate fix" is STALE. Later rulings re-parked `b F4`
  (7 Wonders resources are not depleted by trade - Michael's binding
  correction) and REJECTED `d F37` (not a bug). Only `a F1`, `b F7`, `e F30`
  are fix-now. Worker instructed to correct that clause and change nothing
  else in the row.
- [W1 done] **ACCEPTED.** Lead re-read both changed regions directly.
  - `docs/CODING.md`: anchors HELD exactly. `## Rust: Error Handling` and
    `## Leptos: SSR and Hydration` present, adjacent, in order, one `---`
    between them; the Error Handling section's final paragraph still begins
    `**DOM access in event handlers.**`. New `## Request-Path Invariants`
    section now sits at :69-176 with a fresh `---` below it, all six rules
    verbatim from the proposal. Worker read all 633 lines before editing.
  - **DRIFT: `docs/BACKLOG.md` item #53 ALREADY EXISTED** in the working tree,
    matching the proposal text exactly. Someone (Michael or the concurrent
    agent) had already applied it. Worker correctly did NOT duplicate it and
    applied the stale-clause correction in place instead. Provenance of the
    pre-existing row is UNKNOWN.
  - The `b F4` / `d F37` correction LANDED (row 53): now reads "Three cases
    (a F1, b F7, e F30) are flagged for immediate fix and are outside the park;
    b F4 was re-parked and d F37 was rejected as not-a-bug."
  - **The fuzzer item is #54, not #53** (next free after the pre-existing 53).
    Carries all seven D-51 must-survive points and links the evaluation.
  - Open for Michael, flagged by W1: #54 is not listed in BACKLOG.md's
    "Priority order (updated 2026-07-24)" block at the top. Left alone
    deliberately (outside "change nothing else").
- [dispatch W2] Deliverable 4 - rewrite `ORCHESTRATOR-HANDOVER.md`.
- **LEAD FINDING while cross-checking (new, not previously recorded anywhere):
  the migration-numbering collision is FOUR packages, not three.**
  `landing-order.md` 6.4/6.5 records WP-50, WP-56 and WP-58 as each adding a
  migration and each assuming `022` is the highest. But
  `specs/WP-34-auth-races-session-mechanical.md` names its new migration
  **`rust/web/migrations/023_login_email_sends.sql`** explicitly, and WP-50's
  is **`023_canonical_emails.sql`** - a direct filename-number clash between
  two specs. So the set is **WP-34, WP-50, WP-56, WP-58**; the second, third
  and fourth to land must renumber. Recorded here and carried into the
  rewritten handover. `landing-order.md` itself NOT edited - out of this
  unit's four deliverables; flag it to the next Lead.
- **LEAD FINDING: `README.md`'s "no spec" lists are STALE.** It claims WP-04,
  05, 46, 55, 58, 64, 66, 67 have no spec, but `specs/WP-04-game-parser-design.md`
  and `specs/WP-05-color-dead-parse-api.md` both exist and are complete. The
  spec-writing unit ran after that README text was written. W2 was told to
  establish the remaining-work picture by listing `specs/` directly rather than
  trusting the prose, which is the right call. README not corrected here
  (outside this unit's deliverables).
- **LEAD FIX (planning-file write, in scope):** `decisions-session3.md` had
  **D-42's ruling orphaned** under D-51's "Home for it" section with no heading
  of its own, so it read as part of the fuzzer decision. Restored a
  `## D-42 - lords-of-vegas-1 gets WP-73 too, but stays undeployed` heading with
  a note explaining the restoration. This matters because the handover points
  successors at that file as the authoritative D-41..D-51 record.
- **LEAD FINDING: `specs/WP-50-email-canonicalization.md`'s header carries a
  WITHDRAWN ordering constraint.** It still says *"WP-78 (`db.rs` split) is
  deferred until this lands."* `landing-order.md` 7.3 explicitly **withdraws**
  that and **reverses the direction** - the item is now WP-82, and it is
  **WP-82 -> WP-50**, with WP-82 a hard predecessor of the whole web cluster.
  An implementer reading only the WP-50 spec would sequence it backwards.
  Spec NOT edited (outside this unit's deliverables) - flagged to the next Lead
  and recorded in the rewritten handover's remaining-work notes.
- [W2 done] **ACCEPTED.** `ORCHESTRATOR-HANDOVER.md` rewritten in place, **234
  lines**, ASCII-clean (all `section` spelled out, no em dashes/smart quotes).
  Lead read it in full. Ten sections, all ten required points present.
  - W2 established REMAINING work properly: listed `planning/specs/` (**59 spec
    files** + `notes-conventions.md`), grepped every `checklists/T3-B*.md` for WP
    references, reconciled against a full read of `work-packages.md` and
    `landing-order.md` 7 and 10. **Spec coverage is effectively complete**; the
    only genuine gap is **WP-76, WP-77, WP-79, WP-80** (the unowned cluster).
  - Correctly refused to quote package totals - `work-packages.md`'s own
    Coverage-check disowns them (computed pre-WP-83/84). Marked UNKNOWN.
  - Drift W2 found and recorded: **`work-packages.md` has NO entry for WP-83**
    (only a passing mention in WP-84's package-count note) though the spec
    exists and is complete; `README.md`'s status banner and file map are stale
    (it says the newly-unblocked specs are unwritten - they are written - and
    still lists both doc proposals as unapplied - both are now applied).
  - LEAD EDIT on top of W2: folded in the two Lead findings above - the
    migration collision corrected to **four** packages (WP-34 added, with the
    `023` filename clash against WP-50 named), and the stale `WP-50 -> WP-78`
    ordering in the WP-50 spec header flagged for correction.
- **UNIT COMPLETE.** All four deliverables landed and accepted.

## 2026-07-26 - Unit: planning-corpus inconsistency cleanup (Lead, opus workers)

Scope: five known inconsistencies handed down from the previous Lead. NOT a
general audit. Workers spawned serially, opus, read-only outside `planning/`.

- [W1 done] **ACCEPTED.** Tasks A (migration collision) + B (WP-50 ordering).
  - Verified by reading all four specs: **all four DO add a migration.**
    WP-34 `023_login_email_sends.sql` (hard-coded), WP-50
    `023_canonical_emails.sql` (hard-coded), WP-56 `0NN_settings_email_token.sql`
    ("next free"), WP-58 `0NN_unsubscribe_token.sql` ("next free"). The WP-34 vs
    WP-50 `023` clash is REAL.
  - **Highest existing migration verified: `022_concede_bot_replacement.sql`**,
    by `ls rust/web/migrations/` (read-only). `find` located exactly one
    `migrations` dir under `rust/`. So every spec's "022 is highest" premise is
    correct as of today. UNKNOWN: whether migrations exist embedded elsewhere
    (nothing found).
  - `landing-order.md` 6.4 rewritten to a four-row table + plain renumbering
    rule (only the first lander uses `023`; 2nd/3rd/4th renumber to the
    then-next free number and must not collide with each other; re-`ls` the
    dir immediately before writing rather than trusting the spec's number).
    6.5 collapsed to a pointer. 7.3 updated to include WP-34. The old
    `WP-50 -> WP-78` bullet in 6.4 marked WITHDRAWN.
  - Task B was **plain staleness, not a factual conflict** (landing-order 7.3
    is the later record, explicitly withdraws the old constraint, and
    `work-packages.md` marks WP-78 SUPERSEDED). `specs/WP-50-email-canonicalization.md`
    header now states `WP-82 -> WP-50`.
  - W1 flagged but did not fix: WP-50 section 4 still says "no `db.rs`
    restructuring (WP-78)"; WP-34 and WP-50 still hard-code `023` with no
    pointer to the renumbering rule. -> handed to W2.
- [W2 done] **ACCEPTED.** Mop-up of the stale refs W1 flagged.
  - `specs/WP-50-email-canonicalization.md`: section 4 non-goal now says the
    `db.rs` restructuring is **WP-82** (WP-78 superseded by it); section 3e
    renamed to `0NN_canonical_emails.sql` with an explicit "023 is NOT
    guaranteed" warning, a pointer to `landing-order.md` 6.4, an instruction to
    `ls rust/web/migrations/` immediately before writing, and a note to keep the
    `RAISE EXCEPTION` message in step with the real number.
  - `specs/WP-34-auth-races-session-mechanical.md`: header Files list and the
    section 3 migration paragraph both renamed to `0NN_login_email_sends.sql`
    with the same pointer; the old "verify nothing above 022 exists" phrasing
    replaced by the fresh-`ls` instruction.
  - Grep confirmed no other `WP-78` mentions in either file beyond the WP-50
    header passage that already states the supersession.
  - Left deliberately (recorded, not fixed): WP-50 still contains two literal
    `023` strings inside the `DO $$` block's exception message and in a
    regression-test description; the 3e pointer tells the implementer to update
    them.
- [W3 done] **ACCEPTED.** `work-packages.md` - WP-83 entry, WP-84 check, totals.
  - **WP-83 entry ADDED** under a new `## Released from the rules park
    2026-07-26 (D-35)` section, numerically between WP-82 and WP-84. READY,
    spec cross-referenced, scope = `a F1` / `b F7` / `e F30` seat-order half,
    no `RULES.md` edits, `b F4` re-parked and `d F37` rejected called out as
    explicitly out of scope. Lead verified the heading exists.
  - **Coverage bookkeeping settled from evidence, not assumption: WP-83 is a
    CARVE-OUT, not a re-assignment.** The three findings STAY counted under
    WP-12 / WP-16 / WP-30 - each of those entries still lists them in its own
    Scope line, and WP-30's "3, was 5" drop is attributed solely to `e F39` /
    `e F40` moving to WP-81. WP-81 is the re-assignment precedent and says so
    explicitly; nothing equivalent exists for WP-83. **WP-83 adds 0 to the 570
    sum; no coverage row changes.** Stated in the entry.
  - **WP-84 entry needed ONE correction:** its landing-order bullet read
    `WP-82 -> WP-47 -> WP-42 -> WP-84`, implying all of WP-42 gates it. The
    spec gates only WP-42's **predicate work**. Corrected. Otherwise accurate
    (two-stream, all conditionality removed, D-44..D-50, WP-42 Task B deleted).
  - **TOTALS RECOUNTED RELIABLY** (grep over `^### WP-` headings; the three
    status buckets sum exactly to the heading count):
    **84 headings = 77 READY + 6 BLOCKED-ON-USER-RULES-REVIEW + 1 SUPERSEDED.**
    Parked 6 = WP-11, 12, 16, 20, 26, 30. Superseded 1 = WP-78 (by WP-82).
    **`BLOCKED-ON-DECISION`: 0 headings - extinct, confirmed by grep.**
    Written as a dated block; the three earlier dated updates kept as history.
    Per-package findings table and the 570 sum untouched.
  - Noticed, NOT fixed (recorded for the handover): the prose above the
    coverage table still names only WP-74/WP-75 as the zero-finding
    exceptions, but that set has grown to WP-74..WP-80, WP-82, WP-83, WP-84;
    and the file's section headings are chronological, so numeric ordering
    only holds while each new package takes the next free number.

## 2026-07-26 - Unit: planning-corpus inconsistency cleanup (RESTART Lead)

The previous Lead of this unit died on quota. **Step 0 state audit, by reading
only** (no shell mutations, nothing under `rust/` touched):

- **Item 1 (four-package migration collision): ALREADY DONE.** `landing-order.md`
  6.4 carries the four-row table (WP-34 / WP-50 / WP-56 / WP-58), names the
  WP-34 vs WP-50 `023_*.sql` filename clash, and states the renumbering rule.
  6.5 is a pointer; 7.3 includes WP-34. Independently re-verified by this Lead:
  `rust/web/migrations/` highest is `022_concede_bot_replacement.sql`.
- **Item 2 (WP-50 stale ordering): ALREADY DONE.** `specs/WP-50-*.md` header
  now states `WP-82 -> WP-50` and calls the old `WP-50 -> WP-78` withdrawn;
  section 3e is `0NN_canonical_emails.sql` with the renumber warning.
- **Item 3 (WP-83 entry): ALREADY DONE.** `work-packages.md` has the
  `## Released from the rules park 2026-07-26 (D-35)` section with WP-83;
  WP-84's entry exists and its landing-order bullet was corrected.
- **Item 5 (totals): ALREADY DONE.** Recount block present: 84 = 77 READY +
  6 BLOCKED-ON-USER-RULES-REVIEW + 1 SUPERSEDED. Re-verified by heading grep.
- **Item 4 (README): PARTIAL - this is the dead predecessor's half-applied
  edit.** The banner and file map WERE updated (both proposals marked APPLIED,
  totals 84, four-package collision, session-3 files listed), but two later
  sections still contradict it: the "Corrections to the tiering" section still
  says WP-04/05/46/55/58/64/66/67 have no spec and WP-48/50/69/70/71/72/73 have
  no checklist (all but WP-48/72 now have specs), and the Implementer rules
  section still says "Two doc proposals are unapplied". Verified live:
  `docs/CODING.md` has `## Request-Path Invariants`; `docs/BACKLOG.md` has
  items #53 and #54. -> dispatched to W4.
- Also verified live: 59 spec files in `specs/`, 8 checklists in `checklists/`.

**`specs/WP-84-sse-migration.md` integrity spot-check: PASSES.** Read by the
Lead (406 lines). Header is complete (findings, D-44..D-50, landing order with
"WP-42's predicate work" only, length justification, the SETTLED banner).
Section numbering runs 1..9 with no gaps: regression tests are §8 and riders
§9, exactly as `landing-order.md` 10.4 claims. Every internal cross-reference
(§3a, §3c, §3d, §3e, §3g, §4, §7, §8) resolves to a heading that exists. The
only surviving mention of the single-stream fallback is the banner sentence
recording its deletion. No TODO/TBD/blocked markers, no orphan conditionality.
Not redesigned; integrity only.

- [Lead] `ORCHESTRATOR-HANDOVER.md` "State: what REMAINS" updated: the
  "Package totals: UNKNOWN" paragraph replaced with the recounted 84 = 77 + 6 +
  1 figure and its method; the "Known drift" paragraph (no WP-83 entry, stale
  README) replaced with a drift-cleared note; a new open-items list added
  carrying forward the three things this unit recorded but did not fix
  (work-packages zero-finding prose, chronological heading order, WP-50's two
  literal `023` strings).
- [W4 done] **ACCEPTED.** `README.md` - the dead predecessor's half-applied
  item 4 finished. Lead verified the edits landed by grep.
  - "Corrections to the tiering" rewritten: the 8 formerly-blocked Tier 2
    packages (WP-04, 05, 46, 55, 58, 64, 66, 67) and the formerly-blocked
    Tier 3 set (WP-48, 50, 69, 70, 71, 73) all have specs now; WP-72 has no
    file by design (inside `specs/WP-69-deny-toml-hardening.md`, confirmed by
    reading that spec); WP-17, WP-81 and the three parity carve-outs are
    specced. Closing bullet now agrees with the banner that WP-76/77/79/80 is
    the only remaining gap.
  - "Implementer rules": the "two doc proposals are unapplied" bullet replaced
    with a one-line historical note (both applied 2026-07-26, do not re-apply).
  - `## The tiering` gained a lead-in marking its three bullets as the
    ORIGINAL plan's tiering, and a dead pointer to "note 2 in
    `open-decisions-for-user.md`" was removed (that file is a stub - read).
  - **Correction to the Lead's brief, accepted:** the brief said WP-48 was
    still spec-less. It is not - `specs/WP-48-export-import.md` exists.
    **WP-72 is the only genuinely spec-less package** among those named.
  - W4 verified by `ls`/read: 59 `WP-*.md` specs, 8 checklists T3-B1..B8,
    WP-69 spec's WP-72 statement, WP-83's three carve-out sections, WP-17's
    D-25 constraint, and `open-decisions-for-user.md` being a stub.
  - Recorded, NOT fixed (File map was off-limits, and scope discipline):
    (a) the File map row for `decisions-needed.md` says "D-1..D-41" while
    `decisions-session3.md` covers D-41..D-51 - possible stale range;
    (b) whether D-41..D-51 have been folded into `decisions-ANSWERED.md` is
    **UNKNOWN** - not verified by anyone this unit;
    (c) `tier2-tier3-plan.md` is still stale at source - only README's
    description of it was corrected.

**UNIT COMPLETE.** All five items landed; WP-84 integrity confirmed.

---

## Unit: D-52 / D-53 application (Lead, 2026-07-26)

Scope: apply the two rulings just confirmed by Michael in
`decisions-session3.md`. Write-scope limited to `planning/` + `docs/BACKLOG.md`.
One Worker, opus.

- [W1 done] **ACCEPTED.** All three items landed; Lead re-verified each by
  reading the changed regions.
  - **D-53 - `docs/BACKLOG.md` #54 placement.** Priority block header re-dated
    `(updated 2026-07-24)` -> `(updated 2026-07-26)`. #54 appended to the
    **"Then"** tier after #15, reading: "#54 maximum-performance fuzzer (after
    #31, which reworks the workspace layout the fuzz bins sit on; a faster
    fuzzer makes every subsequent game port and remediation package cheaper to
    validate)". Nothing reordered. Consistency fix in the #54 Status row:
    "unscheduled, nothing measured" -> "scheduled 2026-07-26 into the Then tier
    after #31 (D-53), nothing measured"; rest of the row untouched.
  - **D-52 - WP-84 topic cap.** The **number was already 16** and over-cap was
    already a **400** with no truncation anywhere in the file - no number
    changed. One real hedge did need fixing: the cap bullet said "the
    implementer may pick a different small number within that intent", an
    explicit licence to deviate. `specs/WP-84-sse-migration.md` section 3c now
    reads "**Cap N at 16 (D-52 - Michael's ruling, not a proposal).** ... the
    implementer must use **16**", restating 400-not-truncation.
  - **ORCHESTRATOR-HANDOVER.md open items.** Intro now says two of the four are
    resolved and must not be re-raised. Item 1 (topic cap) struck through,
    marked RESOLVED 2026-07-26 - D-52. Item 4 (#54 priority) struck through,
    marked RESOLVED 2026-07-26 - D-53. Items 2 and 3 untouched, numbering
    preserved.
  - **Recorded, NOT fixed** (out of the unit's stated scope): the handover's
    "Where to start reading" and "State: what is DONE" still describe
    `decisions-session3.md` as covering **D-41..D-51**; it now runs to **D-53**.
    Same class as the stale-range note already logged for `README.md` above.
    Also unchanged: `decisions-session3.md` is not in numeric order - D-52 and
    D-53 sit between D-50 and D-51, with D-42 last.

**UNIT COMPLETE.** Both rulings applied and verified; no number was silently
changed.

---

## 2026-07-26 - Unit: handover cleanup-pass section (Lead + 1 Worker)

Scope: append one section to `ORCHESTRATOR-HANDOVER.md` instructing the successor
Orchestrator to run a cleanup and consolidation pass. Focused unit, one document.

WRITES:
- `ORCHESTRATOR-HANDOVER.md` - appended `## NEXT SESSION: cleanup and consolidation
  pass` at the end. File went 279 -> 374 lines (+95). Existing 279 lines untouched;
  append-only. VERIFIED by the Lead reading lines 276-375.

Section contents: six ordered items - (1) disambiguate `specs/` (~26 compact Tier 2
vs ~25 bloated pre-tiering specs, no filename marker; highest-risk item; archive or
delete the superseded ones; decide BY READING, not filename/date/size); (2) merge
`decisions-needed.md` + `open-decisions-for-user.md` + `decisions-ANSWERED.md`
(D-01..D-34) + `decisions-session3.md` (D-41..D-53) into one sorted `DECISIONS.md`
(session3 is not in numeric order); (3) retire the process exhaust - `specs-LOG.md`,
`ORCHESTRATOR-HANDOVER.md` itself, `tier2-tier3-plan.md`; (4) replace the handover
with a short `EXECUTION-README.md` (landing order, WP-82-first, migration
renumbering across FOUR packages WP-34/50/56/58, parked list plus the two
non-reopenable parity items, line-number and proportionality lessons); (5) refresh
`README.md` LAST; (6) decide WP-76/77/79/80 by reading `landing-order.md` and
`work-packages.md`. Plus a `### Ground rules` subsection: same read-only/opus/serial
rules, and destructive moves LAST and in ONE pass so a dying Lead leaves the corpus
readable rather than half-dismantled.

NOT ADDED to the section (reported to Michael instead, his call - out of brief scope):
other retirement candidates spotted by `ls` only, contents UNKNOWN -
`BACKLOG-note-proposed.md` and `CODING-md-amendment-proposal.md` (both already
recorded as APPLIED/historical), `ws-to-sse-evaluation.md` (superseded in its
recommendation), `triage-LOG.md` (4.9KB, same exhaust family), and
`raw/cathedral-stray-edits.diff` (a diff against `rust/` living in the planning
tree - needs a deliberate keep-or-drop call before committing). Also noted:
`decisions-needed.md` is 81KB, by far the largest of the four decision docs, so item
2's merge needs a real strategy and unit-sizing, not a copy-paste.

Sizing observation, recorded as CORRELATION not proof: `specs/` holds 51 `WP-*`
files plus `notes-conventions.md`; sizes cluster bimodally (~4.8-17KB vs ~30-197KB)
and the large ones skew Jul 25 while the small ones skew Jul 26. Consistent with the
~26/~25 split, but item 1 explicitly forbids deciding on this basis.

ACCEPTED by the Lead. No `rust/` writes, no cargo, no git mutations.
