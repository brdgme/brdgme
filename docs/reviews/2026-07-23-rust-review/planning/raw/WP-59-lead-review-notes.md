# WP-59 lead review notes (adversarial verification + in-place repair)

Reviewed: `planning/specs/WP-59-inbound-processing-quality.md` (2538 lines at
start of review).
Method: READ-ONLY verification against live source at
`/home/beefsack/Development/brdgme`. No cargo/build/test run. No file under
`rust/` created, modified or deleted. No git mutation.

**Verdict: ACCEPT-AFTER-MY-REPAIRS, with ONE item requiring a user decision
(D-15).**

The spec's *engineering judgement* is unusually good — every re-derived
overturn/adjustment I independently re-derived came out the same way, and the
D-15 collision claim is true. Its *citation hygiene* is the weakest of the three
specs reviewed this session: **~215 file:line citations checked, 83 distinct
wrong (~39%)**, including three that would have produced non-compiling code and
one behavioural regression against an existing test. All repaired in place.

---

## 1. Load-bearing judgement re-derivations (independent)

### F21 — premise refutation: CONFIRMED
- `crate::error::internal` is live at `rust/web/src/error.rs:6-12` (the `#[cfg]`
  on :6, body :7-12; the spec said :7-13). Body is exactly
  `tracing::error!("{}: {}", context, e); ServerFnError::new("Internal server error")`.
  So it **does** log and **does** redact. The finding's "emailed verbatim and
  unlogged" is false on both counts. **Spec's ADJUSTED verdict is right.**
- Enumerated every error site in `restart_core` (`game/server_fns.rs:986-1158`,
  not :986-1155) with
  `awk 'NR>=986 && NR<=1160 && (/internal\(/ || /ServerFnError::new/ || ...)'`.
  All 12 `internal(...)` sites and all 6 `ServerFnError::new` sites match the
  spec's classification. **Six of the table's row line numbers were wrong**
  (:1003-1005, :1019, :1022, :1045-1047, :1064, :1079) — repaired to :1002-1003,
  :1018, :1021, :1044, :1050, :1065.
- `ServerFnError`'s `Display`: `server_fn-0.8.13/src/error.rs:234` inside the
  `ServerError(s) =>` arm renders `format!("error running server function: {s}")`.
  **CONFIRMED, cited correctly (:233-234).** The `#[deprecated]` on
  `WrappedServerError` is at :171-177 with the variant at :178 — the spec's
  ":171-178" is fine and the `other =>` catch-all rationale is sound.
- `ServerFnError::new` -> `Self::ServerError(msg.to_string())` (registry
  :199-203), so the classifier's `ServerFnError::ServerError(msg)` pattern is
  reachable and correct.

### F23 — attempt-cap bypass: CONFIRMED
`validate_confirmation_code` (`auth/server.rs:354-389`, cited correctly):
- selects `login_confirmations` **by email** (:361-369);
- the `attempts` bump is inside `if confirmation.code != token` (:378-387), the
  `UPDATE` itself at :379-385. The spec cited ":376-386" — repaired to :378-387.
So selecting the row **by code** (the finding's fix) would mean the code always
matches -> the bump never fires, and a wrong code finds no row -> also no bump.
`CONFIRM_MAX_ATTEMPTS_PER_CODE = 10` (`auth/server.rs:38`) would stop protecting
the email path. **The spec's re-derivation is correct and the finding's
recommendation is genuinely a security regression.**

### F28 — no cursor: CONFIRMED
`find_active_turn_games` (db.rs:3101-3119) is
`ORDER BY gp.is_turn_at ASC NULLS LAST` (:3112) `LIMIT $2` (:3113), with no
`OFFSET` and no keyset predicate. A second `bump` re-sends the same page.
**The finding's "reply bump again for the rest" wording is factually wrong and
the spec is right to reject it.** Spec's `bump_reply` internal line numbers were
all off by ~3 (fetch at :455 not :459, `cap_digest` at :458 not :461) — repaired.

### F9 — AppState field rejection: CONFIRMED, reasoning holds
- `AppState` is `state.rs:6-16` and is ssr-gated at `lib.rs:18-19` (spec said
  :19-20), so a field *would* compile — the spec correctly does not use that as
  the reason to reject.
- `InboundEmailSource` is a trait (`inbound.rs:59-61`) and `AppState` derives
  `Clone` (state.rs:6) — a `dyn` field genuinely forces `Arc<dyn ...>`.
- All three construction sites verified exactly: `main.rs:85`,
  `tests/ssr_pages.rs:45`, `tests/websocket_hygiene.rs:41`. **All three correct.**
**Judgement upheld.**

### F14 — render.rs exclusion: CONFIRMED
`render.rs:227` is `let msg_id = format!("<{thread_id}@brdg.me>");` — a
Message-Id, and `render.rs:237` is
`"<mailto:unsubscribe@brdg.me?subject=unsubscribe>"` — the List-Unsubscribe
target (headers block :235-242). Neither is a reply address. `inbound.rs:1065`
`<game-{}@brdg.me>` is likewise a Message-Id. **The exclusion and its
WP-60/WP-58 ownership are correct.** All F14 citations were correct
(notify.rs:10-12, inbound.rs:882, :1191, :960/:1037/:1082).

### F26 — already-solved-in-db.rs: CONFIRMED, byte-exactness verified
- `db.rs:2838-2839` SQL vs `commands.rs:829`: byte-identical.
- `db.rs:2853`/`:2866`/`:2880` vs `commands.rs:853`/`:866`/`:879`: byte-identical
  including the dead `updated_at = NOW()`.
- Existing coverage: **two** tests, `email_prefs_default_all_true`
  (db.rs:6821-6836) and `set_email_prefs_toggles` (db.rs:6838-6876). The spec
  cited a single test at ":6845-6872" — wrong; repaired.
- Only two new helpers needed. **Spec's scope reduction is correct.**

### F6's "forgiving trailing noise": NOT a recorded user decision — label REFUTED
`rg -n 'F6|trailing noise|forgiv' planning/decisions-needed.md` returns only two
hits, both about a **seven-wonders** finding, not wfe F6. There is no decision
entry for it. The spec's **SKIPPED-BY-DECISION** label was a Worker's unilateral
call mislabelled as a user decision. Repaired to **REJECTED-WITH-REASON**, with
an explicit note that raising it as a user choice requires a new decision entry.
The underlying rejection reasoning (typo-swallowing vs `failure_report_header`
already naming the failing line) is sound and I did not disturb it.

---

## 2. D-15 / F29 verb collision — VERDICT

**The spec's claim is CORRECT in substance and WRONG in its single most-repeated
line number.**

Live evidence:
- `rust/web/src/email/commands.rs:1217` — `"end" => return run_end(ctx).await,`
  inside `match verb_lower.as_str() {` which spans **:1215-1246**.
  **`:1219` — the line the spec cited four times — is the `"restart"` arm.**
  The verb order is :1216 concede, **:1217 end**, :1218 undo, :1219 restart,
  :1220 rules, :1221 help/commands, :1222 new, :1233 bump, :1244 list.
- The game path is reached only at `commands.rs:1264`
  (`crate::game::execute_command`), after that `match`. The spec said :1265.
- `rust/game/acquire-1/src/command.rs:192-197` —
  `fn end_parser()` / `Doc::name_desc("end", "trigger the end of the game at the end of your turn", Map::new(Token::new("end"), |_| Command::End))`.
  **Reachability confirmed:** pushed into the top-level parser list at
  `acquire-1/src/command.rs:68` under `if self.player_can_end(player)`.
- `rust/game/starship-catan-1/src/command.rs:309-313` —
  `if self.can_end(player) { parsers.push(Box::new(Map::new(Doc::name_desc("end", "end the flight early", Token::new("end")), |_| Command::End))); }`,
  pushed into the same `OneOf` list. Its own test at
  `starship-catan-1/src/lib.rs:2467` asserts `g.command(0, "end", &players).is_ok()`
  — so it is a live, exercised, player-issuable move.

**Conclusion: `end` is a legal top-level move in acquire-1 and starship-catan-1
and is unreachable by email today. D-15's recorded basis ("no current collision")
is FALSE.**

**No other reserved verb collides.** Verified repo-wide, not just in
`command.rs`:
```
rg -on '"(concede|end|undo|restart|rules|help|commands|new|bump|list|subscribe|unsubscribe|name|colors|colours|theme|emails|settings)"' rust/game/
```
returns exactly 5 hits: acquire-1/src/command.rs:194, :196;
starship-catan-1/src/command.rs:311 (x2); starship-catan-1/src/lib.rs:2467 (a
test). And
`rg -n 'Token::new\("(<reserved set>)"' rust/game/ rust/lib/` returns only the
acquire-1 and starship-catan-1 hits. **`end` is the whole collision set.**

**This is a USER DECISION and I did not resolve it.** D-15 is in
`decisions-needed.md` as a *Recommendation*, not a Decision. Task 14 now carries
an explicit "DO NOT EXECUTE UNTIL THE LEAD CONFIRMS D-15" gate.

---

## 3. Snapshot drift — CONFIRMED exactly

`diff -u <snapshot> <live>`, exit codes checked:

| File | Spec claim | Measured |
|---|---|---|
| `email/inbound.rs` | empty, exit 0 | empty, **exit 0** |
| `email/notify.rs` | empty, exit 0 | empty, **exit 0** |
| `email/render.rs` | empty, exit 0 | empty, **exit 0** |
| `email/commands.rs` | 126 diff lines | **126** |
| `db.rs` | 606 diff lines | **606** |

Live line counts also match the spec's Architecture section: commands.rs 2259,
notify.rs 679, render.rs 553, db.rs 6877 (inbound.rs is 2014, not stated).

---

## 4. Assumption 1 — the test fixture EXISTS (spec was wrong to hedge)

Task 12 hedged on whether a `StandaloneCommandCtx` fixture exists and used two
placeholder names. Live:
- **`make_standalone_ctx_deps()` at `commands.rs:2096-2105`** returns
  `(crate::websocket::GameBroadcaster, async_nats::jetstream::Context)` by
  connecting to `$NATS_URL`.
- It is already used to build a `StandaloneCommandCtx` at `commands.rs:2112-2119`
  inside `bump_verb_is_case_insensitive` (:2107-2134).
- The spec's placeholders `nats_ctx_for_test()` and `GameBroadcaster::default()`
  **do not exist** anywhere in the crate.
- **`expect_user_err` (`commands.rs:1429-1435`) takes
  `Option<Result<CommandReply, CommandError>>`**, but `run_new_command` returns a
  bare `Result`. The spec's test **would not compile.**
- There is no seeded real game type in a `#[sqlx::test]` DB, so
  `"tic-tac-toe self-namer"` would fail at `resolve_game_type`, not at the branch
  under test.
Task 12's test rewritten in full against these facts.

db.rs test module: no `seed_game` helper, but `CreateGameOpts` game seeding is
already done at `db.rs:3343`, `:4642`, `:6407` (struct at `:824`) — so Task 11's
"if a helper exists" conditional is now resolved to a definite instruction.

## 5. Assumption 2 — `error.rs` is outside WP-59's declared paths, but collision-free

- `work-packages.md` WP-59 paths: `web/src/email/{inbound.rs,commands.rs,notify.rs,render.rs}`,
  `web/src/db.rs`. The spec adds `web/src/error.rs` (Task 9) and
  `docs/authoring/COMMANDS.md` (Task 14), and drops `render.rs`.
- **WP-41** (`WP-41-db-quality-pass.md`): only *reads* `error.rs:7` in its F45
  reasoning; no `Modify:` line targets error.rs. **No collision.**
- **WP-37** (`WP-37-admin-pass.md`): declares `pub const ADMIN_REQUIRED` at its
  spec line 135, and every `Modify:` line in the file targets
  `rust/web/src/admin.rs`. It does **not** touch error.rs. **No collision.**
  (WP-37 also cites `error.rs:6-12` and `error.rs:14-16` — i.e. WP-37 got the
  line numbers right where WP-59 had them wrong.)
- Both additions now flagged in a new "Declared-path note" row of WP-59's
  coordination table rather than absorbed silently.

## 6. WP-41 ordering claim — half REFUTED

WP-59 justified "WP-41 first" partly on *"WP-41's Task 1 header insertion ...
will conflict textually"*. **False.** WP-41's module doc is inserted at db.rs:1
(its spec lines 148-171 show the `//!` block), ~3100 lines from anything WP-59
touches; it merges cleanly.

The **real** collision points, verified against WP-41's accepted text:
1. Both append to the end of `mod tests` (WP-41 confirms `mod tests {` at
   db.rs:3140 and the final `}` at :6877; it adds 11 `#[sqlx::test]`s there).
2. **WP-41's Task 6 rewrites `delete_expired_unverified_emails`' body at
   :3128-3136**, leaving its closing `}` at :3137 — the line *immediately above*
   WP-59's helper insertion point. Adjacent hunks, so git's 3-line context can
   report a conflict even though the edits are disjoint.
3. WP-41's `updated_at` sweep items 20/21/22 do edit db.rs:2852/:2866/:2880 (the
   three `set_user_*_emails_enabled` setters). WP-59's claim that this is
   collision-free **is correct** — WP-59 only adds *callers*.
The ordering conclusion ("WP-41 first") stands; the rationale is repaired.

---

## 7. Every repair applied, with reason

### Compile-breaking / behaviour-breaking (4)

1. **Task 4, delete range `:227-241` -> `:227-239`.** Live: `error_reply_text`
   body is :227-238, blank :239, and **`failure_report_header`'s doc comment
   starts at :240** (not :242 as the spec said). Deleting :240-241 would remove
   the first two lines of that `///` block, leaving a dangling doc comment — a
   compile error. Also corrected the "following item is the doc block (:242-251)"
   claim to ":240-251".
2. **Task 13, body replace range `:455-476` -> `:455-474`.** `bump_reply`'s
   closing brace is at :475 and the blank line at :476. The spec's range deletes
   the brace while the replacement text supplies no brace — the file would not
   compile. Added an explicit "leave :475 in place".
3. **Task 9, classifier insertion point "after :24" -> "after :25".**
   `CommandError`'s closing brace is at :25 (`#[derive]` :19 ... `}` :25).
   Inserting after :24 puts an `fn` inside an enum body.
4. **Task 2, rule 1 rewritten in full.** The spec's rule ("a non-quoted line
   whose immediately following line is `>`-quoted is an attribution") **breaks
   the pre-existing `parse_reply_commands_strips_quoted_lines` test**
   (inbound.rs:1226-1230, input `"play d4\n> previous move was e4\n> another quote"`):
   `play d4` is itself immediately followed by a quoted line, so the rule returns
   `[]` instead of `["play d4"]`. The spec explicitly (and wrongly) claimed all
   9 pre-existing tests pass. Replaced with a **block-retraction** rule that I
   hand-traced against all 9 pre-existing and all 7 new tests:
   *at the first `>`-quoted line, retract the block of collected lines since the
   last blank line iff any of them is attribution-shaped (ends with `:` or
   carries `<...@...>`)*. This also preserves the `continue`-not-`break`
   semantics on quoted lines (the spec's version silently changed nothing there,
   but a naive fix would). Full replacement body written out; two extra
   regression tests added (`_keeps_a_command_typed_below_a_quote_block`,
   `_does_not_retract_a_command_directly_above_a_quote`); checkpoint count
   9+5=14 -> 9+7=16.

### Missing step that would break the build (1)

5. **Task 11 missed two call sites of the fn it deletes.** Task 11 deletes
   `commands.rs`'s private `set_turn_emails_enabled` (:848-859) and listed four
   call sites (:637, :653, :669, :1248). Live, there are **six**: the test
   `subscribe_unsubscribe_toggles_turn_emails` calls it at **:1338** and
   **:1349**. Without those two renames the crate does not compile. Added them,
   corrected :1248 -> :1249 (the setter call; :1248 is the `if let`), and updated
   Global Constraints from "exactly two sanctioned test edits" to three,
   documenting that the test's *assertions* (:1342, :1351) are untouched and the
   SQL executed is byte-identical.

### Test that would not compile / would not reach the branch (1)

6. **Task 12's `#[sqlx::test]` rewritten.** See §4. Now uses the real
   `make_standalone_ctx_deps()`, matches on `Result` directly instead of the
   wrongly-typed `expect_user_err`, seeds its own single-word game type, and
   explains why the bogus `http://127.0.0.1:1` service URI is never dialled (the
   error returns at :382, before `create_game_from_service` at :407).

### Missing instruction that risks losing an edit (1)

7. **Task 1's `select_route` replacement range `:178-182` -> `:176-182`.** The
   replacement block supplies a new 4-line doc comment, but the existing doc
   comment lives at :176-177, outside the stated range — following the spec
   literally leaves a duplicated/stale doc comment above the new one.

### Citation corrections (75 further distinct fixes)

Applied across Architecture, Non-Goals, the coordination table, the
stale-citation map, the disposition table and every task. Grouped:
- `inbound.rs`: ResendInboundData :166-176->:166-174;
  `resolve_user_by_verified_from` :393-406->:393-404 (3 sites);
  `header_value` :418-425->:418-423; the three fetch blocks
  :516-544/:625-644/:1088-1108 -> :518-535/:625-642/:1088-1107 (and the two
  step-level ranges :516-537->:518-536, :625-643 confirmed);
  `GameCommandLoopOutcome` :278-297->:278-296; `CommandLoopOutcome`
  :184-188->:184-187; `error_reply_text` :227-240->:227-238;
  `failure_report_header` :252-276->:252-275; its loop :317-343->:318-344;
  settings loop :1148-1170->:1153-1170; `send_rendered_email`
  :968/:1085/:1213 -> :962/:1084/:1214 (2 sites each);
  `event_type` guard end :474->:475 and the `match` :476->:477 / :476-487->:477-487;
  accept/decline :645-646->:646-647 (3 sites), guard :648->:649,
  return-brace :658-659->:659-660, and the four `accept` consumers
  :722/:735/:806/:810/:817 -> :722/:733/:804/:809; lock :670->:671;
  "no longer open" branch :682-692->:683-694 (3 sites);
  `update_proposal_player_response` :722->:723-724; the eight logged-only
  early returns :684/:697/:737/:745/:749/:756 -> :674/:678/:699/:738/:747/:751/:758;
  the folded lookup :841-855->:841-857 (2 sites); subject :872->:867;
  `rules_url` :870->:873; `resend_webhook` closing brace :488->:489;
  select_route tests :1409-1441->:1408-1441; the unroutable-OK returns
  :465-474/:481-486 -> :469/:474/:488; `verify_webhook` unwraps
  :144-152->:144,:145,:146; `mark_event_processed` :456-464->:456-463 (2 sites);
  `send_invite_reply_response` call-site list made explicit (:650, :684, :710,
  :819) with a note that :704-707 sends nothing.
- `commands.rs`: the `"end"` arm :1219->:1217 (4 sites, with an explicit warning
  that :1219 is `restart`); verb match :1216-1245->:1215-1246 (3 sites) and the
  insertion anchor :1216->:1215; `execute_command` :1265->:1264; error mapping
  :1276-1283->:1275-1281; self-mention :383-385->:382-384 (5 sites); Bot arm
  :367-377->:367-376 (2 sites); `roster_error` :400-402->:398-400;
  `creator_id` :414->:413; `run_new_command` :337-441->:337-443; `bump_reply`
  :449-476->:449-475, fetch :459->:455, `cap_digest` :461->:458 (3 sites), reply
  match :471-475->:470-474; prefs fetch :826-834/:828-833->:827-834;
  emails_confirm test span :1825-1900->:1825-1902 and
  `emails_confirm_verifies_address` :1825-1862->:1825-1863.
- `db.rs`: `SWITCH_DIGEST_CAP` :2905/:2903-2906->:2906; the email-prefs test
  ":6845-6872" -> the two real tests :6821-6836 and :6838-6876 (3 sites);
  order-by cite :3107-3114->:3112 + :3113; `delete_expired_unverified_emails`
  boundaries spelled out.
- `game/server_fns.rs`: `restart_core` :986-1155->:986-1158 and the six wrong
  table rows (above); `create_game_from_service` internals :659,:664->:658,:662.
- `proposals.rs`: :918->:919 (and added the missed :904).
- `auth/server.rs`: :376-386->:378-387 (2 sites); the three genuine-failure
  returns :372/:374/:380 -> :372/:376/:386; the two DB `internal` sites
  :367/:387 -> :368/:385.
- `error.rs`: :7-13->:6-12 (3 sites); :15-17->:14-16.
- `lib.rs`: email gate :39-40->:35-36; state gate :19-20->:18-19.
- `web/Cargo.toml`: ssr feature list :120->:118 (2 sites).
- `mail-parser-0.11.5`: `address.rs:126-148` -> `address.rs:11` (`Address::first`)
  and `:145` (`Addr::address`); added the verified
  `MessageParser::parse` signature from `parsers/message.rs:111`
  (`&'x (impl AsRef<[u8]> + ?Sized)`), which is what makes both
  `.parse(raw.as_bytes())` in Task 1 and the existing `.parse(raw)` in
  `extract_plain_text` legal.
- `notify.rs`: test :501-503->:500-503.

### Substantive / labelling repairs (6)

8. F6 sub-recommendation relabelled SKIPPED-BY-DECISION -> **REJECTED-WITH-REASON**
   with evidence that no such decision is recorded; disposition counts updated
   (3 -> 4 overturned/rejected recommendations, 0 skipped-by-decision).
9. Task 14 gated on D-15 with an explicit "D-15 IS STILL OPEN, do not execute
   until the Lead confirms" and the clarification that
   `decisions-needed.md` records a *Recommendation*, not a Decision.
10. Coordination table: WP-41 rationale corrected (module-doc collision claim
    withdrawn; the two real collision points substituted, one of them —
    WP-41 Task 6's rewrite of the immediately-preceding function — newly
    identified in this review).
11. Coordination table: two new rows for `rust/web/src/error.rs` (with the
    verified WP-41/WP-37 non-collision evidence) and a "Declared-path note" row
    recording the scope delta vs `work-packages.md`.
12. Task 9: removed the `#[cfg(feature = "ssr")]` from `classify_server_fn_error`
    — `rg -c 'cfg\(feature = "ssr"\)'` returns **zero** for commands.rs,
    inbound.rs and notify.rs; the whole module is gated at lib.rs:35-36, so the
    attribute would be the only one of its kind in the file.
13. Task 11: the "if a game-seeding helper exists" conditional resolved to a
    definite instruction (`CreateGameOpts` at db.rs:3343/:4642/:6407); the
    WP-41 adjacency warning added at the insertion point; drift-check
    confirmation added to the Snapshot drift section.
14. Task 2: added the colon-safety re-verification
    (`rg -n 'Token::new\("[a-z]+:'` over `rust/game/*/src/{command,lib}.rs`
    returns nothing) and a `block_start` no-panic argument, plus the two accepted
    limits recorded in the doc comment.

---

## 8. Gaps NOT closed (left as-is, flagged)

1. **D-15 / F29 — USER DECISION.** The collision is proven; the choice between
   option A (document), option B (escape prefix), renaming the email verb
   (`end game`), or renaming the two game verbs is not mine to make. Task 14
   is gated, not resolved.
2. **Task 6 has no automated test, by design.** I verified the spec's stated
   reason: `handle_invite_reply` has zero test coverage, there is no `AppState`
   fixture in `rust/web/tests/` for the email handlers, and asserting
   "lock released before send" needs a second racing connection. I agree with
   the omission but it remains a coverage gap. Already routed as cross-package
   item 6.
3. **Task 13's capped-branch test remains conditional/skipped.** Seeding 21
   games via `create_game_with_users` + `CreateGameOpts` is genuinely
   disproportionate for a nit; the spec's "verify by inspection, record in the
   commit body, do not lower `SWITCH_DIGEST_CAP`" instruction is the right call
   and I left it. Note this is the one place the package ships an unverified
   new branch.
4. **Whether the spec should own `web/src/error.rs` at all.** It is outside
   WP-59's declared paths. Collision-free, one const, behaviour-identical — but
   the scope call is the Lead's. Flagged in the coordination table, not changed.
5. **`run_emails_confirm`'s new N-address attempt-bump side effect.** The spec
   documents it and I confirmed it follows necessarily from keeping
   `validate_confirmation_code` in the loop. It is a real, if minor, behaviour
   change to a rate limiter; if the Lead considers rate-limit semantics
   user-visible, it may warrant a decision. Not invented a resolution.

## 9. Claims REFUTED

| Claim | Live evidence | Status |
|---|---|---|
| `"end"` arm at `commands.rs:1219` | :1217; **:1219 is `"restart"`** | REFUTED (line only; the collision itself is confirmed) |
| Task 2's rule 1 keeps all 9 existing tests passing | `parse_reply_commands_strips_quoted_lines` (inbound.rs:1226-1230) fails | REFUTED, rule rewritten |
| Deleting inbound.rs:227-241 leaves `failure_report_header`'s doc block intact | that block starts at :240 | REFUTED |
| `bump_reply`'s body is :455-476 | :455-474; :475 is the fn's closing brace | REFUTED |
| Insert the classifier "after :24" | `CommandError` closes at :25 | REFUTED |
| Four call sites of `set_turn_emails_enabled` | six (adds :1338, :1349 in tests) | REFUTED |
| WP-41's top-of-file module doc collides with WP-59's db.rs additions | WP-41 inserts at db.rs:1; ~3100 lines away | REFUTED |
| F6's trailing-noise skip is a recorded decision | no entry in `decisions-needed.md` | REFUTED |
| Task 12's test can use `expect_user_err(run_new_command(...))` | `expect_user_err` takes `Option<Result<...>>` | REFUTED |
| `nats_ctx_for_test()` / `GameBroadcaster::default()` are candidate fixtures | neither exists; `make_standalone_ctx_deps()` (:2096) does | REFUTED |
| `mail-parser` `Address::first`/`Addr::address` at `core/address.rs:126-148` | :11 and :145 | REFUTED |
| Whether a db.rs game-seeding fixture exists (spec hedged) | `CreateGameOpts` seeding at :3343, :4642, :6407 | RESOLVED (no longer conditional) |

## 10. Newly discovered defects and routing

**A. `auth/server.rs` has the identical `cap_digest`-after-`LIMIT` redundancy
and the identical missing cap disclosure.** `auth/server.rs:884` calls
`find_active_turn_games(&pool, user.id, SWITCH_DIGEST_CAP)` then
`cap_digest(games, SWITCH_DIGEST_CAP)` at `:887`. `cap_digest` can never remove
anything there either, and that path also cannot tell a full page from a
truncated one. This *closes* WP-59's open cross-package item 5 (which only said
"worth checking"). **Owner: the auth package (WP-34/WP-35/WP-36).** Fix is the
same `+ 1` shape as Task 13. Recorded in the spec.

**B. `proposals.rs:904` is a fourth `internal(...)` site inside
`find_or_create_user_by_email_tx`** that the spec's Task 9 table omitted
(it listed :896, :911, :918). Does not change the classification (all four are
`internal`) but the table is now complete. No routing needed.

**C. Cross-package items 1-4 and 6 as written are all verified accurate** and
stay routed as the spec has them: (1) D-15 verb collision -> USER DECISION;
(2) `ServerFnError` Display leak sweep -> WP-54, corroborated by WP-37's own
AdminPage finding; (3) `SELECT game_version_id FROM games WHERE id = $1` at
`game/server_fns.rs:2333` and `:2375` -> WP-40/WP-53 (both line numbers
confirmed exact); (4) `DELETE FROM login_confirmations WHERE email = $1` with
the `query!` macro at `auth/server.rs:486` and `:850` -> auth packages (both
confirmed exact); (6) no `AppState` email-handler test fixture -> backlog /
WP-57.

## 11. Three load-bearing spot-check anchors for the Lead

1. `/home/beefsack/Development/brdgme/rust/web/src/email/commands.rs:1217`
   must read `        "end" => return run_end(ctx).await,`
   (and **:1219** must read `        "restart" => return run_restart(ctx).await,`).
   This is the D-15 anchor; the draft had it at :1219 four times.
2. `/home/beefsack/Development/brdgme/rust/web/src/email/inbound.rs:240`
   must read
   `/// Builds the header block for a command-failure report email. Layout (each a`
   — i.e. `failure_report_header`'s doc comment starts at :240, which is why
   Task 4's delete range had to stop at :239.
3. `/home/beefsack/Development/brdgme/rust/web/src/email/commands.rs:1338`
   must read `        set_turn_emails_enabled(&pool, user_id, false)`
   — inside the test `subscribe_unsubscribe_toggles_turn_emails`. This is one of
   the two call sites Task 11 originally missed; if it is not there, re-derive
   Task 11's call-site list before implementing.

## 12. Compliance statement

- **No file under `rust/` was created, modified or deleted.** Every interaction
  with `rust/` was `sed`, `rg`, `awk`, `diff`, `ls`, `wc` or the `Read` tool.
- **No cargo, build or test command was run** — no `cargo build/check/test/
  clippy/fmt/sqlx`, no `scripts/rust-test.sh`. All validation is by reading
  source, including the `~/.cargo/registry` sources for `server_fn-0.8.13` and
  `mail-parser-0.11.5`.
- **No git mutation was run** — no `add`, `commit`, `checkout`, `stash`,
  `reset`, `clean` or `rm`. The only git-adjacent command was none at all; drift
  was measured with `diff` against the snapshot working tree.
- Files written: this notes file, and edits confined to
  `planning/specs/WP-59-inbound-processing-quality.md`.
