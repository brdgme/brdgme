# Age of War

## Overview

Age of War is a dice game set in Japan's Sengoku period. Six clans hold
fourteen castles between them, and on your turn you roll dice and reroll
them to gather the exact combination of symbols and infantry a castle
demands, conquering it before your nerve (or your dice) runs out. It plays
2 to 6 players. Capture
every castle in a clan and you take a large bonus instead of the castles'
individual values, so the tension is always between grabbing a quick, safe
castle and pushing your luck for a bigger prize.

## Components

**Castles.** Fourteen castles are grouped into six clans:

| Clan | Castles (points) | Clan bonus if fully conquered |
|------|-------------------|-------------------------------|
| Oda | Azuchi (3), Matsumoto (2), Odani (1), Gifu (1) | 10 |
| Tokugawa | Edo (3), Kiyosu (2), Inuyama (1) | 8 |
| Uesugi | Kasugayama (4), Kitanosho (3) | 8 |
| Mori | Gassantoda (2), Takahashi (2) | 5 |
| Chosokabe | Matsuyama (2), Marugame (1) | 4 |
| Shimazu | Kumamoto (3) | 3 |

Each castle has 1-4 **lines**, listed top to bottom on its card. A line
requires either specific dice symbols (any combination of Archery, Cavalry
and Daimyo, in any quantity), or a fixed amount of infantry, but never both.

**Dice.** Every die has six faces: `1 inf`, `2 inf`, `3 inf`, `arch`
(Archery), `cav` (Cavalry) and `dai` (Daimyo). The three infantry faces are
interchangeable towards a line's infantry total - a `3 inf` face is worth
three infantry points, not three dice.

## Turn Structure

### 1. Roll

At the start of your turn you automatically roll 7 dice.

### 2. Attack a castle

Declare an attack with `attack <castle>` (partial, unambiguous names work,
e.g. `attack azu` for Azuchi). You may attack:

- Any unconquered castle whose clan is not yet fully conquered.
- A castle already conquered by an opponent, to steal it - this adds an
  extra line to the castle requiring a single Daimyo die.

You may not attack a castle you already own, or any castle in a clan that
has been fully conquered by a single player - whether that player is you
or an opponent, castles in a completed clan are locked for the rest of the
game.

If none of your current dice can possibly complete even one line of any
attackable castle, your turn ends immediately without attacking.

### 3. Complete lines

Once attacking, complete any not-yet-completed line you can afford with
`line <n>`, where `<n>` is the line's number as shown on the castle (only
uncompleted lines are valid choices). Completing a line consumes just
enough dice to satisfy it:

- A symbol line consumes exactly the dice matching its listed symbols
  (e.g. a `cav cav` line needs two Cavalry dice).
- An infantry line consumes the fewest dice whose infantry values reach or
  exceed the requirement, spending your highest-value infantry dice first.

After a line is completed, every other remaining die (not spent on the
line) is rerolled - unless that completion was the castle's last line, in
which case the castle is conquered and your turn ends with no reroll. For
example, completing an archery+cavalry line with 2 of your 7 dice leaves
the other 5 to be rerolled.

### 4. Reroll unaffordable dice

If you can't or don't want to complete a line yet, `roll` discards one die
and rerolls the rest, shrinking your pool by one each time you use it.

### Turn ends when

- **Conquered**: every line on the attacked castle is completed. You become
  its owner (taking it from a previous owner if you stole it), your turn
  ends, and play passes to the next player.
- **Failed attack**: your remaining dice are too few to ever complete the
  castle's uncompleted lines, or you're down to exactly the minimum dice
  required but can't afford any of them with the faces you have. Play
  passes to the next player with no effect.

If you reroll before committing to an attack and your shrinking pool can no
longer reach any attackable castle, your turn ends the same way.

## Scoring

Score is calculated fresh whenever it's needed (not accumulated as you
play):

- If a player owns **every** castle in a clan, they score that clan's bonus
  once, instead of the sum of its castles' individual points.
- For every other conquered castle, in a clan that is not fully conquered
  by one player, its owner scores that castle's individual points.
- Unconquered castles score nothing for anyone.

Worked example, two players:

| Castle / clan | Conquered by | Individual points | Scored as |
|---|---|---|---|
| Azuchi, Matsumoto, Odani, Gifu (all of Oda) | Alice | 3+2+1+1 = 7 | Oda bonus: **10** (clan fully owned by Alice) |
| Kasugayama (Uesugi, not fully conquered) | Alice | 4 | **4** (individual) |
| Kitanosho (Uesugi, not fully conquered) | Bob | 3 | **3** (individual) |
| Edo (Tokugawa, not fully conquered) | Bob | 3 | **3** (individual) |
| Kiyosu, Inuyama (Tokugawa) | unconquered | - | **0** |
| Marugame (Chosokabe, not fully conquered) | Bob | 1 | **1** (individual) |
| Matsuyama (Chosokabe) | unconquered | - | **0** |

Alice: 10 + 4 = **14**. Bob: 3 + 3 + 1 = **7**. Note Alice's Oda total (10)
beats what the four castles would be worth individually (7) - completing a
clan is always worth at least as much as, and usually more than, owning the
same castles piecemeal.

## Game End

The game ends the instant all fourteen castles have been conquered.

## Winning

The player with the highest score (by the scoring rules above) wins. Ties
are broken by whoever has conquered the most clans outright; if that is
also tied, the players share the placing.

## Commands

| Command | Action | Example |
|---------|--------|---------|
| `attack <castle>` | Declare an attack on a castle (partial name match) | `attack azu` |
| `line <n>` | Complete line `n` of the castle you're attacking | `line 2` |
| `roll` | Discard one die and reroll the rest | `roll` |
