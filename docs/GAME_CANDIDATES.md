# Board Game Port Candidates

Research into strong candidates to port to brdgme, drawn from the 500
most-voted games on BoardGameGeek. Research only - no implementation.

## Selection Criteria

A good brdgme port, in priority order:

1. Card game, or a simple board that is easy to represent (grids, hex
   tiles). Acquire is the reference example of a board that works well.
2. Low visual complexity. brdgme renders ASCII text via web and email.
3. Player turns are fairly isolated. Little back-and-forth interaction
   inside a single turn. Negotiation and heavy take-that are hard.
4. Smaller hobby publisher. Easier and nicer to work with for licensing.
5. Rules are not huge or complex. Lots of strategic depth is fine.

An implicit sixth criterion: fit for an asynchronous, text-command
interface (play-by-email). Roll/flip-and-write, drafting, trick-taking,
and abstract grid games map onto this very well.

## Data Source and Caveats

- Source: a BGG personal-collection export of the 500 most-voted games
  (`Most voted board games.xlsx`).
- The export is sorted by number of voters and contains: BGG rank, name
  (with embedded year and one-line description), average and Bayes
  average rating, and number of voters.
- It does NOT contain publisher, complexity weight, or player counts.
  Those fields below were looked up per game from boardgamegeek.com and
  are accurate as of this writing but may drift.
- "Publisher size" is a judgement: tiny / small-hobby / medium / large,
  with corporate-parent ownership flagged where relevant.

## Already Ported (excluded)

These 27 games already exist under `rust/game/` and are out of scope:

- Acquire, Age of War, Alhambra, Battleship, Category 5 (6 nimmt),
  Cathedral, Farkle, For Sale, Greed, Jaipur, Liar's Dice, Lords of
  Vegas, Lost Cities, Love Letter, Modern Art, No Thanks!, Red7, Roll
  Through the Ages, Seven Wonders, Splendor, Starship Catan, Sushi Go,
  Sushizock, Texas Hold'em, Tic-Tac-Toe, Zombie Dice.

The legacy `brdgme-go/` tree holds 17 games, all of which are already
covered by the Rust crates above (no additional ports to exclude).

Note: 7 Wonders Duel and Splendor Duel are not ported, but each shares
its core engine with an already-ported base game, so they are mentioned
only as adjacent ideas, not primary candidates.

## Top Recommendations

The five strongest fits across all criteria.

### 1. Cascadia (2021)

- Players: 1-4. Weight: 1.84. Publisher: Flatout Games (tiny) with AEG
  (independent, ~15 staff) distribution.
- Hex tile drafting and pattern building. Each turn: pick one habitat
  tile and one wildlife token from a shared market, place on your own
  board.
- Criterion fit: simple hex grid renders cleanly in ASCII; turns are
  almost entirely solitaire (interaction is only competing for the
  shared market); light rules with real scoring depth; tiny independent
  publisher.
- Async/text fit: excellent. State is your hex grid plus the market;
  commands like `take 2` / `place fox 3` map naturally.
- Concern: none significant. The market is the only shared state.

### 2. Cartographers (2019)

- Players: 1-100. Weight: 1.89. Publisher: Thunderworks Games
  (small-hobby, independent).
- Flip-and-draw map building. A shared card is revealed each round;
  every player draws the shown polyomino onto their own grid, then
  scores against private edict cards.
- Criterion fit: pure grid, low visual complexity; near-zero in-turn
  interaction (the only shared element is the card flip and an
  occasional passed "monster" cell); simple rules, deep scoring puzzle;
  small independent publisher.
- Async/text fit: ideal. Flip-and-write genres are among the best matches
  for play-by-email - no timing, no negotiation, fully parallel turns.
- Concern: rendering each player's grid legibly in email is the main
  layout task, but it is a bounded grid.

### 3. Hanamikoji (2013)

- Players: 2. Weight: 1.68. Publisher: EmperorS4 (tiny, independent).
- Two-player area-majority card game using an I-cut-you-choose action
  system across four geisha districts.
- Criterion fit: pure cards, no board; extremely simple rules with
  genuine two-player depth; turns are a single card allocation with no
  mid-turn response; tiny independent publisher.
- Async/text fit: excellent. Small card hands, four scoring tracks,
  short games.
- Concern: two-player only, which limits matchmaking, but brdgme already
  ships several two-player games.

### 4. The Fox in the Forest (2017)

- Players: 2. Weight: 1.58. Publisher: Foxtrot Games / Renegade Game
  Studios (small-hobby, independent).
- Two-player trick-taking card game with a small set of special "command"
  cards and a cooperative scoring track.
- Criterion fit: pure cards; very light rules; trick-taking is sequential
  with no negotiation; small independent publisher.
- Async/text fit: excellent. brdgme already has trick-taking and card
  infrastructure to reuse.
- Concern: two-player only; the few command cards add minor rules
  overhead but nothing heavy.

### 5. Star Realms (2014)

- Players: 2 (base set). Weight: 1.91. Publisher: Wise Wizard Games
  (small-hobby, independent).
- Two-player deck-building combat: build an engine from a shared card
  row, then attack the opponent's authority.
- Criterion fit: pure cards, low visual complexity; your turn is mostly
  building and attacking with no opponent mid-turn response (no instants
  in the base game); simple rules, deep engine play; small independent
  publisher.
- Async/text fit: excellent. Shared card row plus per-player authority
  and discard; very popular and well understood.
- Concern: head-to-head combat is the interaction, but it is one-directional
  on your turn, so it stays async-friendly. Hero Realms (same publisher,
  2-4 players) is the natural follow-up if multiplayer is wanted.

## Ranked Candidates 6-20

Strong fits, slightly behind the top five on one or more criteria.

| #  | Game                 | Year | Players | Weight | Publisher (size)        | Why / main concern |
|----|----------------------|------|---------|--------|--------------------------|--------------------|
| 6  | Dominion             | 2008 | 2-4     | 2.34   | Rio Grande (medium, indep) | The defining deck builder; pure cards, huge depth, ideal async. Medium (not tiny) publisher. |
| 7  | Onitama              | 2014 | 2       | 1.66   | Arcane Wonders (small, indep) | Elegant 5x5 grid abstract driven by 5 shared move cards; perfect async; two-player only. |
| 8  | Welcome To...        | 2018 | 1-100   | 1.82   | Blue Cocker (small, indep) | Flip-and-write suburb building; parallel turns, ideal async; grid rendering in email is the task. |
| 9  | Hive                 | 2001 | 2       | 2.31   | Gen42 (small, indep)     | Boardless hex-tile abstract; renders as a coordinate graph; two-player only, slightly heavier. |
| 10 | Res Arcana           | 2019 | 2-4     | 2.65   | Sand Castle (tiny, indep) | Card engine builder, mostly solitaire with some resource denial; tiny publisher; weight ~2.6. |
| 11 | Kingdomino           | 2016 | 2-4     | 1.24   | Blue Orange (medium, indep) | Domino tile drafting on a small grid; very light, family-friendly; medium publisher. |
| 12 | Deep Sea Adventure   | 2014 | 2-6     | 1.19   | Oink Games (tiny, indep)  | Push-your-luck dive; extremely light, sequential, tiny publisher; lower strategic depth. |
| 13 | Skull King           | 2013 | 2-8     | 1.73   | Grandpa Beck's (small, indep) | Multiplayer trick-taking with bidding; great async party-ish depth; wider player count. |
| 14 | The Isle of Cats     | 2019 | 1-4     | 2.36   | The City of Games (small, indep) | Card drafting plus polyomino grid packing; small publisher; a touch heavier. |
| 15 | Project L            | 2020 | 1-4     | 1.56   | Boardcubator (small, indep) | Polyomino puzzle engine; satisfying grid play, low interaction; small publisher. |
| 16 | Battle Line          | 2000 | 2       | 1.89   | GMT (small-hobby, indep)  | Classic two-player card formations (a.k.a. Schotten Totten); pure cards; two-player only. |
| 17 | Faraway              | 2023 | 2-6     | 1.94   | Catch Up Games (small, indep) | Card play scored in reverse order; novel, light, low interaction; small publisher. |
| 18 | Cat in the Box       | 2022 | 2-5     | 2.05   | Bezier Games (small, indep) | "Quantum" trick-taking with a shared board; small publisher; modest extra rules. |
| 19 | Railroad Ink         | 2018 | 1-6     | 1.47   | Horrible Guild (small, indep) | Roll-and-write network building; very light, ideal async; lower depth. |
| 20 | Watergate            | 2019 | 2       | 2.29   | Frosted Games (small, indep) | Asymmetric two-player card tug-of-war; small publisher; two-player only, thematic. |

## Considered but Downranked

Mechanically attractive, but flagged mainly on publisher ownership
(criterion 4) or rules weight (criterion 5).

- Azul (2017, 2-4, 1.77): iconic tile drafting, perfect grid fit, but
  Plan B / Next Move is Asmodee-owned - harder licensing.
- It's a Wonderful World (2019, 1-5, 2.33): neat drafting engine, but
  La Boite de Jeu is majority-owned by Hachette Livre.
- Fantasy Realms (2017, 2-7, 1.81): simple combo card game, but WizKids
  is a wholly-owned NECA subsidiary.
- Santorini (2016, 2-4, 1.72): elegant move-and-build, but Spin Master
  (large public toy company) holds the rights.
- Nidavellir (2020, 2-5, 2.11): fun simultaneous coin bidding; soft flag,
  Hachette distributes the US edition.
- Arboretum (2015, 2-4, 2.13): great mean card game; mixed publishing,
  Z-Man / Filosofia editions are Asmodee-owned (Renegade is independent).
- Race for the Galaxy (2007, 2-4, 2.99): superb deep card engine and a
  strong async fit, but the heaviest ruleset here - kept as a "more
  depth" alternative to Dominion rather than a top pick.
- Calico (2020, 1-4, 2.20): close cousin of Cascadia, slightly heavier;
  Cascadia is the preferred pick of the two.
- Tichu (1991, best at 4, 2.35): classic partnership climbing game;
  strong async fit but team/partnership play and a fixed four-player
  sweet spot narrow it.
- Flamecraft (2022, 1-5, 2.19): appealing card-based worker placement,
  small publisher, but heavier per-turn decision load.
- Point Salad (2019, 2-6, 1.15): very light card drafting; fun but thin
  on depth compared with the picks above.

## Cross-Cutting Implementation Notes

- Flip/roll-and-write titles (Cartographers, Welcome To..., Railroad Ink)
  are the single best structural match for play-by-email: fully parallel
  turns, no timing, no negotiation. The recurring task is rendering a
  per-player grid legibly in ASCII email.
- Two-player games (Hanamikoji, Fox in the Forest, Onitama, Hive, Battle
  Line, Watergate, Star Realms) dominate the top of the list because
  isolated turns and pure-card state are easiest to represent; the
  trade-off is matchmaking depth at higher player counts.
- Trick-taking (Fox in the Forest, Skull King, Cat in the Box) and
  deck-building (Star Realms, Dominion) reuse patterns brdgme already has.
- Corporate-owned publishers (Asmodee, Spin Master, Hachette, NECA) are
  concentrated in the "downranked" section on purpose; the top five are
  all tiny or small independent publishers.
