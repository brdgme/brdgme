# Unit 07 - Web domain: remainder (proposals, visibility, export/import, email canon)

Findings continue from F-121.

## Progress

- [x] Recon: commit sizing, spec recovery for WP-42/44/45/46/47/48/50/51/53/79.
- [x] CF-1: WP-50 canonicalization -> F-124 (High), F-125/126 (Med), F-127/128 (Low).
- [x] CF-2: `find_user_by_settings_token` -> F-129/F-130 (Med), F-131 (Low/Med),
      F-132 (Low). F-123 REFUTED. WP-44 verified good throughout.
- [x] CF-3: `a9609e57` `import_game.rs` 100 MiB guard -> F-121 (Low).
- [x] CF-4: `game/import.rs:109,124` `undo_game_state` -> F-122 (Low, no route).
- [x] WP-42 `is_proposal_visible_to_user` + cache (`3c6b3047`) -> F-132; filtering
      confirmed alive in SSE, NOT reverted by `efad81f`.
- [x] WP-44 `f4e76406` -> `email_token` leak genuinely closed; see Verified good.
- [x] WP-50 `33f22f1c` -> F-124..F-128.
- [x] WP-79 -> F-134, F-135 (both High). WP-46 -> F-136 (High), F-141, F-143.
      WP-45 -> F-137 (High), F-138. WP-48 -> F-139, F-142. WP-49 -> F-140.
      WP-47 -> no gaps found, callers verified wired. T3-B5 -> clamp correct.
- [ ] NOT DONE: WP-51 `dcd8844c` (largest gap), WP-53 `3610b957`. See Coverage gaps.

**Unit complete** apart from the two named gaps. 13 findings, F-121..F-143.

Recon done. Commit sizes are moderate; unit fits budget without a split.
Corrections to the brief's file names: there is no `db/game_visibility.rs`
(it is `rust/web/src/db/visibility.rs`) and no `controller/import_game.rs`
(it is `rust/web/src/bin/import_game.rs`). WP-44, WP-53 and WP-79 have **no
spec file** - their checklist rows are the only acceptance criteria (add
WP-44 and WP-53 to `00-STATE.md`'s existing WP-24/WP-27 no-spec note).

All four named carry-forwards are CLOSED. Currently in flight: the final
sweep worker covering WP-47, WP-48, WP-79, WP-46, WP-45, WP-51, WP-49, WP-53
and T3-B5.

## Findings

### F-121 (Low, informational) - CF-3: `a9609e57`'s 100 MiB import guard bounds the file, not the work

`rust/web/src/bin/import_game.rs:20-32`. The guard `stat`s a path the operator
themselves passed on the command line, so it defends against nothing an
attacker controls; it is a sanity limit, correctly labelled as one. Two
residual gaps, both Low given the dev-only context: `metadata.len()` is 0 for
a FIFO or `/dev/stdin`, after which `read_to_string` is unbounded; and the
byte cap does not bound the row counts `import_bundle` then inserts (players,
logs and log targets are all unbounded loops). Suggested fix if it is ever
made reachable by anything but a developer: read through a
`Take`-limited reader instead of stat-then-read, and cap
`bundle.players.len()` / `bundle.logs.len()`.

### F-122 (Low, informational) - CF-4: bundle-supplied `undo_game_state` is real but not attacker-reachable

Unit 06 carried this forward as "attacker-controlled game-state replay via the
import path". Confirming the mechanism and downgrading the reachability:
`rust/web/src/game/import.rs:109,124` does write `undo_game_state` verbatim
from `BundlePlayer.undo_game_state`
(`rust/web/src/game/export.rs:65`), and `undo_game` later replays it after
checking only that it is non-NULL - so the mechanism Unit 06 described is
exactly right. But the only caller of `import_bundle` is the dev CLI
`rust/web/src/bin/import_game.rs:35`; unlike `admin_export_game`
(`export.rs:181`) there is **no HTTP route and no server fn** that reaches it,
and the module header (`import.rs:1-4`) states it is never deployed. The
attacker would have to already be the developer running the binary against
their own database. Keep it recorded as a latent design hazard - the import
path is the only writer of `undo_game_state` outside
`update_game_command_success` and it applies no validation whatsoever - so
that any future admin-facing import UI is required to validate or drop the
field rather than inherit this. _(Basis for "no route": `import_bundle` has
exactly one caller, the CLI at `bin/import_game.rs:35`, and `import.rs`
declares no axum handler and no `#[server]` fn - unlike `export.rs:181`.
Flagged to the sweep worker for an independent router-level confirmation.)_

### F-129 (Medium) - CF-2: the settings-email token never expires and can never be rotated or revoked

`find_user_by_settings_token` (`rust/web/src/email/inbound.rs:520-530`) is the
second authentication mechanism flagged by `00-STATE.md`. Reviewed in full;
generation is sound (`email/outbound.rs:72-79`, `rand::rng()` ChaCha CSPRNG,
32 chars of `[a-zA-Z0-9]` ~= 190 bits, unique index in
`migrations/023_settings_email_token.sql:4-7`). The lifecycle is not.

- **No expiry.** Migration 023 has no TTL column and the lookup has no age
  predicate. The `s-{token}@brdg.me` reply address is a **permanent bearer
  credential embedded in every settings email the user has ever received** -
  an old mailbox archive is a live credential forever.
- **No rotation or revocation.** `ensure_settings_email_token`
  (`outbound.rs:123-139`) returns the existing token forever; the only writer
  of the column in the entire tree is that one UPDATE. It is not invalidated
  on use, on logout, or on email add/remove/unverify. This is a **consistency
  failure inside the same subsystem**: `unsubscribe_token` IS rotated on use
  (`email/unsubscribe.rs:99`) and proposal invite tokens ARE rotated on roster
  change (`proposals.rs:936-944`). The settings token is the one of three with
  no rotation path - the session's pattern 2 (inconsistent hardening within
  one area) applied to credential lifecycle.

Fix: add `settings_email_token_expires_at` and filter on it in the lookup;
NULL the column on email removal/unverification; expose a rotate action.

### F-130 (Medium) - the "settings" token is not scoped to settings

`rust/web/src/email/commands.rs:329-346`. A holder reaching
`dispatch_standalone_server_command` via `inbound.rs:1449-1478` gets more than
settings: `new` - **create a real game naming arbitrary opponents and bots** -
(`commands.rs:335-338`), `bump` (`:339-341`), subscribe/unsubscribe
(`:342-344`), and only then the settings verbs (`:276-284`, `:311-313`). The
credential's name and its issuing context (a settings email) both understate
what it authorises.

Mitigating control, and it is a real one: every command is gated on
`from_matches_verified_email` (`inbound.rs:1421-1433`, defn `:532-545`), which
requires the From address to be a **verified** address on that same account,
after SPF/DKIM/DMARC classification (`inbound.rs:191-214`). So the token alone
is insufficient - an attacker needs token **plus** a From-spoof surviving
DMARC. That keeps this Medium rather than High. Note the dependency: this
token's entire safety margin rests on WP-56's From-authentication, which is
**Unit 09's** to review. If Unit 09 finds a weakness in
`from_matches_verified_email` or the DMARC classification, F-129+F-130 escalate to
account takeover and should be re-rated then. **Carry to Unit 09.**

Fix: narrow the dispatch reachable from an `s-` route to settings verbs only;
require a web session for `new`.

### F-131 (Low/Medium) - SSE streams authenticate once at connect and never again

`rust/web/src/events.rs:33-41`. `validate_session_token` runs exactly once, at
connection. An SSE stream is long-lived, so after logout or session revocation
the stream keeps delivering frames indefinitely - only the *visibility* answer
is refreshed (30s TTL), never the *authentication*. Fix: re-run
`validate_session_token` on the same 30s cadence and break the loop on
failure. (Adjacent to Unit 09's ownership of `efad81f`; raised here because
the visibility work that made me read the file is WP-42's.)

### F-132 (Low) - `VisibilityCache` is safe only by an unstated ownership convention

Downgraded from the High raised earlier in this review - see the REFUTED
entry below. `rust/web/src/visibility_cache.rs:11` keys on an id alone, which
is correct **only** because each instance is owned by exactly one SSE task
with one fixed viewer. Nothing in the type expresses that. Fix: a doc comment
stating the per-viewer ownership requirement, or key on
`(id, Option<Uuid>)` so the invariant cannot be broken by a future caller.

### REFUTED (recorded so it is not re-raised) - `VisibilityCache` cross-user visibility leak

I raised this as High on the strength of `is_proposal_visible_to_user` being
user-scoped while the cache key is not. **It is not a leak.** The single
non-test construction site is `rust/web/src/events.rs:65`, a plain `mut` local
**inside** the per-request `tokio::spawn` opened at `events.rs:47` - not in
`AppState`, not a static, no `Arc`/`Mutex`, never cloned. Both call sites
(`events.rs:81`, `:100`) close over a `viewer` computed once at
`events.rs:33-41` before the spawn and immutable thereafter. One instance =
one connection = one viewer; two viewers cannot share an instance.

Also refuted on the same pass: **WP-42 was NOT silently reverted by the SSE
migration.** Its filtering was carried across into `events_handler`, which
still authenticates and still filters both game and proposal frames. This is
*not* a second instance of pattern 4e - a useful negative result given F-109.

Recording this because the finding was plausible from the code shape alone and
the next reader will re-derive it. It is a good illustration of the session's
own rule: reasoning from shape produced a false High; reading the ownership
site settled it.

### F-133 (Low) - proposal visibility is granted by roster membership only, never by ownership

`rust/web/src/db/proposals.rs:40-52`. `is_proposal_visible_to_user` returns
true iff the viewer has a row in `game_proposal_players`; it never consults
`game_proposals.owner_user_id`. An owner not also inserted as a player cannot
see their own proposal. Both tests (`:172`, `:193`) add the owner as a player
explicitly, so the gap is untested in either direction. Fix: `OR EXISTS(SELECT
1 FROM game_proposals WHERE id = $1 AND owner_user_id = $2)`, or a test
pinning that the owner is always a roster row.

---

### ARCHIVED - original text of the refuted F-123, kept for audit

`rust/web/src/db/proposals.rs:40-52` `is_proposal_visible_to_user(pool,
proposal_id, viewer_id)` is **user-scoped**: it returns true iff the viewer is
a row in `game_proposal_players` for that proposal. Confirmed by reading the
function, not inferred.

`rust/web/src/visibility_cache.rs:12-13,25-31,58` caches its result as
`HashMap<Uuid, (bool, Instant)>` keyed **solely on the proposal id**. The
viewer is not part of the key. Any `VisibilityCache` instance shared by more
than one viewer therefore serves the first viewer's answer to every later
viewer for the 30s TTL - a participant's `true` leaks the proposal to a
stranger, and a stranger's `false` hides it from a participant. `check_game`
is not affected in the anonymous case (`is_game_publicly_visible` is a
property of the game) but IS affected the same way whenever the game lookup
is `is_game_visible_to_user` via
`is_game_visible_to_viewer` (`db/visibility.rs:122-131`), which is
per-viewer too.

Note the cache's own test suite (`visibility_cache.rs:176-190`
`game_and_proposal_ids_do_not_alias`) tests exactly the aliasing axis it did
handle - two separate maps - and not the axis it did not. A textbook instance
of the session's pattern: the checklist row ("cache visibility lookups, don't
let ids alias") satisfied literally while the thing it was for is missed.

Severity is High **iff** a cache instance is reachable by two different
viewers; Low if it is constructed per-connection. Confirming this is the
single highest-value open item in the unit. Suggested fix either way: key on
`(id, Option<Uuid> viewer)`, which costs nothing and removes the question.

Also to confirm on the same pass: WP-42's spec file is
`WP-42-websocket-auth-and-filtering.md`, and Unit 09's `efad81f` replaced
WebSockets with SSE. If `VisibilityCache` lost its consumer in that migration
it is dead code and WP-42 is a **second instance of pattern 4e** (a landed fix
silently reverted by a later commit in the same programme) alongside F-109.

Secondary, Low, same file: `is_proposal_visible_to_user` grants visibility
only via `game_proposal_players` membership, not via
`game_proposals.owner_user_id`; an owner not also inserted as a player cannot
see their own proposal. Both tests add the owner as a player explicitly, so
the gap is untested.

### F-124 (High) - CF-1: `add_proposal_player` is a third invite-by-email entry path that WP-50 missed entirely

`rust/web/src/proposals.rs:1730-1786` (param `:1733`, use `:1772`, policy
check `:1780-1783`). The `#[server(AddProposalPlayer)]` fn takes
`email: Option<String>` and passes it **raw** to
`find_or_create_user_by_email_tx` (exact match `WHERE ue.email = $1`,
`proposals.rs:1169`) and then to `check_invite_policy_tx`. No
`canonicalize_email`, no empty check, no `@` check.

WP-50's spec criterion 3c enumerated `create_proposal` and
`restart_game_with_roster`; both were done correctly
(`proposals.rs:1384-1394`, `game/server_fns.rs:1289-1297`). This third path
does exactly the same thing and was not on the list. **This is the unit's
canonical instance of the session-wide pattern: the checklist row was
satisfied literally and completely, and the thing the row was for was
missed.**

Exploit, concretely: invite `" foo@x.com "` (leading space) at a victim whose
canonical row is `foo@x.com`. The exact-match lookup misses, so a ghost
`users` row plus a `user_emails` row holding the non-canonical duplicate is
inserted with `verified_at = NOW()` (`proposals.rs:1185-1197`) - and the
`lower(email)` index does not stop it (F-125). Two verified rows now point at
one mailbox, and every canonicalized lookup misses the ghost.
`check_invite_policy_tx` then resolves the raw string to the freshly created
ghost, which has default `invite_policy = 'open'` and no blocks, instead of
to the victim - so **D7 block-by-target and `invite_policy` are both
bypassed** (`db/visibility.rs:181-188`; a lookup miss silently passes,
`:167-168`). A case-only variant (`FOO@X.COM`) trips the index and surfaces
as an internal error instead - noisy but not exploitable. The whitespace
variant is the live hole.

Fix: canonicalize at the top of `add_proposal_player` and reject empty /
no-`@` before `find_or_create_user_by_email_tx`, mirroring
`proposals.rs:1384-1394`.

### F-125 (Medium) - the unique index enforces only the case half of the canonical form

`rust/web/migrations/026_canonical_emails.sql:33` creates
`UNIQUE INDEX ... ON public.user_emails (lower(email))`, but the backfill one
line above normalizes with `lower(btrim(email))` (`:32`). The index therefore
does not enforce the trim half, so `" foo@x.com "` and `"foo@x.com"` coexist -
which is precisely what makes F-124 exploitable rather than merely noisy.
There is also no `CHECK` that stored values are canonical, so any future
non-canonicalizing insert path silently poisons the table the same way.

Fix: index on `lower(btrim(email))`, and/or add
`CHECK (email = lower(btrim(email)))`. The `CHECK` is the stronger fix - it
converts every future F-124-class omission from a silent data-integrity
breach into a loud constraint violation.

### F-126 (Medium) - empty-string address reaches an INSERT via the same fn

`rust/web/src/proposals.rs:1733 -> :1772 -> :1191`. WP-50 deliberately made
`canonicalize_email("   ") == ""` and pushed emptiness validation onto
callers; `auth/email_addr.rs:21-24` blesses that with a test. Every caller
honours the contract except `add_proposal_player`: `email: Some("")` passes
the `provided != 1` check at `:1728-1731` and reaches
`INSERT INTO user_emails ... VALUES ($1, '', true, NOW())`. The first call
creates a junk verified account; the second returns a raw 23505 as a 500.
Confirmed rejected on every other path (`login:296`, `confirm_login:349`,
`add_email_address:856`, `create_proposal:1388`,
`restart_game_with_roster:1293`, and both client boundaries).

### F-127 (Low) - `create_game_with_users_tx`'s invite path neither canonicalizes nor carries the required doc comment

`rust/web/src/db/game_write.rs:81-115` (lookup `:85`, insert `:108`) resolves
`opts.opponent_emails` by exact match and inserts the raw string. WP-50's
criterion 3a required the "callers must pass canonicalized addresses" doc
comment on the db helpers; `db/emails.rs:71` and `db/visibility.rs:171` got
it, this one did not. Latent rather than live: all thirteen production
callers currently pass `&[]`. Fix: add the doc comment, or delete the dead
branch.

### F-128 (Low, note) - Rust `to_lowercase` vs Postgres `lower()` asymmetry - fails closed

Recording the negative result so it is not re-derived. Canonicalization is
Rust full-Unicode `to_lowercase` (`auth/email_addr.rs:3-5`) while the unique
index (`026:33`) and the inbound authorization compare
(`email/inbound.rs:538`, `LOWER(email) = LOWER($2)`) use Postgres `lower()`.
The Unicode-folding attack (U+212A KELVIN -> ascii `k`, U+1E9E -> `ß`) **does
not** yield takeover: the fold happens at the Rust boundary *before* both
storage and delivery, so the confirmation code is always mailed to the folded,
legitimate address, and a colliding registration is rejected by the index as
"Address unavailable". The divergence fails closed. Separately, inbound
`extract_addr_spec` (`email/inbound.rs:134,150`) trims but does not lowercase -
a genuinely different normalization from the shared helper, spec-deferred to
WP-56/WP-59, not exploitable today because both sides of the SQL compare are
case-folded. **Carry to Unit 09:** re-check this when WP-59's inbound work is
reviewed.

### F-134 (High) - WP-79 hoisted the HTTP call out of two of four locked transactions

`rust/web/src/proposals.rs:1702-1709`. `start_proposal` begins a tx at `:1647`,
takes the row lock via `lock_proposal_for_update` at `:1652`
(`... FOR UPDATE`), then calls `fetch_game_from_service` - a `reqwest` call to
the game service - at `:1702`, **still holding the lock**, before
`start_proposal_tx` at `:1709` and commit at `:1711`. Hoisting exactly this
call is the entire point of WP-79. It was hoisted in `create_proposal`
(`:1105`) and `restart_core` (`game/server_fns.rs:1091`), and not here. The
commit message reads clean.

Why it matters: a slow or hung game service holds a `game_proposals` row lock
for the full reqwest timeout, and every concurrent respond / cancel / transfer
/ nudge on that proposal blocks behind it - they all take the same
`FOR UPDATE`. Fix: move the fetch above `pool.begin()`; `accepted_count` can
be read outside the tx and re-validated inside (the roster is re-read at
`:1666` regardless), aborting for retry on mismatch.

This is the breakdown's own gotcha for WP-79 ("check no other transaction in
the same module still holds a row lock across a network call") coming back
positive.

### F-135 (High) - WP-79's own commit put a new HTTP call on the wrong side of `begin()`

`rust/web/src/email/inbound.rs:1021-1034`. `handle_invite_reply` begins its tx
at `:922` and locks at `:931`; `91c723d4` - the WP-79 commit itself - inserted
`fetch_game_from_service` at `:1022`, **inside** the lock. The commit moved the
call out of `start_proposal_tx` and into the caller but landed it after
`begin()` rather than before it. This is the sharpest instance of the pattern
in the unit: the refactor named in the checklist row was performed, and the
property the row existed to establish was not.

Harder to hoist than F-134 because `accepted_count` depends on the in-tx
response UPDATE. Fix: restructure as in F-134, or record the exception
explicitly in the module doc so it is not read as already-fixed. Note this is
the inbound-webhook path, which also holds the lock across the whole render.

### F-136 (High) - WP-46 reintroduces mark-without-send through a catch-all arm

`rust/web/src/email/sweep.rs:135-137`:

```
let recipient = match crate::email::outbound::fetch_email_recipient(pool, game_player_id).await {
    Ok(Some(r)) => r,
    _ => return ReminderOutcome::PermanentSkip,
};
```

The `_` arm swallows `Err(_)` as well as `Ok(None)`. `sweep_once`
(`:289-305`) treats `PermanentSkip` identically to `Sent`: it calls
`mark_reminder_sent_tx` and **commits**. So a single transient DB error on the
recipient read permanently sets `turn_reminder_sent_at` and no email is ever
sent - which is precisely the wfe F30 failure WP-46 exists to remove. The spec
is explicit that `PermanentSkip` means "recipient row missing" and that errors
are `Retry`.

This is a clean instance of the session's `_ => <default>` pattern (F-65): the
catch-all makes the match exhaustive and silently reclassifies the one case
that matters. Fix:
`Ok(Some(r)) => r, Ok(None) => PermanentSkip, Err(_) => Retry`.

### F-137 (High) - WP-45 left one of its three named entry points unvalidated

`rust/web/src/game/server_fns.rs:1087`. `restart_core(..., bot_slots:
&[BotSlot], ...)` takes client-supplied bot slots from
`restart_game_with_roster` (`:1271`, `:1299`, `:1334`) and never calls
`validate_bot_slots` - `rg validate_bot_slots` has zero hits in the file.
WP-45's spec section 1 names `restart_core` as one of the three wd F27 call
sites. The solo-vs-bots branch at `:1178` goes straight to
`insert_game_from_service`, so a restart carrying `bot_name: "garbage"`
creates a wedged game. The multi-human branch is saved only incidentally, by
`start_proposal_tx`'s check at `proposals.rs:1411`.

Fix: call `validate_bot_slots` on `bot_slots` before `pool.begin()` in
`restart_core`.

### F-138 (Medium) - WP-45's validator is case-insensitive and does not canonicalize what the caller stores

`rust/web/src/db/bots.rs:61-63`. All four landed entry points
(`proposals.rs:1264`, `:1812`, `:1411`, `email/commands.rs:420`) call the same
validator, so they are genuinely consistent - and consistently case-**in**
sensitive: `n.eq_ignore_ascii_case(&slot.bot_name)`. The validator does not
return or impose a canonical name, so the caller persists the client's string.

This closes the loop on Unit 05b's F-104. That finding observed a test
(`validate_bot_slots_accepts_case_mismatch`) blessing case-insensitive
validation while all four consumers of the stored value match
case-sensitively. Confirmed from the write side: **every one of the four write
paths can store a `bot_name` that validation accepted and no consumer will
ever match.** F-104 and F-138 are one defect and should be remediated
together. Fix: have `validate_bot_slots` return the canonical name (or a
normalised `Vec<BotSlot>`) and have callers store that, not the client string.

### F-139 (Medium) - WP-48's unique-violation fallback cannot execute

`rust/web/src/game/import.rs:190-210`. The wd F10 fallback retries after a
unique violation, but `placeholder_user` is called at `:103` with `&mut tx` -
the import transaction. In Postgres a unique violation aborts the transaction,
so the fallback's `generate_unique_username` and its second INSERT both run on
an aborted connection and fail with 25P02. No SAVEPOINT is taken. The guard is
present, satisfies its checklist row, and changes nothing but the error text.
Capped at Medium because the path is the dev-only CLI. Fix: take a nested
savepoint before the retry, or do the placeholder insert on a separate
connection.

### F-140 (Medium) - WP-49's visibility filter breaks `rules` for in-flight games on deprecated versions

`rust/web/src/db/game_types.rs:81-91`, consumed at
`rust/web/src/email/commands.rs:939-946`. `find_game_version_rules` and
`find_game_version_render_meta` now filter on `is_public = true AND
is_deprecated = false`, which is right for the public page. But `run_rules`
resolves the version id from the **game**
(`find_game_version_id_for_game`, `:934`) and then calls those same filtered
queries. A player in an in-flight game whose version has since been deprecated
gets "Game version not found" from the `rules` email command, and the same
applies to `/rules/<version_id>` links that `email/notify.rs::rules_url`
generates from real games. The spec asked only for the public-page filter and
did not consider the by-game callers. Fix: keep the filtered fns for the public
entrypoint and add unfiltered `*_for_game` variants (or an `allow_deprecated`
flag) for `run_rules` and `rules_url`.

### F-141 (Low) - three of WP-46's four sweep candidate queries are still unbounded

`rust/web/src/proposals.rs:973`, `:1007`, `:1082`. Rider wfe F40 required a
LIMIT on all four sweep candidate queries; only `fetch_candidates`
(`email/sweep.rs:44-53`, `LIMIT $2`) got one. `fetch_nudge_candidates`,
`fetch_expiry_candidates` and `fetch_auto_decline_candidates` are unbounded.
Pattern 2 again - one of a set hardened, the siblings left. Fix: apply the same
shared const limit to all three.

### F-142 (Low) - a WP-48 regression test's second assertion is vacuous

`rust/web/tests/ssr_pages.rs:1290-1300`.
`admin_export_route_rejects_non_admin` uses `Uuid::new_v4()` as the game id, so
the spec's "and the body must not contain the private log body" half asserts
nothing - there is no game to leak. The 403 assertion itself is real. Fix: seed
a real game with a private log and assert against its body.

### F-143 (Low, note) - WP-46 mandates by decree the anti-pattern WP-79 exists to remove

`rust/web/src/email/sweep.rs:260-306`. By design (WP-46 spec 3a) the claim
transaction holds a `game_players` `FOR UPDATE` row lock across
`send_reminder`, which performs a game-service render **and** the Resend API
call - serialised over up to 200 candidates per tick. Not a deviation from its
own spec, and recorded only so the two work packages are not read as
contradicting each other: WP-79 removes network-calls-under-lock from the
proposal path while WP-46 introduces it on the sweep path. The unified report
should reconcile these into one policy.

## Verified good

- `rust/web/src/game/export.rs:181-206` `admin_export_game` is properly gated:
  session user present, `validate_session_token` re-checked against the DB,
  then `is_user_admin`; every error path returns 500 rather than falling open.
  Matches WP-48's admin-only criterion. Route registration independently
  confirmed: `router.rs:147-148` registers only `GET
  /admin/games/{id}/export`, and `rg import_bundle` over all of `rust/` hits
  only `game/import.rs` (defn + tests) and `bin/import_game.rs:35` - no
  `#[server]` fn touches it. F-122's reachability conclusion stands.
- `5e9bae2c` is a **genuine de-flake, not a weakened assertion** - worth
  stating because pattern 4b made this the default suspicion. The test asserts
  the stored `created_at` equals `past` but reads the row back with
  `ORDER BY logged_at, id LIMIT 1` (`import.rs:417-424`), which need not be
  `bundle.logs[0]`; setting every log's `created_at` to `past` makes the read
  order-independent and the asserted value is unchanged. Minor residual: the
  test can no longer catch per-log `created_at` being collapsed to one value.
- **WP-47 is wired, not "built but never wired"** - I went looking for the
  breakdown's predicted repeat and it is not there. Every read path returning
  game details or player identity is gated: `get_game_details` rejects
  non-players via `is_game_visible_to_user` before rendering
  (`game/server_fns.rs:263-270`); `render_game_public` calls
  `is_game_publicly_visible` first (`:415-420`); `get_game_logs` (`:739`) and
  `get_restart_prefill` (`:1387-1393`) are hard player-only. Stats
  anonymisation reaches the **rendered output**, not just one query: all three
  stats server fns resolve `viewer_user_id` (`stats/mod.rs:183,248,333`) and
  thread it to all four query fns; `opponents_by_game`
  (`stats/queries.rs:231-260`) and `head_to_head` (`:559-580`) mask; and
  `players.rs:718-724,78,103` renders masked rows as plain text with links
  dropped, because `HeadToHead.user_id` became `Option<Uuid>`
  (`stats/mod.rs:117`) with no residual non-Option consumer. This is the
  second-strongest piece of work in the unit after `db/visibility.rs`.
- WP-46's non-broken half is substantial: the `ReminderOutcome` mapping
  otherwise matches its spec (`sweep.rs:144,156,230`), `Retry` rolls back by
  drop leaving the row for the next tick,
  `cancel_proposal_for_expiry` (`proposals.rs:1024-1078`) reads owner and
  accepted ids **before** the UPDATE and returns `None` on any error,
  `auto_decline_proposal_player` (`:1108`) checks `rows_affected() == 1` so
  only real pending->declined transitions notify, and nudges mark only when
  every candidate of a proposal returned true (`sweep.rs:507-518`).
- T3-B5 `46847d40` is correct: `players.rs:846`
  `Some((d.page + 1).clamp(1, 1_000_000))` matches the server ceiling in
  `stats/mod.rs::get_player_history`. No off-by-one - `d.page >= 1` always so
  the lower bound is inert, and at the ceiling the link self-references but
  `hide_next` (`d.page >= total_pages`, `:848`) already suppresses it.
- `game_info_rules_version_id` (`game_info/queries.rs:15`) filters correctly
  and orders by `created_at DESC`, with both regression tests present - the
  WP-49 defect is only in the by-game callers (F-140), not the public page.
- WP-50 criteria 3a/3b/3d/3e are genuinely met, verified against final code,
  not just the diff: all six `auth/server.rs` server fns canonicalize as
  their **first** statement (`:287, :342, :855, :906, :931, :981`) with the
  empty/`@`, plus-addressing and blocked-domain guards running *after*, on
  the canonical value. The stored value is the canonical value on every live
  auth path (`:218, :237, :491, :886`), delivery uses the canonical string
  (`:245`), and lookups (`db/emails.rs:73`, `auth/server.rs:401,448,461,467`)
  all receive canonical values. Store side and lookup side agree everywhere
  except F-124. Client boundaries canonicalize too (`new_game.rs:447`,
  `settings.rs:416`).
- `create_proposal` (`proposals.rs:1384-1394`) and `restart_game_with_roster`
  (`game/server_fns.rs:1289-1297`) both canonicalize and reject empty/no-`@`
  *before* the policy check - the ordering that matters.
- `rust/web/src/db/visibility.rs` - `is_game_publicly_visible`,
  `is_game_visible_to_user`, `visible_user_ids` all implement "no player
  fails" rather than "some player passes", bots (NULL `user_id`) are dropped
  by the JOIN, and there is a real drift guard test
  (`visible_user_ids_drift_guard_matches_is_game_visible_to_user`, `:501`)
  plus a two-`friends`-player test (`:419`) for the exact quantifier error.
  This is the strongest-looking work in the unit.
- `rust/web/src/visibility_cache.rs:61-64` fails closed on lookup error and
  does not cache the failure; bounded at 256 with oldest-eviction; TTL
  expiry works. Per-connection ownership confirmed at `events.rs:47,65`.
- **WP-44's `email_token` leak is genuinely closed**, and closed the right
  way. It was `ProposalPlayerView.email_token`, so every roster member
  received every other invitee's bearer invite token in the serialized
  `ProposalView`. At HEAD the **field is removed from the type**
  (`proposals.rs:63-73`), not merely left unpopulated, and the column is
  dropped from `find_proposal_roster` (`:502-509`) - so no serialization is
  possible even by accident. `ProposalPlayer` (`:41-53`) still carries the
  token but every producer is `ssr`-gated and no `#[server]` fn returns it.
  This is the counter-example to the unit's other findings: the fix removed
  the capability rather than adding a guard over it.
- **WP-44's proposal-integrity guards are all acted on, none advisory.**
  `respond_denied_reason` (`proposals.rs:1307-1321`) is consumed with an early
  return at `:1592-1598`; `transfer_target_error` (`:1332-1342`) likewise, and
  the client gate (`show_make_owner`) was changed to match the server rather
  than the reverse. Most importantly
  `find_game_type_player_counts.unwrap_or_default()` became `.ok_or_else(...)`
  at `:1609-1613` **and** at the `start_proposal` sibling - the empty
  `player_counts` had made `roster_error` vacuously pass, so a proposal could
  start with an unvalidated roster. Both siblings fixed: explicitly *not* an
  instance of pattern 2.
- **WP-44's removed guards are net-neutral and I verified each replacement.**
  The commit deletes a pre-transaction `find_proposal` + owner + status check
  from four fns; at HEAD each re-derives under `lock_proposal_for_update`
  (`proposals.rs:1121-1134`) and re-checks inside the transaction
  (`add_proposal_player:1754-1762`, `cancel_proposal:1873-1882`,
  `remove_proposal_slot:1924-1932`,
  `transfer_proposal_ownership:1994-2002`, `start_proposal:1652-1660`).
  Removing the unlocked copy **closes** a TOCTOU window. `cancel_proposal`'s
  roster read also moved inside the lock so the notify audience matches the
  cancelled state.
- `reset_accepted_humans_for_roster_change` (`proposals.rs:934-947`) rotates
  `email_token` as part of the same UPDATE, so a roster change invalidates
  previously mailed invite tokens - the rotation discipline F-129 says the
  settings token lacks.
- Settings-token hygiene that IS right: no token appears in any tracing call
  in the email module (`inbound.rs:1412,1416,763,767,859,863` are all
  token-free literals), and `parse_reply_address` (`inbound.rs:91-105`) does
  no lowercasing, so the 62-char keyspace is preserved.
- `rust/web/migrations/026_canonical_emails.sql` is the right shape for a
  canonicalization backfill: it `RAISE EXCEPTION`s naming the colliding
  canonical forms rather than silently merging two accounts (`:17-27`), and
  purges `login_confirmations` (whose PK is the email) rather than colliding
  on it, and it runs inside sqlx's per-migration transaction so a collision
  leaves the DB untouched. The index it creates is nonetheless wrong - see
  F-125.

## Coverage gaps

- The canonicalization contract is enforced only by doc comment, in at least
  three places: `db/emails.rs:71` ("Callers must pass a canonicalized
  address"), `db/visibility.rs:171` (same wording on
  `check_invite_policy_tx`). Nothing in the type system or the query layer
  stops a raw address reaching `WHERE email = $1`. F-124 is a caller that
  violates it and F-127 is a db helper that did not even get the comment, so
  this is no longer hypothetical. **A newtype `CanonicalEmail` whose only
  constructor is `canonicalize_email` would close the entire class
  permanently** and is the single recommendation from this unit most worth
  putting in the remediation plan.
- **WP-51 `dcd8844c` is the largest unexamined surface in this unit.** Confirmed
  only that it did not revert WP-46, and that `game_log_count` /
  `turn_subject_or_fallback` (`email/notify.rs:261-290`) fail safe (a missing
  turn count de-threads via timestamp rather than merging distinct mails). NOT
  audited: the six `RealInviteMailer` methods, the `spawn_sweep` collapse, and
  `notify_owner_decline`'s new gating - the last of which is exactly where a
  dedup fix would drop legitimate distinct sends. Recommend a short follow-up
  pass before sign-off.
- **WP-53 `3610b957` was not reviewed** beyond its diffstat (9 files, +84/-29,
  no spec file). Small and low-risk-looking, but genuinely unexamined.
- WP-46's `proposals.rs` half (+428) was read only at the four functions its
  spec names; its new test module was not audited.
- Lead-side budget note: `rust/web/src/email/inbound.rs` is 2,503 lines and
  `proposals.rs` is touched by five of this unit's fourteen commits. Both
  were read by workers, not by the Lead.
- **F-130's severity is conditional on Unit 09's work.** The settings-email
  token's whole safety margin is `from_matches_verified_email` plus the
  SPF/DKIM/DMARC classification, both of which are WP-56's and therefore Unit
  09's to review. If Unit 09 weakens confidence in either, F-129 and F-130
  escalate together from Medium to account takeover. This is a live
  cross-unit dependency, not a note.
- The three email-borne bearer tokens (settings, unsubscribe, proposal
  invite) have three different lifecycle disciplines and no shared
  abstraction. Only two of the three rotate. Worth one remediation item
  covering all three rather than three separate fixes.
