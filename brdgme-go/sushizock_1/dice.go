package sushizock_1

import (
	"math/rand"
	"time"

	"github.com/brdgme/brdgme/brdgme-go/render"
)

const (
	DieSushi = iota
	DieBlueChopsticks
	DieBones
	DieRedChopsticks
)

var DieFaces = []int{
	DieSushi,
	DieSushi,
	DieBones,
	DieBones,
	DieBlueChopsticks,
	DieRedChopsticks,
}

var DieText = map[int]string{
	DieSushi:          render.Fg(render.Blue, "Θ"),
	DieBlueChopsticks: render.Fg(render.Blue, "X"),
	DieBones:          render.Fg(render.Red, "¥"),
	DieRedChopsticks:  render.Fg(render.Red, "X"),
}

func RollDie() int {
	return DieFaces[rand.New(rand.NewSource(time.Now().UnixNano())).Int()%
		len(DieFaces)]
}

func RollDice(n int) []int {
	dice := make([]int, n)
	for i := 0; i < n; i++ {
		dice[i] = RollDie()
	}
	return dice
}

func DiceCounts(dice []int) map[int]int {
	counts := map[int]int{}
	for _, d := range dice {
		counts[d] += 1
	}
	return counts
}

func DiceStrings(dice []int) []string {
	strs := make([]string, len(dice))
	for i, d := range dice {
		strs[i] = DieText[d]
	}
	return strs
}
