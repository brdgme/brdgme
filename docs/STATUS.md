# Current Status

## Session: 2026-04-04 (continued)

### Completed this session

**Phase 5.6.1: active_games context removed**
- `App()` no longer creates `active_games` `LocalResource` or provides it via context
- `SidebarMenu` (`components/layout.rs`) now creates its own `LocalResource`, reading `WebSocketTrigger` from context to drive refetches
- Removed unused `get_active_games`/`GameSummary` imports from `app.rs`

**GamesPage resource type fixed**
- `game_types` changed from `Resource::new` to `LocalResource::new`
- `Suspense` wrapper removed; replaced with direct `match game_types.get()` with explicit `None => "Loading..."` arm
- Rationale: `Resource::new` in streaming SSR emits a `<!-- -->` placeholder in initial HTML; if WASM hydration runs before streaming JS fills the template, client finds `<!-- -->` where it expects content

**docs/CODING.md updated** with two new sections:
- `Resource::new` table row: explains streaming SSR placeholder mechanism
- "Resource read order inside Suspense closures": documents that `game_data.get()` must be read unconditionally before any short-circuit (ws_game check), otherwise Suspense never tracks the resource as a dependency
- "Suspense vs no Suspense": corrected from "Suspense + new_blocking is safe" to the actual behaviour discovered this session (see below)

---

### Hydration investigation: key discoveries

**Problem:** Hard refresh on game page produced:
`A hydration error at layout.rs:16:10 — expected <div>, found <!-- -->`

**Root cause (multi-layer):**

1. **`Suspense` always introduces a streaming boundary**, even with `Resource::new_blocking`. The SSR HTML contains a streaming placeholder `<!-- -->` at the Suspense position. WASM hydration runs before (or independently of) the streaming JS that fills in the template, so the client finds `<!-- -->` where it expects `MainLayout`'s `<div class="layout">`.

2. **`Resource::new_blocking` without `Suspense`**: Leptos warns at runtime: "reading a resource in hydrate mode outside a Suspense or effect causes hydration mismatch errors." In hydrate mode (WASM), `game_data.get()` returns `None` initially (even for `new_blocking`), because resource deserialization is not synchronously available during the first reactive pass. This produced a NEW error at `game.rs:347` (expected `<div class="suggestions-container">`, found `<!-- -->`): SSR rendered full game content (is_my_turn=true → GameCommandInput rendered), but WASM rendered the `None` branch (`<MainLayout><div></div></MainLayout>`).

**Experiments performed:**
- Fix 1: Ensured `game_data.get()` is read before `ws_game.get()` in the Suspense closure → did NOT fix layout.rs:16 (Suspense streaming is the real cause, not resource tracking)
- Fix 2: Removed `Suspense` entirely → fixed layout.rs:16, introduced game.rs:347 (resource None in hydrate mode)

**Current state of GamePage:** No Suspense, direct `match effective` with `None => <MainLayout><div/></MainLayout>`. Still has game.rs:347 error because `game_data.get()` = None in hydrate mode.

---

### Immediate plan: fix remaining hydration error

**Problem:** `Resource::new_blocking` outside Suspense → `None` in hydrate mode → structural mismatch (game content missing on client but present in SSR HTML).

**Correct approach** (not yet implemented):
- `MainLayout` must be OUTSIDE `Suspense` (prevents layout.rs:16)
- Game content must be INSIDE `Suspense` (allows Suspense to defer hydration until resource deserializes, prevents game.rs:347)
- `is_my_turn`/`has_next_game`/`has_sub_menu` props on `MainLayout` must accept reactive types (`MaybeSignal<bool>`) so they update after resource deserializes

**Required changes:**

1. **`MainLayout` (`components/layout.rs`):**
   - Change `is_my_turn: bool`, `has_sub_menu: bool`, `has_next_game: bool` to `#[prop(into, default)] is_my_turn: MaybeSignal<bool>` etc.
   - `MaybeSignal` is in `reactive_graph::wrappers` — check if exported from `leptos::prelude::*` or needs explicit import from `reactive_graph::wrappers::MaybeSignal`
   - Change `class:my-turn=is_my_turn` to `class:my-turn=move || is_my_turn.get()`
   - `has_sub_menu` and `has_next_game` conditionals produce STRUCTURAL differences (`<input>` vs `<span>`). This causes hydration mismatch if value differs between SSR and client. Fix: always render `<input>` but use `style:visibility` or `hidden` attribute conditionally — CSS changes don't cause structural hydration errors, only element type/hierarchy does.

2. **`GamePage` (`app.rs`):**
   - Create `Memo<bool>` for `is_my_turn`: reads `game_data.get()` + `ws_game.get()`, defaults to `false` when `None`
   - Structure:
     ```rust
     view! {
         <MainLayout is_my_turn=is_my_turn has_sub_menu=true has_next_game=is_my_turn>
             <Suspense fallback=|| view! { <div></div> }>
                 {move || {
                     let base = game_data.get(); // read first for Suspense tracking
                     let effective = ws_game.get()
                         .filter(|ws| Some(ws.game_id) == game_id())
                         .map(|ws| Ok(ws.game_view))
                         .or(base);
                     effective.map(|res| match res {
                         Ok(data) => { game content (no MainLayout) }.into_any(),
                         Err(e) => { error div (no MainLayout) }.into_any(),
                     })
                 }}
             </Suspense>
         </MainLayout>
     }
     ```
   - `is_my_turn` Memo starts `false` in hydrate mode (class change only, not structural) → no hydration error
   - `Suspense` properly defers game content hydration → SSR and client both have game content → match ✓

**Why `has_sub_menu`/`has_next_game` structural change matters:**
- SSR (new_blocking resolved): `is_my_turn=true` → `has_next_game=true` → renders `<input type="button" value="Next game"/>`
- Client hydrate (Memo starts false): `has_next_game=false` → renders `<span></span>`
- Structural mismatch if these use `if X { <input> } else { <span> }` pattern
- Fix: replace with always-rendered-but-conditionally-visible `<input hidden=move || !has_next_game.get()/>` or `style:display`

**`MaybeSignal` import:** `reactive_graph::wrappers` has `MaybeSignal<T>`. `leptos::prelude::*` re-exports `reactive_graph::prelude::*` but does NOT appear to re-export `wrappers`. May need explicit: `use reactive_graph::wrappers::read::MaybeSignal;` in `layout.rs`. Confirm by compiling.

---

### Known open issues (unchanged from previous session)

- **Runtime panics in `rust/web`** (Phase 5.7): 4 cases — `db.rs:407`, co-nullable LEFT JOIN unwraps, `NodeRef::get().unwrap()` in `app.rs:121,128`, `websocket_client.rs:21-23`
- **Restart 500 error**: root cause unknown
- **Bot restart limitation**: bots not recreated in restarted game
- **Optimistic locking**: race condition in `execute_command`
- **3-player render**: placeholder in `lost-cities-2/RULES.md`
- **NATS bot eventing** (Phase 9): not yet started

### Next steps (in order)

1. **Fix remaining hydration error** (see plan above) — implement `MaybeSignal<bool>` on `MainLayout`, restructure `GamePage` with `MainLayout` outside `Suspense`
2. **Phase 5.7** — fix runtime panics
3. **3-player render** — create Lost Cities 3-player game, extract render
4. **Phase 9** — NATS bot eventing
5. **Phase 6.5** — ArgoCD CD
6. **Phase 7** — legacy decommission
