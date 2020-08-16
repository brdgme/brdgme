package roll_through_the_ages

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

var GoodColours = map[Good]string{
	GoodWood:      "magenta",
	GoodStone:     "gray",
	GoodPottery:   "red",
	GoodCloth:     "blue",
	GoodSpearhead: "yellow",
}

func GoodsReversed() []Good {
	l := len(Goods)
	rev := make([]Good, l)
	for i, _ := range Goods {
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
