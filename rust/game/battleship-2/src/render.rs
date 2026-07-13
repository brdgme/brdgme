use brdgme_color::NamedColor;
use brdgme_game::Renderer;
use brdgme_markup::{Align as A, Node as N, Row};

use crate::{BOARD_SIZE, Board, Cell, Phase, PlayerState, PubState, Ship, other_player};

fn render_cell(cell: Cell, x: usize, y: usize) -> N {
    let bg = if (x + y).is_multiple_of(2) {
        NamedColor::Cyan
    } else {
        NamedColor::Blue
    };
    let content = match cell {
        Cell::Empty => N::text("  "),
        Cell::Carrier | Cell::Battleship | Cell::Cruiser | Cell::Submarine | Cell::Destroyer => {
            N::Bg(NamedColor::Grey.into(), vec![N::text("  ")])
        }
        Cell::Hit => N::Bg(
            NamedColor::Red.into(),
            vec![N::Fg(
                NamedColor::Yellow.into(),
                vec![N::Bold(vec![N::text("XX")])],
            )],
        ),
        Cell::Miss => N::Fg(NamedColor::Grey.into(), vec![N::Bold(vec![N::text("XX")])]),
    };
    N::Bg(bg.into(), vec![content])
}

/// Matches Go's `"  1 2 3 4 5 6 7 8 9 10"` header/footer line: two leading
/// spaces, then column numbers separated by single spaces, no trailing
/// space.
fn header_footer_line() -> String {
    let mut s = String::from("  ");
    for x in 1..=BOARD_SIZE {
        if x > 1 {
            s.push(' ');
        }
        s.push_str(&x.to_string());
    }
    s
}

fn render_board(board: &Board) -> N {
    let header = header_footer_line();
    let mut nodes: Vec<N> = vec![N::text(header.clone())];
    for (y, row) in board.iter().enumerate().take(BOARD_SIZE) {
        let letter = (b'A' + y as u8) as char;
        let row_cells: Row = row
            .iter()
            .enumerate()
            .take(BOARD_SIZE)
            .map(|(x, &cell)| (A::Center, vec![render_cell(cell, x, y)]))
            .collect();
        nodes.push(N::text("\n"));
        nodes.push(N::Bold(vec![N::text(letter.to_string())]));
        nodes.push(N::text(" "));
        nodes.push(N::Table(vec![row_cells]));
        nodes.push(N::text(format!(" {}", letter)));
    }
    nodes.push(N::text("\n"));
    nodes.push(N::text(header));
    N::Group(nodes)
}

fn render_ships_left(ships: &[Ship]) -> N {
    let mut nodes: Vec<N> = vec![N::Bold(vec![N::text(
        "Ships left to place (ship size in brackets):",
    )])];
    for s in ships {
        nodes.push(N::text(format!("\n{} ({})", s, s.size())));
    }
    N::Group(nodes)
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        let mut out: Vec<N> = vec![];
        for p in 0..self.players {
            out.push(N::Player(p));
            out.push(N::text("\n\n"));
            out.push(render_board(&self.boards[p]));
            out.push(N::text("\n\n"));
        }
        out
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        let mut out: Vec<N> = vec![];
        let op = other_player(self.player);
        if self.public.phase == Phase::Placing {
            if !self.left_to_place.is_empty() {
                out.push(render_ships_left(&self.left_to_place));
            } else {
                out.push(N::Bold(vec![N::text(
                    "Waiting for your opponent to place their ships",
                )]));
            }
            out.push(N::text("\n\n"));
        } else {
            out.push(N::Bold(vec![N::text("Enemy board:\n\n")]));
            out.push(render_board(&self.public.boards[op]));
            out.push(N::text("\n\n"));
        }
        out.push(N::Bold(vec![N::text("Your board:\n\n")]));
        out.push(render_board(&self.board));
        out
    }
}
