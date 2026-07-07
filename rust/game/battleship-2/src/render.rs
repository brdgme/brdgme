use brdgme_color as color;
use brdgme_game::Renderer;
use brdgme_markup::{Align as A, Node as N, Row};

use crate::{BOARD_SIZE, Board, Cell, Phase, PlayerState, PubState, Ship, other_player};

fn render_cell(cell: Cell, x: usize, y: usize) -> N {
    let bg = if (x + y).is_multiple_of(2) {
        color::CYAN
    } else {
        color::BLUE
    };
    let content = match cell {
        Cell::Empty => N::text("  "),
        Cell::Carrier | Cell::Battleship | Cell::Cruiser | Cell::Submarine | Cell::Destroyer => {
            N::Bg(color::GREY.into(), vec![N::text("  ")])
        }
        Cell::Hit => N::Bg(
            color::RED.into(),
            vec![N::Fg(
                color::YELLOW.into(),
                vec![N::Bold(vec![N::text("XX")])],
            )],
        ),
        Cell::Miss => N::Fg(color::GREY.into(), vec![N::Bold(vec![N::text("XX")])]),
    };
    N::Bg(bg.into(), vec![content])
}

fn render_board(board: &Board) -> N {
    let mut rows: Vec<Row> = vec![];
    let mut header: Row = vec![(A::Center, vec![N::text(" ")])];
    for x in 1..=BOARD_SIZE {
        header.push((A::Center, vec![N::text(format!("{}", x))]));
    }
    header.push((A::Center, vec![N::text(" ")]));
    rows.push(header.clone());
    for (y, row) in board.iter().enumerate().take(BOARD_SIZE) {
        let letter = (b'A' + y as u8) as char;
        let mut row_cells: Row =
            vec![(A::Center, vec![N::Bold(vec![N::text(letter.to_string())])])];
        for (x, &cell) in row.iter().enumerate().take(BOARD_SIZE) {
            row_cells.push((A::Center, vec![render_cell(cell, x, y)]));
        }
        row_cells.push((A::Center, vec![N::Bold(vec![N::text(letter.to_string())])]));
        rows.push(row_cells);
    }
    rows.push(header);
    N::Table(rows)
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
