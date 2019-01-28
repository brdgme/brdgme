package brdgme

// A Log is a game log entry, either public or private for specific players.
type Log struct {
	Public  bool
	Message string
	To      []int
}

// NewPublicLog creates a new publicly visible log entry.
func NewPublicLog(message string) Log {
	return Log{
		Public:  true,
		Message: message,
	}
}

// NewPrivateLog creates a new private log entry for specific players.
func NewPrivateLog(message string, to []int) Log {
	return Log{
		Public:  false,
		Message: message,
		To:      to,
	}
}
