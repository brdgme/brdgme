package age_of_war

import (
	"fmt"
	"strings"

	"github.com/brdgme/brdgme-go/brdgme"
	"github.com/brdgme/brdgme-go/render"
)

const (
	Dice1Infantry = iota
	Dice2Infantry
	Dice3Infantry
	DiceArchery
	DiceCavalry
	DiceDaimyo
)

var DiceInfantry = map[int]int{
	Dice1Infantry: 1,
	Dice2Infantry: 2,
	Dice3Infantry: 3,
}

var DiceStrings = map[int]string{
	Dice1Infantry: "1 inf",
	Dice2Infantry: "2 inf",
	Dice3Infantry: "3 inf",
	DiceArchery:   "arch",
	DiceCavalry:   "cav",
	DiceDaimyo:    "dai",
}

var InfantryColour = render.Blue

var DiceColours = map[int]render.Color{
	Dice1Infantry: InfantryColour,
	Dice2Infantry: InfantryColour,
	Dice3Infantry: InfantryColour,
	DiceArchery:   render.Purple,
	DiceCavalry:   render.Green,
	DiceDaimyo:    render.Red,
}

func Roll() int {
	return rnd.Int() % 6
}

func RollN(n int) []int {
	if n <= 0 {
		return []int{}
	}
	ints := make([]int, n)
	for i := 0; i < n; i++ {
		ints[i] = Roll()
	}
	return ints
}

func (g *Game) Roll(n int) brdgme.Log {
	g.CurrentRoll = RollN(n)
	return brdgme.NewPublicLog(fmt.Sprintf(
		"%s rolled  %s",
		render.Player(g.CurrentPlayer),
		strings.Join(RenderDice(g.CurrentRoll), "  "),
	))
}
