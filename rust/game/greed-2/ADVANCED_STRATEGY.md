# Greed Advanced Strategy

## Bust probabilities by dice count

The chance of rolling zero scoring dice (bust) depends on how many dice you roll:

| Dice rolled | Bust chance |
|-------------|-------------|
| 1 | 67% |
| 2 | 44% |
| 3 | 28% |
| 4 | 16% |
| 5 | 8% |
| 6 | 3% |

Only D and G score as singles. Triples of $, R, E, or e also score, which lowers the bust chance slightly above the naive (4/6)^N calculation.

## Positional play

- **Ahead (you have the highest score):** Play conservatively. Bank 300-500 per turn. You want to reach 5000 before anyone else, not maximize each turn.
- **Behind:** You need bigger turns. Push for 700+ by rolling more aggressively, especially when you have 4+ dice remaining after scoring.
- **Someone else is close to 5000:** You must score big or the game ends on their next trip around. Take risks you normally would not.

## Die combination values and priorities

- **D is the most valuable single die** (100 pts). Always score it. Four D is 1000 - if you have 3 D scored and roll a 4th, take it.
- **G is the second most valuable single** (50 pts). Score it, but do not re-roll many dice chasing a lone G.
- **Dollar triples (600)** are the best triple. Hold Dollars hoping for pairs.
- **E and e triples (300)** are the weakest triples. If you have 2 E/e and other scoring options, consider re-rolling rather than committing to a low-value triple.
- **The straight ($GREeD = 1000)** only appears on a full 6-die roll. Do not plan around it, but recognize it when it appears.

## Optimal stopping points

- **6 dice remaining (fresh roll):** Bust chance is only 3%. Almost always roll at least once after scoring.
- **4-5 dice remaining:** Bust chance 8-16%. If turn score is under 400, keep rolling.
- **3 dice remaining:** Bust chance 28%. Bank if turn score is 500+. Roll if under 300.
- **2 dice remaining:** Bust chance 44%. Bank if turn score is 300+. Only roll if desperate or turn score is negligible.
- **1 die remaining:** Bust chance 67%. Almost always bank unless turn score is under 100 or you are far behind.

## Endgame strategy

- The game ends when play returns to the first player after someone hits 5000. If you are the first player, you control when the game ends - hit 5000 and the round closes on your next turn.
- If you are not the first player and someone has 4500+, you may only get one more turn. Make it count - push for a big score rather than banking small.
- Going well over 5000 is fine. Placings are by total score, so a 6000-point turn that wins the game is better than a cautious 5000.

## Reading opponents

- If an opponent is at 4000+ and you are at 2000, conservative play will not save you. You need 2-3 turns of 1000+ to catch up.
- If all opponents are below 3000, you can afford to bank 400-500 per turn and grind toward 5000.
- Watch whether opponents are playing aggressively or conservatively. If they are pushing hard, you may need to match their pace.
