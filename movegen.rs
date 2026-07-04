// src/movegen.rs
// Move generation and attack detection for ZeroGravity

use std::sync::OnceLock;
use crate::board::{
    Board, Move, WHITE, BLACK, KNIGHT, BISHOP, ROOK, QUEEN,
    WHITE_PAWN, WHITE_KNIGHT, WHITE_BISHOP, WHITE_ROOK, WHITE_QUEEN, WHITE_KING,
    BLACK_PAWN, BLACK_KNIGHT, BLACK_BISHOP, BLACK_ROOK, BLACK_QUEEN, BLACK_KING,
    CASTLE_WHITE_OO, CASTLE_WHITE_OOO, CASTLE_BLACK_OO, CASTLE_BLACK_OOO,
    bit_scan_forward
};

pub struct MoveTables {
    pub ray_squares: [[Vec<u8>; 8]; 64],
    pub knight_attacks: [u64; 64],
    pub king_attacks: [u64; 64],
    pub pawn_attacks: [[u64; 64]; 2],
}

pub static MOVE_TABLES: OnceLock<MoveTables> = OnceLock::new();

pub fn get_move_tables() -> &'static MoveTables {
    MOVE_TABLES.get_or_init(|| {
        let mut ray_squares: [[Vec<u8>; 8]; 64] = std::array::from_fn(|_| {
            std::array::from_fn(|_| Vec::new())
        });
        let mut knight_attacks = [0_u64; 64];
        let mut king_attacks = [0_u64; 64];
        let mut pawn_attacks = [[0_u64; 64]; 2];

        // 1. Rays
        let dirs = [
            (1, 1),   // NE (0)
            (-1, 1),  // NW (1)
            (-1, -1), // SW (2)
            (1, -1),  // SE (3)
            (0, 1),   // N  (4)
            (0, -1),  // S  (5)
            (1, 0),   // E  (6)
            (-1, 0)   // W  (7)
        ];
        for sq in 0..64 {
            let file = sq % 8;
            let rank = sq / 8;
            for (d_idx, &(df, dr)) in dirs.iter().enumerate() {
                let mut cur_f = file as i32 + df;
                let mut cur_r = rank as i32 + dr;
                let mut ray = Vec::new();
                while (0..8).contains(&cur_f) && (0..8).contains(&cur_r) {
                    let target_sq = (cur_r * 8 + cur_f) as u8;
                    ray.push(target_sq);
                    cur_f += df;
                    cur_r += dr;
                }
                ray_squares[sq][d_idx] = ray;
            }
        }

        // 2. Knight attacks
        let knight_offsets = [(2, 1), (2, -1), (-2, 1), (-2, -1), (1, 2), (1, -2), (-1, 2), (-1, -2)];
        for sq in 0..64 {
            let file = sq % 8;
            let rank = sq / 8;
            for &(df, dr) in knight_offsets.iter() {
                let nf = file as i32 + df;
                let nr = rank as i32 + dr;
                if (0..8).contains(&nf) && (0..8).contains(&nr) {
                    knight_attacks[sq] |= 1_u64 << (nr * 8 + nf);
                }
            }
        }

        // 3. King attacks
        let king_offsets = [(1, 1), (1, 0), (1, -1), (0, 1), (0, -1), (-1, 1), (-1, 0), (-1, -1)];
        for sq in 0..64 {
            let file = sq % 8;
            let rank = sq / 8;
            for &(df, dr) in king_offsets.iter() {
                let nf = file as i32 + df;
                let nr = rank as i32 + dr;
                if (0..8).contains(&nf) && (0..8).contains(&nr) {
                    king_attacks[sq] |= 1_u64 << (nr * 8 + nf);
                }
            }
        }

        // 4. Pawn attacks
        for sq in 0..64 {
            let file = sq % 8;
            let rank = sq / 8;
            // White pawns attack up
            for &df in &[-1, 1] {
                let nf = file as i32 + df;
                let nr = rank as i32 + 1;
                if (0..8).contains(&nf) && (0..8).contains(&nr) {
                    pawn_attacks[WHITE][sq] |= 1_u64 << (nr * 8 + nf);
                }
            }
            // Black pawns attack down
            for &df in &[-1, 1] {
                let nf = file as i32 + df;
                let nr = rank as i32 - 1;
                if (0..8).contains(&nf) && (0..8).contains(&nr) {
                    pawn_attacks[BLACK][sq] |= 1_u64 << (nr * 8 + nf);
                }
            }
        }

        MoveTables { ray_squares, knight_attacks, king_attacks, pawn_attacks }
    })
}

#[inline(always)]
pub fn get_squares(mut bb: u64) -> Vec<u8> {
    let mut squares = Vec::with_capacity(bb.count_ones() as usize);
    while bb != 0 {
        let sq = bb.trailing_zeros() as u8;
        squares.push(sq);
        bb &= bb - 1;
    }
    squares
}

pub fn get_sliding_attacks(sq: u8, occupied: u64, is_bishop: bool) -> u64 {
    let tables = get_move_tables();
    let mut attacks = 0_u64;
    let directions = if is_bishop { &[0, 1, 2, 3] } else { &[4, 5, 6, 7] };
    for &d in directions {
        for &target_sq in &tables.ray_squares[sq as usize][d] {
            attacks |= 1_u64 << target_sq;
            if (occupied & (1_u64 << target_sq)) != 0 {
                break;
            }
        }
    }
    attacks
}

pub fn is_square_attacked(board: &Board, sq: u8, attacker_color: usize) -> bool {
    let tables = get_move_tables();
    let defender_color = 1 - attacker_color;

    // Pawns
    let pawn_attackers = board.pieces[if attacker_color == WHITE { WHITE_PAWN } else { BLACK_PAWN }];
    if (tables.pawn_attacks[defender_color][sq as usize] & pawn_attackers) != 0 {
        return true;
    }

    // Knights
    let knight_attackers = board.pieces[if attacker_color == WHITE { WHITE_KNIGHT } else { BLACK_KNIGHT }];
    if (tables.knight_attacks[sq as usize] & knight_attackers) != 0 {
        return true;
    }

    // Kings
    let king_attackers = board.pieces[if attacker_color == WHITE { WHITE_KING } else { BLACK_KING }];
    if (tables.king_attacks[sq as usize] & king_attackers) != 0 {
        return true;
    }

    // Bishop / Queen
    let bq_attackers = board.pieces[if attacker_color == WHITE { WHITE_BISHOP } else { BLACK_BISHOP }] |
                       board.pieces[if attacker_color == WHITE { WHITE_QUEEN } else { BLACK_QUEEN }];
    if bq_attackers != 0 && (get_sliding_attacks(sq, board.occupied(), true) & bq_attackers) != 0 {
        return true;
    }

    // Rook / Queen
    let rq_attackers = board.pieces[if attacker_color == WHITE { WHITE_ROOK } else { BLACK_ROOK }] |
                       board.pieces[if attacker_color == WHITE { WHITE_QUEEN } else { BLACK_QUEEN }];
    if rq_attackers != 0 && (get_sliding_attacks(sq, board.occupied(), false) & rq_attackers) != 0 {
        return true;
    }

    false
}

pub fn generate_pseudo_legal_moves(board: &Board) -> Vec<Move> {
    let tables = get_move_tables();
    let mut moves = Vec::with_capacity(40);
    let us = board.side_to_move;

    let our_occupied = if us == WHITE { board.occupied_white() } else { board.occupied_black() };
    let their_occupied = if us == WHITE { board.occupied_black() } else { board.occupied_white() };
    let occupied = board.occupied();

    // Piece types
    let pawn_p = if us == WHITE { WHITE_PAWN } else { BLACK_PAWN };
    let knight_p = if us == WHITE { WHITE_KNIGHT } else { BLACK_KNIGHT };
    let bishop_p = if us == WHITE { WHITE_BISHOP } else { BLACK_BISHOP };
    let rook_p = if us == WHITE { WHITE_ROOK } else { BLACK_ROOK };
    let queen_p = if us == WHITE { WHITE_QUEEN } else { BLACK_QUEEN };
    let king_p = if us == WHITE { WHITE_KING } else { BLACK_KING };

    // 1. PAWN MOVES
    let direction: i32 = if us == WHITE { 8 } else { -8 };
    let promo_rank = if us == WHITE { 7 } else { 0 };
    let start_rank = if us == WHITE { 1 } else { 6 };

    let pawns = board.pieces[pawn_p];
    for from_sq in get_squares(pawns) {
        // Single push
        let to_sq = (from_sq as i32 + direction) as u8;
        if (occupied & (1_u64 << to_sq)) == 0 {
            if (to_sq / 8) == promo_rank {
                for p_piece in &[KNIGHT, BISHOP, ROOK, QUEEN] {
                    moves.push(Move {
                        from_sq,
                        to_sq,
                        promotion: Some(*p_piece),
                        is_capture: false,
                        is_en_passant: false,
                        is_double_push: false,
                        is_castling: false,
                    });
                }
            } else {
                moves.push(Move {
                    from_sq,
                    to_sq,
                    promotion: None,
                    is_capture: false,
                    is_en_passant: false,
                    is_double_push: false,
                    is_castling: false,
                });

                // Double push
                if (from_sq / 8) == start_rank {
                    let double_to_sq = (from_sq as i32 + 2 * direction) as u8;
                    if (occupied & (1_u64 << double_to_sq)) == 0 {
                        moves.push(Move {
                            from_sq,
                            to_sq: double_to_sq,
                            promotion: None,
                            is_capture: false,
                            is_en_passant: false,
                            is_double_push: true,
                            is_castling: false,
                        });
                    }
                }
            }
        }

        // Captures
        let pawn_attacks = tables.pawn_attacks[us][from_sq as usize];
        let capture_targets = pawn_attacks & their_occupied;
        for to_sq in get_squares(capture_targets) {
            if (to_sq / 8) == promo_rank {
                for p_piece in &[KNIGHT, BISHOP, ROOK, QUEEN] {
                    moves.push(Move {
                        from_sq,
                        to_sq,
                        promotion: Some(*p_piece),
                        is_capture: true,
                        is_en_passant: false,
                        is_double_push: false,
                        is_castling: false,
                    });
                }
            } else {
                moves.push(Move {
                    from_sq,
                    to_sq,
                    promotion: None,
                    is_capture: true,
                    is_en_passant: false,
                    is_double_push: false,
                    is_castling: false,
                });
            }
        }

        // En Passant
        if let Some(ep) = board.en_passant {
            let correct_rank = if us == WHITE { 5 } else { 2 };
            if (ep / 8) == correct_rank {
                if ((1_u64 << ep) & pawn_attacks) != 0 {
                    moves.push(Move {
                        from_sq,
                        to_sq: ep,
                        promotion: None,
                        is_capture: true,
                        is_en_passant: true,
                        is_double_push: false,
                        is_castling: false,
                    });
                }
            }
        }
    }

    // 2. KNIGHT MOVES
    let knights = board.pieces[knight_p];
    for from_sq in get_squares(knights) {
        let attacks = tables.knight_attacks[from_sq as usize];
        let targets = attacks & !our_occupied;
        for to_sq in get_squares(targets) {
            let is_capture = (their_occupied & (1_u64 << to_sq)) != 0;
            moves.push(Move {
                from_sq,
                to_sq,
                promotion: None,
                is_capture,
                is_en_passant: false,
                is_double_push: false,
                is_castling: false,
            });
        }
    }

    // 3. KING MOVES
    let king = board.pieces[king_p];
    if king != 0 {
        let from_sq = bit_scan_forward(king) as u8;
        let attacks = tables.king_attacks[from_sq as usize];
        let targets = attacks & !our_occupied;
        for to_sq in get_squares(targets) {
            let is_capture = (their_occupied & (1_u64 << to_sq)) != 0;
            moves.push(Move {
                from_sq,
                to_sq,
                promotion: None,
                is_capture,
                is_en_passant: false,
                is_double_push: false,
                is_castling: false,
            });
        }

        // Castling
        if us == WHITE {
            // White OO
            if (board.castling_rights & CASTLE_WHITE_OO) != 0 {
                if (occupied & ((1_u64 << 5) | (1_u64 << 6))) == 0 {
                    if !is_square_attacked(board, 4, BLACK) &&
                       !is_square_attacked(board, 5, BLACK) &&
                       !is_square_attacked(board, 6, BLACK) {
                        moves.push(Move {
                            from_sq: 4,
                            to_sq: 6,
                            promotion: None,
                            is_capture: false,
                            is_en_passant: false,
                            is_double_push: false,
                            is_castling: true,
                        });
                    }
                }
            }
            // White OOO
            if (board.castling_rights & CASTLE_WHITE_OOO) != 0 {
                if (occupied & ((1_u64 << 3) | (1_u64 << 2) | (1_u64 << 1))) == 0 {
                    if !is_square_attacked(board, 4, BLACK) &&
                       !is_square_attacked(board, 3, BLACK) &&
                       !is_square_attacked(board, 2, BLACK) {
                        moves.push(Move {
                            from_sq: 4,
                            to_sq: 2,
                            promotion: None,
                            is_capture: false,
                            is_en_passant: false,
                            is_double_push: false,
                            is_castling: true,
                        });
                    }
                }
            }
        } else {
            // Black OO
            if (board.castling_rights & CASTLE_BLACK_OO) != 0 {
                if (occupied & ((1_u64 << 61) | (1_u64 << 62))) == 0 {
                    if !is_square_attacked(board, 60, WHITE) &&
                       !is_square_attacked(board, 61, WHITE) &&
                       !is_square_attacked(board, 62, WHITE) {
                        moves.push(Move {
                            from_sq: 60,
                            to_sq: 62,
                            promotion: None,
                            is_capture: false,
                            is_en_passant: false,
                            is_double_push: false,
                            is_castling: true,
                        });
                    }
                }
            }
            // Black OOO
            if (board.castling_rights & CASTLE_BLACK_OOO) != 0 {
                if (occupied & ((1_u64 << 59) | (1_u64 << 58) | (1_u64 << 57))) == 0 {
                    if !is_square_attacked(board, 60, WHITE) &&
                       !is_square_attacked(board, 59, WHITE) &&
                       !is_square_attacked(board, 58, WHITE) {
                        moves.push(Move {
                            from_sq: 60,
                            to_sq: 58,
                            promotion: None,
                            is_capture: false,
                            is_en_passant: false,
                            is_double_push: false,
                            is_castling: true,
                        });
                    }
                }
            }
        }
    }

    // 4. SLIDING PIECES
    // Bishops
    let bishops = board.pieces[bishop_p];
    for from_sq in get_squares(bishops) {
        let attacks = get_sliding_attacks(from_sq, occupied, true);
        let targets = attacks & !our_occupied;
        for to_sq in get_squares(targets) {
            let is_capture = (their_occupied & (1_u64 << to_sq)) != 0;
            moves.push(Move {
                from_sq,
                to_sq,
                promotion: None,
                is_capture,
                is_en_passant: false,
                is_double_push: false,
                is_castling: false,
            });
        }
    }

    // Rooks
    let rooks = board.pieces[rook_p];
    for from_sq in get_squares(rooks) {
        let attacks = get_sliding_attacks(from_sq, occupied, false);
        let targets = attacks & !our_occupied;
        for to_sq in get_squares(targets) {
            let is_capture = (their_occupied & (1_u64 << to_sq)) != 0;
            moves.push(Move {
                from_sq,
                to_sq,
                promotion: None,
                is_capture,
                is_en_passant: false,
                is_double_push: false,
                is_castling: false,
            });
        }
    }

    // Queens
    let queens = board.pieces[queen_p];
    for from_sq in get_squares(queens) {
        let attacks = get_sliding_attacks(from_sq, occupied, true) | get_sliding_attacks(from_sq, occupied, false);
        let targets = attacks & !our_occupied;
        for to_sq in get_squares(targets) {
            let is_capture = (their_occupied & (1_u64 << to_sq)) != 0;
            moves.push(Move {
                from_sq,
                to_sq,
                promotion: None,
                is_capture,
                is_en_passant: false,
                is_double_push: false,
                is_castling: false,
            });
        }
    }

    moves
}

pub fn generate_legal_moves(board: &mut Board) -> Vec<Move> {
    let pseudo = generate_pseudo_legal_moves(board);
    let mut legal = Vec::with_capacity(pseudo.len());
    let us = board.side_to_move;
    let them = 1 - us;
    let king_p = if us == WHITE { WHITE_KING } else { BLACK_KING };

    for m in pseudo {
        board.make_move(m);
        let king_bb = board.pieces[king_p];
        if king_bb != 0 {
            let king_sq = bit_scan_forward(king_bb) as u8;
            if !is_square_attacked(board, king_sq, them) {
                legal.push(m);
            }
        }
        board.unmake_move(m);
    }

    legal
}
