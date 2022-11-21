use std::{ops::Index, collections::{HashMap, HashSet}};

use vecm::vec2;

use crate::{Piece, Color, Pos, moves::moves};

#[derive(Clone, Copy)]
pub struct Castle {
    pub short: bool,
    pub long: bool,
}
impl Castle {
    fn new() -> Self {
        Self { short: true, long: true }
    }

    /// (white, black)
    fn from_fen(fen: &str) -> Option<(Self, Self)> {
        let mut white = Castle { short: false, long: false };
        let mut black = Castle { short: false, long: false };

        if fen == "-" { return Some((white, black)) }
        
        for c in fen.chars() {
            match c.to_ascii_lowercase() {
                'k' => black.short = true,
                'q' => black.long = true,
                'K' => white.short = true,
                'Q' => white.long = true,
                _ => return None
            }
        }
        Some((white, black))
    }
}

#[derive(Clone, Copy)]
pub struct Board {
    // rows then files
    board: [[Option<(Piece, Color)>; 8]; 8],
    moved_pawn: Option<Pos>,
    white_castle: Castle,
    black_castle: Castle,
}
impl Index<Pos> for Board {
    type Output = Option<(Piece, Color)>;

    fn index(&self, index: Pos) -> &Self::Output {
        &self.board[index.y as usize][index.x as usize]
    }
}
impl Board {
    pub fn starting_position() -> Self {
        let mut board = [[None; 8]; 8];
        
        for i in 0..8 {
            board[6][i] = Some((Piece::Pawn, Color::Black));
            board[1][i] = Some((Piece::Pawn, Color::White));
        }

        let first_rank = {
            use Piece::*;

            [Rook, Knight, Bishop, Queen, King, Bishop, Knight, Rook]
        };
        for (i, piece) in first_rank.into_iter().enumerate() {
            board[7][i] = Some((piece, Color::Black));
            board[0][i] = Some((piece, Color::White));
        }
        Self {
            board,
            moved_pawn: None,
            white_castle: Castle::new(),
            black_castle: Castle::new()
        }
    }

    pub fn from_fen(fen: &str) -> Option<(Self, Color)> {
        fn piece(c: char) -> Option<Piece> {
            Some(match c {
                'k' => Piece::King,
                'p' => Piece::Pawn,
                'n' => Piece::Knight,
                'b' => Piece::Bishop,
                'r' => Piece::Rook,
                'q' => Piece::Queen,
                _ => return None
            })
        }
        fn pos(s: &str) -> Option<Pos> {
            let a = s.chars().next()?;
            let b = s.chars().next()?;
            if s.chars().next().is_some() || a < 'a' || a > 'h' || b < '1' || b > '8' {
                return None;
            }
            Some(Pos::new((a as u8 - b'a') as i8, (b as u8 - b'1') as i8))
        }

        let mut sections = fen.split(' ');

        let pieces = sections.next()?;

        let mut board = [[None; 8]; 8];

        let mut file = 0;
        let mut rank = 7;

        for c in pieces.chars() {
            match c {
                '/' => {
                    file = 0;
                    rank -= 1;
                    if rank < 0 { return None }
                }
                '0'..='9' => {
                    file += c as u8 - b'0';
                }
                'a'..='z' | 'A'..='Z' => {
                    if file > 7 { return None }
                    board[rank as usize][file as usize] = Some((
                        piece(c.to_ascii_lowercase())?,
                        if c.is_ascii_lowercase() { Color::Black } else { Color::White }
                    ));
                    file += 1;
                }
                _ => return None
            }
        }

        let turn = match sections.next()? {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return None,
        };
        
        let (white_castle, black_castle) = Castle::from_fen(sections.next()?)?;

        let moved_pawn = match sections.next()? {
            "-" => None,
            s => Some(pos(s)? + if turn == Color::White { vec2![0,1] } else { vec2![0, -1] })
        };

        let _halfmoves: u32 = sections.next()?.parse().ok()?;
        let _fullmoves: u32 = sections.next()?.parse().ok()?;

        if sections.next().is_some() { return None }

        Some((
            Self {
                board,
                moved_pawn,
                white_castle,
                black_castle,
            },
            turn
        ))
    }

    pub fn moves(&self, turn: Color) -> (HashMap<Pos, HashSet<Pos>>, usize) {
        let mut all_moves = HashMap::new();

        let mut total_moves = 0;
        for y in 0..8 {
            for x in 0..8 {
                let pos = vec2![x, y];
                if let Some((piece, color)) = self[pos] {
                    if color == turn {
                        let mut piece_moves = moves(self, piece, pos, color);
                        piece_moves.drain_filter(|to_pos| self.in_check_after(pos, *to_pos, turn));
                        total_moves += piece_moves.len();
                        all_moves.insert(pos, piece_moves);
                    }
                }
            }
        }
        let mut f = std::fs::OpenOptions::new().append(true).write(true).create(true).open("debug").unwrap();
        use std::io::Write;
        writeln!(f, "Found {} moves for {:?}", total_moves, turn).unwrap();
        (all_moves, total_moves)
    }

    pub fn move_piece(&mut self, from: Pos, to: Pos) -> Option<Piece> {
        let Some((piece, color)) = self[from] else { panic!("Tried to move nonexistant piece") };
        if piece == Piece::King {
            match color {
                Color::Black => {
                    if to == vec2![2, 7] && self.black_castle.long {
                        self.board[7][3] = self.board[7][0];
                        self.board[7][0] = None;
                    } else if to == vec2![6, 7] && self.black_castle.short {
                        self.board[7][5] = self.board[7][7];
                        self.board[7][7] = None;
                    }
                    self.black_castle.short = false;
                    self.black_castle.long = false;
                }
                Color::White => {
                    if to == vec2![2, 0] && self.white_castle.long {
                        self.board[0][3] = self.board[0][0];
                        self.board[0][0] = None;
                    } else if to == vec2![6, 0] && self.white_castle.short {
                        self.board[0][5] = self.board[0][7];
                        self.board[0][6] = None;
                    }
                    self.white_castle.short = false;
                    self.white_castle.long = false;
                }
            }
        } else if piece == Piece::Rook {
            match (from.x, color) {
                (0, Color::Black) => self.black_castle.long = false,
                (7, Color::Black) => self.black_castle.short = false,
                (0, Color::White) => self.white_castle.long = false,
                (7, Color::White) => self.white_castle.short = false,
                _ => {}
            }
        } else if piece == Piece::Pawn {
            if color == Color::White && to.y == 7 || color == Color::Black && to.y == 0 {
                // TODO: select piece to promote to
                
                self.board[to.y as usize][to.x as usize] = Some((Piece::Queen, color));
                self.board[from.y as usize][from.x as usize] = None;
                self.moved_pawn = None;
                return None;
            } else {
                let y_dir = if color == Color::White { 1 } else { -1 };
                if let Some(moved_pawn) = self.moved_pawn {
                    if to == moved_pawn + vec2![0, y_dir] {
                        let (taken, _) = self.board[moved_pawn.y as usize][moved_pawn.x as usize]
                            .take()
                            .expect("moved pawn internal tracking error");  
                            self.board[to.y as usize][to.x as usize] = self[from];
                            self.board[from.y as usize][from.x as usize] = None;
                            self.moved_pawn = None;
                        return Some(taken);
                    }
                }
            }
        }
        let taken = self[to];
        if let Some((_, taken_color)) = taken {
            assert_ne!(color, taken_color, "Tried to move into own piece");
        }
        self.board[to.y as usize][to.x as usize] = self[from];
        self.board[from.y as usize][from.x as usize] = None;
        self.moved_pawn = (piece == Piece::Pawn).then_some(to);

        taken.map(|(piece, _)| piece)
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = [Option<(Piece, Color)>; 8]> + ExactSizeIterator {
        self.board.into_iter()
    }

    // (long castle, short castle)
    pub fn can_castle(&self, color: Color) -> Castle {
        match color {
            Color::Black => self.black_castle,
            Color::White => self.white_castle,
        }
    }

    pub fn find_king(&self, color: Color) -> Option<Pos> {
        for (y, row) in self.board.iter().enumerate() {
            for (x, piece) in row.iter().enumerate() {
                if let Some((Piece::King, king_color)) = piece {
                    if *king_color == color {
                        return Some(Pos::new(x as i8, y as i8))
                    }
                }
            }
        }
        None
    }
    pub fn in_check_after(&self, from: Pos, to: Pos, color: Color) -> bool {
        assert!(self[from].unwrap().1 == color);

        // board after the move to find checks
        let mut board_copy = *self;
        board_copy.move_piece(from, to);
        
        let king_pos = board_copy.find_king(color).expect("No king found");

        board_copy.threatens(king_pos, !color)
    }

    pub fn threatens(&self, pos: Pos, color: Color) -> bool {
        for y in 0..8 {
            for x in 0..8 {
                let other_pos = vec2![x, y];
                if let Some((other_piece, other_color)) = self[other_pos] {
                    if other_color == color {
                        let moves = moves(self, other_piece, other_pos, other_color);
                        if moves.contains(&pos) {
                            return true;
                        }
                    }
                }
            }
        }
        
        false
    }

    pub fn moved_pawn(&self) -> Option<Pos> {
        self.moved_pawn
    }
}