package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/texas_holdem_1"
)

func main() {
	cmd.Cli(&texas_holdem_1.Game{}, os.Stdin, os.Stdout)
}
