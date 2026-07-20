# Bot Improvements - Design Input

Collected feedback for a dedicated bot-improvement design session.
Relates to backlog #47 (concede with bot replacement) and #43 (bot efficacy).

## Deferred features needing design

### Concede -> bot takeover (#47)
- When a player concedes in a >2 player game, a bot takes over for them.
- Conceding still counts as a loss; finishing position is locked at concession time.
- If only one human remains vs bots, button changes from "Concede" to "End game".
- Placing and ELO adjustments lock in when the second-last human concedes/is eliminated.
- Edge case: player A eliminated, player B concedes, player C wins -> placings C, B, A.
- Score-based games: even if the bot plays well post-concession, the conceded player
  still loses. Rating adjustments based on locked position, not bot performance.

### All-humans-gone -> stop game
- If all human players are eliminated or resign, the game should stop immediately
  (not continue bot-vs-bot in the background - wasteful of resources and LLM spend).
- Edge cases around placings need design: what placing do eliminated humans get
  relative to each other and relative to surviving bots?

## Bot-specific feedback (to be expanded by Michael)

- Bots are currently included in rating adjustments after games - must be 100% excluded.
- (Add further bot feedback here)
