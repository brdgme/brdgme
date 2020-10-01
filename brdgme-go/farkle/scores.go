package farkle

import (
	"fmt"

	"github.com/brdgme/brdgme/brdgme-go/libdie"
)

type Score struct {
	Dice  []int
	Value int
}

func Scores() []Score {
	return []Score{
		{[]int{5}, 50},
		{[]int{1}, 100},
		{[]int{2, 2, 2}, 200},
		{[]int{3, 3, 3}, 300},
		{[]int{4, 4, 4}, 400},
		{[]int{5, 5, 5}, 500},
		{[]int{6, 6, 6}, 600},
		{[]int{1, 1, 1}, 1000},
	}
}

func (s Score) ValueString() string {
	valueString, err := libdie.DiceToValueString(s.Dice)
	if err != nil {
		panic(err.Error())
	}
	return valueString
}

func (s Score) Description() string {
	return fmt.Sprintf("%s (%d points)", s.ValueString(), s.Value)
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

func AvailableScores(dice []int) (available map[string]Score) {
	available = map[string]Score{}
	for _, s := range Scores() {
		isIn, _ := libdie.DiceInDice(s.Dice, dice)
		if isIn {
			available[s.ValueString()] = s
		}
	}
	return
}
