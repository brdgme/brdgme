package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/for_sale_1"
)

func main() {
	cmd.Cli(&for_sale_1.Game{}, os.Stdin, os.Stdout)
}
