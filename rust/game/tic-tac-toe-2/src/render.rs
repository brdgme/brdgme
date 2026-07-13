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

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        render_board(&self.board)
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        render_board(&self.public.board)
    }
}
