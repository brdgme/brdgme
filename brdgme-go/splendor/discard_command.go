package splendor

import (
	"errors"
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/libcost"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

/*
type DiscardCommand struct{}

func (c DiscardCommand) Name() string { return "discard" }

func (c DiscardCommand) Call(
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
		return "", errors.New("please specify at least one gem to discard")
	}
	tokens := []int{}
	gemStrings := GemStrings()
	// Can discard gold too
	gemStrings[Gold] = ResourceStrings[Gold]
	for _, a := range args {
		t, err := helper.MatchStringInStringMap(a, gemStrings)
		if err != nil {
			return "", err
		}
		tokens = append(tokens, t)
	}
	return "", g.Discard(pNum, tokens)
}

func (c DiscardCommand) Usage(player string, context interface{}) string {
	return "{{b}}discard ## (##...){{_b}} to discard tokens down to the maximum of 10, eg. {{b}}discard di go{{_b}}"
}
*/

func (g *Game) CanDiscard(player int) bool {
	return g.CurrentPlayer == player && g.Phase == PhaseDiscard
}

func (g *Game) Discard(player int, tokens []int) ([]brdgme.Log, error) {
	if !g.CanDiscard(player) {
		return nil, errors.New("unable to discard right now")
	}
	if len(tokens) == 0 {
		return nil, errors.New("please specify at least one token")
	}
	tCost := libcost.FromInts(tokens)
	if !g.PlayerBoards[player].Tokens.CanAfford(tCost) {
		return nil, errors.New("you don't have that many tokens")
	}

	g.PlayerBoards[player].Tokens = g.PlayerBoards[player].Tokens.Sub(tCost)
	g.Tokens = g.Tokens.Add(tCost)

	tokenStrs := []string{}
	for _, t := range tokens {
		tokenStrs = append(tokenStrs, render.Bold(
			RenderResourceColour(ResourceStrings[t], t)))
	}

	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		"%s discarded %s",
		render.Player(player),
		brdgme.CommaList(tokenStrs),
	))}

	if g.PlayerBoards[player].Tokens.Sum() <= MaxTokens {
		logs = append(logs, g.NextPhase()...)
	}

	return logs, nil
}
