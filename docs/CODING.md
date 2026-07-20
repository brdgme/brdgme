# Coding Guidelines

Rules for all contributors to `brdgme`. Covers patterns established through
implementation experience. Follow these unless there is a compelling reason not
to, and document the exception when you deviate.

---

## General Principles

**Quality over shortcuts.** Fix the root cause, not the symptom, even if it
takes longer. A quick patch that leaves the underlying problem in place
creates more work later.

**No non-Rust dependencies.** Do not add new services, tools, or libraries in
Go, Node, Python, or any language other than Rust to solve a problem. (The
existing Go game services and `brdgme-go` are legacy, being retired under #31
- see `docs/decisions/GO_VS_RUST_PORTING.md` - not a precedent for new
non-Rust work.)

**Documented exception: Sentry.** The Sentry browser JS SDKs (`@sentry/browser`
+ `@sentry/wasm`, bundled with esbuild at Docker image build time) are the
sole non-Rust runtime dependency in the codebase - no Rust alternative exists
for capturing raw browser wasm addresses for server-side symbolication. A
second, operational-tooling exception covers the hosted Sentry SaaS itself:
it observes the platform but is not required to run or play brdgme. See
`docs/decisions/SENTRY_SAAS_EXCEPTION.md` for the full record.

**No bespoke code.** Prefer an existing, well-maintained crate over a
hand-rolled implementation of a solved problem (parsing, hashing, retries,
etc.) - see Dependency Management below for crates already in use.

**No clever code.** Prefer the boring, obvious implementation over a clever
one. Code is read far more often than it is written - optimize for the next
reader, not for showing off.

**Comment discipline.** Default to no comments. Only add one when the *why*
is non-obvious - a hidden constraint, a workaround for a specific bug, or
behavior that would surprise a reader. Never comment on *what* the code does
when identifiers already say it.

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

For the underlying mechanics (hydration ids, incomplete chunks, the
mounted-gate idiom), known upstream hazards, and a debugging playbook, see
`docs/hydration.md`.

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

## Leptos: Forms

**Use `FormField` for every new form control.** `FormField`
(`rust/web/src/components/form.rs`) renders a bold block label above the
control, an optional muted help line, and an optional red error line -
`.form-field` / `.form-label` / `.form-control` / `.form-help` / `.form-error`
in `main.scss`. Don't hand-roll label/input markup for a new form; wrap the
control in `<FormField>` instead. `.form-actions` (flex row, `gap: 0.5em`) is
the class for a form's button row (see `UsernameSection` in
`rust/web/src/settings.rs`).

**Constrained-content pages cap width via a page-level class, not per-field.**
`.settings` in `main.scss` sets `max-width: 40em; padding: 0 1em;`, with a
sibling rule capping its `input`/`select` children at `max-width: 100%` so
they never overflow the column. Add the same pair of rules for any new
narrow-content page rather than constraining individual controls.

**Save model: match the field's error surface, not one page-wide dirty
state.** Fields that can be rejected server-side (e.g. username: format or
uniqueness) get an explicit Save button and render the rejection inline via
`FormField`'s `error` slot. Fields that are just a choice among valid options
(theme, preferred colours) save immediately on change, fire-and-forget - no
loading state, no page-wide "unsaved changes" banner. See `UsernameSection`
vs `ColorsSection`/`ThemeSection` in `rust/web/src/settings.rs`.

**`<option selected>` only sets `defaultSelected` - drive the value via
`prop:value` on the `<select>`.** Setting the HTML `selected` attribute on an
`<option>` inside a Leptos view only affects the initial render; it does not
keep the select in sync when the backing signal changes later, and doing it
per-`<option>` fights hydration. Bind `prop:value` on the `<select>` itself
instead (see `ColorsSection` in `rust/web/src/settings.rs`).

**`class:selected` is the reactive-highlight pattern for tile/chip pickers.**
A boolean closure toggling one class on an always-present element (not a
conditional element swap - see the Component Design section on structural
hydration mismatches) is how the theme tiles and equivalent pickers show which
option is active; see `ThemeSection`'s `class:selected=move || ...` in
`rust/web/src/settings.rs`.

**Redirect anonymous users from a logged-in-only page via an `Effect`, not a
structural `if`.** `SettingsPage` reads the shared `current_user`
`LocalResource` and calls `use_navigate()` inside an `Effect::new` once it
resolves to `Ok(None)`; SSR and initial hydration render the page normally
(the resource is `None` at that point), and the navigate only fires
client-side once the anonymous state is known - no structural mismatch.

**`PLAYER_COLOR_NAMES` (`rust/web/src/theme.rs`) is the single source of the
player colour palette.** Anything offering a colour choice - selects,
`ColorChip` previews - iterates this constant rather than hard-coding colour
names. Values read back from storage go through `normalize_pref_color` first
(legacy names like "Amber"/"BlueGrey" mapped onto their current slot names -
`db.rs`), and distinctness/membership is validated *after* normalization, not
before (`validate_pref_colors` in `auth/server.rs`).

---

## Server Functions

**Guard logged-in-only server fns with `get_current_user`, inline.** There is
no separate auth middleware layer for server fns - each one starts with
`get_current_user().await?.ok_or_else(|| ServerFnError::new("Not
authenticated"))?` and uses the returned user directly (see `get_settings`,
`set_username`, `set_pref_colors`, `set_theme` in `rust/web/src/auth/server.rs`).

**Expected rejections are data, not `ServerFnError`s.** `set_username`
returns `Result<Option<String>, ServerFnError>`: `Ok(None)` is success,
`Ok(Some(message))` is a field error to render inline (bad format, or "That
name is taken"). `Err` is reserved for real failures - not authenticated, DB
down, transport errors. This lets the calling form distinguish "please fix
this field" from "something broke" (see `UsernameSection`'s two-armed
`match` on the action result in `rust/web/src/settings.rs`).

**Map a Postgres unique-violation SQLSTATE to a field result in the DB
helper, not the server fn.** `set_user_name` (`db.rs`) matches
`Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23505")` and
returns `Ok(false)`; the server fn turns `false` into the "That name is
taken" field error. The server fn layer never inspects Postgres error codes
directly.

**Validation shared between client and server must be a plain, ungated
function.** `validate_username` (`db.rs`) and `validate_pref_colors`
(`auth/server.rs`) carry no `#[cfg(feature = "ssr")]` gate and touch only
strings/vecs - so the identical rule compiles into both the wasm client (for
immediate form feedback) and the server (for actual enforcement). The server
copy is what's authoritative; never rely on the client-side check alone.

**Plain (non-macro) sqlx queries avoid `.sqlx` offline-data regeneration.**
`get_user_theme`/`set_user_theme`/`set_user_name`/`get_user_pref_colors`/
`set_user_pref_colors` (`db.rs`) use `sqlx::query`/`sqlx::query_as` instead of
`query!`/`query_as!` specifically so adding a new column read/write doesn't
require regenerating the `.sqlx` cache against a live database (not always
available). Follow this convention for any new query touching a column not
already covered by an existing macro query.

---

## Dependency Management

**Stay on latest dependencies.** We aggressively track the latest releases of
everything: crates, build tools, Docker base images, pinned CLI versions
(Dockerfile, devenv.nix), and GitHub Actions. Whenever a feature or a
troubleshooting effort might be affected by dependency behavior, check the
involved dependencies against latest and bump first, before building
workarounds against old versions. (Policy set 2026-07-15.)

**`wasm-bindgen` is pinned** to `=0.2.121` in `rust/web/Cargo.toml` to match
the `wasm-bindgen-cli` version provided by nixpkgs, which also drives the
pins in `rust/Dockerfile` and `.github/workflows/ci.yml`. Do not update any
one of these without updating the other two and the devenv shell in
lockstep. A version mismatch between the CLI and the crate causes the WASM
build to fail at link time.

**Other pinned-by-ecosystem crates** (as of 2026-07-15): `js-sys`/`web-sys`
are held at `0.3.98` by the `wasm-bindgen =0.2.121` pin above (latest is
`0.3.103`). `web`'s `sqlx` is on `0.8` (latest `0.9`) and `tower-sessions` is
on `0.14.0` (latest `0.15`) because `tower-sessions-sqlx-store 0.15.0`
requires `sqlx ^0.8` and `tower-sessions ^0.14`; `bot` and `operator` are
already on `sqlx 0.9`. The `sqlx-cli` pin in `rust/Dockerfile` (`0.8.6`) is
kept matching `web`'s `sqlx 0.8`. When `tower-sessions-sqlx-store` releases
`sqlx-0.9` support, move `web`'s `sqlx`, `tower-sessions`, and the
Dockerfile `sqlx-cli` pin together. Run `cargo update --verbose`
periodically to check for patch-level updates; ignore "Unchanged" lines
where a newer major version exists but the Cargo.toml constraint
intentionally excludes it.

**`rust/web/end2end`'s `@types/node`** tracks the Node.js major provided by
the devenv shell (currently 24), not npm latest.

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

**`rust/rust-toolchain.toml` and `devenv.nix`'s Rust channel must be kept in
sync.** `rust-toolchain.toml` pins an explicit rustc channel; CI's
`dtolnay/rust-toolchain@stable` step is overridden by this file the moment
`cargo` runs inside `rust/`, so the toolchain file - not the CI action - is
the real source of truth for `rustfmt`/`rustc` version. If it drifts from
what `devenv.nix` (`languages.rust.channel = "stable"`) actually resolves to
locally, `cargo fmt --all -- --check` can flip-flop between passing locally
and failing in CI (different rustfmt versions format some constructs
differently). When either file's version changes, update the other to match
in the same change.

**Docker builder and runtime stages must be on the same Debian release.**
`rust/Dockerfile`'s builder stage pins an explicit `cargo-chef` tag
(`lukemathwalker/cargo-chef:X-rust-Y-bookworm`) rather than a floating tag
like `latest-rust-1`. A floating tag drifted to Debian 13/trixie while the
runtime stages are `debian:bookworm-slim` (Debian 12); the resulting binary
linked `GLIBC_2.38` symbols the runtime image didn't have, and `web`
crash-looped in production while other binaries in the same image happened
not to trip it. If a pod shows a GLIBC version error, check builder/runtime
Debian alignment first.

**`async-nats` buffers publishes.** The background flush task can delay
delivery under load, which is invisible in local testing but shows up as
flaky "did the subscriber see this yet" tests and slow WS delivery in prod.
Always call `.flush().await` after `.publish()` when timely delivery matters
(see `GameBroadcaster::broadcast_game_update` in `rust/web/src/websocket.rs`)
- log flush errors, don't propagate them.

---

## Database

**`games.updated_at` is trigger-maintained, not application-maintained.** A
`BEFORE UPDATE` trigger (`update_games_updated_at`, defined via
`CREATE OR REPLACE TRIGGER` in `rust/web/migrations/001_initial_schema.sql` -
easy to miss when grepping for `CREATE TRIGGER`) overwrites `updated_at =
now()` on every `UPDATE`, regardless of the `SET` clause. Consequences:

- Any code path that `UPDATE`s a `games` row bumps `updated_at` implicitly -
  relevant wherever recency ordering depends on it (e.g. sidebar sorting).
- Tests that need to backdate `updated_at` must first run
  `ALTER TABLE games DISABLE TRIGGER update_games_updated_at` (safe inside a
  `#[sqlx::test]` per-test database).

**One-off data migrations: sanitize + numeric suffix beats generating fancy
names.** `009_username_rules.sql` backfills any `users.name` violating the D2
charset by regex-stripping disallowed characters, truncating to 16 chars, and
falling back to `'player'` if that leaves it empty; case-insensitive
duplicates keep the name on the earliest-created holder (tie-broken by id)
and get a random 4-digit numeric suffix appended on the others. Simpler and
just as safe for a one-off backfill than generating unique replacement names
from word lists.

**A unique index created at the end of the migration is the safety net for
the backfill.** `009_username_rules.sql`'s `CREATE UNIQUE INDEX
users_name_lower_key ON users (lower(name))` runs only after every row has
been sanitized and deduplicated above it - if the backfill logic missed a
case, index creation itself fails the migration loudly rather than letting a
duplicate slip through silently.

**Never route a NOT NULL column through NULL mid-migration.** NOT NULL is
enforced per-row immediately (unlike unique constraints, which can be checked
at index creation), so an `UPDATE ... SET col = NULLIF(...)` followed by a
second `UPDATE ... WHERE col IS NULL` cleanup aborts on the first row that
hits NULL and the cleanup never runs. Collapse the fallback into the same
statement with `CASE`/`COALESCE` (see the sanitize step of
`009_username_rules.sql`, which was originally written the broken way and
caught in review).

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
ASCII approximation. See `docs/authoring/RULES_AUTHORING.md` for the
extraction process.

**V2 game interface pattern.** Games implementing the V2 interface expose
three additional endpoints beyond V1 (Rules, PlayerCounts, New, Status,
Play): `DataDocs` (field dictionary for the structured YAML states),
`BasicStrategy` (hard rules against dumb moves - never do X), and
`AdvancedStrategy` (optimal play guidance). The `Gamer` trait provides
default empty implementations so V1 games compile unchanged; V2 games
override them.

**Per-game doc structure (V2).** Each V2 game directory contains:
- `RULES.md` - pure game rules (no strategy, no render explanation)
- `DATA_DOCS.md` - field dictionary describing every field in
  `pub_state` and `player_state` YAML (what bots receive)
- `BASIC_STRATEGY.md` - hard rules against obviously bad moves
  (e.g. "never discard a winning card")
- `ADVANCED_STRATEGY.md` - optimal play guidance (e.g. "prioritize
  science sets over military in age 1")

**Strategy doc conventions.** BASIC_STRATEGY is a short list of
absolute don'ts - moves that are almost always wrong regardless of
context. ADVANCED_STRATEGY is longer, contextual, and describes
heuristics for strong play. Both are embedded at compile time via
`include_str!` like RULES.md. Bots receive BASIC at all difficulty
levels; ADVANCED only when the bot config includes it.

**Game interface versioning.** The `interface_version` column in
`game_versions` (set by the operator from the GameVersion CRD's
`interfaceVersion` field) records whether a deployed game speaks V1 or
V2. The shared `game_client` crate abstracts this: `fetch_game_data()`
calls Status plus the V2 endpoints and returns placeholder empty
strings for V1 games. Callers (bot, web) never check versions
themselves.

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
`docs/superpowers/specs/2026-07-04-11-testing-foundation-design.md` for the
current deferral).

**Use `#[sqlx::test]` for anything touching the database.** It gives each
test its own isolated, migrated database, so tests never share state with
each other. Do not build ad hoc shared fixtures or rely on test ordering;
each `#[sqlx::test]` function should set up exactly the rows it needs.

**Two-layer frontend/page testing: prefer the in-process layer.** Page-level
coverage is split into two layers (see
`docs/superpowers/specs/2026-07-04-11-testing-foundation-design.md` 11.6):

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

**Don't assert turn order by comparing player-index equality in games where a
turn can cascade back to the same player** (e.g. a bust auto-advances to the
next player, who can also immediately bust). `assert_ne!(current,
g.current_player)` is inherently flaky in that shape - a same-player
bounce-back is a legal outcome, not a bug. Assert on the emitted log content
instead (e.g. that a "it is now X's turn" log names the *other* player),
which holds regardless of how many players the turn cascades through.
