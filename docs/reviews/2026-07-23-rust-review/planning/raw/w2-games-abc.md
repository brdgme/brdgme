# W2 triage rows: games-batch-a / games-batch-b / games-batch-c

## Tallies (post-verification)
- games-batch-a: 20 findings (0c/6M/6m/8n) - matches expected; 0 rejected, F9 wording-adjusted only
- games-batch-b: 35 findings (1c/5M/13m/16n) - matches expected; F9 adjusted nit->minor
- games-batch-c: 34 findings (0c/4M/14m/16n) - matches expected; F23 adjusted minor->nit

## Rows

games-batch-a F1 | major | roll() re-matches phase after keep_skulls() advanced it; extra roll skipped / reroll lost | game/roll-through-the-ages-2/src/lib.rs | D | rtta-roll-phase: fix vs document as Go quirk (test currently locks `next`-path behaviour)
games-batch-a F2 | minor | RULES.md claims developments globally unique; code is per-player (code correct) | game/roll-through-the-ages-2/RULES.md | M | rtta-rules-doc
games-batch-a F3 | minor | RULES.md says pestilence hits every player; roller is exempt in code | game/roll-through-the-ages-2/RULES.md | M | rtta-rules-doc
games-batch-a F4 | minor | RULES.md says skulls never rerolled; Leadership extra roll can reroll them | game/roll-through-the-ages-2/RULES.md | M | rtta-rules-doc
games-batch-a F5 | nit | Food cap of 15 undocumented in RULES.md | game/roll-through-the-ages-2/RULES.md, game/roll-through-the-ages-2/src/lib.rs | M | rtta-rules-doc
games-batch-a F6 | minor | command() repeats finished-scores epilogue 11x (~110 lines) | game/roll-through-the-ages-2/src/lib.rs | M | epilogue-dup
games-batch-a F7 | nit | build_parser ship bound uses max(wood,cloth) not min, ignores 5-ship cap | game/roll-through-the-ages-2/src/command.rs | D | rtta-go-fidelity: fix parser bound vs quirk comment
games-batch-a F8 | nit | Quarrying +1 bypasses per-type good cap; undocumented quirk | game/roll-through-the-ages-2/src/player_board.rs | M | rtta-quirk-comments
games-batch-a F9 | nit | roll() bounds check n < 0 admits n == 0, which still burns the reroll | game/roll-through-the-ages-2/src/lib.rs | D | rtta-go-fidelity: fix guard vs keep Go wart
games-batch-a F10 | nit | Pointless `let mut logs = logs;` rebind in discard() | game/roll-through-the-ages-2/src/lib.rs | M | rtta-cleanup
games-batch-a F11 | major | Cannon cost surcharge checks booster count instead of cannon count | game/starship-catan-1/src/lib.rs | M | starship-logic
games-batch-a F12 | major | can_lose_module uses ||; voluntary module sacrifice skips any pirate | game/starship-catan-1/src/lib.rs | M | starship-logic
games-batch-a F13 | major | TradeAndBuild buys never check astro affordability; players go negative | game/starship-catan-1/src/lib.rs | M | starship-logic
games-batch-a F14 | major | Unbounded buy/sell amounts overflow i32; debug panic from player input | game/starship-catan-1/src/command.rs, game/starship-catan-1/src/lib.rs | M | starship-logic
games-batch-a F15 | major | Sensor peek never rendered to peeking player; module unusable for humans | game/starship-catan-1/src/render.rs | M | starship-render
games-batch-a F16 | minor | "Current turn:" row shows viewer instead of current player | game/starship-catan-1/src/render.rs | M | starship-render
games-batch-a F17 | minor | Dead code: next_turn, Transaction::gain, Module::description/join_dice, start_card | game/starship-catan-1/src/lib.rs, game/starship-catan-1/src/card.rs | M | starship-cleanup
games-batch-a F18 | nit | Direction-mismatch error interpolates attempted direction, not card's | game/starship-catan-1/src/lib.rs | M | starship-cleanup
games-batch-a F19 | nit | last_sectors grows unbounded and is rendered in full | game/starship-catan-1/src/lib.rs, game/starship-catan-1/src/render.rs | M | starship-cleanup
games-batch-a F20 | nit | flight_actions BTreeMap<usize,bool> only ever stores true; should be a set | game/starship-catan-1/src/lib.rs | D | starship-cleanup: serde shape change vs leave as-is
games-batch-b F1 | major | Halicarnassus B DrawDiscard wonder-stage VP never scored (-3 VP) | game/seven-wonders-1/src/lib.rs | M | seven-wonders-scoring
games-batch-b F2 | major | DrawDiscard resolver permanent soft-lock when all discards already owned | game/seven-wonders-1/src/lib.rs | M | seven-wonders-rules
games-batch-b F3 | major | Auto-discarded 7th card of each age wrongly pays 3 coins | game/seven-wonders-1/src/lib.rs | M | seven-wonders-rules
games-batch-b F4 | minor | Same-turn trade of freshly built resources (asymmetric by player index) | game/seven-wonders-1/src/lib.rs | D | seven-wonders-rules: snapshot tradable goods vs document deviation
games-batch-b F5 | minor | MimicGuild can only copy Bonus-effect guilds, not Science guilds | game/seven-wonders-1/src/lib.rs | D | seven-wonders-rules: extend mimic vs document restriction
games-batch-b F6 | minor | Wonder-stage sacrifice card enters shared discard pile (contradicts own RULES.md) | game/seven-wonders-1/src/lib.rs | D | seven-wonders-rules: drop push vs Go-parity note
games-batch-b F7 | minor | Both sides of one wonder can be dealt in a game | game/seven-wonders-1/src/lib.rs, game/seven-wonders-1/src/card.rs | D | seven-wonders-rules: distinct boards (RNG determinism ordering) vs document
games-batch-b F8 | minor | Discard pile contents hidden from all players; Halicarnassus takes blind | game/seven-wonders-1/src/lib.rs | D | seven-wonders-privacy: add discard to PubState (privacy shape)
games-batch-b F9 | minor | Chosen trade deal re-validated by index into recomputed list; wrong-neighbor pay / free build reachable | game/seven-wonders-1/src/lib.rs | M | seven-wonders-trade
games-batch-b F10 | nit | Unguarded player indexing in player_state/command_parser vs sibling crates | game/seven-wonders-1/src/lib.rs, game/seven-wonders-1/src/command.rs | M | seven-wonders-cleanup
games-batch-b F11 | nit | Finished-game scoring block copy-pasted six times in command() | game/seven-wonders-1/src/lib.rs | M | epilogue-dup
games-batch-b F12 | nit | Military-conflict log uses raw player index instead of N::Player | game/seven-wonders-1/src/lib.rs | M | seven-wonders-cleanup
games-batch-b F13 | nit | start_hand() is dead-weight indirection | game/seven-wonders-1/src/lib.rs | M | seven-wonders-cleanup
games-batch-b F14 | minor | Test coverage gaps: conflicts, guild scoring, Halicarnassus B VP, deals, determinism (MimicGuild IS tested) | game/seven-wonders-1/src/lib.rs | M | seven-wonders-tests
games-batch-b F15 | nit | lib.rs is a 1,565-line grab-bag; scoring/trade could split out | game/seven-wonders-1/src/lib.rs | M | seven-wonders-cleanup
games-batch-b F16 | critical | take() mints duplicate cards - money-duplication exploit via `take b1 b1` | game/alhambra-1/src/lib.rs | M | alhambra-dup-cards
games-batch-b F17 | major | place indices diverge from rendered indices after placement; Empty-tile corruption | game/alhambra-1/src/lib.rs, game/alhambra-1/src/render.rs | M | alhambra-place-index
games-batch-b F18 | major | grid_longest_ext_wall unconditional break undercounts wall; also nondeterministic (HashMap order) | game/alhambra-1/src/card.rs | M | alhambra-wall-walk
games-batch-b F19 | minor | Dirk excluded from final placings in 2-player games | game/alhambra-1/src/lib.rs | D | alhambra-rules: include Dirk vs document deviation
games-batch-b F20 | minor | Reduced 72-card money deck for 2-player games vs official 108 | game/alhambra-1/src/card.rs | D | alhambra-rules: verify 2p deck size vs rulebook
games-batch-b F21 | minor | Tests miss take-multiplicity, place-index, wall-walk diagonal, final-place ties, Dirk flows | game/alhambra-1/src/lib.rs | M | alhambra-tests
games-batch-b F22 | nit | is_finished() epilogue copy-pasted into six command arms | game/alhambra-1/src/lib.rs | M | epilogue-dup
games-batch-b F23 | nit | Invariant-guarded panics could be expect() naming the invariant | game/alhambra-1/src/lib.rs, game/alhambra-1/src/command.rs, game/alhambra-1/src/card.rs | M | alhambra-cleanup
games-batch-b F24 | nit | Gap-check loop range asymmetry (x inclusive, y exclusive) reads like off-by-one | game/alhambra-1/src/card.rs | M | alhambra-cleanup
games-batch-b F25 | nit | Debug {:?} formatting in user-facing messages (5 sites) | game/alhambra-1/src/lib.rs | M | alhambra-cleanup
games-batch-b F26 | nit | tile_counts duplicated between render.rs and PlayerBoard | game/alhambra-1/src/render.rs, game/alhambra-1/src/card.rs | M | alhambra-cleanup
games-batch-b F27 | nit | Grid column headers wrap past 26 columns (practically unreachable) | game/alhambra-1/src/render.rs | M | alhambra-cleanup
games-batch-b F28 | nit | Vec-as-queue and HashMap-as-set in flood walks | game/alhambra-1/src/card.rs | M | alhambra-cleanup
games-batch-b F29 | minor | Prestige ties broken by MOST cards instead of fewest (Go-parity, test locks it in) | game/splendor-2/src/lib.rs | D | splendor-rules: fix tie-break vs keep Go parity documented
games-batch-b F30 | minor | take() action layer never validates requested tokens are gems (parser-only guard) | game/splendor-2/src/lib.rs | M | splendor-hardening
games-batch-b F31 | minor | Local cost.rs vs lib/cost: add get/set to lib/cost then migrate; keep gold-joker can_afford | game/splendor-2/src/cost.rs, lib/cost | M | lib-cost-consolidation
games-batch-b F32 | nit | reserve parser offers row-3 own-reserve locations; stale test comment | game/splendor-2/src/lib.rs, game/splendor-2/src/command.rs | M | splendor-hardening
games-batch-b F33 | nit | is_finished() epilogue copy-pasted into five command arms | game/splendor-2/src/lib.rs | M | epilogue-dup
games-batch-b F34 | nit | "remaning" typo in user-facing error (Go-parity typo, safe to fix) | game/splendor-2/src/lib.rs | M | splendor-cleanup
games-batch-b F35 | nit | .expect() in visit_phase auto-visit (verified unreachable) | game/splendor-2/src/lib.rs | M | splendor-cleanup
games-batch-c F1 | minor | Raise parser min uses largest_raise not min_raise(); "Go quirk" comment factually wrong | game/texas-holdem-2/src/command.rs | M | texas-holdem-parity
games-batch-c F2 | minor | MAX_PLAYERS 8 vs Go's 9; undocumented divergence | game/texas-holdem-2/src/lib.rs | D | texas-holdem-parity: restore 9 vs document cap (player-count cap)
games-batch-c F3 | nit | bet_up_to uses .expect() in runtime path | game/texas-holdem-2/src/lib.rs | M | texas-holdem-cleanup
games-batch-c F4 | nit | Documented Go-mirroring panics in next_player_in_set / pop_n (all guarded) | game/texas-holdem-2/src/lib.rs, game/texas-holdem-2/src/card.rs | M | texas-holdem-cleanup
games-batch-c F5 | nit | HandResult.category Option<Category> redundant with Category::None variant | game/texas-holdem-2/src/poker.rs | M | texas-holdem-cleanup
games-batch-c F6 | nit | Placings-log block duplicated across all five command() arms | game/texas-holdem-2/src/lib.rs | M | epilogue-dup
games-batch-c F7 | major | player_counts() returns (2..6), excluding 6 players; 6p games never offered | game/acquire-1/src/lib.rs | M | acquire-fixes
games-batch-c F8 | major | 2-player dummy shareholder die roll is 1..=5, never 6 (contradicts own log and RULES.md) | game/acquire-1/src/lib.rs | M | acquire-fixes
games-batch-c F9 | minor | panic! in pay_bonuses on empty major-bonus list (runtime path) | game/acquire-1/src/lib.rs | M | acquire-hardening
games-batch-c F10 | minor | expect() cluster panics on deserialized state missing HashMap keys; render path too; typo | game/acquire-1/src/lib.rs, game/acquire-1/src/command.rs, game/acquire-1/src/render.rs | M | acquire-hardening
games-batch-c F11 | minor | "Trades" stat reports merge count (copy-paste) | game/acquire-1/src/stats.rs | M | acquire-stats
games-batch-c F12 | minor | Stats tracked but never surfaced; to_brdgme_stats has zero callers | game/acquire-1/src/lib.rs, game/acquire-1/src/stats.rs | D | acquire-stats: wire into status() vs delete machinery
games-batch-c F13 | minor | Start player chosen randomly instead of by initial tile draw | game/acquire-1/src/lib.rs | D | acquire-edition: rulebook setup vs document simplification
games-batch-c F14 | minor | Full-hand redraw permanently discards temporarily-unplayable tiles | game/acquire-1/src/lib.rs, game/acquire-1/src/board.rs | D | acquire-edition: which edition's redraw rule
games-batch-c F15 | minor | Tile-bag exhaustion ends game mid-turn; edition-dependent behaviour (compounds with F14) | game/acquire-1/src/lib.rs | D | acquire-edition: bag-exhaustion behaviour
games-batch-c F16 | minor | Unused thiserror dependency | game/acquire-1/Cargo.toml | M | acquire-cleanup
games-batch-c F17 | nit | can_undo in handle_found_command is a tautology | game/acquire-1/src/lib.rs | M | acquire-cleanup
games-batch-c F18 | nit | unwrap() on single-element neighbouring_corps set | game/acquire-1/src/lib.rs | M | acquire-hardening
games-batch-c F19 | nit | unwrap() in board render row-run logic | game/acquire-1/src/render.rs | M | acquire-hardening
games-batch-c F20 | nit | Full-game deep clone per command_parser build just to compute can_end | game/acquire-1/src/lib.rs | M | acquire-cleanup
games-batch-c F21 | nit | Nondeterministic HashSet corp ordering in found parser | game/acquire-1/src/command.rs | M | acquire-cleanup
games-batch-c F22 | major | Box::leak of 100 strings per parser construction - unbounded leak per request | game/cathedral-2/src/command.rs | M | cathedral-leak
games-batch-c F23 | nit | Cathedral traversable by capture flood-fill - documented in RULES.md; add walk-site comment | game/cathedral-2/src/lib.rs | M | cathedral-docs
games-batch-c F24 | minor | Dead code: parse_loc never called (pub suppresses lint) | game/cathedral-2/src/loc.rs | M | cathedral-cleanup
games-batch-c F25 | minor | pieces() panics on out-of-range player index; reachable via unvalidated harness forward | game/cathedral-2/src/piece.rs | M | cathedral-hardening
games-batch-c F26 | nit | Loc::to_key arithmetic overflow on out-of-range coords; Game::tile_at unguarded | game/cathedral-2/src/loc.rs, game/cathedral-2/src/lib.rs | M | cathedral-hardening
games-batch-c F27 | nit | Unused rand dependency | game/cathedral-2/Cargo.toml | M | cathedral-cleanup
games-batch-c F28 | nit | Dead impl Display for Loc | game/cathedral-2/src/loc.rs | M | cathedral-cleanup
games-batch-c F29 | major | Steal n = i32::MIN overflows len - n; panic in debug/overflow-check builds | game/sushizock-2/src/lib.rs, game/sushizock-2/src/command.rs | M | sushizock-fixes
games-batch-c F30 | minor | Game end via forced take_worst (roll path) never emits placings log | game/sushizock-2/src/lib.rs | M | sushizock-fixes
games-batch-c F31 | minor | roll suggest offers dice past legal count - user-visible impact of lib/game Many-ignores-max bug | game/sushizock-2/src/command.rs, lib/game | M | lib-game-suggest (fix lives in lib/game, not this crate)
games-batch-c F32 | nit | .unwrap() in roll_dice runtime path (infallible but banned) | game/sushizock-2/src/lib.rs | M | sushizock-cleanup
games-batch-c F33 | nit | take_worst hand-rolled min loops, duplicated branches, implicit non-empty invariant | game/sushizock-2/src/lib.rs | M | sushizock-cleanup
games-batch-c F34 | nit | take_blue/red and steal_blue/red near-verbatim duplicates (Go-parity duplication) | game/sushizock-2/src/lib.rs | M | sushizock-cleanup

## Grouping notes

- Cross-crate pattern: copy-pasted finished-game scores/placings epilogue in command() - slug "epilogue-dup" covers 5 crates here (rtta F6 x11, seven-wonders F11 x6, alhambra F22 x6, splendor F33 x5, texas-holdem F6 x5). Same shape, same fix (extract per-arm logs then run one shared finish block); a good single sweep package, though each crate's edit is independent.
- Cross-crate pattern: deserialized-state / bad-input trust. Panic-or-corruption paths that hold only under fresh-game invariants: acquire F9/F10 (expect on HashMap keys, real concern given the crate's existing legacy-state migration shim), cathedral F25 (harness forwards player index unvalidated - requester/gamer.rs:130), sushizock F29 and starship F14 (unbounded player ints overflowing i32). Hardening slugs per crate, but a common "no panic reachable from requests" pass could own them.
- Rules-vs-Go-port decisions cluster by crate: rtta-go-fidelity (F7, F9, and arguably F1), seven-wonders-rules (F4-F7; F2/F3 are unambiguous bugs since PORTING_NOTES/official rules agree), alhambra-rules (F19, F20 - no Go source exists, judged on official rules only), splendor-rules (F29 - verified Go parity, test locks in the inversion), texas-holdem-parity (F2 player cap). One design session per crate can settle its whole batch.
- acquire-edition (F13, F14, F15) is a single decision: which Acquire edition's rules to follow for setup/redraw/bag-exhaustion. F14+F15 compound (verification: mass redraw can drain the bag and trigger premature end), so decide together.
- games-batch-b F9: verification INVALIDATED the original "verified unreachable" analysis - the deal list CAN be reordered/shrunk mid-execution (lib/cost early-return + execute_actions mutation order), making wrong-neighbor payment or free build reachable. The original recommendation (store the chosen deal at choose_deal time) is still the right fix and was not disputed; only the risk assessment changed (hence nit->minor).
- games-batch-c F23: verification refuted the finding's premise (cathedral RULES.md documents the behaviour as intended); the surviving work item is only a walk-site code comment referencing RULES.md, not a behaviour change. Do not "fix" the flood-fill.
- games-batch-a F1: verification confirmed the bug, but note the crate's own test locks in the `next`-command path behaviour - fixing roll() requires deciding whether the `next` path or the `roll` path is canonical (hence D).
- games-batch-b F18 (alhambra wall walk): verification strengthened it - result is also nondeterministic across runs because Grid is a HashMap; fix (move break inside the if) is uncontroversial, keep with alhambra-wall-walk plus the diagonal-blocker regression test from F21's list.
- alhambra majors are three separate mechanisms (take multiplicity, place indexing, wall walk) but share the crate and F21's missing tests enumerate exactly these paths - natural package: fix all three + add the F21 test list in one go.
- starship-logic (F11-F14) are four independent small fixes in one file plus a parser bound; one package. F20 is the only starship item needing a decision (serialized-shape change).
- lib-cost-consolidation (splendor F31) touches lib/cost first (add get/set), benefiting seven-wonders-1 too; sequence it before or independently of splendor cleanup.
- lib-game-suggest (sushizock F31) is a cross-reference: no crate-local fix; resolves when the lib/game Many-ignores-max suggest bug (tracked in another unit) is fixed. Include only to link the user-visible impact.
- Doc-only fixes (rtta F2-F5 RULES.md edits, cathedral F23 comment) are trivial and can ride along with any package touching the crate.
