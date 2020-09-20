package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/texas_holdem"
)

func main() {
	cmd.Cli(&texas_holdem.Game{}, os.Stdin, os.Stdout)
}
