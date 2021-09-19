package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/modern_art_1"
)

func main() {
	cmd.Cli(&modern_art_1.Game{}, os.Stdin, os.Stdout)
}
