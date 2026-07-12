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

## Reading the Display

```brdgme
{{table}}{{row}}{{cell center}}{{b}}Current roll{{/b}}{{/cell}}{{/row}}{{row}}{{cell center}}{{b}}{{fg rgb(211,47,47)}}dai{{/fg}}{{/b}}   {{b}}{{fg rgb(123,31,162)}}arch{{/fg}}{{/b}}   {{b}}{{fg rgb(123,31,162)}}arch{{/fg}}{{/b}}   {{b}}{{fg rgb(25,118,210)}}3 inf{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell center}}{{/cell}}{{/row}}{{row}}{{cell center}}{{b}}Currently attacking{{/b}}{{/cell}}{{/row}}{{row}}{{cell center}}{{/cell}}{{/row}}{{row}}{{cell center}}{{table}}{{row}}{{cell center}}{{b}}{{fg rgb(97,97,97)}}Edo{{/fg}}{{/b}} (3){{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}1.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}complete{{/fg}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}2.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(123,31,162)}}arch{{/fg}}{{/b}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(56,142,60)}}cav{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}3.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}complete{{/fg}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell center}}{{/cell}}{{/row}}{{row}}{{cell center}}{{/cell}}{{/row}}{{row}}{{cell center}}{{b}}Castles{{/b}}{{/cell}}{{/row}}{{row}}{{cell center}}{{/cell}}{{/row}}{{row}}{{cell center}}{{table}}{{row}}{{cell center}}{{table}}{{row}}{{cell center}}{{b}}{{fg rgb(251,192,45)}}Azuchi{{/fg}}{{/b}} (3){{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}1.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(123,31,162)}}arch{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}2.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(56,142,60)}}cav{{/fg}}{{/b}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(56,142,60)}}cav{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}3.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(25,118,210)}}5 inf{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}      {{/cell}}{{cell center}}{{table}}{{row}}{{cell center}}{{b}}{{fg rgb(251,192,45)}}Matsumoto{{/fg}}{{/b}} (2){{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}1.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(123,31,162)}}arch{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}2.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(123,31,162)}}arch{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}3.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(25,118,210)}}7 inf{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}      {{/cell}}{{cell center}}{{table}}{{row}}{{cell center}}{{b}}{{fg rgb(251,192,45)}}Odani{{/fg}}{{/b}} (1){{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}1.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(25,118,210)}}10 inf{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}      {{/cell}}{{cell center}}{{table}}{{row}}{{cell center}}{{b}}{{fg rgb(251,192,45)}}Gifu{{/fg}}{{/b}} (1){{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}1.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(211,47,47)}}dai{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}2.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(123,31,162)}}arch{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}3.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(56,142,60)}}cav{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{/cell}}{{/row}}{{row}}{{cell center}}{{table}}{{row}}{{cell center}}{{table}}{{row}}{{cell center}}{{b}}{{fg rgb(97,97,97)}}Edo{{/fg}}{{/b}} (3){{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}1.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}complete{{/fg}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}2.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(123,31,162)}}arch{{/fg}}{{/b}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(56,142,60)}}cav{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}3.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}complete{{/fg}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}      {{/cell}}{{cell center}}{{table}}{{row}}{{cell center}}{{b}}{{fg rgb(97,97,97)}}Kiyosu{{/fg}}{{/b}} (2){{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}1.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(211,47,47)}}dai{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}2.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(123,31,162)}}arch{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}3.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(56,142,60)}}cav{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}4.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(25,118,210)}}3 inf{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}      {{/cell}}{{cell center}}{{table}}{{row}}{{cell center}}{{b}}{{fg rgb(97,97,97)}}Inuyama{{/fg}}{{/b}} (1){{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}1.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(211,47,47)}}dai{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}2.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(123,31,162)}}arch{{/fg}}{{/b}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(123,31,162)}}arch{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{/cell}}{{/row}}{{row}}{{cell center}}{{table}}{{row}}{{cell center}}{{table}}{{row}}{{cell center}}{{b}}{{fg rgb(123,31,162)}}Kasugayama{{/fg}}{{/b}} (4){{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}1.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(123,31,162)}}arch{{/fg}}{{/b}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(123,31,162)}}arch{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}2.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(56,142,60)}}cav{{/fg}}{{/b}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(56,142,60)}}cav{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}      {{/cell}}{{cell center}}{{table}}{{row}}{{cell center}}{{b}}{{fg rgb(123,31,162)}}Kitanosho{{/fg}}{{/b}} (3){{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}1.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(211,47,47)}}dai{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}2.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(123,31,162)}}arch{{/fg}}{{/b}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(56,142,60)}}cav{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}3.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(25,118,210)}}6 inf{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{/cell}}{{/row}}{{row}}{{cell center}}{{table}}{{row}}{{cell center}}{{table}}{{row}}{{cell center}}{{b}}{{fg rgb(211,47,47)}}Gassantoda{{/fg}}{{/b}} (2){{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}1.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(211,47,47)}}dai{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}2.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(25,118,210)}}8 inf{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}      {{/cell}}{{cell center}}{{table}}{{row}}{{cell center}}{{b}}{{fg rgb(211,47,47)}}Takahashi{{/fg}}{{/b}} (2){{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}1.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(56,142,60)}}cav{{/fg}}{{/b}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(56,142,60)}}cav{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}2.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(25,118,210)}}5 inf{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}3.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(25,118,210)}}2 inf{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{/cell}}{{/row}}{{row}}{{cell center}}{{table}}{{row}}{{cell center}}{{table}}{{row}}{{cell center}}{{b}}{{fg rgb(0,0,0)}}Matsuyama{{/fg}}{{/b}} (2){{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}1.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(211,47,47)}}dai{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}2.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(25,118,210)}}4 inf{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}3.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(25,118,210)}}4 inf{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}      {{/cell}}{{cell center}}{{table}}{{row}}{{cell center}}{{b}}{{fg rgb(0,0,0)}}Marugame{{/fg}}{{/b}} (1){{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}1.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(211,47,47)}}dai{{/fg}}{{/b}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(211,47,47)}}dai{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}2.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(56,142,60)}}cav{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{/cell}}{{/row}}{{row}}{{cell center}}{{table}}{{row}}{{cell center}}{{table}}{{row}}{{cell center}}{{b}}{{fg rgb(56,142,60)}}Kumamoto{{/fg}}{{/b}} (3){{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}1.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(211,47,47)}}dai{{/fg}}{{/b}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(211,47,47)}}dai{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}2.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(56,142,60)}}cav{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}3.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(123,31,162)}}arch{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{fg rgb(97,97,97)}}4.{{/fg}}{{/cell}}{{cell left}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}} {{/cell}}{{cell left}}{{b}}{{fg rgb(25,118,210)}}4 inf{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell center}}{{/cell}}{{/row}}{{row}}{{cell center}}{{b}}Scores{{/b}}{{/cell}}{{/row}}{{row}}{{cell center}}{{player 0}}: {{b}}0{{/b}}   {{player 1}}: {{b}}0{{/b}}{{/cell}}{{/row}}{{/table}}
```

- **Current roll**: your dice pool for this turn, shown in colour by face.
- **Currently attacking**: the castle you've declared an attack on, if any.
  Each numbered line shows either `complete` (already satisfied this turn)
  or the symbols/infantry it still needs. A line that is affordable with
  the current roll is marked with a green `X`. Before an attack is
  declared, the same `X` markers also appear on affordable lines of
  castles in the Castles list, showing at a glance which attacks the roll
  could immediately progress.
- **Castles**: all fourteen castles, grouped and left-to-right ordered by
  clan (Oda, Tokugawa, Uesugi, Mori, Chosokabe, Shimazu). Each castle shows
  its name, its individual point value in parentheses, and its lines. A
  conquered castle shows `(<player>)` under its name in place of line
  detail while it's not being attacked; a fully-conquered clan is replaced
  by a line stating who conquered it and for how many bonus points.
- **Scores**: each player's current total, recalculated live from the
  scoring rules above.

## Commands

| Command | Action | Example |
|---------|--------|---------|
| `attack <castle>` | Declare an attack on a castle (partial name match) | `attack azu` |
| `line <n>` | Complete line `n` of the castle you're attacking | `line 2` |
| `roll` | Discard one die and reroll the rest | `roll` |

## Strategy Tips

Tips will be added.
