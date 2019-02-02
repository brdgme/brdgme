package main

import (
	"os"

	"brdgme-go/age_of_war"
	"brdgme-go/cmd"
)

func main() {
	cmd.Cli(&age_of_war.Game{}, os.Stdin, os.Stdout)
}
