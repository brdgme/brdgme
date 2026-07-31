# Unit 08 - Web domain: stats / query performance

Scope: WP-52, commit `f374434d` (95 files, 220 insertions / 2947 deletions).
Findings numbered from **F-150**.

## Sizing note (important for the unified report)

`f374434d` is advertised as 95 files / 220+ / 2947-. **91 of the 95 files are
`.sqlx` offline query-cache JSON.** The commit deletes 82 stale entries from
`rust/.sqlx/` (a workspace-root cache dir that WP-66's sqlx unification left
behind), deletes/renames a handful under `rust/web/.sqlx/`, and adds one. Only
**9 files are Rust source, +162 / -80 total**:

| Source file | Ins | Del |
|---|---|---|
| `rust/web/src/stats/queries.rs` | +58 | -13 |
| `rust/web/src/index.rs` | +25 | -14 |
| `rust/web/src/db/social.rs` | +23 | -0 |
| `rust/web/src/stats/mod.rs` | +21 | -7 |
| `rust/web/src/friends.rs` | +9 | -18 |
| `rust/web/src/game_info/mod.rs` | +9 | -15 |
| `rust/web/src/db/common.rs` | +8 | -2 |
| `rust/web/src/game/server_fns.rs` | +8 | -10 |
| `rust/web/src/players.rs` | +1 | -1 |

The breakdown's "mostly-deletions, likely consolidating duplicated query/stats
code" framing is **wrong**: this is not a consolidation of duplicated logic, it
is a build-artifact cleanup with a small behavioural change riding along. The
deletion-risk (pattern 4e) surface is therefore near zero - **no test file and no
guard was deleted by this commit; it touches no test file at all.** That last
fact is itself the unit's headline finding (see F-150).

## Acceptance criteria

**There is no WP-52 spec.** Confirmed by listing
`868094a6:docs/reviews/2026-07-23-rust-review/planning/specs/` (50 entries, plus
`archive/`): no `WP-52-*.md`. `planning/archive/specs-LOG.md` explains why - the
spec-writing Lead was briefed to author WP-51, WP-60, WP-52 in that order and
the unit ended after WP-60. The authoritative acceptance criteria are the 13 rows
of `planning/checklists/T3-B5-web-domain-stats-misc.md`.

**`00-STATE.md`'s no-spec list should be extended: WP-52 joins WP-24, WP-27,
WP-44, WP-53 and WP-79.**

The 13 rows, with the checklist's own `Test?` column:

| Row | Target | Test? |
|---|---|---|
| `wd F50` | `queries.rs` - hoist single-human eligibility subquery to one `const`, 8 sites | **y** |
| `wd F51` | `queries.rs::game_history` - 4 correlated subqueries -> `LEFT JOIN LATERAL` | **y** |
| `wd F47` | `queries.rs::rating_series` - shared `const INITIAL_RATING` | n |
| `wd F55` | `queries.rs::finished_games`,`recent_form` - `NULLS LAST` on `finished_at DESC` | **y** |
| `wd F53` | `queries.rs` - 3 `query_as` -> `query!`, or comment | n |
| `wd F48` | `stats/mod.rs::get_player_game_type_stats` + `queries.rs::game_type_stats` - nullable name filter | **y** |
| `wd F49` | `stats/mod.rs::get_player_game_type_stats` - cap anonymous page payload | n |
| `wd F52` | `stats/mod.rs::get_player_history` - resolve `game_type` via `find_game_type_name` | **y** |
| `wd F46` | `stats/mod.rs::get_player_history` + `players.rs::PlayerHistoryPage` - clamp `page` | **y** |
| `wd F21` | `game/server_fns.rs::get_game_details` - batched `should_hide_add_friend_many` | **y** |
| `wd F62` | `friends.rs::get_friends_overview` - `try_join!` six queries | n |
| `wd F74` | `index.rs::get_logged_in_index` - bound `list_friends`, concurrent per-friend calls | n |
| `wd F75` | `game_info/mod.rs::get_game_info` - `try_join!` seven awaits | n |

## Findings

### F-150 (Medium) - all seven "Test? y" rows of WP-52 shipped with no test

`f374434d` touches **no test file and adds no `#[cfg(test)]` block** (confirmed
against the full 95-file list: the only non-`.sqlx` files are the 9 source files
above, none of which is a test module addition - pending W2 confirmation that no
inline `#[cfg(test)]` was added within them).

The T3-B5 checklist marks **7 of 13 WP-52 rows `Test? y`**: `wd F50`, `wd F51`,
`wd F55`, `wd F48`, `wd F52`, `wd F46`, `wd F21`. Four of those seven are
behaviour-changing in ways a reader cannot verify from the diff:

- `wd F55` changes **row ordering** (`NULLS LAST`).
- `wd F51` changes **how per-row aggregates are computed** (correlated subquery
  -> `LEFT JOIN LATERAL`), which is exactly the class of rewrite that silently
  changes row counts and NULL handling.
- `wd F48` changes **which rows a stats query returns** (adds a name filter).
- `wd F46` changes **pagination bounds**.

Why it matters: this is the fourth confirmed instance of the "Test? y with no
test" pattern (after F-142, F-148, F-149) and by far the largest - seven rows in
a single commit. It is also the highest-consequence instance, because unlike the
earlier three these rows change *query result semantics*, and the checklist's own
`Test? y` marks are the only place anyone recorded that a test was required.

Suggested fix: process-level. See the Coverage gaps section - the per-row
verification below establishes which of the seven actually changed behaviour and
therefore which need a regression test retrofitted.

**Correction to the above, on reading the final file:** `stats/queries.rs` does
have a large pre-existing `#[cfg(test)]` module (fixtures + 14 `#[sqlx::test]`
cases, `rust/web/src/stats/queries.rs:762-2287`). So the harness, the fixtures
and the house style for exactly these tests were all already in place. That makes
the omission worse, not better: adding a case for `wd F48` would have been a
ten-line addition to an existing module, and it would have caught **F-151 below**
immediately. F-150's severity rests on that.

---

### F-151 (High) - `wd F48`'s game-type filter is applied to only one side of a FULL OUTER JOIN, so the game-type page can show a different game type's rating and record

`rust/web/src/stats/queries.rs:104-152` (`game_type_stats`), consumed at
`rust/web/src/stats/mod.rs:266-279` (`get_player_game_type_stats`).

**What.** `wd F48` asked for a nullable game-type-name filter on `game_type_stats`
so the caller could stop computing every type and `.find`ing one row. The filter
was added - but only inside the `qualifying` CTE:

```
WHERE gp.user_id = $1
  AND g.is_finished = true
  AND ($3::text IS NULL OR gt.name = $3)      <-- filter is here, CTE only
```

The final SELECT then does:

```
FROM agg
FULL OUTER JOIN (
    SELECT game_type_id, rating, peak_rating FROM game_type_users WHERE user_id = $1
) gtu ON gtu.game_type_id = agg.game_type_id
LEFT JOIN game_types gt ON gt.id = gtu.game_type_id
ORDER BY "game_type_name!"
```

The `gtu` subquery is filtered by `user_id` only - **not by `$3`**. Because the
join is a FULL OUTER JOIN, every game type the user holds a `game_type_users`
rating row for still produces an output row, with `agg.*` NULL, so
`COALESCE(agg.game_type_name, gt.name)` resolves to *that other game type's*
name, `games`/`wins` COALESCE to 0, and `gtu.rating`/`gtu.peak_rating` carry that
other game type's rating.

The caller then replaced the old `.find(|s| s.game_type_name == canonical)` with:

```rust
.into_iter()
.next()
```

`ORDER BY "game_type_name!"` is alphabetical, so `.next()` returns the
**alphabetically first** game type the user has a rating in - not necessarily the
one that was requested.

**Concretely.** A user rated in "Acquire" and "Zebra Game" who opens
`/players/<name>/zebra-game`: `qualifying` yields Zebra Game only; `gtu` yields
Acquire and Zebra Game; the FULL OUTER JOIN yields two rows; ordering puts
Acquire first; `.next()` takes Acquire. The page renders
`PlayerGameTypeData.game_type_name = "Zebra Game"` (taken from `canonical` at
`stats/mod.rs:313`, so the heading is right) while `stats.games`, `stats.wins`,
`stats.win_percent`, `stats.avg_place_percentile`, `stats.rating` and
`stats.peak_rating` all come from Acquire. The page silently misreports the
user's record and Elo for the game type being viewed.

The pre-change code was correct: it computed all types and `.find`ed by name.
The regression is entirely in the pairing of a half-applied filter with the
switch from `.find` to `.next`.

**Why it matters.** This is the exact failure the brief flagged as the second
hunt target: *a "performance" rewrite that changes which players/rows a stat
covers, while the checklist row only asked for speed.* It is a public,
unauthenticated endpoint (`get_player_game_type_stats` treats
`viewer_user_id: None` as valid), so any visitor sees wrong data for any player
who has ratings in more than one game type - i.e. most active players. It is also
silent: no error, no empty state, just wrong numbers under a correct heading.

**Also note the `.unwrap_or_else` fallback at `stats/mod.rs:271-279` is now
nearly unreachable.** It exists to synthesise a zeroed `GameTypeStats` when the
user has never played the type - but with an unfiltered `gtu` side, a row is
almost always returned, so the "never played this type" case renders another
game type's numbers instead of zeros.

**Suggested fix.** Push the filter to both sides and make the caller's selection
explicit rather than positional. Either add `AND ($3::text IS NULL OR
gt2.name = $3)` to the `gtu` subquery (joining `game_types gt2`), or - simpler
and safer - keep `.find(|s| s.game_type_name == canonical)` in
`get_player_game_type_stats` so the filter is a pure optimisation that cannot
change the selected row. Do both. Then add the `wd F48` regression test the
checklist already required: a user with ratings in two game types, requesting the
alphabetically-later one, asserting `stats.rating` matches that type.

---

### F-152 (Medium) - `wd F55`'s `NULLS LAST` fix skipped the third, byte-identical ordering in the same file (pattern 2)

`rust/web/src/stats/queries.rs:713` (`recent_form_for_game_type`).

**What.** `wd F55` reads: *"fns `finished_games` and `recent_form` - Add `NULLS
LAST` to the `finished_at DESC` ordering in both ... so legacy NULL-`finished_at`
finished games stop pinning to the top."* Both named sites were fixed:

- `finished_games`, `:312` - `ORDER BY g.finished_at DESC NULLS LAST, g.id`
- `recent_form`, `:638` - `row_number() OVER (PARTITION BY gt.id ORDER BY g.finished_at DESC NULLS LAST, g.id)`

`recent_form_for_game_type` sits 75 lines below `recent_form`, is its
multi-user sibling, selects the same columns, and has the same window:

```
row_number() OVER (
    PARTITION BY gp.user_id ORDER BY g.finished_at DESC, g.id
) AS rn
```

**No `NULLS LAST`.** PostgreSQL defaults `DESC` to `NULLS FIRST`, so this is the
identical defect `wd F55` describes, untouched. It filters `g.is_finished = true`
without any `finished_at IS NOT NULL` guard, so legacy rows with a NULL
`finished_at` sort to `rn = 1..n` and displace the genuinely recent games from
every user's form window on the game-type leaderboard.

(For completeness: `rating_series:195` orders `finished_at` ASC but explicitly
filters `AND g.finished_at IS NOT NULL`, so it is unaffected. `game_history:455`
and `game_history_count` order on `g.created_at`, which is NOT NULL. Those are
the only other orderings in the file. `recent_form_for_game_type` is the sole
miss.)

**Why it matters.** Systemic pattern 2 - *inconsistent hardening within a single
file* - now has a clean web-half instance to sit beside F-61 and F-116. The
shape is the same every time: the checklist row enumerates function names, the
implementer fixes exactly those names, and the sibling three screens away in the
same file that the row's *rationale* obviously covers is never grepped for.
Note `recent_form_for_game_type` was demonstrably in the implementer's field of
view - `wd F50` required editing it (the `>= 2` comment at `:700-701` was added
by this same commit), so the function was open and edited while carrying the bug
the neighbouring row was about.

**Suggested fix.** `ORDER BY g.finished_at DESC NULLS LAST, g.id` at `:713`. For
the remediation plan, the general form: when a row names N functions, grep the
file for the *predicate being fixed*, not the function names.

---

### F-153 (Medium) - `wd F50`'s "one const used by all eight sites" shipped as a dead `#[allow(dead_code)]` string used by zero sites

`rust/web/src/stats/queries.rs:7-20`.

**What.** The row: *"Extract the single-human eligibility correlated subquery
into one `const` SQL fragment **used by all eight sites**."* What landed:

```rust
#[allow(dead_code)]
const ELIGIBILITY_PREDICATE: &str = "(SELECT count(*) FROM game_players gp_elig WHERE gp_elig.game_id = g.id AND gp_elig.user_id IS NOT NULL) >= CASE WHEN $include_single_human THEN 1 ELSE 2 END";
```

with a doc comment that states the position plainly: *"This is documentation, not
a substitute ... The SQL is therefore inlined at each call site ... and MUST be
kept in sync with this constant by hand. Each inlined occurrence is tagged with
an `ELIGIBILITY_PREDICATE` comment."*

The `#[allow(dead_code)]` is the proof: the constant has zero referents. The
eight sites each carry their own inlined copy plus a `// ELIGIBILITY_PREDICATE
(see const at top of file)` comment. `wd F50`'s actual defect - one predicate
duplicated eight times with no single source of truth, free to drift - is not
closed. It is marginally worsened: there are now **nine** hand-synced copies
instead of eight, and the ninth is the one that looks authoritative.

**The stated blocker is real but not insurmountable.** `sqlx::query!` does
require a string literal and cannot interpolate a `const` item - that much is
true. But it *does* accept `concat!` of literals, so a `macro_rules!` expanding
to a literal fragment (the standard idiom for exactly this problem) would have
satisfied the row as written. The checklist's own instruction was: *"if it does
not match the description, skip the row and report it - do not improvise."* The
row was neither satisfied nor skipped-and-reported; it was improvised into an
artifact shaped like the deliverable.

**I verified the nine copies are currently in sync**, so there is no live
correctness bug today - this is a durability finding, not a data bug. The
`recent_form_for_game_type` deviation (`>= 2` hardcoded) is the row's explicitly
permitted alternative and is correctly documented at `:700-701`.

**Why it matters - propose this as a new named pattern.** Call it the
**documentation-only constant**: a row asking for *extraction to a shared
definition* is satisfied by creating the definition and leaving every call site
untouched, with a comment explaining that manual synchronisation is now required.
It is a close relative of pattern 5 (`_ => <default>`) - a structural artifact
that satisfies the row's noun while inverting its verb - but distinct enough to
name, because unlike pattern 5 it leaves a `#[allow(dead_code)]` marker that
makes it trivially greppable at sign-off. **Sweep suggestion for the unified
report: `rg "allow\(dead_code\)" rust/` across the whole remediation range.**

**Suggested fix.** Replace with `macro_rules! eligibility_predicate { () => { "..." } }`
and `concat!()` it into each `query!` literal, or accept the inlining and delete
the constant so nothing implies a single source of truth that does not exist.

---

### F-154 (Medium) - `wd F52`'s canonicalization turns an unknown `game_type` filter into no filter at all

`rust/web/src/stats/mod.rs:343-348` (`get_player_history`).

**What.** `wd F52` asked that the client-supplied `game_type` be resolved through
`find_game_type_name` before being passed down, *"matching
`get_player_game_type_stats`' case-insensitive behaviour."* What landed:

```rust
let game_type = match game_type {
    Some(ref gt) if !gt.is_empty() => find_game_type_name(&pool, gt)
        .await
        .map_err(internal("get_player_history: find game type"))?,
    _ => None,
};
```

`find_game_type_name` returns `Option<String>` and yields `None` for an unknown
name. That `None` is then bound straight into `game_history` /
`game_history_count`, whose predicate is `($3::text IS NULL OR gt.name = $3)` -
i.e. **`None` means "no filter"**. So a request for a game type that does not
exist now returns the player's *entire* history rather than an empty list.
Before the change the raw string was passed through and matched zero rows.

It also does **not** match the behaviour the row told it to match:
`get_player_game_type_stats` returns `Ok(None)` (a 404) when
`find_game_type_name` misses (`stats/mod.rs:258-264`). `get_player_history`
silently succeeds with unfiltered data. The row's one explicit acceptance
criterion - parity with the sibling - is the part that was not met.

**Why it matters.** The response's `filters.game_type` field is set from the
same resolved `Option` (`:384`), so the client is told "no game type filter was
applied" and will render the unfiltered list under whatever the user typed. It
is a correctness and UI-consistency defect rather than a security one (history
rows are already subject to `visible_user_ids` redaction downstream), which is
why this is Medium and not High. But it is a second instance in this one commit
of *a row that changed result semantics while its `Test? y` box went unfilled* -
see F-150.

**Suggested fix.** Distinguish "not supplied" from "supplied but unknown":
return `Ok(None)` on an unresolvable game type, matching
`get_player_game_type_stats`; or plumb a sentinel that forces an empty result
set. Add the regression test the row already required.

---

### F-155 (Low) - `wd F53`'s justifying comment is copy-pasted to three sites and is factually wrong at one of them

`rust/web/src/stats/queries.rs:232`, `:429-430`, `:509-510`.

**What.** `wd F53` offered a choice: convert three runtime `sqlx::query_as` calls
to the compile-time `sqlx::query!` macro, *"or add a comment stating why runtime
checking is needed."* The comment branch was taken, with the identical line at
all three sites:

```rust
// Runtime query_as: result shape maps naturally to a named FromRow struct; binds are static.
```

At `opponents_by_game` (`OpponentRow`) and `game_history` (`GameHistoryRow`) the
claim is at least accurate. At `game_history_count` (`:511`) the destination type
is `(i64,)` - an anonymous tuple, not a named `FromRow` struct - so the stated
reason does not describe the site it annotates. Worse, the second clause,
*"binds are static"*, is an argument **for** the compile-time macro, not against
it: static binds are precisely the condition under which `sqlx::query!` works.
The comment as written justifies the opposite of what it concludes.

**Why it matters.** Low severity - a single-row count query is not a correctness
risk. It is logged because it is the third distinct row in this commit satisfied
by producing an artifact rather than the effect (with F-153 and, in a different
way, F-150), and because at sign-off a reviewer grepping for "was `wd F53`
addressed?" finds a comment at all three cited sites and marks it closed. This is
the shape F-147's carry-forward warns about: the citation exists and is
unreachable as an actual justification.

**Suggested fix.** `game_history_count` is a bare `SELECT count(*)` with four
static binds and no shape problem at all - convert it to `sqlx::query_scalar!`
and delete the comment. Leave the other two.

---

### F-156 (Medium) - `wd F74`'s bound truncates the friends feed alphabetically, on an axis unrelated to the feature

`rust/web/src/index.rs:47-73` (`get_logged_in_index`).

**What.** `wd F74` asked to *"bound the `list_friends` result and run the
per-friend `friend_recent_visible_game` calls concurrently."* The concurrency
half is done correctly (`futures_util::future::try_join_all`). The bound is:

```rust
let friends: Vec<_> = friends.into_iter().take(20).collect();
```

`list_friends` (`rust/web/src/db/social.rs:205-217`) ends `ORDER BY
lower(u.name)`. So the bound is *the alphabetically first 20 friends*.

The feature being bounded is the logged-in home page's **"friends' recent
games"** feed - `FriendRecentGame` carries `game_id`, `game_type_name` and
`updated_at`, and `try_join_all` preserves input order, so the rendered list
stays in name order too. For any user with more than 20 friends, every friend
whose name sorts after the 20th is permanently invisible in that feed,
regardless of how recently they played. There is no "show more", no count, and no
indication that truncation occurred.

**Why it matters.** The bound is correct as a resource limit and is exactly what
the row's words asked for, but it is applied on the wrong axis: a recency feed
truncated by name. The old code was unbounded and therefore always correct-if-slow;
the new code is fast and silently incomplete. This is the milder form of the same
class as F-151 - *a performance row that changed which subjects the feature
covers* - and it is the reason the brief flags mostly-perf commits as
result-semantics risks. Severity Medium rather than High because the data is not
wrong, only missing, and the page is per-user rather than public.

**Suggested fix.** Bound on the axis the feature is about. Either push the limit
into SQL ordered by the friend's most recent visible game, or - cheapest - fetch
all friends' recent games concurrently but chunk the concurrency (e.g.
`buffer_unordered(10)`) rather than truncating the input set. If truncation must
stay, surface it in the UI and order by recency before taking 20.

Note also the concurrency shape: `try_join_all` fires up to 20 simultaneous
`friend_recent_visible_game` queries, each acquiring its own pool connection, for
a single page render. That is within the row's mandate and not itself a finding,
but it converts a serial N+1 into a burst that can saturate the pool from one
request - worth a `buffer_unordered` bound in the same change.

---

### F-157 (Low) - `tokio::try_join!` collapsed eleven distinct error contexts into two

`rust/web/src/friends.rs:100-108` (`get_friends_overview`, six queries -> one
`internal("get_friends_overview: queries")`) and
`rust/web/src/game_info/mod.rs:164-172` (`get_game_info`, five queries -> one
`internal("get_game_info: queries")`).

**What.** Both `wd F62` and `wd F75` were satisfied correctly on their own terms
- the queries genuinely run concurrently now. But each rewrite replaced per-query
`.map_err(internal("...: friends"))` / `"...: incoming"` / `"...: outgoing"` /
`"...: blocked"` / `"...: policy"` / `"...: visibility"` with a single
catch-all context string.

**Why it matters.** `internal(...)` is this crate's error-context helper; the
string is what lands in the log when a query fails. After the change, a failure
in any of the six friends queries logs `get_friends_overview: queries`, so
on-call cannot tell which query broke without reproducing. Low severity - purely
an observability regression, no behaviour change, and no row prohibited it - but
it is a real cost that the rows did not ask anyone to pay, and it is trivially
avoidable.

**Suggested fix.** Keep the per-future `.map_err` inside the `try_join!`
arguments (each argument is an expression; `crate::db::list_friends(&pool,
user.id).map_err(internal("...: friends"))` composes fine) and drop the outer
one.

## Verified good

Verified against the final code, not the commit message. These are the rows that
did what they were asked and did not change semantics:

- **`wd F51` (`game_history` LATERAL rewrite) - correct, including the NULL
  handling.** The four correlated subqueries became one
  `LEFT JOIN LATERAL (...) agg ON true` (`stats/queries.rs:444-450`). I checked
  the two ways this class of rewrite normally breaks:
  1. *Row multiplication.* The lateral subquery is aggregate-only with no
     `GROUP BY`, so it returns exactly one row always; `ON true` therefore cannot
     drop or duplicate an outer row. Row counts are identical.
  2. *NULL handling.* The old subqueries each carried an explicit
     `AND r.rating_before IS NOT NULL`; the new lateral does **not**. This is
     nonetheless equivalent, because SQL's `min`/`max`/`avg` ignore NULL inputs
     by definition, and the one non-NULL-sensitive aggregate, `count(*)`, was
     never filtered in the old code either (`SELECT count(*) FROM game_players
     gp2 WHERE gp2.game_id = g.id`). `player_count` is unchanged; `match_min`/
     `match_max`/`match_avg` are unchanged; `avg(rating_before)::int` keeps the
     same cast and therefore the same rounding.
  This one is genuinely a pure performance win. **REFUTED as a finding** - I
  raised the missing `IS NOT NULL` as a candidate and the aggregate semantics
  refute it.
- **`wd F21` (`should_hide_add_friend_many`) - the batched predicate is exactly
  equivalent to the singular one.** Compare
  `db/social.rs:168-179` with `:182-202`: the singular is
  `EXISTS(... WHERE (source=$viewer AND target=$t) OR (has_accepted AND
  source=$t AND target=$viewer))`; the batch is the same two disjuncts as a
  `UNION` of two `= ANY($2)` selects, with `has_accepted = TRUE` correctly
  applied to the reverse direction **only**, matching the singular. The
  `targets.is_empty()` early return avoids an empty-array bind. The caller's
  `*uid != user.id` self-exclusion moved from the loop body into a `filter`
  before the call (`game/server_fns.rs:130-134`) and is preserved. This is the
  best row in the commit.
- **`wd F47` (`INITIAL_RATING`) - done properly, both sites.**
  `db/common.rs:24` defines `pub const INITIAL_RATING: i32 = 1200;`, and it is
  actually *used* at both places the row named: `build_game_type_user`'s
  synthetic row (`db/common.rs:35-36`, previously two `1200` literals) and
  `stats::queries::rating_series` (`queries.rs:203`, previously `let mut rating
  = 1200`). Contrast with F-153 - this is what `wd F50` was supposed to look
  like, in the same commit, by the same author.
- **`wd F62` / `wd F75` (concurrency) - the joins are correct.** Six independent
  reads in `get_friends_overview` and five in `get_game_info` now run under
  `tokio::try_join!`. All are `&PgPool` reads with no ordering dependency and no
  shared transaction, so concurrency is safe. `get_game_info`'s sixth and seventh
  awaits (`recent_form_for_game_type` and the follow-on) correctly stay
  sequential - they consume `ranking_rows`. (The row offered "or merge the three
  count queries with `FILTER` clauses"; the `try_join!` alternative was taken,
  which the row explicitly permits.) See F-157 for the error-context cost.
- **`wd F49` (cap the anonymous game-type page) - satisfied.**
  `finished_games` now gets `Some(100)` instead of `None`, `head_to_head` is
  `truncate(50)`d and `rating_series` is `split_off` to the last 200
  (`stats/mod.rs:284-309`). Note the latter two bound the *payload* in Rust
  after fetching every row, not the query - so the DB work is unbounded even
  though the response is not. The row said "bound ... too" without specifying
  where, so this satisfies it; flagging it only because the stated motivation
  ("since this is an anonymous endpoint") is about work, not bytes. Not raised
  as a finding.
- **WP-47's anonymization was NOT undone** - this was the breakdown's specific
  Unit 08 gotcha and it is clean. `opponents_by_game` still resolves
  `crate::db::visible_user_ids` and maps invisible opponents to
  `(None, "Anonymous")` (`queries.rs:252-279`), and `head_to_head` does the same
  (`:593-607`). The `viewer` parameter threads through `finished_games`,
  `active_games` and `game_history` unchanged. No merge or ordering conflict
  between WP-47 and WP-52.
- **No pattern 4e revert.** This is the highest-deletion commit in the programme
  and it deletes **no guard and no test**. Verified two ways: the 95-file list
  contains no test path, and grepping the diff for removed `#[test]` /
  `#[tokio::test]` / `#[cfg(test)]` / `fn test_` returns nothing. The 2947
  deletions are 91 `.sqlx` offline-cache JSON files - build artifacts. The
  breakdown's "highest-risk shape" framing does not apply to this commit.

## Coverage gaps

- **Seven `Test? y` rows, zero tests** - F-150. Specifically untested at HEAD:
  the `game_type_stats` name filter (which is why F-151 shipped), the
  `NULLS LAST` ordering, the `clamp(1, 1_000_000)` paging bound, the
  `Some(100)`/`truncate(50)`/`split_off(200)` caps, `should_hide_add_friend_many`,
  and `get_player_history`'s game-type resolution. The pre-existing
  `#[sqlx::test]` module and its fixtures (`queries.rs:762-908`) make every one
  of these a short addition.
- **`rating_before_aggregates_exclude_nulls` (`queries.rs:1287-1346`) is a
  decoy.** Its name reads as coverage for exactly `wd F51`'s risk, and a
  sign-off grep would treat it as such - but it never calls `game_history`. It
  issues its own raw `SELECT min/max/avg(...) FROM game_players WHERE
  rating_before IS NOT NULL` and asserts PostgreSQL's aggregate semantics. It
  pins the database's behaviour, not the application's, and would keep passing if
  `game_history`'s lateral were deleted outright. Pre-existing, not WP-52's
  doing, but it is the exact hazard F-147's carry-forward describes: the citation
  exists and is not reachable from the code it appears to protect. **Add to the
  unified report's sign-off procedure: a regression test must call the function
  under test.**
- **`wd F46`'s next-page link ceiling is unverified.** The row asked for the
  clamp on three places: the server (`stats/mod.rs:351` - done), the client-side
  page parse (`players.rs:236` - done, `page.max(1)` -> `page.clamp(1,
  1_000_000)`), and *"the `page + 1` next-page link"*. `players.rs` changed by
  exactly one line in this commit, so the link was not touched. I did not read
  the link-construction code and am **not** raising this as a finding - the
  practical effect is benign in any case, since a `page=1000001` link is clamped
  back to 1,000,000 by both the client parse and the server on the next request.
  Flagged so a remediation pass confirms rather than assumes.
- **`rust/.sqlx/` vs `rust/web/.sqlx/`.** This commit deletes 82 entries from a
  workspace-root `rust/.sqlx/` cache while maintaining `rust/web/.sqlx/`. Two
  files existed in *both* directories. Nothing in WP-52's scope mentions the
  cache layout; this looks like fallout from WP-66's sqlx unification (Unit 10)
  being cleaned up opportunistically here. **Route to Unit 10:** confirm which
  directory `cargo sqlx prepare` writes in the current workspace and that no
  crate still resolves against the deleted root cache. Not reviewable from this
  unit's scope.

## Carry-forward handled

- **`Gamer::points()` ordering contract - handed back, not consumed.** WP-52
  touches none of it. All nine changed files are in `rust/web`, and the stats
  layer never calls `points()`: it reads the `game_players.place`,
  `ranked_placing`, `rating_change` and `rating_before` **columns**, and derives
  wins from `place = 1` and percentile from `(n - place) / (n - 1)`. There is no
  path from this commit to `lib/game`'s trait surface, so the assumption
  cathedral-2 inverts is not exercised here. It needs a genuine owner - the
  natural one is a `lib/game` trait-surface unit, which no unit in this session
  has. Recommend it become a remediation-plan item rather than being routed to
  another review unit.

## Progress

- [x] W1: recon - commit shape, no-spec confirmation, 13 checklist rows recovered.
- [x] W2: full source diff + sqlx cache diff + test-presence/absence greps.
- [x] Lead: read final code of `stats/queries.rs`, `stats/mod.rs`, `index.rs`,
      `db/social.rs` and verified all 13 rows individually against it.
- [x] Report complete. 8 findings, F-150..F-157.

## Verified good

_(pending)_

## Coverage gaps

_(pending)_
