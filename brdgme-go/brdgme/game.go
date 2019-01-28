package brdgme

// CommandResponse is data relating to a successful command.
type CommandResponse struct {
	Logs      []Log
	CanUndo   bool
	Remaining string
}

type Status struct {
	Active   *StatusActive   `json:",omitempty"`
	Finished *StatusFinished `json:",omitempty"`
}

type StatusActive struct {
	WhoseTurn  []int `json:"whose_turn"`
	Eliminated []int `json:"eliminated"`
}

func (sa StatusActive) ToStatus() Status {
	return Status{
		Active: &sa,
	}
}

type StatusFinished struct {
	Placings []int         `json:"placings"`
	Stats    []interface{} `json:"stats"`
}

func (sf StatusFinished) ToStatus() Status {
	return Status{
		Finished: &sf,
	}
}

// Gamer is a playable game.
type Gamer interface {
	New(players int) ([]Log, error)
	PubState() interface{}
	PlayerState(player int) interface{}
	Command(
		player int,
		input string,
		players []string,
	) (CommandResponse, error)
	Status() Status
	CommandSpec(player int) *Spec
	PlayerCount() int
	PlayerCounts() []int
	PubRender() string
	PlayerRender(player int) string
	Points() []float32
}
