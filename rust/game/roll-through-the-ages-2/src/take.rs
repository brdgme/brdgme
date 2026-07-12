//! Port of the `TakeAction` type from `brdgme-go/roll_through_the_ages_1/take_command.go`.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum TakeAction {
    Food,
    Workers,
}
