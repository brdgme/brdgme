# Liar's Dice Advanced Strategy

## Probability Calculation

With N total dice in play, each die has a 1/6 chance of showing any given face plus a 1/6 chance of being a wild 1. So the expected count matching a bid of value V (where V != 1) is N * 2/6 = N/3. For a bid of value 1, the expected count is N/6 (only actual 1s count, no wild bonus).

Use this to judge whether a bid is conservative (below expectation) or aggressive (above it). A bid at exactly N/3 is roughly a coin flip; below it favours the bidder, above it favours the caller.

## Bluffing Strategy

- Bid a value you don't hold to mislead opponents about your dice. If you hold many 4s, bid 5s to make opponents think 5s are plentiful.
- Bluff early in the round when the bid is low and the cost of being called is small (opponents are less likely to call a low bid).
- Don't bluff the same value repeatedly across rounds; observant opponents will pattern-match.
- A bluff bid of 1s is especially deceptive because opponents assume you hold the face you're bidding.

## Reading Opponents

- A player who raises quantity aggressively likely holds several of that face (or many 1s).
- A player who switches the bid value away from yours may not hold your value and is steering toward their own strength.
- A player who calls quickly likely holds few of the bid face and few 1s.
- Track bidding across rounds: a player who always bids 3s probably rolls well for 3s or is running a long bluff.

## Calling vs Raising

- Call when the bid quantity exceeds what's plausible given total dice and your own holdings. If you hold zero of the bid face and zero 1s, the remaining dice must cover the entire quantity.
- Raise when you have information that the current bid is safe (you hold matching dice) and you want to push the bidder further into danger.
- Raising is safer than calling when you're uncertain: a raise keeps you in control and forces the next player to make the hard decision.
- Calling is correct when any further raise would be a lie you can't back up.

## Position Play

- As first bidder, open conservatively (low quantity, a value you actually hold). You set the floor and gather information.
- As a later bidder, you have more information from prior bids. Use opponents' choices to infer their dice and bid accordingly.
- The player immediately after the bidder faces the most pressure: they must either escalate or call with the least information gain.

## Endgame (Few Dice Remaining)

- With few total dice in play, probability bounds tighten. A bid of "3 of a kind" with only 4 dice total is extremely aggressive.
- When each player has 1-2 dice, you can often deduce the exact distribution from bidding history.
- In heads-up (2 players), each player knows their own dice exactly, so the only unknown is the opponent's. Probability becomes a simple count of what the opponent could hold.
- Low-dice endgames favour calling: the variance is small, so bids above expectation are almost certainly bluffs.

## Using Wild Dice (1s) Strategically

- 1s are the most valuable dice because they match every bid. Holding multiple 1s makes almost any quantity bid safe for you.
- Bidding 1s as the face value removes the wild bonus (only actual 1s count), so only bid 1s when you hold several.
- When opponents bid a value you don't hold, check your 1s before considering a call; they may save you.
- In endgame, a single 1 can be the difference between a call succeeding or failing.
