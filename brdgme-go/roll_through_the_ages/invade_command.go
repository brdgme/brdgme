package roll_through_the_ages

import (
	"bytes"
	"errors"
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

// type InvadeCommand struct{}

// func (c InvadeCommand) Name() string { return "invade" }

// func (c InvadeCommand) Call(
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
// 	if err != nil || len(args) < 1 {
// 		return "", errors.New("you must specify how many spearheads to use")
// 	}
// 	amount, err := strconv.Atoi(args[0])
// 	if err != nil || amount < 1 {
// 		return "", errors.New("the amount must be a positive number")
// 	}

// 	return "", g.Invade(pNum, amount)
// }

// func (c InvadeCommand) Usage(player string, context interface{}) string {
// 	return "{{b}}invade #{{/b}} to use spearheads to inflict extra damage on other players, eg. {{b}}invade 2{{/b}}"
// }

func (g *Game) CanInvade(player int) bool {
	return g.CurrentPlayer == player && g.Phase == PhaseInvade &&
		g.Boards[player].Developments[DevelopmentSmithing] &&
		g.Boards[player].Goods[GoodSpearhead] > 0
}

func (g *Game) Invade(player, amount int) ([]brdgme.Log, error) {
	if !g.CanInvade(player) {
		return nil, errors.New("you can't invade at the moment")
	}
	if amount <= 0 {
		return nil, errors.New("you must specify a positive amount of spearheads")
	}
	sh := g.Boards[player].Goods[GoodSpearhead]
	if amount > sh {
		return nil, fmt.Errorf("you only have %d spearheads", sh)
	}

	g.Boards[player].Goods[GoodSpearhead] -= amount
	buf := bytes.NewBufferString(fmt.Sprintf(
		`%s used {{b}}%d{{/b}} spearheads to cause extra damage`,
		g.RenderName(player),
		amount,
	))
	playerCount := g.PlayerCount()
	for p := 0; p < playerCount; p++ {
		if p == player {
			continue
		}
		if g.Boards[p].HasBuilt(MonumentGreatWall) {
			buf.WriteString(fmt.Sprintf(
				"\n  %s avoids the extra damage with their wall",
				g.RenderName(p),
			))
		} else {
			g.Boards[p].Disasters += amount * 2
			buf.WriteString(fmt.Sprintf(
				"\n  %s takes {{b}}%d disaster points{{/b}}",
				g.RenderName(p),
				amount,
			))
		}
	}

	logs := []brdgme.Log{brdgme.NewPublicLog(buf.String())}
	logs = append(logs, g.NextPhase()...)
	return logs, nil
}

func (g *Game) InvadeCommand(
	player int,
	amount int,
	remaining string,
) (brdgme.CommandResponse, error) {
	logs, err := g.Invade(player, amount)
	if err != nil {
		return brdgme.CommandResponse{}, err
	}
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   false,
		Remaining: remaining,
	}, nil
}
