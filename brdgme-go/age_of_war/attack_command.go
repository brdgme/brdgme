package age_of_war

import (
	"errors"
	"fmt"

	"brdgme-go/brdgme"
	"brdgme-go/render"
)

func (g *Game) AttackCommand(
	pNum int,
	castle int,
	remaining string,
) (brdgme.CommandResponse, error) {
	logs, err := g.Attack(pNum, castle)
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   true,
		Remaining: remaining,
	}, err
}

func (g *Game) CanAttack(player int) bool {
	return g.CurrentPlayer == player && g.CurrentlyAttacking == -1
}

func (g *Game) Attack(player, castle int) ([]brdgme.Log, error) {
	if !g.CanAttack(player) {
		return nil, errors.New("unable to attack a castle right now")
	}
	if castle < 0 || castle >= len(Castles) {
		return nil, errors.New("that is not a valid castle")
	}
	if g.Conquered[castle] && g.CastleOwners[castle] == player {
		return nil, errors.New("you have already conquered that castle")
	}
	if ok, _ := g.ClanConquered(Castles[castle].Clan); ok {
		return nil, errors.New("that clan is already conquered")
	}
	g.CurrentlyAttacking = castle
	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		"%s is attacking:\n%s",
		render.Player(player),
		g.RenderCastle(castle, []int{}),
	))}
	_, endLogs := g.CheckEndOfTurn()
	logs = append(logs, endLogs...)
	return logs, nil
}
