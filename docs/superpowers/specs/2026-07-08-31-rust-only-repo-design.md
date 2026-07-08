# 31: Rust-Only Repository (eliminate Go and TS/JS) - Design

> Extracted 2026-07-08 from `docs/plan/31-rust-only-repo.md` (superpowers layout
> migration). Content dates from 2026-07-08; this is a point-in-time decision
> record, not a living document.

**Status:** Ready 2026-07-08 - WP1 unblocked and runnable pre-cutover;
WP2 ongoing via #23; WP3-WP5 post-go-live.

This item owns the *removal* milestones. Game porting itself is tracked in
[23-rust-game-ports.md](../plans/2026-07-04-23-rust-game-ports.md) /
`docs/GAME_PORTING_PLAN.md` (kept separate - active tracker with per-port
notes). Decision record: memory 539c3447 (2026-07-08, rewrite all Go games
to Rust ASAP; Go changes light-touch until then).

## Explicit keeps (existing decisions, do not delete)

- `rust/game/lords-of-vegas-1` (implemented, undeployed - 2026-07-02
  decision).
- `games.chat_id` column and chat tables (unported legacy data).
- `rust/tools/{fuzz,repl}` (porting workflow depends on them).
- Everything else in `rust/` is load-bearing (bot, operator, web, lib/*,
  game/*). There is no further removable Rust beyond `rust/api`.

## Decisions

1. **No rollback support (decided 2026-07-08):** the project supports no
   simultaneous deployments and no rollback paths. Solo side project,
   friends-only user base - operator effort is the scarce resource and
   downtime is acceptable. The `k8s/prod-rollback/` overlay planned in #16
   is dropped, along with the never-verified `http.ts` apex-derivation
   item (moot). The Linode box happens to exist until its decommission,
   but reviving it is not a maintained procedure. Legacy images remain on
   GHCR (`ghcr.io/brdgme/brdgme/{web-legacy,websocket,api}`) for
   archaeology only.
2. **Playwright e2e (`rust/web/end2end`, TS): kept for now** (decided
   2026-07-08), but candidate for outright *removal* later - it is flaky,
   already `continue-on-error` in CI, and its value may not cover its
   maintenance overhead. Revisit when it next causes work; removal beats a
   Rust rewrite. Not on any critical path.
3. **Shelving UX specifics** - deferred to WP4 design (below).

## WP4: Game shelving lifecycle (post-go-live feature work) - design

Deprecation today stops *new* games; running `-1` services wait for natural
drain. Shelving ends that wait and is the mechanism that eventually deletes
each pinned `-1` Deployment. Lifecycle: active -> deprecated (exists today)
-> shelved.

Design sketch (to be fleshed out when scheduled):

- Shelve operation per GameVersion:
  1. Unfinished games on the version are frozen: marked abandoned,
     unrated (no ELO effect), bots unsubscribed, undo history closed.
  2. Final renders cached to the database - public render plus one render
     per player (new table/columns; today no render is stored anywhere,
     the game service renders live on every request).
  3. GameVersion marked shelved; web serves the game page read-only from
     the cached render (logs are already in the DB and stay visible);
     command input, undo, concede, restart hidden.
  4. Deployment/Service manifests deleted; the pinned image is no longer
     pulled.
- Open questions for the design pass: cached-render format (rendered
  markup vs markup AST), whether restart-into-`-2` is offered for frozen
  games, operator UX for triggering a shelve (operator CRD field vs
  one-off job).
- First consumers: the 17 Go `-1` versions (shelve as each drains or by
  fiat once `-2` is proven); later reusable for any old Rust edition.
