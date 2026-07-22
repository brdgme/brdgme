use brdgme_color::NamedColor;
use brdgme_game::Renderer;
use brdgme_markup::Node as N;

use crate::{BOARD_SIZE, Board, Cell, Loc, PlayerState, PubState};

fn render_board(board: &Board) -> Vec<N> {
    let mut nodes = vec![];
    for (row, cells) in board.iter().enumerate() {
        for (col, cell) in cells.iter().enumerate() {
            nodes.push(match cell {
                Cell::Empty => N::Fg(
                    NamedColor::Blue.into(),
                    vec![N::text(Loc { row, col }.to_string())],
                ),
                Cell::X => N::Bold(vec![N::text("x")]),
                Cell::O => N::Bold(vec![N::text("o")]),
            });
            if col + 1 < BOARD_SIZE {
                nodes.push(N::Fg(NamedColor::Grey.into(), vec![N::text("|")]));
            }
        }
        if row + 1 < BOARD_SIZE {
            nodes.push(N::text("\n"));
        }
    }
    nodes
}

fn render_with_labels(board: &Board, start_player: usize) -> Vec<N> {
    let mut nodes = render_board(board);
    nodes.push(N::text("\n"));
    let x_player = start_player;
    let o_player = 1 - start_player;
    nodes.push(N::Player(x_player));
    nodes.push(N::text(" is X, "));
    nodes.push(N::Player(o_player));
    nodes.push(N::text(" is O"));
    nodes
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        render_with_labels(&self.board, self.start_player)
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        render_with_labels(&self.public.board, self.public.start_player)
    }
}
