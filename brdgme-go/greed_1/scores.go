package greed_1

import (
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/libdie"
)

type Score struct {
	Dice  []Die
	Value int
}

func Scores() []Score {
	return []Score{
		{[]Die{DieDollar, DieDollar, DieDollar, DieDollar, DieDollar, DieDollar}, 5000},
		{[]Die{DieG, DieG, DieG, DieG, DieG, DieG}, 5000},
		{[]Die{DieR, DieR, DieR, DieR, DieR, DieR}, 5000},
		{[]Die{DieE1, DieE1, DieE1, DieE1, DieE1, DieE1}, 5000},
		{[]Die{DieE2, DieE2, DieE2, DieE2, DieE2, DieE2}, 5000},
		{[]Die{DieD, DieD, DieD, DieD, DieD, DieD}, 5000},
		{[]Die{DieD, DieD, DieD, DieD}, 1000},
		{[]Die{DieDollar, DieG, DieR, DieE1, DieE2, DieD}, 1000},
		{[]Die{DieDollar, DieDollar, DieDollar}, 600},
		{[]Die{DieG, DieG, DieG}, 500},
		{[]Die{DieR, DieR, DieR}, 400},
		{[]Die{DieE1, DieE1, DieE1}, 300},
		{[]Die{DieE2, DieE2, DieE2}, 300},
		{[]Die{DieD}, 100},
		{[]Die{DieG}, 50},
	}
}

func (s Score) ValueString(delim string) string {
	return RenderDice(s.Dice, delim)
}

func (s Score) Description() string {
	return fmt.Sprintf("%s (%d points)", s.ValueString(""), s.Value)
}

func ScoreStrings() (scoreStrings []string) {
	for _, s := range Scores() {
		valueString, err := libdie.DiceToValueString(s.Dice)
		if err != nil {
			panic(err.Error())
		}
		scoreStrings = append(scoreStrings, fmt.Sprintf("%s (%d points)",
			valueString, s.Value))
	}
	return
}

func AvailableScores(dice []Die) (available []Score) {
	available = []Score{}
	for _, s := range Scores() {
		isIn, _ := libdie.DiceInDice(s.Dice, dice)
		if isIn {
			available = append(available, s)
		}
	}
	return
}
