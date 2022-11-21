use color_format::cformat;


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
    pub fn character(self, color: Color) -> String {
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
impl std::ops::Not for Color {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Self::Black => Self::White,
            Self::White => Self::Black,
        }
    }
}