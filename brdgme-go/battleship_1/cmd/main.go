package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/battleship_1"
	"github.com/brdgme/brdgme/brdgme-go/cmd"
)

func main() {
	cmd.Cli(&battleship_1.Game{}, os.Stdin, os.Stdout)
}
