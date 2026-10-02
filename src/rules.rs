use crate::game::{Color, File, Offset, Piece, PieceKind, Rank, Square};

// Responsible for checking legality of game state -----------------------------
struct MoveOutcome {
    start: Square,
    end: Square,
    kind: MoveKind,
}

enum MoveKind {
    Quiet,
    Capture,
    EnPassant,
    Castle(CastleSide),
    Promotion(PieceKind),
}

// todo: move somewhere else?
enum OpponentState {
    Chilling,
    Check,
    Checkmate,
    Stalemate,
}

pub struct Position {
    grid: [[Option<Piece>; 8]; 8],
    castle_origins: CastleOrigins,
    en_passant_shadow: Option<Square>, // one rank behind turboed pawn
}

impl Position {
    pub fn legal_moves(&mut self, start: Square) -> Option<Vec<MoveOutcome>> {
        let piece = self.cell(start)?;
        let pseudo_legal = self.moves_ignoring_check(start)?;

        let kings = self.kings(piece.color());

        let mut legal_moves = Vec::with_capacity(128); // todo: need to dedup
        for candidate in pseudo_legal {
            self.apply(candidate);
            for king in &kings {
                if self.is_attacked(*king, piece.color().opposite()) {
                    legal_moves.push(candidate);
                }
            }
        }

        // Some(
        //     pseudo_legal
        //         .into_iter()
        //         .filter(|candidate| {
        //             let after = self.applied(start, candidate);
        //             after.kings(color).all(|k| !after.is_attacked(k, color.other()))
        //         })
        //         .collect(),
        // )

        Some(legal_moves)
    }

    pub fn apply(&mut self, mv: MoveOutcome) {
        match mv.kind {
            MoveKind::Quiet => {
                // #[expect(clippy::expect_used, reason = "should only be passed valid moves")]
                let piece = self
                    .take_cell(mv.start)
                    .expect("move start must have some piece in quiet");
                self.set_cell(mv.end, piece);
            }
            MoveKind::Capture => {
                // #[expect(clippy::expect_used, reason = "should only be passed valid moves")]
                let piece = self
                    .take_cell(mv.start)
                    .expect("move start must have some piece in capture");
                let end_piece = self
                    .take_cell(mv.start)
                    .expect("move end must have some piece in capture");
                // todo: need to do something with the end_piece, needs to go to reserve
                self.set_cell(mv.end, piece);
            }
            MoveKind::EnPassant => {
                // #[expect(clippy::expect_used, reason = "should only be passed valid moves")]
                let piece = self
                    .take_cell(mv.start)
                    .expect("move start must have some piece in en passant");
                let end_piece = self
                    .take_cell(mv.start)
                    .expect("move end must have some piece in en passant");
                // todo: need to do something with the end_piece, needs to go to reserve
                self.set_cell(mv.end, piece); // todo: plus 1?
            }
            MoveKind::Castle() => {}
            MoveKind::Promotion() => {}
        }
    }

    pub fn undo(&mut self, mv: MoveOutcome) {}

    fn moves_ignoring_check(&self, start: Square) -> Option<Vec<MoveOutcome>> {
        let piece = self.cell(start)?;
        let moves = match piece.kind() {
            PieceKind::Pawn => self.pawn_moves(start, piece.color()),
            PieceKind::Bishop => self.slide_moves(start, piece.color(), &Self::BISHOP_SLIDES),
            PieceKind::Knight => self.step_moves(start, piece.color(), &Self::KNIGHT_OFFSETS),
            PieceKind::Rook => self.slide_moves(start, piece.color(), &Self::ROOK_SLIDES),
            PieceKind::Queen => self.slide_moves(start, piece.color(), &Self::QUEEN_SLIDES),
            PieceKind::King => {
                let mut moves = self.step_moves(start, piece.color(), &Self::KING_OFFSETS);
                moves.extend(self.king_moves(start, piece.color()));
                moves
            }
        };

        Some(moves)
    }

    fn step_moves(&self, start: Square, color: Color, offsets: &[Offset]) -> Vec<MoveOutcome> {
        let mut move_candidates = Vec::with_capacity(16);
        for offset in offsets {
            // check in bounds
            let Some(end) = start.offset(*offset) else {
                continue;
            };

            let end_piece = self.cell(end);
            // check if square open
            if end_piece.is_none() {
                move_candidates.push(MoveOutcome {
                    start,
                    end,
                    kind: MoveKind::Quiet,
                });
            // check if can capture enemy
            } else if end_piece.is_some_and(|e_p| e_p.color() != color) {
                move_candidates.push(MoveOutcome {
                    start,
                    end,
                    kind: MoveKind::Capture,
                });
            }
        }

        move_candidates
    }

    fn slide_moves(&self, start: Square, color: Color, dirs: &[Offset]) -> Vec<MoveOutcome> {
        let mut move_candidates = Vec::with_capacity(32);
        for offset in dirs {
            let mut curr_square = start;

            // apply move until OOB or at another piece
            while let Some(end) = curr_square.offset(*offset) {
                let end_piece = self.cell(end);
                // empty
                if end_piece.is_none() {
                    move_candidates.push(MoveOutcome {
                        start,
                        end,
                        kind: MoveKind::Quiet,
                    });
                // can take enemy
                } else if end_piece.is_some_and(|e_p| e_p.color() != color) {
                    move_candidates.push(MoveOutcome {
                        start,
                        end,
                        kind: MoveKind::Capture,
                    });
                    break;
                // blocked by own piece
                } else {
                    break;
                }

                curr_square = end;
            }
        }

        move_candidates
    }

    fn pawn_moves(&self, start: Square, color: Color) -> Vec<MoveOutcome> {
        // promotion is inferred and applied after the fact
        let (rank, _) = start.index();
        let mut moves = Vec::with_capacity(5);
        let (rank_delta, spawn_rank) = match color {
            Color::White => (1, 1),
            Color::Black => (-1, 6),
        };

        // forward
        if let Some(one) = start
            .offset(Offset::new(rank_delta, 0))
            .filter(|e| self.cell(*e).is_none())
        {
            moves.push(MoveOutcome {
                start,
                end: one,
                kind: MoveKind::Quiet,
            });

            // turbo
            if rank == spawn_rank
                && let Some(two) = one
                    .offset(Offset::new(rank_delta, 0))
                    .filter(|e| self.cell(*e).is_none())
            {
                moves.push(MoveOutcome {
                    start,
                    end: two,
                    kind: MoveKind::Quiet,
                });
            }
        }

        // diagonal capture
        for file_delta in [-1, 1] {
            // check in bounds
            let Some(end) = start.offset(Offset::new(rank_delta, file_delta)) else {
                continue;
            };

            // check diagonal target
            if self.cell(end).is_some_and(|p| p.color() != color) {
                moves.push(MoveOutcome {
                    start,
                    end,
                    kind: MoveKind::Capture,
                });
            } else if self.en_passant_shadow == Some(end) {
                moves.push(MoveOutcome {
                    start,
                    end,
                    kind: MoveKind::EnPassant,
                });
            }
        }

        moves
    }

    fn king_moves(&self, start: Square, color: Color) -> Vec<MoveOutcome> {
        // castle speicifc king moves
        let mut moves = Vec::with_capacity(2);
        let rank = match color {
            Color::White => Rank::lit(0),
            Color::Black => Rank::lit(7),
        };
        let o = &self.castle_origins;

        let a_rook_ok = self
            .cell(Square::new(rank, o.a_side_rook))
            .is_some_and(|p| p.color() == color && p.kind() == PieceKind::Rook);
        let king_ok = self
            .cell(Square::new(rank, o.king))
            .is_some_and(|p| p.color() == color && p.kind() == PieceKind::King);
        let h_rook_ok = self
            .cell(Square::new(rank, o.h_side_rook))
            .is_some_and(|p| p.color() == color && p.kind() == PieceKind::Rook);

        let exempt = [o.a_side_rook, o.king];
        if a_rook_ok
            && king_ok
            && self.rank_clear(rank, o.a_side_rook, CastleSide::ASide.rook_file(), exempt)
            && self.rank_clear(rank, o.king, CastleSide::ASide.king_file(), exempt)
        {
            moves.push(MoveOutcome {
                start,
                end: Square::new(rank, CastleSide::ASide.king_file()),
                kind: MoveKind::Castle(CastleSide::ASide),
            });
        }

        let exempt = [o.king, o.h_side_rook];
        if king_ok
            && h_rook_ok
            && self.rank_clear(rank, o.king, CastleSide::HSide.king_file(), exempt)
            && self.rank_clear(rank, o.h_side_rook, CastleSide::HSide.rook_file(), exempt)
        {
            moves.push(MoveOutcome {
                start,
                end: Square::new(rank, CastleSide::HSide.king_file()),
                kind: MoveKind::Castle(CastleSide::HSide),
            });
        }

        moves
    }

    fn is_attacked(&self, start: Square, color: Color) -> bool {
        // PAWN
        let rank_delta = match color {
            Color::White => -1,
            Color::Black => 1,
        };
        for file_delta in [-1, 1] {
            // check in bounds
            let Some(end) = start.offset(Offset::new(rank_delta, file_delta)) else {
                continue;
            };

            // check diagonal target
            if self.cell(end).is_some_and(|p| p.color() != color) {
                return true;
            }
        }

        // KNIGHT, KING
        for (piece_offsets, piece_kind) in Self::STEP_ATTACKS {
            for offset in piece_offsets {
                // check in bounds
                let Some(end) = start.offset(*offset) else {
                    continue;
                };

                // check if enemy piece is attacking
                let end_piece = self.cell(end);
                if end_piece.is_some_and(|e_p| e_p.color() != color && e_p.kind() == piece_kind) {
                    return true;
                }
            }
        }

        // BISHOP, ROOK, QUEEN
        for (piece_offsets, is_piece) in Self::SLIDE_ATTACKS {
            for offset in piece_offsets {
                let mut curr_square = start;

                // apply move until OOB or at another piece
                while let Some(end) = curr_square.offset(*offset) {
                    let end_piece = self.cell(end);

                    // check if enemy piece is attacking
                    if end_piece.is_some_and(|e_p| e_p.color() != color && is_piece(e_p.kind())) {
                        return true;
                    // blocked by own piece
                    } else if end_piece.is_some_and(|e_p| e_p.color() == color) {
                        break;
                    }

                    curr_square = end;
                }
            }
        }

        false
    }

    fn cell(&self, square: Square) -> Option<&Piece> {
        let (rank, file) = square.index();
        #[expect(clippy::indexing_slicing, reason = "Square must be 0..=7")]
        let cell = &self.grid[rank][file];
        cell.as_ref()
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

    fn kings(&self, color: Color) -> Vec<Square> {
        let mut kings = Vec::with_capacity(2);
        for rank in 0..=7u8 {
            for file in 0..=7u8 {
                let square = Square::new(Rank::lit(rank), File::lit(file));
                let piece = self.cell(square);
                if let Some(piece) = piece
                    && piece.color() == color
                    && matches!(piece.kind(), PieceKind::King)
                {
                    kings.push(square);
                }
            }
        }

        kings
    }

    const STEP_ATTACKS: [(&[Offset], PieceKind); 2] = [
        (&Self::KING_OFFSETS, PieceKind::King),
        (&Self::KNIGHT_OFFSETS, PieceKind::Knight),
    ];
    const SLIDE_ATTACKS: [(&[Offset], fn(PieceKind) -> bool); 2] = [
        (&Self::BISHOP_SLIDES, |k| {
            matches!(k, PieceKind::Bishop | PieceKind::Queen)
        }),
        (&Self::ROOK_SLIDES, |k| {
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
    const QUEEN_SLIDES: [Offset; 8] = [
        Offset::new(1, 0),
        Offset::new(-1, 0),
        Offset::new(0, 1),
        Offset::new(0, -1),
        Offset::new(1, 1),
        Offset::new(1, -1),
        Offset::new(-1, 1),
        Offset::new(-1, -1),
    ];
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
}

enum CastleSide {
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
struct CastleOrigins {
    a_side_rook: File,
    king: File,
    h_side_rook: File,
}
