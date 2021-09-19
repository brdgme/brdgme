package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/zombie_dice_1"
)

func main() {
	cmd.Cli(&zombie_dice_1.Game{}, os.Stdin, os.Stdout)
}
