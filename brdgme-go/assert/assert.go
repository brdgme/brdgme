package assert

import "testing"

func Equal(t *testing.T, expected, actual interface{}, msgAndArgs ...interface{}) bool {
	panic("not implemented")
}

func NoError(t *testing.T, err error, msgAndArgs ...interface{}) bool {
	panic("not implemented")
}

func Nil(t *testing.T, object interface{}, msgAndArgs ...interface{}) bool {
	panic("not implemented")
}

func True(t *testing.T, value bool, msgAndArgs ...interface{}) bool {
	panic("not implemented")
}

func False(t *testing.T, value bool, msgAndArgs ...interface{}) bool {
	panic("not implemented")
}
