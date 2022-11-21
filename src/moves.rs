use std::collections::HashSet;
use vecm::vec2;

use crate::{Piece, Pos, Color, board::Board};

pub fn moves(game: &Board, piece: Piece, pos: Pos, color: Color) -> HashSet<Pos> {
    #[derive(PartialEq, Eq)]
    enum Ty { No, Enemy, Ally }
    let occupied = |p: Pos| -> Ty {
        match game[p] {
            Some((_, c)) => if c == color { Ty::Ally } else { Ty::Enemy } 
            None => Ty::No
        }
    };

    let mut moves = HashSet::new();

    let dir_moves = |moves: &mut HashSet<Pos>, dir: Pos| {
        let mut cur = pos;
        loop {
            cur += Pos::from(dir);
            if !inside(cur) { break }
            match occupied(cur) {
                Ty::No => {
                    moves.insert(cur);
                }
                Ty::Enemy => {
                    moves.insert(cur);
                    break;
                }
                Ty::Ally => break
            }
            moves.insert(cur);
        }
    };

    let rook = |moves: &mut HashSet<Pos>| {
        for dir in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            dir_moves(moves, Pos::from(dir));
        }
    };
    let bishop = |moves: &mut HashSet<Pos>| {
        for dir in [(-1, -1), (-1, 1), (1, -1), (1, 1)] {
            dir_moves(moves, Pos::from(dir));
        }
    };
    match piece {
        Piece::King => {
            for y in 0.max(pos.y-1) ..= 7.min(pos.y+1) {
                for x in (pos.x-1).max(0) ..= (pos.x+1).min(7) {
                    let cur = vec2![x, y];
                    if occupied(cur) != Ty::Ally {
                        moves.insert(cur);
                    }
                }
            }

            let castle = game.can_castle(color);
            let y = if color == Color::Black { 7 } else { 0 };
            // performance optimization possible here by not recalculating all moves

            if 
                castle.long
                && (1..4).all(|x| occupied(vec2![x, y]) == Ty::No)
                && (2..=4).all(|x| !game.threatens(vec2![x, y], !color))
            {
                moves.insert(vec2![2, y]);
            }
            if
                castle.short
                && (5..7).all(|x| occupied(vec2![x, y]) == Ty::No)
                && (4..=6).all(|x| !game.threatens(vec2![x, y], !color))
            {
                moves.insert(vec2![6, y]);
            }
        }
        Piece::Queen => {
            rook(&mut moves);
            bishop(&mut moves);
        }
        Piece::Bishop => bishop(&mut moves),
        Piece::Knight => {
            let offsets = [(-2, 1), (-1, 2), (1, 2), (2, 1), (2, -1), (1, -2), (-1, -2), (-2, -1)];
            for o in offsets {
                let pos = pos + Pos::from(o);
                if inside(pos) && occupied(pos) != Ty::Ally {
                    moves.insert(pos);
                }
            }
        }
        Piece::Rook => rook(&mut moves),
        Piece::Pawn => {
            let d = if color == Color::White {
                if pos.y == 7 { return moves }
                1
            } else {
                if pos.y == 0 { return moves }
                -1
            };
            let (l_en_passant, r_en_passant) = if let Some(moved_pawn) = game.moved_pawn() {
                (moved_pawn == pos - vec2![1, 0], moved_pawn == pos + vec2![1, 0])
            } else { (false, false) };

            let l = pos + vec2![-1, d];
            if l_en_passant || inside(l) && occupied(l) == Ty::Enemy {
                moves.insert(l);
            }
            let r = pos + vec2![1, d];
            if r_en_passant || inside(r) && occupied(r) == Ty::Enemy {
                moves.insert(r);
            }
            let m = pos + vec2![0, d];
            if game[m].is_none() {
                moves.insert(m);
                let m2 = pos + vec2![0, 2*d];
                if matches!((color, pos.y), (Color::White, 1) | (Color::Black, 6)) && game[m2].is_none() {
                    moves.insert(m2);
                }
            }
        }
    }
    moves
}

fn inside(pos: Pos) -> bool {
    pos.x >= 0 && pos.y >= 0 && pos.x <= 7 && pos.y <= 7
}
