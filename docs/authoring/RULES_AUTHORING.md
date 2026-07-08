# Game Rules Documentation

Rules files live at `rust/game/{game-name}/RULES.md`. Each game version has its own file. The content is stored in the `game_versions.rules` column at deploy time and served to the bot and (eventually) the frontend.

## Authoring Constraints

- **No copying.** Do not reproduce source rulebook text verbatim. Rewrite everything in your own words.
- **Concise and high-value.** Every sentence must earn its place. No fluff, no filler, no restating the obvious.
- **Comprehensive.** Cover all rules including edge cases. A player (or bot) should be able to play correctly from the RULES.md alone.
- **Source-verified.** Read the game implementation source before writing. The code is authoritative - if it contradicts the physical rulebook, the code wins. Check scoring formulae, edge case handling, and valid move validation directly from the source.
- **Version-specific.** Only include rules that apply to this version. If v1 is 2-player only, do not mention 3-player. If v2 adds a 3-player mode, include a dedicated section covering only the differences.

## Required Sections

### Overview
One short paragraph. What is the game, what is the goal, what makes it interesting.

### Cards / Components
Describe the card types, values, and any notation used in the display (e.g. `RX` = Red wager, `G5` = Green 5).

### Turn Structure
Numbered phases in order. For each phase: what the player must do, what the options are, and any constraints on those options.

Include inline command examples immediately after explaining each action - not just in the Commands table at the end. A player reading about discarding should see `discard y3` right there, not have to scroll to find it. This is especially valuable for the bot, which uses the rules as a reference during play.

Example of good inline placement:
> Play a card face-up to that expedition (`play g5`, `play rx`) or discard it to the shared pile (`discard y3`). Then draw from the deck (`draw`) or take the top card from a discard pile (`take g`).

### Scoring
State the formula explicitly. Include a worked example with a table showing multiple expeditions, covering:
- A profitable expedition
- A break-even expedition
- A wager card case
- An unstarted expedition (0 cost/reward)
- A loss (wager or expedition cost exceeding gains)

### Rounds / Game End
When a round ends, how many rounds are played, who starts the next round and why.

### Winning
How the winner is determined.

### Reading the Display
Explain the rendered board layout. This is critical for the bot, which receives the render as raw brdgme markup text and must interpret it correctly to play well. Cover:
- What each section of the display represents
- Column/row ordering
- What empty slots look like (e.g. `--`)
- What information is visible vs hidden
- The score table format

**Use a real render, not a contrived example.** Each game has a `*_cli`
binary that reads a JSON request from stdin and writes a JSON response to
stdout. Build it locally and pipe a game state from the DB directly:

```bash
# 1. Fetch game state from DB
psql "$DATABASE_URL" -t -A -c \
  "SELECT game_state FROM games WHERE id = '<game-id>';" \
  > /tmp/game_state.txt

# 2. Build the CLI binary
cargo build --release -p <game-crate> --bin <game>_cli

# 3. Pipe a Status request and extract the render
jq -Rs '{"Status":{"game":.}}' /tmp/game_state.txt \
  | ./target/release/<game>_cli \
  | jq -r '.Status.player_renders[0].render'
```

`DATABASE_URL` is available in the shell environment. Use a game state that
is mid-game with some expeditions started, discard piles populated, and at
least one round scored - this is far more illustrative than an initial deal.

Place the render in a `brdgme` fenced code block:

````markdown
```brdgme
{{b}}Round 2 of 3{{/b}}
...actual markup output...
```
````

The `brdgme` language identifier is a convention for future frontend rendering
(the frontend will detect it and render the markup styled, identical to how the
game itself looks). Markdown renderers that don't support it fall back to
preformatted text gracefully.

For games with multiple player counts, capture a render for each distinct
layout (e.g. 2-player and 3-player have different tableau arrangements).

### Commands
A exhaustive reference table of every command with syntax and examples. This
is in addition to inline examples throughout the rules - not a replacement for
them. A player should encounter each command in context before seeing it here.

| Command | Action | Example |
|---------|--------|---------|
| `play <card>` | ... | `play g5` |

### Strategy Tips
Always include this section. It may only be populated from two sources:

1. **The official rulebook** - some rulebooks include explicit hints or strategy notes. Summarise these concisely in your own words.
2. **Tips provided directly by the user** - after the initial draft, the user may supply additional strategy advice to add here.

**Do not use your own knowledge, intuition, or general game-playing reasoning to fill this section.** If neither source has provided any strategy content yet, include the section header with a note that tips will be added, rather than inventing content. The goal is a trustworthy, curated set of tips - not a comprehensive AI-generated strategy guide.

## Process

1. Read the official rulebook (PDF or physical rules)
2. Read all relevant source files: `lib.rs`, `card.rs`, `command.rs`, `render.rs`
3. Cross-check: where the code and rulebook differ, follow the code
4. Write the RULES.md - do not copy rulebook text
5. Verify command syntax against `command.rs` parsers (token names, card notation, exact argument format)
6. Verify scoring formula against the scoring function in `lib.rs`, including any player-count-dependent parameters
7. Extract a real mid-game render from the game service (see Reading the Display above) and insert it as a `brdgme` code block

## Storage

Rules are stored in `game_versions.rules` in the database. The operator
populates this column by calling `Request::Rules` against the game service
during reconcile. The game service returns the content of the `rules()` method
on the `Gamer` trait implementation, which should `include_str!` the RULES.md
file at compile time. The bot reads rules directly from the DB - it does not
call the game service for rules at runtime.
