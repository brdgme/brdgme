package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/love_letter_1"
)

func main() {
	cmd.Cli(&love_letter_1.Game{}, os.Stdin, os.Stdout)
}
