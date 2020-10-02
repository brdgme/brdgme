package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/battleship"
	"github.com/brdgme/brdgme/brdgme-go/cmd"
)

func main() {
	cmd.Cli(&battleship.Game{}, os.Stdin, os.Stdout)
}
