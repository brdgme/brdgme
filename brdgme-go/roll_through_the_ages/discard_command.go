package roll_through_the_ages

import (
	"errors"
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

// type DiscardCommand struct{}

// func (c DiscardCommand) Name() string { return "discard" }

// func (c DiscardCommand) Call(
// 	player string,
// 	context interface{},
// 	input *command.Reader,
// ) (string, error) {
// 	g := context.(*Game)
// 	pNum, err := g.PlayerNum(player)
// 	if err != nil {
// 		return "", err
// 	}
// 	args, err := input.ReadLineArgs()
// 	if err != nil || len(args) < 2 {
// 		return "", errors.New(
// 			"you must pass an amount to discard and the name of a thing to discard")
// 	}

// 	amount, err := strconv.Atoi(args[0])
// 	if err != nil {
// 		return "", errors.New("you must specify an amount")
// 	}

// 	good, err := helper.MatchStringInStringMap(args[1], GoodStrings)
// 	if err != nil {
// 		return "", err
// 	}

// 	return "", g.Discard(pNum, amount, good)
// }

// func (c DiscardCommand) Usage(player string, context interface{}) string {
// 	return "{{b}}discard # (good){{/b}} to discard goods down to the required 6, eg. {{b}}discard 2 wood{{/b}}"
// }

func (g *Game) CanDiscard(player int) bool {
	return g.CurrentPlayer == player && g.Phase == PhaseDiscard
}

func (g *Game) Discard(player, amount int, good Good) ([]brdgme.Log, error) {
	if !g.CanDiscard(player) {
		return nil, errors.New("you can't discard at the moment")
	}
	if amount < 1 {
		return nil, errors.New("amount must be a positive number")
	}
	if !ContainsGood(good, Goods) {
		return nil, errors.New("invalid good")
	}
	if num := g.Boards[player].Goods[good]; amount > num {
		return nil, fmt.Errorf("you only have %d %s", num, GoodStrings[good])
	}
	if num := g.Boards[player].GoodsNum(); num-amount < 6 {
		return nil, fmt.Errorf("you only need to discard %d", num-6)
	}
	g.Boards[player].Goods[good] -= amount
	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		"%s discarded %d %s",
		g.RenderName(player),
		amount,
		RenderGoodName(good),
	))}
	if g.Boards[player].GoodsNum() <= 6 {
		logs = append(logs, g.NextTurn()...)
	}
	return logs, nil
}
