package greed

import (
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/die"
)

type Score struct {
	Dice  []Die
	Value int
}

func Scores() []Score {
	return []Score{
		{[]Die{DieG}, 50},
		{[]Die{DieD}, 100},
		{[]Die{DieE1, DieE1, DieE1}, 300},
		{[]Die{DieE2, DieE2, DieE2}, 300},
		{[]Die{DieR, DieR, DieR}, 400},
		{[]Die{DieG, DieG, DieG}, 500},
		{[]Die{DieDollar, DieDollar, DieDollar}, 600},
		{[]Die{DieD, DieD, DieD, DieD}, 1000},
		{[]Die{DieDollar, DieG, DieR, DieE1, DieE2, DieD}, 1000},
		{[]Die{DieDollar, DieDollar, DieDollar, DieDollar, DieDollar, DieDollar}, 5000},
		{[]Die{DieG, DieG, DieG, DieG, DieG, DieG}, 5000},
		{[]Die{DieR, DieR, DieR, DieR, DieR, DieR}, 5000},
		{[]Die{DieE1, DieE1, DieE1, DieE1, DieE1, DieE1}, 5000},
		{[]Die{DieE2, DieE2, DieE2, DieE2, DieE2, DieE2}, 5000},
		{[]Die{DieD, DieD, DieD, DieD, DieD, DieD}, 5000},
	}
}

func (s Score) ValueString() string {
	return RenderDice(s.Dice)
}

func (s Score) Description() string {
	return fmt.Sprintf("%s (%d points)", s.ValueString(), s.Value)
}

func ScoreStrings() (scoreStrings []string) {
	for _, s := range Scores() {
		valueString, err := die.DiceToValueString(s.Dice)
		if err != nil {
			panic(err.Error())
		}
		scoreStrings = append(scoreStrings, fmt.Sprintf("%s (%d points)",
			valueString, s.Value))
	}
	return
}

func AvailableScores(dice []Die) (available map[string]Score) {
	available = map[string]Score{}
	for _, s := range Scores() {
		isIn, _ := die.DiceInDice(s.Dice, dice)
		if isIn {
			available[s.ValueString()] = s
		}
	}
	return
}
