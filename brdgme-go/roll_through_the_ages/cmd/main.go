package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/roll_through_the_ages"
)

func main() {
	cmd.Cli(&roll_through_the_ages.Game{}, os.Stdin, os.Stdout)
}
