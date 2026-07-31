# Unit 01b - lib/color, lib/cmd, lib/game_client, lib/rand_bot

Continuation of `01-core-libraries.md`, covering the three commits that report
listed as "not reviewed - needs a follow-up sub-unit (01b)", excluding
WP-08/08b (epilogue dedup), which is sub-unit 01c.

Adversarial quality/correctness review of the remediation itself. Method: read
each commit's diff, then read the resulting code in its final form.

## Commits in scope

| hash | WP | subject | diff lines |
|---|---|---|---|
| `4a978cbe` | WP-05 | lib/color dead-API delete | ~2,749 |
| `a543120f` | WP-06 | lib/cmd HTTP + CLI hardening | ~914 |
| `63063a4b` | WP-07 | game_client / rand_bot | ~1,234 |

## Files reviewed

Final form (HEAD) read in full: `rust/lib/color/src/lib.rs`,
`rust/lib/color/src/css.rs`, `rust/lib/color/src/error.rs`,
`rust/lib/color/Cargo.toml`, `rust/lib/cmd/src/lib.rs`,
`rust/lib/cmd/src/repl.rs`, `rust/lib/cmd/src/cli.rs`,
`rust/lib/cmd/src/http.rs`, `rust/lib/cmd/src/requester/gamer.rs`,
`rust/lib/game_client/src/lib.rs`, `rust/lib/rand_bot/src/lib.rs`,
`rust/lib/markup/src/transform.rs:1-50`.

Read in part: `rust/lib/color/src/palette.rs` (the theme constants and
`NamedColor`/`Palette` impls; the `mix`/`soften`/`contrast` bodies and the
2,000-line `tests::gate*` block were sampled via a delegated extraction, not
read line by line - see Coverage gaps).

Cross-checked: `docs/CODING.md` (General Principles, Error Handling,
Request-Path Invariants).

## Findings

### F-17 (High) The deferred "CLI REPL will panic" fix did not land - `repl.rs` is still panic-driven throughout

`rust/lib/cmd/src/repl.rs` - final state at HEAD

WP-02 (`91f26820`) explicitly deferred the "CLI REPL will panic" fix to WP-06
(`a543120f`). WP-06 **did** touch `repl.rs`, and it did harden `lib/cmd`'s HTTP
and `cli.rs` paths (see Verified good) and add EOF/blank-stdin handling to
`prompt` (`:259-270`, now returning `Option<String>` so Ctrl-D exits instead of
looping) - so this is not a missed file, it is a partial fix that stopped short
of the panic class the deferral named. Every markup and I/O path in the REPL
still unwraps:

| line | construct |
|---|---|
| `:210` | `brdgme_markup::from_string(&l.content).unwrap()` in `output_logs` |
| `:252` | `brdgme_markup::from_string(markup).unwrap().0` in `output_markup` |
| `:44`, `:145`, `:192` | `client.request(..).unwrap()` |
| `:55`, `:57`, `:169`, `:175`, `:203`, `:204` | `panic!(..)` on `UserError`/`SystemError`/unexpected response |
| `:107` | `serde_json::ser::to_string_pretty(&game).unwrap()` |
| `:109`-`:120` | four `expect(..)` on `:save`/`:load` file I/O and JSON |
| `:264` | `expect("failed to flush stdout")` |

`:210` and `:252` are the exact deferred bug: the REPL takes markup produced by
the game service and `unwrap()`s the parse. WP-02 made this *strictly more
likely* to fire, because it changed `from_string` to reject any unconsumed input
(see `01-core-libraries.md` F-01) - markup that previously parsed partially now
returns `Err`, and the REPL's response to `Err` is to abort. So the two commits
together moved the failure mode from "renders partial output" to "panics", which
is the opposite of the deferral's intent.

`:55`/`:169` are also plainly wrong independent of markup: a `Response::UserError`
from the game service (e.g. `check_player`'s "invalid player N", or any invalid
command at game start) is a *normal* outcome, and at `:54-56` the REPL panics on
it. `:169` panics on `SystemError` mid-game, discarding the whole session
including the `:save` escape hatch, where printing the message and continuing
the loop is both easier and obviously better.

`docs/CODING.md`'s "Acceptable panics" list covers process startup, `cfg(test)`,
and `unreachable!()` in client stubs. An interactive dev tool's steady-state
loop is none of those, and `tools/repl` is the primary way game authors exercise
a new crate - a panic there is a stack trace instead of an error message, on the
one surface whose entire job is diagnostics.

Remediation: make `output_markup`/`output_logs` fall back to printing the raw
string (or an inline error marker) on `Err` rather than unwrapping - the REPL has
no reason to die because one log line is malformed. Turn the `UserError` arms at
`:54` and `:167` into `output_error(message); continue;` (the pattern already
used at `:171-174`). Give `repl` a `Result<(), ...>` return (or an internal
`fn run() -> Result<..>`) so `client.request` and the `:save`/`:load` handlers
propagate with `?` and the `main` in `tools/repl` prints the error and exits
non-zero. If a subset of these is genuinely intended to stay a panic, say so in
a comment and in the WP's decision record - right now nothing distinguishes
"deliberate" from "not done".

### F-16 (Low) Three request variants silently discard their payload

`rust/lib/cmd/src/requester/gamer.rs:91-93`

```rust
Request::DataDocs { .. } => Ok(handle_data_docs::<G>()),
Request::BasicStrategy { .. } => Ok(handle_basic_strategy::<G>()),
Request::AdvancedStrategy { .. } => Ok(handle_advanced_strategy::<G>()),
```

All three carry a `game` string (and `BasicStrategy`/`AdvancedStrategy` a
`player`) that the handlers ignore entirely, returning a static per-crate
string. Consequently these are the only state-carrying requests that never
deserialise or `validate()` the state, and `check_player` never runs, so a
caller passing a nonsense `player` gets a success response. WP-09a/09b's
requester-boundary hardening (`validate()` + `check_player` on every state-
carrying variant) therefore has a documented-looking hole that is invisible at
the call site: `game_client::fetch_game_data` (`rust/lib/game_client/src/lib.rs:341-393`)
faithfully sends `game` and `player` on all three, so a reader reasonably
believes they are per-state and per-player.

Not a live exploit (nothing is indexed), but the API shape misrepresents the
behaviour and it is the kind of gap the review's D-36 boundary work was meant to
close uniformly.

Remediation: either drop the unused `game`/`player` fields from these three
`Request` variants (and from `game_client`'s senders), or deserialise +
`validate()` + `check_player` like every sibling arm, so the boundary rule has
no exceptions.

### F-15 (Medium) `IN_USE_SOFTENS` is an unenforced whitelist coupled to arbitrary game markup

`rust/lib/color/src/css.rs:13-28` vs `rust/lib/markup/src/html_class.rs:64-79`
and `rust/lib/markup/src/transform.rs:21-32`

`palette_css_vars` emits CSS custom properties only for the three pairs listed
in `IN_USE_SOFTENS` and for `IN_USE_MIXES` (empty). But markup carries an
arbitrary pct: `ColType::Named { color, soften: Some(pct) }` and
`ColType::Mix { source, target, pct }` are parsed from a game's own markup
string and resolved by `brdgme_color::soften`/`mix` with no reference to either
whitelist. The two representations are only kept in agreement by the doc
comment's claim that the set was "audited from acquire-1 and lords-of-vegas-1
`render.rs`".

The HTML path makes the coupling concrete: `color_token`
(`html_class.rs:64-79`) formats the class name straight from the parsed pct -
`format!("soften-{}-{}", color, pct)` / `format!("mix-{}-{}-{}", ...)` - while
`markup_class_css()` only emits a rule for the whitelisted pcts.

Nothing enforces the subset relation: `in_use_softens_matches_palette_css_vars`
(`css.rs:104-110`) only asserts that `palette_css_vars` emits a variable for
each whitelist entry - the converse (every pct a game emits is whitelisted) is
untested. A game author who writes `soften(foreground, 75)` gets a correct
ANSI/terminal render and an HTML render emitting
`class="mk-fg-soften-foreground-75"` with no matching CSS rule and no defined
`--mk-soften-foreground-75`, with no compile error and no failing test. The
same commit's doc comment records that 75/78/86 call sites were *rewritten* to
80/90 to shrink this list, which is exactly the maintenance burden an
unenforced whitelist creates.

Confirmed by sweep: **no live violation exists today** - the only soften pairs
any `rust/game/*/src/` file emits are `(Foreground, 90)`, `(Foreground, 80)` and
`(Pink, 80)` (`acquire-1/src/render.rs:20`, `:166`,
`lords-of-vegas-1/src/render.rs:212`), exactly the whitelist, and no game emits a
`Mix` at all. This is a latent-defect / maintainability finding, not a live bug.

Remediation: add a test that walks every `rust/game/*/src/render.rs` soften/mix
literal (or, better, move the whitelist to a build-time/`include!`-generated
set), or make the web layer emit the variable on demand from the parsed markup
rather than from a hand-maintained const. At minimum add a test asserting the
game-emitted set equals `IN_USE_SOFTENS`, so the next divergence fails CI
rather than rendering wrong.

### F-09 (High) `rand_bot` still panics on two degenerate specs - the exact class WP-07 claimed to fix

`rust/lib/rand_bot/src/lib.rs:33` and `:13` (reached from `:70-72`)

WP-07 fixed the degenerate-spec panics for `Spec::OneOf(vec![])`,
`Spec::Enum { values: [] }` and `Spec::Player` with no players (all now return
an empty token vector, with tests). Two panics of the same class survive:

```rust
command::Spec::Int { min, max } => {
    if min.is_some() && max.is_some() && min > max {
        panic!("invalid Int spec\nSpec: {:?}\nContext: {:?}", spec, ctx)
    }
```

```rust
fn bounded_i32(v: i32, min: i32, max: i32) -> i32 {
    assert!(min <= max);
```

and in the `Many` arm:

```rust
let min = min.unwrap_or(0) as i32;
let max = max.unwrap_or(3) as i32;
let n = bounded_i32(rng.random(), min, max);
```

So any `Spec::Many { min: Some(5), max: None }` (or any `max < min`) hits
`assert!(min <= max)` and aborts, because the `None` default for `max` is the
magic literal `3`, not "unbounded" and not `min`. And an inverted
`Spec::Int` is an outright `panic!`. `command::Spec` is deserialised from
whatever the game service returns, so both are wire-reachable, and
`docs/CODING.md`'s "No panics anywhere a request can reach, in any crate"
applies verbatim. Note that `lib/game`'s own `Many` combinators were changed
in WP-03/WP-04 to degrade gracefully on `max < min` rather than panic - so the
library and the bot now disagree about the same malformed spec.

Remediation: treat both as degenerate and return no tokens (the pattern WP-07
already chose for `OneOf`/`Enum`/`Player`) - `if min > max { return vec![] }`
for `Int`, and `let max = max.unwrap_or(3).max(min)` (or an early `vec![]`)
before calling `bounded_i32`. Drop `assert!` from `bounded_i32` and make the
function total, or make it return `Option<i32>`. Add tests for
`Int { min: Some(5), max: Some(1) }` and `Many { min: Some(5), max: None }`.

### F-10 (Medium) `Many`'s `min`/`max` are `usize` cast with `as i32`

`rust/lib/rand_bot/src/lib.rs:70-71`

`min.unwrap_or(0) as i32` / `max.unwrap_or(3) as i32` silently wrap for any
value above `i32::MAX`. Combined with F-09 this turns a large `min` into a
negative one, which then *passes* `assert!(min <= max)` and produces `n < 0`,
so the `for i in 0..n` loop is skipped and the bot emits a command that
violates the spec's own minimum. `as` casts on deserialised values are exactly
what the review's char/byte class warned about in a different guise.

Remediation: `i32::try_from(...).unwrap_or(i32::MAX)` (or clamp with
`min(i32::MAX as usize)`), and derive `max` from `min` as in F-09.

### F-11 (Medium) `Response::UserError` is silently downgraded to `UnexpectedResponse`

`rust/lib/game_client/src/lib.rs:192-195`

```rust
match resp {
    Response::SystemError { message } => Err(GameClientError::SystemError { message }),
    other => Ok(other),
}
```

`Response::UserError { message }` is not handled. It flows out as `Ok(other)`
and every typed helper (`pub_render`, `player_render`, `fetch_game_data`'s five
match arms) then falls into its `_ =>` arm and returns
`GameClientError::UnexpectedResponse { request: "PubRender" }`. The
service-supplied message is discarded, so an operator sees "unexpected response
to PubRender request" for what is actually a diagnosable user error. WP-07's
stated goal was replacing anyhow strings so "callers can branch on kind" -
`UserError` is precisely a kind a caller wants to branch on, and there is no
variant for it.

Remediation: add `GameClientError::UserError { message }` and map it in
`request_with_config` alongside `SystemError`, so no caller can mistake it for
a protocol violation.

### F-12 (Medium) The per-attempt timeout ceiling is not a request ceiling

`rust/lib/game_client/src/lib.rs:57-65`, `:119-169`, `:181-186`

The doc comment claims the ceiling "composes - the tighter of the two always
wins". That is true per attempt but not per call. With the defaults
(`max_attempts: 3`, `request_timeout: 90s`) the worst case is three 90s send
attempts plus backoff plus a *fourth, separate* 90s ceiling on the body read
(`:181`), i.e. ~6 minutes - and for `web`, whose `reqwest` client timeout is
10s, ls F31's "no bound" became "3x the bound the caller configured", because
`e.is_timeout()` is classified retryable. `fetch_game_data` then issues up to
four such calls (one sequential `Status`, three joined), so a caller's
effective ceiling is a multiple of what it configured. The finding ls F31 asked
for a bound; a bound exists, but it is not the one the comment describes and it
is not the one the caller set.

Remediation: track a single deadline for the whole `request_with_config` call
(`tokio::time::Instant::now() + total_budget`) and pass the remaining budget to
each attempt and to the body read, rather than restarting a fresh
`request_timeout` per phase. At minimum, correct the doc comment to state the
worst case is `max_attempts * request_timeout + backoff + body read`.

### F-13 (Low) `validate_version_name` accepts uppercase and leading digits

`rust/lib/game_client/src/lib.rs:104-117`

The error message and doc comment both say "must be a DNS label", but the check
is `is_ascii_alphanumeric() || b'-'`, which admits `Acquire-1` and `1acquire`.
The value is interpolated into a `Host` header the KEDA interceptor routes on by
string match; an uppercase name would be sent verbatim and silently 404 rather
than being rejected up front with the clear error this function exists to give.

Remediation: require `b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'`
and a non-digit first byte, matching RFC 1123 / k8s object-name rules, and add
`"Acquire-1"` to the rejection test's table.

### F-14 (Low) A known-flaky timing race was accepted rather than removed

`rust/lib/game_client/src/lib.rs:435-442`
(`test_retry_on_connect_refused_then_success`)

The test binds a port, drops the listener, and races a 15ms `sleep` against a
20-40ms jittered backoff. The comment records this as "Known race, accepted
(ls F37)" and even spells out the deterministic fix ("bind the replacement
listener on a second port first"). The deterministic fix is two lines; recording
a flake as accepted is a shortcut, and the surrounding tests
(`test_bounded_max_attempts_on_permanent_failure`,
`test_crate_timeout_bounds_client_without_timeout`) already show the
counter-based deterministic pattern that would remove the race entirely.

Remediation: apply the fix the comment describes, or assert on the attempt
counter (as the sibling tests do) instead of on wall-clock elapsed time.

## Verified good

Read in final form, not just diff-read.

**WP-05 `4a978cbe` - the deletion is clean.** Exhaustive symbol sweep over the
commit's removed declarations, cross-checked against the whole working tree at
HEAD (`rust/**` plus non-Rust sources, excluding `docs/reviews/`):

| deleted symbol | verdict |
|---|---|
| `Color::from_hex` | no live reference (one prose mention in `docs/superpowers/plans/2026-07-14-mix-transform.md:210`, a historical plan doc) |
| `impl FromStr for Color` / `Color::from_str` | no live reference; no `parse::<Color>()` anywhere |
| `named` (free colour-name lookup fn) | no live reference; its only caller was the deleted `FromStr` impl |
| test fns `color_from_hex_works`, `color_from_str_named_works` | deleted with the API they covered, not weakened to pass |
| test-local `srgb_to_linear` and the nested `lin` in `rgb_to_lab` | consolidated into one shared linearizer; no dangling name |

No file was deleted (`git ls-tree -r` on `4a978cbe^` vs `4a978cbe` for
`rust/lib/color/` shows the same 6 paths). `ColorError::Parse`, the variant the
deleted parsers constructed, correctly survives because `impl FromStr for
NamedColor` (`rust/lib/color/src/palette.rs:61-83`) still uses it - the delete
was scoped to `Color`'s parsers, not the module. `lazy_static` and `regex` were
dropped from `rust/lib/color/Cargo.toml`, the workspace `Cargo.toml` and
`Cargo.lock`, and nothing hand-rolled replaced them (the API went away rather
than being reimplemented), so `docs/CODING.md`'s "No bespoke code - prefer an
existing, well-maintained crate over a hand-rolled implementation" is not
violated by the drop.

- **`Color::hex()` -> `self.to_string()`** (`rust/lib/color/src/lib.rs:45-47`)
  removes the duplicated `format!("#{:02x}{:02x}{:02x}")` and the new test
  `hex_equals_to_string` pins the two together. `mono()`'s `+ 1` rounding
  boundary is pinned at 127/128 by `mono_boundary`.
- **`palette_css_vars`** (`rust/lib/color/src/css.rs:39-76`) is a pure
  string builder with no indexing, no `unwrap`, and emits the `-contrast`
  counterpart for every token it defines. See F-15 for the whitelist coupling,
  which is a design issue, not a bug in this function.
- **`rust/lib/cmd/src/cli.rs:6-23`** is properly hardened: a malformed request
  JSON becomes `Response::SystemError` rather than a panic, and a `Requester`
  error is folded into `SystemError` via `unwrap_or_else`. The only remaining
  `expect`s are on `serde_json::to_string(&Response)` (infallible for this type)
  and the final `writeln!` to the process's own output.
- **`rust/lib/cmd/src/http.rs:26-49`** - `handle` has no panicking path:
  `headers` values use `to_str().unwrap_or_default()`, and `g.request(&req)` is
  folded with `unwrap_or_else` into `Response::SystemError`. `DefaultBodyLimit`
  caps the body at `MAX_CONTENT_LENGTH` (16 MiB) and
  `oversized_content_length_is_rejected` pins the 413. The three tests exercise
  the malformed-state path, the happy path, and the size cap - none of them is
  an `||`-style tautology.
- **`requester::gamer::request`** (`rust/lib/cmd/src/requester/gamer.rs:39-95`)
  applies `validate()` before any `Gamer` call on all four state-carrying
  variants, and `check_player` before `handle_play`/`handle_player_render`;
  `validate_error_returns_system_error_for_all_request_types` covers all four,
  and the `PanicGame` fixture (whose `player_state` panics out of range) is a
  genuine regression test - it would panic if `check_player` were removed,
  rather than merely returning a different error. See F-16 for the three
  variants that bypass the boundary.
- **`GameClientError`** (`rust/lib/game_client/src/lib.rs:11-47`) is a real
  typed-error replacement for the previous anyhow strings: every variant carries
  the data a caller needs to branch, `Transport`/`StateYaml` use `#[from]`,
  `ParseResponse` keeps the body for diagnosis, and there is no catch-all
  `Other(String)` escape hatch. See F-11 for the missing `UserError` variant.
- **`send_with_retry`'s retry classification** (`:148-168`) is correct on the
  point the review cared about: only transport failures
  (`is_connect`/`is_timeout`/`is_request`) and the crate ceiling are retried;
  any received HTTP response, including 5xx, returns immediately.
  `test_no_retry_on_http_error_response` asserts the attempt *counter* is
  exactly 1, not merely that an error came back, so it would catch a
  regression that started retrying 500s. `backoff_delay(attempt - 1, ...)`
  cannot underflow because `attempt` is incremented before every use, and the
  cap/jitter band is pinned by three separate deterministic tests.
- **`rand_bot`'s degenerate-spec fixes for `OneOf`/`Enum`/`Player`**
  (`rust/lib/rand_bot/src/lib.rs:45-52`, `:85-88`) return an empty token vector
  instead of panicking on an empty `choose`, with two tests. The `Space` arm
  emitting `" "` and joining with `""` is pinned by
  `space_tokens_join_without_double_spaces`. `bounded_i32`'s widening to `i64`
  before the modulo is correct: `max64 - min64 + 1` cannot overflow `i64` even
  for `i32::MIN..=i32::MAX`, and the negative-`v` branch adds a whole multiple
  of `range_size` so the result is always in range. See F-09/F-10 for the
  degenerate cases it still aborts on.

## Coverage gaps

Sub-unit 01b covered all three assigned commits (`4a978cbe`, `a543120f`,
`63063a4b`). What was *not* audited within them:

- **`rust/lib/color/src/palette.rs`'s `tests::gate*` block (~1,800 of the
  file's 2,385 lines).** WP-05 rewrote every `Palette` literal onto the new
  `rgb()` helper, which is mechanical, and I confirmed the surviving public API
  and the `NamedColor`/`Palette` accessors by hand. I did **not** verify the
  contrast-gate assertions themselves, in particular the claim in
  `css.rs:9-12` that `soften(foreground, 75)` measures 2.86:1 for Solarized
  Dark while 80 clears 3:1. If that number is wrong, the 75->80 rewrite of
  `acquire-1`/`lords-of-vegas-1` was an unnecessary visual change to two games.
  Re-deriving it needs the `mix`/`soften`/`contrast_ratio` bodies plus the gate
  test, which did not fit this budget.
- **The `mix`/`soften`/`contrast`/`contrast_ratio` implementations.** WP-05 did
  not change them (they predate it), so they are out of this commit's scope, but
  F-15's whitelist argument assumes they are pure functions of
  `(color, pct, background)` - which the call sites are consistent with but
  which I did not prove from the bodies.
- **`rust/web/src/theme.rs`.** It is the only production consumer of
  `IN_USE_SOFTENS`/`IN_USE_MIXES` and therefore where F-15 actually bites, but
  it belongs to Unit 09 (web frontend). Carry F-15 into that unit's brief so the
  undefined-CSS-variable question is settled against the real emitter.
- **`rust/lib/cmd/src/test_support.rs`.** The delegated sweep counted **37**
  non-test panic constructs under `rust/lib/cmd/src/` at HEAD: `repl.rs` 18 (all
  of F-17), `test_support.rs` 14, `http.rs` 3, `cli.rs` 2, zero elsewhere. Eight
  (cli.rs's 2 and 6 in repl.rs) were added or reworded by `a543120f`; the rest
  predate it. The 14 in `test_support.rs` are **not** covered by this report:
  `a543120f` did not touch that file, and although it is gated behind the
  `test-support` feature rather than `#[cfg(test)]` - so it compiles into any
  dependent that enables the feature - deciding whether those panics are
  acceptable needs the list of crates enabling it, which I did not enumerate.
  `api.rs`, `bot_cli.rs` and `requester/{mod,error}.rs` are panic-free but were
  not line-read for logic.
- **`http.rs` is axum, not warp, in its final form.** WP-71 (`dcec1adf`, Unit
  10) replaced warp after WP-06 landed, so the malformed-*envelope* case is now
  handled by axum's `Json` rejection - a 400 with a plain-text body, not a
  `Response::SystemError` JSON. `game_client` turns that into
  `GameClientError::HttpStatus`, which is survivable, but no test covers it and
  the behaviour differs from what WP-06's test
  (`malformed_game_json_returns_system_error_not_panic`, which exercises a
  malformed *inner* state string) implies. Flagging for Unit 10 rather than
  scoring it against WP-06.
- **Callers of `game_client`.** F-12's severity depends on the `reqwest` client
  timeouts the doc comment attributes to web (10s) and bot (60s); those live in
  `rust/web` and `rust/bot` (Units 05/09/10) and I did not confirm them.
- **`rand_bot`'s `fuzz`/`Fuzzer` path** (`rust/lib/rand_bot/src/lib.rs:134-140`,
  `brdgme_game::bot::Fuzzer`) was not reviewed; F-09's panics are reachable from
  it too, and the fuzz tool's own hang fix is WP-63 in Unit 10.
- **WP-05's decision D-39 (regex/lazy_static drop) cannot be cross-checked, and
  that is itself the gap.** The only surviving textual record of D-39 is the
  one-line entry in `docs/reviews/2026-07-23-rust-review/SUMMARY.md` plus the
  commit message's own "option A" gloss - i.e. the author's self-description.
  `docs/CODING.md`'s Dependency Management section contains no rule that either
  confirms or contradicts delete-rather-than-rewrite. The drop does satisfy the
  General Principles' "No bespoke code" (nothing was reimplemented), so there is
  no violation to report; but the breakdown's instruction to treat
  `docs/CODING.md` as ground truth for "was this fixed the way the project
  decided to fix it" cannot be discharged for D-39, because the project never
  wrote the decision down anywhere durable. If the deleted parse API is ever
  wanted back, nothing on disk explains why it went.

