# 31: Rust-Only Repository (eliminate Go and TS/JS)

**Status:** Ready 2026-07-08 - WP1 unblocked and runnable pre-cutover;
WP2 ongoing via #23; WP3-WP5 post-go-live.

**Goal:** the repository contains only Rust application code. Non-Rust
support files remain by design: docs, k8s YAML, Tiltfile (Starlark), CI
YAML, OpenTofu HCL, Dockerfiles, devenv.nix, shell scripts.

This item owns the *removal* milestones. Game porting itself is tracked in
[23-rust-game-ports.md](23-rust-game-ports.md) /
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

## WP1: Delete the legacy stack (pre-cutover, unblocked)

Supersedes the matching lines of the #16 decommission list; #16 keeps the
Linode decommission and its human steps. Single commit, tagged beforehand.

- [ ] Tag `legacy-final` on master; record the last GHCR sha tags for
      `web-legacy`, `websocket`, `api` in this file.
- [ ] Delete source: `web/`, `websocket/`, `rust/api/` (includes
      `rust/api/migrations` - the migrate image uses `rust/web/migrations`,
      self-sufficient since `001_initial_schema.sql`).
- [ ] Remove `"api"` from `rust/Cargo.toml` workspace members.
- [ ] Delete k8s: `k8s/base/legacy/`, `k8s/base/web-legacy/`,
      `k8s/base/api/`, `k8s/base/websocket/`, `k8s/base/redis/` (kept only
      for break-glass + LEGACY dev; monolith dropped Redis in #17),
      `k8s/dev-legacy/`; remove the `web-legacy`/`websocket`/`api` image
      entries from `k8s/prod/app/kustomization.yaml`.
- [ ] Tiltfile: remove `LEGACY=1` mode (image builds, k8s_yaml, gateway
      legacy routes/listeners).
- [ ] CI: remove the `build-legacy` job from `.github/workflows/ci.yml`.
- [ ] Delete stale root artifacts (pulled forward from #16): `WORKSPACE`,
      `build.sh`, `test.sh`, `docker-compose.yml` - verify nothing
      references them first.
- [ ] Update DEV.md (LEGACY mode docs) and ARCHITECTURE.md. (#16 already
      updated 2026-07-08: overlay superseded, rollback section replaced by
      the no-rollback decision.)

Payoff: smaller workspace (clippy/deny/test/Renovate stop covering the
Rocket-era api crate), three CI image builds gone, #16 loses its one
unwritten deliverable.

## WP2: Finish Track B ports (ongoing; tracked in #23)

The Go-elimination critical path is the 7 remaining `-2` conversions:
age-of-war, cathedral, love-letter, modern-art, roll-through-the-ages,
splendor, texas-holdem. Library prerequisites first where blocking:

- [ ] cost/permutation module in `rust/lib` (blocks splendor-2)
- [ ] poker hand evaluation (blocks texas-holdem-2)
- [ ] 7 conversions, each per the #23 recipe (crate + 1:1 tests, workspace,
      Dockerfile, CI matrix, Tiltfile, k8s manifests, `-1` marked
      `isDeprecated: true`)

Track A ports are net-new content, not required for Go elimination -
interleave at will.

## WP3: Remove the Go stack (after all 17 `-2` games are deployed)

The `-1` GameVersions keep serving existing games from pinned images; the
repo stops carrying Go. Single commit, tagged beforehand.

- [ ] Tag `go-final`; wait for master CI to push the final
      `sha-<short>` Go game images; record the digests in this file.
- [ ] Pin every `-1` Deployment in `k8s/prod/app/kustomization.yaml` to the
      recorded GHCR digest (immutable `@sha256:` pins, not `latest`).
- [ ] Drop `-1` games from the dev environment (Tiltfile Go builds and dev
      overlay entries) - dev has no legacy games to serve; `-1` k8s base
      manifests stay for prod until WP4 shelves each game.
- [ ] Delete `brdgme-go/` and root `go.mod`.
- [ ] CI: remove `test-go` and `build-go-games` jobs.
- [ ] Update ARCHITECTURE.md/DEV.md; note in #23 that the "retire
      brdgme-go" line is done.

## WP4: Game shelving lifecycle (post-go-live feature work)

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

## WP5: Lift `rust/` to the repository root (last)

Only after WP1 and WP3 - fewer moving paths. Mechanical, one commit, `git
mv` to preserve history.

- [ ] Move `rust/*` to root: workspace `Cargo.toml`/`Cargo.lock`,
      `rust-toolchain.toml`, `deny.toml`, `Dockerfile`, crate dirs,
      `.sqlx` if present.
- [ ] Fix paths: `rust/Dockerfile` COPY paths, `docker-bake.hcl`
      (`dockerfile = "rust/Dockerfile"`), Tiltfile `only=["rust/"]`
      filters and dockerfile args, CI `working-directory: rust` +
      `Swatinem/rust-cache` `workspaces: rust`, devenv.nix, `.gitignore`
      (`rust/target` -> `target`).
- [ ] Sweep docs for `rust/` path references (CODING.md, DEV.md,
      ARCHITECTURE.md, GAME_PORTING*.md, plan files).

## Sequencing

- WP1: any time - pre-cutover is *better* (simplifies #16). Does not block
  or depend on cutover.
- WP2: ongoing, post-go-live priority per PLAN.md (#23).
- WP3: gated on WP2 complete (17/17 `-2` deployed).
- WP4: independent feature work, post-go-live; unblocks deleting `-1`
  Deployments that WP3 pinned.
- WP5: gated on WP1 + WP3 (no `web/`, `websocket/`, `brdgme-go/`, `go.mod`
  left at root).

End state: application code is 100% Rust at the repo root; games ship as
versioned editions with a deprecate -> shelve lifecycle; the only
non-Rust files are docs, manifests, CI, infra, and (pending decision 2)
the Playwright e2e suite.
