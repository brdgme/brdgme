package splendor

import (
	"errors"
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/libcost"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

/*
type TakeCommand struct{}

func (c TakeCommand) Name() string { return "take" }

func (c TakeCommand) Call(
	player string,
	context interface{},
	input *command.Reader,
) (string, error) {
	g := context.(*Game)
	pNum, found := g.PlayerNum(player)
	if !found {
		return "", errors.New("could not find player")
	}
	args, err := input.ReadLineArgs()
	if err != nil || len(args) == 0 {
		return "", errors.New("please specify two or three tokens")
	}
	tokens := []int{}
	gemStrings := GemStrings()
	for _, a := range args {
		t, err := helper.MatchStringInStringMap(a, gemStrings)
		if err != nil {
			return "", err
		}
		tokens = append(tokens, t)
	}
	return "", g.Take(pNum, tokens)
}

func (c TakeCommand) Usage(player string, context interface{}) string {
	return "{{b}}take ## ## (##){{_b}} to take two or three tokens, eg. {{b}}take di di{{_b}}.  If you take two you must take two of the same type of tokens, and there must be at least four in the supply.  If you take three, they must be three different tokens."
}
*/

func (g *Game) CanTake(player int) bool {
	return g.CurrentPlayer == player && g.Phase == PhaseMain
}

func (g *Game) Take(player int, tokens []int) ([]brdgme.Log, error) {
	if !g.CanTake(player) {
		return nil, errors.New("unable to take right now")
	}
	logs := []brdgme.Log{}
	switch l := len(tokens); l {
	case 2:
		if tokens[0] != tokens[1] {
			return nil, errors.New("must take the same type of tokens when taking two")
		}
		if g.Tokens[tokens[0]] < 4 {
			return nil, errors.New("can only take two when there are four or more remaining")
		}
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"%s took {{b}}2 %s{{/b}}",
			render.Player(player),
			RenderResourceColour(ResourceStrings[tokens[0]], tokens[0]),
		)))
	case 3:
		tokenStrs := []string{}
		for i, t := range tokens {
			if t == tokens[(i+1)%l] {
				return nil, errors.New("must take different tokens when taking three")
			}
			if g.Tokens[t] == 0 {
				return nil, errors.New("there aren't enough tokens remaning to take that")
			}
			tokenStrs = append(tokenStrs, render.Bold(
				RenderResourceColour(ResourceStrings[t], t)))
		}
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"%s took %s",
			render.Player(player),
			brdgme.CommaList(tokenStrs),
		)))
	default:
		return nil, errors.New("can only take two or three tokens")
	}
	amount := libcost.Cost{}
	for _, t := range tokens {
		amount[t] += 1
	}
	g.PlayerBoards[player].Tokens = g.PlayerBoards[player].Tokens.Add(amount)
	g.Tokens = g.Tokens.Sub(amount)
	logs = append(logs, g.NextPhase()...)
	return logs, nil
}
