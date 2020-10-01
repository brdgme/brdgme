package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/splendor"
)

func main() {
	cmd.Cli(&splendor.Game{}, os.Stdin, os.Stdout)
}
