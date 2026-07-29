# Hanamikoji - Advanced Strategy

Higher-level strategic considerations.

## I-cut-you-choose

- In a gift you keep two cards and your opponent takes one; in a competition you keep one pair and they take the other. Your opponent always takes whichever option hurts you most, so you do not get to keep your favourite - you get the leftover.
- Structure every offer so that both possible outcomes are acceptable to you. If one of the three gift cards would wreck you in the opponent's hands, do not offer it.
- Use gifts and competitions to place cards on geisha you want while forcing the opponent to take cards that help them least. Two cards you keep on one geisha can win it outright.
- A competition splits four cards into two pairs; pair them so the two pairs are close in value to you. Lopsided pairs let the opponent take the strong pair and leave you the weak one.

## Action ordering

- You draw one card per turn and spend 1, 2, 3 and 4 cards across your four actions, so your hand is largest early. Play competition (4 cards) and gift (3 cards) while you still have the cards; secret (1) and trade-off (2) can wait until your hand is small.
- The starting player begins with seven cards and the other with six, then both draw each turn. If you are short on cards, you may be forced to take a small action first - plan the order so you are never unable to afford competition when you want it.
- Save the trade-off for cards that genuinely help neither player, or for a pair you would otherwise be forced to gift away.

## Markers across rounds

- Victory markers persist between rounds, and a tied geisha keeps its current owner. If you already control a geisha, you can defend it cheaply - matching your opponent's cards keeps it yours without winning the count outright.
- A geisha you do not control must be won by a strict majority, which costs one more card than your opponent has there. Contesting a defended geisha is expensive; pick your battles.
- Early rounds set the board. Controlling high-charm geisha early forces your opponent to spend cards to claw them back in later rounds.

## The two races

- Track both the geisha count and the charm count at all times. The summary table shows both. A player at 3 geisha and 10 charm is one card from winning on either axis.
- If you are racing on charm, prioritise Tea (5) and Taiko (4). If you are racing on geisha count, cheap low-charm geisha win just as well as expensive ones - four of any value ends the game.
- Remember the tiebreak: if you reach four geisha but your opponent reaches eleven charm in the same scoring, they win. Do not tunnel-vision on geisha count while handing over the high-charm columns.

## Reading the opponent

- Each player has exactly one secret, revealed at scoring. The public `has_secret` flag and the cards you can see (your hand, face-up cards, the gift and competition discards) let you narrow down what their secret likely is.
- Cards you cannot account for are in the opponent's hand, their secret, their trade-off or the one removed card. Counting these tells you which geisha they can still contest.
- A player who has not used their secret late in the round is holding it for a geisha they care about - watch which geisha they have been building.
