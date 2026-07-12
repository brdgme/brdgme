//! Port of `render.go`/`board.go`'s rendering logic. Both `PubState` and
//! `PlayerState` render through the same `player_render`/`render_board`
//! machinery, matching Go's `PubRender() = PlayerRender(0)` (verbatim, not a
//! separate neutral layout) and `PlayerRender` reading directly off `Game`
//! (no hidden information to redact for either state).

use std::collections::HashMap;

use brdgme_color::GREY;
use brdgme_game::Renderer;
use brdgme_markup::ast::Col;
use brdgme_markup::{Align, Node as N, Row, table_with_gap};

use crate::loc::{self, DIR_DOWN, DIR_LEFT, DIR_RIGHT, DIR_UP, Dir, Loc};
use crate::piece::{self, Piece};
use crate::tile::{NO_PLAYER, Tile};
use crate::{PlayerState, PubState, opponent};

const TILE_WIDTH: usize = 6;
const TILE_HEIGHT: usize = 3;
/// Port of `emptyAbove` (`render.go`): `(TileHeight-1)/2`. `emptyBelow`
/// (`TileHeight/2`) is not carried as a separate constant - see
/// `render_empty_tile`'s comment for why it always contributes zero visible
/// content for `TileHeight == 3`.
const EMPTY_ABOVE: usize = (TILE_HEIGHT - 1) / 2;

const NO_TILE_STR: &str = " ";
const PIECE_BACKGROUND: &str = " ";

/// Port of `Tiler` (`game.go`): anything that can answer "what tile (if any)
/// occupies this location", shared by both the board and a standalone piece
/// (used for the remaining-tiles catalogue render).
trait Tiler {
    fn tile_at(&self, loc: Loc) -> Option<Tile>;
}

impl Tiler for HashMap<String, Tile> {
    fn tile_at(&self, loc: Loc) -> Option<Tile> {
        // Guard against off-board locations before converting to a key:
        // `Loc::to_key` assumes `0..=9` and panics on overflow for negative
        // `y` (Go's equivalent `Loc.String()` has no such check, but an
        // out-of-range key there simply fails to match any real board entry
        // - harmless - so this guard preserves that behaviour without the
        // panic).
        if !loc.valid() {
            return None;
        }
        self.get(&loc.to_key()).cloned()
    }
}

/// Port of `Piece.TileAt` (`piece.go`).
impl Tiler for Piece {
    fn tile_at(&self, loc: Loc) -> Option<Tile> {
        for (i, &l) in self.positions.iter().enumerate() {
            if l == loc {
                let mut t = Tile {
                    player: self.player_type.player,
                    typ: self.player_type.typ,
                    owner: NO_PLAYER,
                    text: String::new(),
                };
                if i == 0 {
                    t.text = self.player_type.typ.to_string();
                }
                return Some(t);
            }
        }
        None
    }
}

/// Port of `WallStrs` (`render.go`): every wall/corner character is
/// determined purely by whether the direction combination has a vertical
/// component (`Up`/`Down`), a horizontal component (`Left`/`Right`), or
/// both - this is equivalent to Go's 15-entry explicit map (every nonzero
/// 4-bit combination falls into exactly one of these three cases).
fn wall_char(dir: Dir) -> &'static str {
    let vertical = dir & (DIR_UP | DIR_DOWN) != 0;
    let horizontal = dir & (DIR_LEFT | DIR_RIGHT) != 0;
    match (vertical, horizontal) {
        (true, true) => "+",
        (true, false) => "|",
        (false, true) => "-",
        (false, false) => panic!("wall_char: empty dir"),
    }
}

/// Port of `SideWall` (`render.go`): a `TileHeight`-tall, 1-char-wide
/// vertical wall column with no trailing newline.
fn side_wall_text() -> String {
    [wall_char(DIR_UP | DIR_DOWN); TILE_HEIGHT].join("\n")
}

fn is_open(open: &HashMap<Dir, bool>, d: Dir) -> bool {
    *open.get(&d).unwrap_or(&false)
}

/// Port of `OpenSides` (`game.go`): whether the neighbour in each of the 8
/// directions is the *same* `PlayerType`, in which case the shared wall
/// renders as blank piece-background instead, visually merging same-piece
/// tiles into one shape.
fn open_sides<T: Tiler>(src: &T, loc: Loc) -> HashMap<Dir, bool> {
    let mut open = HashMap::new();
    let t = match src.tile_at(loc) {
        Some(t) => t,
        None => return open,
    };
    for d in loc::dirs() {
        if let Some(nt) = src.tile_at(loc.neighbour(d))
            && t.player == nt.player
            && t.typ == nt.typ
        {
            open.insert(d, true);
        }
    }
    open
}

/// Port of `RenderCorner` (`render.go`).
fn render_corner(dir: Dir, open: &HashMap<Dir, bool>) -> &'static str {
    let all_dirs = loc::dirs();

    // If all three contributing directions are open, render nothing (fully
    // merges the corner into the piece background).
    let mut num_open = 0;
    for &d in &all_dirs {
        if dir & d == d && is_open(open, d) {
            num_open += 1;
            if num_open == 3 {
                return PIECE_BACKGROUND;
            }
        }
    }

    // Map the two orthogonal directions contributing to this corner to each
    // other (mirrors Go's `cornerMap`, which stops after finding the second
    // match and so never records the diagonal direction itself).
    let mut corner_map: HashMap<Dir, Dir> = HashMap::new();
    let mut first: Option<Dir> = None;
    for &d in &all_dirs {
        if dir & d != d {
            continue;
        }
        match first {
            None => first = Some(d),
            Some(f) => {
                corner_map.insert(f, d);
                corner_map.insert(d, f);
                break;
            }
        }
    }

    let mut corner: Dir = 0;
    for (&d, &other) in &corner_map {
        if is_open(open, d) {
            corner |= d;
        } else {
            corner |= loc::dir_inv(other);
        }
    }
    wall_char(corner)
}

/// Port of `render.Align(render.Center, ...)`'s centering split as used by
/// `RenderPlayerTile`/`RenderEmptyTile`: `before = diff/2` (floor),
/// `after = diff - before` (ceil).
fn center_pad(text: &str, width: usize) -> String {
    let len = text.chars().count();
    let diff = width.saturating_sub(len);
    let before = diff / 2;
    let after = diff - before;
    format!("{}{}{}", " ".repeat(before), text, " ".repeat(after))
}

fn player_col(player: i32) -> Col {
    Col::from(player as usize)
}

/// Port of `RenderPlayerTile` (`render.go`).
fn render_player_tile(tile: &Tile, open: &HashMap<Dir, bool>) -> Vec<N> {
    let mut s = String::new();

    // Top row.
    s.push_str(render_corner(DIR_UP | DIR_LEFT, open));
    let top_c = if is_open(open, DIR_UP) {
        PIECE_BACKGROUND
    } else {
        wall_char(DIR_LEFT | DIR_RIGHT)
    };
    s.push_str(&top_c.repeat(TILE_WIDTH - 2));
    s.push_str(render_corner(DIR_UP | DIR_RIGHT, open));
    s.push('\n');

    // Middle rows.
    let left = if is_open(open, DIR_LEFT) {
        PIECE_BACKGROUND
    } else {
        wall_char(DIR_UP | DIR_DOWN)
    };
    let right = if is_open(open, DIR_RIGHT) {
        PIECE_BACKGROUND
    } else {
        wall_char(DIR_UP | DIR_DOWN)
    };
    let middle_row = format!(
        "{}{}{}\n",
        left,
        center_pad(&tile.text, TILE_WIDTH - 2),
        right
    );
    for _ in 0..(TILE_HEIGHT - 2) {
        s.push_str(&middle_row);
    }

    // Bottom row.
    s.push_str(render_corner(DIR_DOWN | DIR_LEFT, open));
    let bottom_c = if is_open(open, DIR_DOWN) {
        PIECE_BACKGROUND
    } else {
        wall_char(DIR_LEFT | DIR_RIGHT)
    };
    s.push_str(&bottom_c.repeat(TILE_WIDTH - 2));
    s.push_str(render_corner(DIR_DOWN | DIR_RIGHT, open));

    let fg = player_col(tile.player).mono().inv();
    let bg = player_col(tile.player);
    vec![N::Bold(vec![N::Fg(fg, vec![N::Bg(bg, vec![N::text(s)])])])]
}

/// Port of `RenderTile` (`render.go`).
fn render_tile<T: Tiler>(src: &T, loc: Loc) -> Option<Vec<N>> {
    let t = src.tile_at(loc)?;
    if t.player == NO_PLAYER {
        return None;
    }
    Some(render_player_tile(&t, &open_sides(src, loc)))
}

/// Port of `RenderEmptyTile` (`render.go`). Note `emptyBelow` contributes no
/// visible output for `TileHeight == 3`: Go's
/// `strings.TrimSpace(strings.Repeat(blankLine, emptyBelow))` trims a
/// single all-whitespace blank line down to `""`, and the unconditional
/// trailing `\n` written just before it (via `buf.WriteByte('\n')`) is what
/// actually produces the tile's third (empty) line when the block is later
/// split on `\n` - so the "bottom" section is omitted here rather than
/// literally ported as an always-empty blank-line block.
fn render_empty_tile(loc: Loc, owner: i32) -> Vec<N> {
    let blank_line = format!("{}\n", NO_TILE_STR.repeat(TILE_WIDTH));
    let top = blank_line.repeat(EMPTY_ABOVE);
    let s = loc.to_key();
    let remaining_width = TILE_WIDTH - s.chars().count();
    let before = remaining_width / 2;
    let after = remaining_width - before;
    let label = if owner == NO_PLAYER {
        N::Bold(vec![N::Fg(GREY.into(), vec![N::text(s)])])
    } else {
        N::Bold(vec![N::Fg(player_col(owner), vec![N::text(s)])])
    };
    vec![N::Fg(
        GREY.into(),
        vec![
            N::text(format!("{}{}", top, " ".repeat(before))),
            label,
            N::text(format!("{}\n", " ".repeat(after))),
        ],
    )]
}

/// Port of `Board.Render` (`board.go`): a zero-spacing `render.Table` of
/// bold box-drawing borders wrapping one 6x3-multi-line-cell body row per
/// board row.
fn render_board(board: &HashMap<String, Tile>) -> Vec<N> {
    let mut rows: Vec<Row> = vec![];

    let mut header: Row = vec![];
    header.push((
        Align::Left,
        vec![N::Bold(vec![N::text(wall_char(DIR_DOWN | DIR_RIGHT))])],
    ));
    for _ in 0..10 {
        header.push((
            Align::Left,
            vec![N::Bold(vec![N::text(
                wall_char(DIR_LEFT | DIR_RIGHT).repeat(TILE_WIDTH),
            )])],
        ));
    }
    header.push((
        Align::Left,
        vec![N::Bold(vec![N::text(wall_char(DIR_DOWN | DIR_LEFT))])],
    ));
    rows.push(header);

    for y in 0..10 {
        let mut row: Row = vec![];
        row.push((Align::Left, vec![N::Bold(vec![N::text(side_wall_text())])]));
        for x in 0..10 {
            let l = Loc::new(x, y);
            let cell_nodes = match render_tile(board, l) {
                Some(nodes) => nodes,
                None => {
                    let owner = board.get(&l.to_key()).map(|t| t.owner).unwrap_or(NO_PLAYER);
                    render_empty_tile(l, owner)
                }
            };
            row.push((Align::Left, cell_nodes));
        }
        row.push((Align::Left, vec![N::Bold(vec![N::text(side_wall_text())])]));
        rows.push(row);
    }

    let mut footer: Row = vec![];
    footer.push((
        Align::Left,
        vec![N::Bold(vec![N::text(wall_char(DIR_UP | DIR_RIGHT))])],
    ));
    for _ in 0..10 {
        footer.push((
            Align::Left,
            vec![N::Bold(vec![N::text(
                wall_char(DIR_LEFT | DIR_RIGHT).repeat(TILE_WIDTH),
            )])],
        ));
    }
    footer.push((
        Align::Left,
        vec![N::Bold(vec![N::text(wall_char(DIR_UP | DIR_LEFT))])],
    ));
    rows.push(footer);

    vec![N::Table(rows)]
}

/// Port of `Piece.Render` (`piece.go`): a zero-spacing `render.Table` over
/// the piece's bounding box, blank cells where the box has no piece cell.
fn render_piece(p: &Piece) -> Vec<N> {
    let (lower, upper) = p.bounds();
    let mut rows: Vec<Row> = vec![];
    for y in lower.y..=upper.y {
        let mut row: Row = vec![];
        for x in lower.x..=upper.x {
            let cell = render_tile(p, Loc::new(x, y)).unwrap_or_default();
            row.push((Align::Left, cell));
        }
        rows.push(row);
    }
    vec![N::Table(rows)]
}

/// Port of `Game.RenderPlayerRemainingTiles` (`render.go`): every unplayed
/// piece of `p_num`, in index order, packed into rows wrapped at a running
/// width of 10 (`colSpacing=2` needs the manual spacer cells that
/// `table_with_gap` inserts). Each wrapped row is emitted as its own table
/// (mirroring Go's per-flush `render.Table` calls, each prefixed by a
/// literal `"\n"`), not as multiple rows of one shared table.
fn render_player_remaining_tiles(state: &PubState, p_num: usize) -> Vec<N> {
    let all_pieces = piece::pieces(p_num as i32);
    let mut nodes: Vec<N> = vec![];
    let mut current_cells: Row = vec![];
    let mut cur_width: i32 = 0;
    let mut has_tiles = false;

    for (i, p) in all_pieces.iter().enumerate() {
        if state.played_pieces[p_num][i] {
            continue;
        }
        has_tiles = true;
        let p_width = p.width();
        if cur_width + p_width > 10 {
            nodes.push(N::text("\n"));
            nodes.push(table_with_gap(&[current_cells.clone()], 2));
            current_cells = vec![];
            cur_width = 0;
        }
        current_cells.push((Align::Left, render_piece(p)));
        cur_width += p_width;
    }

    if !has_tiles {
        return vec![N::Bold(vec![N::Fg(GREY.into(), vec![N::text("None")])])];
    }

    nodes.push(N::text("\n"));
    nodes.push(table_with_gap(&[current_cells], 2));
    nodes
}

/// Port of `Game.PlayerRender` (`render.go`): board, instructional line,
/// then this player's then the opponent's remaining-tiles catalogues (both
/// always shown - there is no hidden information to redact).
fn player_render(state: &PubState, p_num: usize) -> Vec<N> {
    let mut nodes: Vec<N> = vec![];
    nodes.extend(render_board(&state.board));
    nodes.push(N::text("\n\nAll pieces are shown in their "));
    nodes.push(N::Bold(vec![N::text("down")]));
    nodes.push(N::text(" position and pivot around the number."));

    let opp = opponent(p_num);
    nodes.push(N::Bold(vec![
        N::text("\n\n"),
        N::Player(p_num),
        N::text(" remaining tiles:\n"),
    ]));
    nodes.extend(render_player_remaining_tiles(state, p_num));
    nodes.push(N::Bold(vec![
        N::text("\n\n"),
        N::Player(opp),
        N::text(" remaining tiles:\n"),
    ]));
    nodes.extend(render_player_remaining_tiles(state, opp));
    nodes
}

/// Port of `Game.PubRender` (`render.go`): literally `PlayerRender(0)`.
fn pub_render(state: &PubState) -> Vec<N> {
    player_render(state, 0)
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        pub_render(self)
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        player_render(&self.public, self.player)
    }
}
