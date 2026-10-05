## Random Thoughts
- Wikipedia says Bughouse has no endgame play as pieces are endlessly recycled
- What if choosing to retain a pieces costs a time penalty?
- Or some minigame needs to be played in order to keep the piece?
- There should probably be some skill based challenge with the opportunity to take the piece out of circulation

- Wikipedia says Bughouse discourages calculation as the boards are not kept in sync
- Only have one timer per team? Play serially?

- Allow multiple kings

- Allow illegal moves. Let receiver call out opponent and undo?
- Go by clock move, not touch move. UI simulates grabbing, swapping, and placing pieces

## Todos
- I call piece.color() a lot. Prob doing something wrong

// GENERATED BELOW
- legal_drop_moves ignores the reserve count (`_`), so a zero-count entry still produces drops
- valid_promos: rename `opp_back_rank`, it holds the end square's rank
- Color helpers: `forward() -> i8`, `back_rank() -> Rank`, `pawn_rank() -> Rank`. Each is matched by hand in 5 places (pawn_moves, is_attacked, apply EnPassant, king_castle_moves, apply Castle). Also removes the usize compares on `start.index()`
- `replace_cell(sq, piece) -> Option<Piece>`: lets apply handle Quiet and Capture in one branch, drops the expect, and makes a drop onto an occupied square return the displaced piece instead of deleting it
- apply: compute the promoted piece once with `promo.map_or(piece, |k| Piece::new(k, piece.color(), true))` instead of repeating the `if let` in two branches
- `promo` on Move::Board means a Turbo/EnPassant/Castle move can carry a promo. Decide: move it into `Quiet { promo }` / `Capture { promo }`, or accept apply ignoring it
- `Turbo { shadow }` can be derived from start/end. A client can send a bogus one and plant a fake en passant square. Compute it in apply instead
- Castling: add `CastleOrigins::rook(side) -> File` and loop over `[ASide, HSide]` to merge the two blocks in king_castle_moves and the match in apply
- step_moves / slide_moves: replace the is_none / is_some_and chain with `match self.cell(end) { None => .., Some(p) if p.color() != pc => .., Some(_) => break }`
- SLIDE_ATTACKS: store `(&ROOK_SLIDES, &[Rook, Queen])` and use `.contains`, not closures
- Square::all() to replace the nested loops in kings() and valid_drops. Have helpers return iterators and collect only at the boundary
- Stale: expect messages in apply, the "// king" comment on the rook line in Castle, the pawn_moves "inferred after the fact" comment, Board's 3-fold todo
- Reserve: `struct Reserve { color, counts: [u8; 5] }` (no King slot)
- Crash vectors: the expects in apply and the assert in kings() can be triggered by client input
