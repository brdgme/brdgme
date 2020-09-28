package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cathedral"
	"github.com/brdgme/brdgme/brdgme-go/cmd"
)

func main() {
	cmd.Cli(&cathedral.Game{}, os.Stdin, os.Stdout)
}
