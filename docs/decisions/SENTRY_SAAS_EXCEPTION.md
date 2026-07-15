# Sentry SaaS Exception

**DECIDED 2026-07-15: use the Sentry browser JS SDKs plus hosted Sentry SaaS,
as two documented exceptions to CODING.md's "No non-Rust dependencies"
principle** - accepted by Michael.

## Trigger

Production wasm errors were unreadable: the release-built `web.wasm` gave raw
`wasm-function[N]:0xOFFSET` addresses, no function names, making prod
frontend failures undiagnosable. The chosen fix keeps the wasm name section
(WS2: `wasm-opt -g`) and adds server-side symbolication of DWARF debug files
via Sentry, using the getsentry `wasm-split` companion-file tool, uploaded
from CI.

## Findings / options evaluated

- **DIY DWARF-to-sourcemap tooling.** No community-standard prod path for
  Rust wasm exists; every setup in the wild (stale `wasm2map` crate,
  emscripten's `wasm-sourcemap.py`) is bespoke, hand-rolled tooling - exactly
  what CODING.md's "No bespoke code" principle forbids. Rejected.
- **Self-hosted Sentry.** Removes the SaaS dependency entirely, but Sentry
  self-hosted is a large multi-service deployment to run and maintain -
  operational burden disproportionate to a hobby platform. Rejected.
- **Hosted Sentry SaaS.** The only maintained, productized, documented path
  for unattended prod wasm symbolication. Accepted.

## Decision

Use hosted Sentry SaaS as an operational tool that observes the platform, not
a component required to run or play brdgme:

- Sentry is FSL-licensed (Functional Source License - source-available, not
  OSI open source). Its use here is scoped to observability tooling, not a
  runtime dependency of the product itself.
- Free Developer tier: 5,000 errors/month, 30-day retention, $0 spend cap
  configured. On overage Sentry returns HTTP 429 and drops events -
  degradation is loss of error visibility only, never spend.
- Two projects in org `brdgme`: `web-javascript` (browser JS + wasm; debug-file
  uploads and releases target this project) and `web-server` (Rust server).
- `send_default_pii: false` explicitly set on both SDKs - a deliberate
  override of the SDK quickstarts' default of `true`.
- Tracing disabled entirely on the frontend: no `tracesSampleRate` key at all
  (omitted, not `0`) - protects the 5k/month error quota from being consumed
  by transactions.
- DSNs are public-by-design, event-submit-only credentials - safe as literals
  in `k8s/prod/app/web-patch.yaml` (they cannot read data).
- `SENTRY_AUTH_TOKEN` (the CI debug-file upload credential) is the only real
  secret: lives in GitHub org secrets scoped to `brdgme/brdgme`, never in
  images or the repo.

## Consequences / guardrails

- The Sentry JS SDKs are the sole non-Rust runtime dependency in the codebase
  (see the CODING.md exception paragraph).
- Revisit WS2's `-g` name-section size cost once Sentry symbolication proves
  out in production - it may become redundant for Sentry-captured errors
  (raw browser consoles without Sentry open would still benefit from it).
- Event-loss visibility: the ad-blocker tunnel (same-origin proxy for
  Sentry requests blocked by client ad-blockers) is skipped for now: revisit
  if event loss becomes visible.
