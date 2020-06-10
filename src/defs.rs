/**
 * defs.rs holds all the definitions that are needed throughout the program.
 * If there are definitions that are needed only inside a single module, those
 * can be found within that module.
*/

pub type Bitboard = u64;
pub type Piece = usize;
pub type Side = usize;
pub type Square = usize;

pub const WHITE: Side = 0;
pub const BLACK: Side = 1;

pub const FEN_START_POSITION: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

pub struct NrOf;
impl NrOf {
    pub const SQUARES: usize = 64;
    pub const FILES: usize = 8;
    pub const RANKS: usize = 8;
    pub const PIECES: usize = 6;
    pub const CASTLING_PERMISSIONS: usize = 16; // 0-15
}

pub const EACH_SIDE: usize = 2;

pub struct Castling;
impl Castling {
    pub const WK: u8 = 1;
    pub const WQ: u8 = 2;
    pub const BK: u8 = 4;
    pub const BQ: u8 = 8;
    pub const ALL: u8 = 15;
}

pub const EMPTY: u64 = 0;
pub const MAX_GAME_MOVES: usize = 2048;
pub const MAX_LEGAL_MOVES: u8 = 255;
