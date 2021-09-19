package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/sushizock_1"
)

func main() {
	cmd.Cli(&sushizock_1.Game{}, os.Stdin, os.Stdout)
}
