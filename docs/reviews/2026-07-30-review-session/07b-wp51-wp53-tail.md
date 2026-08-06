# Unit 07b - WP-51 `dcd8844c` + WP-53 `3610b957` (Unit 07's unexamined tail)

Findings continue from F-144.

## Progress

- [x] W1 (prior attempt): criteria recovered into a now-stale scratchpad; re-recovery
      dispatched.
- [x] Lead read of the WP-51 end state: `email/notify.rs`, `email/sweep.rs`,
      `proposals.rs:100-720` (the `InviteMailer` trait + all six `RealInviteMailer`
      methods) and the invite-nudge sweep.
- [x] W1 re-run: recon complete (`w1-recon.md`, 2948 lines, this session's
      scratchpad). WP-51 spec = 7 tasks; WP-53 = 12 checklist rows, no spec file.
- [x] W2: WP-51 diff attribution, revert check, and spec tasks 1/5/6 verified
      (`w2-wp51.md`, scratchpad). **No pattern-4e revert** - see F-147.
- [x] W3: WP-53 `3610b957` verified against all 12 checklist rows
      (`w3-wp53.md`, scratchpad). 10 PASS, 2 PARTIAL, 0 FAIL.

**Unit complete.** 6 findings, F-144..F-149: 1 High, 3 Medium, 2 Low.

**WP-51 attribution summary (W2, evidenced by `git log -S`):** of the four defects
below, WP-51 (`dcd8844c`) **introduced none**. F-144/F-145 come from `69bcd1e`
(WP-46, "sweep delivery semantics"); F-146 from the original #24 invite work
(`4bd3135`/`db8f4b6`/`b88ff26`). They are recorded here because this unit is the
first review pass over that code and because WP-51 **edited the exact lines** of
F-145 without fixing them. F-147 is WP-51's own.

### Correction to the brief (carried from the prior attempt, re-confirmed)

**WP-51 DOES have a spec file**:
`docs/reviews/2026-07-23-rust-review/planning/specs/WP-51-invite-mailer-notify-dedup.md`
(recover at `868094a6`). `00-STATE.md` says WP-51/WP-53 have no spec; that is true
only for **WP-53**. WP-51 is a Tier-2 spec package and owns no `T3-B*` row at all.
WP-51 acceptance criteria = `wd F8 F32 F33 F34` + `wfe F36 F38 F39 F41 F42 F43`.

Recon fact worth carrying to Units 08-11: `SUMMARY.md` at HEAD is a narrative
compaction carrying **no** `wd Fnn`/`wfe Fnn` ids or per-finding text; the finding
text only survives in `868094a6:.../findings/*.md`.

### WP-51 acceptance criteria (7 spec tasks, recovered verbatim in `w1-recon.md` s11)

1. `execute_command` returns the pre-command `GameExtended` snapshot it already
   loads (wd F8, wfe F42).
2. `notify_game_emails`: one game load + one honest `game_log_count` per batch;
   `send_one` moves the count before the recipient gate; calls
   `try_send_rendered_email` (wfe F41, F43). A concurrent send loop was explicitly
   rejected by the spec.
3. `send_reminder` body **deleted** in favour of `send_one`; `bool` contract
   preserved arm-by-arm; fixes threading (wfe F36, F39).
4. A single sweep-spawning helper replacing five copy-pasted interval loops
   (wfe F38).
5. `notify_owner_decline` gated like the other invite mailers (wd F32).
6. Dead Reply-To / "reply to this email" footers replaced in `proposals.rs`
   (wd F33); keep the word "unsubscribe".
7. Mailer tasks log DB errors and stop blank-name substitutions (wd F34).

Fenced off by the spec (do not judge WP-51 on these): `outbound.rs`/`render.rs`
(WP-60), `sweep.rs` candidate/mark fns (WP-46), the three lifecycle server fns
(WP-40), `notify.rs:8-12` (WP-59).

## Findings

### F-144 (High) - the invite-nudge dedup key is per-proposal but the send is per-invitee, so one suppressed invitee re-nudges the whole roster every tick

`rust/web/src/email/sweep.rs:507-519`, with
`rust/web/src/proposals.rs:973-992` (`fetch_nudge_candidates`) and `:995-1004`
(`mark_proposal_nudged`).

**What the dedup key actually is.** `fetch_nudge_candidates` selects one row per
*pending invitee* (`JOIN game_proposal_players`), but its dedup predicate is
`gp.nudged_at IS NULL` - a column on `game_proposals`, i.e. **per proposal**. The
only writer of that column in the sweep path is `mark_proposal_nudged`, and
`sweep_invite_nudge_once` calls it only when every invitee of that proposal
succeeded:

```
*all_sent.entry(c.proposal_id).or_insert(true) &= ok;
...
for (pid, sent) in &all_sent { if *sent { mark_proposal_nudged(pool, *pid).await; } }
```

**The collision.** `send_invite_now` -> `send_invite_core`
(`proposals.rs:257-345`) returns `false` - deliberately, per its own doc comment
and D-02 at-least-once - when the invitee is suppressed by web presence
(`:271-275`) or when the Resend send fails (`:344`). So a **single** invitee who
happens to be active on the web at sweep time makes `all_sent[proposal] == false`,
`nudged_at` stays NULL, and on the next tick **every other pending invitee of that
proposal is a candidate again and is mailed an identical nudge**. There is no
per-invitee marker anywhere: `send_invite_core`'s only per-player guard is
`pp.response != "pending"` (`:291-296`), which a not-yet-responding invitee passes
every time.

**Why it matters.** The loop is self-sustaining and unbounded: default sweep
interval is 900s (`sweep.rs:11`) and default invite expiry is 14 days
(`sweep.rs:471-472`), so a proposal with one persistently-web-active invitee mails
each *other* pending invitee up to ~1,344 duplicate "invite from X" emails before
the proposal expires. Web presence is exactly the state a real invitee sitting on
the invite page is in, so this is not an exotic path. It is also self-reinforcing:
the recipients most likely to be suppressed are the engaged ones.

This is the failure the brief predicted for a send-dedup change, in its mirror
form: the fix did not drop legitimate sends, it **repeats** them, because the
retry unit (invitee) and the dedup unit (proposal) are different.

Fix: mark per invitee, not per proposal - add a `nudged_at` (or
`nudge_sent_at`) column on `game_proposal_players`, set it for each invitee whose
`send_invite_now` returned `true`, and add it to `fetch_nudge_candidates`'
predicate. Keep the proposal-level column only as a display field if it is needed.
A minimal stopgap that does not need a migration is to mark the proposal when
*any* invitee succeeded, which trades the storm for a dropped nudge - strictly
better than the current behaviour but still wrong.

### F-145 (Medium) - `send_invite_core` reports transient DB failures as "permanently unsendable", silently dropping the nudge

`rust/web/src/proposals.rs:257-296`, via `proposals.rs:180-208`
(`mailer_proposal` / `mailer_recipient`).

`send_invite_core`'s contract is stated in its own doc comment
(`proposals.rs:250-256`): `true` = sent or permanently unsendable, do not retry;
`false` = **only** on a transient condition. Three of its early returns violate
that contract by folding `Err(_)` into the same arm as `Ok(None)`:

- `:268` `let Some(recip) = mailer_recipient(...) else { return true; }` -
  `mailer_recipient` (`:196-208`) returns `None` for **both** "user not found"
  and "user lookup failed: {e}".
- `:282` `let Some(proposal) = mailer_proposal(...) else { return true; }` -
  same shape (`:180-192`).
- `:288` `let Ok(Some(pp)) = find_proposal_player_by_email_token(...) else { return true; };` -
  a `let-else` on `Ok(Some(_))` treats `Err(_)` identically to "token not found".

Consequence on the nudge path: one transient pool timeout or connection reset
during the sweep marks that invitee as successfully nudged, `mark_proposal_nudged`
commits, and **the nudge is never sent to anyone on that proposal** - the exact
mark-without-send failure that F-136 found in the sibling reminder sweep, arriving
here by a different syntactic route.

**Attribution, and why it is worse than it looks.** The `return true` fold
pre-existed (`69bcd1e`, WP-46, which introduced the `bool` contract).
`dcd8844c` (WP-51) then **rewrote these exact three lines** for its spec task 7
(wd F34, "mailer tasks log DB errors"):

```
-        let Ok(Some(recip)) = fetch_invite_recipient(pool, invitee_user_id).await else {
+        let Some(recip) = mailer_recipient(pool, invitee_user_id, "send_invite").await else {
             return true;
         };
```

wd F34's whole purpose was that inside a spawned task "a DB error is otherwise
indistinguishable from 'proposal deleted' and from 'recipient opted out'"
(`proposals.rs:176-178`, WP-51's own words). WP-51 built two helpers that
*observe* the distinction, logged it at the right level - and then returned
`Option<T>`, a type that **cannot carry it**, so every call site re-merges the
two one line later. The row is satisfied literally (the errors are now logged);
the property the row existed to establish (the caller can tell them apart) is not.
This is the session's canonical pattern, and it is the second time in this unit
that WP-51 shipped the observable half of a criterion and not the behavioural half
(cf. F-147).

The `let-else`/`_ =>` collapse of `Err` into a default is also **pattern 5**
again - third instance in the web half (F-65, F-136, F-145).

Fix: give `mailer_proposal`/`mailer_recipient` a three-way return (e.g.
`Result<Option<T>, ()>` or a small `Lookup` enum) and have `send_invite_core`
return `false` on the error arm. Same for the `find_proposal_player_by_email_token`
`let-else`.

### F-146 (Low/Medium) - five distinct proposal notifications share one subject and one thread id

`rust/web/src/proposals.rs:401`, `:478`, `:547`, `:615`, `:688` (subject) and
`:439`, `:518`, `:583`, `:651`, and the `notify_owner_ready` counterpart (thread
id).

`notify_changed_reinvite`, `notify_owner_decline`, `notify_cancelled`,
`notify_started` and `notify_owner_ready` all render with subject
`format!("{game_type_name} invite")` and thread id
`Some(format!("proposal-{proposal_id}"))`. The bodies are entirely different
events - "the owner changed the game, accept again", "X declined your invite",
"the invite was cancelled", "the game has started", "everyone accepted, ready to
start" - and several are *actionable*.

`notify.rs:88-94` documents the house rule for exactly this: "A unique subject per
turn is the reliable de-threading lever (Resend overwrites custom Message-Id)".
The turn path applies it (`turn_subject_or_fallback`); the proposal path does the
opposite and gives five distinct events one identical subject *and* an explicit
shared thread id. In Gmail/Outlook these collapse into one conversation with only
the newest line visible, so "the game has started" can arrive hidden under an
earlier "X declined your invite". `send_invite` is the only one that differs
(`:303`, `"{type} invite from {owner}"`), and it shares the same thread id.

Pre-existing (`4bd3135`/`db8f4b6`/`b88ff26`, the original #24 invite work); the
counts are 5 and 6 at `dcd8844c^` as at HEAD, and the lines appear in WP-51's diff
only as context. Recorded because WP-51's wd F33 work edited these very blocks and
this unit is the first review pass over them.

Capped below High because nothing is *dropped* - the mail is delivered, just
collapsed - and because threading behaviour is client-dependent. Fix: give each
event a distinguishing subject (the same treatment `send_invite` already gets),
and reserve the shared `proposal-{id}` thread id for the invite/reinvite pair that
genuinely is one conversation.

### F-147 (Medium) - wfe F36's dedup was consciously abandoned, but the code and the checklist both still claim it happened

`rust/web/src/email/notify.rs:523-543` and `rust/web/src/email/sweep.rs:105-232`.

WP-51 spec task 3 required `send_reminder`'s body to be deleted in favour of the
shared `send_one` pipeline (wfe F36). The Lead **explicitly decided not to do it**
and recorded why - `EXECUTION-STATE.md:172` @`868094a6`:

> WP-51 Lead decisions: Task 3 full dedup of sweep's send_reminder NOT done (would
> regress WP-46's ReminderOutcome/D-11 gating/tx tokens/unsubscribe wiring -
> threading bug wfe F39 fixed directly in sweep.rs instead, notify-side infra
> added)

The reasoning is sound and I am **not** faulting the decision: the reminder gate is
`reminder_emails_enabled` (`outbound.rs:171`, D-11/WP-46) whereas
`SendMode::Normal` gates on `turn_emails_enabled` (`outbound.rs:208-210`), so
routing reminders through `send_one` as-is would have silently changed which users
get reminders. Deviating was right. **What is wrong is everything the deviation
left behind.**

1. **`notify::send_turn_reminder` is dead code, and was dead at birth.**
   `git grep send_turn_reminder dcd8844c` and `rg` at HEAD both return exactly one
   hit: its own definition (`notify.rs:526`). It has never had a caller.
   `dcd8844c` added it - along with `NotifyKind::Reminder` and `SendResult` - and
   never wired it up. Its doc comment nonetheless states the dedup as accomplished
   fact: "Replaces the ~90-line copy of this pipeline that used to live in
   `email::sweep` (wfe F36)" (`notify.rs:523-525`). It replaced nothing. The
   `SendResult` doc is false the same way (`notify.rs:292-296`: "Only the
   turn-reminder sweep currently reads this (wfe F36)" - the sweep reads its own
   `ReminderOutcome`). A future reader grepping wfe F36 finds two comments telling
   them the finding is closed and a dead function to prove it.
2. **It is also a live trap.** `send_turn_reminder` uses `SendMode::Normal`, i.e.
   the *turn* opt-out. Any future caller that adopts this "shared" reminder helper
   silently applies the wrong gate.
3. **The duplication it was meant to remove is what F-136 (High) lives in.**
   `sweep.rs:134-138`'s `_ => PermanentSkip` has no counterpart in the
   `notify.rs:331-342` original, which distinguishes `Ok(None)` from `Err(e)`
   correctly. Two copies, one hardened, one not - the exact cost wfe F36 predicted,
   realised.
4. **The checklist records wfe F36 as closed.** `EXECUTION-STATE.md:91` lists
   WP-51 as done "wd F8 F32 F33 F34 + wfe F36 F38 F39 F41 F42 F43" and only the
   free-text parenthetical mentions the carve-out. Nothing routes wfe F36 onward -
   `00-STATE.md`'s **pattern 1, the routing leak**: the sending package closed a
   finding it explicitly did not fix, with no receiving package.

Fix: delete `notify::send_turn_reminder` (or give it a `SendMode::Reminder` arm
gated on `reminder_emails_enabled` and actually call it from the sweep), correct
the `notify.rs:523-525` doc comment, and reopen wfe F36 against WP-46's sweep copy
where it belongs - F-136 and F-147 are then one remediation item.

> Update (2026-08-06): `notify::send_turn_reminder` was already deleted (see
> `97-REMEDIATION-PROGRESS.md`), and `NotifyKind::Reminder` - the last piece of
> this notify-side infrastructure, still never constructed - was deleted by
> user ruling. Re-add it, or a `SendMode::Reminder` arm, only when a real
> reminder caller through this pipeline exists.

### F-148 (Medium) - WP-53's most load-bearing fix is the one of three that shipped without its required test

`rust/web/src/db/game_write.rs:739`, test gap in the same file's test module.

`wd F6` is one of only **three** WP-53 rows carrying "Test? **y**". The code half
is right and internally consistent: `is_eliminated = CASE WHEN $9 THEN
is_eliminated ELSE $3 END` with `.bind(status.is_finished)` at `:755`, and
`status_fields`' `Status::Finished` arm (`game/mod.rs:37-42`) still emits
`eliminated: vec![]` - which is now correct-by-construction, because the empty
list is never written on the finish path. Good fix.

**No test asserts the property.** The file's three elimination-aware tests are
`update_game_command_success_writes_active_fields` (`:874`, `:901-904`,
`is_finished: false`), `elimination_sets_left_at_once` (`:1291-1374`, both calls
`is_finished: false`), and `update_game_command_success_writes_finished_fields`
(`:972-1057`) - the only finish-path test, which passes `eliminated: vec![]`
(`:993`) and asserts `place`, `finished_at` and `is_finished` only
(`:1016-1017`, `:1046-1056`). It never sets `is_eliminated` true beforehand and
never asserts on it. Every other `is_finished: true` `StatusUpdate` in the crate
(`db/rating.rs:410,466,512,537,575,629,692,805`;
`game_write.rs:1840,2077,2203`) pairs with `eliminated: vec![]`.

**Deleting the `CASE WHEN $9` guard fails no test in the repository.** The exact
regression the row was written to prevent - eliminate a player while Active, run a
finishing command, assert `is_eliminated` survives - has zero coverage.

Explicitly **not** pattern 4b: no test was edited to agree with the code. This is
the plainer failure of simply not writing one, on the row where the checklist
demanded it. Worth naming separately in the process-fixes section, because it is
invisible to every check the session has proposed so far: the fix is correct, the
commit is clean, the checklist row is honestly closed, and the guard is one
careless refactor from silently disappearing.

Fix: a `#[sqlx::test]` that runs a command with `eliminated: vec![0], is_finished:
false`, then a second with `is_finished: true`, and asserts player 0's
`is_eliminated` is still true.

### F-149 (Low) - `wd F61`'s required test is also absent, and `friends.rs` has no test module at all

`rust/web/src/friends.rs:229-231`.

The code is correct and matches the shape the row prescribed:
`get_user(...).ok_or_else(|| ServerFnError::new("User not found"))?`, identical in
constructor and message to `send_friend_request`'s (`friends.rs:150-153`). The
TOCTOU that the resolve-then-insert shape invites is not reachable - there is no
`DELETE FROM users` anywhere in `rust/`, so the resolved user cannot vanish before
`db::block_user`'s insert (`db/social.rs:269-289`).

But `wd F61` is the second of the three "Test? y" rows, and **`friends.rs` (634
lines) contains no `#[cfg(test)]` module and no `#[sqlx::test]` at all**. Nothing
anywhere asserts the "User not found" message for `block_user`. The Lead's own
recorded decision (`EXECUTION-STATE.md:175`) covers only the *integration* test -
"wd F61 no integration test (server fns need full Leptos context; db layer already
tested)" - which is reasonable, but the db layer's `block_user` is not where the
new guard lives, so "db layer already tested" does not cover this row. Low because
the behaviour is a message string, not an invariant.

Only `wd F25` of the three test-required rows actually got one - and it is a good
one; see Verified good.

## Verified good

- `notify.rs:261-290` - `game_log_count` returning `Option<i64>` and
  `turn_subject_or_fallback` timestamping on `None` genuinely close wfe F41: a
  failed count de-threads via timestamp instead of collapsing every turn onto
  `{type} {id}-0`. The unit test at `:661-672` asserts the right property (that
  the fallback is *not* equal to the `-0` subject), not merely that the function
  returns something.
- `notify.rs:307-342` - `send_one_loaded`'s own recipient lookup is the
  three-arm form F-145 is missing: `Ok(Some)`, `Ok(None)` and `Err(e)` are
  distinguished, each with its own log line. The same author wrote both; only the
  invite path collapsed them.
- `notify.rs:344-356` - the three `SendMode` arms are explicit and each states
  which guards it bypasses; `Forced` still requires `email.is_some() && !is_bot`,
  so the `bump` command cannot mail a bot or an address-less account.
- `notify.rs:554-637` - `notify_game_emails`' `before: Option<GameExtended>`
  contract (wfe F42) is documented at the signature *and* pinned by a test
  (`:898-913`) asserting `None` means brand-new game. The finished branch returns
  early so a finishing move cannot also emit turn mails.
- `notify.rs:199-202` - the reminder's "no digest" rule (wfe F36) is implemented
  as an explicit `NotifyKind::Reminder => None` arm, and `Reminder` deliberately
  shares `Turn`'s subject so the nudge groups with the mail it nudges (wfe F39,
  documented at `:133-136`). This is the one place a shared subject is correct and
  it says why - which is what makes F-146's silence notable. (Update
  2026-08-06: `NotifyKind::Reminder` was deleted by user ruling; see the F-147
  note above.)
- `sweep.rs:314-328` - `spawn_sweep` (wfe F38) is a faithful collapse of the six
  duplicated loops: `MissedTickBehavior::Skip` preserved, no behaviour change, and
  all six call sites go through it (including WP-38's `spawn_bot_turn_sweep`, which
  the spec did not name).
- **WP-51 spec task 1 (wd F8 / wfe F42) is delivered and still holds at HEAD.**
  `execute_command` returns `Result<GameExtended, ExecuteCommandError>`
  (`game/mod.rs:78-90`, `:174`), and `dcd8844c` deleted the best-effort
  `find_game_extended(..).ok().flatten()` re-read in `handle_bot_command_event`.
  All **13** `notify_game_emails` call sites were audited: 8 pass `Some(before)`;
  the 5 that pass `None` (`email/commands.rs:470` `run_new`, `:906` restart,
  `game/server_fns.rs:1341` restart, `proposals.rs:1471` `create_proposal`,
  `:1717` `start_proposal`, `email/inbound.rs:1076` invite-accept) each do so for
  a game created moments earlier in the same function. **No caller passes `None`
  for a pre-existing game, and none passes it on an error path** - which is the
  exact abuse `notify.rs:549-553` forbids in capitals. This is the strongest piece
  of WP-51 and the one most likely to have rotted, since T-b (`ca7925b`) added
  three of those `None` sites *after* WP-51 landed. It did not rot.
- **wd F32 is genuinely closed and the sweep for its siblings comes back clean.**
  `notify_owner_decline` (`proposals.rs:461-468`) got the recommended
  `suppress_for_web_presence` + `invite_recipient_should_send` pair verbatim, and
  all six mailer methods now gate (`send_invite_core:276`,
  `notify_changed_reinvite:390`, `notify_owner_decline:466`,
  `notify_cancelled:542`, `notify_started:610`, `notify_owner_ready:680`).
  Explicitly **not** pattern 2. `send_invite_core:276` passing a literal `false`
  for `suppressed_by_presence` is equivalent, not a bypass: `:271-275` already
  returned on the suppressed case, so the predicate computed is identical; the
  split exists only to distinguish WP-46's transient `false` from permanent `true`.
- **wd F33 is closed correctly, including the part that is easy to get backwards.**
  The four one-way mails (`proposals.rs:488, 557, 625, 700`) dropped the reply
  promise and kept "unsubscribe" as the spec required, each with an explanatory
  comment; the two mails that *do* have a live reply channel (`:314` `send_invite`,
  `:413` `notify_changed_reinvite`) correctly kept theirs, because they carry real
  `i-{token}` addresses (`:341`, `:441`). Residual, below finding threshold: the
  constant dead address `i-noreply@brdg.me` is not special-cased in the router -
  `inbound.rs:95-96` yields `InboundRoute::Invite("noreply")` and
  `inbound.rs:856-866` runs a real token lookup that misses, logs at **info** and
  returns `Done`. One wasted query, no webhook retry, no error-log noise; worth a
  two-line short-circuit if anyone is in the file.
- **WP-53 is the cleanest work reviewed in this unit: 10 of 12 rows PASS, 0 FAIL,
  and both pattern-2 candidates came back negative.**
  - `wd F20` (markup-parse warn) hit **both** named sites with byte-identical
    blocks (`game/server_fns.rs:456-459`, `:753-756`), and the sweep for unnamed
    siblings is clean - all 7 `from_string` sites in `rust/web/src` accounted for;
    the other five were already non-swallowing, and `theme.rs:222`'s
    `unwrap_or_default()` is on a compile-time constant that cannot vary.
  - `wd F54` (viewBox from consts) fixed **both** `RatingChart` (`viz.rs:134`) and
    `Histogram` (`:216`); the only surviving `320`/`120` literals in the file are
    the four const definitions themselves.
  - `wd F65` is clean on **pattern 4b**, which is the trap this row was built for.
    The new `NON_ALPHANUMERIC.remove(b'-').remove(b'.').remove(b'_').remove(b'~')`
    (`players.rs:36-40`) is provably the same set as the old hand-rolled loop's
    `A-Za-z0-9-_.~`, and `utf8_percent_encode` encodes all non-ASCII bytes
    unconditionally, matching the old byte loop - **no widening or narrowing of the
    escape set**. The commit's `players.rs` hunk touches only lines 34-44, so all
    four retained tests (`:987-1004`) are unmodified and still pin the boundary
    cases. `percent-encoding = "2.3.2"` is a real entry at `web/Cargo.toml:34`, as
    the row required on pain of skipping.
  - `wd F22` took the **fix**, not the escape-hatch comment the row also allowed:
    `is_new = log.logged_at >= last_turn_at` (`server_fns.rs:762`) now matches both
    the sort key and the displayed field (`db/games.rs:461,467`).
  - `wd F23` took the comment option and it is the right call:
    `generate_bot_name` (`server_fns.rs:697-701`) is a one-line
    `petname::petname(1, "-")` - no pool, no query, no I/O, constant work - while
    its DB-touching neighbour `get_available_bots` (`:703-718`) does carry the
    `get_current_user` guard. Given F-94 (no rate limiting anywhere in `rust/web`),
    leaving anonymous the one endpoint that touches nothing is correct.
  - `wd F25`'s test `restart_core_rejects_non_player`
    (`game/server_fns.rs:2169-2196`) is **not a tautology**: passing
    `&[creator_id]` as `opponent_ids` clears `roster_error` and leaves `fetched`
    as `None`, so no game-service dependency exists and the new membership guard is
    the only check that can fire; the assertion pins its message.
- **`restart_core`'s pool-vs-transaction read is NOT a deadlock - recorded so it is
  not raised as one.** `game/server_fns.rs:1119-1124` calls
  `is_player_in_game(pool, ..)` while `tx` holds `FOR UPDATE` on the **`games`**
  row (`:1110-1115`), but the read is on **`game_players`** and the transaction
  writes nothing to that table, so no row-lock cycle exists - unlike the same-row
  case `email/outbound.rs:97-101` documents as the reason `ensure_email_token_tx`
  exists. The only cost is a second pooled connection (sqlx default max 10,
  `db/mod.rs:98`), i.e. a self-clearing stall at >=10 concurrent restarts of the
  same game. Stylistically off-convention - its neighbours at `:1136` and `:1145`
  both use `_tx` variants - and an `is_player_in_game_tx` would be strictly better,
  but it is not a correctness bug. Related and equally not a finding:
  `fetch_game_from_service` (`:1099-1103`) runs before the membership check, but
  only when there are no human invitees, and it performs no DB write.
- **Checked and clean, do not re-derive: `undo_game` does NOT need WP-53's wd F6
  guard.** `db/game_write.rs:584-590` writes `is_eliminated = $2` unguarded, unlike
  its `update_game_command_success` sibling at `:739`. That is correct, not a
  pattern-2 miss: wd F6 is about the *finish* path wiping elimination history, and
  a finished game can never be undone (`claim_unfinished_game_tx:644-646` plus the
  standing D-3 note at `:609-614`), so the `CASE WHEN $is_finished` arm would be
  inert there. (The `left_at` half of the same statement IS a real sibling miss -
  that is Unit 06's F-116, already raised.)

## Coverage gaps

- **WP-51 is fully covered; all seven spec tasks were checked against final code.**
  Tasks 1, 4, 5, 6 pass cleanly; task 2 passes (wfe F41/F43 verified, and the
  concurrent-send loop the spec rejected was indeed not built); task 3 is F-147;
  task 7 is F-145.
- **Not audited: `sweep.rs`'s and `notify.rs`'s test modules beyond the four
  `#[sqlx::test]`s quoted above.** `notify.rs:639-914` was read; `sweep.rs`'s test
  module was not.
- **Carry to the unified report - wfe F36 needs re-opening.** It is recorded as
  closed by WP-51 (`EXECUTION-STATE.md:91`) on the strength of a deviation recorded
  only in free text at `:172`. Nothing routes it onward. This is **pattern 1, the
  routing leak**, in its purest form yet: not a finding deferred from one package
  to another and dropped in transit, but a finding **closed by the package that
  documented its own decision not to fix it**. The sign-off procedure F-109
  proposed - "assert each closed finding's citation or regression test still
  exists" - would not catch this one, because the citation does exist: it is a dead
  function whose doc comment says the finding is fixed. The stronger check is
  "assert each closed finding's citation is *reachable*".
- **Carry to whoever owns the invite/proposal email surface in remediation:**
  F-144, F-145 and F-146 all belong to WP-46's and #24's code, not WP-51's, and
  none had ever been reviewed. F-144 (High) in particular is a live
  duplicate-email bug, not a review nit. F-145 should be fixed in the same change
  as F-136 - they are the same defect class in the two halves of the same sweep
  module.
- **WP-53 is fully covered** - all 12 rows checked against final code, plus both
  dropped rows (`wd F18`, `wd F56`) confirmed unclaimed by the commit. Two residual
  cosmetics not worth findings: `3610b957` deleted `encode_path_segment`'s doc
  comment and left a mid-file `use percent_encoding::...` at `players.rs:34`
  (D4 in `w3-wp53.md`); and `wd F77` was satisfied by a two-word swap
  ("email placeholder" -> "email management", `settings.rs:1-2`) which is *true* of
  `EmailSection` (`:302`) but does not enumerate add/confirm/make-active/remove as
  the row asked (D5).
- **Carry to the unified report's process-fixes section - a new failure mode,
  distinct from pattern 4b.** F-148 is a *correct* fix, honestly closed, on a row
  that demanded a test, with no test written and no test edited. Every safeguard
  this session has proposed so far assumes the artefact is wrong or the test is
  wrong; here both are right and the guard is simply unpinned. Of WP-53's three
  "Test? y" rows, only `wd F25` got one. **The cheap systemic check is to grep the
  checklists for "Test? y" rows and confirm a test exists for each** - it is
  mechanical, and it would have caught F-148, F-149 and (per Unit 07) F-142's
  vacuous assertion.
- **Also carry:** F-147's variant of pattern 1. wfe F36 is recorded as closed by
  the very package that documented its decision not to fix it. F-109's proposed
  sign-off check ("assert each closed finding's citation or regression test still
  exists") does **not** catch it, because the citation exists - it is a dead
  function whose doc comment asserts the fix. The check must be that each closed
  finding's citation is *reachable*, not merely present.
