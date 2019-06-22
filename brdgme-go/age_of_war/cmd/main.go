package main

import (
	"os"

	"github.com/brdgme/brdgme-go/age_of_war"
	"github.com/brdgme/brdgme-go/cmd"
)

func main() {
	cmd.Cli(&age_of_war.Game{}, os.Stdin, os.Stdout)
}
