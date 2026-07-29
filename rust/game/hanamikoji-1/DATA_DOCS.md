# Hanamikoji Data Dictionary

## PubState (public information)

- `players` (usize): Number of players in the game. Always 2.
- `round` (u32): The current round number, starting at 1. Increments each time a round scores without a winner.
- `finished` (bool): True once a winner has been decided.
- `phase` (Phase): Which step of the turn the game is in. See the Phase enum below.
- `current` (usize): The actor of the current real turn. During `OpponentChoose` this is still the player who played the gift or competition, not the player choosing.
- `whose_turn` (Vec<usize>): The player(s) expected to act next. In `ChooseAction` this is `current`; in `OpponentChoose` it is the opponent (`1 - current`); empty when finished.
- `starting` (usize): The player who takes the first turn this round. Alternates each round.
- `deck_remaining` (usize): Number of cards left in the draw pile. The deck order is hidden and not present in the state.
- `marker` (Vec<Option<usize>>): Victory marker position per geisha, indexed by geisha (Flute=0, Koto=1, Fan=2, Shamisen=3, Umbrella=4, Taiko=5, Tea=6). `None` means contested (no one controls it); `Some(p)` means player `p` controls it. Markers persist across rounds.
- `faceup` (Vec<[u32; 2]>): Face-up card counts per geisha, indexed by geisha. Each entry is `[player0_count, player1_count]`. These are the cards placed on each side this round (gift, competition and revealed secrets); the majority decides the marker at scoring.
- `used` (Vec<[bool; 4]>): Which of the four action markers each player has used this round, indexed by player. The four slots are `[secret, trade, gift, compete]`. `true` means spent.
- `hand_counts` (Vec<usize>): Number of cards in each player's hand, indexed by player. Contents are hidden; only the count is public.
- `has_secret` (Vec<bool>): Whether each player has a face-down secret card, indexed by player. The card's identity is hidden.
- `traded_counts` (Vec<usize>): Number of cards each player has set aside for trade-off, indexed by player. Always 0 or 2. The card identities are hidden.
- `pending` (Option<Pending>): The face-up pending choice, if a gift or competition is awaiting a response. See the Pending enum below. `None` when no choice is pending.
- `geisha_counts` (Vec<usize>): Number of geisha each player currently controls, indexed by player. Reaching 4 wins.
- `charms` (Vec<i32>): Charm points each player currently controls, indexed by player. The sum of the charm values of the geisha they control. Reaching 11 wins.
- `winner` (Option<usize>): The winner, if the game is over. `None` while the game is in progress.

## PlayerState (player-private information)

- `public` (PubState): The full public game state, as described above.
- `player` (usize): Which player this private state belongs to.
- `hand` (Vec<Geisha>): This player's hand, as a multiset of geisha types. Cards of the same type are interchangeable. Sorted by geisha index.
- `secret` (Option<Geisha>): This player's face-down secret card, if they have played one this round. `None` otherwise.
- `traded` (Vec<Geisha>): This player's face-down trade-off discard for this round. Empty until they use the trade-off action, then two cards.

The opponent's `hand`, `secret` and `traded` are never exposed - the public state carries only their counts (`hand_counts`, `has_secret`, `traded_counts`).

## Phase enum

- `ChooseAction`: The current player draws (automatically) and picks one of their unused actions.
- `OpponentChoose`: A gift or competition is pending; the opponent of `current` must choose from the offered cards.
- `Finished`: The game is over.

## Pending enum

The face-up cards awaiting a choice. Both variants are fully public.

- `Gift { actor, cards }`: `actor` offered the three `cards`; the opponent takes one, `actor` keeps the other two.
- `Competition { actor, sets }`: `actor` offered two pairs `sets[0]` and `sets[1]`; the opponent takes one pair, `actor` keeps the other.

## Geisha enum

The seven geisha, in board order, with their charm value. Each has three identical item cards.

- `Flute` (2), `Koto` (2), `Fan` (2), `Shamisen` (3), `Umbrella` (3), `Taiko` (4), `Tea` (5). Charm totals 21.
