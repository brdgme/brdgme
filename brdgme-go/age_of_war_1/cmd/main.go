package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/age_of_war_1"
	"github.com/brdgme/brdgme/brdgme-go/cmd"
)

func main() {
	cmd.Cli(&age_of_war_1.Game{}, os.Stdin, os.Stdout)
}
