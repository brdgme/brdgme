package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/zombie_dice"
)

func main() {
	cmd.Cli(&zombie_dice.Game{}, os.Stdin, os.Stdout)
}
