# Feature Ideas (Brainstorm)

STATUS: brainstorm for review - NOT committed direction. Nothing here is
scheduled, scoped, or approved. These are candidate ideas to discuss.

Ground rules used while writing this:
- Ideas must fit brdgme's async, text-first, play-by-email character.
- Ideas already in docs/VISION.md (planned features) or docs/BACKLOG.md
  are excluded. Where an idea extends an existing backlog item, it says so.
- Sizes: S = days, M = one to few weeks, L = a quarter-scale swing.

## Top picks

The five most compelling, in rough priority order:

1. Spectate live games (M) - cheap to build on the existing SSE fan-out,
   high social and learning value, and it is the missing "third place"
   between the lobby and a player's own games.
2. Game tags and a filterable library (M) - with 27 games and growing,
   discovery is becoming a real problem; this also de-risks onboarding and
   extends backlog #44 (new game screen usability).
3. Email digest mode (M) - directly reinforces the play-by-email identity
   while solving notification fatigue for players in many concurrent games.
4. Turn timeline scrubber (M) - reuses game_logs to let players rewind a
   game visually; great for async review, learning, and post-game analysis.
5. Daily challenge vs a bot (L) - the biggest swing here; a daily curated
   scenario gives a reason to return and showcases the bot system, but it
   needs per-game scenario authoring so it is the riskiest of the five.

---

## Social and community

### Spectate live games (M)
Read-only, real-time view of an in-progress public game. Anyone can open a
game they are not playing and watch the board update live via the existing
SSE streams. Value: entertainment, learning by watching, and a social hub
beyond one's own games. Implementation: new route (e.g. /games/{id}/watch)
subscribing to the same game.{id} NATS subject the players use, a read-only
render path, and a "spectatable" flag. Risks/open questions: hidden-info
games (poker, Liar's Dice) need a fog-of-war spectator render that hides
other players' private state; how to avoid spoilers for players who want to
stay current.

### Rematch / "run it back" (S)
One-click rematch from a finished game: same players, same settings, new
game linked to the original. Value: removes friction from the most common
next action after a close game. Implementation: a domain action that clones
the setup into a new game and reuses the existing invite/notification path;
a button on the game page and in the post-game email. Risks: player set may
have changed; needs graceful handling when someone declines. Distinct from
backlog #24 (game invites) - this is post-game, same-group re-creation.

### Tournaments and scheduled events (L)
Round-robin or bracketed events spanning many games, with a standings page
and (optionally) a schedule. Value: community events, reason to rally
players at a time, prestige. Implementation: new tables (events,
event_entries, event_rounds, event_standings), a scheduler to advance
rounds, and admin tooling to seed and run them. Risks: async timing is hard
- players finish at different rates; needs a policy for byes and timeouts
(interacts with backlog #46 turn timer). Big surface area.

### Clubs / groups (L)
Self-organised groups with membership, a group activity feed, and optional
group ladders. Value: durable social structure for friend circles and
communities; retention glue. Implementation: new tables (clubs,
club_members, club_games), a club page, and scoping of existing
friends/ratings views to a club. Risks: moderation surface grows (extends
backlog #48); risk of building social infra few use early on.

### Player profile flair (S)
Extend the existing player profile (/players/{name}) with a short bio,
favourite games, and display badges. Value: identity and self-expression;
makes profiles feel lived-in. Implementation: a few new user columns and
profile-page sections. Risks: bio text needs moderation (backlog #48).
Small and safe.

### Achievements / badges (M)
Unlockable milestones: first win, 100 games played, a win at every game,
longest win streak. Shown on the profile and announced in turn emails.
Value: lightweight progression and collection hook that works in an async
game. Implementation: an achievements table plus an event handler that
evaluates unlocks when games finish; render on profile and in email.
Risks: must be cheap to evaluate on every game end; balance so they are
meaningful, not noise. Builds on backlog #29 (game history/stats) for the
underlying counts.

---

## Game experience

### Private in-game notes (S)
A per-player scratchpad attached to each game, visible only to its owner,
persisted across sessions. Value: async players lose context between turns;
a notebook ("opponent likely holding X") supports deeper play. Implementation:
a notes table keyed by (game, player) and a collapsible panel on the game
page. Risks: minimal; keep it private-state-only so it never leaks via
pub_state.

### Turn timeline scrubber (M)
A visual rewind of a game: a slider/stepper over prior turns that re-renders
the board at each point. Value: post-game analysis, learning from losses,
and catching up on a game you joined or missed turns of. Implementation:
game_logs already records turns; add a route/handler that replays logs to a
given turn and renders that state, plus frontend controls. Risks: rendering
historical states must be deterministic and cheap; hidden-info handling for
the scrubber owner's own past knowledge.

### Keyboard-first command palette (S)
A command palette on the game page: type to filter valid commands, with
autocomplete and inline help, fully keyboard operable. Value: faster, more
confident play for power users and accessibility. Implementation: frontend
component over the existing suggest/advancement engine in the command
parser (rust/lib/game/src/command/parser). Risks: low; the suggest engine
already exists, this is a UI onto it.

### "Why can't I?" move explainer (M)
When a command is rejected, show a plain-language reason ("you cannot buy
that: you have 3 coins, it costs 5") instead of a bare error. Value: lowers
the learning curve and frustration, especially by email. Implementation:
extend the game V2 error path to carry structured reasons and render them in
web and email. Risks: requires touching all 27 game services to produce good
messages; can be rolled out per game.

### Accessibility themes and font sizing (S)
Extend the existing colour settings with high-contrast and colourblind-safe
palettes and a global font-size control. Value: the text-first design is
already accessibility-friendly; this makes it deliberate and complete.
Implementation: settings columns plus theme tokens applied in the Leptos
shell and email. Risks: low; mostly CSS/token work.

### Pace and duration indicators (S)
Show per-game averages: typical turn time and expected total game duration,
plus a live "this game is moving slowly/fast" hint. Value: sets expectations
for async play and nudges stalled games. Implementation: aggregate over
game_logs timestamps; surface on the game page and game info page. Risks:
cold-start with little data; keep it approximate.

---

## Email experience

### Email digest mode (M)
An opt-in setting that batches turn notifications into a daily (or weekly)
digest instead of one email per turn. Value: players in many concurrent
games get notification fatigue; a digest keeps play-by-email usable at
volume. Implementation: a digest queue and a scheduled send (cron/worker),
a digest template aggregating "your moves" across games, and a settings
toggle. Risks: a turn buried for a day can stall time-sensitive games; offer
per-game override and keep urgent events (you've been invited) immediate.
Extends the VISION turn-notification work (Phase 22b) with a delivery mode.

### Richer per-game email board layouts (M)
Tailor the email render per game so the board reads well in a mail client
(tables for grids, compact hands, clear "your move" block). Value: the email
IS the product for play-by-email players; better renders are core UX.
Implementation: per-game email templates in rust/web/src/email. Risks: email
client rendering quirks are severe (Gmail foster-parenting, font-size:0) -
see docs/email.md before touching this; needs headless-Chromium verification.

### Configurable turn reminders (S/M)
Email nudges on a player-chosen cadence when a game is waiting on them
("3 games await your move"), separate from any forced-timer behaviour.
Value: re-engages lapsed players without taking the game away from them.
Implementation: a scheduler that scans for stale turns and sends a batched
reminder, plus a settings toggle. Distinct from backlog #46 (turn timer that
auto-plays/concedes): this only reminds, never acts. Risks: avoid spamming;
cap frequency and honour unsubscribe kinds.

### Reply-to-play onboarding email (S)
Make the first login email a short interactive tutorial: it explains
reply-to-play and includes a sample game the player can reply to immediately.
Value: the signature feature (play from your inbox) is demonstrated, not just
described. Implementation: a templated onboarding email plus a seeded sample
game or sandbox. Risks: keep it short; one clear call to action.

---

## Bots

### Featured bot / "bot of the week" (S)
Surface a rotating featured bot persona on the index page with a one-click
"play this bot" for a quick game. Value: showcases bot personalities and
gives new visitors an instant, low-commitment game. Implementation: pick a
bot from the existing bot config, render a card on HomePage, deep-link into
new-game setup. Risks: low. Distinct from the VISION admin GUI for bot
config - this is user-facing discovery, not admin management.

### Post-game difficulty feedback (M)
After a game against a bot, ask the player to rate the difficulty (too easy /
just right / too hard) and aggregate per bot persona. Value: a feedback loop
to tune bot personas over time. Implementation: a rating capture on the
post-game screen/email and an aggregation view for admins. Risks: small
sample sizes early; treat as telemetry, not auto-tuning. Feeds the eventual
bot admin GUI (VISION).

### Daily challenge vs a bot (L)
A curated daily scenario (a starting position or objective) that all players
can attempt against a bot, with a streak counter and a lightweight
leaderboard of results. Value: a daily reason to return and a showcase for
bot play. Implementation: a scenario store, a daily scheduler that publishes
the challenge, seeded game creation, and result tracking. Risks: needs
per-game scenario authoring and a way to seed arbitrary game states - the
biggest unknown; start with the few games whose state is easy to seed.

### Bot exhibition matches (M)
Run scheduled bot-vs-bot games that anyone can spectate, with commentary in
the log. Value: entertainment, a demo of bot strength, and training data /
introspection for bot tuning. Implementation: an orchestrator that creates
all-bot games on a schedule, paired with the spectate feature; log bot
"reasoning" snippets. Risks: compute/LLM cost of many bot turns; rate-limit
and schedule off-peak. Synergises strongly with spectate live games.

---

## Discovery and onboarding

### Game tags and a filterable library (M)
Tag each game with structured metadata - player count, weight/complexity,
play time, mechanics (drafting, trick-taking, push-your-luck) - and let
players filter and sort the game library by them. Value: with 27+ games,
"what should I play?" is a real friction point; tags make the catalogue
navigable and power recommendations later. Implementation: tag tables plus
game_type metadata, a filterable library view, and tagging the existing
game info / new-game pages. Risks: tagging is editorial effort; keep the
taxonomy small. Extends backlog #44 (new game screen usability) and lays
groundwork for a recommendation engine.

### Game recommendations (M)
"Because you played X, try Y" suggestions on the home and library pages,
driven by shared tags and the player's history. Value: surfaces the long
tail of the catalogue and helps new players find a fit. Implementation: a
lightweight recommender over tags + play history (no ML needed to start),
rendered as a row of cards. Risks: cold start for new players (fall back to
popular games). Depends on game tags existing first.

### Global leaderboards and ratings page (M)
A public, per-game ratings leaderboard (top players, most active, rising),
complementing the friend-scoped ratings already on the home page. Value:
competition and prestige; a public page that is shareable and good for SEO.
Implementation: a new route backed by ratings queries, with caching. Risks:
ratings queries can be heavy - cache aggressively; decide provisional-rating
and activity-threshold rules to avoid stale/abuseable boards.

### Interactive learn mode (L)
A guided first game per title: hints, suggested moves, and explanations of
rules in context, playable against a patient bot. Value: the single biggest
onboarding lever - turns "I don't know this game" into "I just played it."
Implementation: a hint/explanation source per game (could reuse the bot's
strategy docs), a tutorial flow on the game page, and progress tracking.
Risks: authoring quality hints for 27 games is a lot; pilot with a few
popular titles. Builds on the bot strategy-doc infrastructure (VISION).

---

## Operator and admin

### Admin analytics dashboard (M)
An admin page with platform health and usage: daily active players, games
created/finished per day, most-played games, bot turn volume and latency,
email send volume and bounces. Value: visibility to make product and
capacity decisions instead of guessing. Implementation: aggregate queries
over existing tables plus bot/email metrics, rendered on the admin page;
optionally pipe to Sentry/Grafana. Risks: query cost on large tables -
precompute daily rollups.

### Feature flags / gradual rollout (M)
A small flags system to enable features per-user or by percentage, toggled
from the admin page without a deploy. Value: safer rollouts, easy A/B, and a
kill switch for new features. Implementation: a flags table/cache, a helper
in the request path, and admin CRUD. Risks: flag sprawl and stale flags -
need a cleanup discipline.

### Game-version canary traffic splitting (L)
Route a percentage of new games to a newly deployed game version before
promoting it to everyone. Value: safe rollout of game-logic changes across
the 27 services; catches regressions on a small cohort. Implementation:
routing weight in game_client / the operator's GameVersion handling, plus
metrics split by version. Risks: players in canary games must stay on that
version for the game's life (pin version at creation); adds routing
complexity. Extends the operator's existing GameVersion lifecycle (VISION).

### Self-serve stuck-game recovery tools (M)
Admin UI to diagnose and recover wedged games: force-resolve a stalled turn,
re-publish a missed NATS event, refund a corrupted rating, or reseat a bot.
Value: turns support incidents from manual DB surgery into safe, audited
clicks. Implementation: admin endpoints over the domain with audit logging.
Risks: every action must be safe and reversible/audited; gate behind admin.
Complements the bot-turn sweep/retry already added in the recent review.

---

## Platform

### Public API for third-party clients (L)
A documented, versioned API (OpenAPI) covering auth, game listing, moves,
and SSE events, so others can build alternate clients, tools, or bots.
Value: extensibility and community tooling; positions brdgme as a platform.
Implementation: an API layer over the existing domain, API tokens, rate
limits, and docs. Risks: an API is a long-term compatibility commitment;
needs versioning and abuse controls from day one.

### Webhooks / RSS for your turns (S)
Let a player subscribe to their own events (your turn, you were invited) as
an RSS feed or outbound webhook, to pipe into their own tools. Value: fits
the hacker-friendly, text-first ethos; lets players build their own
notifications. Implementation: a per-user signed feed URL and/or webhook
registration, reusing existing event data. Risks: abuse/rate limits; keep it
self-scoped to the owner's events only.

### Rating transparency (S/M)
Show the rating model's details on the profile: current rating, confidence,
provisional status, and a rating-over-time graph. Value: demystifies the
ladder and makes progress tangible. Implementation: expose rating internals
already stored, plus a sparkline/series on the player page. Risks: low;
mostly read-path and frontend work. Extends the ratings/sparklines already
on the home page.

### Installable PWA / offline board viewing (M)
Make the web app installable and let players view their games' last known
state offline. Value: better mobile experience and resilience; a step toward
native-feeling clients without building native apps. Implementation: a
service worker, a web manifest, and client-side caching of last state.
Risks: offline mutation is out of scope - read-only offline; shares service
worker groundwork with backlog #36 (web push), so sequence them together.

### Internationalisation (L)
Localise the UI and outbound emails into additional languages. Value: opens
the platform to non-English players. Implementation: an i18n layer in the
Leptos frontend and templated email strings, with a translation workflow.
Risks: ongoing translation maintenance; game rule text and bot personas
complicate coverage. Defer until there is demand signal.
