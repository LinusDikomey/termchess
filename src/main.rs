#![feature(hash_drain_filter, iter_intersperse)]

use std::{io::Write, error::Error, sync::{mpsc::{Receiver, self, TryRecvError}, Arc}, thread::{self, JoinHandle}, time::Duration};
use board::Board;
use color_format::cprintln;
use console::{Term, Key};
use piece::{Color, Piece};
use online::{Move, Remote};
use vecm::{vec::PolyVec2, vec2};

use crate::game::{Game, GameEnd};

mod ai;
mod board;
mod game;
mod moves;
mod piece;
mod online;

type Pos = PolyVec2<i8>;

enum PlayerType {
    Me,
    Remote(Remote),
    Cpu {
        depth: usize,
        computation: Option<JoinHandle<ai::Move>>,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let mut server = false;
    let mut fen = None;
    let mut ip = None;
    let mut ai = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-s" | "--server" => server = true,
            "-f" | "--fen" => fen = Some(args.next().expect("fen expected after -f/--fen")),
            "-c" | "--connect" => ip = Some(args.next().expect("connect requires ip")),
            "-a" | "--ai" => ai = Some(
                args.next()
                    .expect("give ai depth as argument")
                    .parse::<usize>()
                    .expect("depth has to be a positive integer")
                ),
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
            match online::run_server(board, color) {
                Ok(()) => println!("Server ended"),
                Err(err) => {
                    println!("Server failed: {err}");
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

        let (the_game, white, black) = if let Some(ip) = ip {
            println!("Connecting to ip: {ip}");
            let (remote, game_info) = online::connect(&ip, name.clone())?;

            
            let mut white_name = name;
            let mut black_name = game_info.other_player;
            if game_info.is_black {
                std::mem::swap(&mut white_name, &mut black_name);
            }

            let mut game = Game::new(vec2![0, 0], white_name, black_name, board, color);
            game.flip_board = game_info.is_black;

            let me = if let Some(depth) = ai {
                PlayerType::Cpu { depth, computation: None }
            } else {
                PlayerType::Me
            };

            if game_info.is_black {
                (game, PlayerType::Remote(remote), me)
            } else {
                (game, me, PlayerType::Remote(remote))
            }
        
        } else if let Some(depth) = ai { 
            let game = Game::new(vec2![0, 0], name.clone(), format!("Computer ({depth})"), board, color);
            (game, PlayerType::Me, PlayerType::Cpu { depth, computation: None })
        } else {
            let game = Game::new(vec2![0, 0], name.clone(), name, board, color);
            (game, PlayerType::Me, PlayerType::Me)
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

        let render = move |game: &Game, term: &Term| -> Result<(), Box<dyn Error>> {
            use std::fmt::Write;

            let y_offset = 2;
            
            for i in 0..y_offset {
                term.move_cursor_to(0, i)?;
                term.clear_line()?;
            }


            let mut s = String::new();
            write!(&mut s, "{game}")?;

            let mut max_line = 0;

            for (i, line) in s.lines().enumerate() {
                let y = i + 2;
                term.move_cursor_to(1, y)?;
                print!("{}", line);
                max_line = y;
            }

            for y in (max_line + 1)..(term.size().1 as usize) {
                term.move_cursor_to(0, y)?;
                term.clear_line()?;
            }

            
            std::io::stdout().flush()?;
            Ok(())
        };

        render(&the_game, &term)?;

        game(render, &term, keys, the_game, white, black)
    }
}

fn game(
    mut render: impl FnMut(&Game, &Term) -> Result<(), Box<dyn Error>>,
    term: &Term,
    keys: Receiver<Key>,
    mut game: Game,
    mut white: PlayerType,
    mut black: PlayerType,
) -> Result<(), Box<dyn Error>> {
    fn render_end(mut render: impl FnMut(&Game, &Term) -> Result<(), Box<dyn Error>>, game: Game, term: &Term, end: GameEnd)
    -> Result<(), Box<dyn Error>> {
        render(&game, term)?;
        match end {
            GameEnd::Winner(Color::Black) => cprintln!("\n\n{} #g<won> as Black!", game.black.name),
            GameEnd::Winner(Color::White) => cprintln!("\n\n{} #g<won> as White!", game.white.name),
            GameEnd::Draw => cprintln!("Game ended in a #rgb(127,127,127)<draw>!")
        }

        Ok(())
    }

    let mut last_term_size = term.size();

    fn play(game: &mut Game, from: Pos, to: Pos, white: &mut PlayerType, black: &mut PlayerType) -> Result<Option<GameEnd>, Box<dyn Error>> {
        if !game.possible_moves.get(&from).map_or(false, |moves| moves.contains(&to)) {
            panic!("{:?} played illegal move: {from} -> {to}", game.turn);
        }
        let other_player = if game.turn == Color::White { black } else { white };
        if let PlayerType::Remote(remote) = other_player {
            online::send(&mut remote.socket, Move { x1: from.x, y1: from.y, x2: to.x, y2: to.y })?;

        }
        Ok(game.play_move(from, to))
    }

    loop {
        let term_size = term.size();
        
        if term_size != last_term_size {
            last_term_size = term_size;
            term.clear_screen()?;
            render(&game, term)?;
        }

        let active_player = if game.turn == Color::White { &mut white } else { &mut black };

        let key = match active_player {
            PlayerType::Me => keys.recv().unwrap(),
            PlayerType::Remote(remote) => {
                match remote.server.try_recv() {
                    Ok(m) => {
                        if let Some(end) = play(&mut game, vec2![m.x1, m.y1], vec2![m.x2, m.y2], &mut white, &mut black)? {
                            render_end(render, game, term, end)?;
                            return Ok(());
                        } else {
                            render(&game, term)?;
                            continue;
                        }
                    }
                    Err(TryRecvError::Empty) => match keys.try_recv() {
                        Ok(t) => t,
                        Err(TryRecvError::Empty) => {
                            std::thread::sleep(Duration::from_millis(10));
                            continue;
                        }
                        Err(TryRecvError::Disconnected) => panic!("Keys disconnected")
                    }
                    Err(TryRecvError::Disconnected) => {
                        eprintln!("Server disconnected");
                        return Ok(());
                    }
                }
            }
            PlayerType::Cpu { depth, computation } => {
                if let Some(available_computation) = computation {
                    if available_computation.is_finished() {
                        let mov = computation.take().unwrap().join().expect("AI compute thread failed");
                        if let Some(end) = play(&mut game, mov.from, mov.to, &mut white, &mut black)? {
                            render_end(render, game, term, end)?;
                            return Ok(());
                        } else {
                            render(&game, term)?;
                            continue;
                        }
                    }
                } else {
                    *computation = Some(ai::movalyzer(&game.board, game.turn, *depth));
                }
                match keys.try_recv() {
                    Ok(t) => t,
                    Err(TryRecvError::Empty) => {
                        std::thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                    Err(err) => panic!("Keys disconnected {err}")
                }
            }
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
                if !matches!(active_player, PlayerType::Me) {
                    game.moving = None;
                    continue;
                }
                if let Some(moving) = game.moving {
                    let cursor = game.cursor;
                    if game.possible_moves.get(&moving).unwrap().contains(&cursor) {
                        if let Some(end) = play(&mut game, moving, cursor, &mut white, &mut black)? {
                            render_end(render, game, term, end)?;
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

        render(&game, term)?;
    }
}