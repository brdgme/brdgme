package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/farkle_1"
)

func main() {
	cmd.Cli(&farkle_1.Game{}, os.Stdin, os.Stdout)
}
