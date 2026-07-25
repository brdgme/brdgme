# Consolidation notes: units 1-3 (lib-game, lib-support, games-batch-a)

Worker notes for the Lead. F-IDs are per-unit as assigned in the
verification reports (each unit numbers from F1). Snapshot `f8763a5`.

## Unit 1: lib-game (rust/lib/game, 3,737 LOC)

### Tallies
- Verification-corrected: 3 critical / 4 major / 8 minor / 5 nit (20 total).
- Identical to the curated file's original tally. No REJECTED findings.

### Verification
- Verdicts: 20 CONFIRMED, 0 ADJUSTED, 0 REJECTED, 0 UNVERIFIABLE.
- No rejections, no severity changes. Lead spot-checked F1 and F5 directly;
  both reproduced.
- Factual refinements (verdicts unchanged):
  - F7: 12 GameError::Parse construction sites, not 10; the 2 extras are
    inductively always offset 0, so "ranking is dead" holds exactly.
  - F12: reachability stronger than the original's "latent" framing -
    sushi-go-2/sushizock-2 bounded specs feed spec.doc() via repl.rs:95 and
    notify.rs:94, so the misrendering is user-visible today.
- No recommendations flagged as invalid.

### Headlines
Criticals (all: char-count-used-as-byte-index panics, reachable from raw
user input server-side per command and client-side in the WASM suggest
engine on every keystroke):
- F1: Space::parse panics on multi-byte whitespace - char count byte-sliced;
  NBSP (iOS autocorrect) panics. lib/game/src/command/parser/mod.rs:431.
- F2: Token::parse panics splitting a multi-byte char - byte-length check
  passes while `&input[..t_len]` splits a char. parser/mod.rs:50.
- F3: Enum::parse panics on multi-byte values - shared_prefix returns chars,
  sliced as bytes; reachable via non-ASCII player names through
  Player::parse. parser/mod.rs:641.

Notable majors (all 4):
- F4: Exact Enum with multi-byte values can never match - chars-vs-bytes
  comparison at parser/mod.rs:622; also corrupts full-match priority.
- F5: Enum full-match priority is declaration-order dependent - values
  ["abc","ab"] + input "ab" gives spurious ambiguity; prefix values can be
  unselectable. parser/mod.rs:626.
- F6: Many loops (typed, spec, suggest) lack a zero-progress guard - a
  zero-width item parser loops forever; latent (no in-tree spec) but Spec is
  public + Deserialize. parser/mod.rs:353.
- F7: OneOf "furthest error wins" machinery is dead code - all Parse offsets
  are provably 0, ranking degrades to declaration-order accumulation.
  parser/mod.rs:473.

### Unit state
High-quality crate with genuinely strong tests (~70 suggest tests, typed/
spec parity guards) and clean rng/game/errors/chain modules - but one
systematic defect class dominates: char/byte unit confusion in the
hand-rolled parser produces all 3 criticals, 1 major, and 1 nit, and the
test suite has zero non-ASCII coverage, exactly where the panics live.
Secondary class: typed-vs-spec impl divergence (F8, F13, F14) that the
parity tests do not cover.

### Theme evidence
- Request-reachable panics: F1, F2, F3 (server per-command + WASM
  per-keystroke); F16 (same pattern, latent ASCII-only); F10 (debug-build
  overflow, spec-supplied).
- Boilerplate duplication / impl drift: F8, F13, F14 (typed vs spec impls
  disagree on parse/expected); F6 spans three parallel Many loops.
- Unmaintained or duplicated dependencies: F15 (combine declared, unused -
  dead dep in every game crate).
- Error-swallowing adjacent: F7 (error-ranking machinery silently inert,
  degrading error message quality).
- Missing-test-coverage pattern: no non-ASCII input tests anywhere;
  parity tests cover parse only, not expected.

### Discussion candidates
- F7: fix direction is a design call - implement real offset propagation
  (better OneOf error messages) vs delete the ranking and document
  declaration order.
- F13: typed vs spec Doc::expected divergence may be deliberate (doc name
  as aggregate hint) - needs an intent ruling before aligning.
- F17: case-folding semantics (UniCase vs to_lowercase) - pick one
  convention for suggest vs parse, or document the split.

## Unit 2: lib-support (lib/cmd, game_client, markup, color, cost, rand_bot; ~9.3k LOC)

### Tallies
- Verification-corrected: 1 critical / 5 major / 23 minor / 16 nit (45 total).
- Identical to the curated file's original tally (neither ADJUSTED verdict
  changed severity). No REJECTED findings.

### Verification
- Verdicts: 43 CONFIRMED, 2 ADJUSTED, 0 REJECTED, 0 UNVERIFIABLE.
- Adjustments (severity unchanged):
  - F8 (word_wrap): "collapses runs of spaces" overbroad - mid-line runs
    ARE preserved; only leading spaces and runs at line starts / wrap
    boundaries collapse. Byte-length width measure stands.
  - F15 (palette verbosity): counts overstated - ~2,000 literal lines not
    ~3,000; const-fn rewrite lands ~2,300 lines not ~400. Substance and
    recommendation stand.
- Minor factual refinement: F21 - bot_cli::cli has four unwraps, not three.
- No recommendations flagged as invalid.

### Headlines
Critical:
- F1: markup slice() indexes text by byte offset while all offsets are char
  counts (TNode::len) - any multi-byte char in a {{canvas}} layer panics or
  silently corrupts output; also off-by-one skip check (`<` vs `<=`).
  lib/markup/src/transform.rs:274.

Majors (all 5):
- F2: parse_u8/parse_usize unwrap on overflow - `{{fg rgb(999,1,1)}}`-class
  markup typos panic the process. lib/markup/src/parser.rs:54.
- F3: malformed/unterminated markup silently truncates - many(choice)
  "succeeds" with the tail in rest; all web callers discard rest; a stray
  `{` silently drops everything after it; no escape for literal `{`.
  lib/markup/src/lib.rs:37.
- F12: regex + lazy_static exist solely to serve a dead parse API
  (Color::from_hex/from_str test-only); brdgme_color is the sole regex
  dependent workspace-wide. lib/color/src/lib.rs:51.
- F19: `.unwrap()` in the production warp handler - malformed game JSON in
  a request panics the connection task instead of returning SystemError;
  `impl Reject` started and abandoned. lib/cmd/src/http.rs:54.
- F31: game_client never enforces a timeout despite documenting
  timeout-retry; operator uses bare Client::new() so a hung game pod blocks
  a reconcile worker forever. lib/game_client/src/lib.rs:47.

### Unit state
Quality varies by crate: game_client and cost are near-clean (bounded
retry, good tests), markup carries the worst defects (byte/char confusion,
panic-on-overflow, silent truncation), color's real issue is dependency
footprint serving dead API, cmd is panic-heavy in dev tools with one
production-path unwrap. Dominant classes: panics on reachable input, and
dependency hygiene (unused/dead/duplicated deps in 4 of 6 crates).

### Theme evidence
- Request-reachable panics: F1 (markup canvas), F2 (markup number
  overflow), F19 (warp handler unwrap), F23 (pervasive unwraps in cmd
  runtime paths), F43 (rand_bot panics on degenerate specs).
- Char/byte unit confusion (same class as unit 1 criticals): F1, F8
  (word_wrap byte widths).
- Error-swallowing: F3 (rest silently discarded by all callers), F5
  (eprintln + silent color substitution), F9 (parse diagnostics discarded
  to a unit error), F29 (child exit status unchecked - surfaces as
  confusing JSON error).
- Unmaintained or duplicated dependencies: F12 (regex/lazy_static for dead
  API), F24 (term_size RUSTSEC-2020-0163), F34 (serde_yaml deprecated),
  F40 (chrono unused), F42 (rand_bot pulls warp/tokio/sentry via default
  features), F25 (warp vs axum dual HTTP stacks).
- Boilerplate duplication: F6 (HTML escape duplicated - security-relevant),
  F14 (three divergent color-alias tables), F17 (triplicated sRGB
  linearization), F18 (hex/Display), F39 (splendor-2 re-implements
  lib/cost from the same Go origin), F41 (join logic duplicated vs
  tools/fuzz with divergent separators).
- Privacy/consistency of conventions: F32 (anyhow in a library vs
  thiserror everywhere else), F7 (PLAYER_COUNT hardcoded vs palette).
- Go-port parity: F39 (two divergent ports of libcost/cost.go).

### Discussion candidates
- F3: making non-empty rest an error is a behavior change - audit whether
  any caller legitimately wants streaming/partial parses; escape-syntax
  design for literal `{` is a product/format decision.
- F5: is the unknown-rgb fallback load-bearing backwards compatibility, or
  should it be a parse error?
- F12: delete the Color parse API vs keep-and-reimplement - API surface
  decision.
- F25: warp -> axum consolidation is a stack-strategy call, not a bug fix.
- F39: consolidation direction for cost (splendor-2 adopts lib/cost vs
  inline into seven-wonders-1) - both defensible, review recommends the
  former.
- F8 (word_wrap): decide whether space handling at wrap points is intended
  behavior to document or a bug to fix.

## Unit 3: games-batch-a (roll-through-the-ages-2, starship-catan-1; ~9.0k LOC)

### Tallies
- Verification-corrected: 0 critical / 6 major / 6 minor / 8 nit (20 total).
- Identical to the curated file's original tally. No REJECTED findings.

### Verification
- Verdicts: 19 CONFIRMED, 1 ADJUSTED (F9), 0 REJECTED, 0 UNVERIFIABLE.
- F9 adjustment (nit stands): "silently a no-op" understates - `roll 0`
  rolls nothing but still consumes the reroll/extra roll.
- Evidence strengthenings: F12 - can_lose_module also lacks the
  phase/pirate guards its siblings carry; the recommended `&&` fix remains
  sufficient. F14 - overflow reachable specifically because several trade
  cards have `maximum: 0`, skipping the lib.rs:921 amount cap.
- No recommendations flagged as invalid.

### Headlines
No criticals. All 6 majors are notable:
- F1 (rtta-2): roll() re-matches self.phase after keep_skulls() may have
  advanced it - Leadership extra roll silently skipped on all-skull
  rerolls; next player can lose a reroll via the Revolt cascade.
  game/roll-through-the-ages-2/src/lib.rs:742.
- F11 (starship): cannon cost surcharge checks boosters, not cannons -
  copy-paste from booster_transaction; players over/underpay.
  game/starship-catan-1/src/lib.rs:311.
- F12 (starship): can_lose_module uses `||` - current player can
  voluntarily sacrifice a module to skip any pirate fight/ransom.
  lib.rs:1267.
- F13 (starship): TradeAndBuild buys never check astro affordability -
  legal command sequence drives astro negative. lib.rs:996.
- F14 (starship): unbounded buy/sell amounts overflow i32 - debug-build
  panic reachable from player input (Int::positive() uncapped +
  maximum: 0 cards skip the amount cap). command.rs:121.
- F15 (starship): Sensor peek never rendered to the peeking player
  (`_peeking` unused) - module unusable by humans; data leaks only via
  raw PlayerState JSON to API clients. render.rs:108.

### Unit state
Two very different crates. rtta-2 is a careful, heavily annotated Go port
with strong tests; its one real logic bug (F1) plus a cluster of
RULES.md-vs-code contradictions (F2-F5, F7, F8 - code is Go-faithful, docs
wrong). starship-catan-1 is structurally sound (transactions, redaction,
parser gating) but has three rules/economy bugs reachable by legal play,
a reachable overflow, and a feature-breaking render gap. Boilerplate
binaries and Cargo.tomls clean in both.

### Theme evidence
- Request-reachable panics: F14 (starship i32 overflow from player-typed
  amounts; debug/CI panic).
- Go-port parity vs rules divergence: F1 (Go-inherited but undocumented
  bug), F2/F3/F4/F5 (RULES.md contradicts Go-faithful code), F7/F8
  (undocumented Go quirks); rtta-2 also carries three deliberately
  documented Go quirks (not findings).
- Player-count/amount validation: F13 (no affordability check), F14
  (unbounded amount parser) - parser-gate vs can_* guard gaps.
- Boilerplate duplication: F6 (11 copies of the finished-scores epilogue
  in command()); the 108-near-identical-binaries observation deferred to
  the dependencies unit.
- Privacy/visibility gates not wired: F15 (peek data present in
  PlayerState JSON but never rendered - the render layer, not the
  redaction layer, is the gap); F16 (current-turn row shows viewer).
- Dead code: F17 (next_turn, Transaction::gain, description/join_dice,
  start_card).

### Discussion candidates
- F1 (rtta-2): fix vs Go fidelity - the crate deliberately preserves other
  Go quirks; this one produces objectively wrong state and diverges from
  the crate's own next-path test, but the fix diverges from Go. Needs a
  fidelity-policy ruling.
- F7 (rtta-2): same question smaller - fix the ship bound (diverge from
  Go) or document as a quirk.
- F20 (starship): BTreeMap -> BTreeSet changes serialized state shape -
  saved-state compatibility call.
- F15 (starship): how much of the peek to render, and whether the
  PlayerState JSON exposure to bots is intended.
