# 35: User Settings Page (display name, colour prefs) - Design

**Status:** Decided 2026-07-11 - pre-beta (wanted in place before the #16
beta period). This is a point-in-time decision record, not a living
document.

**Problem:** users have no way to change anything about themselves. The
display name is derived from the email localpart at signup (leaks part of
a sensitive identifier, unconstrained charset/length, collisions
possible) and can never be changed. `users.pref_colors` is dead code in
the new stack: always written as an empty array, never read - game
creation assigns colours purely by shuffled position index into a
hardcoded palette (`rust/web/src/db.rs` ~679-710). The legacy stack had a
preference-satisfaction algorithm (`rust/api/src/db/color.rs::choose`)
but no UI ever existed to set preferences in either stack, so the
algorithm never ran on real data.

## Decisions (2026-07-11)

- **D1 - `/settings` route, linked from the sidebar.** Contains display
  name and colour preferences. (A browser-notification toggle was
  considered and moved to #36 Web Push, which is deferred post-go-live.)
- **D2 - display name rules: unique, short, ASCII.** Names must match
  `^[a-zA-Z0-9_-]{1,16}$` (no whitespace - fits the ASCII-art theme) and
  be unique case-insensitively (`CREATE UNIQUE INDEX ... ON users
  (lower(name))`), so `Sam` cannot be impersonated by `sam`. Rationale
  for uniqueness: email addresses are sensitive and must never be
  exposed, so the display name is the public identifier users will
  search and friend each other by (#30). Rejected for now: Discord-style
  `nickname#1234` discriminators - a possible later pivot if uniqueness
  contention ever hurts, not wanted yet (Michael, 2026-07-11).
- **D3 - default names are generated petnames, not email localparts.**
  Signup stops deriving the name from the email entirely. Use the
  `petname` crate to generate names like `big-scary-walrus`,
  regenerating until the result satisfies D2 (<= 16 chars, charset,
  unique) - library word lists include long words, so a
  regenerate-until-fit loop is expected and cheap. This removes the need
  to sanitise localparts or handle localpart collisions. A one-off
  migration regenerates names for any existing rows that violate D2
  (invalid chars, too long, or case-insensitive duplicates).
- **D4 - colour preferences are an ordered pick of 3.** The user picks
  up to 3 distinct colours, in preference order, from the canonical
  7-colour palette (Green, Red, Blue, Amber, Purple, Brown, BlueGrey).
  Stored in the existing `users.pref_colors` text array. Game creation
  ports the legacy `choose()` algorithm from
  `rust/api/src/db/color.rs` (try everyone's first preference, then
  second, etc.; unresolved players take from the remaining palette),
  replacing the fixed-index assignment in `rust/web/src/db.rs`. Colours
  remain decided once at game creation and persisted per
  `game_players.color` row - existing games are untouched. Cleanup
  opportunity while in here: the palette list currently exists in three
  places (legacy `COLORS`, the hardcoded array in `rust/web/src/db.rs`,
  and `brdgme_color::player_colors()` in `rust/lib/color`); `rust/web`
  should use the shared `brdgme_color` copy.

## Non-goals

Notification toggle (moved to #36), avatar/profile images, email address
management (#22d multi-email switching covers that), name-change history
or cooldowns, discriminators (D2), editing colours of in-progress games
(D4).
