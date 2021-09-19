package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cathedral_1"
	"github.com/brdgme/brdgme/brdgme-go/cmd"
)

func main() {
	cmd.Cli(&cathedral_1.Game{}, os.Stdin, os.Stdout)
}
