# Concede with Bot Replacement & End Game (#47)

Date: 2026-07-23
Status: Draft
Relates to: #46 (turn timer), #43 (bot efficacy)

## Problem

Conceding is only supported in 2-player games, where it ends the game
immediately. In games with more players there is no way to leave: a player
who stops responding stalls the game for everyone else.

Design input lives in `docs/bot-improvements-input.md`. This spec resolves
the open questions flagged there and in backlog #47.

## Goals

1. Let a player concede in any unfinished game. They are replaced by a bot
   so the remaining players can continue.
2. Distinguish "game placings" (in-game result, including bot performance)
   from "ranked placings" (used for ELO and Form, where conceding and
   elimination are treated equally).
3. When only one human remains, give them an "End game" button instead of
   "Concede" so they can stop the game (and the bots) at will.
4. When all humans are gone via elimination, let bots play out naturally.
   The last human can still end the game early.
5. Make the replacement bot configurable via an admin flag, so future turn
   timers (#46) can reuse the same mechanism for slow players.
6. Never let bot performance affect rating changes.

Non-goals (deferred):
- Turn timers / replacing slow players (#46) - reuses the replacement-bot
  mechanism designed here but is not implemented.
- Auto-stop / timeout for bot-vs-bot play - bots play to natural completion
  for now. Revisit if LLM spend becomes a concern.

---

## D1: Data Model

### game_players

The existing `game_bot_id` FK is reused to represent a replaced human. The
current CHECK constraint enforces that exactly one of `user_id` /
`game_bot_id` is set. A new migration relaxes it to allow both:

- `user_id` set, `game_bot_id` NULL - normal human player.
- `user_id` NULL, `game_bot_id` set - pure bot (game started with a bot).
- Both set - a human who conceded and was replaced by a bot. The row keeps
  `user_id` so the player name and link still point at the original player.

New columns:

- `ranked_placing INT NULL` - the placing used for ELO and Form. Populated
  for human players when the game finishes. NULL for pure bots.
- `left_at TIMESTAMPTZ NULL` - when the player conceded or was eliminated.
  Used to order ranked placings.

The existing `placing` column is unchanged and becomes the "game placing" -
the in-game result, computed at game end including bot performance.

### game_bots

- `can_replace_humans BOOLEAN NOT NULL DEFAULT FALSE` - admin flag marking
  a bot as eligible to take over for a conceding player.

### Migration notes

- Migrations are immutable once applied (see AGENTS.md). This is a new
  numbered migration that drops the old XOR CHECK and adds
  `CHECK (user_id IS NOT NULL OR game_bot_id IS NOT NULL)`, plus the new
  columns above.

## D2: Placing Algorithm

Two distinct placing concepts:

**Game placing** (existing `placing`): the in-game result, computed at game
end from final scores / elimination order. Includes bot performance. A
replaced player's game placing reflects what their replacement bot achieved.

**Ranked placing** (new `ranked_placing`): the placing used for ELO and
Form. Conceding and elimination are treated equally - both mean the player
left the active pool.

Algorithm (run once when the game finishes):

1. Pure bots (`user_id IS NULL`) get `ranked_placing = NULL`.
2. Order all players who conceded or were eliminated by `left_at` ascending.
   The earliest to leave gets the worst ranked placing, the latest to leave
   gets the best among this group.
3. Players still active at game end (never left) fill the remaining ranked
   placings, ordered by their game placing relative to each other. These are
   always better than any leaver's ranked placing.

Worked example (4 players): A eliminated t=1, B concedes t=2 (bot replaces),
C eliminated t=3, D survives. Replacement bot B wins on points.

- Game placings: bot-B=1, D=2, C=3, A=4.
- Ranked placings: D=1 (survivor), C=2 (left t=3), B=3 (left t=2), A=4
  (left t=1).

Note the distinction: "player 2 (bot: Hard)" won the game (game placing 1),
but "player 2" is ranked second-last because they conceded.

### Prerequisite: verify elimination mechanics

This algorithm assumes eliminations set `left_at` correctly. Games with
elimination mechanics must be audited during implementation to confirm
eliminated players are flagged at the right time, so ranked placings can be
trusted.

## D3: Concede Flow

**"Concede" button is shown when:**
- The viewer is an active human player (not eliminated, not replaced).
- The game is not finished.
- Either replacement bots are configured, OR no replacement bots are
  configured and exactly 2 humans remain (current behavior).

**On concede, replacement bots available (any game size):**
1. Set `left_at = now()`.
2. Set `game_bot_id` to a randomly selected bot where
   `can_replace_humans = true`.
3. The player's name and link are unchanged. The UI shows a replacement
   indicator (see D6).
4. If it is now the replaced slot's turn, trigger a bot turn.
5. If active humans drops to 1, that human's UI switches to "End game".
6. The game continues.

**On concede, no replacement bots, exactly 2 humans:**
- Current behavior unchanged: the game ends immediately, the conceder
  accepts the loss.

**On concede, no replacement bots, more than 2 humans:**
- Concede is not available (button hidden). There is no one to take the
  seat, so the game must play out.

`ranked_placing` is NOT set at concession time - it is computed for all
players at game end from `left_at` order (D2). Only `left_at` and
`game_bot_id` are set on concession.

## D4: End Game Flow

**"End game" button is shown when:**
- The viewer is the last active human (`active_humans == 1`), OR
- The viewer was the last human and has been eliminated, with bots still
  playing out the game.

**On end game:**
1. Set `is_finished = true`, `finished_at = now()`.
2. Compute game placings from the current game state (scores / positions).
3. Compute ranked placings for all human players (D2).
4. Apply rating changes for human players.
5. Show the "Restart game" option.

The last human's ranked placing is 1 - they are the last human standing,
regardless of score. "End game" is a voluntary stop, not a concession.

## D5: All Humans Gone (via elimination)

If the last human is eliminated (rather than conceding), zero humans remain
and only bots are left:

- Bots continue playing to natural completion. The game is not force-ended.
- The last human (now eliminated, ranked placing 1) can still click
  "End game" at any point to stop the bots early and reveal "Restart game".
- When the game finishes (naturally or via "End game"), game placings are
  computed from the final state and ranked placings from `left_at` order.
- Ratings are applied once, at game finish.

## D6: Ratings / ELO

- Rating exclusion changes from `game_bot_id IS NOT NULL` to
  `user_id IS NULL` (pure bots only). Replaced humans still have `user_id`
  set and are rated.
- Replaced humans are rated using their `ranked_placing`, which is locked by
  concession order and unaffected by how well their replacement bot plays.
- Remaining humans are rated using their `ranked_placing` from game end.
- Rating changes apply once, at game finish, for all human players together.
- Bot performance never affects rating changes - not in bot-vs-human games,
  not in multi-human games with bots, not for replaced players.
- Games with zero human players (pure bot games): no ratings, no Form.

## D7: UI / Display

**Replaced player display:** the original player's name and link (to their
player page) are preserved. A replacement indicator shows the bot, e.g.
"PlayerName (bot: Hard)".

**Buttons:**
- "Concede" - per D3 rules.
- "End game" - per D4 rules. Replaces "Concede" for the last human.

**Form lines:**
- Include games where the viewer was a human player, including games where
  they were later replaced (they played part of it).
- Exclude games where the viewer was a pure bot (`user_id IS NULL`).
- Use `ranked_placing`, not `placing`.
- Show the last 5 games only.
- Most recent game on the left.
- Only the most recent (leftmost) entry is bold; the rest are normal weight.

## D8: Admin

- Bot management page: add a "Can replace humans" checkbox per bot.
- Replacement selection:
  `SELECT id FROM game_bots WHERE can_replace_humans = true ORDER BY random() LIMIT 1`.
- If no bots are flagged, concede degrades per D3.

## D9: Bot Turn Triggering

Bot turns currently trigger for slots where `game_bot_id IS NOT NULL`. Since
a replaced human now has `game_bot_id` set, the existing trigger logic works
for replacements with no change.

---

## Open Items / Implementation Notes

- Audit elimination mechanics across all games to confirm `left_at` will be
  set correctly (D2 prerequisite).
- The email command path (`rust/web/src/email/commands.rs`) needs the same
  concede changes as the web server fn, plus an "end" verb or equivalent.
- `is_2player` gating in the UI (`rust/web/src/components/game.rs`) is
  replaced by the active-human-count logic in D3/D4.
