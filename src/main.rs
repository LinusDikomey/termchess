#![feature(hash_drain_filter, iter_intersperse)]

use std::{io::Write, error::Error, net::TcpStream, sync::{mpsc::{Receiver, self, TryRecvError}, Arc}, thread, time::Duration};
use binverse::error::BinverseError;
use board::Board;
use color_format::cprintln;
use console::{Term, Key};
use piece::{Color, Piece};
use server::Move;
use vecm::{vec::PolyVec2, vec2};

use crate::{server::send, game::{Game, GameEnd}};

mod board;
mod game;
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
                    if keys_tx.send(term.read_key().unwrap()).is_err() {
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
                    println!("\n\nServer disconnected!");
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