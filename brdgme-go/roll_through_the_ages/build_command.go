package roll_through_the_ages

import (
	"errors"
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

func (g *Game) BuildCommand(
	player int,
	args BuildCommand,
	remaining string,
) (brdgme.CommandResponse, error) {
	switch args.Target.Type {
	case BuildTypeCity:
		return g.BuildCityCommand(player, args.Amount, remaining)
	case BuildTypeShip:
		return g.BuildShipCommand(player, args.Amount, remaining)
	case BuildTypeMonument:
		return g.BuildMonumentCommand(player, args.Amount, args.Target.Monument, remaining)
	}
	panic("unreachable")
}

func (c BuildCommand) Usage(player string, context interface{}) string {
	return "{{b}}build # (thing){{/b}} to build monuments or cities using workers, or ships using cloth and wood. Eg. {{b}}build 2 great{{/b}} or {{b}}build 3 city{{/b}} or {{b}}build 1 ship{{/b}}"
}

func (g *Game) CanBuild(player int) bool {
	return g.CanBuildBuilding(player) || g.CanBuildShip(player) || g.CanTrade(player)
}

func (g *Game) CanBuildBuilding(player int) bool {
	return g.CurrentPlayer == player && g.Phase == PhaseBuild &&
		g.RemainingWorkers > 0
}

func (g *Game) CanBuildShip(player int) bool {
	b := g.Boards[player]
	return g.CurrentPlayer == player && g.Phase == PhaseBuild &&
		b.Developments[DevelopmentShipping] &&
		b.Goods[GoodWood] > 0 && b.Goods[GoodCloth] > 0
}

func (g *Game) BuildCity(player, amount int) ([]brdgme.Log, error) {
	if !g.CanBuildBuilding(player) {
		return nil, errors.New("you can't build at the moment")
	}
	if amount < 1 {
		return nil, errors.New("amount must be a positive number")
	}
	if amount > g.RemainingWorkers {
		return nil, fmt.Errorf("you only have %d workers left", g.RemainingWorkers)
	}
	if g.Boards[player].CityProgress+amount > MaxCityProgress {
		return nil, errors.New("that is more than what remains to be built")
	}
	initialCities := g.Boards[player].Cities()
	g.RemainingWorkers -= amount
	g.Boards[player].CityProgress += amount
	logs := []brdgme.Log{
		brdgme.NewPublicLog(fmt.Sprintf(
			"{{player %d}} used {{b}}%d{{/b}} workers on {{b}}cities{{/b}}",
			player,
			amount,
		)),
	}
	newCities := g.Boards[player].Cities()
	if newCities > initialCities {
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"{{player %d}} now has {{b}}%d cities{{/b}}",
			player,
			newCities,
		)))
	}
	if !g.CanBuild(player) {
		logs = append(logs, g.NextPhase()...)
	}
	return logs, nil
}

func (g *Game) BuildCityCommand(player, amount int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.BuildCity(player, amount)
	if err != nil {
		return brdgme.CommandResponse{}, err
	}
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   true,
		Remaining: remaining,
	}, nil
}

func (g *Game) BuildShip(player, amount int) ([]brdgme.Log, error) {
	if !g.CanBuildShip(player) {
		return nil, errors.New("you can't build a ship at the moment")
	}
	if amount < 1 {
		return nil, errors.New("amount must be a positive number")
	}
	if w := g.Boards[player].Goods[GoodWood]; amount > w {
		return nil, fmt.Errorf("you only have %d wood left", w)
	}
	if c := g.Boards[player].Goods[GoodWood]; amount > c {
		return nil, fmt.Errorf("you only have %d cloth left", c)
	}
	if g.Boards[player].Ships+amount > 5 {
		return nil, errors.New("you can only have 5 ships")
	}

	g.Boards[player].Ships += amount
	g.Boards[player].Goods[GoodWood] -= amount
	g.Boards[player].Goods[GoodCloth] -= amount

	logs := []brdgme.Log{
		brdgme.NewPublicLog(fmt.Sprintf(
			"%s built {{b}}%d ships{{/b}}",
			g.RenderName(player),
			amount,
		)),
	}
	if !g.CanBuild(player) {
		logs = append(logs, g.NextPhase()...)
	}
	return logs, nil
}

func (g *Game) BuildShipCommand(player, amount int, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.BuildShip(player, amount)
	if err != nil {
		return brdgme.CommandResponse{}, err
	}
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   true,
		Remaining: remaining,
	}, nil
}

func (g *Game) BuildMonument(player, amount int, monument MonumentID) ([]brdgme.Log, error) {
	if !g.CanBuildBuilding(player) {
		return nil, errors.New("you can't build at the moment")
	}
	if amount < 1 {
		return nil, errors.New("amount must be a positive number")
	}
	if !ContainsInt(monument, Monuments) {
		return nil, errors.New("that isn't a valid monument")
	}
	if amount > g.RemainingWorkers {
		return nil, fmt.Errorf("you only have %d workers left", g.RemainingWorkers)
	}
	mv := MonumentValues[monument]
	if g.Boards[player].Monuments[monument]+amount > mv.Size {
		return nil, errors.New("that is more than what remains to be built")
	}
	g.RemainingWorkers -= amount
	g.Boards[player].Monuments[monument] += amount
	logs := []brdgme.Log{
		brdgme.NewPublicLog(fmt.Sprintf(
			"{{player %d}} used {{b}}%d{{/b}} workers on the {{b}}%s{{/b}}",
			player,
			amount,
			mv.Name,
		)),
	}
	if g.Boards[player].Monuments[monument] >= mv.Size {
		first := true
		for pNum := 0; pNum < g.PlayerCount(); pNum++ {
			if g.Boards[pNum].MonumentBuiltFirst[monument] {
				first = false
				break
			}
		}
		if first {
			g.Boards[player].MonumentBuiltFirst[monument] = true
		}
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			"{{player %d}} completed the {{b}}%s{{/b}}",
			player,
			mv.Name,
		)))
		g.CheckGameEndTriggered(player)
	}
	if !g.CanBuild(player) {
		logs = append(logs, g.NextPhase()...)
	}
	return logs, nil
}

func (g *Game) BuildMonumentCommand(player, amount int, monument MonumentID, remaining string) (brdgme.CommandResponse, error) {
	logs, err := g.BuildMonument(player, amount, monument)
	if err != nil {
		return brdgme.CommandResponse{}, err
	}
	return brdgme.CommandResponse{
		Logs:      logs,
		CanUndo:   true,
		Remaining: remaining,
	}, nil
}
