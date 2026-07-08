# Leptos Risk Evaluation

Seed created 2026-07-03; deep dive completed same day (data via GitHub API,
RUSTSEC advisory-db, web). **DECIDED 2026-07-03: stay on Leptos + tripwires
+ hedges** (option b) - accepted by Michael.

## Trigger

Found during the 2026-07-03 tech review final pass:
[leptos-rs/leptos#4707](https://github.com/leptos-rs/leptos/issues/4707)
(2026-05-08) - the creator (gbj) declared:

- Leptos is **feature-complete**; no major new features planned.
- The project will be **"lightly maintained"** going forward - not
  abandoned, but reduced engagement with issues and PRs.
- `leptos_0.9` work continues slowly: cleanup and semver-breaking fixes
  only, explicitly no urgency on a release date.
- Community members are invited to take on active maintenance roles.
- Component libraries and routing alternatives are pushed to
  community-driven projects rather than core.

Stated driver is fatigue from LLM-generated issue/PR spam and declining
human engagement, not loss of interest in the code: "not planning to
abandon", "still get all the same notifications", open to onboarding
maintainers.

## brdgme exposure (measured 2026-07-03)

- Versions: `leptos 0.8.14`, `leptos_router 0.8.10`, `leptos_axum 0.8.7`,
  `leptos_meta 0.8.5` (rust/web/Cargo.toml).
- `rust/web/src` is ~4,600 lines; ~1,200 lines are directly Leptos view
  code (`app.rs` 576, `components/game.rs` 421, `components/layout.rs` 93,
  `websocket_client.rs` 60, `lib.rs` 30). 9 of ~20 source files import
  leptos.
- 15 `#[server]` functions (11 in `game/server_fns.rs`, 4 in
  `auth/server.rs`) - the main coupling beyond views. Context-based DI via
  `leptos_routes_with_context` (PgPool, GameBroadcaster, reqwest Client).
- Hard-won SSR/hydration knowledge encoded in `docs/CODING.md`
  (Resource::new_blocking vs LocalResource, Suspense/Transition rules,
  structural-mismatch panics) - Leptos-specific institutional knowledge,
  a sunk cost and a migration cost simultaneously.
- Framework-agnostic and NOT at risk: Axum backend, `brdgme_game`/
  `brdgme_markup` WASM libs (command parsing/suggestions compile to WASM
  independently of the UI framework), the game HTTP contract, all of
  `db.rs`/auth/orchestration.
- Build tooling exposure: `cargo-leptos`, the `wasm-bindgen = 0.2.108` pin
  (must match nixpkgs `wasm-bindgen-cli`) - a known ongoing friction point.
- Planned dependency (PLAN Phase 17): adopt `leptos-use` `use_websocket` -
  community-maintained (Synphonyte), separate from core Leptos.

## Findings (2026-07-03)

### 1. Bus factor since #4707 - mixed

- Activity is UP, not down: 30+ PRs merged in June 2026 alone, v0.8.19
  released 2026-06-25, gbj backporting fixes as recently as 2026-06-28.
- Strong contributors emerged: sabify (sustained perf work across
  server_fn/tachys/router/integrations), Noethix55555 (correctness fixes),
  plus Baptistemontan, edmondj; even ealmloff (Dioxus core) contributed.
- BUT merge rights remain bus factor = 1: every sampled merged PR was
  merged by gbj personally. No co-maintainer stepped up publicly in the
  #4707 thread through late June.
- Net: the announced "lightly maintained" state has not materialized in
  practice; the concentrated risk is gbj stopping merges with no one
  granted rights.

### 2. What "lightly maintained" leaves unowned

- RUSTSEC: zero advisories exist for leptos, leptos_axum, server_fn,
  tachys, reactive_graph, or cargo-leptos (checked advisory-db crates/
  directly). No current security exposure.
- Axum majors: the real risk. Axum's 0.9 milestone was 10 closed / 2 open
  (2026-07-03), so a next major is plausible late 2026. gbj was still
  merging Axum-integration fixes in June (body-size-limit fix), so a port
  is likely but on his timeline. Tripwire #1.
- wasm-bindgen: a "don't rely on unstable wasm-bindgen APIs" PR was merged
  then reverted in June 2026 - churn here is real; the nixpkgs pin
  friction continues.

### 3. 0.9 trajectory - alive, not shelved

0.9.0-alpha published 2026-05-19; milestone 17 closed / 3 open; branch
commits through 2026-06-25. Scope is cleanup + semver-breaking fixes, no
new features. The 0.8 -> 0.9 upgrade is cheap insurance when it ships; no
reason to chase the alpha.

### 4. Ecosystem drift - all satellites healthy

- cargo-leptos: v0.3.6 (2026-04-08), commits through June 2026, lpotthast
  effectively co-maintaining.
- leptos-use: maccesch committing near-daily as of 2026-06-23. Independent
  of core; Phase 17 adoption is safe.
- No fork/successor signal anywhere.

### 5. Alternatives

- **Dioxus**: 0.7 rebuilt server functions around Axum (good fit for the
  monolith) with streaming SSR/hydration; v0.7.9 (2026-05-08),
  0.8.0-alpha published. But the safety argument is weaker than assumed:
  YC S23, ~4 people, ~$500k total funding, and main-branch velocity in
  June 2026 was ~10 commits/month - LOWER than Leptos's. Web SSR is one
  of four platform targets, not the focus. Trades a solo passionate
  maintainer with active contributors for a small startup whose survival
  depends on funding, plus re-learning an equivalent body of hydration
  lore.
- **SSR-only retreat** (Axum + Askama/Maud + WASM command-parser island +
  thin WS script): the only option that deletes the framework-risk class
  entirely, and it fits the lo-fi vision. Cost is a full view rebuild plus
  hand-rolled glue for the autocomplete island and WS-driven updates -
  likely exceeding the ~1,200-line surface. Right shape for a RESPONSE to
  a fired tripwire, not a pre-emptive move.
- **Staying put**: 0.8 is stable, feature-complete, and in practice
  actively patched. "Boring and done" is accurate today.

### 6. Migration cost

- Dioxus: views rewritten in rsx!, 15 server fns ported to a
  near-identical `#[server]` model, CODING.md hydration knowledge mostly
  re-derived. Weeks, not days.
- SSR retreat: larger - full view rebuild + bespoke islands/WS glue, but
  permanently removes cargo-leptos, the wasm-bindgen pin, and
  hydration-panic classes.
- Both are post-cutover-sized projects. Neither justifies delaying
  cutover.

## Tripwires (convert "watch" into "act")

| Condition | N | Check lives in |
|---|---|---|
| RUSTSEC advisory in leptos tree unpatched | 4 weeks | cargo-deny in CI (Quick wins) |
| Axum 0.9/1.0 released, leptos_axum incompatible | 3 months | Renovate lag + quarterly review |
| cargo-leptos broken on current stable Rust | 6 weeks | CI breakage, immediate |
| No PRs merged to leptos while mergeable PRs accumulate | 2 months | quarterly review |
| 0.9 formally shelved or branch dead | 6 months idle | quarterly review |

If a tripwire fires, the decision is between Dioxus (re-verify its state
at that time) and the SSR retreat - retreat favored if the product is
stable by then.

## Hedges (land regardless of outcome)

- Phase 17 leptos-use adoption reduces bespoke framework-internal code.
- Keep server fns thin, delegating to `db.rs`/`game/` (already the
  pattern) - keeps the portable core large.
- cargo-deny + Renovate from the Quick wins provide passive tripwire
  monitoring.

## Constraints respected

- Open source only (VISION principle) - all options comply.
- Nothing here blocks or delays cutover (PLAN Phase 16): decision is (b)
  stay + tripwires + hedges; migration options are shelved as tripwire
  responses.

## Sources (2026-07-03)

- https://github.com/leptos-rs/leptos/issues/4707 - status update + thread.
- GitHub API: commits/PRs/milestones for leptos-rs/leptos,
  leptos-rs/cargo-leptos, Synphonyte/leptos-use, tokio-rs/axum,
  DioxusLabs/dioxus.
- https://github.com/rustsec/advisory-db - crates/ directory checks.
- https://dioxuslabs.com/blog/release-070/ - Dioxus 0.7 scope.
- https://www.ycombinator.com/companies/dioxus-labs +
  https://opencollective.com/dioxus-labs - Dioxus Labs funding/status.
- Memory: `project_tech_review_2026_07.md`, `feedback_coding_rules.md`.
