package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/farkle"
)

func main() {
	cmd.Cli(&farkle.Game{}, os.Stdin, os.Stdout)
}
