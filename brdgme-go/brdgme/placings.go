package brdgme

import (
	"sort"
	"strconv"
	"strings"
)

type placOrd int8

const (
	placOrdLess    placOrd = -1
	placOrdEqual   placOrd = 0
	placOrdGreater placOrd = 1
)

func cmpMetrics(a, b []int) placOrd {
	aLen, bLen := len(a), len(b)
	if aLen == 0 && bLen == 0 {
		return placOrdEqual
	}
	if aLen == 0 {
		return placOrdLess
	}
	if bLen == 0 {
		return placOrdGreater
	}
	if a[0] < b[0] {
		return placOrdLess
	}
	if a[0] > b[0] {
		return placOrdGreater
	}
	return cmpMetrics(a[1:], b[1:])
}

type placMetrics [][]int

func (s placMetrics) Len() int      { return len(s) }
func (s placMetrics) Swap(i, j int) { s[i], s[j] = s[j], s[i] }
func (s placMetrics) Less(i, j int) bool {
	return cmpMetrics(s[i], s[j]) == placOrdLess
}

var _ sort.Interface = placMetrics{}

func placMetricKey(metric []int) string {
	metricStrs := make([]string, len(metric))
	for k, m := range metric {
		metricStrs[k] = strconv.Itoa(m)
	}
	return strings.Join(metricStrs, ",")
}

func GenPlacings(metrics [][]int) []int {
	grouped := map[string][]int{}
	byMetricKey := map[string][]int{}

	for p, m := range metrics {
		mKey := placMetricKey(m)
		grouped[mKey] = append(grouped[mKey], p)
		byMetricKey[mKey] = m
	}

	uniqueMetrics := [][]int{}
	for _, m := range byMetricKey {
		uniqueMetrics = append(uniqueMetrics, m)
	}

	sort.Sort(sort.Reverse(placMetrics(uniqueMetrics)))

	placingMap := map[int]int{}
	curPlace := 1
	for _, m := range uniqueMetrics {
		mKey := placMetricKey(m)
		for _, p := range grouped[mKey] {
			placingMap[p] = curPlace
		}
		curPlace++
	}

	placings := make([]int, len(metrics))
	for p := range metrics { // nolint
		placings[p] = placingMap[p]
	}

	return placings
}
