/**
 * The print.rs module is used to print information such as the current position,
 * and the contents of bitboards to the screen. This is mainly useful for debugging,
 * because the position and bitboards contain only numbers, which are meaningless to
 * humans when viewed in in decimal, or as one long string. In normal play, the
 * functionality of this module will not be used.
 */
use crate::board::representation::Board;
use crate::defs::{
    Bitboard, ALL_FILES, ALL_RANKS, BISHOP, BLACK, CASTLE_BK, CASTLE_BQ, CASTLE_WK, CASTLE_WQ,
    KING, KNIGHT, NR_OF_FILES, NR_OF_SQUARES, PAWN, QUEEN, ROOK, SQUARE_NAME, WHITE,
};
use crate::movegen::{magics::Magics, movedefs::Move};

type AsciiBoard = [char; NR_OF_SQUARES as usize];

const ASCII_EMPTY_SQUARE: char = '.';
const CHAR_WK: char = 'K';
const CHAR_WQ: char = 'Q';
const CHAR_WR: char = 'R';
const CHAR_WB: char = 'B';
const CHAR_WN: char = 'N';
const CHAR_WP: char = 'I';
const CHAR_BK: char = 'k';
const CHAR_BQ: char = 'q';
const CHAR_BR: char = 'r';
const CHAR_BB: char = 'b';
const CHAR_BN: char = 'n';
const CHAR_BP: char = 'i';

const PIECE_CHAR: [&str; 7] = ["K", "Q", "R", "B", "N", "", "_"];
pub const PIECE_NAME: [&str; 7] = ["King", "Queen", "Rook", "Bishop", "Knight", "Pawn", "-"];

/* Prints the current position to the screen. */
#[allow(dead_code)]
pub fn position(board: &Board, mark_square: Option<u8>) {
    let mut ascii_board: AsciiBoard = [ASCII_EMPTY_SQUARE; NR_OF_SQUARES as usize];
    bitboards_to_ascii(board, &mut ascii_board);
    to_console(&ascii_board, mark_square);
    metadata(board);
}

/* This prints a bitboard (64-bit number) to the screen in an 8x8 grid. */
#[allow(dead_code)]
pub fn bitboard(bitboard: Bitboard, mark_square: Option<u8>) {
    const SQUARE_OCCUPIED: char = '1';
    let mut ascii_board: AsciiBoard = [ASCII_EMPTY_SQUARE; 64];
    put_character_on_square(bitboard, &mut ascii_board, SQUARE_OCCUPIED);
    to_console(&ascii_board, mark_square);
}

/* Prints a given movelist to the screen. */
#[allow(dead_code)]
pub fn movelist(moves: &[Move]) {
    for m in moves.iter() {
        move_data(*m);
    }
}

pub fn move_data(m: Move) {
    println!(
        "Move: {}{}{} capture: {}, promotion: {}, ep: {}, double: {}, castling: {}",
        PIECE_CHAR[m.piece() as usize],
        SQUARE_NAME[m.from() as usize],
        SQUARE_NAME[m.to() as usize],
        PIECE_NAME[m.captured() as usize],
        PIECE_NAME[m.promoted() as usize],
        m.en_passant(),
        m.double_step(),
        m.castling(),
    );
}

/* This function prints a found magic number and its stats. */
#[allow(dead_code)]
pub fn found_magic(sq: u8, m: Magics, offset: u64, end: u64, attempts: u64) {
    println!(
        "Magic found for {}: {:24}u64 (offset: {:6} end: {:6}, attempts: {})",
        SQUARE_NAME[sq as usize], m.magic, offset, end, attempts
    );
}

/* Create a printable ASCII-board out of bitboards. */
#[allow(dead_code)]
fn bitboards_to_ascii(board: &Board, ascii_board: &mut AsciiBoard) {
    let bb_w = board.bb_side[WHITE];
    let bb_b = board.bb_side[BLACK];

    for (piece, (w, b)) in bb_w.iter().zip(bb_b.iter()).enumerate() {
        match piece {
            KING => {
                put_character_on_square(*w, ascii_board, CHAR_WK);
                put_character_on_square(*b, ascii_board, CHAR_BK);
            }
            QUEEN => {
                put_character_on_square(*w, ascii_board, CHAR_WQ);
                put_character_on_square(*b, ascii_board, CHAR_BQ);
            }
            ROOK => {
                put_character_on_square(*w, ascii_board, CHAR_WR);
                put_character_on_square(*b, ascii_board, CHAR_BR);
            }
            BISHOP => {
                put_character_on_square(*w, ascii_board, CHAR_WB);
                put_character_on_square(*b, ascii_board, CHAR_BB);
            }
            KNIGHT => {
                put_character_on_square(*w, ascii_board, CHAR_WN);
                put_character_on_square(*b, ascii_board, CHAR_BN);
            }
            PAWN => {
                put_character_on_square(*w, ascii_board, CHAR_WP);
                put_character_on_square(*b, ascii_board, CHAR_BP);
            }
            _ => (),
        }
    }
}

//** This function actually puts the correct character into the ASCII board. */
#[allow(dead_code)]
fn put_character_on_square(bitboard: Bitboard, ascii_board: &mut AsciiBoard, character: char) {
    for (i, square) in ascii_board.iter_mut().enumerate() {
        if (bitboard >> i) & 1 == 1 {
            *square = character;
        }
    }
}

/* Print the generated ASCII-board to the console. Optionally mark one square. */
#[allow(dead_code)]
fn to_console(ascii_board: &AsciiBoard, mark_square: Option<u8>) {
    let coordinate_alpha: &str = "ABCDEFGH";
    let mut coordinate_digit = NR_OF_FILES;

    println!();
    for current_rank in ALL_RANKS.rev() {
        print!("{}   ", coordinate_digit);
        for current_file in ALL_FILES {
            let square = (current_rank * NR_OF_FILES + current_file) as usize;
            let character = ascii_board[square];
            if let Some(m) = mark_square {
                if m == (square as u8) {
                    // \x1b[0;35m is magenta
                    print!("\x1b[0;35m{} \x1b[0m", character);
                } else {
                    print!("{} ", character);
                }
            } else {
                print!("{} ", character);
            }
        }
        println!();
        coordinate_digit -= 1;
    }
    println!();
    print!("    ");
    for c in coordinate_alpha.chars() {
        print!("{} ", c);
    }
    println!();
    println!();
}

/** This function prints all of the metadata about the position. */
fn metadata(board: &Board) {
    let castling = castling_as_string(board.castling);
    let en_passant = if let Some(ep) = board.en_passant {
        SQUARE_NAME[ep as usize]
    } else {
        "-"
    };
    let active_color = if (board.active_color as usize) == WHITE {
        "White"
    } else {
        "Black"
    };

    println!("{:<20}{:x}", "Zobrist key:", board.zobrist_key);
    println!("{:<20}{}", "Active Color:", active_color);
    println!("{:<20}{}", "Castling:", castling);
    println!("{:<20}{}", "En Passant:", en_passant);
    println!("{:<20}{}", "Half-move clock:", board.halfmove_clock);
    println!("{:<20}{}", "Full-move number:", board.fullmove_number);
    println!();
}

fn castling_as_string(permissions: u8) -> String {
    let mut castling_as_string: String = String::from("");
    let p = permissions;

    castling_as_string += if p & CASTLE_WK > 0 { "K" } else { "" };
    castling_as_string += if p & CASTLE_WQ > 0 { "Q" } else { "" };
    castling_as_string += if p & CASTLE_BK > 0 { "k" } else { "" };
    castling_as_string += if p & CASTLE_BQ > 0 { "q" } else { "" };

    castling_as_string
}
