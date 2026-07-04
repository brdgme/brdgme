# Phase 23: Rust Game Ports

**Status:** Pending

Port games to Rust game crates. Decision to target Rust over Go (2026-07-04):
`docs/GO_VS_RUST_PORTING.md`. Method: `docs/GAME_PORTING.md`. Per-game
analysis: `docs/GAME_PORTING_PLAN.md`. Template: `rust/game/lost-cities-1`.

Each port includes: `rust/game/<name>-N` crate + tests, workspace member,
Dockerfile stage, Tiltfile entry, `k8s/base/game/<name>-N` manifests
(deployment/service/GameVersion), prod kustomization image entry.

Test policy: 1:1 porting of all existing Go tests is required (they are the
executable rules spec proving behaviour is preserved), plus the standard
contract test; games with thin/no suites get baseline command + scoring
tests written during the port. See GAME_PORTING.md step 8.

**Library prerequisites:**

- [ ] Rust cost/permutation module (port Go `libcost`; blocks seven_wonders-1
      and splendor-2)
- [ ] Rust poker hand evaluation (port Go `libpoker`; blocks texas-holdem-2)

**Track A - new ports from the old Go project (suggested order):**

- [ ] tic-tac-toe-1 (small; 2p, no hidden info; Rust-port warm-up)
- [ ] jaipur-1 (medium; 2p, hidden hands, goods enum)
- [ ] red7-1 (medium; 2-4p, in-round eliminations, command chaining)
- [ ] alhambra-1 (large; 2-6p, card enum, own square-grid module)
- [ ] starship-catan-1 (very large; 2p, card enum redesign, 20+ commands)
- [ ] seven-wonders-1 (very large; 3-7p, card enum redesign, simultaneous
      turns; needs cost module)

**Track B - convert brdgme-go games to Rust as `-2` editions** (easier: Go
source already matches platform architecture; mark the `-1` GameVersion
`isDeprecated: true` like lost-cities-1; retiring all 17 removes the Go
stack entirely). Small dice game first to set the rhythm, then by value:

- [ ] liars-dice-2, greed-2, farkle-2, zombie-dice-2, no-thanks-2 (small)
- [ ] category-5-2, battleship-2, for-sale-2, sushizock-2 (small-medium)
- [ ] texas-holdem-2 (needs poker module), sushi-go-2, age-of-war-2,
      modern-art-2, love-letter-2, cathedral-2 (medium)
- [ ] splendor-2 (needs cost module), roll-through-the-ages-2 (large)
- [ ] Retire brdgme-go stack once no active `-1` games remain (Dockerfile,
      Tiltfile entries, Bazel, go.mod)

**Deferred (not ports — old versions incomplete):**

- hive: old code is a stub (no commands, demo render); new Rust development
  incl. hex-grid rendering. (Partial Go bring-over in stash
  `wip-go-hive-chess-port`; superseded, do not build on it.)
- chess: old code is a move-generation engine only, never a playable game;
  game layer would be new development.

