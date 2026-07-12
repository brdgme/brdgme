//! Placeholder renderer (Task 1). Full render parity with `render.go`/
//! `board.go` (box-drawing board grid, per-tile wall/corner rendering,
//! remaining-tiles catalogue) is Task 2 of the port plan.

use brdgme_game::Renderer;
use brdgme_markup::Node as N;

use crate::{PlayerState, PubState};

fn render(state: &PubState) -> Vec<N> {
    let mut nodes: Vec<N> = vec![];
    nodes.push(N::Bold(vec![N::text("Cathedral")]));
    nodes.push(N::text(format!(
        "\n\nCurrent player: {}\n",
        state.current_player
    )));
    if state.finished {
        nodes.push(N::text("Game finished.\n"));
    } else if state.no_open_tiles {
        nodes.push(N::text(
            "No open tiles remain; players play simultaneously.\n",
        ));
    }
    for p in 0..state.players {
        let remaining = state.played_pieces[p]
            .iter()
            .filter(|&&played| !played)
            .count();
        nodes.push(N::text(format!(
            "\nPlayer {} unplayed pieces: {}\n",
            p, remaining
        )));
    }
    nodes.push(N::text(format!("\nBoard cells: {}\n", state.board.len())));
    nodes
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        render(self)
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        render(&self.public)
    }
}
