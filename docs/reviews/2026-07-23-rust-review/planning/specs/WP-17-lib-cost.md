# WP-17: lib/cost consolidation (splendor-2 port)

**Findings:** `b F31`, `ls F39`, `dp F27`. **Decision:** D-25 ANSWERED
2026-07-26 - **option A, port splendor-2 onto the shared `brdgme_cost` crate**.
**Status:** READY.

> **Read the named functions before editing. If one does not match what this
> spec describes, STOP and report rather than improvising. No line numbers are
> cited on purpose - the tree is under concurrent edit.**

*Deliberately over the ~120-line Tier 2 cap (Lead-accepted): section 4 is
Michael's binding D-25 testing constraint and must not be compressed into a
one-liner, and section 1 has to carry the equivalence proof that makes the
port safe.*

**Scope split - do not widen.** WP-17 carries 8 findings. This spec is *only*
the three that are one indivisible consolidation: `b F31`, `ls F39`, `dp F27`.
The other five - **`b F30`, `b F32`, `b F34`, `b F35`, `ls F38`** - are
independent rows in `planning/checklists/T3-B3-splendor-libcost-holdem.md` and
are **NOT in this spec**. In particular **`ls F38`** (moving `Cost::new()` into
an impl block bounded only by `Hash + Eq`) is a checklist row; do not do it
here. Option B (deleting `lib/cost`) is closed and must not be substituted.

## 1. Findings verdicts - state these so nobody reverts them

- **`b F31` CORRECT.** `rust/lib/cost/src/lib.rs` has no `get`/`set`; the
  gold-joker `can_afford` free function in `rust/game/splendor-2/src/cost.rs`
  has no lib equivalent. Four splendor-2 files touch `Cost` (`lib.rs`,
  `render.rs`, `card.rs`, `player_board.rs`). Verification's wording nit stands:
  the lib type is `Cost<K>`, not literally `Cost(HashMap<Resource, i32>)`.
- **`ls F39` CORRECT.** Re-checked against live code: splendor's
  `from_resources`/`add`/`inv`/`sub`/`sum` and the *method* `Cost::can_afford`
  **are semantically equivalent** to the lib's `from_keys`/`add`/`inv`/`sub`/
  `sum`/`can_afford`. The only non-obvious one is `can_afford`: splendor asserts
  `self.sub(other)` has all values `>= 0`; the lib asserts the `neg` half of
  `sub(other).pos_neg()` is empty, and `pos_neg` files a key into `neg` exactly
  when its value is `< 0`. Same predicate. Serde shape is identical (both are
  newtype tuple structs over `HashMap<_, i32>`), so persisted states survive.
- **`dp F27` CORRECT** (lead-verified only, re-derived here). Grepping every
  manifest confirms `brdgme_cost` is depended on by exactly one crate,
  `rust/game/seven-wonders-1`, while splendor-2 carries a parallel
  implementation. Resolved as port-onto.

## 2. Add generic `get`/`set` to `lib/cost`

File: `rust/lib/cost/src/lib.rs`. Add to the **existing
`impl<K: Hash + Eq + Clone> Cost<K>` block** (the one holding `new`, `add`,
`sub`, ...). Do **not** open a new impl block - `ls F38` may later rearrange
these bounds and a competing block would collide with it.

```rust
#[must_use]
pub fn get(&self, k: &K) -> i32 {
    self.0.get(k).copied().unwrap_or(0)
}

pub fn set(&mut self, k: K, v: i32) {
    self.0.insert(k, v);
}
```

- `get` takes `&K`, not `K`: the lib is generic and must not force non-`Copy`
  keys to be cloned or moved for a read. This is the one behavioural difference
  from splendor's `get(&self, r: Resource)` and it is why call sites need `&`.
- The tuple field is already `pub`, so these are **not strictly necessary** -
  callers can reach the map. They are wanted for encapsulation and because
  splendor-2 has ~41 `.get(..)`/`.set(..)` call sites that would otherwise all
  become `.0` map pokes. Both are additive; adding them cannot break
  seven-wonders-1 (it defines no `get`/`set` on `Cost`, inherent or by trait).

## 3. Port splendor-2

**`rust/game/splendor-2/src/cost.rs` shrinks to a type alias plus the retained
gold-joker function.** Keep the file - every importer already says
`use crate::cost::{self, Cost}`, so the diff stays local:

```rust
use crate::card::Resource;

pub type Cost = brdgme_cost::Cost<Resource>;

/// Splendor's own affordability check: folds a gold reserve in to cover any
/// per-resource shortfall. Keep the existing body, adjusted for `get(&k)`.
pub fn can_afford(a: &Cost, c: &Cost) -> bool { /* unchanged logic */ }
```

**The gold-joker `can_afford` is splendor-specific and MUST NOT be moved into
`lib/cost`.** It encodes a Splendor rule (gold is a wild token), not a general
cost operation. It stays a crate-local free function in splendor-2. Anyone who
"tidies" it into the shared crate has broken the decision.

Delete from `cost.rs`: `struct Cost`, `new`, `from_resources`, `get`, `set`,
`add`, `inv`, `sub`, the `Cost::can_afford` **method** (not the free function),
`sum`, and the now-unused `use serde::{..}`.

**Trap:** do **not** blanket-delete `use std::collections::HashMap` from the
file. It leaves the top-level scope, but the retained `#[cfg(test)] mod tests`
builds its fixtures with `Cost(HashMap::from([..]))`, so the test module needs
its own `use std::collections::HashMap;`. Move the import rather than dropping
it, or the retained gold-joker test will not compile.

`rust/game/splendor-2/Cargo.toml`, under `[dependencies]`:

```toml
brdgme_cost = { path = "../../lib/cost" }
```

### Call-site fixes (mechanical; find them by compiling, not by list)

- **`.get(x)` -> `.get(&x)`** everywhere (~41 sites). Affected: `render.rs`
  (`render_amount`, and the player/bank token cells), `lib.rs` (`pay`, `take`,
  the reserve/gold path, `new`/setup) and their tests. `.set(..)` is unchanged.
- **`Cost::from_resources(tokens)` -> `Cost::from_keys(tokens.iter().copied())`**
  in `lib.rs` - `take` and the return-tokens path. Two sites.
- `cost::can_afford(..)` in `render.rs::card_cells`,
  `player_board.rs::PlayerBoard::can_afford` and `lib.rs`'s noble-visit filter
  is **unchanged** - same function, same module path.
- `Cost(..)` tuple construction still works through the alias: the `cost!` macro
  in `card.rs` and the test helper in `lib.rs` need no change. `Cost::new()` and
  `Cost::default()` still resolve (`Resource` is `Clone + Copy + Hash + Eq`).

## 4. Required testing - NOT optional

Michael's standing D-25 constraint: `lib/cost` gains a second consumer and must
carry its own automated tests. `rust/lib/cost/src/lib.rs` **already has a
substantial `#[cfg(test)] mod tests`** covering `add`, `inv`, `sub`, `pos_neg`,
`can_afford`, `take`, `drop`, `keys`, `to_keys`, `is_zero`, `trim`, `sum`,
`from_keys` and 10 `can_afford_perm` cases via a local `TestRes` enum and a
`cost(&[(TestRes, i32)])` helper. **Do not rewrite, reorganise or churn those
tests.** The constraint binds on two additions:

**(a) The new `get`/`set`, in `lib/cost`'s test module.** Done when all of
these are asserted: `get` on a **missing** key returns 0; `get` on a present
non-zero key returns the value; `get` on a key explicitly set to 0 returns 0;
`set` **inserts** a key that was absent; `set` **overwrites** an existing value
(including to a negative value); `set(k, 0)` leaves an explicit zero entry, so
`trim`/`keys` drop it while `sum` and `get` still see it.

**(b) Equivalence coverage, so the port is provably behaviour-preserving.**
splendor-2's `cost.rs` tests today are `test_cost_clone`, `test_cost_add`,
`test_cost_inv`, `test_cost_sub`, `test_cost_can_afford`, `test_cost_sum`,
`test_can_afford`. Before deleting any of them, **check each against its lib
counterpart** (`test_clone`, `test_add`, `test_inv`, `test_sub`,
`test_can_afford`, `test_sum`) and confirm the lib test asserts the same shape;
delete only the ones that are genuinely covered. **`test_can_afford` - the
gold-joker one - has no lib counterpart and MUST be retained** in splendor-2
alongside the function it tests, and **extended**: exact payment with no gold;
gold covering the shortfall exactly; gold one short; a cost that itself names
`Resource::Gold` (the `c.get(&Gold)` subtraction path); an empty cost; and a
shortfall spread across two resources. Additionally add a **serde round-trip
test of a serialized splendor `Game`** to pin persisted-state compatibility -
the two `Cost` shapes serialize identically, but the test locks it.

## 5. Non-goals

- `b F30`, `b F32`, `b F34`, `b F35`, `ls F38` - checklist rows, not this spec.
- Any other `lib/cost` API change: no new operations, no bound changes, no
  moving `can_afford_perm`, no touching the existing tests.
- Any change to `rust/game/seven-wonders-1`. It must compile and pass untouched.

## 6. Verification

Read-only, after the change:

- `grep -rn 'from_resources' rust/` -> zero hits.
- `grep -n 'pub fn can_afford\|pub struct Cost\|pub type Cost'
  rust/game/splendor-2/src/cost.rs` -> the free function and the type alias
  only, no struct.
- `grep -rn 'brdgme_cost' rust/game/splendor-2/Cargo.toml` -> one hit.
- `grep -rn 'Gold\|gold' rust/lib/cost/src/lib.rs` -> zero hits.

Builds (AGENTS.md forbids workspace-wide builds on dev machines - use `-p`;
package names taken from the manifests):

- `cargo test -p brdgme_cost` and `cargo clippy -p brdgme_cost --all-targets`
- `cargo test -p splendor-2` and `cargo clippy -p splendor-2 --all-targets`
- `cargo test -p seven-wonders-1` - the **no-regression check** for the existing
  `brdgme_cost` consumer. If this goes red, the lib change was not additive.
