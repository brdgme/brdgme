package roll_through_the_ages

import (
	"bytes"
	"errors"
	"fmt"
	"math/rand"
	"time"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

type Phase int

const (
	PhasePreserve Phase = iota
	PhaseRoll
	PhaseExtraRoll
	PhaseCollect
	PhaseResolve
	PhaseInvade
	PhaseBuild
	PhaseTrade
	PhaseBuy
	PhaseDiscard
)

var r = rand.New(rand.NewSource(time.Now().UnixNano()))

type Game struct {
	CurrentPlayer    int
	Phase            Phase
	Boards           []*PlayerBoard
	RolledDice       []Die
	KeptDice         []Die
	RemainingRolls   int
	RemainingCoins   int
	RemainingWorkers int
	RemainingShips   int
	FinalRound       bool
	Finished         bool
}

var _ brdgme.Gamer = &Game{}

func (g *Game) Command(
	player int,
	input string,
	playerNames []string,
) (brdgme.CommandResponse, error) {
	parseOutput, err := g.CommandParser(player).Parse(input, playerNames)
	if err != nil {
		return brdgme.CommandResponse{}, err
	}
	switch value := parseOutput.Value.(type) {
	case NextCommand:
		return g.NextCommand(player, parseOutput.Remaining)
	case TradeCommand:
		return g.TradeCommand(player, value.Amount, parseOutput.Remaining)
	case BuildCommand:
		return g.BuildCommand(player, value, parseOutput.Remaining)
	case TakeCommand:
		return g.TakeCommand(player, value.Actions, parseOutput.Remaining)
	case BuyCommand:
		return g.BuyCommand(player, value, parseOutput.Remaining)
	case DiscardCommand:
		return g.DiscardCommand(player, value.Amount, value.Good, parseOutput.Remaining)
	case InvadeCommand:
		return g.InvadeCommand(player, value.Amount, parseOutput.Remaining)
	case RollCommand:
		return g.RollCommand(player, value.Dice, parseOutput.Remaining)
	case SellCommand:
		return g.SellCommand(player, value.Amount, parseOutput.Remaining)
	case PreserveCommand:
		return g.PreserveCommand(player, parseOutput.Remaining)
	case SwapCommand:
		return g.SwapCommand(player, value.Amount, value.From, value.To, parseOutput.Remaining)
	}
	return brdgme.CommandResponse{}, errors.New("inexhaustive command handler")
}

func (g *Game) CommandSpec(player int) *brdgme.Spec {
	parser := g.CommandParser(player)
	if parser != nil {
		spec := parser.ToSpec()
		return &spec
	}
	return nil
}

func (g *Game) PlayerCounts() []int {
	return []int{2, 3, 4}
}

func (g *Game) New(players int) ([]brdgme.Log, error) {
	if players < 2 || players > 4 {
		return nil, errors.New("Roll Through the Ages is 2-4 player")
	}
	g.Boards = make([]*PlayerBoard, players)
	for i := 0; i < players; i++ {
		g.Boards[i] = NewPlayerBoard()
	}
	return g.StartTurn(), nil
}

func (g *Game) PlayerCount() int {
	return len(g.Boards)
}

func (g *Game) IsFinished() bool {
	return g.Finished
}

func (g *Game) Winners() []int {
	if !g.IsFinished() {
		return []int{}
	}
	winners := []int{}
	winningScore := 0
	players := g.PlayerCount()
	for p := 0; p < players; p++ {
		score := g.Boards[p].Score()
		if score > winningScore {
			winners = []int{}
			winningScore = score
		}
		if score == winningScore {
			winners = append(winners, p)
		}
	}
	if len(winners) < 2 {
		return winners
	}
	// There's a tie, goods value is tie breaker
	goodsWinners := []int{}
	goodsScore := 0
	for p := range winners {
		score := g.Boards[p].GoodsValue()
		if score > goodsScore {
			goodsWinners = []int{}
			goodsScore = score
		}
		if score == goodsScore {
			goodsWinners = append(goodsWinners, p)
		}
	}
	return goodsWinners
}

func (g *Game) WhoseTurn() []int {
	return []int{g.CurrentPlayer}
}

func (g *Game) StartTurn() []brdgme.Log {
	g.RemainingCoins = 0
	g.RemainingWorkers = 0
	return g.PreservePhase()
}

func (g *Game) NextPhase() []brdgme.Log {
	switch g.Phase {
	case PhasePreserve:
		return g.RollPhase()
	case PhaseRoll:
		return g.RollExtraPhase()
	case PhaseExtraRoll:
		return g.CollectPhase()
	case PhaseCollect:
		return g.PhaseResolve()
	case PhaseResolve, PhaseInvade:
		return g.BuildPhase()
	case PhaseBuild:
		return g.TradePhase()
	case PhaseTrade:
		return g.BuyPhase()
	case PhaseBuy:
		return g.DiscardPhase()
	case PhaseDiscard:
		return g.NextTurn()
	}
	panic("unreachable")
}

func (g *Game) PreservePhase() []brdgme.Log {
	g.Phase = PhasePreserve
	if !g.CanPreserve(g.CurrentPlayer) {
		return g.NextPhase()
	}
	return []brdgme.Log{}
}

func (g *Game) RollPhase() []brdgme.Log {
	g.Phase = PhaseRoll
	logs := g.NewRoll(g.Boards[g.CurrentPlayer].Cities())
	g.RemainingRolls = 2
	return logs
}

func (g *Game) RollExtraPhase() []brdgme.Log {
	g.Phase = PhaseExtraRoll
	// Can reroll anything
	g.RolledDice = append(g.RolledDice, g.KeptDice...)
	g.KeptDice = []Die{}
	if !g.Boards[g.CurrentPlayer].Developments[DevelopmentLeadership] {
		return g.NextPhase()
	}
	return []brdgme.Log{}
}

func (g *Game) CollectPhase() []brdgme.Log {
	g.Phase = PhaseCollect
	g.KeptDice = append(g.RolledDice, g.KeptDice...)
	g.RolledDice = []Die{}
	// Collect goods and food
	cp := g.CurrentPlayer
	hasFoodOrWorkersDice := false
	goods := 0
	for _, d := range g.KeptDice {
		switch d {
		case DiceFood:
			g.Boards[cp].Food += 3 + g.Boards[cp].FoodModifier()
		case DiceGood:
			goods += 1
		case DiceSkull:
			goods += 2
		case DiceWorkers:
			g.RemainingWorkers += 3 + g.Boards[cp].WorkerModifier()
		case DiceFoodOrWorkers:
			hasFoodOrWorkersDice = true
		case DiceCoins:
			g.RemainingCoins += g.Boards[cp].CoinsDieValue()
		}
	}
	g.Boards[cp].GainGoods(goods)
	if !hasFoodOrWorkersDice {
		return g.NextPhase()
	}
	return []brdgme.Log{}
}

func (g *Game) PhaseResolve() []brdgme.Log {
	g.Phase = PhaseResolve
	cp := g.CurrentPlayer
	players := g.PlayerCount()
	logs := []brdgme.Log{}
	// Check food isn't over maximum
	if g.Boards[cp].Food > 15 {
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			`{{player %d}} had their food reduced from {{b}}%d{{/b}} to the maximum of {{b}}15{{/b}}`,
			cp,
			g.Boards[cp].Food,
		)))
		g.Boards[cp].Food = 15
	}
	// Feed cities
	if cities := g.Boards[cp].Cities(); g.Boards[cp].Food >= cities {
		g.Boards[cp].Food -= cities
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			`{{player %d}} fed {{b}}%d{{/b}} cities`,
			cp,
			cities,
		)))
	} else {
		// Famine
		famine := cities - g.Boards[cp].Food
		g.Boards[cp].Food = 0
		g.Boards[cp].Disasters += famine
		logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
			`Famine! {{player %d}} takes {{b}}%d disaster points{{/b}}`,
			cp,
			famine,
		)))
	}
	// Resolve disasters
	skulls := 0
	for _, d := range g.KeptDice {
		if d == DiceSkull {
			skulls++
		}
	}
	switch skulls {
	case 0, 1:
		break
	case 2:
		if g.Boards[cp].Developments[DevelopmentIrrigation] {
			logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
				`{{player %d}} avoids a drought with their irrigation development`,
				cp,
			)))
		} else {
			g.Boards[cp].Disasters += 2
			logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
				`Drought! {{player %d}} takes {{b}}2 disaster points{{/b}}`,
				cp,
			)))
		}
	case 3:
		buf := bytes.NewBufferString("Pestilence!")
		for p := 0; p < players; p++ {
			if p == cp {
				continue
			}
			if g.Boards[p].Developments[DevelopmentMedicine] {
				buf.WriteString(fmt.Sprintf(
					"\n  {{player %d}} avoids pestilence with their medicine development",
					p,
				))
			} else {
				g.Boards[p].Disasters += 3
				buf.WriteString(fmt.Sprintf(
					"\n  {{player %d}} takes {{b}}3 disaster points{{/b}}",
					p,
				))
			}
		}
		logs = append(logs, brdgme.NewPublicLog(buf.String()))
	case 4:
		if g.Boards[cp].Developments[DevelopmentSmithing] {
			buf := bytes.NewBufferString(fmt.Sprintf(
				"Invasion! {{player %d}} has the smithing development, so {{b}}all other players are invaded{{/b}}",
				cp,
			))
			for p := 0; p < players; p++ {
				if p == cp {
					continue
				}
				if g.Boards[p].HasBuilt(MonumentGreatWall) {
					buf.WriteString(fmt.Sprintf(
						"\n  {{player %d}} avoids an invasion with their wall",
						p,
					))
				} else {
					g.Boards[p].Disasters += 4
					buf.WriteString(fmt.Sprintf(
						"\n  {{player %d}} takes {{b}}4 disaster points{{/b}}",
						p,
					))
				}
			}
			logs = append(logs, brdgme.NewPublicLog(buf.String()))
			logs = append(logs, g.InvadePhase()...)
			return logs
		} else if g.Boards[cp].HasBuilt(MonumentGreatWall) {
			logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
				`{{player %d}} avoids an invasion with their wall`,
				cp,
			)))
		} else {
			g.Boards[cp].Disasters += 4
			logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
				`Invasion! {{player %d}} takes {{b}}4 disaster points{{/b}}`,
				cp,
			)))
		}
	default:
		if g.Boards[cp].Developments[DevelopmentReligion] {
			for p := 0; p < players; p++ {
				if p == cp {
					continue
				}
				for _, good := range Goods {
					g.Boards[p].Goods[good] = 0
				}
			}
			logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
				`Revolt! {{player %d}} has the religion development, so {{b}}all other players{{/b}} lose {{b}}all of their goods{{/b}}`,
				cp,
			)))
		} else {
			for _, good := range Goods {
				g.Boards[cp].Goods[good] = 0
			}
			logs = append(logs, brdgme.NewPublicLog(fmt.Sprintf(
				`Revolt! {{player %d}} loses {{b}}all of their goods{{/b}}`,
				cp,
			)))
		}
	}
	logs = append(logs, g.NextPhase()...)
	return logs
}

func (g *Game) InvadePhase() []brdgme.Log {
	g.Phase = PhaseInvade
	if !g.CanInvade(g.CurrentPlayer) {
		return g.NextPhase()
	}
	return []brdgme.Log{}
}

func (g *Game) BuildPhase() []brdgme.Log {
	g.Phase = PhaseBuild
	if !g.CanBuildOrTrade(g.CurrentPlayer) {
		return g.NextPhase()
	}
	return []brdgme.Log{}
}

func (g *Game) TradePhase() []brdgme.Log {
	g.Phase = PhaseTrade
	g.RemainingShips = g.Boards[g.CurrentPlayer].Ships
	if g.Boards[g.CurrentPlayer].Ships == 0 ||
		g.Boards[g.CurrentPlayer].GoodsNum() == 0 {
		return g.NextPhase()
	}
	return []brdgme.Log{}
}

func (g *Game) BuyPhase() []brdgme.Log {
	g.Phase = PhaseBuy
	b := g.Boards[g.CurrentPlayer]
	buyingPower := g.RemainingCoins + b.GoodsValue()
	if b.Developments[DevelopmentGranaries] {
		buyingPower += b.Food * 6
	}
	if buyingPower < 10 {
		return g.NextPhase()
	}
	return []brdgme.Log{}
}

func (g *Game) DiscardPhase() []brdgme.Log {
	g.Phase = PhaseDiscard
	if g.Boards[g.CurrentPlayer].GoodsNum() <= 6 ||
		g.Boards[g.CurrentPlayer].Developments[DevelopmentCaravans] {
		return g.NextPhase()
	}
	return []brdgme.Log{}
}

func (g *Game) NextTurn() []brdgme.Log {
	g.CurrentPlayer = (g.CurrentPlayer + 1) % g.PlayerCount()
	if g.CurrentPlayer == 0 && g.FinalRound {
		g.Finished = true
	}
	if !g.IsFinished() {
		return g.StartTurn()
	}
	return []brdgme.Log{}
}

func (g *Game) CheckGameEndTriggered(player int) []brdgme.Log {
	if g.FinalRound {
		// End game already triggered
		return nil
	}
	// 5th development built
	if len(g.Boards[player].Developments) >= 7 {
		return g.TriggerGameEnd()
	}
	// Every monument built
	for _, m := range Monuments {
		built := false
		for _, b := range g.Boards {
			if b.HasBuilt(m) {
				built = true
				break
			}
		}
		if !built {
			return nil
		}
	}
	// All were built
	return g.TriggerGameEnd()
}

func (g *Game) TriggerGameEnd() []brdgme.Log {
	g.FinalRound = true
	return []brdgme.Log{brdgme.NewPublicLog(
		"{{b}}Game end has been triggered, the game will be finished after the last player has their turn{{/b}}",
	)}
}

func (g *Game) AvailableMonuments(player int) []MonumentID {
	available := []MonumentID{}
	for _, m := range Monuments {
		if g.Boards[player].Monuments[m] < MonumentValues[m].Size {
			available = append(available, m)
		}
	}
	return available
}

func (g *Game) AvailableDevelopments(player int) []DevelopmentID {
	available := []DevelopmentID{}
	for _, d := range Developments {
		if !g.Boards[player].Developments[d] {
			available = append(available, d)
		}
	}
	return available
}

func ContainsInt(needle int, haystack []int) bool {
	for _, i := range haystack {
		if needle == i {
			return true
		}
	}
	return false
}

func Contains(needle interface{}, haystack []interface{}) bool {
	for _, i := range haystack {
		if needle == i {
			return true
		}
	}
	return false
}

func ContainsGood(needle Good, haystack []Good) bool {
	for _, i := range haystack {
		if needle == i {
			return true
		}
	}
	return false
}

func (g *Game) PlayerState(player int) interface{} {
	return nil
}

func (g *Game) PubState() interface{} {
	return nil
}

func (g *Game) Points() []float32 {
	points := make([]float32, len(g.Boards))
	for i, b := range g.Boards {
		points[i] = float32(b.Score())
	}
	return points
}

func (g *Game) Status() brdgme.Status {
	if g.IsFinished() {
		scores := make([][]int, len(g.Boards))
		for i, b := range g.Boards {
			scores[i] = []int{b.Score()}
		}
		return brdgme.StatusFinished{
			Placings: brdgme.GenPlacings(scores),
			Stats:    []interface{}{},
		}.ToStatus()
	}
	return brdgme.StatusActive{
		WhoseTurn:  g.WhoseTurn(),
		Eliminated: []int{},
	}.ToStatus()
}
