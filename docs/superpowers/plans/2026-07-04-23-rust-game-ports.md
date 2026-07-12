# 23: Rust Game Ports - Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.
>
> Extracted 2026-07-08 from `docs/plan/23-rust-game-ports.md`. Task granularity is
> work-package level; run superpowers:writing-plans against the paired spec
> before execution if bite-sized steps are needed.

**Goal:** Port games to Rust game crates.

**Spec:** `docs/superpowers/specs/2026-07-04-23-rust-game-ports-design.md`

## Work package definition

Each port includes: `rust/game/<name>-N` crate + tests, workspace member,
Dockerfile stage, Tiltfile entry, `k8s/base/game/<name>-N` manifests
(deployment/service/GameVersion), prod kustomization image entry.

## Test policy

1:1 porting of all existing Go tests is required (they are the
executable rules spec proving behaviour is preserved), plus the standard
contract test; games with thin/no suites get baseline command + scoring
tests written during the port. See GAME_PORTING.md step 8.

## Library prerequisites

- [ ] Rust cost/permutation module (port Go `libcost`; blocks seven_wonders-1
      and splendor-2)
- [ ] Rust poker hand evaluation (port Go `libpoker`; blocks texas-holdem-2)

## Track A - new ports from the old Go project (suggested order)

- [x] tic-tac-toe-2 (small; 2p, no hidden info; Rust-port warm-up)
- [x] jaipur-2 (medium; 2p, hidden hands, goods enum; complete)
- [ ] red7-1 (medium; 2-4p, in-round eliminations, command chaining)
- [ ] alhambra-1 (large; 2-6p, card enum, own square-grid module)
- [ ] starship-catan-1 (very large; 2p, card enum redesign, 20+ commands)
- [ ] seven-wonders-1 (very large; 3-7p, card enum redesign, simultaneous
      turns; needs cost module)

## Track B - convert brdgme-go games to Rust as `-2` editions

(easier: Go source already matches platform architecture; mark the `-1`
GameVersion `isDeprecated: true` like lost-cities-1; retiring all 17 removes
the Go stack entirely). Small dice game first to set the rhythm, then by
value:

- [x] liars-dice-2, greed-2, farkle-2, zombie-dice-2, no-thanks-2, category-5-2, battleship-2, for-sale-2, sushizock-2, sushi-go-2 (small/small-medium)
- [x] love-letter-2 (medium; 2-4p, deck/hand/discard tracking, complete)
- [ ] texas-holdem-2 (needs poker module), age-of-war-2,
      modern-art-2, cathedral-2 (medium)
- [ ] splendor-2 (needs cost module), roll-through-the-ages-2 (large)
- [ ] Retire brdgme-go stack once no active `-1` games remain (Dockerfile,
      Tiltfile entries, Bazel, go.mod)
