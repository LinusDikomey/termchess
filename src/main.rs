#![feature(hash_drain_filter, iter_intersperse)]

use std::{fmt, io::Write, collections::{HashSet, HashMap}, error::Error, net::TcpStream, sync::{mpsc::{Receiver, self, TryRecvError}, Arc}, thread, time::Duration};
use binverse::error::BinverseError;
use board::Board;
use color_format::{cwrite, cprintln, cformat};
use console::{Term, Key};
use piece::{Color, Piece};
use server::Move;
use vecm::{vec::PolyVec2, vec2};

use crate::server::send;

mod board;
mod moves;
mod piece;
mod server;

type Pos = PolyVec2<i8>;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let mut server = false;
    let mut fen = None;
    let mut ip = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-s" | "--server" => server = true,
            "-f" | "--fen" => fen = Some(args.next().expect("fen expected after -f/--fen")),
            "-c" | "--connect" => ip = Some(args.next().expect("connect requires ip")),
            _ => eprintln!("unrecognized arg {arg}")
        }
    }
    let (board, color) = if let Some(fen) = fen {
        Board::from_fen(&fen).expect("invalid FEN provided as argument")
    } else {
        (Board::starting_position(), Color::White)
    };  
    if server {
        loop {
            match server::game(board, color) {
                Ok(()) => println!("Played a game!"),
                Err(err) => {
                    println!("Game failed: {err}");
                    std::thread::sleep(Duration::from_millis(500));
                }
            }
        }
    } else {
        print!("Enter Name: ");
        std::io::stdout().flush()?;
        let mut name = String::new();
        std::io::stdin().read_line(&mut name)?;
        name = name.trim().to_owned();

        let (the_game, remote) = if let Some(ip) = ip {
            println!("Connecting to ip: {ip}");
            let mut server = TcpStream::connect(ip)?;
            server::send(&mut server, server::PlayerInfo { name: name.clone() })?;
            let game_info: server::GameInfo = server::recv(&mut server)?;

            let mut white_name = name;
            let mut black_name = game_info.other_player;
            if game_info.is_black {
                std::mem::swap(&mut white_name, &mut black_name);
            }

            let mut game = Game::new(vec2![0, 0], white_name, black_name, board, color);
            game.flip_board = game_info.is_black;

            let (tx, rx) = mpsc::channel();

            let server2 = server.try_clone()?;
            thread::spawn(move || {
                let mut server = server2;
                loop {
                    match server::recv(&mut server) {
                        Ok(move_) => match tx.send(move_) {
                            Ok(_) => {}
                            Err(_) => break
                        }
                        Err(BinverseError::IO(io)) if io.kind() == std::io::ErrorKind::UnexpectedEof => {
                            break
                        }
                        Err(err) => {
                            println!("Server disconnected {err:?}");
                            break
                        }
                    }
                }
            });

            let remote = Remote {
                server: rx,
                color: if game_info.is_black { Color::Black } else { Color::White },
                socket: server,
            };
            (game, Some(remote))
        
        } else {
            let game = Game::new(vec2![0, 0], name.clone(), "Computer".to_owned(), board, color);
            (game, None)
        };

 
        cprintln!("  ~~~  #b<CHESS>   ~~~\n");
 
        let term = Arc::new(Term::stdout());
        term.hide_cursor()?;
        term.clear_screen()?;

        let (keys_tx, keys) = mpsc::channel();
        {
            let term = term.clone();
            thread::spawn(move || {
                loop {
                    if let Err(_) = keys_tx.send(term.read_key().unwrap()) {
                        break;
                    }
                }
            });
        }

        let render = move |game: &Game| -> Result<(), Box<dyn Error>> {
            use std::fmt::Write;
            
            //term.clear_screen()?;

            let y_offset = 2;
            
            for i in 0..y_offset {
                term.move_cursor_to(0, i)?;
                term.clear_line()?;
            }


            let mut s = String::new();
            write!(&mut s, "{game}")?;

            for (i, line) in s.lines().enumerate() {
                term.move_cursor_to(1, i + 2)?;
                print!("{}", line);
            }
            
            std::io::stdout().flush()?;
            Ok(())
        };

        render(&the_game)?;

        game(render, keys, the_game, remote)
    }
}

pub struct Game {
    board: Board,
    turn: Color,
    cursor: Pos,
    moving: Option<Pos>,
    possible_moves: HashMap<Pos, HashSet<Pos>>,
    white: Player,
    black: Player,
    flip_board: bool,
}
impl Game {
    fn new(cursor: Pos, white_name: String, black_name: String, board: Board, turn: Color) -> Self {
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

struct Remote {
    socket: TcpStream,
    server: Receiver<Move>,
    color: Color,
}

fn game(
    mut render: impl FnMut(&Game) -> Result<(), Box<dyn Error>>, 
    keys: Receiver<Key>,
    mut game: Game,
    mut remote: Option<Remote>
) -> Result<(), Box<dyn Error>> {
    fn render_end(mut render: impl FnMut(&Game) -> Result<(), Box<dyn Error>>, game: Game, end: GameEnd)
    -> Result<(), Box<dyn Error>> {
        render(&game)?;
        match end {
            GameEnd::Winner(Color::Black) => cprintln!("\n\n{} #g<won> as Black!", game.black.name),
            GameEnd::Winner(Color::White) => cprintln!("\n\n{} #g<won> as White!", game.white.name),
            GameEnd::Draw => cprintln!("Game ended in a #rgb(127,127,127)<draw>!")
        }

        Ok(())
    }

    loop {
        let key = if let Some(remote) = &remote {
            match remote.server.try_recv() {
                Ok(m) => {
                    if game.turn == remote.color {
                        println!("Remote sent move while it was your turn!");
                        return Ok(());
                    }
                    let end = game.play_move(vec2![m.x1, m.y1], vec2![m.x2, m.y2]);
                    if let Some(end) = end {
                        render_end(render, game, end)?;
                        return Ok(());
                    } else {
                        render(&game)?;
                        continue;
                    }
                }
                Err(TryRecvError::Empty) => match keys.try_recv() {
                    Ok(t) => t,
                    Err(TryRecvError::Empty) => {
                        std::thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                    Err(err) => panic!("Keys disconnected {err}")
                }
                Err(TryRecvError::Disconnected) => {
                    println!("Server disconnected!");
                    return Ok(())
                }
            }

        } else {
            keys.recv()?
        };

        let up = |game: &mut Game| {
            if game.cursor.y < 7 {
                game.cursor.y += 1;
            }
        };
        let down = |game: &mut Game| {
            if game.cursor.y > 0 {
                game.cursor.y -= 1;
            }
        };

        match key {
            Key::Char('m') | Key::ArrowLeft => if game.cursor.x > 0 { game.cursor.x -= 1; },
            Key::Char('i') | Key::ArrowRight => if game.cursor.x < 7 { game.cursor.x += 1; },
            Key::Char('e') | Key::ArrowUp => if game.flip_board { down(&mut game) } else { up(&mut game) }
            Key::Char('n') | Key::ArrowDown => if game.flip_board { up(&mut game) } else { down(&mut game) }
            Key::Char(' ') | Key::Char('\n') => {
                if let Some(remote) = &remote {
                    if game.turn != remote.color {
                        game.moving = None;
                        continue;
                    }
                }
                if let Some(moving) = game.moving {
                    let cursor = game.cursor;
                    if game.possible_moves.get(&moving).unwrap().contains(&cursor) {
                        if let Some(remote) = &mut remote {
                            send(&mut remote.socket, Move { x1: moving.x, y1: moving.y, x2: cursor.x, y2: cursor.y })?;
                        }
                        let end = game.play_move(moving, cursor);
                        if let Some(end) = end {
                            render_end(render, game, end)?;
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
            Key::Escape => {
                game.moving = None;
            }
            Key::PageUp => {} // history
            Key::PageDown => {} // history
            Key::Char(_) => {}
            _ => {}
        }

        render(&game)?;
    }
}