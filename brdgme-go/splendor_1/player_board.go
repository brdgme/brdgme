package splendor_1

import "github.com/brdgme/brdgme/brdgme-go/libcost"

type PlayerBoard struct {
	Cards   []Card
	Reserve []Card
	Nobles  []Noble
	Tokens  libcost.Cost
}

func NewPlayerBoard() PlayerBoard {
	return PlayerBoard{
		Cards:   []Card{},
		Reserve: []Card{},
		Nobles:  []Noble{},
		Tokens:  libcost.Cost{},
	}
}

func (pb PlayerBoard) Bonuses() libcost.Cost {
	bonuses := libcost.Cost{}
	for _, c := range pb.Cards {
		bonuses[c.Resource]++
	}
	return bonuses
}

func (pb PlayerBoard) BuyingPower() libcost.Cost {
	return pb.Bonuses().Add(pb.Tokens)
}

func (pb PlayerBoard) CanAfford(cost libcost.Cost) bool {
	return CanAfford(pb.BuyingPower(), cost)
}

func (pb PlayerBoard) Prestige() int {
	prestige := 0
	for _, c := range pb.Cards {
		prestige += c.Prestige
	}
	for _, n := range pb.Nobles {
		prestige += n.Prestige
	}
	return prestige
}
