use crate::defines::{
    Bitboard, Piece, Side, BB_FOR_FILES, BB_FOR_RANKS, BITBOARDS_FOR_PIECES, BITBOARDS_PER_SIDE,
    BLACK, PNONE, WHITE,
};
use crate::fen;
use crate::utils::*;

pub struct Board {
    pub bb_w: [Bitboard; BITBOARDS_PER_SIDE as usize],
    pub bb_b: [Bitboard; BITBOARDS_PER_SIDE as usize],
    pub bb_pieces: [Bitboard; BITBOARDS_FOR_PIECES as usize],
    pub bb_files: [Bitboard; BB_FOR_FILES as usize],
    pub bb_ranks: [Bitboard; BB_FOR_RANKS as usize],
    pub active_color: u8,
    pub castling: u8,
    pub en_passant: Option<u8>,
    pub halfmove_clock: u8,
    pub fullmove_number: u16,
}

impl Default for Board {
    fn default() -> Board {
        Board {
            bb_w: [0; BITBOARDS_PER_SIDE as usize],
            bb_b: [0; BITBOARDS_PER_SIDE as usize],
            bb_pieces: [0; BITBOARDS_FOR_PIECES as usize],
            bb_files: [0; BB_FOR_FILES as usize],
            bb_ranks: [0; BB_FOR_RANKS as usize],
            active_color: WHITE as u8,
            castling: 0,
            en_passant: None,
            halfmove_clock: 0,
            fullmove_number: 0,
        }
    }
}

impl Board {
    pub fn create_piece_bitboards(&mut self) {
        // Iterate through all white and black bitboards.
        for (bb_w, bb_b) in self.bb_w.iter().zip(self.bb_b.iter()) {
            // Combine all white bitboards into one, having all white pieces,
            // Also combine all black bitboards into one, having all black pieces
            self.bb_pieces[WHITE] ^= bb_w;
            self.bb_pieces[BLACK] ^= bb_b;
        }
    }

    pub fn reset(&mut self) {
        self.bb_w = [0; BITBOARDS_PER_SIDE as usize];
        self.bb_b = [0; BITBOARDS_PER_SIDE as usize];
        self.bb_pieces = [0; BITBOARDS_FOR_PIECES as usize];
        self.active_color = WHITE as u8;
        self.castling = 0;
        self.en_passant = None;
        self.halfmove_clock = 0;
        self.fullmove_number = 0;
    }

    pub fn initialize(&mut self, fen: &str) {
        fen::read(fen, self);
        self.bb_files = create_bb_files();
        self.bb_ranks = create_bb_ranks();
    }

    pub fn get_pieces(&self, piece: Piece, side: Side) -> Bitboard {
        debug_assert!(piece <= 5, "Not a piece: {}", piece);
        debug_assert!(side == 0 || side == 1, "Not a side: {}", side);
        match side {
            WHITE => self.bb_w[piece],
            BLACK => self.bb_b[piece],
            _ => 0,
        }
    }

    pub fn which_piece(&self, square: u8) -> Piece {
        debug_assert!(square < 64, "Not a correct square number: {}", square);
        let inspect = 1u64 << square as u64;
        for (piece, (white, black)) in self.bb_w.iter().zip(self.bb_b.iter()).enumerate() {
            if (*white & inspect > 0) || (*black & inspect > 0) {
                return piece;
            }
        }
        PNONE
    }

    pub fn occupancy(&self) -> Bitboard {
        self.bb_pieces[WHITE] ^ self.bb_pieces[BLACK]
    }

    pub fn square_on_rank(&self, square: u8, rank: u8) -> bool {
        let start = (rank) * 8;
        let end = start + 7;
        (start..=end).contains(&square)
    }
}
