# 29: Player Stats and Historical Reports - Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.
>
> Extracted 2026-07-08 from `docs/plan/29-stats-reports.md`. Task granularity is
> work-package level; run superpowers:writing-plans against the paired spec
> before execution if bite-sized steps are needed.

**Spec:** `docs/superpowers/specs/2026-07-08-29-stats-reports-design.md`

## v1 tasks

- [ ] DB queries (rust/web/src/db.rs or a new stats module): per-user
      totals, per-user-per-game-type aggregates, rating series,
      recent-games list, head-to-head - all parameterized by the D1
      inclusion rule (>= 2 humans, optional include-single-human flag).
- [ ] Server fns + WASM-safe view types for the above.
- [ ] Sparkline + form-strip components (text-based, D2 tiers 1-2).
- [ ] SVG chart components (line chart, histogram - D2 tier 3),
      including sensible rendering for tiny datasets (1-2 points).
- [ ] `/players/:name` route + page.
- [ ] `/players/:name/:game_type` route + page.
- [ ] Form strips in the game meta panel.
- [ ] One-off `peak_rating` backfill from reconstructed series.
- [ ] Rating reconstruction sanity check (drift vs
      `game_type_users.rating`) as a test or admin query.
- [ ] Tests: aggregate queries against seeded fixtures (D1 rule
      including the mixed human+bot case, tie placings, bot-only
      exclusion), rating series reconstruction, percentile placing.
