# 18: Production Hardening - Design

> Extracted 2026-07-08 from `docs/plan/18-production-hardening.md` (superpowers layout
> migration). Content dates from 2026-07-07; this is a point-in-time decision
> record, not a living document.

**Status:** Pending (fully specced 2026-07-05; delegable except the marked
human tasks)

## Sequencing

**Resequenced 2026-07-04: pre-go-live.** With the hard-cutover decision
(Phase 16) this phase moves before cutover so error visibility exists from
day one of prod traffic - the Phase 16 validation gate explicitly checks
the log/trace backend for unexplained 5xx/panics.

## Decisions

**Decision (2026-07-05): all-in Grafana Cloud free tier.** Supersedes the
2026-07-03 VictoriaLogs decision and the planned VictoriaMetrics/vmalert
additions. Logs, metrics, traces (APM), dashboards, and alerting (including
email delivery) all go to a Grafana Cloud free-tier stack; the only
in-cluster observability component is a single Grafana Alloy agent.
Rationale: the s-2vcpu-4gb single-node budget cannot comfortably carry
VictoriaLogs + Vector + VictoriaMetrics + vmalert + Alertmanager alongside
the app (~1-1.5GB), Grafana Cloud's free tier (50GB logs, 10k metric
series, 50GB traces, alert rules + email contact points) covers this
project's volume by orders of magnitude, and it adds real APM - wanted for
the hard cutover's first weeks. Exit path if the free tier degrades:
collection stays in-cluster (Alloy speaks Loki/Prometheus/OTLP protocols),
so the backends can be swapped back to self-hosted VictoriaLogs/
VictoriaMetrics by changing Alloy's write endpoints - no app changes.

**Decision (2026-07-05): no in-cluster alert evaluation.** vmalert /
Alertmanager / a webhook-to-Resend bridge are all trimmed - alert rules run
in Grafana Cloud against the shipped telemetry, and Grafana Cloud sends the
emails itself (Resend is NOT used for alerts). The blind spot ("the cluster
stopped shipping telemetry at all") is covered by (a) a Grafana Cloud
no-data alert on a heartbeat metric and (b) the external uptime monitor
(see the implementation plan). Documented fallback if in-cluster alerting
is ever needed: a `POST /api/webhooks/alertmanager` endpoint on the
monolith sending via the existing `resend-rs` client - do not build it now.

## Deferred: client-side error telemetry

**Note (2026-07-05):** client-side error *telemetry* - getting production
users' WASM panics to reach the operator at all - remains a known gap. It is
intentionally deferred as a separate future decision (Sentry, a
panic-reporting endpoint, etc. are all out of scope here). The WASM source
maps item in the implementation plan is a local debugging aid only: it makes
a panic already visible in someone's browser console (or reported by a user)
resolve to a real Rust file:line, not a mechanism for getting that panic to
the operator.

## Not planned

- Client-side error reporting services (Sentry etc.) have immature WASM
  support and add meaningful bundle size. Not worth the trade-off given that
  the SSR layer already captures the important server-side errors and
  `console_error_panic_hook` handles client panics.
