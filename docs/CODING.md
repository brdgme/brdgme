# Coding Guidelines

Rules for all contributors to `brdgme`. Covers patterns established through
implementation experience. Follow these unless there is a compelling reason not
to, and document the exception when you deviate.

---

## Rust: Error Handling

**No panicking code in runtime paths.** `.unwrap()`, `.expect()`, `panic!()`,
and `unreachable!()` are forbidden in server request handlers, database
functions, and Leptos component code. A panic in the server process kills the
request. A panic in WASM kills the frontend session.

**Acceptable panics:**
- Process startup failures (`main.rs`, session store init, DB pool creation).
  If the process cannot start correctly it should not start at all.
- Inside `#[cfg(test)]` blocks.
- `unreachable!()` in `#[cfg(not(feature = "ssr"))]` client stubs for server
  functions. These code paths are never compiled into the server and are
  provably unreachable on the client.

**Propagate errors with `?`.** Server functions and DB functions return
`Result`. Use `ok_or_else(|| ...)` to convert `Option` to `Result` with a
descriptive message before propagating with `?`. Never silently swallow errors.

**DOM access in event handlers.** `NodeRef::get()` returns `Option`. Do not
`.unwrap()` it. Use `.map(|el| ...)` and return early or use a default on
`None`. The node may not be mounted if an event fires at an unexpected time.

---

## Leptos: SSR and Hydration

Leptos renders pages on the server (SSR) and then the WASM re-runs the same
component logic in the browser to attach to the existing DOM nodes (hydration).
If server and client produce structurally different HTML, Leptos panics with an
unrecoverable hydration error. These errors are silent during client-side
navigation and only appear on hard refresh, making them easy to miss.

### Choosing a resource type

| Type | SSR behaviour | When to use |
|------|--------------|-------------|
| `Resource::new_blocking` | Blocks page response until resolved; data available immediately on client | Data that must be in the initial HTML (game board, page content) |
| `Resource::new` | In streaming SSR, emits a `<!-- -->` placeholder in initial HTML and streams resolved content via `<template>`. If WASM hydration runs before the streaming JS processes the template, client sees `<!-- -->` where it expects content. | Rarely needed; prefer `new_blocking` or `LocalResource` |
| `LocalResource::new` | Always `None` on SSR; fetches fresh after hydration | Secondary UI data where a loading state after hydration is acceptable (sidebar lists, log panels) |

**`LocalResource` is the safe default for anything that is not core page
content.** It cannot produce a hydration mismatch because SSR and client both
start as `None`.

### Suspense and new_blocking

Two conflicting constraints apply to `Resource::new_blocking`:

1. **`Suspense` introduces a streaming placeholder `<!-- -->` at its boundary**
   in SSR output, even when all tracked resources are `new_blocking`. Any DOM
   node placed inside a `Suspense` may appear as `<!-- -->` in the initial HTML,
   causing a hydration mismatch if WASM expects a real element there.

2. **Reading `new_blocking` outside `Suspense` returns `None` in hydrate mode.**
   Leptos warns: "reading a resource in hydrate mode outside a Suspense or
   effect causes hydration mismatch errors." Without `Suspense`, the resource
   is `None` during the initial WASM reactive pass, so any content that depends
   on it renders as `<!-- -->` on the client while SSR had the resolved content.

**The solution: keep layout wrappers outside `Suspense`, data-dependent content inside.**

```rust
// Wrong: layout wrapper inside Suspense → <!-- --> at layout level
<Suspense fallback=...>
    {move || game_data.get().map(|_| view! { <MainLayout>...</MainLayout> })}
</Suspense>

// Wrong: no Suspense → game_data is None in hydrate mode → content is <!-- -->
{move || game_data.get().map(|_| view! { <div class="game-board">...</div> })}

// Correct: layout outside, content inside Suspense
<MainLayout>
    <Suspense fallback=|| view! { <div></div> }>
        {move || {
            let base = game_data.get(); // read first so Suspense tracks it
            base.map(|res| match res { ... })
        }}
    </Suspense>
</MainLayout>
```

The layout wrapper (`MainLayout`) is always in the initial SSR HTML — no
streaming placeholder risk. `Suspense` defers hydration of the inner content
until `game_data` deserializes from the serialized resource state, at which
point both SSR and client have the resolved data. Match ✓.

`Transition` renders children directly on SSR with no fallback mechanism.
If those values are `None`, SSR emits `<!-- -->`. Avoid `Transition` for
components that need SSR data.

### Structural vs attribute hydration mismatches

Leptos hydration checks **element type and hierarchy** — it does not check
attribute values, class names, or inline styles. This means:

- **Structural differences** (different element types, presence/absence of
  elements) always cause hydration errors.
- **Attribute/class differences** (e.g. a class present on SSR but absent on
  client) do not cause errors — reactive bindings attach after the structural
  traversal and update the DOM without panicking.

Consequence: when a component prop controls which element to render (e.g.
`if condition { <input/> } else { <span/> }`), and that prop depends on async
data that starts as `false`/`None` in hydrate mode, the structural mismatch
will panic. Fix by making the element always present and toggling visibility:

```rust
// Wrong: structural difference when condition differs between SSR and client
{if has_next_game { view! { <input.../> } else { view! { <span/> } }}

// Correct: same element always rendered; only a CSS attribute changes
<input type="button" value="Next game" hidden=move || !has_next_game.get()/>
```

### Reactive props for layout components

When a layout component (like `MainLayout`) sits outside `Suspense` but its
props depend on async data, use `Signal<bool>` with `#[prop(into, default)]`.
`Signal<T>` is in `leptos::prelude::*` — no extra import needed. It implements
`From<T>` (so `Signal::from(false)` works as a default), `From<Memo<T>>`, and
`From<RwSignal<T>>`, and is `Copy`.

```rust
#[component]
pub fn MainLayout(
    #[prop(into, default = Signal::from(false))] is_my_turn: Signal<bool>,
    ...
) -> impl IntoView {
    view! {
        <div class:my-turn=move || is_my_turn.get()>
            ...
        </div>
    }
}
```

Callers pass a static `bool` (converted via `Signal::from(true)`), or a reactive
`Memo<bool>` (converted via `Signal::from(memo)`):

```rust
let is_my_turn = Memo::new(move |_| { ... });
view! {
    <MainLayout
        is_my_turn=Signal::from(is_my_turn)
        has_sub_menu=Signal::from(true)
    >
```

In `GamePage`, derive the value with a `Memo` that reads from both the blocking
resource and the WS signal. In hydrate mode the `Memo` returns `false` until the
resource deserializes — this changes a CSS class only (no structural mismatch).

**Do not use `MaybeSignal<T>`.** It is deprecated in `reactive_graph 0.2.11` in
favour of `Signal<T>`. `Signal<T>` covers the same use-cases and is always
`Copy`.

### Resource placement

A resource must be created in the same component (or a direct ancestor) that
owns the `Suspense` tracking it. Passing a resource via context and then
reading it inside a `Suspense` in a different component breaks SSR tracking -
the `Suspense` cannot see the resource as pending and will not wait for it.

### Resource read order inside Suspense closures

`Suspense` tracks resources by observing which ones are read during the
evaluation of its children. If a closure inside `Suspense` reads a context
signal first and returns early before calling `.get()` on the resource,
the `Suspense` never sees the resource and will not wait for it on SSR.

**Always bind the resource read unconditionally before any branching logic:**

```rust
// Wrong: ws_game check can short-circuit before game_data is read
{move || {
    if let Some(ws) = ws_game.get() {
        if ws.game_id == id { return Some(Ok(ws.data)); }
    }
    game_data.get().map(...)
}}

// Correct: game_data is always read first; Suspense always sees it
{move || {
    let base = game_data.get();        // Suspense sees this unconditionally
    let effective = ws_game.get()
        .filter(|ws| ws.game_id == id)
        .map(|ws| Ok(ws.data))
        .or(base);
    effective.map(...)
}}
```

---

## Leptos: State and Context

**Context is for app-wide cross-cutting concerns only.** Legitimate uses:
- The WebSocket game update signal (`RwSignal<Option<BrdgmeGameUpdate>>`):
  written by the WS client at app level, read by game components at arbitrary
  depth.
- `WebSocketTrigger`: the refetch counter, needed by both the WS client and
  action components across the tree.
- Server-side request context (`PgPool`, `reqwest::Client`): injected by the
  Axum handler and read inside server functions.

**Do not use context to share data between sibling or cousin components.**
If only one component needs a piece of data, that component creates and owns
it. Data flows from parent to child via props, not laterally via context.

**The component that consumes data owns the resource.** Do not hoist a
`Resource` or `LocalResource` to a parent component just because it seems
like a natural place to put it. Hoisting breaks SSR tracking (see above) and
makes data flow implicit.

---

## Leptos: Component Design

**Props over context for parent-to-child data.** If a parent component has
data that one or two child components need, pass it as a prop. Only reach for
context when the data is needed at many places across the tree without a clean
prop-drilling path.

**Keep reactive closures structurally stable.** Leptos hydration requires that
the HTML structure produced on the server matches the client's initial render
exactly. Avoid conditional rendering that produces structurally different
output on server vs client. When in doubt, use `LocalResource` to ensure both
sides start from the same empty state.

---

## Dependency Management

**`wasm-bindgen` is pinned** to `=0.2.108` in `Cargo.toml` to match the
`wasm-bindgen-cli` version provided by nixpkgs. Do not update it without
updating `devenv.nix` in lockstep. A version mismatch between the CLI and the
crate causes the WASM build to fail at link time.

**Other pinned-by-ecosystem crates** (as of 2026-04-04): `gloo-net 0.6`,
`gloo-timers 0.3`, `js-sys/web-sys 0.3.85`, `tower 0.4`, `tower-sessions 0.14`,
`reqwest 0.12`, `redis 0.28`. Newer major versions have breaking API changes and
require coordinated updates. Run `cargo update --verbose` periodically to check
for patch-level updates; ignore "Unchanged" lines where a newer major version
exists but the Cargo.toml constraint intentionally excludes it.

**rustls crypto backends: the workspace enables both, and any binary relying
on the process default provider must install one in `main`.** `reqwest`'s
`rustls` feature enables rustls' `aws-lc-rs` backend, while the defaults of
`sqlx` (`tls-rustls`), `kube`, and `async-nats` enable `ring`. With both
backend features enabled, rustls 0.23 cannot auto-select a process-level
`CryptoProvider` and panics at the first use of it. This is invisible to CI
(dual backends are legal feature unification and no test opens a TLS
connection) and to dev (the dev Postgres connection is plaintext); it first
surfaced as the operator CrashLooping in prod (2026-07-08). Whether a crate
is affected depends on how it builds its rustls config: `sqlx` selects its
backend explicitly and `reqwest` falls back to its own default, so both are
immune; `kube` uses the bare process default and panics. The rule:

- Any binary using a crate that reads the process default provider (today:
  `kube` in the operator) must call
  `rustls::crypto::aws_lc_rs::default_provider().install_default()` at the
  top of `main` (see `rust/operator/src/main.rs`). When adding a new
  TLS-using dependency, check how it obtains its provider; when in doubt,
  install the default - it is always safe.

Full consolidation on `aws-lc-rs` (banning `ring` in `deny.toml`) was
implemented and then deliberately reverted on 2026-07-08: it required
`default-features = false` plus hand-copied default-feature lists on `kube`
and `async-nats`, which silently drop new upstream default features on every
upgrade. That maintenance cost outweighs the marginal benefit while
`install_default()` already eliminates the panic. Revisit if upstream
defaults flip to `aws-lc-rs` (kube and async-nats both already expose the
feature), at which point the migration becomes one-line feature swaps plus a
`ring` entry in the `deny.toml` `[bans]` deny list.

---

## Game Services

**The code is authoritative, not the physical rulebook.** When implementing
game rules documentation, read the source (`lib.rs`, `command.rs`, `render.rs`,
`card.rs`) and follow the code. Where the code and rulebook disagree, the code
wins.

**Embed rules at compile time.** The `rules()` method on `Gamer` must return
`include_str!("../RULES.md").to_string()`. This ensures Tilt rebuilds the game
container image whenever `RULES.md` changes, and the operator can read the
current rules from the running service.

**Use real renders in RULES.md.** The "Reading the Display" section must
contain actual brdgme markup output from the game binary, not a hand-crafted
ASCII approximation. See `docs/RULES.md` for the extraction process.

---

## Testing Conventions

**`db.rs`, `game/mod.rs`, and `auth/` require tests.** These are the files
agents and reviewers touch most often, and they are also the files where a
silent regression is most dangerous (money-equivalent state: game outcomes,
ratings, login). New or changed logic in `rust/web/src/db.rs`,
`rust/web/src/game/mod.rs`, or `rust/web/src/auth/` must land with tests
covering the change. A PR touching these files without tests should be
rejected in review, whether the reviewer is human or an agent.

**Never call the real game service or the LLM in a test.** `rust/web` tests
mock the game service HTTP layer with an in-process Axum server returning
canned `Response` JSON - see the pattern in `rust/web/src/game/client.rs`.
This keeps tests fast and deterministic and avoids depending on a running
game binary. The LLM is never called in any test; bot-loop behaviour that
would require a live LLM call is out of scope for the test suite (see
`docs/plan/11-testing-foundation.md` for the current deferral).

**Use `#[sqlx::test]` for anything touching the database.** It gives each
test its own isolated, migrated database, so tests never share state with
each other. Do not build ad hoc shared fixtures or rely on test ordering;
each `#[sqlx::test]` function should set up exactly the rows it needs.

**Two-layer frontend/page testing: prefer the in-process layer.** Page-level
coverage is split into two layers (see `docs/plan/11-testing-foundation.md`
11.6):

- **In-process SSR page tests (primary)** - `#[sqlx::test]` +
  `tower::ServiceExt::oneshot` against the real Axum/Leptos router (see
  `rust/web/tests/ssr_pages.rs`, built via the shared `web::router::build_router`
  helper). No browser, no running binary; runs in the existing `test-rust` CI
  job in milliseconds. Use this layer for route/page coverage: assert 200,
  `text/html`, a page-specific marker, and no SSR panic. This is where new
  page or route coverage should be added by default.
- **Playwright hydration smoke (residue only)** - a single spec,
  single browser context, chromium only (`rust/web/end2end/tests/page-loads.spec.ts`).
  The only thing that genuinely requires a real browser is client-side
  hydration (hydration mismatches and WASM panics only manifest on a hard
  page load), so this layer is a hard-load smoke test asserting zero console
  errors/`pageerror`s, not a scenario suite. Do not add multi-context,
  WebSocket-propagation, or command/undo/concede/restart driving here - that
  logic is covered by Rust tests (11.2-11.4). Keep this layer under its time
  budget (currently < 1 minute of Playwright time, excluding the release
  build).
