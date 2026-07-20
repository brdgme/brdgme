# Zombie Dice Data Dictionary

## PubState (public information)

- `players` (usize): Number of players in this game (2 to 8).
- `current_turn` (usize): Index of the player whose turn it is.
- `scores` (Vec<i32>): Banked brain scores for each player, indexed by player. Brains are only added to this total when a player chooses to keep.
- `cup` (Vec<Dice>): Dice remaining in the cup, in draw order. Each die has a `colour` field (Green, Yellow, or Red). At game start: 6 green, 4 yellow, 3 red (13 total).
- `current_roll` (DiceResultList): Dice showing footprints from the latest roll. These stay in front of the player and are re-rolled on the next `roll` command. Each entry has a `dice` (colour) and `face` (always Footprints here).
- `kept` (DiceResultList): Dice set aside this turn. Includes both brains eaten (Face::Brain) and shotguns taken (Face::Shotgun). Each entry has a `dice` (colour) and `face`.
- `round_brains` (i32): Number of brains eaten this turn, not yet banked. Lost if the player busts (3 shotguns).
- `round_shotguns` (i32): Number of shotguns taken this turn. At 3, the turn ends immediately and all round_brains are lost.
- `finished` (bool): True when the game is over (a player reached 13+ brains with a unique lead, or won a rolloff).
- `placings` (Vec<usize>): Final placings for each player (1 = first place). Only populated when `finished` is true; empty vec during play.

## PlayerState (player-private information)

- `public` (PubState): The full public game state, as described above. Zombie Dice has no hidden information per player; PlayerState is just a wrapper around PubState.

## Dice

- `colour` (Colour): Green, Yellow, or Red. Determines the face distribution:
  - Green: 3 brains, 2 footprints, 1 shotgun (safest)
  - Yellow: 2 brains, 2 footprints, 2 shotguns (balanced)
  - Red: 1 brain, 2 footprints, 3 shotguns (riskiest)

## DiceResult

- `dice` (Dice): The die that was rolled (includes colour).
- `face` (Face): The result - Brain, Shotgun, or Footprints.

## Face enum

- `Brain`: Eaten and set aside. Counts toward round_brains.
- `Shotgun`: Set aside. Three shotguns in a turn busts the player.
- `Footprints`: Stays in front of the player and is re-rolled next turn.
