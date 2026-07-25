# Unit 12 bot-operator-tools - Lead log

Session 2026-07-24. Lead: Fable 5 (this session). Snapshot
`f8763a5ba9c0ce3d0e85d61db7133d19a26ed313` at
`/home/beefsack/Development/brdgme-review-snapshot/rust`.

Scope: `bot/` (1,708 LOC, 6 files), `operator/` (412 LOC, 3 files),
`tools/fuzz` (358), `tools/render_plain` (32), `tools/repl` (10).
SKIP `lib/rand_bot` (covered by unit 2 lib-support).

## Worker plan (serial)

| Worker | Scope | LOC | Raw dump |
|---|---|---:|---|
| W1 | bot/ (all 6 files) | 1,708 | raw/bot-operator-tools-bot.md |
| W2 | operator/ + tools/fuzz + tools/render_plain + tools/repl | ~812 | raw/bot-operator-tools-operator-tools.md |

Workers on model fable per user override. Established finding format
(severity critical/major/minor/nit; category correctness/quality/
simplicity/consistency/dependencies; location relative to rust/ with
snapshot line numbers; finding; recommendation; clean areas listed).

### W1 dispatched
- Scope: bot/ full crate, all 6 src files + Cargo.toml + templates.

### W1 returned
- 16 findings: 0 critical / 2 major / 9 minor / 5 nit in
  raw/bot-operator-tools-bot.md.
- Headlines: reachable `unreachable!()` at main.rs:454 (continue paths
  skip MAX_ATTEMPTS check); no AckKind::Progress during long LLM turns
  (redelivery double-processing, UNCERTAIN on ack_wait); merge_json_patch
  not RFC 7396; unwrap_or masking decode errors; unused deps + archived
  serde_yaml.
- Clean: crypto, nats event structs vs web, templates, provider failover.
- Lead verification of both majors PENDING before curation.

### W2 dispatched
- Scope: operator/ + tools/fuzz + tools/render_plain + tools/repl.

### W2 returned
- 14 findings: 0 critical / 2 major / 6 minor / 6 nit in
  raw/bot-operator-tools-operator-tools.md.
- Headlines: hand-rolled finalizer add/remove via whole-array Merge
  patch vs kube-rs finalizer(); fuzz step_rx.recv() hangs if all worker
  threads die (parent keeps step_tx alive); crd printcolumn jsonPath to
  nonexistent field.
- Clean: render_plain, repl in full; operator upserts vs schema.
- Lead verification of all 4 unit majors PENDING.

## Lead verification

- VERIFIED main.rs:242 `for attempt in 0..MAX_ATTEMPTS`, continues at
  311/372 skip check at 420; unreachable!() at 454 reachable. CONFIRMED.
- VERIFIED ack sites main.rs:824/856-862, no AckKind::Progress anywhere
  in bot/. CONFIRMED (kept UNCERTAIN qualifier re monolith ack_wait).
- VERIFIED controller.rs:80-105 whole-array Patch::Merge finalizer
  writes. CONFIRMED.
- VERIFIED fuzz lib.rs:23 original step_tx kept alive, clones at 27,
  recv().expect at 59. CONFIRMED.
- VERIFIED merge_json_patch recursion condition (main.rs:625) and
  crd.rs:18 printcolumn `.spec.playerCounts` vs spec fields. CONFIRMED.
- No rejections or downgrades; all 30 raw findings curated as-is.

## Curation complete (2026-07-24)

Curated file: findings/bot-operator-tools.md. Tally: 0 critical /
4 major / 15 minor / 11 nit (30 findings). Unit 12 CLOSED.

(Unit 13 was subsequently completed in the same session; see
raw/dependencies-LOG.md for the joint completion entry.)
