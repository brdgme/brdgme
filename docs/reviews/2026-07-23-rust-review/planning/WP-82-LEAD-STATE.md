# WP-82 Lead state (db.rs module split)

Lead-owned scratch doc for continuity. Authority remains the spec
(`specs/WP-82-db-module-split.md`), `EXECUTION-README.md`, and `DECISIONS.md`.

## Tree at start
- Clean on master @ 37118d3 (per brief). 13 packages already landed.
- WP-82 is a PURE MOVE: same fns, same SQL (byte-identical), same signatures.
  `pub use` re-exports in `db/mod.rs` keep all external `crate::db::foo` callers
  compiling unchanged. No behaviour change. No extra refactors.

## Governing rulings
- Spec cites no D-nn rulings. Only constraint: WP-82 lands FIRST (EXECUTION-README s1). Satisfied.

## Plan (serial Workers)
- T1 survey (read-only): verify live tree == spec inventory; capture baseline
  test-name list; confirm doc-comment sections, `Result` alias (spec says
  anyhow::Result, Lead to confirm via worker), the 5 import lines, 12 fixtures.
  STOP-AND-REPORT gate.
- T2 atomic production split: mkdir db; create mod.rs (doc rewritten per 3f,
  create_pool, BotSlot re-export, `mod x; pub use x::*;` per submodule) + 12
  production modules; bump cross-module private items to pub(crate) per 3d;
  move the single `mod tests` VERBATIM into mod.rs (fixtures stay inside for
  now); delete db.rs. Verify clippy+fmt+test-compile, test-name set identical.
- T3 test distribution: hoist 12 fixtures -> db/test_support.rs (pub(crate),
  cfg-gated, declared in mod.rs, NOT re-exported); distribute test clusters
  into per-module `mod tests` per 3g; fix friend_recent_visible_game
  cross-reference doc comment per 3h, keep drift-guard test. Verify.
- T4 final verification (spec s5) + commit naming WP-82. No push.

## Key spec constraints (do not violate)
- Per-item `#[cfg(feature="ssr")]` gates travel verbatim. NO module-level ssr
  gate. `validate_username` stays ungated (client build depends on it).
- Item attrs travel: 3x `#[allow(clippy::too_many_arguments)]`
  (build_game_type_user, build_game_player_from_row, update_game_command_success),
  12x `#[tracing::instrument]`.
- SQL byte-identical. No `cargo sqlx prepare` needed (cache keyed by query text
  hash only). If build demands re-prepare, a query was edited -> STOP.
- Visibility: only bump to pub(crate) what 3d lists; never widen to pub what
  was not pub.
- Zero changes outside rust/web/src/db/ except deletion of db.rs.
- DB-dependent test failures locally are KNOWN/pre-existing, not a regression.

## Module table (spec 3c) - 13 modules + test_support
mod.rs, common, game_types, games, game_write, bots, rating, users, emails,
social, visibility, discovery, proposals (+ test_support, cfg-gated, not re-exported).

## Verification commands (run via Workers)
- cargo fmt --all -- --check
- cargo clippy -p web --all-targets --features ssr -- -D warnings
- cargo test -p web --features ssr --no-run  (compile tests; DB tests fail locally = known)
- cargo check -p web  (client/non-ssr build still sees validate_username)
- test-name set before == after (sorted fn-name list from the test module)
- git diff --stat: db.rs deleted, db/* added, near-zero net line delta

## Status
- [x] T1 survey - CLEAN, no STOP. HEAD 37118d3 confirmed, tree clean (only untracked
      planning doc under docs/). db.rs 8149 lines, single cfg-gated mod tests @3312.
      All production symbols present, none extra. 5 import lines match. 3 doc sections
      present. 12 fixtures present. Result=anyhow::Result. Attrs: 3 too_many_arguments,
      15 tracing::instrument (spec said 12 - drift, irrelevant: all travel).
      Baselines: /tmp/opencode/wp82-test-names-before.txt (128 attributed test fns),
      /tmp/opencode/wp82-all-fns-in-tests-before.txt (145 fns incl helpers).
- [x] T2 production split - DONE, verified. 13 files (mod.rs+12), 126 blocks moved
      verbatim. clippy/fmt/test--no-run/hydrate-check all exit 0. test-name diffs
      EMPTY (128+145). sqlx no re-prepare (SQL byte-identical). No files outside db/.
- [x] T3 test distribution - DONE, verified. test_support.rs (12 pub(crate) helpers),
      tests distributed to per-module mod tests (counts sum to 128). elo_rating_change
      narrowed back to private. friend_recent_visible_game doc-comment updated to name
      crate::db::is_game_visible_to_user; drift-guard test in discovery.rs. clippy/fmt/
      test--no-run/hydrate all pass. Identity diffs EMPTY (128 tests, 145 fns no MISSING).
- [x] T4 verify + commit - DONE. Independent re-verify all green; one commit.
      SHA 4d31f6eb317afcf869e05720b846c9edd268bca1
      Message: refactor(web): split db.rs into a module (WP-82)
      15 files changed, 8312 insertions(+), 8149 deletions(-), net +163 (headers/
      use-blocks/re-exports/mod-tests wrappers/module map/pub(crate) bumps).
      Only rust/web/src/db/ in commit; planning doc untracked; NOT pushed.

## PACKAGE COMPLETE

## T3 accepted deviations
- No separate intermediate compile after fixture hoist (process shortcut; end state
  fully verified; parse checked against baseline before writing).
- insert_proposal NOT hoisted: live it is nested (8-space) local helper inside one
  proposals test, not a shared top-level fixture (inventory listed it as shared; live
  disagrees). Left in place, moved intact with the test. Per brief instruction.
- accept_friends ADDED to test_support (genuinely shared: visibility+discovery+social).
  Spec said "~12" (approximate). test_support final set = 11 spec-listed + accept_friends
  - insert_proposal(nested) = 12.
- Drift-guard test references is_game_visible_to_user via re-export (use super::*),
  not a literal full path. Functionally equivalent; compiles; test present. Accepted.
- Per-module test counts: mod.rs 3, common 9, rating 11, discovery 12, social 19,
  visibility 13, game_write 17, users 14, emails 10, games 13, game_types 3, bots 3,
  proposals 1 = 128.
- mod.rs leftover tests: migrations_apply_and_pool_connects (create_pool lives in mod.rs),
  session_token_validation (exercises crate::auth::session, no db submodule home),
  ws_f35_* (spans many domains). Allowed by spec 3g.

## T2 accepted deviations (forced, behaviour-preserving)
- `pub(crate) use rating::*;` (not `pub use`) - rating has no pub items; `pub use`
  errors under clippy -D warnings. Public surface unchanged (rating contributes
  nothing public). All other modules keep `pub use X::*;`.
- `use super::*;` added to games, game_write, users, visibility, discovery (the 5
  modules with cross-module bare calls) to keep fn bodies byte-identical. 7
  self-contained modules omit it.
- mod.rs imports StatusUpdate/User/Uuid re-gated `#[cfg(all(test,feature="ssr"))]`
  (only the test module needs them now); Result/PgPool stay `#[cfg(feature="ssr")]`.
- fmt wrapped write_ranked_placings signature (pub(crate) bump > 100 col). Formatting only.
- mod.rs `mod tests` now @ line 107.

## Finding (non-blocking, pre-existing)
- bare `cargo check -p web` (no features) = 15 pre-existing errors in UNTOUCHED files
  (theme.rs/settings.rs/server_fns.rs/components/game.rs: unresolved optional deps
  brdgme_color/brdgme_game/brdgme_markup). None mention db/. Real client build
  `cargo check -p web --features hydrate` exit 0 -> validate_username ungated/visible.
  T4 to re-confirm hydrate build + note no-feature check is pre-existing.

## Drift notes (non-blocking, resolved by spec's own rules)
- 4 bump-candidates already `pub`: are_friends_conn, has_block_conn,
  pick_replacement_bot, generate_unique_username. KEEP pub (do not narrow; do not
  widen non-pub to pub). Bump only genuinely-private ones to pub(crate):
  build_user_from_row, build_game_bot_from_row, build_game_type_user,
  build_game_player_from_row, choose_colors, write_ranked_placings,
  apply_rating_changes. normalize_pref_color already pub(crate), keep.
- elo_rating_change is private + exercised by tests but NOT in 3d list. In T2 the
  verbatim test module sits in mod.rs, so elo_rating_change must be temporarily
  pub(crate) to compile. T3 narrows it back to private once elo tests move into
  rating.rs (spec intent: private, tests are child module).

## Deviations / STOP events
(none)

## Commit hygiene
- Final commit must stage ONLY rust/web/src/db/ changes (db.rs deletion + db/* add).
  Do NOT commit WP-82-LEAD-STATE.md (planning scratch) or /tmp baselines.
