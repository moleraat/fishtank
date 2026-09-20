use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

// Whole env
struct Game {
    boards: (Board, Board, Vec<Board>),
    players: HashMap<PlayerId, Player>,
    teams: (Team, Team),
}

//
struct Board {
    clock: Clock,
    white_player: PlayerId,
    black_player: PlayerId,
    grid: Grid,
}

struct Clock {
    white_time: Duration,
    black_time: Duration,
}

struct Team {
    roster: (PlayerId, PlayerId, Vec<PlayerId>),
}

//
struct Player {
    id: PlayerId, // todo: prob something typed
    name: String,
    reserve: Vec<Piece>,
}

struct Grid {
    grid: [[Cell; 8]; 8],
}

struct Cell {
    piece: Option<Piece>,
}

// Standard chess pieces
enum Piece {
    Pawn(Color),
    Bishop(Color),
    Knight(Color),
    Rook(Color),
    Queen(Color),
    King(Color),
}

// Chess side
enum Color {
    White,
    Black,
}

#[derive(Hash, Eq, PartialEq)]
struct PlayerId {
    id: Uuid,
}
