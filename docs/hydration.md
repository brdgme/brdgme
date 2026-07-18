# Hydration Guide

How SSR hydration works in this stack, the rules that keep it safe, known
hazards, a troubleshooting playbook, and case history. Complements the
"Leptos: SSR and Hydration" section of `docs/CODING.md` (which covers
day-to-day coding rules); this doc goes deeper into the mechanics and
debugging.

Stack: Leptos 0.8.20, tachys 0.2.18, leptos_router 0.8.14,
hydration_context 0.3.1, SSR with out-of-order streaming (the default
mode), built with cargo-leptos.

Official references:

- Book, "Hydration Bugs": https://book.leptos.dev/ssr/24_hydration_bugs.html
- Book, "SSR Modes" (streaming): https://book.leptos.dev/ssr/23_ssr_modes.html
- docs.rs `LocalResource`:
  https://docs.rs/leptos/0.8.20/leptos/prelude/struct.LocalResource.html
- docs.rs `Effect`:
  https://docs.rs/leptos/0.8.20/leptos/prelude/struct.Effect.html

## How hydration works here

- **SSR pass**: the server renders the page to HTML. With out-of-order
  streaming, `Suspense`/`Transition` boundaries whose resources are still
  pending emit their fallback inline plus marker comments; when a resource
  resolves later in the response, the resolved children are streamed as a
  `<template id="N-f">` fragment plus a small script that swaps it in.
  `Resource::new_blocking` instead blocks the response until resolved, so
  its content is in the initial HTML.
- **Hydration pass**: the client WASM re-runs the same component tree and
  walks the real DOM with a cursor (tachys), matching each virtual node it
  builds against the next DOM node. It does not diff or re-render; it
  attaches. Any structural divergence between what SSR emitted and what the
  client builds derails the cursor.
- **SerializedDataId allocation**: ids for serialized resources and
  boundaries are allocated from a flat, monotonically increasing counter in
  tree-construction order, independently on BOTH passes. There is no keying
  or reconciliation - the server's id N and the client's id N match only if
  both passes construct the same resources/boundaries in the same order.
- **SuspenseBoundary run 0**: on its first run during hydration, a boundary
  renders its children (not the fallback) unless it "starts local" - which
  it determines by looking its server-side id up in the incomplete-chunk
  set. If the boundary was incomplete on the server (fallback HTML was
  sent), the client must render the fallback on run 0 to match.
- **Incomplete chunks**: boundaries the server could not resolve before the
  response ended are announced to the client via `__INCOMPLETE_CHUNKS`,
  keyed by the server-side SerializedDataId. This is how the client learns
  "this boundary's HTML is the fallback, not the children".
- **LocalResource never resolves on SSR**: docs.rs is explicit that a
  `LocalResource` "will only begin loading data if you are on the client
  (i.e., if you do not have the `ssr` feature activated)". On the server it
  is pending forever, so any boundary that waits on one can never resolve
  server-side. The docs state only the client-only loading; the
  consequences for a server-rendered boundary (below) are ours, confirmed
  by source reading and the 2026-07 incident.

## Rules to stay safe

The core invariant (book, "Hydration Bugs"): the HTML the server sends and
the client's first render must be identical.

1. **Create resources and boundaries unconditionally and in identical order
   on both passes.** SerializedDataId matching depends entirely on
   construction order. Never create a resource or `Suspense` inside a
   branch that only one side takes, and never let SSR construct extra ones
   the client will not (see the hazard below).
2. **Never read a LocalResource (or any source that never resolves on the
   server) under a boundary that SSR must render.** The boundary will be
   serialized as an incomplete chunk (fallback HTML) at best, or hang the
   stream at worst, and the client-side accounting has to line up exactly
   for hydration to survive it.
3. **Keep browser-only APIs out of SSR paths.** `js_sys`/`web_sys` calls
   (beyond pure rendering) must only be reachable from effects, event
   handlers, or code gated so it cannot run during the server or hydration
   render. Example: `format_log_time` in
   `rust/web/src/components/game.rs` uses `js_sys::Date` and is only
   reachable via a `LocalResource` value, which is `None` on both SSR and
   the initial client pass.
4. **For client-only content, use the mounted-gate idiom.** `Effect::new`
   is inert on SSR and runs on the client after hydration (docs.rs Effect:
   "effects do not run on the server"; the book's "Hydration Bugs" chapter
   recommends wrapping browser-only work in `Effect::new` and driving the
   view from a signal whose initial value both sides render). Leptos 0.8
   ships no `ClientOnly` component and does not name this pattern, but it
   is a direct specialization of that documented effect-plus-signal
   advice. Both passes render the gated content as nothing, and it appears
   immediately after mount:

   ```rust
   // From rust/web/src/components/game.rs (GameLogs):
   // Effects never run during SSR, so `mounted` is false in the server
   // HTML and on the client's initial hydration pass - both sides render
   // nothing, sidestepping a hydration-cursor mismatch.
   let mounted = RwSignal::new(false);
   Effect::new(move |_| mounted.set(true));

   view! {
       {move || mounted.get().then(|| logs.get()).flatten().map(|result| ...)}
   }
   ```

   If an effect genuinely must run on the server too, that is what
   `Effect::new_isomorphic` is for - never use it for a mounted gate.
   Related upstream alternatives, for awareness only: the islands
   architecture (`experimental-islands`) is a page-level way to limit
   hydration to marked components, and the third-party `leptos_hydrated`
   crate packages a "hydrated" signal equivalent to our gate. Neither is
   worth adopting here.
5. **Beware cleanup-race closures.** `request_animation_frame` and event
   handler closures can fire after the reactive scope that created them is
   disposed (navigation, remount). Reading a `NodeRef` or signal with
   `get_untracked` then panics; use `try_get_untracked` and handle `None`.
   See the raf scroll handlers in `GameLogs`/`RecentGameLogs`.
6. Also observe the rules in `docs/CODING.md` ("Leptos: SSR and
   Hydration"): resource-type selection, layout-outside/content-inside
   `Suspense`, unconditional resource reads inside Suspense closures, and
   structural-vs-attribute mismatch semantics. The book additionally calls
   out invalid HTML nesting (e.g. `<p><div>`) as a hydration killer: the
   browser rewrites invalid markup before the cursor walks it.

## Known hazard (upstream): id skew for boundaries inside reactive closures

Boundaries created inside a reactive closure under an async-resolving
parent (e.g. a `Suspense` inside the render closure of a `Transition`
tracking a blocking resource) get skewed hydration ids on the server. The
server's suspense machinery calls the closure more than once -
`dry_resolve` and `resolve` each re-invoke it (leptos-0.8.20
`suspense_component.rs:383` and `:445`) - and every invocation burns
SerializedDataIds from the flat counter. The client constructs the tree
once, so its ids for those inner boundaries are lower than the server's.
The incomplete-chunk protocol is keyed by server-side ids, so the client
looks up its own (different) id, misses the entry, assumes the boundary's
children were rendered, and the cursor walks into fallback HTML.

Consequence: **nesting a `Suspense` around a LocalResource under an
async-resolving boundary is NOT a safe fix** - that was the 63fef22
band-aid and it still panicked. The mounted-gate avoids the problem
entirely because both passes render nothing regardless of what ids the
boundaries got.

This looks like an upstream Leptos bug worth filing (official docs never
mention SerializedDataId/incomplete-chunk internals; the analysis here is
ours, from reading leptos 0.8.20 / tachys 0.2.18 sources during the
2026-07 incident).

## Troubleshooting playbook

- **`failed_to_cast_element` (tachys `hydration.rs:163`)**: the hydration
  cursor found a DOM node of the wrong type where the client expected a
  specific element. The node it names is where the walk fell over; the
  actual divergence may be earlier in the document (the cursor can drift
  for a while before hitting a hard cast).
- **Get the exact source location**: rebuild with
  `RUSTFLAGS="--cfg leptos_debuginfo"` - the panic then includes
  "element defined at ..." pointing at the view! line of the mismatched
  element.
- **Inspect the raw SSR HTML**: fetch the page without JS side effects
  (`curl http://localhost:3000/games/<id>` or Playwright's
  `page.request`), and read the marker comments to compare server output
  against what the client will build:
  - `<!--s-N-o-->` / boundary markers: suspense boundary N open/close
  - `<template id="N-f">`: streamed resolved content for boundary N
  - `<!>` and `<!--<() />-->`: placeholders for empty/unit views
  - `__INCOMPLETE_CHUNKS`: the ids of boundaries whose fallback was sent
- **Run the e2e regression test**: `rust/web/end2end/tests/page-loads.spec.ts`
  asserts zero console errors and waits on `data-hydrated`. Hydration
  panics are silent on client-side navigation and only surface on hard
  refresh, which is exactly what this test does. To run it: start the dev
  docker containers (`brdgme-postgres-dev`, `brdgme-nats-dev`), then
  `SQLX_OFFLINE=true cargo leptos build --release`, then
  `E2E_SKIP_BUILD=1 ./run.sh` from `rust/web/end2end`.

## Case history: the 2026-07 game-page incident

Two releases, one root cause:

1. `GameLogs`/`RecentGameLogs` read a `LocalResource` (game logs) inside
   the game page's `Transition`. On SSR the LocalResource never resolves,
   so the server emitted fallback HTML for that region while the client's
   first render built children - hydration panic
   (`failed_to_cast_element`) on hard refresh of `/games/<id>`.
2. **63fef22** wrapped the log components in nested
   `<Suspense fallback=|| ()>` so the outer `Transition` would not observe
   the pending LocalResource on SSR. It still panicked: the nested
   boundaries live inside the Transition's render closure, so the id-skew
   hazard above applied - server ids and client ids diverged and the
   incomplete-chunk lookup missed.
3. **c7e63d1** shipped the real fix: the mounted-gate in
   `GameLogs`/`RecentGameLogs` (render `None` until an `Effect` flips
   `mounted`), making SSR and hydration run 0 identical regardless of
   boundary ids. The gate exposed a latent second bug - raf scroll
   closures outliving their scope and panicking in `NodeRef::get_untracked`
   on remount - fixed with `try_get_untracked`. The page-loads e2e test
   was re-enabled and strengthened with `data-hydrated` waits.

The nested `Suspense` wrappers remain in the code (`app.rs` GamePage and
`GameMeta` in `components/game.rs`) - they still serve to keep the outer
`Transition` from tracking the pending LocalResource on SSR - but the
mounted-gate is what makes hydration safe.
