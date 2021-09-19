package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/liars_dice_1"
)

func main() {
	cmd.Cli(&liars_dice_1.Game{}, os.Stdin, os.Stdout)
}
