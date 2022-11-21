#![feature(hash_drain_filter)]

use std::{fmt, io::Write, collections::{HashSet, HashMap}, error::Error, ops::Not};
use board::Board;
use color_format::{cwrite, cprintln, cformat};
use vecm::{vec::PolyVec2, vec2};

mod board;
mod moves;

type Pos = PolyVec2<i8>;

pub fn inside(pos: Pos) -> bool {
    pos.x >= 0 && pos.y >= 0 && pos.x <= 7 && pos.y <= 7
}

pub struct Player {
    name: String,
    taken_pieces: Vec<Piece>,
}
impl Player {
    fn new(name: String) -> Self {
        Self {
            name,
            taken_pieces: vec![],
        }
    }
}

enum GameEnd {
    Draw,
    Winner(Color),
}

pub struct Game {
    board: Board,
    turn: Color,
    cursor: Pos,
    moving: Option<Pos>,
    possible_moves: HashMap<Pos, HashSet<Pos>>,
    white: Player,
    black: Player,
}
impl Game {
    fn new(cursor: Pos, white_name: String, black_name: String) -> Self {
        let mut board = Self {
            board: Board::starting_position(),
            turn: Color::White,
            cursor,
            possible_moves: HashMap::new(),
            moving: None,
            white: Player::new(white_name),
            black: Player::new(black_name),
        };
        
        board.compute_moves();

        board
    }

    // optionally returns the winner
    fn compute_moves(&mut self) -> Option<GameEnd> {
        let (possible, count) = self.board.moves(self.turn);
        if count == 0 {
            self.possible_moves.clear();
            let king_pos = self.board.find_king(self.turn).expect("king not found");
            let end = if self.board.moves(!self.turn).0.iter().any(|(_, moves)| moves.contains(&king_pos)) {
                GameEnd::Winner(!self.turn)
            } else {
                GameEnd::Draw
            };
            return Some(end)
        }
        self.possible_moves = possible;
        None
    }

    fn play_move(&mut self, from: Pos, to: Pos) -> Option<GameEnd> {
        let taken = self.board[to];
        if let Some((piece, color)) = taken {
            assert_ne!(color, self.turn, "player took own piece");
            if self.turn == Color::White {
                self.white.taken_pieces.push(piece);
            } else {
                self.black.taken_pieces.push(piece);
            }
        }
        self.board.move_piece(from, to);
        self.turn = if self.turn == Color::White { Color::Black } else { Color::White };
        self.compute_moves()
    }

    fn after_text(&self, f: &mut fmt::Formatter<'_>, y: i32) -> fmt::Result {
        cwrite!(f, "    ")?;
        match y {
            0 => cwrite!(f, "#bg:rgb(255,255,255);rgb(0,0,0)<{}>", self.white.name)?,
            1 => {
                for piece in &self.white.taken_pieces {
                    cwrite!(f, "{}", piece.character(Color::Black))?;
                }
            }
            6 => {
                for piece in &self.black.taken_pieces {
                    cwrite!(f, "{}", piece.character(Color::White))?;
                }
            }
            7 => cwrite!(f, "#bg:rgb(0,0,0)<{}>", self.black.name)?,
            _ => {}
        }
        Ok(())
    }
}
impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        cwrite!(f, "#bg:rgb(102,51,0);black<## >")?;
        for file in 0..8 {
            cwrite!(f, "#bg:rgb(102,51,0);g<{} >", ('a' as u8 + file) as char)?;
        }
        cwrite!(f, "#bg:rgb(102,51,0)<  >")?;
        self.after_text(f, -1)?;
        writeln!(f)?;

        let mut bg_white = true;
        for (rank, row) in self.board.iter().enumerate().rev() {
            cwrite!(f, "#bg:rgb(102,51,0);g<{} >", rank + 1)?;
            for (file, piece) in row.into_iter().enumerate() {
                let on_cursor = self.cursor.x == file as i8 && self.cursor.y == rank as i8;
                let moving = self.moving.unwrap_or(self.cursor);
                let extra = if self.possible_moves.get(&moving).map_or(false, |s| s.contains(&Pos::new(file as i8, rank as i8))) {
                    if on_cursor {
                        cformat!("#b<##>")
                    } else {
                        cformat!("#m<##>")
                    }
                   
                } else if on_cursor {
                    if self.moving.is_some() {
                        cformat!("#g<<>")
                    } else {
                        cformat!("#r<<>")
                    }
                } else { " ".to_owned() };

                let p = if let Some((piece, color)) = piece {
                    piece.character(color)
                } else {
                    // doesn't matter which color spaces have
                    String::from(" ")
                };
                match bg_white {
                    // color used twice here because it is reset by inner string
                    true => {
                        cwrite!(f, "#bg:rgb(238,238,238)<{}>", p)?;
                        cwrite!(f, "#bg:rgb(238,238,238)<{}>", extra)?;
                    }
                    false => {
                        cwrite!(f, "#bg:rgb(118,150,86)<{}>", p)?;
                        cwrite!(f, "#bg:rgb(118,150,86)<{}>", extra)?;
                    }
                }
                bg_white = !bg_white;
            }
            bg_white = !bg_white;
            cwrite!(f, "#bg:rgb(102,51,0);g<  >")?;
            self.after_text(f, rank as i32)?;
            self.after_text(f, 8)?;
            writeln!(f)?;
        }
        cwrite!(f, "#bg:rgb(102,51,0)<{}>", " ".repeat(2*8+4))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Piece {
    King,
    Queen,
    Bishop,
    Knight,
    Rook,
    Pawn,
}
impl Piece {
    fn character(self, color: Color) -> String {
        let c = match (self, Color::Black) {
            (Piece::King, Color::Black) => '♚',
            (Piece::King, Color::White) => '♔',
            (Piece::Queen, Color::Black) => '♛',
            (Piece::Queen, Color::White) => '♕',
            (Piece::Bishop, Color::Black) => '♝',
            (Piece::Bishop, Color::White) => '♗',
            (Piece::Knight, Color::Black) => '♞',
            (Piece::Knight, Color::White) => '♘',
            (Piece::Rook, Color::Black) => '♜',
            (Piece::Rook, Color::White) => '♖',
            (Piece::Pawn, Color::Black) => '♟',
            (Piece::Pawn, Color::White) => '♙',
        };
        match color {
            Color::White => cformat!("#rgb(180,180,180)<{}>", c),
            Color::Black => cformat!("#rgb(86,83,82)<{}>", c),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Color {
    Black,
    White,
}
impl Not for Color {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Self::Black => Self::White,
            Self::White => Self::Black,
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let term = console::Term::stdout();
    loop {
        game(&term)?;
    }
}

fn game(term: & console::Term) -> Result<(), Box<dyn Error>> {
    print!("Enter Name: ");
    std::io::stdout().flush()?;
    let name = term.read_line()?;

    let mut game = Game::new(vec2![0, 0], name, "Computer".to_owned());
    term.hide_cursor()?;
    cprintln!("  ~~~  #b<CHESS>   ~~~\n");

    print!("{game}");
    std::io::stdout().flush()?;

    let render = |game: &Game| -> Result<(), Box<dyn Error>>{
        term.move_cursor_up(9)?;
        term.move_cursor_left(100)?;
        print!("{game}");
        std::io::stdout().flush()?;
        Ok(())
    };

    loop {
        
        let key = term.read_key()?;
        
        match key {
            console::Key::Char('m') | console::Key::ArrowLeft => if game.cursor.x > 0 { game.cursor.x -= 1; },
            console::Key::Char('i') | console::Key::ArrowRight => if game.cursor.x < 7 { game.cursor.x += 1; },
            console::Key::Char('e') | console::Key::ArrowUp => if game.cursor.y < 7 { game.cursor.y += 1; },
            console::Key::Char('n') | console::Key::ArrowDown => if game.cursor.y > 0 { game.cursor.y -= 1; },
            console::Key::Char(' ') | console::Key::Enter => {
                if let Some(moving) = game.moving {
                    let cursor = game.cursor;
                    if game.possible_moves.get(&moving).unwrap().contains(&cursor) {
                        let end = game.play_move(moving, cursor);
                        if let Some(end) = end {
                            render(&game)?;
                            match end {
                                GameEnd::Winner(Color::Black) => cprintln!("\n\n{} #g<won> as Black!", game.black.name),
                                GameEnd::Winner(Color::White) => cprintln!("\n\n{} #g<won> as White!", game.white.name),
                                GameEnd::Draw => cprintln!("Game ended in a #rgb(127,127,127)<draw>!")
                            }
                            return Ok(());
                        }
                    }
                    game.moving = None;
                } else {
                    match game.board[game.cursor] {
                        Some((_, color)) if color == game.turn => {
                            game.moving = Some(game.cursor);
                        }
                        _ => {}
                    }
                }
            }
            console::Key::Escape => {
                game.moving = None;
            }
            console::Key::PageUp => {} // history
            console::Key::PageDown => {} // history
            console::Key::Char(_) => {}
            _ => {}
        }

        render(&game)?;
    }
}