package roll_through_the_ages_1

import (
	"github.com/brdgme/brdgme/brdgme-go/render"
)

type Die int

const (
	DiceFood Die = iota
	DiceGood
	DiceSkull
	DiceWorkers
	DiceFoodOrWorkers
	DiceCoins
)

var DiceFaces = []Die{
	DiceFood,
	DiceGood,
	DiceSkull,
	DiceWorkers,
	DiceFoodOrWorkers,
	DiceCoins,
}

var DiceStrings = map[Die]string{
	DiceFood:          "FFF",
	DiceGood:          "G",
	DiceSkull:         "GXG",
	DiceWorkers:       "WWW",
	DiceFoodOrWorkers: "FF/WW",
	DiceCoins:         "C",
}

var DiceValueColours = map[string]render.Color{
	"F": render.Green,
	"G": render.Purple,
	"X": render.Red,
	"W": render.Cyan,
	"C": render.Yellow,
}

func Roll() Die {
	return Die(r.Int() % len(DiceFaces))
}

func RollN(n int) []Die {
	dice := make([]Die, n)
	for i := 0; i < n; i++ {
		dice[i] = Roll()
	}
	return dice
}
