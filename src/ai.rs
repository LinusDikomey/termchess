use std::thread::{JoinHandle, self};

use vecm::vec2;

use crate::{board::Board, Pos, piece::{Color, Piece}};

type Score = i32;

pub struct Move {
    pub from: Pos,
    pub to: Pos,
}

pub fn movalyzer(board: &Board, turn: Color, depth: usize) -> JoinHandle<Move> {
    let board = *board;

    thread::spawn(move || {
        find_best(&board, turn, depth, 0).0.unwrap()
    })
}

fn find_best(board: &Board, turn: Color, depth: usize, level: usize) -> (Option<Move>, Score) {
    let (all_moves, count) = board.moves(turn);
    let mut new_board;

    if level == 0 {
        //eprintln!("Checking {} moves", count);
    }

    if count == 0 {
        let king = board.find_king(turn).expect("ai lost the king");
        if board.threatens(king, !turn, false) {
            return (None, -100_000);
        } else {
            return (None, 0);
        }
    }

    let mut best_move = (Move { from: Pos::zero(), to: Pos::zero() }, Score::MIN);

    let mut _checked_count = 0;

    for (from, to) in all_moves {
        for to in to {
            new_board = *board;
            new_board.move_piece(from, to);
            let score = if depth == 0 {
                eval(board, turn)
            } else {
                let (_, enemy_score) = find_best(&new_board, !turn, depth-1, level + 1);
                -enemy_score
            };
            if score > best_move.1 {
                best_move = (Move { from, to }, score);
            }
            _checked_count += 1;
            if level == 0 {
                //eprintln!("Checked {}/{}", checked_count, count);
            }
        }
    }
    (Some(best_move.0), best_move.1)
}

fn eval(board: &Board, turn: Color) -> i32 {
    let mut score = 0;
    for (y, row) in board.iter().enumerate() {
        for (x, piece) in row.iter().enumerate() {
            if let Some((piece, color)) = *piece {
                let mut piece_score = piece_score(piece, vec2![x as _, y as _], color);
                if color != turn {
                    piece_score *= -1;
                }
                score += piece_score;
            }
            
        }
    }
    score
}
fn piece_score(piece: Piece, pos: Pos, color: Color) -> i32 {
    match piece {
        Piece::King => {
            //let progress = if color == Color::White { pos.y } else { 7 - pos.y };
            //progress as i32 * 10000
            0
        }
        Piece::Queen => 9000,
        Piece::Bishop => 3000,
        Piece::Knight => 3000,
        Piece::Rook => 5000,
        Piece::Pawn => {
            let progress = if color == Color::White { pos.y } else { 7 - pos.y };
            1000 + progress as i32 * 114
        }
    }
}