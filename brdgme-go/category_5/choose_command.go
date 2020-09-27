package category_5

import (
	"errors"
	"fmt"
	"strconv"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

func (g *Game) CanChoose(player int) bool {
	return g.Resolving && g.ChoosePlayer == player
}

func (g *Game) Choose(player, row int) ([]brdgme.Log, error) {
	if !g.CanChoose(player) {
		return nil, errors.New("you can't choose at the moment")
	}

	if row < 1 || row > 4 {
		return nil, errors.New("the row must be between 1 and 4")
	}
	row -= 1

	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		"%s played %s and chose to take row %s for {{b}}%d points{{/b}}",
		render.Player(player),
		g.Plays[player],
		render.Bold(strconv.Itoa(row+1)),
		CardsHeads(g.Board[row]),
	))}

	g.PlayerCards[player] = append(g.PlayerCards[player], g.Board[row]...)
	g.Board[row] = []Card{g.Plays[player]}
	g.Plays[player] = 0

	logs = append(logs, g.ResolvePlays()...)

	return logs, nil
}
