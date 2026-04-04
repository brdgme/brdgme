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
| `Resource::new` | Requires `Suspense` in the same component to resolve during SSR | Rarely needed; prefer `new_blocking` or `LocalResource` |
| `LocalResource::new` | Always `None` on SSR; fetches fresh after hydration | Secondary UI data where a loading state after hydration is acceptable (sidebar lists, log panels) |

**`LocalResource` is the safe default for anything that is not core page
content.** It cannot produce a hydration mismatch because SSR and client both
start as `None`.

### Suspense vs Transition

- Use `Suspense` when the component renders for the first time on a hard
  refresh. `Suspense` with a `new_blocking` resource blocks the page and
  renders the actual resolved content, so SSR and client always agree.
- `Transition` keeps the previous content visible while new content loads.
  It does not render the fallback on SSR initial render - it renders the
  children directly with whatever reactive values are available
  synchronously. If those values are `None`, SSR emits `<!-- -->` and the
  client hydrates against missing content. Avoid `Transition` for top-level
  page components that need SSR data.

### Resource placement

A resource must be created in the same component (or a direct ancestor) that
owns the `Suspense` tracking it. Passing a resource via context and then
reading it inside a `Suspense` in a different component breaks SSR tracking -
the `Suspense` cannot see the resource as pending and will not wait for it.

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
