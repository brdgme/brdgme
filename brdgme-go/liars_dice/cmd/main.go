package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/liars_dice"
)

func main() {
	cmd.Cli(&liars_dice.Game{}, os.Stdin, os.Stdout)
}
