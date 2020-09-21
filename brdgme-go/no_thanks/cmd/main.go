package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/no_thanks"
)

func main() {
	cmd.Cli(&no_thanks.Game{}, os.Stdin, os.Stdout)
}
