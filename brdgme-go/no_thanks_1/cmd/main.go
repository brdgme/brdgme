package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/no_thanks_1"
)

func main() {
	cmd.Cli(&no_thanks_1.Game{}, os.Stdin, os.Stdout)
}
