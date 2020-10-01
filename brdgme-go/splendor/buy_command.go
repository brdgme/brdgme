package splendor

import (
	"errors"
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

/*
type BuyCommand struct{}

func (c BuyCommand) Name() string { return "buy" }

func (c BuyCommand) Call(
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
		return "", errors.New("you must specify which card")
	}
	row, col, err := ParseLoc(args[0])
	if err != nil {
		return "", err
	}
	return "", g.Buy(pNum, row, col)
}

func (c BuyCommand) Usage(player string, context interface{}) string {
	return "{{b}}buy ##{{_b}} to buy a card from the board or your reserve, eg. {{b}}buy 2B{{_b}}"
}
*/

func (g *Game) CanBuy(player int) bool {
	return g.CurrentPlayer == player && g.Phase == PhaseMain
}

func (g *Game) Buy(player, row, col int) ([]brdgme.Log, error) {
	if !g.CanBuy(player) {
		return nil, errors.New("unable to buy right now")
	}
	pb := g.PlayerBoards[player]
	logs := []brdgme.Log{}
	switch row {
	case 0, 1, 2:
		if col < 0 || col >= len(g.Board[row]) {
			return nil, errors.New("that is not a valid card")
		}
		if !pb.CanAfford(g.Board[row][col].Cost) {
			return nil, errors.New("you can't afford that card")
		}
		c := g.Board[row][col]
		_ = g.Pay(player, c.Cost)
		g.PlayerBoards[player].Cards = append(
			g.PlayerBoards[player].Cards, c)
		if len(g.Decks[row]) > 0 {
			g.Board[row][col] = g.Decks[row][0]
			g.Decks[row] = g.Decks[row][1:]
		} else {
			g.Board[row] = append(
				g.Board[row][:col],
				g.Board[row][col+1:]...,
			)
		}
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"%s bought %s from the board",
			render.Player(player),
			RenderCard(c),
		)))
	case 3:
		if col < 0 || col >= len(pb.Reserve) {
			return nil, errors.New("that is not a valid reserve card")
		}
		if !pb.CanAfford(pb.Reserve[col].Cost) {
			return nil, errors.New("you can't afford that card")
		}
		c := pb.Reserve[col]
		_ = g.Pay(player, c.Cost)
		g.PlayerBoards[player].Cards = append(
			g.PlayerBoards[player].Cards, c)
		g.PlayerBoards[player].Reserve = append(
			g.PlayerBoards[player].Reserve[:col],
			g.PlayerBoards[player].Reserve[col+1:]...,
		)
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"%s bought %s from their reserve",
			render.Player(player),
			RenderCard(c),
		)))
	default:
		return nil, errors.New("that is not a valid row")
	}
	logs = append(logs, g.NextPhase()...)
	return logs, nil
}
