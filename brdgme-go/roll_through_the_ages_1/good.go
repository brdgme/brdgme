package roll_through_the_ages_1

import "github.com/brdgme/brdgme/brdgme-go/render"

type Good int

const (
	GoodWood Good = iota
	GoodStone
	GoodPottery
	GoodCloth
	GoodSpearhead
)

var Goods = []Good{
	GoodWood,
	GoodStone,
	GoodPottery,
	GoodCloth,
	GoodSpearhead,
}

var GoodStrings = map[Good]string{
	GoodWood:      "wood",
	GoodStone:     "stone",
	GoodPottery:   "pottery",
	GoodCloth:     "cloth",
	GoodSpearhead: "spearhead",
}

var GoodColours = map[Good]render.Color{
	GoodWood:      render.Purple,
	GoodStone:     render.Grey,
	GoodPottery:   render.Red,
	GoodCloth:     render.Blue,
	GoodSpearhead: render.Yellow,
}

func GoodsReversed() []Good {
	l := len(Goods)
	rev := make([]Good, l)
	for i := range Goods {
		rev[i] = Good(l - i - 1)
	}
	return rev
}

func GoodMaximum(good Good) int {
	return int(8 - good)
}

func GoodValue(good Good, n int) int {
	return (n * (n + 1) / 2) * int(good+1)
}
