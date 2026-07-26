# Archive - DO NOT EXECUTE

**STOP. Do not act on anything in this directory.**

Every spec in here describes work that has **already fully landed** in `rust/` on `master`.
Executing it would be redundant at best and destructive at worst: these specs contain
line-addressed "replace lines A-B" instructions against files that the landing commits have
since rewritten. Following those instructions now would corrupt correct, shipped code.

These files are kept for **provenance only**. They record what was fixed and why, and they
are the audit trail behind the finding IDs cited in the review.

## Verification

Each spec's own task roster was enumerated, and every task was checked individually against
live committed source on a clean `rust/` tree - not against any summary. The full per-task
evidence (`file:line` plus symbol for each task) is in
[`../../specs-CLASSIFICATION.md`](../../specs-CLASSIFICATION.md), section
`ARCHIVE re-verification against clean committed master (2026-07-27)`.

Result: **all 13 CONFIRMED-LANDED, 0 NOT-LANDED.**

## Contents

| spec | landing commit(s) | tasks confirmed |
|---|---|---:|
| `WP-01-char-byte-panic-elimination.md` | `9abe8b4` | 7/7 |
| `WP-03-lib-game-parser-mechanical.md` | `c39786f` | 8/8 |
| `WP-06-lib-cmd-tools-http.md` | `a543120` | 5/5 |
| `WP-13-starship-catan-fixes.md` | `4e0abe6` | 9/9 |
| `WP-14-alhambra-core-fixes.md` | `c52f1a5` | 10/10 |
| `WP-15-seven-wonders-mechanical.md` | `52680e5` | 9/9 |
| `WP-21-cathedral-sushizock-fixes.md` | `f547238` | 10/10 |
| `WP-25-modern-art-liveness.md` | `6c0c19c`, `e560a75`, `b0babb8`, `af2c014`, `7821938` | 5/5 |
| `WP-36-crypto-deploy-hardening.md` | `13a1e69` | 6/6 (T6 = cargo gate) |
| `WP-37-admin-pass.md` | `b49df61` | 13/13 |
| `WP-39-bot-consumer-supervision.md` | `347970a` | 8/8 (T8 = cargo gate) |
| `WP-41-db-quality-pass.md` | `baa5fc6` | 11/11 |
| `WP-44-proposals-integrity-email-token-leak.md` | `f4e7640` | 10/10 |

## Where the real work is

The live, actionable specs are the ones remaining in the parent directory,
`planning/specs/`. Work only from those.
