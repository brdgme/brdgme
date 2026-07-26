# WP-50: email canonicalization

**Findings:** ws F9 (CONFIRMED by `findings/verification/web-server.md`), wd
F37, wd F60, wd F72 - all minor, all re-derived live below. **Decision:** D-9
answered **option B** - boundary normalization **plus** a one-off migration
lowercasing stored rows **plus** a `lower(email)` unique index. Boundary-only
was explicitly rejected.

**Landing order: WP-82 -> WP-50.** WP-82 (the `db.rs` module split) lands
**first**; this package rebases onto the post-split tree, where its `db.rs`
doc-comment edits land in the relevant `db/*.rs` submodule. (The old header
here said "WP-78 (`db.rs` split) is deferred until this lands" - **withdrawn**:
WP-78 is SUPERSEDED by WP-82 and the direction is reversed. See
`landing-order.md` 7.3.) WP-56 Task 4 deletes the `emails add/confirm/active/remove` verbs
in `email/commands.rs`; those call sites are excluded here as dead code.

> **Read the named functions before editing. If one does not match what this
> spec describes, STOP and report rather than improvising.** This code is under
> concurrent edit; no line numbers are cited on purpose.

## 1. Problem

- **ws F9** - `login`, `confirm_login`, `add_email_address`
  (`rust/web/src/auth/server.rs`) use the raw client string as the
  `login_confirmations` PK and the `user_emails.email` value. No trim, no
  lowercase; only the *domain* is lowercased, and only for the blocklist check.
- **wd F37** - `find_or_create_user_by_email_tx` (`rust/web/src/proposals.rs`)
  and `check_invite_policy_tx` (`rust/web/src/db.rs`) match `user_emails.email`
  by exact string; `create_proposal` passes `opponent_emails` through untouched.
- **wd F60** - `on_submit` (`rust/web/src/new_game.rs`) pushes
  `OpponentSlot::Email(email)` verbatim while Player slots are validated.
- **wd F72** - `on_add_submit` (`rust/web/src/settings.rs`) dispatches
  `el.value()` raw to `add_email_address`.

## 2. Why it's wrong

- **All four findings are correct as written.** Verified live; none is stale.
  Do not revert any of them.
- `user_emails.email` carries a case-sensitive text `UNIQUE`
  (`user_emails_email_key`, migration `001_initial_schema.sql`): `Foo@x.com` and
  `foo@x.com` are two rows, two accounts, one mailbox.
  `login_confirmations.email` is a text **primary key** (migration `005`), so a
  case variant gets its own code row and the code never matches on confirm.
- **Correction to the ws F9 verification note:** the text PK in `005` is
  `login_confirmations.email`, **not** `user_emails.email` - that table's PK is
  `id uuid`. No FK anywhere references `user_emails.email`, `user_emails.id` or
  `login_confirmations.email`; the only FK on the table is
  `user_emails_user_id_fkey -> users(id)`, untouched by a lowercasing UPDATE.

## 3. Required end state

### 3a. One canonical helper - `rust/web/src/auth/email_addr.rs` (new)

New module, declared in `rust/web/src/auth/mod.rs`. It must **not** be
`#[cfg(feature = "ssr")]` (copy `blocked_domains.rs`) - the two client
boundaries call it too.

```rust
/// Canonical form of an email address: trimmed, lowercased. Every boundary
/// that stores or looks up an address must call this first.
pub fn canonicalize_email(raw: &str) -> String {
    raw.trim().to_lowercase()
}
```

Policy: `str::trim` (Unicode whitespace, both ends), then `str::to_lowercase`
(Unicode, **not** ASCII-only, so it agrees with Postgres `lower()`). Emptiness is
**not** the helper's job - each caller keeps its own existing validation and runs
it after canonicalizing. Residual Rust/PG divergence on exotic input fails closed
on the unique index as the existing "Address unavailable".

Do **not** canonicalize inside `db.rs` (`find_email_owner`,
`insert_unverified_email`, `mark_email_verified`, `set_primary_email`,
`check_invite_policy_tx`) or `find_or_create_user_by_email_tx` - they stay
exact-match. Add a one-line doc comment to `find_email_owner` and
`check_invite_policy_tx`: callers must pass a canonicalized address.

### 3b. `rust/web/src/auth/server.rs`

In each of `login`, `confirm_login`, `add_email_address`,
`confirm_email_address`, `make_email_address_active`, `remove_email_address`:
as the **first** statement, `let email = crate::auth::email_addr::canonicalize_email(&email);`.
The existing empty/`'@'`, plus-addressing and blocked-domain guards then run on
the canonical value, otherwise unchanged. The last three already receive stored
(post-migration canonical) values; the call is defence in depth, one line each.

### 3c. Invite emails

`create_proposal` (`rust/web/src/proposals.rs`) and `restart_game_with_roster`
(`rust/web/src/game/server_fns.rs`): right after
`let opponent_emails = opponent_emails.unwrap_or_default();`, map every entry
through `canonicalize_email`, then reject the whole call with
`ServerFnError::new("Invalid email address")` if any entry is empty or lacks
`'@'` - **before** `check_invite_policy_tx` or `find_or_create_user_by_email_tx`
see the values.

### 3d. Client boundaries

- `on_submit` (`rust/web/src/new_game.rs`): the `OpponentSlot::Email(email)` arm
  canonicalizes; if empty, set `form_error` and `return`, mirroring the existing
  unselected-Player arm.
- `on_add_submit` (`rust/web/src/settings.rs`): canonicalize `el.value()` before
  the `is_empty()` check; dispatch the canonical string.

### 3e. Migration `rust/web/migrations/0NN_canonical_emails.sql` (new)

**The number `023` is NOT guaranteed.** Four packages this cycle each add a
migration and only the first to land gets `023` - see `landing-order.md`
section 6.4 for the renumbering rule. `ls rust/web/migrations/` immediately
before writing the file and take the next free number; do not trust the number
written here or in the `RAISE EXCEPTION` message below (update that message to
match whatever number you use). Three steps, in order:

1. `DELETE FROM login_confirmations;` - its PK *is* the address, so lowercasing
   could collide there too. Rows are 1-hour ephemeral codes the app already GCs
   opportunistically; purging is deterministic and costs at most a re-request.
2. **Abort on collisions, naming them.** Two stored addresses differing only by
   case become one when lowercased, and the index below would fail:

   ```sql
   DO $$
   DECLARE dups text;
   BEGIN
       SELECT string_agg(k, ', ' ORDER BY k) INTO dups
       FROM (SELECT lower(btrim(email)) AS k FROM public.user_emails
             GROUP BY 1 HAVING count(*) > 1) d;
       IF dups IS NOT NULL THEN
           RAISE EXCEPTION
             'migration 023: case-duplicate addresses must be merged by hand first: %', dups;
       END IF;
   END $$;
   ```

   **Abort, never auto-resolve** - collapsing two rows owned by different users
   means merging two accounts (games, ratings, friendships), which no migration
   can do deterministically. This is D-9's "surface the risk once, deliberately".
   Operator pre-flight query: `SELECT lower(btrim(email)) AS canonical,
   array_agg(email), array_agg(user_id) FROM public.user_emails GROUP BY 1
   HAVING count(*) > 1;`
3. `UPDATE public.user_emails SET email = lower(btrim(email)) WHERE email <>
   lower(btrim(email));` then `CREATE UNIQUE INDEX IF NOT EXISTS
   user_emails_email_lower_key ON public.user_emails (lower(email));` Leave the
   existing `user_emails_email_key` in place - redundant once rows are canonical
   but harmless, and dropping it is separate risk.

## 4. Non-goals

- The `LOWER(email) = LOWER($2)` call sites in `rust/web/src/email/inbound.rs`
  become redundant once rows are canonical. **Leave them** - cheap, defensive,
  and that file belongs to WP-56/WP-59 this cycle. Address *parsing*
  (`parse_reply_address`, display-name From) is WP-59.
- `emails add/confirm/active/remove` in `rust/web/src/email/commands.rs`:
  deleted by WP-56 Task 4, **no-op here**. Do not normalize dead code.
- No `citext`, no account merging, no `db.rs` restructuring (that is WP-82;
  WP-78 is superseded by it), no change to the plus-addressing or
  blocked-domain rules.

## 5. Regression test cases

- **`rust/web/src/auth/email_addr.rs`, new `#[cfg(test)] mod tests`:**
  `" Foo@X.COM "` -> `"foo@x.com"`; canonical input unchanged; `"   "` -> `""`.
- **`rust/web/src/auth/server.rs`, existing `mod tests`** (has the
  `with_pool_context` pool harness): `add_email_address(" Foo@X.com ")` stores
  `foo@x.com`; `login` then `confirm_login` succeeds when the two disagree on
  case; adding a case variant of an address already on the account is rejected
  as "Address already on your account", not inserted.
- **`rust/web/src/proposals.rs`, existing `mod tests`:** inviting `"Foo@x.com "`
  when `foo@x.com` is registered resolves to the existing `user_id` and creates
  **no** second `users` row; an empty email slot makes `create_proposal` return
  "Invalid email address".
- **Migration test**, using the `include_str!("../../migrations/...")` pattern
  already in `rust/web/src/stats/queries.rs mod tests`: two rows differing only
  by case make `023` raise with the canonical address in the message;
  `" Foo@X.com "` alone becomes `foo@x.com` and the new index then rejects
  `FOO@X.COM`.

## 6. Riders

None - all four findings are handled above.
