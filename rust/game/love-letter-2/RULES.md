# Love Letter

A 2-4 player game of risk, deduction and luck. Be the first to accumulate enough points by ending each round holding the highest card, or by eliminating your rivals from the round.

## The deck

16 cards, numbered 1 (lowest) to 8 (highest):

| # | Card | Count | Effect |
|---|------|-------|--------|
| 8 | Princess | 1 | You are eliminated if you discard the Princess |
| 7 | Countess | 1 | Discard the Countess if you have the King or Prince in your hand |
| 6 | King | 1 | Trade your hand with another player |
| 5 | Prince | 2 | Choose a player (or yourself) to discard and draw a new card |
| 4 | Handmaid | 2 | Immune to the effects of other players' cards until next turn |
| 3 | Baron | 2 | Compare hands with another player, lowest card is eliminated |
| 2 | Priest | 2 | Look at another player's hand |
| 1 | Guard | 5 | Guess another player's card to eliminate them, except for Guard |

## Setup

Each round, the deck is shuffled and one card is set aside face-down and unseen (4 cards in a 2-player game). Each player is dealt one card to start their hand.

## Turns

On your turn you draw a card, giving you two in hand, then play one of them - the other stays in your hand until your next turn. Playing a card resolves its effect (see the table above), then discards it. If you're forced into a position with no valid other target (everyone else eliminated or protected by the Handmaid), you may target yourself instead.

**Countess rule:** if you're holding the Countess and the King or Prince, you must play the Countess.

## Round end

The round ends immediately when either only one player remains (not eliminated), or the deck runs out. On a deck-out, whichever remaining player holds the highest card wins the round; ties are broken by whoever has discarded the most (by card number total). The winner scores a point and starts the next round.

## Winning

The game ends once a player reaches the target score for the player count:

| Players | Points to win |
|---------|---------------|
| 2 | 7 |
| 3 | 5 |
| 4 | 4 |

## Commands

| Command | Action | Example |
|---------|--------|---------|
| `princess` | Play the Princess (you will be eliminated) | `princess` |
| `countess` | Play the Countess | `countess` |
| `king <player>` | Trade hands with another player | `king steve` |
| `prince <player>` | Make a player (or yourself) discard and draw | `prince mick` |
| `handmaid` | Protect yourself until your next turn | `handmaid` |
| `baron <player>` | Compare hands, lower card is eliminated | `baron steve` |
| `priest <player>` | Look at another player's hand | `priest steve` |
| `guard <player> <card>` | Guess another player's card | `guard steve priest` |

Only commands for cards currently in your hand are available.
