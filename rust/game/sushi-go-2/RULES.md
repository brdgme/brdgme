# Sushi Go

A 2-5 player card drafting game. Pick a card from your hand, pass the rest, repeat until empty. Three rounds. Most points wins.

## Setup

- 108-card deck: tempura (14), sashimi (14), dumpling (14), maki x3 (8), maki x2 (12), maki x1 (6), salmon nigiri (10), squid nigiri (5), egg nigiri (5), pudding (10), wasabi (6), chopsticks (4).
- Each player is dealt cards (9 for 2-3p, 8 for 4p, 7 for 5p). 2p uses a dummy third player and a 9-card variant deal.
- 3 rounds. Round 1 passes left, round 2 passes right, round 3 passes left.

## Turn

Each hand, every player simultaneously chooses one card to play. With chopsticks already on your table, you may play two cards (returning chopsticks to your hand).

- **play `<card> [<card>]`** - play one card, or two if you have chopsticks on the table. Card numbers are shown in parentheses next to your hand.
- **dummy `<card>`** - (2p only) play a card for the dummy from your own hand.

After all players have played, hands are passed and the next hand begins.

## Scoring

- **Maki rolls**: most maki symbols across all players gets 6 points; second most gets 3 (split among tied players).
- **Nigiri**: egg 1, salmon 2, squid 3. Wasabi triples the next nigiri played after it.
- **Tempura**: 2 = 5 points.
- **Sashimi**: 3 = 10 points.
- **Dumpling**: 1, 3, 6, 10, 15 (capped at 15).
- **Pudding** (end of round 3): most puddings gets 6 points; fewest gets -6 (not in 2p).

## 2-Player Variant

A dummy player joins. The controller (alternating each hand) draws an extra card from the dummy's hand, then plays one card for themselves and one for the dummy.

## Commands

| Command | Action | Example |
|---------|--------|---------|
| `play <card> [<card>]` | Play one or two cards from your hand | `play 3` or `play 1 4` |
| `dummy <card>` | Play a card for the dummy (2p only) | `dummy 5` |
