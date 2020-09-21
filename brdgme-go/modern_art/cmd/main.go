package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/modern_art"
)

func main() {
	cmd.Cli(&modern_art.Game{}, os.Stdin, os.Stdout)
}
