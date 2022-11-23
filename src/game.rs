use std::{fmt, collections::{HashMap, HashSet}};

use color_format::{cwrite, cformat};

use crate::{piece::{Piece, Color}, Pos, board::Board};


pub struct Game {
    pub board: Board,
    pub turn: Color,
    pub cursor: Pos,
    pub moving: Option<Pos>,
    pub possible_moves: HashMap<Pos, HashSet<Pos>>,
    pub white: Player,
    pub black: Player,
    pub flip_board: bool,
}
impl Game {
    pub fn new(cursor: Pos, white_name: String, black_name: String, board: Board, turn: Color) -> Self {
        let mut board = Self {
            board,
            turn,
            cursor,
            possible_moves: HashMap::new(),
            moving: None,
            white: Player::new(white_name),
            black: Player::new(black_name),
            flip_board: false,
        };
        
        board.compute_moves();

        board
    }

    // optionally returns the winner
    pub fn compute_moves(&mut self) -> Option<GameEnd> {
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

    pub fn play_move(&mut self, from: Pos, to: Pos) -> Option<GameEnd> {
        let taken = self.board.move_piece(from, to);
        if let Some(piece) = taken {
            if self.turn == Color::White {
                self.white.taken_pieces.push(piece);
            } else {
                self.black.taken_pieces.push(piece);
            }
        }
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
            cwrite!(f, "#bg:rgb(102,51,0);g<{} >", (b'a' + file) as char)?;
        }
        cwrite!(f, "#bg:rgb(102,51,0)<  >")?;
        self.after_text(f, -1)?;
        writeln!(f)?;

        let mut bg_white = true;
        for i in 0usize..8 {

            let rank = if self.flip_board { i } else { 7-i };
            let row = self.board.iter().nth(rank).unwrap();

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

pub struct Player {
    pub name: String,
    pub taken_pieces: Vec<Piece>,
}
impl Player {
    fn new(name: String) -> Self {
        Self {
            name,
            taken_pieces: vec![],
        }
    }
}

pub enum GameEnd {
    Draw,
    Winner(Color),
}