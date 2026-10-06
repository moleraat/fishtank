use crate::game::{
    Color, File, Offset, Piece,
    PieceKind::{self, Bishop, King, Knight, Pawn, Queen, Rook},
    Rank, Reserve, Square,
};

// Responsible for checking legality of game state -----------------------------
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Move {
    Board {
        start: Square,
        end: Square,
        move_kind: MoveKind,
        promo: Option<PieceKind>,
    },
    Drop {
        piece_kind: PieceKind,
        color: Color,
        end: Square,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MoveKind {
    Quiet,
    Turbo { shadow: Square },
    Capture,
    EnPassant,
    Castle(CastleSide),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Position {
    grid: [[Option<Piece>; 8]; 8],
    castle_origins: CastleOrigins,
    en_passant_shadow: Option<Square>, // one rank behind turbo pawn
}

impl Position {
    pub fn generate_start(index: StartIndex) -> Self {
        // mixed radix decoding from index
        let mut back_rank: [Option<PieceKind>; 8] = [None; 8];

        let mut n = usize::from(index.0);
        // place constrained bishops, prevents illegal start
        let b_0 = n % 4;
        n /= 4;
        let b_1 = n % 4;
        n /= 4;

        back_rank[b_0 * 2] = Some(Bishop);
        back_rank[(b_1 * 2) + 1] = Some(Bishop);

        let mut empty: Vec<File> = (0..=7)
            .filter(|&f| back_rank[f].is_none())
            .map(|f| File::lit(f as u8))
            .collect();
        // place queen
        let q = n % 6;
        n /= 6;
        back_rank[empty[q]] = Some(Queen);
        empty.remove(q);

        // place knights
        let (n_0, n_1) = KNIGHT_START_TABLE[n];
        back_rank[empty[n_0]] = Some(Knight);
        back_rank[empty[n_1]] = Some(Knight);
        empty.remove(n_1);
        empty.remove(n_0);

        // place rook, king, rook
        let [r_0, k, r_1]: [File; 3] = empty
            .try_into()
            .expect("must have 3 indices left for r, k, r");
        back_rank[r_0] = Some(Rook);
        back_rank[k] = Some(King);
        back_rank[r_1] = Some(Rook);

        // fill board
        let back_rank: [PieceKind; 8] = back_rank.map(|p| p.expect("back rank filled"));
        let back = |c| back_rank.map(|p_k| Some(Piece::new(p_k, c, false)));
        let pawns = |c| [Some(Piece::new(Pawn, c, false)); 8];
        let grid = [
            back(Color::White),
            pawns(Color::White),
            [None; 8],
            [None; 8],
            [None; 8],
            [None; 8],
            pawns(Color::Black),
            back(Color::Black),
        ];

        Self {
            grid,
            castle_origins: CastleOrigins {
                a_side_rook: File::lit(r_0 as u8),
                king: File::lit(k as u8),
                h_side_rook: File::lit(r_1 as u8),
            },
            en_passant_shadow: None,
        }
    }

    pub fn legal_board_moves(&self, start: Square) -> Option<Vec<Move>> {
        let piece = self.cell(start)?;
        let pseudo_legal = self.moves_ignoring_check(start, piece);

        let pc_color = piece.color();
        // filter out illegal moves that have king(s) in check
        Some(
            pseudo_legal
                .into_iter()
                .filter(|mv| {
                    let mut next_pos = self.clone();
                    _ = next_pos.apply(*mv);
                    next_pos
                        .kings(pc_color)
                        .iter()
                        .all(|k| !next_pos.is_attacked(*k, pc_color))
                })
                .collect(),
        )
    }

    pub fn legal_drop_moves(&self, reserve: &Reserve, pc_color: Color) -> Vec<Move> {
        let pseudo_legal = reserve
            .iter()
            .flat_map(|(pk, c, _)| self.possible_drops(*pk, *c));

        // filter out illegal moves that have king(s) in check
        pseudo_legal
            .into_iter()
            .filter(|mv| {
                let mut next_pos = self.clone();
                _ = next_pos.apply(*mv);
                next_pos
                    .kings(pc_color)
                    .iter()
                    .all(|k| !next_pos.is_attacked(*k, pc_color))
            })
            .collect()
    }

    pub fn apply(&mut self, mv: Move) -> Option<Piece> {
        self.en_passant_shadow = None;

        match mv {
            Move::Board {
                start,
                end,
                move_kind,
                promo,
            } => {
                #[expect(clippy::expect_used, reason = "should only be passed valid moves")]
                let mut piece = self
                    .take_cell(start)
                    .expect("move start must have some piece in quiet");
                piece.set_moved();

                match move_kind {
                    MoveKind::Quiet => {
                        if let Some(promo) = promo {
                            let promo_piece = Piece::new(promo, piece.color(), true);
                            self.set_cell(end, promo_piece);
                        } else {
                            self.set_cell(end, piece);
                        }

                        None
                    }

                    MoveKind::Turbo { shadow } => {
                        self.set_cell(end, piece);
                        self.en_passant_shadow = Some(shadow);

                        None
                    }

                    MoveKind::Capture => {
                        #[expect(clippy::expect_used, reason = "should only be passed valid moves")]
                        let end_piece =
                            self.take_cell(end).expect("end must have piece in capture");
                        if let Some(promo) = promo {
                            let promo_piece = Piece::new(promo, piece.color(), true);
                            self.set_cell(end, promo_piece);
                        } else {
                            self.set_cell(end, piece);
                        }

                        Some(end_piece)
                    }

                    MoveKind::EnPassant => {
                        let ep_offset = match piece.color() {
                            Color::White => Offset::new(-1, 0),
                            Color::Black => Offset::new(1, 0),
                        };
                        #[expect(clippy::expect_used, reason = "should only be passed valid moves")]
                        let pawn_target = end // end is the shadow
                            .offset(ep_offset)
                            .expect("shadow + offset must have piece in en passant");
                        #[expect(clippy::expect_used, reason = "should only be passed valid moves")]
                        let end_piece = self
                            .take_cell(pawn_target)
                            .expect("move end must have some piece in en passant");
                        self.set_cell(end, piece);

                        Some(end_piece)
                    }

                    MoveKind::Castle(castle_side) => {
                        let back_rank = match piece.color() {
                            Color::White => Rank::lit(0),
                            Color::Black => Rank::lit(7),
                        };
                        let (rook_start, rook_end) = match castle_side {
                            CastleSide::ASide => (
                                Square::new(back_rank, self.castle_origins.a_side_rook),
                                Square::new(back_rank, castle_side.rook_file()),
                            ),
                            CastleSide::HSide => (
                                Square::new(back_rank, self.castle_origins.h_side_rook),
                                Square::new(back_rank, castle_side.rook_file()),
                            ),
                        };
                        #[expect(clippy::expect_used, reason = "should only be passed valid moves")]
                        let mut rook = self
                            .take_cell(rook_start)
                            .expect("rook_start must have rook in castle");
                        rook.set_moved();
                        self.set_cell(end, piece); // king
                        self.set_cell(rook_end, rook); // king

                        None
                    }
                }
            }

            Move::Drop {
                piece_kind,
                color,
                end,
            } => {
                let piece = Piece::new(piece_kind, color, false);
                self.set_cell(end, piece);

                None
            }
        }
    }

    fn moves_ignoring_check(&self, start: Square, piece: Piece) -> Vec<Move> {
        match piece.kind() {
            PieceKind::Pawn => self.pawn_moves(start, piece.color()),
            PieceKind::Knight => self.step_moves(start, piece.color(), &KNIGHT_OFFSETS),
            PieceKind::Bishop => self.slide_moves(start, piece.color(), &BISHOP_SLIDES),
            PieceKind::Rook => self.slide_moves(start, piece.color(), &ROOK_SLIDES),
            PieceKind::Queen => self.slide_moves(start, piece.color(), &QUEEN_SLIDES),
            PieceKind::King => {
                let mut moves = self.step_moves(start, piece.color(), &KING_OFFSETS);
                moves.extend(self.king_castle_moves(start, piece.color()));
                moves
            }
        }
    }

    fn step_moves(&self, start: Square, pc_color: Color, offsets: &[Offset]) -> Vec<Move> {
        let mut move_candidates = Vec::with_capacity(16);
        for offset in offsets {
            // check in bounds
            let Some(end) = start.offset(*offset) else {
                continue;
            };

            let end_piece = self.cell(end);
            // check if square open
            if end_piece.is_none() {
                move_candidates.push(Move::Board {
                    start,
                    end,
                    move_kind: MoveKind::Quiet,
                    promo: None,
                });
            // check if can capture enemy
            } else if end_piece.is_some_and(|e_p| e_p.color() != pc_color) {
                move_candidates.push(Move::Board {
                    start,
                    end,
                    move_kind: MoveKind::Capture,
                    promo: None,
                });
            }
        }

        move_candidates
    }

    fn slide_moves(&self, start: Square, pc_color: Color, dirs: &[Offset]) -> Vec<Move> {
        let mut move_candidates = Vec::with_capacity(32);
        for offset in dirs {
            let mut curr_square = start;

            // apply move until out of bounds or at another piece
            while let Some(end) = curr_square.offset(*offset) {
                let end_piece = self.cell(end);
                // empty
                if end_piece.is_none() {
                    move_candidates.push(Move::Board {
                        start,
                        end,
                        move_kind: MoveKind::Quiet,
                        promo: None,
                    });
                // can take enemy
                } else if end_piece.is_some_and(|e_p| e_p.color() != pc_color) {
                    move_candidates.push(Move::Board {
                        start,
                        end,
                        move_kind: MoveKind::Capture,
                        promo: None,
                    });
                    break;
                // blocked pc_color own piece
                } else {
                    break;
                }

                curr_square = end;
            }
        }

        move_candidates
    }

    fn pawn_moves(&self, start: Square, pc_color: Color) -> Vec<Move> {
        // promotion is inferred and applied after the fact
        let (rank, _) = start.index();
        let mut moves = Vec::with_capacity(5);
        let (rank_delta, spawn_rank) = match pc_color {
            Color::White => (1, 1),
            Color::Black => (-1, 6),
        };

        // forward
        if let Some(one) = start
            .offset(Offset::new(rank_delta, 0))
            .filter(|e| self.cell(*e).is_none())
        {
            moves.extend(Self::possible_promos(start, one, MoveKind::Quiet, pc_color));

            // turbo
            if rank == spawn_rank
                && let Some(two) = one
                    .offset(Offset::new(rank_delta, 0))
                    .filter(|e| self.cell(*e).is_none())
            {
                moves.push(Move::Board {
                    start,
                    end: two,
                    move_kind: MoveKind::Turbo { shadow: one },
                    promo: None,
                });
            }
        }

        // diagonal capture
        for file_delta in [-1, 1] {
            // check in bounds
            let Some(end_shadow) = start.offset(Offset::new(rank_delta, file_delta)) else {
                continue;
            };

            // check diagonal target
            if self.cell(end_shadow).is_some_and(|p| p.color() != pc_color) {
                moves.extend(Self::possible_promos(
                    start,
                    end_shadow,
                    MoveKind::Capture,
                    pc_color,
                ));
            } else if self.en_passant_shadow == Some(end_shadow) {
                moves.push(Move::Board {
                    start,
                    end: end_shadow,
                    move_kind: MoveKind::EnPassant,
                    promo: None,
                });
            }
        }

        moves
    }

    fn king_castle_moves(&self, start: Square, pc_color: Color) -> Vec<Move> {
        // king specific castle moves
        let mut moves = Vec::with_capacity(2);
        let rank = match pc_color {
            Color::White => Rank::lit(0),
            Color::Black => Rank::lit(7),
        };
        let o = &self.castle_origins;
        let king_spawn = Square::new(rank, o.king);

        let a_rook_ok = self
            .cell(Square::new(rank, o.a_side_rook))
            .is_some_and(|p| p.color() == pc_color && p.kind() == PieceKind::Rook && !p.moved());
        let king_ok = king_spawn == start
            && self.cell(king_spawn).is_some_and(|p| {
                p.color() == pc_color && p.kind() == PieceKind::King && !p.moved()
            });
        let h_rook_ok = self
            .cell(Square::new(rank, o.h_side_rook))
            .is_some_and(|p| p.color() == pc_color && p.kind() == PieceKind::Rook && !p.moved());

        let exempt = [o.a_side_rook, o.king];
        if a_rook_ok
            && king_ok
            && self.rank_clear(rank, o.a_side_rook, CastleSide::ASide.rook_file(), exempt)
            && self.rank_clear(rank, o.king, CastleSide::ASide.king_file(), exempt)
            && !self.span_attacked(rank, o.king, CastleSide::ASide.king_file(), pc_color)
        {
            moves.push(Move::Board {
                start,
                end: Square::new(rank, CastleSide::ASide.king_file()),
                move_kind: MoveKind::Castle(CastleSide::ASide),
                promo: None,
            });
        }

        let exempt = [o.king, o.h_side_rook];
        if h_rook_ok
            && king_ok
            && self.rank_clear(rank, o.h_side_rook, CastleSide::HSide.rook_file(), exempt)
            && self.rank_clear(rank, o.king, CastleSide::HSide.king_file(), exempt)
            && !self.span_attacked(rank, o.king, CastleSide::HSide.king_file(), pc_color)
        {
            moves.push(Move::Board {
                start,
                end: Square::new(rank, CastleSide::HSide.king_file()),
                move_kind: MoveKind::Castle(CastleSide::HSide),
                promo: None,
            });
        }

        moves
    }

    fn is_attacked(&self, start: Square, pc_color: Color) -> bool {
        // PAWN
        let rank_delta = match pc_color {
            Color::White => 1,
            Color::Black => -1,
        };
        for file_delta in [-1, 1] {
            // check in bounds
            let Some(end) = start.offset(Offset::new(rank_delta, file_delta)) else {
                continue;
            };

            // check diagonal target
            if self
                .cell(end)
                .is_some_and(|p| p.color() != pc_color && p.kind() == PieceKind::Pawn)
            {
                return true;
            }
        }

        // KNIGHT, KING
        for (piece_offsets, piece_kind) in STEP_ATTACKS {
            for offset in piece_offsets {
                // check in bounds
                let Some(end) = start.offset(*offset) else {
                    continue;
                };

                // check if enemy piece is attacking
                let end_piece = self.cell(end);
                if end_piece.is_some_and(|e_p| e_p.color() != pc_color && e_p.kind() == piece_kind)
                {
                    return true;
                }
            }
        }

        // BISHOP, ROOK, QUEEN
        for (piece_offsets, is_piece) in SLIDE_ATTACKS {
            for offset in piece_offsets {
                let mut curr_square = start;

                // apply move until out of bounds or at another piece
                while let Some(end) = curr_square.offset(*offset) {
                    let end_piece = self.cell(end);

                    // check if enemy piece is attacking
                    if end_piece.is_some_and(|e_p| e_p.color() != pc_color && is_piece(e_p.kind()))
                    {
                        return true;
                    // blocked by own or other piece
                    } else if end_piece.is_some() {
                        break;
                    }

                    curr_square = end;
                }
            }
        }

        false
    }

    fn cell(&self, square: Square) -> Option<Piece> {
        let (rank, file) = square.index();
        #[expect(clippy::indexing_slicing, reason = "Square must be 0..=7")]
        self.grid[rank][file]
    }

    fn take_cell(&mut self, square: Square) -> Option<Piece> {
        let (rank, file) = square.index();
        #[expect(clippy::indexing_slicing, reason = "Square must be 0..=7")]
        let cell = &mut self.grid[rank][file];
        cell.take()
    }

    fn set_cell(&mut self, square: Square, piece: Piece) {
        let (rank, file) = square.index();
        #[expect(clippy::indexing_slicing, reason = "Square must be 0..=7")]
        let cell = &mut self.grid[rank][file];
        *cell = Some(piece);
    }

    fn rank_clear(&self, rank: Rank, from: File, to: File, exempt: [File; 2]) -> bool {
        from.span(to)
            .all(|f| self.cell(Square::new(rank, f)).is_none() || exempt.contains(&f))
    }

    fn span_attacked(&self, rank: Rank, from: File, to: File, pc_color: Color) -> bool {
        from.span(to)
            .any(|f| self.is_attacked(Square::new(rank, f), pc_color))
    }

    // todo: refactor
    fn kings(&self, pc_color: Color) -> Vec<Square> {
        let mut kings = Vec::with_capacity(2);
        for rank in 0..=7u8 {
            for file in 0..=7u8 {
                let square = Square::new(Rank::lit(rank), File::lit(file));
                let piece = self.cell(square);
                if let Some(piece) = piece
                    && piece.color() == pc_color
                    && matches!(piece.kind(), PieceKind::King)
                {
                    kings.push(square);
                }
            }
        }

        assert!(!kings.is_empty(), "Side must have >= 1 King");
        kings
    }

    fn possible_promos(start: Square, end: Square, kind: MoveKind, pc_color: Color) -> Vec<Move> {
        let (opp_back_rank, _) = end.index();
        let mut moves = Vec::with_capacity(8);

        let can_promo = match pc_color {
            Color::White => opp_back_rank == 7,
            Color::Black => opp_back_rank == 0,
        };
        if can_promo {
            for piece in PieceKind::ALL {
                moves.push(Move::Board {
                    start,
                    end,
                    move_kind: kind,
                    promo: Some(piece),
                });
            }
        } else {
            moves.push(Move::Board {
                start,
                end,
                move_kind: kind,
                promo: None,
            });
        }

        moves
    }

    // todo: refactor
    fn possible_drops(&self, piece_kind: PieceKind, color: Color) -> Vec<Move> {
        let (start_rank, end_rank): (u8, u8) = match piece_kind {
            PieceKind::Pawn => (1, 6),
            _ => (0, 7),
        };
        let mut empties = Vec::with_capacity(64);
        for rank in start_rank..=end_rank {
            for file in 0..=7u8 {
                let square = Square::new(Rank::lit(rank), File::lit(file));
                let piece_opt = self.cell(square);
                if piece_opt.is_none() {
                    empties.push(Move::Drop {
                        piece_kind,
                        color,
                        end: square,
                    });
                }
            }
        }

        empties
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CastleSide {
    ASide,
    HSide,
}
impl CastleSide {
    pub const fn king_file(self) -> File {
        match self {
            Self::ASide => File::lit(2),
            Self::HSide => File::lit(6),
        }
    }

    pub const fn rook_file(self) -> File {
        match self {
            Self::ASide => File::lit(3),
            Self::HSide => File::lit(5),
        }
    }
}

// Spawn info for setup and castling
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct CastleOrigins {
    a_side_rook: File,
    king: File,
    h_side_rook: File,
}

const STEP_ATTACKS: [(&[Offset], PieceKind); 2] = [
    (&KING_OFFSETS, PieceKind::King),
    (&KNIGHT_OFFSETS, PieceKind::Knight),
];

type IsPieceKind = fn(PieceKind) -> bool;
const SLIDE_ATTACKS: [(&[Offset], IsPieceKind); 2] = [
    (&BISHOP_SLIDES, |k| {
        matches!(k, PieceKind::Bishop | PieceKind::Queen)
    }),
    (&ROOK_SLIDES, |k| {
        matches!(k, PieceKind::Rook | PieceKind::Queen)
    }),
];

const KNIGHT_OFFSETS: [Offset; 8] = [
    Offset::new(2, -1),
    Offset::new(2, 1),
    Offset::new(1, 2),
    Offset::new(-1, 2),
    Offset::new(-2, 1),
    Offset::new(-2, -1),
    Offset::new(-1, -2),
    Offset::new(1, -2),
];
const BISHOP_SLIDES: [Offset; 4] = [
    Offset::new(1, 1),
    Offset::new(1, -1),
    Offset::new(-1, 1),
    Offset::new(-1, -1),
];
const ROOK_SLIDES: [Offset; 4] = [
    Offset::new(1, 0),
    Offset::new(-1, 0),
    Offset::new(0, 1),
    Offset::new(0, -1),
];
const QUEEN_SLIDES: [Offset; 8] = KING_OFFSETS; // same 8 directions
const KING_OFFSETS: [Offset; 8] = [
    Offset::new(1, 0),
    Offset::new(1, 1),
    Offset::new(0, 1),
    Offset::new(-1, 1),
    Offset::new(-1, 0),
    Offset::new(-1, -1),
    Offset::new(0, -1),
    Offset::new(1, -1),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StartIndex(u16);
impl StartIndex {
    pub const fn new(x: u16) -> Result<Self, &'static str> {
        if x >= 960 {
            return Err("x must be 0..960");
        }

        Ok(Self(x))
    }
}

const KNIGHT_START_TABLE: [(usize, usize); 10] = [
    (0, 1),
    (0, 2),
    (0, 3),
    (0, 4),
    (1, 2),
    (1, 3),
    (1, 4),
    (2, 3),
    (2, 4),
    (3, 4),
];
