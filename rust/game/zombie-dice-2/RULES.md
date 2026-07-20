# Zombie Dice

A 2-8 player press-your-luck dice game. You are zombies chasing brains. Roll three dice from the cup, set aside brains (eaten) and shotguns (you got shot). Footprints stick around for your next roll. Decide whether to keep pushing for more brains or bank what you have. Three shotguns ends your turn with nothing. First to 13 brains triggers the final round; ties break with a rolloff.

## Setup

Each player starts with zero brains. Player 1 takes the first turn. The cup starts with all 13 dice: 6 green, 4 yellow, 3 red.

## The Dice

There are 13 dice in three colours. Each die has six faces: Brains, Footprints, and Shotguns, in different proportions per colour.

| Colour | Brains | Footprints | Shotguns |
|---|---|---|---|
| Green | 3 | 2 | 1 |
| Yellow | 2 | 2 | 2 |
| Red | 1 | 2 | 3 |

Green dice are safest (most brains, fewest shotguns); red dice are riskiest.

## Turn Structure

On your turn the game automatically rolls three dice from the cup. Each die shows one face:

- **Brain** - set aside; you ate a brain. Stays out of play until your turn ends.
- **Shotgun** - set aside; you were shot. Three shotguns in a turn ends your turn with zero banked brains.
- **Footprints** - stays in front of you and is re-rolled on your next `roll`.

Then you choose one of:

- **Roll** - re-roll the dice showing footprints plus enough new dice from the cup to make three. If the cup runs dry, all your set-aside dice (brains and shotguns) go back in, the cup is reshuffled, and you continue.
- **Keep** - bank the brains you ate this turn and pass play to the next player.

## Ending and Winning

When play returns to player 1 and someone has 13 or more brains, the leader wins outright. If multiple players are tied at 13+, those tied players enter a tie-breaker rolloff: only they keep taking turns, skipping everyone else, until one of them is strictly ahead when play wraps back to player 1.

Placings are by brain count, highest first; ties share a place.

## Commands

| Command | Action |
|---|---|
| `roll` | Push your luck and roll the dice |
| `keep` | Be a coward and keep your brains |


