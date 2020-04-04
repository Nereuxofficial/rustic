use super::representation::{Board, UnMakeInfo};
use super::unmake_move::unmake_move;
use crate::defs::{
    Side, A1, A8, BLACK, C1, C8, CASTLE_BK, CASTLE_BQ, CASTLE_WK, CASTLE_WQ, D1, D8, E1, E8, F1,
    F8, G1, G8, H1, H8, KING, PAWN, PNONE, ROOK, WHITE,
};
use crate::movegen::information::square_attacked;
use crate::movegen::movedefs::Move;

#[allow(clippy::cognitive_complexity)]
pub fn make_move(board: &mut Board, m: Move) -> bool {
    // create the unmake info and store it.
    let unmake_info = UnMakeInfo::new(
        board.active_color,
        board.castling,
        board.en_passant,
        board.halfmove_clock,
        board.fullmove_number,
        board.zobrist_key,
        m,
    );
    board.unmake_list.push(unmake_info);

    // Set "us" and "opponent"
    let us = board.active_color as usize;
    let opponent = (us ^ 1) as usize;

    // Dissect the move
    let piece = m.piece() as usize;
    let from = m.from();
    let to = m.to();
    let captured = m.captured() as usize;
    let promoted = m.promoted() as usize;
    let castling = m.castling();
    let double_step = m.double_step();
    let en_passant = m.en_passant();
    let promotion_move = promoted != PNONE;

    // Move captured a piece. Remove it.
    if captured != PNONE {
        board.remove_piece(opponent, captured, to);
        if captured == ROOK {
            adjust_castling_perms_on_rook_capture(board, us, to);
        }
    }

    // Actually perform the move, taking promotion into account.
    board.remove_piece(us, piece, from);
    if !promotion_move {
        board.put_piece(us, piece, to);
    } else {
        board.put_piece(us, promoted, to);
    }

    // We're castling. This is a special case.
    if castling {
        // remove current castling rights from the zobrist key.
        board.zobrist_key ^= board.zobrist_randoms.castling(board.castling);

        // The king was already moved as a "normal" move. Now move the correct rook.
        if to == G1 {
            // White is castling short. Pick up rook h1.
            board.remove_piece(us, ROOK, H1);

            // Put it back down on f1.
            board.put_piece(us, ROOK, F1);

            // Remove all castling permissions for white (clear bits 0 and 1)
            board.castling &= !(CASTLE_WK + CASTLE_WQ);
        }

        if to == C1 {
            // White is castling long. Pick up rook A1.
            board.remove_piece(us, ROOK, A1);

            // Put it back down on d1.
            board.put_piece(us, ROOK, D1);

            // Remove all castling permissions for white (clear bits 0 and 1)
            board.castling &= !(CASTLE_WK + CASTLE_WQ);
        }

        if to == G8 {
            // Black is castling short. Pick up rook h8.
            board.remove_piece(us, ROOK, H8);

            // Put it back down on f8.
            board.put_piece(us, ROOK, F8);

            // Remove all castling permissions for black (clear bits 2 and 3)
            board.castling &= !(CASTLE_BK + CASTLE_BQ);
        }

        if to == C8 {
            // Black is castling long. Pick up rook a8.
            board.remove_piece(us, ROOK, A8);

            // Put it back down on d8.
            board.put_piece(us, ROOK, D8);

            // Remove all castling permissions for black (clear bits 2 and 3)
            board.castling &= !(CASTLE_BK + CASTLE_BQ);
        }

        // add resulting castling permissions to the zobrist key.
        board.zobrist_key ^= board.zobrist_randoms.castling(board.castling);
    }

    // After the en-passant maneuver, the opponent's pawn has yet to be removed.
    if en_passant {
        let pawn_square = if us == WHITE { to - 8 } else { to + 8 };
        board.remove_piece(opponent, PAWN, pawn_square);
    }

    //region: Updating the board state

    // If moving the king or one of the rooks, castling permissions are dropped.
    if !castling && (piece == KING || piece == ROOK) {
        // remove current castling permissions from zobrist key.
        board.zobrist_key ^= board.zobrist_randoms.castling(board.castling);

        if from == H1 {
            // remove kingside castling (clear bit 0)
            board.castling &= !CASTLE_WK;
        }
        if from == A1 {
            // remove queenside castling (clear bit 1)
            board.castling &= !CASTLE_WQ;
        }
        if from == E1 {
            // remove both castling rights (clear bit 0 and 1)
            board.castling &= !(CASTLE_WK + CASTLE_WQ);
        }

        if from == H8 {
            // remove kingside castling (clear bit 2)
            board.castling &= !CASTLE_BK;
        }
        if from == A8 {
            // remove queenside castling (clear bit 3)
            board.castling &= !CASTLE_BQ;
        }
        if from == E8 {
            // remove all castling rights (clear bit 2 and 3)
            board.castling &= !(CASTLE_BK + CASTLE_BQ);
        }

        // add resulting castling rights back into zobrist key.
        board.zobrist_key ^= board.zobrist_randoms.castling(board.castling);
    }

    // If the en-passant square is set, every move will unset it...
    if board.en_passant.is_some() {
        board.zobrist_key ^= board.zobrist_randoms.en_passant(board.en_passant);
        board.en_passant = None;
    }

    // ...except a pawn double-step, which will set it (again, if just unset).
    if double_step {
        board.en_passant = if us == WHITE {
            Some(to - 8)
        } else {
            Some(to + 8)
        };
        board.zobrist_key ^= board.zobrist_randoms.en_passant(board.en_passant);
    }

    // change the color to move: out with "us", in with "opponent"
    board.zobrist_key ^= board.zobrist_randoms.side(us);
    board.zobrist_key ^= board.zobrist_randoms.side(opponent);
    board.active_color = opponent as u8;

    // If a pawn moves or a piece is captured, reset the 50-move counter.
    // Otherwise, increment the counter by one move.
    if (piece == PAWN) || (captured != PNONE) {
        board.halfmove_clock = 0;
    } else {
        board.halfmove_clock += 1;
    }

    // Increase full move number if black has moved
    if us == BLACK {
        board.fullmove_number += 1;
    }
    //endregion

    /*** Validating move ***/

    // Move is done. Check if it's actually legal. (King can not be in check.)
    let king_square = board.get_pieces(KING, us).trailing_zeros() as u8;
    let is_legal = !square_attacked(board, opponent, king_square);

    if !is_legal {
        unmake_move(board);
    }

    is_legal
}

// This function changes castling permissions according to the rook being captured
fn adjust_castling_perms_on_rook_capture(board: &mut Board, side: Side, square: u8) {
    board.zobrist_key ^= board.zobrist_randoms.castling(board.castling);
    if side == WHITE {
        match square {
            H8 => board.castling &= !CASTLE_BK,
            A8 => board.castling &= !CASTLE_BQ,
            _ => (),
        }
    } else {
        match square {
            H1 => board.castling &= !CASTLE_WK,
            A1 => board.castling &= !CASTLE_WQ,
            _ => (),
        }
    }
    board.zobrist_key ^= board.zobrist_randoms.castling(board.castling);
}
