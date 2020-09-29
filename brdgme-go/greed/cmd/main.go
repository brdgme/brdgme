package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/greed"
)

func main() {
	cmd.Cli(&greed.Game{}, os.Stdin, os.Stdout)
}
