pub mod defs;
mod fen;
mod gamestate;
mod history;
mod playmove;
mod utils;
mod zobrist;

use crate::{
    defs::{Bitboard, NrOf, Piece, Side, Sides, Square, EMPTY},
    evaluation::{defs::PIECE_VALUES, material},
    misc::bits,
    movegen::{movelist::MoveList, MoveGenerator},
};
use defs::{Pieces, BB_SQUARES};
use gamestate::GameState;
use history::History;
use std::sync::Arc;
use zobrist::{ZobristKey, ZobristRandoms};

// TODO: Update comments
#[derive(Clone)]
pub struct Board {
    pub bb_side: [[Bitboard; NrOf::PIECE_TYPES as usize]; Sides::BOTH],
    pub bb_pieces: [Bitboard; Sides::BOTH],
    pub game_state: GameState,
    pub history: History,
    pub piece_list: [Piece; NrOf::SQUARES],
    pub material_count: [u16; Sides::BOTH],
    zobrist_randoms: Arc<ZobristRandoms>,
    move_generator: Arc<MoveGenerator>,
}

// Public functions for use by other modules.
impl Board {
    // Creates a new board with either the provided FEN, or the starting position.
    pub fn new(zr: Arc<ZobristRandoms>, mg: Arc<MoveGenerator>) -> Self {
        Self {
            bb_side: [[EMPTY; NrOf::PIECE_TYPES as usize]; Sides::BOTH],
            bb_pieces: [EMPTY; Sides::BOTH],
            game_state: GameState::new(),
            history: History::new(),
            piece_list: [Pieces::NONE; NrOf::SQUARES],
            material_count: [0; Sides::BOTH],
            zobrist_randoms: zr,
            move_generator: mg,
        }
    }

    // After reading the FEN-string, piece bitboards and lists must be initialized.
    pub fn init(&mut self) {
        let piece_bitboards = self.init_piece_bitboards();
        self.bb_pieces[Sides::WHITE] = piece_bitboards.0;
        self.bb_pieces[Sides::BLACK] = piece_bitboards.1;

        self.piece_list = self.init_piece_list();
        self.game_state.zobrist_key = self.init_zobrist_key();

        let material = material::count(&self);
        self.material_count[Sides::WHITE] = material.0;
        self.material_count[Sides::BLACK] = material.1;
    }

    // Reset the board.
    pub fn reset(&mut self) {
        self.bb_side = [[0; NrOf::PIECE_TYPES as usize]; Sides::BOTH];
        self.bb_pieces = [EMPTY; Sides::BOTH];
        self.piece_list = [Pieces::NONE; NrOf::SQUARES];
        self.game_state = GameState::new();
        self.history.clear();
    }

    // Return a bitboard with locations of a certain piece type for one of the sides.
    pub fn get_pieces(&self, piece: Piece, side: Side) -> Bitboard {
        self.bb_side[side][piece]
    }

    // Return a bitboard containing all the pieces on the board.
    pub fn occupancy(&self) -> Bitboard {
        self.bb_pieces[Sides::WHITE] | self.bb_pieces[Sides::BLACK]
    }

    pub fn us(&self) -> usize {
        self.game_state.active_color as usize
    }

    pub fn opponent(&self) -> usize {
        (self.game_state.active_color ^ 1) as usize
    }

    pub fn king_square(&self, side: Side) -> Square {
        self.bb_side[side][Pieces::KING].trailing_zeros() as Square
    }

    // Remove a piece from the board, for the given side, piece, and square.
    pub fn remove_piece(&mut self, side: Side, piece: Piece, square: Square) {
        self.piece_list[square] = Pieces::NONE;
        self.material_count[side] -= PIECE_VALUES[piece];
        self.game_state.zobrist_key ^= self.zobrist_randoms.piece(side, piece, square);
        self.bb_side[side][piece] ^= BB_SQUARES[square];
        self.bb_pieces[side] ^= BB_SQUARES[square];
    }

    // Put a piece onto the board, for the given side, piece, and square.
    pub fn put_piece(&mut self, side: Side, piece: Piece, square: Square) {
        self.bb_side[side][piece] |= BB_SQUARES[square];
        self.bb_pieces[side] |= BB_SQUARES[square];
        self.game_state.zobrist_key ^= self.zobrist_randoms.piece(side, piece, square);
        self.material_count[side] += PIECE_VALUES[piece];
        self.piece_list[square] = piece;
    }

    // Remove a piece from the from-square, and put it onto the to-square.
    pub fn move_piece(&mut self, side: Side, piece: Piece, from: Square, to: Square) {
        self.remove_piece(side, piece, from);
        self.put_piece(side, piece, to);
    }

    // Set a square as being the current ep-square.
    pub fn set_ep_square(&mut self, square: Square) {
        self.game_state.zobrist_key ^= self.zobrist_randoms.en_passant(self.game_state.en_passant);
        self.game_state.en_passant = Some(square as u8);
        self.game_state.zobrist_key ^= self.zobrist_randoms.en_passant(self.game_state.en_passant);
    }

    // Clear the ep-square. (If the ep-square is None already, nothing changes.)
    pub fn clear_ep_square(&mut self) {
        self.game_state.zobrist_key ^= self.zobrist_randoms.en_passant(self.game_state.en_passant);
        self.game_state.en_passant = None;
        self.game_state.zobrist_key ^= self.zobrist_randoms.en_passant(self.game_state.en_passant);
    }

    // Swap side from WHITE <==> BLACK
    pub fn swap_side(&mut self) {
        self.game_state.zobrist_key ^= self
            .zobrist_randoms
            .side(self.game_state.active_color as usize);
        self.game_state.active_color ^= 1;
        self.game_state.zobrist_key ^= self
            .zobrist_randoms
            .side(self.game_state.active_color as usize);
    }

    // Update castling permissions and take Zobrist-key into account.
    pub fn update_castling_permissions(&mut self, new_permissions: u8) {
        self.game_state.zobrist_key ^= self.zobrist_randoms.castling(self.game_state.castling);
        self.game_state.castling = new_permissions;
        self.game_state.zobrist_key ^= self.zobrist_randoms.castling(self.game_state.castling);
    }

    // True if the given side is attacking the given square.
    pub fn square_attacked(&self, attacker: Side, square: Square) -> bool {
        self.move_generator.square_attacked(self, attacker, square)
    }

    // Generates all pseudo-legal moves and puts them in the given move list.
    pub fn gen_all_moves(&self, ml: &mut MoveList) {
        self.move_generator.gen_all_moves(self, ml);
    }
}

// Private board functions (for initializating on startup)
impl Board {
    fn init_piece_bitboards(&self) -> (Bitboard, Bitboard) {
        let mut white: Bitboard = 0;
        let mut black: Bitboard = 0;

        for (bb_w, bb_b) in self.bb_side[Sides::WHITE]
            .iter()
            .zip(self.bb_side[Sides::BLACK].iter())
        {
            white |= *bb_w;
            black |= *bb_b;
        }

        (white, black)
    }

    fn init_piece_list(&self) -> [Piece; NrOf::SQUARES] {
        let bb_w = self.bb_side[Sides::WHITE]; // White bitboards
        let bb_b = self.bb_side[Sides::BLACK]; // Black bitboards
        let mut piece_list: [Piece; NrOf::SQUARES] = [Pieces::NONE; NrOf::SQUARES];

        for (p, (w, b)) in bb_w.iter().zip(bb_b.iter()).enumerate() {
            let mut white = *w; // White pieces of type "p"
            let mut black = *b; // Black pieces of type "p"

            while white > 0 {
                let square = bits::next(&mut white);
                piece_list[square] = p;
            }

            while black > 0 {
                let square = bits::next(&mut black);
                piece_list[square] = p;
            }
        }

        piece_list
    }

    fn init_zobrist_key(&self) -> ZobristKey {
        let mut key: u64 = 0;
        let zr = &self.zobrist_randoms;
        let bb_w = self.bb_side[Sides::WHITE];
        let bb_b = self.bb_side[Sides::BLACK];

        for (piece, (w, b)) in bb_w.iter().zip(bb_b.iter()).enumerate() {
            let mut white = *w;
            let mut black = *b;

            while white > 0 {
                let square = bits::next(&mut white);
                key ^= zr.piece(Sides::WHITE, piece, square);
            }

            while black > 0 {
                let square = bits::next(&mut black);
                key ^= zr.piece(Sides::BLACK, piece, square);
            }
        }

        key ^= zr.castling(self.game_state.castling);
        key ^= zr.side(self.game_state.active_color as usize);
        key ^= zr.en_passant(self.game_state.en_passant);

        key
    }
}
