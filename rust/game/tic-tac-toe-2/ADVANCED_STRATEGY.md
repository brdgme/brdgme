# Tic-tac-toe - Advanced Strategy

Higher-level strategic considerations.

## Fork creation

- A fork is a move that creates two winning threats simultaneously. Your opponent can only block one.
- The classic fork: take opposite corners when your opponent takes the center. If they do not block correctly, you fork.
- Creating a fork is the primary way to win against imperfect play.

## Opponent's fork prevention

- If your opponent can create a fork next turn, prevent it. This often means playing in a specific cell that blocks both potential fork lines.
- When you have the center and your opponent has a corner, play the opposite corner to prevent their fork setup.

## Perfect play

- With perfect play, tic-tac-toe is always a draw. The goal against imperfect opponents is to create situations where they must find the only correct response.
- As X (first player): center opening is strongest. Corner opening is also drawish but gives more chances for opponent errors.
- As O (second player): respond to center with a corner. Respond to a corner with the center.

## Tempo

- Every move should either create a threat, block a threat, or improve your position. Wasted moves (no threat, no block, no positional gain) lose to optimal play.
- In the mid-game, prioritize moves that create multiple threats over single-threat moves.
