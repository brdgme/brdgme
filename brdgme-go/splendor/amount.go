package splendor

import "github.com/brdgme/brdgme/brdgme-go/libcost"

func CanAfford(a, c libcost.Cost) bool {
	short := 0
	for g, n := range c {
		if a[g] < n {
			short += n - a[g]
		}
	}
	return a[Gold]-c[Gold] >= short
}
