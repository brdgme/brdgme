# Proposed `docs/CODING.md` amendment - NOT APPLIED

**Why this is a proposal and not an edit.** `docs/CODING.md` is outside
`docs/reviews/2026-07-23-rust-review/planning/`, which is the only directory
this unit may write to. Apply the section below verbatim once that constraint
is relaxed. Re-read `docs/CODING.md` first - it is edited often.

## Insertion point

Insert as a new top-level section **between** the existing `## Rust: Error
Handling` section (whose final paragraph begins `**DOM access in event
handlers.**`) and the existing `## Leptos: SSR and Hydration` heading. Keep the
`---` separator that already sits between those two, put the new section under
it, and add a fresh `---` line below the new section.

Rationale: rule 2 extends the existing "No panicking code in runtime paths"
rule to the game services and shared libs, so it must sit adjacent to it.

## Proposed new section (verbatim)

~~~markdown
## Request-Path Invariants

Six invariants that every request-reachable code path must hold. Each one is
the root cause of a critical or major finding in the 2026-07-23 Rust review;
they recur across `lib/`, `game/*` and `web/`, so they live here rather than in
any one section.

**Never mix char counts with byte indices.** `s.chars().count()` counts
characters and `s[a..b]` slices bytes, so any value computed in one unit and
used in the other panics on the first multi-byte input - and iOS autocorrect
inserts a 2-byte NBSP for a plain space.

```rust
// Wrong: Space::parse in rust/lib/game/src/command/parser/mod.rs - char count
// used as a byte index; "\u{a0}x" panics the game service and the WASM suggest
let consumed = input.chars().take_while(|c| c.is_whitespace()).count();

// Correct: byte length of the same whitespace run, always on a char boundary
let consumed = input.len() - input.trim_start().len();
```

**No panics anywhere a request can reach, in any crate.** The Error Handling
rule above is not `rust/web`-only: a `.unwrap()` in a game service or a shared
lib kills that service's request just as dead.

```rust
// Wrong: the warp handler in rust/lib/cmd/src/http.rs - malformed game JSON
// from the wire panics the running game service
let request = serde_json::from_slice(&body).unwrap();

// Correct: propagate as a response
let request = serde_json::from_slice(&body)
    .map_err(|e| warp::reject::custom(BadRequest(e.to_string())))?;
```

**Every read endpoint gates on visibility; a predicate with no callers is a
bug.** `game_visibility` and `db::is_game_visible_to_user` both existed while
no read path called either, so the setting was decorative and any authenticated
user could read any game.

```rust
// Wrong: get_game_details in rust/web/src/game/server_fns.rs - a non-player
// falls through to the full spectator render
let player = find_player(&pool, game_id, user.id).await?;

// Correct: non-players must pass the predicate
if player.is_none() && !crate::db::is_game_visible_to_user(&pool, game_id, user.id).await? {
    return Err(ServerFnError::new("Game not found"));
}
```

**Mutate state only under a claim that re-checks the precondition inside the
transaction.** A precondition read from a snapshot outside the transaction is a
TOCTOU window: `undo_game` and `concede_game` both checked `is_finished` on a
stale pool read and then overwrote a concurrent move or a real result.

```rust
// Wrong: server_fns.rs reads ge.game.is_finished, then db::undo_game
// UPDATEs with no is_finished and no updated_at guard

// Correct: db.rs claim helper, first statement after pool.begin()
// SELECT is_finished, updated_at FROM games WHERE id = $1 FOR UPDATE
//   no row -> "Game not found"
//   is_finished -> GameAlreadyFinished
//   updated_at != expected -> StaleStateConflict
```

**Mark work done only after it succeeded, and fail loudly so the sender
retries.** Marking before doing turns every transient failure into permanent
silent data loss; a rare duplicate is the cheaper failure mode.

```rust
// Wrong: resend_webhook in rust/web/src/email/inbound.rs - marks before
// processing and always 200s, so a failed move reply is dropped forever
mark_event_processed(&pool, &event_id).await?;
handle_game_reply(...).await;
StatusCode::OK

// Correct: read-only dupe check, dispatch, mark on success only
if event_already_processed(&pool, &event_id).await? { return StatusCode::OK; }
match dispatch(...).await {
    RouteOutcome::Done => { mark_event_processed(&pool, &event_id).await.ok(); StatusCode::OK }
    RouteOutcome::Retry => StatusCode::INTERNAL_SERVER_ERROR,
}
```

Retrying re-runs the first attempt, and game commands are not idempotent, so
only failures occurring **before** any state mutation may return 5xx.

**Deserialized state and wire-supplied indices are untrusted; bounds-check at
the boundary, once.** Indices arriving in a request envelope are forwarded
verbatim into game state today, and fixing ~15 game crates one at a time
protects none of the crates not yet written.

```rust
// Wrong: handle_player_render in rust/lib/cmd/src/requester/gamer.rs forwards
// the wire index straight in; lost-cities-1/2 player_state does self.hands[player]
game.player_state(player)

// Correct: check at the single boundary the wire crosses
if player >= game.player_count() as usize {
    return Err(...);
}
```

A defaulted `Gamer::validate(&self) -> Result<(), GameError>` runs after
deserialization for games with cross-field invariants; per-crate index checks
remain as defence in depth, not as the primary guard.
~~~
