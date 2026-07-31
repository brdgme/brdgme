# Unit 11 - hanamikoji-1 + unassociated tail fixes

Findings F-208.. . Last review unit.

## Progress

- [x] Carry-forward 1: hanamikoji-1 epilogue guard - REFUTED (gate present at `:830`)
- [x] `validate` override + `test_validate` present - first crate in the session with a validate test
- [x] Carry-forward 2: Dockerfile / docker-bake.hcl / k8s three-way discrepancy - DONE (F-208; 43-vs-26 premise REFUTED)
- [x] Carry-forward 3: all four tail fixes reviewed - `a99bf754` good, `3f52d2b7` good, `e2aef66b` F-211, `ae04843c` F-210
- [x] hanamikoji-1 crate review: pub_state clean, `Log::public` clean, validate present (F-209 gap), redaction test present, char/byte clean, no non-test panics
- **Unit 11 COMPLETE.** F-208, F-209, F-210, F-211 + one refuted carry-forward premise (F-208a) and one refuted carry-forward (CF-1).

## Findings

### F-208 (High) - `hanamikoji-1` is built by CI and then thrown away: no image stage, no bake target, no Deployment

- **Where**: `rust/Cargo.toml:13` (workspace member) vs `rust/Dockerfile:36` (`RUN cargo build --release --workspace --exclude web`) and `rust/Dockerfile:174-302` (26 game stages); `docker-bake.hcl:36-64` (26 game matrix entries); `k8s/base/game/kustomization.yaml` (43 Deployments).
- **What**: The three-way inventory, verified directly:
  - **28** workspace `game/*` members.
  - **26** Dockerfile game stages (`:174-302`), each a 4-line distroless block copying `/app/target/release/<name>_http`.
  - **26** `docker-bake.hcl` game targets - **exactly equal** to the Dockerfile stage set, one-to-one (`target = tgt`, `name = tgt`), zero difference either way.
  - **43** k8s game Deployments.
  - Absent from stages/bake/k8s: **`hanamikoji-1`** and **`lords-of-vegas-1`** (WIP, excluded by owner ruling).
  - `rust/game/hanamikoji-1/src/bin/hanamikoji_1_http.rs` exists, so the binary *is* compiled by `:36`'s `--workspace` build on every CI image build, consuming build time, and is then never copied into any image.
  - `rg 'hanamikoji'` across the whole repo returns **zero** hits in `rust/Dockerfile`, `docker-bake.hcl`, `k8s/**`, `.github/**`, `Tiltfile`, `scripts/**`. Its only non-crate mentions are `rust/Cargo.toml:13` and `rust/Cargo.lock:2243`.
- **Why**: A complete, tested, documented new game port (1761 + 213 + 268 lines across `f4cbc51d`, `c882d413`, `16dae9dd`) is **unshippable**. There is no build-time or deploy-time signal - the workspace build succeeds, bake succeeds, kustomize succeeds. Nothing in the repo can detect a game crate that has no delivery path. Compounding: **no commit since `082b63f` (2026-07-20) touches `rust/Dockerfile` or `docker-bake.hcl`** - the whole 127-commit remediation window included - so the omission was never going to be caught by the programme either.
- **Fix**: Add the 4-line stage to `rust/Dockerfile`, the matrix entry to `docker-bake.hcl`, and `k8s/base/game/hanamikoji-1/` + its `kustomization.yaml` base and the `k8s/prod/app/kustomization.yaml` `images:` rewrite. Then add a CI guard asserting every `rust/Cargo.toml` `game/*` member has a Dockerfile stage (an explicit allowlist for deliberate WIP exclusions such as `lords-of-vegas-1`), since all three lists are hand-maintained and mutually unlinked.

### F-209 (Medium) - `hanamikoji-1::validate` checks every parallel-vector length but not the one cross-field invariant the state machine depends on: `phase` vs `pending`

- **Where**: `rust/game/hanamikoji-1/src/lib.rs:673-730` (`validate`), against `:190-196` (`whose_turn_calc`), `src/command.rs:22-60` (`command_parser`), `lib.rs:356-367` (`assert_action`), `:452-481` (`gift`).
- **What**: `validate` bounds `players`, all six vector lengths, `current`, `starting`, `winner`, every `marker` owner and both `Pending` variants' `actor`. It never relates `self.phase` to `self.pending`. Both inconsistent combinations are accepted, and each is reachable only through the D-36 deserialized-state boundary that `validate` exists to defend.
- **Failure path A - permanent wedge.** State with `phase: OpponentChoose, pending: None` passes `validate` (the `if let Some(pending)` at `:713` simply does not fire).
  - `status()` (`:732`) returns `Status::Active` with `whose_turn: self.whose_turn_calc()`, which for `OpponentChoose` is `vec![1 - self.current]` (`:193`) - so the server believes that seat is on turn.
  - That seat's `command_parser` takes the `Phase::OpponentChoose if player == 1 - self.current` arm and matches `None => {}` (`command.rs:51`), leaving `parsers` empty, so `:55-56` returns `None`.
  - The other seat falls through to `_ => {}` (`command.rs:53`) and also gets `None`.
  - `Gamer::command` (`lib.rs:787-794`) turns `None` into `Err(invalid_input("not expecting any commands at the moment"))` for **both** seats.
  - Nothing else mutates `phase`. The game is Active, names a player to move, and has no legal move for anyone, forever.
- **Failure path B - silent card destruction.** State with `phase: ChooseAction, pending: Some(Pending::Gift { .. })` also passes. `assert_action` (`:356-367`) only checks `phase == ChooseAction` and `current == player`, so the current player may play `gift` again: `:464` removes three more cards from hand and `:465-468` **overwrites** `self.pending`. The first gift's three cards were already removed from the hand at `:464` on the earlier call and are never placed face-up (that happens in `choose_gift:542-549`), so they leave the game entirely. `used[2]` is likewise only set in `choose_gift:550-552`, so the gift action is not even consumed. Same shape via `compete`/`Pending::Competition`.
- **Why**: This is systemic pattern **2b** verbatim - "`validate` overrides cover the parallel-vector sweep but miss the one cross-field invariant each crate's remaining panic actually depends on" (F-66/67/68/76). Notable difference: here the consequence is not a panic but a wedged or corrupted game, which is quieter. `test_validate` (`:1079-1103`) exercises only the length and range rejections, so it does not cover this - the crate's presence of a validate test does not close the gap. The same omission means `validate` performs **no card conservation check at all**: the 21-card multiset across `deck`/`hands`/`secret`/`traded`/`faceup` is never verified, so a supplied state can contain four copies of one geisha or none.
- **Fix**: In `validate`, require `self.phase == Phase::OpponentChoose` iff `self.pending.is_some()`; require `Phase::Finished` iff `self.winner.is_some()` (currently also uncorrelated - `:752` and `:194` both read one without the other); and for `Pending::Gift` require `cards.len() == 3`, for `Pending::Competition` require `sets[0].len() == 2 && sets[1].len() == 2`. Add the card-conservation check against `Geisha::full_deck()` (`card.rs:81-89`). Extend `test_validate` with one rejection case per new rule.

### F-210 (Medium) - `ae04843c` converts a live-but-wrong default into a process abort at the D-36 boundary, and self-certifies with a premise that is only true for freshly-started games

- **Where**: `rust/game/sushi-go-2/src/lib.rs:140-147`, call site `:289` (`let dc = draw_count(self.all_players);`).
- **What**: The commit replaced `_ => 9` with `_ => unreachable!()`. Its own message states the justification: *"start() rejects player counts outside 2..=5 before draw_count is called, so it could never fire"*. `start()` is not the only way a `Game` comes into existence. `all_players` is a deserialized state field, and `draw_count` is called from the round-advance path inside `command()` (`:289`), i.e. **after** the D-36 deserialized-state trust boundary that WP-09a/09b were written to defend. A state with `all_players` outside `2..=5` no longer produces a playable-but-wrong game; it aborts.
- **Why**: This is the F-96 class ("hardening that converts a soft default into a panic") landing inside a game crate, and it is the exact inverse of pattern 5 (`_ => <default>`): rather than a catch-all masking a bug, a catch-all was removed on the strength of an invariant enforced only at one of two entry points. `unreachable!()` in a game service is not a local error - it unwinds out of `command()` into the HTTP handler for a live game. The blast radius is strictly worse than the `_ => 9` it replaced, and the change bought nothing that a `validate` bound would not have bought safely.
- **Process**: `ae04843c` is **self-certifying, the WP-72 precedent**. It has no spec, no `T3-B*` checklist row, and its acceptance evidence is a row **it wrote itself** into `docs/archive/BACKLOG.md` marked "Done" in the same commit. Nothing outside the commit can verify the claim in its message, and the claim is the part that is wrong. `test_draw_counts` (`:1279-1285`) covers only 2, 3, 4, 5 - **no test reaches the `unreachable!()` arm**, so nothing would have caught the premise either.
- **CONFIRMED, and worse than the above**: `rg 'fn validate'` over `rust/game/sushi-go-2/src/lib.rs` returns **nothing** - sushi-go-2 is one of the 13 crates in F-06's no-override list, so `Gamer::validate` is the fail-open `Ok(())` default and **no deserialized state is checked at all**. Further:
  - `all_players` is **never** bounded anywhere. The `MIN_PLAYERS..=MAX_PLAYERS` check at `:732-735` bounds `players`, not `all_players`; `all_players` is derived once at `:741` (`if players == 2 { players + 1 } else { players }`) and never re-checked.
  - `struct Game` (`:173-188`) derives `Default` and every field is `pub`, so `Game::default()` yields `all_players: 0`, which reaches `:289` and hits `unreachable!()`.
  - So the concrete failing path needs no exotic input: any persisted or reconstructed `Game` whose `all_players` is not in `{3,4,5}` - `0` included - panics the game service on the first round advance instead of, previously, dealing 9 cards.
  - `all_players` is also the length source for eight parallel vectors (`:288, 300, 331, 376, 380, 457, 718, 752-754`), which is precisely the pattern-2 surface WP-09b was meant to close for this crate and did not.
- **Fix**: Do not restore `_ => 9`. Add a `Gamer::validate` override to sushi-go-2 bounding `players` to `2..=5`, requiring `all_players == if players == 2 { 3 } else { players }`, and asserting the eight parallel vectors are all `all_players` long - then `draw_count`'s `unreachable!()` becomes genuinely unreachable and F-06 is closed for this crate at the same time. Remediate as one item with F-06's sushi-go-2 row.

### F-211 (Low) - `e2aef66b` is pattern 4b in the e2e suite: the assertion was edited down to the code's fallback string, inside a job that cannot fail the build

- **Where**: `rust/web/end2end/tests/page-loads.spec.ts:8`; `.github/workflows/ci.yml:145-152`.
- **What**: `await expect(page.getByRole("heading", { name: "Welcome to brdg.me" })).toBeVisible();` became `... { name: "brdg.me" }`. Per the commit's own message the `<h1>` now renders *"game-type name with a `brdg.me` fallback"* - so the smoke test's remaining assertion is that the page rendered its **fallback**, which is also what it would render if the game-type lookup failed entirely. The assertion no longer distinguishes a healthy index page from a degraded one.
- **Why**: Systemic pattern **4b** - the test adjusted to agree with the code rather than the discrepancy being examined. Milder than F-72a/F-83/F-79/F-95 because the code change (`68ebef7`) was intentional and the old string genuinely no longer exists, but the replacement chose the weakest available assertion. Compounding: the `e2e` job carries `continue-on-error: true` (`ci.yml:148`, "Flaky (hydration-race in the login flow)"), so **no assertion in this file can fail a merge or a deploy** - the fix restored a green tick on a job whose ticks are advisory.
- **Fix**: Assert something the fallback cannot satisfy (the logged-out call-to-action, or the game list itself). Separately, the `continue-on-error` on the e2e job is a deployment-checklist item for the F-96 family - track the hydration race rather than leaving the whole suite non-blocking indefinitely.

### F-208a (REFUTED, carried premise) - the "43 k8s Deployments vs 26 image stages" discrepancy is not a discrepancy

`00-STATE.md`'s Unit-10b carry-forward records the two lists as already disagreeing. They do not disagree in any way that indicates a defect. **43 = 26 Rust stages + 17 legacy Go games.** All 17 surplus Deployments have image stages in `/home/beefsack/Development/brdgme/brdgme-go/Dockerfile` (`age-of-war-1` :14, `liars-dice-1` :22, `for-sale-1` :30, `roll-through-the-ages-1` :38, `texas-holdem-1` :46, `modern-art-1` :54, `no-thanks-1` :62, `sushizock-1` :70, `sushi-go-1` :78, `zombie-dice-1` :86, `love-letter-1` :94, `category-5-1` :102, `cathedral-1` :110, `farkle-1` :118, `greed-1` :126, `splendor-1` :134, `battleship-1` :142). Not every `-1` suffix means Go: `acquire-1`, `alhambra-1`, `lost-cities-1`, `red7-1`, `seven-wonders-1`, `starship-catan-1` are Rust. **Dockerfile stages with no Deployment: 0. Bake targets with no stage: 0.** The only real gap is F-208's, in the workspace-vs-stage direction. Do not re-derive; do not carry the 43-vs-26 framing into the unified report.

## Verified good

### CF-1 REFUTED - `hanamikoji-1`'s epilogue **is** gated; it did not copy the pre-WP-08 pattern

`00-STATE.md` (line 549) carried "a single unguarded epilogue site (`:833`) and no `finish_epilogue` - it copied the pre-WP-08 pattern". The first half is **false**. Final code, `rust/game/hanamikoji-1/src/lib.rs:796` and `:830-834`:

```rust
796        let was_finished = self.is_finished();
...
830        if !was_finished && self.is_finished() {
831            let charm = self.charms();
832            let scores: Vec<(usize, i32)> = (0..self.players).map(|p| (p, charm[p])).collect();
833            logs.push(placings_log(&self.placings(), Some(&scores)));
834        }
```

This is byte-for-byte the WP-08 gate. Compare `rust/game/jaipur-2/src/lib.rs:759` / `:783-785` - identical `let was_finished = self.is_finished();` ... `if !was_finished && self.is_finished()`. `:833` is the *inside* of the guard, not an unguarded site. It is also **exactly one** epilogue site (`Status::Finished` appears once, `:734`), so there is no dedup problem to have.

The second half is true but cosmetic: there is no `finish_epilogue` method (12 crates have one: age-of-war-2, alhambra-1, greed-2, jaipur-2, love-letter-2, roll-through-the-ages-2, seven-wonders-1, splendor-2, starship-catan-1, sushi-go-2, texas-holdem-2, zombie-dice-2). hanamikoji-1 inlines the 3-line body instead. **Correction for the unified report: `finish_epilogue` is a per-crate private inherent method, not a shared helper in `rust/lib/game`.** The only shared helper is `placings_log` (`rust/lib/game/src/game_log.rs:36-71`), which hanamikoji-1 does use. With one call site there is nothing to factor out. Not a finding. Do not re-derive. This also means F-18/F-71's carry-forward list does **not** gain hanamikoji-1.

Corollary: hanamikoji-1 also has a **dedicated regression test for the gate** - `test_finish_emits_epilogue_once` (`:1220`), with a `has_epilogue` helper (`:1178`) that string-matches the rendered log. None of the WP-08-migrated crates were reported to have one.

### `Status::Finished` populates `stats` - the F-35 park does not apply here

`:734-737` passes `self.finished_stats()` (`:623-634`), which emits per-player `geisha` and `charm` `Stat::Int`. This is **not** a `stats: vec![]` site. Record for the F-35 tally as a *negative*: the one game crate written outside the remediation programme is the one that populated stats.

### `validate` override exists AND is tested - first such crate in the session

`:673-730`. Systemic pattern 2b in `00-STATE.md` records "**No crate reviewed so far has a `validate` test**". hanamikoji-1 breaks that streak: `test_validate` at `:1079`. The override covers the D-36 parallel-vector surface completely for the fields that later index raw:

- `players != 2`, and `hands/secret/traded/used`.len() != 2, `marker/faceup`.len() != GEISHA - which is what makes the raw indexing at `:266`, `:295-297`, `:543`, `:589`, `:629-630` and `:832` safe.
- Seat-range checks on `current`, `starting`, `winner`, every `marker` owner, and both `Pending` variants' `actor`.

This is materially better than the crates in pattern 2b, which each missed the one cross-field invariant their remaining panic depended on. See Coverage gaps for what it still does not check.

### No char/byte class regression (Unit 01's concern)

`rg 'chars()|byte|&<ident>['` over `rust/game/hanamikoji-1/src/lib.rs` returns **zero** hits. The crate does no string slicing at all; all input handling is delegated to `command.rs`'s parser. `test_multibyte_and_hostile_input` (`:1318`) drives NBSP, ideographic space, precomposed e-acute, emoji and a combining acute through `command()` and asserts `GameError::InvalidInput` for each. `test_garbage_command_is_user_error` (`:1118`) covers the non-Unicode half. `c882d413`'s multibyte test is real, not a decoy.

### No `unwrap`/`expect`/`panic!`/`unreachable!` in non-test code

Zero occurrences below `:878`. All 30-odd hits are inside `mod tests`.

### `Log::public` content carries no hidden information (pattern 3)

All 13 public and 3 private log sites enumerated. Card **identities** reach `Log::public` at exactly one place, `:285-289` (`" revealed a secret "` + the geisha), inside `score_round` - the rules require the secret to be revealed at end of round, so this is public by rule. Every other identity-bearing log is `Log::private` targeted at the owning seat: the drawn card (`:234-237`), the secret as played (`:399-405`), the two trade-off cards (`:438-445`). The gift/competition logs (`:480`, `:514`) name cards that the rules place face-up. The draw log (`:228-233`) exposes only `deck.len()`. **No pattern-3 leak.** Note this required checking the log layer, which per `00-STATE.md` no other game crate's tests do - and hanamikoji-1's tests do not check it either (see Coverage gaps), so this is a manual verification, not a tested property.

### `pub_state` is structurally redacted, and `player_state` is bounds-safe

`PubState` (`:72-109`) omits `deck`, `hands`, `secret` and `traded` **by construction** - it exposes `deck_remaining: usize`, `hand_counts: Vec<usize>`, `has_secret: Vec<bool>`, `traded_counts: Vec<usize>`. This is the strongest form of the D-33 pattern (no `Option`-blanking that a future field can bypass). `pending` is carried through, but both `Pending` variants hold only face-up cards by rule. `player_state` (`:771-779`) uses `.get(player).cloned().unwrap_or_default()` for all three private fields rather than raw indexing - correct hardening for an out-of-range seat.

### `render.rs` is not a leak vector - full field enumeration

`rust/game/hanamikoji-1/src/render.rs` has exactly two `pub fn` (`geisha_node:10`, `comma_geisha:17`); everything else is private. The single `fn render` (`:186-261`) takes hidden data only through four `Option` parameters. `impl Renderer for PubState` (`:263-267`) passes `None, None, None, None`, and every hidden-data branch is inside `if player.is_some()` (`:230`). `impl Renderer for PlayerState` (`:269-278`) passes only `self.hand`/`self.secret`/`self.traded` - the viewer's own fields, never an opponent-indexed read. Everything drawn for other seats comes from count/boolean `PubState` fields. `pending_nodes` (`:153-183`) renders the gift/competition cards ungated, which is **correct**: those cards are placed face-up by the rules and `lib.rs:470-480` / `:502-514` already log them via `Log::public`. All lookups use `.get(..).unwrap_or(..)`, so a short vector renders a placeholder rather than panicking. No pattern-2 gap in this file - the hardening is uniform.

### `a99bf754` is a genuine bug fix, correctly done

The one-character change (`exec "$SCRIPT_DIR/rust-ci-commands.sh"` -> the same without `exec`, `scripts/rust-test.sh:69`) fixes a real leak: `trap cleanup EXIT` is registered at `:19`, but `exec` **replaces the bash process**, so the trap could never fire and both `docker run -d` containers (`:23`, `:30`, uniquely named `$$`) survived every local test run. Exit status is preserved - `set -euo pipefail` (`:10`) is in force, the child is the last command so its status becomes the script's, and the EXIT trap does not call `exit`, so it cannot mask a failure (`cleanup`'s `|| true` affects only the trap body). Verified good, no caveat.

### `3f52d2b7` is correctly scoped - not a prod fail-open

Full `rg 'ALLOW_INSECURE_DEFAULT_KEY'` across the repo: the only setters are `scripts/rust-test.sh:64`, `.github/workflows/ci.yml:56` and `:152`, and `k8s/dev/web-patch.yaml:18-19`. **Zero hits under `k8s/base/**` or `k8s/prod/**`.** The commit reuses the existing house gate rather than introducing a new one, and both readers (`rust/web/src/crypto.rs:60`, `rust/web/src/main.rs:42`) still fail closed without it. This is consistent with the F-96 out-of-band conclusion and does not weaken WP-35's fail-closed key work. Verified good.

### hanamikoji-1's tests actually run in CI

`scripts/rust-ci-commands.sh:27` is `cargo test --workspace --exclude web`, invoked by both `scripts/rust-test.sh:69` and `.github/workflows/ci.yml`. The crate is a workspace member, so all 27 unit tests plus `tests/contract.rs` execute. `tests/contract.rs` calls `assert_gamer_contract::<Game>()` from `brdgme_cmd::test_support`, correctly wired as a `[dev-dependencies]` feature (`Cargo.toml:16-17`) - consistent with Unit 10a's discharged `test_support` carry-forward. `[lints] workspace = true` is present (`Cargo.toml:19-20`), so WP-73's sweep holds for this crate too.

## Coverage gaps

1. **No test exercises `Log::public` content for hidden information** - in hanamikoji-1 or anywhere. The crate's log layer is clean (verified by hand above), but nothing pins it: adding a geisha to the `" drew a card"` log at `lib.rs:228-233` would break no test. This is systemic pattern 3 and hanamikoji-1 does not fix it. Recommended: a test that renders every log from a full playthrough through `Log::public` only and asserts the rendered text contains no geisha name that the acting player did not place face-up.

2. **`test_redaction` (`:1104-1115`) tests the opening position only.** It calls `Game::start(2, 1)` and immediately asserts `hand_counts`, `deck_remaining`, `has_secret`, `traded_counts` and that `player_state(0).hand == g.hands[0]`. It never reaches a mid-round state, never checks `pending` redaction, and - most importantly - never serializes `PubState` and searches the output for a card identity, so it would not catch a future field addition that carries one. It is the strongest redaction test seen in the session and still only a structural smoke test. (WP-10 3a's "every game crate" gap - 13 crates with no redaction test at all - is untouched by this unit.)

3. **`test_validate` (`:1079-1101`) tests only length and range rejections** - `current`, `marker.len()`, `hands.len()`, `players`, `winner`. It asserts nothing about acceptance of a *valid* mutated state, and covers none of the cross-field invariants in F-209. Being the session's only validate test, it is worth naming in the unified report both as the model to copy and as evidence that having one is not sufficient.

4. **No test reaches sushi-go-2's new `unreachable!()`** (`draw_count`, `:145`); `test_draw_counts` (`:1279-1285`) covers 2/3/4/5 only. Per F-210 this is the change most in need of a test and has none.

5. **Nothing links the three delivery lists.** `rust/Cargo.toml`'s game members, `rust/Dockerfile`'s stages and `docker-bake.hcl`'s matrix are three hand-maintained lists with no cross-check, plus `k8s/base/game/kustomization.yaml` as a fourth (spanning two repos' Dockerfiles). F-208 is the first miss; nothing prevents the next. A CI guard is a remediation item, not just a fix for hanamikoji-1.

6. **The e2e suite is `continue-on-error: true`** (`.github/workflows/ci.yml:148`), so none of its assertions gate anything. Group with F-211 and the F-96 deployment-checklist family.

7. **`hanamikoji-1` was never subject to any work package**, so it inherits none of the programme's fixes by construction and none of its checklists apply. It happens to have landed in good shape (validate override, validate test, epilogue gate, structural redaction, multibyte test - better than most remediated crates), which is itself a data point for the unified report: the crate written *after* the review internalised the review's lessons better than the crates the review remediated. Its two real gaps (F-208, F-209) are both in areas no checklist covered.
