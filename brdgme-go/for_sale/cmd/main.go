package main

import (
	"os"

	"github.com/brdgme/brdgme/brdgme-go/cmd"
	"github.com/brdgme/brdgme/brdgme-go/for_sale"
)

func main() {
	cmd.Cli(&for_sale.Game{}, os.Stdin, os.Stdout)
}
