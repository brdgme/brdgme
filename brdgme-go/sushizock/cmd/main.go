package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/sushizock"
)

func main() {
	cmd.Cli(&sushizock.Game{}, os.Stdin, os.Stdout)
}
