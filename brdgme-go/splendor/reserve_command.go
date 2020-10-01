package splendor

import (
	"errors"
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

/*
type ReserveCommand struct{}

func (c ReserveCommand) Name() string { return "reserve" }

func (c ReserveCommand) Call(
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
	if err != nil || len(args) < 1 {
		return "", errors.New("you must specify a card")
	}
	row, col, err := ParseLoc(args[0])
	if err != nil {
		return "", err
	}
	return "", g.Reserve(pNum, row, col)
}

func (c ReserveCommand) Usage(player string, context interface{}) string {
	return "{{b}}reserve #{{_b}} to reserve a card for yourself, eg. {{b}}reserve 2B{{_b}}"
}
*/

func (g *Game) CanReserve(player int) bool {
	return g.CurrentPlayer == player && g.Phase == PhaseMain &&
		len(g.PlayerBoards[player].Reserve) < 3
}

func (g *Game) Reserve(player, row, col int) ([]brdgme.Log, error) {
	if !g.CanReserve(player) {
		return nil, errors.New("unable to reserve right now")
	}
	if row < 0 || row > 2 {
		return nil, errors.New("that is not a valid row")
	}
	if col < 0 || col >= len(g.Board[row]) {
		return nil, errors.New("that is not a valid card")
	}
	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		"%s reserved %s",
		render.Player(player),
		RenderCard(g.Board[row][col]),
	))}
	g.PlayerBoards[player].Reserve = append(
		g.PlayerBoards[player].Reserve,
		g.Board[row][col],
	)
	if g.Tokens[Gold] > 0 {
		g.PlayerBoards[player].Tokens[Gold] += 1
		g.Tokens[Gold] -= 1
	}
	if len(g.Decks[row]) > 0 {
		g.Board[row][col] = g.Decks[row][0]
		g.Decks[row] = g.Decks[row][1:]
	} else {
		g.Board[row] = append(
			g.Board[row][:col],
			g.Board[row][col+1:]...,
		)
	}
	logs = append(logs, g.NextPhase()...)
	return logs, nil
}
