use std::{error::Error, net::{TcpListener, IpAddr}, io::{Read, Write}, time::Duration};

use binverse::{streams::{Serializer, Deserializer}, serialize::{Serialize, Deserialize}, error::BinverseError};
use binverse_derive::serializable;
use vecm::vec2;

use crate::{board::Board, Color, GameEnd};


#[serializable]
pub struct PlayerInfo {
    pub name: String,
}

#[serializable]
pub struct Move {
    pub x1: i8,
    pub y1: i8,
    pub x2: i8,
    pub y2: i8,
}

#[serializable]
pub struct GameInfo {
    pub other_player: String,
    pub is_black: bool,
}

pub fn send<T: Serialize<W>, W: Write>(p: W, t: T) -> Result<(), BinverseError> {
    let mut s = Serializer::new_no_revision(p);
    t.serialize(&mut s)
}
pub fn recv<T: Deserialize<R>, R: Read>(p: R) -> Result<T, BinverseError> {
    Deserializer::new_no_revision(p, 0).deserialize()
}

pub fn game(mut board: Board, mut turn: Color) -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind((IpAddr::from([127, 0, 0, 1]), 1337))?;

    let (mut p1, _) = listener.accept()?;
    let p1_info: PlayerInfo = recv(&mut p1)?;
    println!("Player 1: {} connected", p1_info.name);

    let (mut p2, _) = listener.accept()?;
    let p2_info: PlayerInfo = recv(&mut p2)?;
    println!("Player 2: {} connected", p2_info.name);

    send(&mut p1, GameInfo { other_player: p2_info.name, is_black: false })?;
    send(&mut p2, GameInfo { other_player: p1_info.name, is_black: true })?;

    loop {
        let mover = if turn == Color::White { &mut p1 } else { &mut p2 };
        let played_move: Move = Deserializer::new_no_revision(mover, 0).deserialize()?;

        let from = vec2![played_move.x1, played_move.y1];
        let to = vec2![played_move.x2, played_move.y2];
        match board.move_piece(from, to) {
            Some(taken) => println!("{:?} played {} -> {} and took {:?}", turn, from, to, taken),
            None => println!("{:?} played {} -> {}", turn, from, to),
        }
        turn = !turn;
        
        let (_, count) = board.moves(turn);
        println!("{count} moves found");
        let game_end = if count == 0 {
            let king_pos = board.find_king(turn).expect("king not found");
            println!("King found at {king_pos}");
            if board.moves(!turn).0.iter().any(|(_, moves)| moves.contains(&king_pos)) {
                Some(GameEnd::Winner(!turn))
            } else {
                Some(GameEnd::Draw)
            }
        } else { None };

        println!("serializing move");
        let other = if turn == Color::White { &mut p1 } else { &mut p2 };
        let mut s = Serializer::new_no_revision(other);
        played_move.serialize(&mut s)?;

        if let Some(end) = game_end {
            match end {
                GameEnd::Draw => println!("Game ended in a draw!"),
                GameEnd::Winner(Color::White) => println!("White won the game!"),
                GameEnd::Winner(Color::Black) => println!("Black won the game!"),
            }
            std::thread::sleep(Duration::from_millis(100));
            break Ok(());
        }
    }
}