# WASM Production Error Visibility - Session Handover

**Status:** WS1 and WS2 complete, verified, uncommitted. WS3 (Sentry)
implementation is well underway in the working tree but blocked on a real
wasm-opt/DWARF toolchain crash. This session is ending; a fresh session
picks up from here.

**Do this first in the new session:** implement the DWARF pipeline reorder
described under "Current blocker" below, then resume WS3 plan Phase C.

## Surviving records

- Spec (approved, reflects WS1/WS2 done + WS3 design):
  `docs/superpowers/specs/2026-07-15-wasm-prod-errors-design.md`
- WS1 plan (executed): `docs/superpowers/plans/2026-07-15-ws1-dep-bump.md`
- WS3 plan: `docs/superpowers/plans/2026-07-15-ws3-sentry.md` - reflects the
  **original pre-implementation design only**. It does not yet reflect the
  Phase B DWARF blocker or its fix - update it once the reorder lands.
- This handover: `docs/superpowers/plans/2026-07-15-wasm-prod-errors-handover.md`

## What's done and verified

### WS1: full dependency bump - DONE, verified, uncommitted

Rust toolchain 1.96.1 -> 1.97.0, cargo-leptos 0.3.6 -> 0.3.7, dart-sass ->
1.101.0, resend-rs 0.28, gloo-net 0.7, tokio-tungstenite 0.30, 47 transitive
crates via `cargo update`, devenv.lock refreshed, Playwright 1.61.1,
TypeScript 5 -> 7, @types/node 24. `docs/CODING.md` Dependency Management
rewritten to match reality (as-of 2026-07-15) and now also carries the new
"stay on latest dependencies" policy paragraph.

Deliberately held back (documented in CODING.md, not oversights):
wasm-bindgen `=0.2.121` (nixpkgs CLI ceiling - the whole lockstep group
stays together; latest is 0.2.126), `sqlx 0.8` in web
(`tower-sessions-sqlx-store` doesn't support 0.9 yet), cargo-leptos `0.3.6`
in devenv (nixpkgs ceiling; Dockerfile/CI are on 0.3.7 - drift flagged).

Verification: workspace build/tests/clippy pass, wasm hydrate check passes,
`docker buildx bake web` passed **before** WS3 touched the Dockerfile.

### WS2: prod wasm function names - DONE, verified, uncommitted, decision recorded

Change: `rust/web/Cargo.toml` `[package.metadata.leptos]` gained
`wasm-opt-features = ["-Oz", "--enable-bulk-memory",
"--enable-nontrapping-float-to-int", "-g"]`. Verified against cargo-leptos
0.3.7 source: key name correct, list *replaces* the defaults (hence they're
reproduced verbatim), wasm-opt is the last binary-mutating build step.

Verified in a built image: shipped `web.wasm` gained a `name` custom section
(4,427,815 bytes) with demangled Rust symbols
(`web[hash]::app::__component_app::`, `tachys`/`reactive_graph` frames,
etc.) - readable in the browser console/panic-hook output without any
tooling. Cost: raw 1,374,723 -> 5,802,543 bytes (+322%), gzip -9 527,854 ->
777,972 bytes (**+250,118 bytes, +47.4% transfer**).

**Michael's decision (binding):** keep the `-g` change for now; revisit the
size cost after WS3 ships, since Sentry's server-side symbolication should
make the name section redundant for *captured* errors (raw browser consoles
without Sentry would still benefit from it, so this is a real trade-off to
revisit, not a foregone conclusion).

## WS3: Sentry error tracking - IN PROGRESS, blocked

Design fully approved (see spec, WS3 section) after a second research pass
specifically to avoid bespoke tooling. Two Sentry projects already created
by Michael with real DSNs (not secret - safe to reuse, see below).
`SENTRY_AUTH_TOKEN` already created as a **GitHub org secret scoped to
`brdgme/brdgme` only** (confirmed correct - `brdgme-config` never needs it,
it holds no build/CI pipeline).

Phase breakdown (per `docs/superpowers/plans/2026-07-15-ws3-sentry.md`):

| Phase | What | Status |
|---|---|---|
| A | Frontend Sentry bundle (esbuild) + `shell()` script tags + SHORT_SHA/SENTRY_RELEASE plumbing | **Done**, verified (cargo check ssr + hydrate/wasm32, npm build, fmt) |
| B | DWARF pipeline: `--wasm-debug`, wasm-split install/run, `web-debug` stage/bake target | **Broken - see blocker below.** Code is present in the tree but the build fails |
| B-fix | `debug = true` on `[profile.wasm-release]` | **Applied**, confirmed via direct `git diff` inspection (see below) - necessary but not sufficient; exposed the real blocker |
| C | CI wiring: debug-artifact extraction + gated `sentry-cli` upload | **Not started** |
| D | Rust server wiring: `sentry`/`sentry-tower`/`sentry-tracing` deps, `main.rs` init guard, `router.rs` tower layers | **Done**, verified (cargo check/clippy/fmt clean, boots with `SENTRY_DSN_SERVER` both unset and set) |
| E | k8s (`web-patch.yaml`), CODING.md exception, `docs/decisions/` entry | **Not started** |
| F | Full build/inspection verification pass | **Not started** (blocked by B) |

Phase D note: a "WS3 Phase D" worker was killed by Michael mid-session, but
this happened *after* it had already delivered a genuine, verified
completion report (clean cargo check/clippy/fmt, tests, and a real boot
check both with and without `SENTRY_DSN_SERVER`) - direct inspection of
`rust/web/src/main.rs` and `rust/web/src/router.rs` below confirms the code
is complete and coherent, not partially written. The kill stopped an idle,
already-finished agent instance, not in-progress work. Treat Phase D as
genuinely done.

## Current blocker (top priority for the new session)

**Root cause:** `wasm-opt` (binaryen - reproduced on both cargo-leptos's
pinned 123 and a locally tested 129) crashes with a bare SIGABRT (exit 134,
zero stderr/stdout) when given *any* wasm module carrying DWARF debug info
from `wasm-bindgen --keep-debug`, regardless of optimization flags - it
crashes even at `-O0` with zero passes specified, and even without `-g`. A
sanity check on the identical command against a non-DWARF input succeeded
and produced the expected ~5.8MB (WS2-baseline-sized) wasm, isolating the
crash specifically to the presence of DWARF, not anything else about the
build. Not OOM (verified `free -h`/`ulimit`, 14GB free, no limits).

This diagnosis came from a subagent's investigation, relayed through the
orchestration chain rather than independently reproduced by me in this
final compilation pass - it is detailed, specific, and reproduced across
two binaryen versions with a clean isolating sanity check, so it should be
treated as reliable, but the new session should re-confirm it empirically
(re-running the failing build is fast) before spending further effort on
anything downstream of it.

**Why this breaks production, not just the debug artifact:**
`rust/Dockerfile`'s `web-builder` stage runs
`cargo leptos build --release --wasm-debug` as a single, unconditional,
shared step (this is cargo-leptos's *monolithic* build - it runs
wasm-bindgen then wasm-opt internally with no hook to intervene between
them). **Both** the production `web` final stage and the `web-debug`
extraction stage `COPY --from=web-builder` this same output. So right now,
in the current working tree, `docker buildx bake web` fails outright, not
just `web-debug` - confirmed via direct `git diff rust/Dockerfile`
inspection (see below), the reorder has **not** been applied yet.

**Michael's decision (binding): take the reorder path**, not "pause and
reassess." Rationale recorded: each tool is still invoked exactly as its
vendor documents - only the *sequencing* changes, and only for this one
build stage. It does not reintroduce the bespoke-tooling risk Michael
flagged earlier in this project (which is why WS3 uses Sentry's official
DWARF/wasm-split pipeline instead of a DIY DWARF-to-sourcemap tool in the
first place).

**The fix:** stop using cargo-leptos's monolithic `--wasm-debug` build for
this stage. Manually orchestrate wasm-bindgen, wasm-split, and wasm-opt
directly in `rust/Dockerfile`, in this order:

1. `cargo build` (wasm-release profile, `debug = true` already in place -
   this step is unaffected and does not need to change)
2. `wasm-bindgen --target=web --keep-debug` on the raw pre-bindgen wasm
   (produces a large, unoptimized wasm with both DWARF and the name section
   present - confirmed ~140MB locally, vs ~12MB without `--keep-debug`)
3. **`wasm-split --strip --debug-out=web.debug` here, before wasm-opt runs**
   - this is the reorder. `wasm-split` only strips `.debug_*` sections
     (confirmed from its source: `--strip-names` would be needed to also
     drop the name section, and we don't pass that), so the name section
     required by WS2 survives. Output: `web.debug` (DWARF companion for
     Sentry) + a DWARF-free but not-yet-size-optimized wasm.
4. `wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int -g`
   on that now-DWARF-free wasm - safe, because wasm-opt never sees DWARF at
   all in this ordering. `-g` here preserves the name section through the
   optimization pass, matching WS2. This produces the final shipped
   `web.wasm`.

This means the `cargo leptos build --release --wasm-debug` line currently
in `rust/Dockerfile` needs to be replaced with these four manually
orchestrated steps for the wasm side (the SSR/server binary build via
`cargo leptos build` is unaffected and can likely stay as-is, or needs
`--wasm-debug` removed since it's no longer doing anything - check whether
cargo-leptos's build still needs to run at all for the non-wasm parts, or
whether the wasm-side steps above fully replace it for this stage).

**Options considered and rejected:** shipping without wasm-opt (defeats the
whole point of `-Oz`, unacceptable for production size); trying yet another
wasm-opt binary (already failed locally on two versions, low-confidence
speculative work).

## Real credentials already provided (safe to reuse - Sentry DSNs are not secret)

DSNs are designed for public client-side embedding (they can only submit
events, never read data) - safe in source/config/manifests.

- Browser (JS + wasm) project DSN:
  `https://9d0c02b1f53022489261350da4820fbb@o4511737783451648.ingest.us.sentry.io/4511737800032256`
- Rust server project DSN:
  `https://fdafb6fb57688645d1dd8dfee49304fc@o4511737783451648.ingest.us.sentry.io/4511737808551936`

`SENTRY_AUTH_TOKEN` already exists as a GitHub Actions org secret scoped to
`brdgme/brdgme` (its value is not known to any Claude session, by design -
CI reads it directly).

## Decisions Michael has made this session (binding, do not re-litigate)

- WS3 reorder path chosen for the DWARF blocker (see above), not
  pause/reassess.
- WS2's `-g` name-section change: keep for now, revisit after WS3 makes it
  redundant for Sentry-captured errors.
- Sentry: two separate projects (browser + Rust server), matching the
  original per-platform design.
- `send_default_pii: false` on **both** frontend and server Sentry configs -
  a deliberate override of the Sentry SDKs' own quickstart default of
  `true`.
- No `tracesSampleRate` key at all in the frontend init (omitted, not `0`) -
  disables tracing entirely to protect the 5k/month free-tier error quota.
- Ad-blocker tunnel: skipped, revisit only if event loss becomes visible.
- CI secret scoping (`brdgme/brdgme` only, not `brdgme-config`): confirmed
  correct.
- **Model constraint (current, may not still apply in the new session -
  check `/model` state first):** fable quota was exhausted mid-session;
  Michael switched everything - main orchestration and all sub-orchestrators
  - to Sonnet. Sub-agents (the actual workers) were always required to be
    Sonnet regardless. If fable quota has recovered by the new session,
    confirm with Michael whether to revert the top-level orchestration
    model.
- **No commits or pushes without Michael's explicit, specific request** -
  this handover document is the sole exception, explicitly authorized this
  turn (see the commit made alongside this file).

## Exact file state (brdgme repo, verified via direct `git diff`/`git status` immediately before writing this handover)

```
 M .github/workflows/ci.yml          (WS1 toolchain bump only, ~4 lines)
 M devenv.lock                       (WS1)
 M docker-bake.hcl                   (WS3 Phase A/B: SHORT_SHA arg, web-debug target)
 M docs/CODING.md                    (WS1 policy + pin refresh; Phase E's Sentry exception paragraph NOT yet added)
 M rust/Cargo.lock                   (WS1)
 M rust/Cargo.toml                   (WS1 toolchain-adjacent + WS3 B-fix: debug = true on [profile.wasm-release])
 M rust/Dockerfile                   (WS1 toolchain bump + WS3 Phase A/B - reorder NOT yet applied, currently broken)
 M rust/rust-toolchain.toml          (WS1)
 M rust/web/.env.template            (WS3 Phase A + D: SENTRY_DSN_WEB, SENTRY_DSN_SERVER examples)
 M rust/web/Cargo.toml               (WS1 dep bumps + WS2 wasm-opt-features + WS3 Phase D sentry deps)
 M rust/web/end2end/package-lock.json (WS1)
 M rust/web/end2end/package.json     (WS1)
 M rust/web/src/app.rs               (WS3 Phase A: shell() Sentry script tags)
 M rust/web/src/main.rs              (WS3 Phase D: init_sentry, tracing layer)
 M rust/web/src/router.rs            (WS3 Phase D: sentry_tower layers)
?? docs/superpowers/plans/2026-07-15-ws1-dep-bump.md
?? docs/superpowers/plans/2026-07-15-ws3-sentry.md
?? docs/superpowers/specs/2026-07-15-wasm-prod-errors-design.md
?? rust/web/js/                     (new: package.json, sentry.js, package-lock.json, node_modules/ - gitignored, confirmed)
?? rust/web/public/sentry.js        (new, esbuild output - generated, may want a .gitignore entry or intentional commit, TBD)
?? websocket/                       (pre-existing, unrelated to this effort, present since before this session started)
```

Note: `rust/web/js/node_modules/` is confirmed covered by the repo's
`node_modules/` gitignore rule - safe, will not be accidentally committed.

## brdgme-config repo

**No changes.** Confirmed clean (`git status`/`git diff --stat` both
empty). WS3 needs zero changes here - all Sentry config is non-secret and
belongs in `k8s/prod/app/web-patch.yaml` in the **main** repo (Phase E, not
started), following the exact pattern already used for
`OTEL_EXPORTER_OTLP_ENDPOINT`/`OTEL_TRACES_SAMPLER_ARG`.

## Orchestration model used this session (for the new session to replicate or adjust)

Michael directed: main session acts as a read-only orchestrator (filesystem
reads + plan files only, no shell/mutating tools), delegating each
workstream to a single-use "sub-orchestrator" agent (same read-only
constraint, plus permission to write plan files), which in turn spawns
actual-work subagents for file edits/builds/verification. Sub-orchestrators
run serially, one per workstream, never reused across workstreams. All
subagents (sub-orchestrators and workers alike, after the fable-quota
exhaustion) ran on Sonnet.

**Known infra quirk worth knowing about:** worker subagents frequently tried
to reply to their parent via `SendMessage` using a generic name like
`"general-purpose"` and got "not reachable" - their results then arrived as
notifications to whichever agent happened to be listening (often the main
session), requiring manual relay back to the intended recipient. This
happened repeatedly and never indicated lost work - every case investigated
found the worker had genuinely completed and just couldn't route its reply.
If the new session sees a similar bounce, the fix is the same: relay the
content manually to the actual intended recipient by its real agent id/name,
not the generic label it tried to use.

## Recommended plan for the new session

1. Re-verify the wasm-opt SIGABRT empirically (fast: `wasm-bindgen
   --keep-debug` then `wasm-opt` on the result, outside Docker) before
   trusting this handover's diagnosis blindly.
2. Implement the DWARF pipeline reorder in `rust/Dockerfile` (see "Current
   blocker" above) - replace the monolithic `cargo leptos build --release
   --wasm-debug` wasm-side build with the four manually orchestrated steps.
3. `docker buildx bake web-debug` and `docker buildx bake web` - confirm
   both succeed.
4. Extract and inspect: `web.debug` contains real DWARF; shipped `web.wasm`
   has the name section, zero `.debug_*` sections, and size back near the
   WS2 baseline (528KB / 778KB gzip class of numbers, not the 140MB
   intermediate).
5. Resume WS3 plan Phase C (CI wiring - debug-artifact extraction +
   `sentry-cli debug-files upload`, gated on `SENTRY_AUTH_TOKEN` presence),
   Phase E (k8s `web-patch.yaml` with the real DSNs above, `docs/CODING.md`
   exception paragraph, `docs/decisions/SENTRY_SAAS_EXCEPTION.md`), Phase F
   (full verification pass - see WS3 plan file for the exact checklist).
6. Update `docs/superpowers/plans/2026-07-15-ws3-sentry.md` to record the
   Phase B blocker and its resolution once fixed - it currently only
   reflects the pre-implementation design.
7. Full report to Michael; batch commit/push review remains his to
   organize, as throughout this whole effort - do not commit anything
   beyond what he explicitly requests.
