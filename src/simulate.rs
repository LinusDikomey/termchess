use crate::{board::Board, Pos, piece::Color};


pub fn game() {
    let s = "
White played [3, 1] -> [3, 3]
Black played [3, 6] -> [3, 4]
White played [6, 0] -> [5, 2]
Black played [2, 7] -> [4, 5]
White played [1, 0] -> [2, 2]
Black played [5, 6] -> [5, 4]
White played [5, 2] -> [4, 4]
Black played [5, 4] -> [5, 3]
White played [4, 1] -> [4, 3]
Black played [5, 3] -> [4, 2] and took Pawn
White played [5, 1] -> [4, 2] and took Pawn
Black played [6, 7] -> [5, 5]
White played [5, 0] -> [1, 4]
Black played [2, 6] -> [2, 5]
White played [4, 4] -> [2, 5] and took Pawn
Black played [1, 6] -> [2, 5] and took Knight
White played [4, 2] -> [4, 3]
Black played [2, 5] -> [1, 4] and took Bishop
White played [4, 3] -> [3, 4] and took Pawn
Black played [4, 5] -> [6, 3]
White played [2, 2] -> [4, 1]
Black played [3, 7] -> [0, 4]
White played [2, 0] -> [3, 1]
";
    let mut board = Board::starting_position();

    let moves: Vec<_> = s.lines().filter(|l| !l.trim().is_empty()).map(|l| {
        let x1 = &l[14..15];
        let y1 = &l[17..18];

        let x2 = &l[24..25];
        let y2 = &l[27..28];

        let from = Pos::new(x1.parse().unwrap(), y1.parse().unwrap());
        let to = Pos::new(x2.parse().unwrap(), y2.parse().unwrap());
        (from, to)
    }).collect();
    
    let mut turn = Color::White;

    for (i, (from, to)) in moves.into_iter().enumerate() {
        println!("Simulating {from} -> {to} (#{i})");
        board.move_piece(from, to);
        let (_, a) = board.moves(turn);
        println!("{a}");
        turn = !turn;
    }

    std::process::exit(0);
}