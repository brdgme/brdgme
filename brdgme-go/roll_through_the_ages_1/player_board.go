package roll_through_the_ages_1

const (
	BaseCitySize = 3
	GoodsLimit   = 6
)

var CityLevels = []int{3, 7, 12, 18}

var MaxCityProgress = CityLevels[len(CityLevels)-1]

type PlayerBoard struct {
	CityProgress       int
	Developments       map[DevelopmentID]bool
	Monuments          map[MonumentID]int
	MonumentBuiltFirst map[MonumentID]bool
	Food               int
	Goods              map[Good]int
	Disasters          int
	Ships              int
}

func NewPlayerBoard() *PlayerBoard {
	return &PlayerBoard{
		Developments:       map[DevelopmentID]bool{},
		Monuments:          map[MonumentID]int{},
		MonumentBuiltFirst: map[MonumentID]bool{},
		Food:               3,
		Goods:              map[Good]int{},
	}
}

func (b *PlayerBoard) Cities() int {
	size := BaseCitySize
	for _, l := range CityLevels {
		if b.CityProgress < l {
			break
		}
		size += 1
	}
	return size
}

func (b *PlayerBoard) Score() int {
	score := 0
	// Developments
	for d, ok := range b.Developments {
		if !ok {
			continue
		}
		score += DevelopmentValues[d].Points
	}
	// Monuments
	builtMonuments := 0 // Track how many built for bonus score calculation
	for m, num := range b.Monuments {
		mv := MonumentValues[m]
		if num >= mv.Size {
			builtMonuments += 1
			if b.MonumentBuiltFirst[m] {
				score += mv.Points
			} else {
				score += mv.SubsequentPoints
			}
		}
	}
	// Bonus points
	if b.Developments[DevelopmentCommerce] {
		score += b.GoodsNum()
	}
	if b.Developments[DevelopmentArchitecture] {
		score += builtMonuments * 2
	}
	if b.Developments[DevelopmentEmpire] {
		score += b.Cities()
	}
	return score - b.Disasters
}

func (b *PlayerBoard) CoinsDieValue() int {
	if b.Developments[DevelopmentCoinage] {
		return 12
	}
	return 7
}

func (b *PlayerBoard) FoodModifier() int {
	if b.Developments[DevelopmentAgriculture] {
		return 1
	}
	return 0
}

func (b *PlayerBoard) WorkerModifier() int {
	if b.Developments[DevelopmentMasonry] {
		return 1
	}
	return 0
}

func (b *PlayerBoard) GainGoods(n int) {
	quarryingUsed := false
	good := GoodWood
	for i := 0; i < n; i++ {
		b.GainGood(good)
		// Extra stone if player has quarry
		if good == GoodStone && b.Developments[DevelopmentQuarrying] && !quarryingUsed {
			b.Goods[good] += 1
			quarryingUsed = true
		}
		good = (good + 1) % Good(len(Goods))
	}
}

func (b *PlayerBoard) GainGood(good Good) {
	max := GoodMaximum(good)
	if b.Goods[good] < max {
		b.Goods[good] += 1
	}
}

func (b *PlayerBoard) GoodsNum() int {
	num := 0
	for _, n := range b.Goods {
		num += n
	}
	return num
}

func (b *PlayerBoard) GoodsValue() int {
	val := 0
	for g, n := range b.Goods {
		val += GoodValue(g, n)
	}
	return val
}

func (b *PlayerBoard) HasBuilt(monument MonumentID) bool {
	return b.Monuments[monument] >= MonumentValues[monument].Size
}

func (b *PlayerBoard) GoodsOverLimit() int {
	if b.Developments[DevelopmentCaravans] == true {
		return 0
	}
	overLimit := b.GoodsNum() - GoodsLimit
	if overLimit < 0 {
		return 0
	}
	return overLimit
}
