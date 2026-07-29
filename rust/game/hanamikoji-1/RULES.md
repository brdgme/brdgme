# Hanamikoji

A two-player game of favour and majority. Seven geisha line the table; you and your rival court them by placing item cards on your side of each. Win a geisha by holding the majority of cards on it, then claim the game by controlling four geisha or eleven charm. Every card you commit is a card your opponent can see - except the ones you hide.

## Components

- Seven geisha, each with a charm value: Flute (2), Koto (2), Fan (2), Shamisen (3), Umbrella (3), Taiko (4), Tea (5). Charm totals 21.
- 21 item cards: three identical cards per geisha, matching its type.
- Four action markers per player: secret, trade-off, gift, competition. Each is used once per round.
- A victory marker per geisha, tracking who currently controls it.

## Setup

- Shuffle the 21 item cards. Remove one unseen; it is out of the round and never revealed.
- Deal six cards to each player. The remaining eight form the draw deck.
- Choose a starting player at random.

## The Round

Players alternate turns, starting with the starting player, until each has taken four turns (eight turns total).

On your turn you first draw one card from the deck (this is mandatory and happens automatically), then take exactly one of your four actions. The eight draws drain the deck exactly over the round. Each action may be used once per round, in any order; once used, its marker is spent for the round.

Cards you place go on your side of the matching geisha. Cards are referred to by geisha type; the three cards of a type are interchangeable, so a hand is a multiset (for example `gift fan fan shamisen` offers two Fan cards and one Shamisen).

## The Four Actions

- **Secret** - place one card face-down (`secret tea`). It stays hidden until scoring, when it is revealed onto its geisha. Only you may look at it.
- **Trade-off** - place two cards face-down, out of the round (`trade flute koto`). They are not scored this round. Only you may look at them.
- **Gift** - offer three cards face-up (`gift flute koto fan`). Your opponent chooses one of the three for their side; you place the other two on yours.
- **Competition** - offer four cards as two face-up pairs (`compete flute koto fan tea` makes pair 1 {Flute, Koto} and pair 2 {Fan, Tea}). Your opponent chooses one pair for their side; you place the other on yours.

Gift and competition give your opponent a choice, so they pause for a response. While a choice is pending you must resolve it before anything else:

- For a gift, take one of the offered cards by name (`choose fan`).
- For a competition, take one of the two pairs (`choose 1` or `choose 2`).

## Scoring

When both players have spent all four actions, the round scores:

1. Each player reveals their secret card onto its geisha.
2. For each geisha, compare the number of cards on each side (face-up cards plus the just-revealed secret). The player with more cards wins the geisha and its victory marker moves to their side. A tie - or a geisha with no cards - leaves the marker where it was.
3. Tally the geisha each player controls and the charm they carry (the sum of the charm values of those geisha).

A player wins the game if, after scoring, they control four or more geisha, or carry eleven or more charm. If one player reaches four geisha and the other reaches eleven charm in the same scoring, the player with eleven charm wins.

## Rounds and Game End

If no one has won, a new round begins:

- Victory markers stay where they are; control earned in earlier rounds persists.
- All item cards - both sides, the trade-off discards and the removed card - are gathered, reshuffled into a fresh deck and re-dealt as in setup.
- The other player becomes the starting player.

Play continues round after round until a player meets a winning condition during scoring.

## Reading the Display

The display is brdgme markup. `{{player 0}}` and `{{player 1}}` mark the two players (the client shows their names), `{{b}}...{{/b}}` is bold and `{{fg <color>}}...{{/fg}}` colours a geisha name.

The first line states the situation - whose turn it is, or who must resolve a pending choice - followed by the round number and how many cards remain in the deck.

### The board

The first table has one row per geisha, always in the order Flute, Koto, Fan, Shamisen, Umbrella, Taiko, Tea. Columns are: the geisha, its charm value, `{{player 0}}`'s face-up card count, the victory marker, and `{{player 1}}`'s face-up count. The marker column reads `< {{player 0}}` when player 0 controls the geisha, `{{player 1}} >` when player 1 controls it, and a grey `-` while it is unclaimed.

Here is a real mid-game board, in round 2 with two geisha claimed by each player and several cards already placed:

```brdgme
{{b}}{{player 1}}'s turn - choose an action (secret, trade, gift or compete){{/b}}Round 2  -  1 cards left in the deck
{{table}}{{row}}{{cell left}}{{b}}Geisha{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}Charm{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{player 0}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}Marker{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{player 1}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg cyan}}Flute{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}2{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}0{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg grey}}-{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}0{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg green}}Koto{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}2{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}0{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{player 1}}{{b}} >{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}0{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg pink}}Fan{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}2{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}1{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg grey}}-{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}1{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg purple}}Shamisen{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}3{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}1{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}< {{/b}}{{player 0}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}0{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg blue}}Umbrella{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}3{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}1{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}< {{/b}}{{player 0}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}1{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg orange}}Taiko{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}4{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}0{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{player 1}}{{b}} >{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}1{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg yellow}}Tea{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}5{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}0{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg grey}}-{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}0{{/b}}{{/cell}}{{/row}}{{/table}}
{{table}}{{row}}{{cell left}}{{b}}Player{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}Geisha{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}Charm{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}Hand{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}Secret{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}Traded{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{b}}Actions{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{player 0}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}2{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}6{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}3{{/cell}}{{cell left}}  {{/cell}}{{cell center}}yes{{/cell}}{{cell left}}  {{/cell}}{{cell center}}2{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg grey}}S{{/fg}} {{fg grey}}T{{/fg}} {{fg grey}}G{{/fg}} {{b}}{{fg green}}C{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{player 1}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}2{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}6{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}4{{/cell}}{{cell left}}  {{/cell}}{{cell center}}yes{{/cell}}{{cell left}}  {{/cell}}{{cell center}}2{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg grey}}S{{/fg}} {{fg grey}}T{{/fg}} {{fg grey}}G{{/fg}} {{b}}{{fg green}}C{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}
```

### The pending choice

When a gift or competition is waiting for a response, a line above the board describes the offer. A gift lists its three cards; a competition lists its two pairs (a literal opening brace appears as `{{lbrace}}`):

```brdgme
{{b}}Competition: {{/b}}{{player 1}} offered set 1 {{lbrace}} {{b}}{{fg purple}}Shamisen{{/fg}}{{/b}}, {{b}}{{fg orange}}Taiko{{/fg}}{{/b}} } and set 2 {{lbrace}} {{b}}{{fg yellow}}Tea{{/fg}}{{/b}}, {{b}}{{fg yellow}}Tea{{/fg}}{{/b}} } - {{player 0}}{{b}} chooses a set{{/b}}
```

### The summary

The second table summarises each player: geisha controlled, charm carried, hand size, whether they have a face-down secret (`yes` or a grey `-`), how many cards they have set aside for trade-off, and their four action markers - `S T G C`, bold green while available and grey once spent.

### Hidden information

The deck order, hand contents, secret identity and trade-off identities are never shown - only counts and public consequences appear. Your own private cards are appended below the board in your player view:

```brdgme
{{b}}Your hand: {{/b}}{{b}}{{fg purple}}Shamisen{{/fg}}{{/b}}, {{b}}{{fg orange}}Taiko{{/fg}}{{/b}}, {{b}}{{fg yellow}}Tea{{/fg}}{{/b}}, {{b}}{{fg yellow}}Tea{{/fg}}{{/b}}
{{b}}Your secret: {{/b}}{{b}}{{fg cyan}}Flute{{/fg}}{{/b}}
{{b}}Your trade-off discard: {{/b}}{{b}}{{fg pink}}Fan{{/fg}}{{/b}}, {{b}}{{fg purple}}Shamisen{{/fg}}{{/b}}
```

The opponent's hand, secret and trade-off stay hidden; you see only their counts in the summary table.

## Commands

| Command | Action | Example |
|---------|--------|---------|
| `secret <geisha>` | Place one card face-down (revealed at scoring) | `secret tea` |
| `trade <geisha> <geisha>` | Set two cards aside face-down, out of the round | `trade flute koto` |
| `gift <geisha> <geisha> <geisha>` | Offer three cards; opponent takes one | `gift flute koto fan` |
| `compete <a> <b> <c> <d>` | Offer four cards as two pairs (a,b) and (c,d); opponent takes one pair | `compete flute koto fan tea` |
| `choose <geisha>` | Take one card from a pending gift | `choose fan` |
| `choose <set>` | Take one pair from a pending competition (set is 1 or 2) | `choose 2` |

Geisha names are case-insensitive. Only the commands you can legally make right now are offered; drawing is automatic and has no command.
