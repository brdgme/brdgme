# WASM Production Error Visibility - Design

Date: 2026-07-15
Status: WS1 done (2026-07-15). WS2 done (2026-07-15): -g accepted at +250KB
gzip / +47.4% transfer, decision "keep for now, revisit after WS3 makes names
redundant for Sentry-captured errors". WS3 in design.

## Problem

Prod (beta.brdg.me) wasm errors are unreadable. Stack frames render as
`wasm-function[N]:0xOFFSET`, e.g. the current tachys hydration panic
(`tachys-0.2.18/src/hydration.rs:163: internal error: entered unreachable
code`). The panic *message* carries file:line (via `console_error_panic_hook`)
but the *frames* do not, so the call chain into the panic is opaque.

## Research findings (verified 2026-07-15)

- cargo-leptos (0.3.6 pinned; latest is 0.3.7, diff irrelevant) release
  builds unconditionally run `wasm-opt -Oz` WITHOUT `--debuginfo`, stripping
  DWARF and the name section. `--wasm-debug` only affects the wasm-bindgen
  step and is designed for dev (`cargo leptos watch` + Chrome DWARF
  extension). No upstream work on prod symbolication exists.
- There is NO community-standard prod source-map path for Rust wasm. Trunk
  and wasm-pack have the same dev-only DWARF story. Chrome abandoned wasm
  source maps in favor of DWARF; the DWARF consumer (Chrome "C/C++ DevTools
  Support" extension) is a local dev tool. Every `.map`-for-wasm setup in the
  wild is a DIY DWARF-to-map pipeline (stale `wasm2map` crate or emscripten's
  `wasm-sourcemap.py`). Rejected as bespoke.
- DevTools never symbolicates *stringified* stacks, and
  `console_error_panic_hook` logs a plain string - so source maps would not
  have fixed the panic-hook output anyway. The wasm **name section**, by
  contrast, is embedded by the JS engine into `Error.stack` strings natively:
  names work everywhere, including the existing panic hook, with DevTools
  closed.
- cargo-leptos's `wasm-opt-features` config (Cargo.toml
  `[package.metadata.leptos]`, added upstream for exactly this) REPLACES the
  raw wasm-opt argument list. Adding `-g` preserves the name section (and any
  DWARF present in input) through optimization. Supported knob, no scripts.
- Sentry is the only maintained, productized, documented path for unattended
  prod wasm symbolication: `@sentry/browser` + `@sentry/wasm` capture raw
  wasm addresses; server-side Symbolicator resolves file:line from an
  uploaded DWARF companion produced by Sentry's `wasm-split` tool. Sentry
  explicitly does not use source maps for wasm.
- Sentry free "Developer" tier fits brdgme: no credit card, 5,000
  errors/month, 1 seat, 30-day retention, debug-file upload + Symbolicator
  included, overage = HTTP 429 drop with spend capped at $0 by default.

## Decision

Three workstreams, executed in order, each in its own session with its own
implementation plan:

### WS1: Full dependency bump

Per the dependency-currency policy added to `docs/CODING.md` (stay on latest
everything; check-and-bump before dependency-sensitive work), bring the whole
repo to latest before touching the wasm pipeline.

Scope:
- Rust workspace: `cargo update`, plus major-version bumps where the
  "pinned-by-ecosystem" list in CODING.md is stale (it is dated 2026-04-04
  and already contradicts Cargo.toml, e.g. wasm-bindgen =0.2.108 vs actual
  =0.2.121, reqwest 0.12 vs actual 0.13).
- Dockerfile pins: cargo-chef/rust image, cargo-leptos 0.3.6 -> 0.3.7 (or
  latest at execution time), wasm-bindgen-cli (must stay in lockstep with the
  Cargo.toml `=` pin AND devenv.nix), dart-sass, sqlx-cli, cargo-binstall.
- devenv.nix tool versions in lockstep with the above.
- end2end npm deps (Playwright).
- Refresh the stale Dependency Management section of CODING.md to match
  reality after the bump.
- Out of scope: brdgme-go / go.mod (legacy, being retired under #31).
- Follow-up noted, not in scope: extend renovate.json coverage so Dockerfile
  and devenv pins stop drifting silently.

Verification: full workspace build + tests, dev stack boots under Tilt, web
image builds via docker bake.

### WS2: Prod function names (supported-config only)

Change: in `rust/web/Cargo.toml` `[package.metadata.leptos]` add

    wasm-opt-features = ["-Oz", "--enable-bulk-memory",
                         "--enable-nontrapping-float-to-int", "-g"]

(defaults reproduced verbatim plus `-g`).

Result: prod stack traces - both live uncaught RuntimeErrors and the panic
hook's stringified stacks - show real demangled Rust function names instead
of `wasm-function[N]`. No code changes, no new tooling, no panic-hook change.

Verification (must-pass before done):
- Confirm at the pinned cargo-leptos version that `wasm-opt-features`
  replaces raw args as researched (build succeeds, wasm-opt invoked with the
  four args).
- Confirm shipped `pkg/web.wasm` contains a name section and browser traces
  show Rust function names (build web image locally, run, force a panic or
  inspect DevTools).
- Measure and report the wasm size delta (name section cost) so Michael can
  accept or reject the trade-off with numbers.

### WS3: Sentry error tracking (design approved 2026-07-15)

Decisions made by Michael 2026-07-15:
- Policy: documented exception in CODING.md permitting the Sentry browser
  SDKs as the sole non-Rust runtime dependency (no Rust alternative exists
  for browser wasm-address capture); documented exception for hosted Sentry
  (FSL SaaS) as an operational tool - observes the platform, not required to
  run or play brdgme, free Developer tier with $0 spend cap.
- Scope: browser (JS + wasm) AND Rust server capture.
- JS bundling: esbuild step in the Docker web-builder stage (nodejs/npm
  already present there); SDK versions tracked via a package.json.

Design (research-verified 2026-07-15):

- Frontend: `@sentry/browser` + `@sentry/wasm` (`wasmIntegration()`),
  bundled by esbuild at image build, served from the site assets. No Sentry
  CDN bundle includes wasm support. Init MUST run before the wasm module
  loads: script tag goes in `shell()` (rust/web/src/app.rs) before
  `<HydrationScripts>`. DSN plumbing follows the THEME_BOOT_SCRIPT pattern:
  SSR reads SENTRY_DSN env, emits inline init snippet; unset = no snippet =
  disabled (dev/Tilt unaffected). No CSP exists in app or k8s; nothing to
  whitelist.
- DWARF pipeline (correction to WS2 assumption: `-g` alone is a DWARF no-op
  because wasm-bindgen strips DWARF before wasm-opt runs): add
  `--wasm-debug` to `cargo leptos build --release` in rust/Dockerfile so
  wasm-bindgen keeps DWARF; wasm-opt `-g` then round-trips it. Then
  `wasm-split` (prebuilt binary from getsentry/symbolicator releases):
  `wasm-split web.wasm -o web.wasm --strip --debug-out=web.debug` - strips
  only `.debug_*` sections, injects ~20-byte build_id, PRESERVES the name
  section (verified in wasm-split source), so shipped size stays at the WS2
  baseline.
- CI: `FROM scratch` Dockerfile stage exposes web.debug; bake target with
  local output extracts it on the runner; `sentry-cli debug-files upload`
  runs on the runner with SENTRY_AUTH_TOKEN from GitHub secrets (never
  baked into images). Matching is by wasm build_id; also set a sha-based
  Sentry release string aligned with the image tag scheme.
- Rust server: `sentry` 0.48.x + `sentry-tower` + `sentry-tracing` as one
  extra layer on the existing tracing_subscriber registry (main.rs), plus
  `sentry::init` guard (default panic-hook capture). DSN unset = disabled
  no-op client, same convention as the OTLP layer.
- Defaults: sendDefaultPii off; tracesSampleRate OMITTED entirely (errors
  only, protects 5k/month quota; 0 is not equivalent to omitting);
  beforeSend scrub of cookies/auth headers. Ad-blocker tunnel SKIPPED
  (needs same-origin proxy route; revisit if event loss is visible).
- Account (Michael, in progress): org + two projects (browser JS
  `web-frontend`-style, Rust `web-server`-style), two DSNs, org auth token
  with project write/release scopes as a GitHub Actions secret. Exact
  minimal token scopes to be confirmed during implementation.

## Out of scope (rejected)

- Browser `.map` source maps for wasm: bespoke by nature (no maintained
  generator, no toolchain support, Chrome moved away from them), and they
  cannot symbolicate the panic hook's string output at all.
- Fixing the tachys hydration panic itself: separate debugging task once
  WS2/WS3 make traces readable.

## Success criteria

1. All dependency pins current, CODING.md Dependency Management accurate.
2. A prod wasm panic produces a stack trace with readable Rust function
   names in any user's browser console.
3. (WS3) Prod errors arrive in Sentry symbolicated to Rust file:line without
   anyone having DevTools open.
