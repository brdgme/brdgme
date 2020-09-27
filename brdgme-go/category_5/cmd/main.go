package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/category_5"
	"github.com/brdgme/brdgme/brdgme-go/cmd"
)

func main() {
	cmd.Cli(&category_5.Game{}, os.Stdin, os.Stdout)
}
