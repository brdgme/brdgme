# R-07 - `CanonicalEmail` newtype - Lead handover / design

> **Correction note (added 2026-08-07):** This document's references to
> "migration 027" below (the AC4 heading, the
> `rust/web/migrations/027_canonical_email_check.sql` path, and the W3
> worker-plan line) are incorrect. R-07's migration is actually
> `rust/web/migrations/029_canonical_email_check.sql`; `026_canonical_emails.sql`
> is the migration batch's actual first blocker (029 drops and replaces 026's
> index); `027_settings_token_expiry.sql` belongs to R-02 and is unrelated. This
> was established by local verification in commits `c0275c7c` and `0033c50c`.
> The body below is preserved unedited as a historical record of what was
> believed when this handover was written — do not treat its "migration 027"
> references as current fact.

Work package R-07 from `98-REMEDIATION-PLAN.md`. Closes F-124, F-125, F-126,
F-127, F-128, F-173. Branch `master`, base HEAD `3cd727e`.

## Design (decided once; workers follow this exactly)

### The type - `rust/web/src/auth/email_addr.rs`

```rust
/// An email address in canonical form (trimmed, lowercased). The only way to
/// build one is `canonicalize_email`, so a `CanonicalEmail` is always canonical
/// by construction. Store and compare this type - never a raw `String`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CanonicalEmail(String);

impl CanonicalEmail {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::ops::Deref for CanonicalEmail {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

pub fn canonicalize_email(raw: &str) -> CanonicalEmail {
    CanonicalEmail(raw.trim().to_lowercase())
}
```

- `canonicalize_email` is the ONLY constructor. No `From<String>`, no public
  `.0`, no `new`. `as_str`/`Deref` give read-only `&str` access.
- NO custom sqlx `Encode`/`Type` impls. At every bind site, bind `.as_str()`.
  This keeps `sqlx::query!`/`query_as!` macro verification unchanged.
- Existing unit tests in this file: update assertions to use `.as_str()`
  (e.g. `assert_eq!(canonicalize_email(" Foo@X.COM ").as_str(), "foo@x.com")`).

### Why the wire boundary stays `String`

Leptos server fns (`create_proposal`, `restart_game_with_roster`,
`add_proposal_player`, `add_email_address`, `login`, ...) keep `String` /
`Option<String>` / `Vec<String>` parameters - that is a serialization boundary,
not a DB write path. Each server fn canonicalizes immediately on entry (most
already do) and passes `&CanonicalEmail` down to the DB helpers. Client-side
canonicalize calls (`new_game.rs:447`, `settings.rs:416`) canonicalize for
validation, then hand a `String` to the dispatch via `.as_str().to_owned()`.

### DB write-path helper signatures to convert (raw String -> CanonicalEmail)

These are the "write paths" for AC1's grep. After the change, ZERO of them take
a raw `String`/`&str` email.

| fn | file | change |
|----|------|--------|
| `find_or_create_user_by_email_tx` | `proposals.rs:1164` | `email: &str` -> `email: &CanonicalEmail`; bind `email.as_str()` at :1171 and :1194 |
| `CreateGameOpts.opponent_emails` | `db/game_write.rs:22` | `&'a [String]` -> `&'a [CanonicalEmail]`; loop binds `email.as_str()` at :86 and :112 |
| `insert_unverified_email` | `db/emails.rs:86` | `email: &str` -> `&CanonicalEmail`; bind `.as_str()` at :96 |
| `mark_email_verified` | `db/emails.rs:109` | `email: &str` -> `&CanonicalEmail`; bind `.as_str()` at :115 |
| `find_email_owner` | `db/emails.rs:73` | `email: &str` -> `&CanonicalEmail`; bind `.as_str()` at :76 (owner-lookup choke point named in findings) |
| `check_invite_policy_tx` | `db/visibility.rs:173` | `opponent_emails: &[String]` -> `&[CanonicalEmail]`; bind `.as_str()` at :183 |

### Bind-site fixes forced by the return-type change (canonicalize_email now
returns `CanonicalEmail`)

- `auth/server.rs` six sites (:287, :342, :855, :906, :931, :981). `email.is_empty()`
  / `email.contains('@')` still work via `Deref`. Where the canonical `email` is
  bound directly to a query (e.g. confirm_login insert at :495), bind
  `email.as_str()`. Where it is passed to a helper now taking `&CanonicalEmail`,
  pass `&email`. Where passed to a helper that stays `&str` (pure reads like
  `get_user_by_email`), pass `email.as_str()`. Worker must read each of the six
  sites and adjust every downstream use.
- `settings.rs:416`: `let val = canonicalize_email(&el.value()); if !val.is_empty()
  { add_action.dispatch(val.as_str().to_owned()) }`.
- `new_game.rs:447`: `let email = canonicalize_email(&email); if email.is_empty()
  {...}; emails.push(email.as_str().to_owned())` (`emails: Vec<String>`).
- `proposals.rs` create_proposal :1385-1388: `opponent_emails: Vec<CanonicalEmail>
  = ...map(|e| canonicalize_email(&e)).collect()`. The `.is_empty()`/`.contains('@')`
  check at :1389-1394 works via Deref. `check_invite_policy_tx(..., &opponent_emails)`
  and `find_or_create_user_by_email_tx(&mut tx, email)` (email: &CanonicalEmail)
  now type-check.
- `game/server_fns.rs` :1298-1301: same `Vec<CanonicalEmail>` change; :1155
  `check_invite_policy_tx` and :1163-1165 `find_or_create_user_by_email_tx` get
  `&CanonicalEmail`.

### F-124 / F-126 fix - `add_proposal_player` (`proposals.rs:1730-1794`)

Currently `email: Option<String>` reaches `find_or_create_user_by_email_tx` raw
(:1774) and `check_invite_policy_tx` raw (:1781). Restructure so the email is
canonicalized + validated ONCE up front:

```rust
let canonical_email: Option<crate::auth::email_addr::CanonicalEmail> =
    match &email {
        Some(raw) => {
            let c = crate::auth::email_addr::canonicalize_email(raw);
            if c.is_empty() || !c.contains('@') {
                return Err(ServerFnError::new("Invalid email address"));
            }
            Some(c)
        }
        None => None,
    };

let human_id = if let Some(uid) = user_id {
    Some(uid)
} else if let Some(canonical) = &canonical_email {
    Some(find_or_create_user_by_email_tx(&mut tx, canonical).await?)
} else {
    None
};
```

and the policy check:

```rust
let policy_emails: Vec<crate::auth::email_addr::CanonicalEmail> =
    canonical_email.clone().into_iter().collect();
let violations =
    crate::db::check_invite_policy_tx(&mut tx, user.id, &policy_ids, &policy_emails)
        ...
```

This rejects empty/`@`-less email BEFORE any account is created (closes F-126's
junk-account+500 and F-124's ghost account / policy bypass).

### F-128 / F-173 fix - `from_matches_verified_email` (`inbound.rs:535-548`)

Move comparison OUT of SQL; canonicalize in Rust so both sides use the same
full-Unicode `to_lowercase`:

```rust
async fn from_matches_verified_email(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    from: &str,
) -> anyhow::Result<bool> {
    let canonical = crate::auth::email_addr::canonicalize_email(from);
    let (exists,): (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM user_emails WHERE user_id = $1 AND verified_at IS NOT NULL AND email = $2)",
    )
    .bind(user_id)
    .bind(canonical.as_str())
    .fetch_one(pool)
    .await?;
    Ok(exists)
}
```

No caller changes needed (callers at :770, :871, :1421 pass `from: &str`).
Existing truth-table test (:2302) still passes: stored rows are canonical, and
`canonicalize_email("VERIFIED@brdg.me") == "verified@brdg.me"`.

### AC5 - delete the prose contract comments

The canonicalization contract is now enforced by the type, so the prose
"callers must pass canonicalized" comments must be DELETED (not kept alongside):
- `db/emails.rs:71` - delete the line `/// Callers must pass a canonicalized address (see auth::email_addr::canonicalize_email).`
- `db/visibility.rs:171` - delete the line `/// Callers must pass canonicalized addresses (see auth::email_addr::canonicalize_email).`
- `auth/email_addr.rs:1-2` - the imperative "Every boundary that stores or looks
  up an address must call this first" is replaced by the type-level doc above.

### AC4 - migration 027 (NEW; 026 is IMMUTABLE)

`rust/web/migrations/027_canonical_email_check.sql`:

```sql
-- R-07 / F-125: enforce canonical storage and align the unique index with the
-- backfill expression used in 026 (lower(btrim(email))). Migration 026 is
-- immutable; this adds the missing CHECK and replaces the lower(email) index
-- (which disagreed with the btrim backfill) with one on the same expression.

ALTER TABLE public.user_emails
    ADD CONSTRAINT user_emails_email_canonical_chk
    CHECK (email = lower(btrim(email)));

DROP INDEX IF EXISTS public.user_emails_email_lower_key;
CREATE UNIQUE INDEX user_emails_email_canonical_key
    ON public.user_emails (lower(btrim(email)));
```

Migration test (AC4): a `#[sqlx::test]` that introspects `pg_indexes` /
`pg_constraint` to assert the unique index expression is `lower(btrim(email))`
(same as the 026 backfill) and the CHECK exists; then seeds trim-variant
duplicates and asserts the index/CHECK reject them. The reconciliation count
("trim-duplicate rows found") is recorded by the test against the migrated
schema; the production-data count is deferred to CI/operator (no populated DB
locally; DB tests do not run on this machine).

## Tests to write (compile-verified ONLY; execution deferred to CI)

- AC2: `#[sqlx::test]` calling `from_matches_verified_email` with `İ@example.com`
  (U+0130). Store the verified address via `canonicalize_email("İ@example.com")`
  and assert the lookup returns true; assert the Rust canonical string is what
  the exact-match SQL needs (i.e. Rust path == SQL path).
- AC3: `#[sqlx::test]` calling `add_proposal_player` with a raw `" Foo@x.com "`,
  a non-canonical `"BAR@x.com"`, and an empty `""`, asserting NO verified
  `user_emails` row is created in any case and the empty case errors cleanly
  (no 500/junk account). NOTE: `add_proposal_player` is a `#[server]` fn that
  pulls `expect_context` - the test may need to call the inner logic or set up
  context. Worker: if `expect_context` blocks a direct call, extract the
  email-handling core into a testable helper OR drive it through the test harness
  the way existing proposal tests do. Investigate existing `proposals.rs` tests
  (e.g. :3755-3787) for the pattern. Keep it compiling.

## Gates

- `cargo check -p web --all-targets --features ssr`
- `cargo clippy -p web --all-targets --features ssr -- -D warnings`
- `cargo fmt --all -- --check`
- BANNED: `cargo build/test/run -p web`, workspace-wide builds, `scripts/rust-test.sh`.

## Worker plan (serial)

1. W1: newtype + full threading + AC5 comment deletion + from_matches impl
   change. Ends compiling + clippy green + commit.
2. W2: AC2 + AC3 tests (compile-verified). Commit.
3. W3: migration 027 + AC4 migration test. Commit.

Commit message: `fix(web): <summary> (R-07, F-124, F-125, F-126, F-127, F-128, F-173)`.
Stage named files only. Never push.
