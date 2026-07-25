# Unit 13 dependencies - Lead log

Session 2026-07-24. Lead: Fable 5 (same session as unit 12). Snapshot
`f8763a5ba9c0ce3d0e85d61db7133d19a26ed313` at
`/home/beefsack/Development/brdgme-review-snapshot/rust`.

Scope: 40 Cargo.toml files (~998 lines), Cargo.lock (709 packages -
grep/query only, never read wholesale), deny.toml, rust-toolchain.toml.
Known drift to audit: sqlx 0.8 (web) vs 0.9 (bot/operator), getrandom
0.3 (bot) vs 0.4 (web), no [workspace.dependencies], serde_json dev-dep
pin variance, 108 boilerplate game binaries, lib/cost single-consumer
duplication. Charter: only popular/modern/well-maintained/battle-hardened
deps; lean modern; no bespoke where off-the-shelf fits (parser combinator
exempt). Workers may run read-only shell (grep/wc/sort); NO cargo
mutation commands, NO network - currency assessed from knowledge +
lockfile, flagged where confirmation needed.

## Worker plan (serial)

| Worker | Scope | Raw dump |
|---|---|---|
| W1 | Workspace/manifest structure: root Cargo.toml, all 40 manifests, deny.toml coverage, rust-toolchain.toml, missing workspace.dependencies, drift, unused/bespoke deps | raw/dependencies-structure.md |
| W2 | Version currency + duplication: Cargo.lock duplicate-version analysis, per-dep currency vs knowledge (cutoff-aware), unmaintained/fringe crates | raw/dependencies-currency.md |

Workers on model fable per user override.

### W1 dispatched
- Scope: root + 40 manifests structure, deny.toml, toolchain, drift.

### W1 returned
- 12 findings: 0 critical / 2 major / 8 minor / 2 nit in
  raw/dependencies-structure.md.
- Headlines: no [workspace.dependencies] (serde x36, rand x33, tokio
  x33, serde_json 3 spellings); sqlx 0.8 vs 0.9 confirmed (web pinned
  by tower-sessions-sqlx-store); getrandom 0.3/0.4 + rand triplicate in
  lock; deny.toml multiple-versions only "warn"; 4 stale advisory
  ignores; no workspace.package/lints; ignored [profile] in web
  manifest; tokio "full" in 27 game crates; 108 boilerplate bins;
  lib/cost duplication. Root members actually 40, not 39.
- Lead verification of both majors PENDING.

### W2 dispatched
- Scope: Cargo.lock duplication, direct-dep currency, unmaintained/
  fringe crates, bespoke-vs-off-the-shelf, warp/axum split.

### W2 returned
- 15 findings: 0 critical / 2 major / 9 minor / 4 nit in
  raw/dependencies-currency.md.
- Headlines: sentry 0.48 default features drag actix-web stack + ureq/
  native-tls into every server+game binary; term_size 0.3.2
  (RUSTSEC-2020-0163, unmaintained) direct dep of lib/cmd; combine
  dormant; serde_yaml deprecated with 2 consumers (bot + game_client);
  three rand stacks; svix last http-0.2 holdout; warp/axum split.
- Clean: no atty/proc-macro-error/dotenv/failure; core stack current.
- Lead verification of all 4 unit majors PENDING.

## Lead verification

- VERIFIED zero workspace.dependencies/workspace=true hits; serde
  "1.0.228" in 33 manifests via grep (W1's 34 count includes spelling
  variants). CONFIRMED.
- VERIFIED sqlx 0.8 (web) vs 0.9 (bot, operator x2 incl dev-dep) in
  manifests; both 0.8.6 and 0.9.0 in Cargo.lock. CONFIRMED.
- VERIFIED sentry 0.48.5 lock entry depends on sentry-actix, ureq,
  native-tls; 8 actix-* packages in lock; all four sentry declarations
  use default features. CONFIRMED.
- VERIFIED term_size 0.3.2 direct dep at lib/cmd/Cargo.toml:16 and in
  lock. CONFIRMED.
- MERGED chrono-in-rand_bot (W1 nit + W2 minor) into one minor finding.
- No rejections; no downgrades.

## Curation complete (2026-07-24)

Curated file: findings/dependencies.md. Tally: 0 critical / 4 major /
17 minor / 5 nit (26 findings). Unit 13 CLOSED.

## Units 12 and 13 complete (2026-07-24)

Both curated files exist and LOGs are closed:
- findings/bot-operator-tools.md (0c/4M/15m/11n, 30 findings)
- findings/dependencies.md (0c/4M/17m/5n, 26 findings)
