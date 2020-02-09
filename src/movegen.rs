use crate::board::Board;
use crate::defines::*;

fn next(bitboard: &mut Bitboard) -> u8 {
    let location = bitboard.trailing_zeros();
    *bitboard ^= 1 << location;
    (location as u8)
}

fn king(board: &Board, side: Side) {
    println!("King");
    let mut bitboard = board.piece(KING, side);
    println!("Before: {:b}", bitboard);
    let location = next(&mut bitboard);
    println!("King location: {}", SQUARE_NAME[location as usize]);
    println!("After: {:b}", bitboard);
}

fn knight(board: &Board, side: Side) {
    println!("Knight");
    let mut bitboard = board.piece(KNIGHT, side);
    while bitboard > 0 {
        println!("Before: {:b}", bitboard);
        let location = next(&mut bitboard);
        println!("Knight location: {}", SQUARE_NAME[location as usize]);
        println!("After: {:b}", bitboard);
    }
}

fn pawns(board: &Board, side: Side) {
    println!("Pawns");
    let mut bitboard = board.piece(PAWN, side);
    while bitboard > 0 {
        println!("Before: {:b}", bitboard);
        let location = next(&mut bitboard);
        println!("Pawn location: {}", SQUARE_NAME[location as usize]);
        println!("After: {:b}", bitboard);
    }
}

pub fn generate(board: &Board, side: Side) {
    println!("Generating moves...");
    king(board, side);
    knight(board, side);
    pawns(board, side);
}
