package sushizock

import (
	"bytes"
	"errors"
	"fmt"
	"strconv"
	"strings"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme/brdgme-go/render"
)

type Game struct {
	Players         int
	CurrentPlayer   int
	BlueTiles       Tiles
	RedTiles        Tiles
	PlayerBlueTiles []Tiles
	PlayerRedTiles  []Tiles
	RolledDice      []int
	KeptDice        []int
	RemainingRolls  int
}

var _ brdgme.Gamer = &Game{}

func (g *Game) PlayerCount() int {
	return g.Players
}

func (g *Game) PlayerCounts() []int {
	return []int{2, 3, 4, 5}
}

func (g *Game) PlayerState(player int) interface{} {
	return nil
}

func (g *Game) PubState() interface{} {
	return nil
}

func (g *Game) Points() []float32 {
	pts := make([]float32, g.Players)
	for p := 0; p < g.Players; p++ {
		pts[p] = float32(g.PlayerScore(p))
	}
	return pts
}

func (g *Game) PlayerRender(player int) string {
	return g.PubRender()
}

func (g *Game) Status() brdgme.Status {
	if g.IsFinished() {
		return brdgme.StatusFinished{
			Placings: g.Placings(),
		}.ToStatus()
	}
	return brdgme.StatusActive{
		WhoseTurn: g.WhoseTurn(),
	}.ToStatus()
}

func (g *Game) PubRender() string {
	buf := bytes.NewBuffer([]byte{})

	// Dice
	diceCounts := g.DiceCounts()
	diceNumbers := make([]render.Cell, len(g.RolledDice))
	for i := range g.RolledDice {
		diceNumbers[i] = render.Cel(render.Fg(render.Grey, strconv.Itoa(i+1)))
	}
	dice := append(BoldStrings(DiceStrings(g.RolledDice)),
		DiceStrings(g.KeptDice)...)
	diceRow := make([]render.Cell, len(dice))
	for i, d := range dice {
		diceRow[i] = render.Cel(d)
	}
	cells := [][]render.Cell{
		diceRow,
		diceNumbers,
	}
	table := render.Table(cells, 0, 2)
	buf.WriteString(render.Bold("Dice\n"))
	buf.WriteString(table)
	buf.WriteString("\n\n")

	// Tiles
	blueTilesCells := g.BlueTiles.Cells()
	if diceCounts[DieSushi] > 0 && diceCounts[DieSushi] <= len(blueTilesCells) {
		blueTilesCells[diceCounts[DieSushi]-1].Content =
			render.Bold(blueTilesCells[diceCounts[DieSushi]-1].Content)
	}
	redTilesCells := g.RedTiles.Cells()
	if diceCounts[DieBones] > 0 && diceCounts[DieBones] <= len(redTilesCells) {
		redTilesCells[diceCounts[DieBones]-1].Content =
			render.Bold(redTilesCells[diceCounts[DieBones]-1].Content)
	}
	cells = [][]render.Cell{
		blueTilesCells,
		redTilesCells,
	}
	table = render.Table(cells, 0, 1)
	buf.WriteString(render.Bold("Tiles\n"))
	buf.WriteString(table)
	buf.WriteString("\n\n")

	// Players
	cells = [][]render.Cell{{
		render.Cel(render.Bold("Player")),
		render.Cel(render.Bold("Blue")),
		render.Cel(render.Bold("Red")),
	}}
	for pNum := 0; pNum < g.Players; pNum++ {
		blueText := render.Fg(render.Grey, "none")
		redText := blueText
		bLen := len(g.PlayerBlueTiles[pNum])
		if bLen > 0 {
			blueText = fmt.Sprintf(`%s %s`,
				g.PlayerBlueTiles[pNum][bLen-1].Render(),
				render.Fg(render.Grey, fmt.Sprintf("(%d tiles)", bLen)),
			)
		}
		rLen := len(g.PlayerRedTiles[pNum])
		if rLen > 0 {
			redText = fmt.Sprintf(`%s %s`,
				g.PlayerRedTiles[pNum][rLen-1].Render(),
				render.Fg(render.Grey, fmt.Sprintf("(%d tiles)", rLen)),
			)
		}
		cells = append(cells, []render.Cell{
			render.Cel(render.Player(pNum)),
			render.Cel(blueText),
			render.Cel(redText),
		})
	}
	table = render.Table(cells, 0, 2)
	buf.WriteString(table)
	return buf.String()
}

func (g *Game) New(players int) ([]brdgme.Log, error) {
	if players < 2 || players > 5 {
		return nil, errors.New("must be between 2 and 5 players")
	}

	g.Players = players
	g.BlueTiles = ShuffleTiles(BlueTiles())
	g.RedTiles = ShuffleTiles(RedTiles())
	g.PlayerBlueTiles = make([]Tiles, g.Players)
	g.PlayerRedTiles = make([]Tiles, g.Players)
	for p := 0; p < g.Players; p++ {
		g.PlayerBlueTiles[p] = Tiles{}
		g.PlayerRedTiles[p] = Tiles{}
	}
	return g.StartTurn(), nil
}

func (g *Game) StartTurn() []brdgme.Log {
	g.RolledDice = RollDice(5)
	g.KeptDice = []int{}
	g.RemainingRolls = 2
	return []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		`%s rolled  %s`,
		render.Player(g.CurrentPlayer),
		render.Bold(strings.Join(DiceStrings(g.RolledDice), "  ")),
	))}
}

func (g *Game) NextPlayer() []brdgme.Log {
	if g.IsFinished() {
		return g.LogGameEnd()
	}
	g.CurrentPlayer = (g.CurrentPlayer + 1) % g.Players
	return g.StartTurn()
}

func (g *Game) LogGameEnd() []brdgme.Log {
	buf := bytes.NewBuffer([]byte{})
	buf.WriteString(render.Bold("The game is now finished, scores are as follows:\n"))
	cells := [][]render.Cell{}
	for pNum := 0; pNum < g.Players; pNum++ {
		cells = append(cells, []render.Cell{
			render.Cel(render.Player(pNum)),
			render.Cel(render.Bold(strings.Join(
				append(append(Tiles{}, g.PlayerBlueTiles[pNum]...),
					g.PlayerRedTiles[pNum]...).RenderSlice(), " "))),
			render.Cel(render.Bold(fmt.Sprintf(
				"%d points",
				g.PlayerScore(pNum),
			))),
		})
	}
	table := render.Table(cells, 0, 2)
	buf.WriteString(table)
	return []brdgme.Log{brdgme.NewPublicLog(buf.String())}
}

func (g *Game) Dice() []int {
	dice := []int{}
	dice = append(dice, g.RolledDice...)
	dice = append(dice, g.KeptDice...)
	return dice
}

func (g *Game) DiceCounts() map[int]int {
	return DiceCounts(g.Dice())
}

func (g *Game) IsFinished() bool {
	return len(g.BlueTiles) == 0 && len(g.RedTiles) == 0
}

func (g *Game) PlayerScore(player int) int {
	return Score(g.PlayerBlueTiles[player], g.PlayerRedTiles[player])
}

func (g *Game) Placings() []int {
	metrics := make([][]int, g.Players)
	for p := 0; p < g.Players; p++ {
		metrics[p] = []int{g.PlayerScore(p)}
	}
	return brdgme.GenPlacings(metrics)
}

func (g *Game) WhoseTurn() []int {
	if g.IsFinished() {
		return []int{}
	}
	return []int{g.CurrentPlayer}
}

func (g *Game) CanTake(player int) bool {
	return g.CanTakeBlue(player) || g.CanTakeRed(player)
}

func (g *Game) CanTakeBlue(player int) bool {
	if player != g.CurrentPlayer {
		return false
	}
	diceCounts := g.DiceCounts()
	return diceCounts[DieSushi] > 0 &&
		len(g.BlueTiles) >= diceCounts[DieSushi]
}

func (g *Game) CanTakeRed(player int) bool {
	if player != g.CurrentPlayer {
		return false
	}
	diceCounts := g.DiceCounts()
	return diceCounts[DieBones] > 0 &&
		len(g.RedTiles) >= diceCounts[DieBones]
}

func (g *Game) TakeBlue(player int) ([]brdgme.Log, error) {
	if !g.CanTakeBlue(player) {
		return nil, errors.New("unable to take blue at the moment")
	}
	t, remaining := g.BlueTiles.Remove(g.DiceCounts()[DieSushi] - 1)
	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		`%s took %s`,
		render.Player(player),
		render.Bold(t.Render()),
	))}
	g.PlayerBlueTiles[player] = append(g.PlayerBlueTiles[player], t)
	g.BlueTiles = remaining
	logs = append(logs, g.NextPlayer()...)
	return logs, nil
}

func (g *Game) TakeRed(player int) ([]brdgme.Log, error) {
	if !g.CanTakeRed(player) {
		return nil, errors.New("unable to take red at the moment")
	}
	t, remaining := g.RedTiles.Remove(g.DiceCounts()[DieBones] - 1)
	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		`%s took %s`,
		render.Player(player),
		render.Bold(t.Render()),
	))}
	g.PlayerRedTiles[player] = append(g.PlayerRedTiles[player], t)
	g.RedTiles = remaining
	logs = append(logs, g.NextPlayer()...)
	return logs, nil
}

func (g *Game) CanRoll(player int) bool {
	return g.CurrentPlayer == player && g.RemainingRolls > 0 &&
		len(g.RolledDice) > 1
}

func (g *Game) RollDice(player int, dice []int) ([]brdgme.Log, error) {
	if !g.CanRoll(player) {
		return nil, errors.New("unable to roll at the moment")
	}
	rollMap := map[int]bool{}
	for _, d := range dice {
		if d < 1 || d > len(g.RolledDice) {
			return nil, fmt.Errorf("%d is not a valid die number", d)
		}
		rollMap[d-1] = true
	}
	if len(rollMap) == len(g.RolledDice) {
		return nil, fmt.Errorf("you must keep at least one die")
	}
	rolled := []int{}
	for i, d := range g.RolledDice {
		if !rollMap[i] {
			g.KeptDice = append(g.KeptDice, d)
		} else {
			rolled = append(rolled, d)
		}
	}
	g.RolledDice = RollDice(len(rollMap))
	g.RemainingRolls -= 1
	rolledStrs := append(BoldStrings(DiceStrings(g.RolledDice)),
		DiceStrings(g.KeptDice)...)
	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		"%s rolled  %s",
		render.Player(player),
		strings.Join(rolledStrs, "  "),
	))}
	if g.RemainingRolls == 0 || len(g.RolledDice) == 1 {
		g.KeptDice = append(g.KeptDice, g.RolledDice...)
		g.RolledDice = []int{}
		g.RemainingRolls = 0
		if !g.CanTake(player) && !g.CanSteal(player) {
			logs = append(logs, g.TakeWorst()...)
		}
	}
	return logs, nil
}

func (g *Game) CanSteal(player int) bool {
	return g.CanStealBlue(player) || g.CanStealRed(player)
}

func (g *Game) AnotherPlayerHasBlue(player int) bool {
	for p := 0; p < g.Players; p++ {
		if p != player && len(g.PlayerBlueTiles[p]) > 0 {
			return true
		}
	}
	return false
}

func (g *Game) AnotherPlayerHasRed(player int) bool {
	for p := 0; p < g.Players; p++ {
		if p != player && len(g.PlayerRedTiles[p]) > 0 {
			return true
		}
	}
	return false
}

func (g *Game) CanStealBlue(player int) bool {
	return player == g.CurrentPlayer && g.AnotherPlayerHasBlue(player) &&
		g.DiceCounts()[DieBlueChopsticks] >= 3
}

func (g *Game) CanStealRed(player int) bool {
	return player == g.CurrentPlayer && g.AnotherPlayerHasRed(player) &&
		g.DiceCounts()[DieRedChopsticks] >= 3
}

func (g *Game) CanStealBlueN(player int) bool {
	return player == g.CurrentPlayer && g.AnotherPlayerHasBlue(player) &&
		g.DiceCounts()[DieBlueChopsticks] >= 4
}

func (g *Game) CanStealRedN(player int) bool {
	return player == g.CurrentPlayer && g.AnotherPlayerHasRed(player) &&
		g.DiceCounts()[DieRedChopsticks] >= 4
}

func (g *Game) StealRed(player, targetPlayer int) ([]brdgme.Log, error) {
	if !g.CanStealRed(player) {
		return nil, errors.New("can't steal a red tile at the moment")
	}
	if player == targetPlayer {
		return nil, errors.New("can't steal from yourself")
	}
	if len(g.PlayerRedTiles[targetPlayer]) == 0 {
		return nil, errors.New("they don't have any red tiles to steal")
	}
	t, remaining := g.PlayerRedTiles[targetPlayer].Remove(
		len(g.PlayerRedTiles[targetPlayer]) - 1)
	g.PlayerRedTiles[player] = append(g.PlayerRedTiles[player], t)
	g.PlayerRedTiles[targetPlayer] = remaining
	logs := g.StealLog(player, targetPlayer, t)
	logs = append(logs, g.NextPlayer()...)
	return logs, nil
}

func (g *Game) StealBlue(player, targetPlayer int) ([]brdgme.Log, error) {
	if !g.CanStealBlue(player) {
		return nil, errors.New("can't steal a blue tile at the moment")
	}
	if player == targetPlayer {
		return nil, errors.New("can't steal from yourself")
	}
	if len(g.PlayerBlueTiles[targetPlayer]) == 0 {
		return nil, errors.New("they don't have any blue tiles to steal")
	}
	t, remaining := g.PlayerBlueTiles[targetPlayer].Remove(
		len(g.PlayerBlueTiles[targetPlayer]) - 1)
	g.PlayerBlueTiles[player] = append(g.PlayerBlueTiles[player], t)
	g.PlayerBlueTiles[targetPlayer] = remaining
	logs := g.StealLog(player, targetPlayer, t)
	logs = append(logs, g.NextPlayer()...)
	return logs, nil
}

func (g *Game) StealRedN(player, targetPlayer, n int) ([]brdgme.Log, error) {
	if n == 1 {
		return g.StealRed(player, targetPlayer)
	}
	if !g.CanStealRed(player) {
		return nil, errors.New("can't steal a hidden red tile at the moment")
	}
	if player == targetPlayer {
		return nil, errors.New("can't steal from yourself")
	}
	if len(g.PlayerRedTiles[targetPlayer]) == 0 {
		return nil, errors.New("they don't have any red tiles to steal")
	}
	index := len(g.PlayerRedTiles[targetPlayer]) - n
	if index < 0 || index >= len(g.PlayerRedTiles[targetPlayer]) {
		return nil, fmt.Errorf(
			"invalid tile number, you need to pick something between 1 and %d",
			len(g.PlayerRedTiles[targetPlayer]))
	}
	t, remaining := g.PlayerRedTiles[targetPlayer].Remove(index)
	g.PlayerRedTiles[player] = append(g.PlayerRedTiles[player], t)
	g.PlayerRedTiles[targetPlayer] = remaining
	logs := g.StealLog(player, targetPlayer, t)
	logs = append(logs, g.NextPlayer()...)
	return logs, nil
}

func (g *Game) StealBlueN(player, targetPlayer, n int) ([]brdgme.Log, error) {
	if n == 1 {
		return g.StealBlue(player, targetPlayer)
	}
	if !g.CanStealBlue(player) {
		return nil, errors.New("can't steal a hidden blue tile at the moment")
	}
	if player == targetPlayer {
		return nil, errors.New("can't steal from yourself")
	}
	if len(g.PlayerBlueTiles[targetPlayer]) == 0 {
		return nil, errors.New("they don't have any blue tiles to steal")
	}
	index := len(g.PlayerBlueTiles[targetPlayer]) - n
	if index < 0 || index >= len(g.PlayerBlueTiles[targetPlayer]) {
		return nil, fmt.Errorf(
			"invalid tile number, you need to pick something between 1 and %d",
			len(g.PlayerBlueTiles[targetPlayer]))
	}
	t, remaining := g.PlayerBlueTiles[targetPlayer].Remove(index)
	g.PlayerBlueTiles[player] = append(g.PlayerBlueTiles[player], t)
	g.PlayerBlueTiles[targetPlayer] = remaining
	logs := g.StealLog(player, targetPlayer, t)
	logs = append(logs, g.NextPlayer()...)
	return logs, nil
}

func (g *Game) StealLog(player, targetPlayer int, tile Tile) []brdgme.Log {
	return []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		`%s stole %s from %s`,
		render.Player(player),
		render.Bold(tile.Render()),
		render.Player(targetPlayer),
	))}
}

func (g *Game) TakeWorst() []brdgme.Log {
	var (
		t     Tile
		index int
	)
	if len(g.RedTiles) > 0 {
		for i, r := range g.RedTiles {
			if i == 0 || r.Value < t.Value {
				t = r
				index = i
			}
		}
		g.PlayerRedTiles[g.CurrentPlayer] =
			append(g.PlayerRedTiles[g.CurrentPlayer], t)
		_, g.RedTiles = g.RedTiles.Remove(index)
	} else {
		for i, b := range g.BlueTiles {
			if i == 0 || b.Value < t.Value {
				t = b
				index = i
			}
		}
		g.PlayerBlueTiles[g.CurrentPlayer] =
			append(g.PlayerBlueTiles[g.CurrentPlayer], t)
		_, g.BlueTiles = g.BlueTiles.Remove(index)
	}
	logs := []brdgme.Log{brdgme.NewPublicLog(fmt.Sprintf(
		`%s is forced to take %s`,
		render.Player(g.CurrentPlayer),
		render.Bold(t.Render()),
	))}
	logs = append(logs, g.NextPlayer()...)
	return logs
}

func BoldStrings(strs []string) []string {
	bolded := make([]string, len(strs))
	for i, s := range strs {
		bolded[i] = render.Bold(s)
	}
	return bolded
}
