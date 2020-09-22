package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/love_letter"
)

func main() {
	cmd.Cli(&love_letter.Game{}, os.Stdin, os.Stdout)
}
