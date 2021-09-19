package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/splendor_1"
)

func main() {
	cmd.Cli(&splendor_1.Game{}, os.Stdin, os.Stdout)
}
