use crate::rules::Position;
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

// Responsible for storing the state of the environment ------------------------
// Calls out to rules to check legality ----------------------------------------

// Whole env
struct Game {
    boards: (Board, Board, Vec<Board>),
    players: HashMap<PlayerId, Player>,
    teams: (Team, Team),
}

//
pub struct Board {
    clock: Clock,
    white_player: PlayerId,
    black_player: PlayerId,
    position: Position,
    // todo: 3-fold detection
}

struct Clock {
    white_time: Duration,
    black_time: Duration,
}

struct Team {
    roster: (PlayerId, PlayerId, Vec<PlayerId>),
}

// todo: refactor
pub type Reserve = Vec<(PieceKind, Color, u8)>;

struct Player {
    id: PlayerId,
    name: String,
    reserve: Reserve,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Square {
    rank: Rank,
    file: File,
}
impl Square {
    pub fn try_new(rank: u8, file: u8) -> Result<Self, &'static str> {
        let rank = Rank::new(rank)?;
        let file = File::new(file)?;

        Ok(Self { rank, file })
    }

    pub const fn new(rank: Rank, file: File) -> Self {
        Self { rank, file }
    }

    pub fn offset(self, offset: Offset) -> Option<Self> {
        let (new_rank, new_file): (i8, i8) = (
            offset.rank.saturating_add_unsigned(self.rank.0),
            offset.file.saturating_add_unsigned(self.file.0),
        );
        let new_rank = u8::try_from(new_rank)
            .map_err(|_x| "move resulted in invalid rank")
            .ok()?;
        let new_file = u8::try_from(new_file)
            .map_err(|_x| "move resulted in invalid file")
            .ok()?;

        Self::try_new(new_rank, new_file).ok()
    }

    pub fn index(self) -> (usize, usize) {
        (usize::from(self.rank.0), usize::from(self.file.0))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Offset {
    rank: i8,
    file: i8,
}
impl Offset {
    pub const fn new(rank: i8, file: i8) -> Self {
        Self { rank, file }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rank(u8);
impl Rank {
    pub const fn new(x: u8) -> Result<Self, &'static str> {
        if x > 7 {
            return Err("x must be 0..=7");
        }

        Ok(Self(x))
    }

    pub const fn lit(x: u8) -> Self {
        assert!(x <= 7, "x must be 0..=7");
        Self(x)
    }

    pub fn index(self) -> usize {
        usize::from(self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct File(u8);
impl File {
    pub const fn new(x: u8) -> Result<Self, &'static str> {
        if x > 7 {
            return Err("x must be 0..=7");
        }

        Ok(Self(x))
    }

    pub const fn lit(x: u8) -> Self {
        assert!(x <= 7, "x must be 0..=7");
        Self(x)
    }

    pub fn index(self) -> usize {
        usize::from(self.0)
    }

    pub fn span(self, other: Self) -> impl Iterator<Item = Self> {
        (self.0.min(other.0)..=self.0.max(other.0)).map(Self)
    }
}

// todo: move somewhere else?
enum OpponentState {
    Chilling,
    Check,
    Checkmate,
    Stalemate,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Piece {
    kind: PieceKind,
    color: Color,
    moved: bool,
    promoted: bool,
}
impl Piece {
    pub const fn new(kind: PieceKind, color: Color, promoted: bool) -> Self {
        Self {
            kind,
            color,
            moved: false,
            promoted,
        }
    }

    pub const fn kind(self) -> PieceKind {
        self.kind
    }

    pub const fn color(self) -> Color {
        self.color
    }

    pub const fn moved(self) -> bool {
        self.moved
    }

    pub const fn set_moved(&mut self) {
        self.moved = true;
    }
}

// Standard chess pieces
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PieceKind {
    Pawn,
    Bishop,
    Knight,
    Rook,
    Queen,
    King,
}
impl PieceKind {
    pub const ALL: [Self; 6] = [
        Self::Pawn,
        Self::Bishop,
        Self::Knight,
        Self::Rook,
        Self::Queen,
        Self::King,
    ];
}

// Chess side
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Color {
    White,
    Black,
}
impl Color {
    pub fn opposite(self) -> Self {
        if self == Self::White {
            return Self::Black;
        }

        Self::White
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
struct PlayerId(Uuid);

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_() {
        println!("Piece size: {}", size_of::<Piece>());
        println!("Player size: {}", size_of::<Player>());
        println!("Clock size: {}", size_of::<Clock>());
        println!("Position size: {}", size_of::<Position>());
        println!("Board size: {}", size_of::<Board>());
        println!("Game size: {}", size_of::<Game>());
    }
}
