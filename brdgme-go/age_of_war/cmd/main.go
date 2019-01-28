package main

import (
	"os"

	"github.com/brdgme-go/age_of_war"
	"github.com/brdgme-go/cmd"
)

func main() {
	cmd.Cli(&age_of_war.Game{}, os.Stdin, os.Stdout)
}
