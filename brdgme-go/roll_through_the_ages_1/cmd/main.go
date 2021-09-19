package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/roll_through_the_ages_1"
)

func main() {
	cmd.Cli(&roll_through_the_ages_1.Game{}, os.Stdin, os.Stdout)
}
