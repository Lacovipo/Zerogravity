// src/evaluation.rs
// Static evaluation function for ZeroGravity
use crate::board::{
    Board, WHITE, BLACK, PAWN, QUEEN, KING, WHITE_PAWN, WHITE_BISHOP, BLACK_PAWN, BLACK_BISHOP
};
use std::sync::OnceLock;

pub const MATERIAL_VALUES: [i32; 6] = [100, 320, 330, 500, 900, 20000];

pub const PASSED_PAWN_MG: [i32; 8] = [0, 10, 20, 35, 65, 110, 180, 0];
pub const PASSED_PAWN_EG: [i32; 8] = [0, 15, 30, 55, 95, 160, 260, 0];

pub const PAWN_PST: [i32; 64] = [
      0,  0,  0,  0,  0,  0,  0,  0,
     50, 50, 50, 50, 50, 50, 50, 50,
     10, 10, 20, 30, 30, 20, 10, 10,
      5,  5, 10, 25, 25, 10,  5,  5,
      0,  0,  0, 20, 20,  0,  0,  0,
      5, -5,-10,  0,  0,-10, -5,  5,
      5, 10, 10,-20,-20, 10, 10,  5,
      0,  0,  0,  0,  0,  0,  0,  0
];

pub const KNIGHT_PST: [i32; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50
];

pub const BISHOP_PST: [i32; 64] = [
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5, 10, 10,  5,  0,-10,
    -10,  5,  5, 10, 10,  5,  5,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10, 10, 10, 10, 10, 10, 10,-10,
    -10,  5,  0,  0,  0,  0,  5,-10,
    -20,-10,-10,-10,-10,-10,-10,-20
];

pub const ROOK_PST: [i32; 64] = [
      0,  0,  0,  0,  0,  0,  0,  0,
      5, 10, 10, 10, 10, 10, 10,  5,
     -5,  0,  0,  0,  0,  0,  0, -5,
     -5,  0,  0,  0,  0,  0,  0, -5,
     -5,  0,  0,  0,  0,  0,  0, -5,
     -5,  0,  0,  0,  0,  0,  0, -5,
     -5,  0,  0,  0,  0,  0,  0, -5,
      0,  0,  0,  5,  5,  0,  0,  0
];

pub const QUEEN_PST: [i32; 64] = [
    -20,-10,-10, -5, -5,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5,  5,  5,  5,  0,-10,
     -5,  0,  5,  5,  5,  5,  0, -5,
      0,  0,  5,  5,  5,  5,  0, -5,
    -10,  5,  5,  5,  5,  5,  0,-10,
    -10,  0,  5,  0,  0,  5,  0,-10,
    -20,-10,-10, -5, -5,-10,-10,-20
];

pub const KING_PST_MG: [i32; 64] = [
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -20,-30,-30,-40,-40,-30,-30,-20,
    -10,-20,-20,-20,-20,-20,-20,-10,
     20, 20,  0,  0,  0,  0, 20, 20,
     20, 30, 10,  0,  0, 10, 30, 20
];

pub const KING_PST_EG: [i32; 64] = [
    -50,-40,-30,-20,-20,-30,-40,-50,
    -30,-20,-10,  0,  0,-10,-20,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-30,  0,  0,  0,  0,-30,-30,
    -50,-30,-30,-30,-30,-30,-30,-50
];

pub const PST_TABLES: [&[i32; 64]; 5] = [
    &PAWN_PST,
    &KNIGHT_PST,
    &BISHOP_PST,
    &ROOK_PST,
    &QUEEN_PST
];

static FILE_MASKS: OnceLock<[u64; 8]> = OnceLock::new();
static PASSED_PAWN_MASKS_WHITE: OnceLock<[u64; 64]> = OnceLock::new();
static PASSED_PAWN_MASKS_BLACK: OnceLock<[u64; 64]> = OnceLock::new();

macro_rules! for_each_square {
    ($bb:expr, $sq:ident, $body:block) => {
        let mut bb = $bb;
        while bb != 0 {
            let $sq = bb.trailing_zeros() as u8;
            $body
            bb &= bb - 1;
        }
    };
}

fn get_file_masks() -> &'static [u64; 8] {
    FILE_MASKS.get_or_init(|| {
        let mut masks = [0; 8];
        for f in 0..8 {
            let mut mask = 0;
            for r in 0..8 {
                mask |= 1 << (r * 8 + f);
            }
            masks[f] = mask;
        }
        masks
    })
}

fn get_passed_pawn_masks_white() -> &'static [u64; 64] {
    PASSED_PAWN_MASKS_WHITE.get_or_init(|| {
        let mut masks = [0; 64];
        for sq in 0..64 {
            let f = sq % 8;
            let r = sq / 8;
            let mut mask_w = 0;
            for rank in (r + 1)..8 {
                for file in [f as i32 - 1, f as i32, f as i32 + 1] {
                    if file >= 0 && file <= 7 {
                        mask_w |= 1 << (rank * 8 + file as usize);
                    }
                }
            }
            masks[sq] = mask_w;
        }
        masks
    })
}

fn get_passed_pawn_masks_black() -> &'static [u64; 64] {
    PASSED_PAWN_MASKS_BLACK.get_or_init(|| {
        let mut masks = [0; 64];
        for sq in 0..64 {
            let f = sq % 8;
            let r = sq / 8;
            let mut mask_b = 0;
            for rank in 0..r {
                for file in [f as i32 - 1, f as i32, f as i32 + 1] {
                    if file >= 0 && file <= 7 {
                        mask_b |= 1 << (rank * 8 + file as usize);
                    }
                }
            }
            masks[sq] = mask_b;
        }
        masks
    })
}

pub fn evaluate(board: &Board, use_mobility: bool) -> i32 {
    // Calculate game phase and check if it is a pawn ending
    let w_knights = board.pieces[crate::board::WHITE_KNIGHT].count_ones() as i32;
    let b_knights = board.pieces[crate::board::BLACK_KNIGHT].count_ones() as i32;
    let w_bishops = board.pieces[crate::board::WHITE_BISHOP].count_ones() as i32;
    let b_bishops = board.pieces[crate::board::BLACK_BISHOP].count_ones() as i32;
    let w_rooks = board.pieces[crate::board::WHITE_ROOK].count_ones() as i32;
    let b_rooks = board.pieces[crate::board::BLACK_ROOK].count_ones() as i32;
    let w_queens = board.pieces[crate::board::WHITE_QUEEN].count_ones() as i32;
    let b_queens = board.pieces[crate::board::BLACK_QUEEN].count_ones() as i32;

    let is_pawn_ending = (w_knights | b_knights | w_bishops | b_bishops | w_rooks | b_rooks | w_queens | b_queens) == 0;
    if is_pawn_ending {
        return evaluate_pawn_ending(board);
    }

    let mut white_score = 0_i32;
    let mut black_score = 0_i32;

    let mut phase = (w_knights + b_knights) * 1 + (w_bishops + b_bishops) * 1 + (w_rooks + b_rooks) * 2 + (w_queens + b_queens) * 4;
    if phase > 24 {
        phase = 24;
    }

    // 1. Material and Positional (PST) scoring (non-king pieces)
    for piece_type in PAWN..=QUEEN {
        let val = MATERIAL_VALUES[piece_type];
        let pst = PST_TABLES[piece_type];

        // White pieces
        let w_bb = board.pieces[piece_type];
        white_score += w_bb.count_ones() as i32 * val;
        for_each_square!(w_bb, sq, {
            white_score += pst[sq as usize] / 2;
        });

        // Black pieces
        let b_bb = board.pieces[6 + piece_type];
        black_score += b_bb.count_ones() as i32 * val;
        for_each_square!(b_bb, sq, {
            // Black's perspective is vertically flipped
            black_score += pst[(sq ^ 56) as usize] / 2;
        });
    }

    // King material and tapered PST scoring
    // White King
    let w_king_bb = board.pieces[crate::board::WHITE_KING];
    white_score += w_king_bb.count_ones() as i32 * MATERIAL_VALUES[KING];
    for_each_square!(w_king_bb, sq, {
        let mg_pst = KING_PST_MG[sq as usize];
        let eg_pst = KING_PST_EG[sq as usize];
        white_score += (mg_pst * phase + eg_pst * (24 - phase)) / 24;
    });

    // Black King
    let b_king_bb = board.pieces[crate::board::BLACK_KING];
    black_score += b_king_bb.count_ones() as i32 * MATERIAL_VALUES[KING];
    for_each_square!(b_king_bb, sq, {
        let mg_pst = KING_PST_MG[(sq ^ 56) as usize];
        let eg_pst = KING_PST_EG[(sq ^ 56) as usize];
        black_score += (mg_pst * phase + eg_pst * (24 - phase)) / 24;
    });

    // 2. Bishop pair bonus (+30)
    let w_bishops = board.pieces[WHITE_BISHOP];
    if w_bishops.count_ones() >= 2 {
        white_score += 30;
    }
    let b_bishops = board.pieces[BLACK_BISHOP];
    if b_bishops.count_ones() >= 2 {
        black_score += 30;
    }

    // 2.5 Castling rights bonus (+15 OO / +10 OOO, tapered)
    let mut w_castle = 0;
    if (board.castling_rights & crate::board::CASTLE_WHITE_OO) != 0 { w_castle += 15; }
    if (board.castling_rights & crate::board::CASTLE_WHITE_OOO) != 0 { w_castle += 10; }
    white_score += (w_castle * phase) / 24;

    let mut b_castle = 0;
    if (board.castling_rights & crate::board::CASTLE_BLACK_OO) != 0 { b_castle += 15; }
    if (board.castling_rights & crate::board::CASTLE_BLACK_OOO) != 0 { b_castle += 10; }
    black_score += (b_castle * phase) / 24;

    // 3. Doubled pawns penalty (-15 per extra pawn on a file)
    let w_pawns = board.pieces[WHITE_PAWN];
    let b_pawns = board.pieces[BLACK_PAWN];
    let file_masks = get_file_masks();
    for f in 0..8 {
        let w_count = (w_pawns & file_masks[f]).count_ones() as i32;
        if w_count > 1 {
            white_score -= 15 * (w_count - 1);
        }
        let b_count = (b_pawns & file_masks[f]).count_ones() as i32;
        if b_count > 1 {
            black_score -= 15 * (b_count - 1);
        }
    }

    // 3.5 Isolated pawns penalty (-15 MG / -20 EG per isolated pawn)
    let mut w_isolated = 0;
    for_each_square!(w_pawns, sq, {
        let f = sq as usize % 8;
        let mut has_neighbor = false;
        if f > 0 && (w_pawns & file_masks[f - 1]) != 0 {
            has_neighbor = true;
        }
        if f < 7 && (w_pawns & file_masks[f + 1]) != 0 {
            has_neighbor = true;
        }
        if !has_neighbor {
            w_isolated += 1;
        }
    });
    white_score -= (w_isolated * 15 * phase + w_isolated * 20 * (24 - phase)) / 24;

    let mut b_isolated = 0;
    for_each_square!(b_pawns, sq, {
        let f = sq as usize % 8;
        let mut has_neighbor = false;
        if f > 0 && (b_pawns & file_masks[f - 1]) != 0 {
            has_neighbor = true;
        }
        if f < 7 && (b_pawns & file_masks[f + 1]) != 0 {
            has_neighbor = true;
        }
        if !has_neighbor {
            b_isolated += 1;
        }
    });
    black_score -= (b_isolated * 15 * phase + b_isolated * 20 * (24 - phase)) / 24;

    // 4. Passed pawns bonus (tapered & exponential)
    let passed_masks_w = get_passed_pawn_masks_white();
    for_each_square!(w_pawns, sq, {
        if (b_pawns & passed_masks_w[sq as usize]) == 0 {
            let r = sq as usize / 8;
            let mg_bonus = PASSED_PAWN_MG[r];
            let eg_bonus = PASSED_PAWN_EG[r];
            let mut bonus = (mg_bonus * phase + eg_bonus * (24 - phase)) / 24;

            // Protected passed pawn check
            let f = sq % 8;
            let left_defender = f > 0 && sq >= 9 && (w_pawns & (1_u64 << (sq - 9))) != 0;
            let right_defender = f < 7 && sq >= 7 && (w_pawns & (1_u64 << (sq - 7))) != 0;
            if left_defender || right_defender {
                bonus += (40 * phase + 80 * (24 - phase)) / 24;
            }

            white_score += bonus;
        }
    });

    let passed_masks_b = get_passed_pawn_masks_black();
    for_each_square!(b_pawns, sq, {
        if (w_pawns & passed_masks_b[sq as usize]) == 0 {
            let r = sq as usize / 8;
            let rel_r = 7 - r;
            let mg_bonus = PASSED_PAWN_MG[rel_r];
            let eg_bonus = PASSED_PAWN_EG[rel_r];
            let mut bonus = (mg_bonus * phase + eg_bonus * (24 - phase)) / 24;

            // Protected passed pawn check
            let f = sq % 8;
            let left_defender = f > 0 && sq <= 56 && (b_pawns & (1_u64 << (sq + 7))) != 0;
            let right_defender = f < 7 && sq <= 54 && (b_pawns & (1_u64 << (sq + 9))) != 0;
            if left_defender || right_defender {
                bonus += (40 * phase + 80 * (24 - phase)) / 24;
            }

            black_score += bonus;
        }
    });

    // 4.5 King Safety (Pawn Shield)
    // White King safety
    let w_king_bb = board.pieces[crate::board::WHITE_KING];
    if w_king_bb != 0 {
        let w_king_sq = w_king_bb.trailing_zeros() as usize;
        let w_file = w_king_sq % 8;
        let w_rank = w_king_sq / 8;
        if w_rank == 0 {
            if w_file >= 5 { // King-side: expect pawns at f2 (13), g2 (14), h2 (15)
                let shield_mask = (1_u64 << 13) | (1_u64 << 14) | (1_u64 << 15);
                let missing = 3 - (board.pieces[WHITE_PAWN] & shield_mask).count_ones() as i32;
                white_score -= (missing * 30 * phase) / 24;
            } else if w_file <= 2 { // Queen-side: expect pawns at a2 (8), b2 (9), c2 (10)
                let shield_mask = (1_u64 << 8) | (1_u64 << 9) | (1_u64 << 10);
                let missing = 3 - (board.pieces[WHITE_PAWN] & shield_mask).count_ones() as i32;
                white_score -= (missing * 30 * phase) / 24;
            }
        }
    }

    // Black King safety (fixed indices to look at rank 7: f7=53, g7=54, h7=55 and a7=48, b7=49, c7=50)
    let b_king_bb = board.pieces[crate::board::BLACK_KING];
    if b_king_bb != 0 {
        let b_king_sq = b_king_bb.trailing_zeros() as usize;
        let b_file = b_king_sq % 8;
        let b_rank = b_king_sq / 8;
        if b_rank == 7 {
            if b_file >= 5 { // King-side: expect pawns at f7 (53), g7 (54), h7 (55)
                let shield_mask = (1_u64 << 53) | (1_u64 << 54) | (1_u64 << 55);
                let missing = 3 - (board.pieces[BLACK_PAWN] & shield_mask).count_ones() as i32;
                black_score -= (missing * 30 * phase) / 24;
            } else if b_file <= 2 { // Queen-side: expect pawns at a7 (48), b7 (49), c7 (50)
                let shield_mask = (1_u64 << 48) | (1_u64 << 49) | (1_u64 << 50);
                let missing = 3 - (board.pieces[BLACK_PAWN] & shield_mask).count_ones() as i32;
                black_score -= (missing * 30 * phase) / 24;
            }
        }
    }

    // 4.5.5 King Exposure Penalty (tapered)
    // If enemy has a queen, we penalize the king for being in the center of the board in the middle game
    if phase >= 12 {
        let b_queens = board.pieces[crate::board::BLACK_QUEEN].count_ones() as i32;
        let w_queens = board.pieces[crate::board::WHITE_QUEEN].count_ones() as i32;
        // White King
        if w_king_bb != 0 {
            let w_king_sq = w_king_bb.trailing_zeros() as usize;
            let w_rank = w_king_sq / 8;
            let w_file = w_king_sq % 8;
            if b_queens > 0 {
                if w_rank == 3 || w_rank == 4 { // ranks 4 and 5 (index 3 and 4)
                    white_score -= 200;
                } else if w_rank == 2 || w_rank == 5 { // ranks 3 and 6 (index 2 and 5)
                    white_score -= 100;
                } else if (w_rank == 1 || w_rank == 6) && (w_file >= 2 && w_file <= 5) { // ranks 2 and 7 center files c..f
                    white_score -= 80;
                }
            }
        }
        // Black King
        if b_king_bb != 0 {
            let b_king_sq = b_king_bb.trailing_zeros() as usize;
            let b_rank = b_king_sq / 8;
            let b_file = b_king_sq % 8;
            if w_queens > 0 {
                if b_rank == 3 || b_rank == 4 { // ranks 4 and 5 (index 3 and 4)
                    black_score -= 200;
                } else if b_rank == 2 || b_rank == 5 { // ranks 3 and 6 (index 2 and 5)
                    black_score -= 100;
                } else if (b_rank == 1 || b_rank == 6) && (b_file >= 2 && b_file <= 5) { // ranks 2 and 7 center files c..f
                    black_score -= 80;
                }
            }
        }
    }

    // 4.6 Rook on Open / Semi-Open File
    // White rooks
    let w_rooks = board.pieces[crate::board::WHITE_ROOK];
    for_each_square!(w_rooks, sq, {
        let f = sq as usize % 8;
        let w_pawns_on_file = w_pawns & file_masks[f];
        let b_pawns_on_file = b_pawns & file_masks[f];
        if w_pawns_on_file == 0 {
            if b_pawns_on_file == 0 {
                white_score += 20;
            } else {
                white_score += 10;
            }
        }
    });

    // Black rooks
    let b_rooks = board.pieces[crate::board::BLACK_ROOK];
    for_each_square!(b_rooks, sq, {
        let f = sq as usize % 8;
        let w_pawns_on_file = w_pawns & file_masks[f];
        let b_pawns_on_file = b_pawns & file_masks[f];
        if b_pawns_on_file == 0 {
            if w_pawns_on_file == 0 {
                black_score += 20;
            } else {
                black_score += 10;
            }
        }
    });

    // 4.7 Rook on 7th Rank
    // White rooks on 7th rank (rank index 6)
    for_each_square!(w_rooks, sq, {
        if sq / 8 == 6 {
            white_score += 30;
        }
    });

    // Black rooks on 2nd rank (rank index 1)
    for_each_square!(b_rooks, sq, {
        if sq / 8 == 1 {
            black_score += 30;
        }
    });

    // 4.8 Knight Outposts
    // White knights
    let w_knights = board.pieces[crate::board::WHITE_KNIGHT];
    for_each_square!(w_knights, sq, {
        let f = sq as usize % 8;
        let r = sq as usize / 8;
        if r >= 3 && r <= 5 && f >= 2 && f <= 5 {
            let mut supported = false;
            if f > 0 && sq >= 9 && (w_pawns & (1_u64 << (sq - 9))) != 0 {
                supported = true;
            }
            if f < 7 && sq >= 7 && (w_pawns & (1_u64 << (sq - 7))) != 0 {
                supported = true;
            }
            if supported {
                white_score += 25;
            }
        }
    });

    // Black knights
    let b_knights = board.pieces[crate::board::BLACK_KNIGHT];
    for_each_square!(b_knights, sq, {
        let f = sq as usize % 8;
        let r = sq as usize / 8;
        if r >= 2 && r <= 4 && f >= 2 && f <= 5 {
            let mut supported = false;
            if f > 0 && sq <= 56 && (b_pawns & (1_u64 << (sq + 7))) != 0 {
                supported = true;
            }
            if f < 7 && sq <= 54 && (b_pawns & (1_u64 << (sq + 9))) != 0 {
                supported = true;
            }
            if supported {
                black_score += 25;
            }
        }
    });

    // 4.9 Doubled Rooks on a File
    let file_masks = get_file_masks();
    for f in 0..8 {
        let w_count = (w_rooks & file_masks[f]).count_ones() as i32;
        if w_count >= 2 {
            white_score += 20;
        }
        let b_count = (b_rooks & file_masks[f]).count_ones() as i32;
        if b_count >= 2 {
            black_score += 20;
        }
    }

    // 5. Fast Bitboard Mobility scoring
    if use_mobility {
        let tables = crate::movegen::get_move_tables();
        let occupancy = board.occupied();
        
        // White mobility
        let w_mobility_area = !board.occupied_white();
        let mut w_mobility = 0_i32;
        // Knights
        for_each_square!(board.pieces[crate::board::WHITE_KNIGHT], sq, {
            let attacks = tables.knight_attacks[sq as usize];
            w_mobility += (attacks & w_mobility_area).count_ones() as i32 * 4;
        });
        // Bishops
        for_each_square!(board.pieces[crate::board::WHITE_BISHOP], sq, {
            let attacks = crate::movegen::get_sliding_attacks(sq, occupancy, true);
            w_mobility += (attacks & w_mobility_area).count_ones() as i32 * 3;
        });
        // Rooks
        for_each_square!(board.pieces[crate::board::WHITE_ROOK], sq, {
            let attacks = crate::movegen::get_sliding_attacks(sq, occupancy, false);
            w_mobility += (attacks & w_mobility_area).count_ones() as i32 * 2;
        });
        // Queens
        for_each_square!(board.pieces[crate::board::WHITE_QUEEN], sq, {
            let attacks = crate::movegen::get_sliding_attacks(sq, occupancy, true) | 
                          crate::movegen::get_sliding_attacks(sq, occupancy, false);
            w_mobility += (attacks & w_mobility_area).count_ones() as i32 * 1;
        });
        white_score += w_mobility;

        // Black mobility
        let b_mobility_area = !board.occupied_black();
        let mut b_mobility = 0_i32;
        // Knights
        for_each_square!(board.pieces[crate::board::BLACK_KNIGHT], sq, {
            let attacks = tables.knight_attacks[sq as usize];
            b_mobility += (attacks & b_mobility_area).count_ones() as i32 * 4;
        });
        // Bishops
        for_each_square!(board.pieces[crate::board::BLACK_BISHOP], sq, {
            let attacks = crate::movegen::get_sliding_attacks(sq, occupancy, true);
            b_mobility += (attacks & b_mobility_area).count_ones() as i32 * 3;
        });
        // Rooks
        for_each_square!(board.pieces[crate::board::BLACK_ROOK], sq, {
            let attacks = crate::movegen::get_sliding_attacks(sq, occupancy, false);
            b_mobility += (attacks & b_mobility_area).count_ones() as i32 * 2;
        });
        // Queens
        for_each_square!(board.pieces[crate::board::BLACK_QUEEN], sq, {
            let attacks = crate::movegen::get_sliding_attacks(sq, occupancy, true) | 
                          crate::movegen::get_sliding_attacks(sq, occupancy, false);
            b_mobility += (attacks & b_mobility_area).count_ones() as i32 * 1;
        });
        black_score += b_mobility;
    }

    // Base score relative to side to move
    if board.side_to_move == WHITE {
        white_score - black_score
    } else {
        black_score - white_score
    }
}

fn chebyshev_distance(sq1: u8, sq2: u8) -> i32 {
    let f1 = sq1 % 8;
    let r1 = sq1 / 8;
    let f2 = sq2 % 8;
    let r2 = sq2 / 8;
    std::cmp::max((f1 as i32 - f2 as i32).abs(), (r1 as i32 - r2 as i32).abs())
}

fn king_centralization_bonus(sq: u8) -> i32 {
    let f = sq % 8;
    let r = sq / 8;
    let file_dist = (2 * (f as i32) - 7).abs();
    let rank_dist = (2 * (r as i32) - 7).abs();
    let center_dist = std::cmp::max(file_dist, rank_dist);
    (7 - center_dist) * 4
}

fn evaluate_pawn_ending(board: &Board) -> i32 {
    let w_pawns = board.pieces[crate::board::WHITE_PAWN];
    let b_pawns = board.pieces[crate::board::BLACK_PAWN];

    // King vs King draw check
    if w_pawns == 0 && b_pawns == 0 {
        return 0;
    }

    let mut white_score = 0_i32;
    let mut black_score = 0_i32;

    let w_king_bb = board.pieces[crate::board::WHITE_KING];
    let b_king_bb = board.pieces[crate::board::BLACK_KING];
    
    // Guard against missing kings in illegal/test positions
    if w_king_bb == 0 || b_king_bb == 0 {
        return 0;
    }

    let w_king_sq = w_king_bb.trailing_zeros() as u8;
    let b_king_sq = b_king_bb.trailing_zeros() as u8;

    // 1. King Centralization
    white_score += king_centralization_bonus(w_king_sq);
    black_score += king_centralization_bonus(b_king_sq);

    // 2. Direct Opposition
    let w_f = w_king_sq % 8;
    let w_r = w_king_sq / 8;
    let b_f = b_king_sq % 8;
    let b_r = b_king_sq / 8;
    let file_dist = (w_f as i32 - b_f as i32).abs();
    let rank_dist = (w_r as i32 - b_r as i32).abs();
    if (file_dist == 0 && rank_dist == 2) || (rank_dist == 0 && file_dist == 2) {
        if board.side_to_move == WHITE {
            black_score += 30; // Black has opposition
        } else {
            white_score += 30; // White has opposition
        }
    }

    // 3. Pawn material, structure and quality
    let file_masks = get_file_masks();
    let passed_masks_w = get_passed_pawn_masks_white();
    let passed_masks_b = get_passed_pawn_masks_black();

    // White pawns
    let mut temp_w = w_pawns;
    while temp_w != 0 {
        let sq = temp_w.trailing_zeros() as u8;
        let r = sq / 8;
        let f = sq % 8;

        white_score += 100; // Base material

        // King proximity (defense)
        let dist_to_king = chebyshev_distance(w_king_sq, sq);
        white_score += (8 - dist_to_king) * 5;

        // Enemy king proximity (safety/distance)
        let dist_to_enemy_king = chebyshev_distance(b_king_sq, sq);
        white_score -= (8 - dist_to_enemy_king) * 5;

        // Passed pawn check
        if (b_pawns & passed_masks_w[sq as usize]) == 0 {
            let mut passed_bonus = 50 + 15 * r as i32;

            // Protected passed pawn check
            let left_defender = f > 0 && sq >= 9 && (w_pawns & (1_u64 << (sq - 9))) != 0;
            let right_defender = f < 7 && sq >= 7 && (w_pawns & (1_u64 << (sq - 7))) != 0;
            if left_defender || right_defender {
                passed_bonus += 150;
            }

            // Outside passed pawn check
            let mut is_outside = true;
            let mut other_pawns = (w_pawns | b_pawns) & !(1_u64 << sq);
            while other_pawns != 0 {
                let other_sq = other_pawns.trailing_zeros() as u8;
                let other_f = other_sq % 8;
                if (other_f as i32 - f as i32).abs() < 2 {
                    is_outside = false;
                    break;
                }
                other_pawns &= other_pawns - 1;
            }
            if is_outside {
                passed_bonus += 120;
            }

            // Rule of the square check
            let d = 7 - r as i32;
            let d_eff = if r == 1 { 5 } else { d };
            let king_dist_to_promo = std::cmp::max((7 - b_r as i32).abs(), (f as i32 - b_f as i32).abs());
            let limit = if board.side_to_move == WHITE { d_eff - 1 } else { d_eff };
            if king_dist_to_promo > limit {
                let file_mask = file_masks[f as usize];
                let rank_mask_white = !((1_u64 << ((r + 1) * 8)) - 1);
                let front_mask = file_mask & rank_mask_white;
                if (board.occupied() & front_mask) == 0 {
                    passed_bonus += 800; // Unstoppable!
                }
            }

            white_score += passed_bonus;
        }

        temp_w &= temp_w - 1;
    }

    // Black pawns
    let mut temp_b = b_pawns;
    while temp_b != 0 {
        let sq = temp_b.trailing_zeros() as u8;
        let r = sq / 8;
        let f = sq % 8;

        black_score += 100; // Base material

        // King proximity (defense)
        let dist_to_king = chebyshev_distance(b_king_sq, sq);
        black_score += (8 - dist_to_king) * 5;

        // Enemy king proximity (safety/distance)
        let dist_to_enemy_king = chebyshev_distance(w_king_sq, sq);
        black_score -= (8 - dist_to_enemy_king) * 5;

        // Passed pawn check
        if (w_pawns & passed_masks_b[sq as usize]) == 0 {
            let mut passed_bonus = 50 + 15 * (7 - r as i32);

            // Protected passed pawn check
            let left_defender = f > 0 && sq <= 56 && (b_pawns & (1_u64 << (sq + 7))) != 0;
            let right_defender = f < 7 && sq <= 54 && (b_pawns & (1_u64 << (sq + 9))) != 0;
            if left_defender || right_defender {
                passed_bonus += 150;
            }

            // Outside passed pawn check
            let mut is_outside = true;
            let mut other_pawns = (w_pawns | b_pawns) & !(1_u64 << sq);
            while other_pawns != 0 {
                let other_sq = other_pawns.trailing_zeros() as u8;
                let other_f = other_sq % 8;
                if (other_f as i32 - f as i32).abs() < 2 {
                    is_outside = false;
                    break;
                }
                other_pawns &= other_pawns - 1;
            }
            if is_outside {
                passed_bonus += 120;
            }

            // Rule of the square check
            let d = r as i32;
            let d_eff = if r == 6 { 5 } else { d };
            let king_dist_to_promo = std::cmp::max((w_r as i32).abs(), (f as i32 - w_f as i32).abs());
            let limit = if board.side_to_move == BLACK { d_eff - 1 } else { d_eff };
            if king_dist_to_promo > limit {
                let file_mask = file_masks[f as usize];
                let rank_mask_black = (1_u64 << (r * 8)) - 1;
                let front_mask = file_mask & rank_mask_black;
                if (board.occupied() & front_mask) == 0 {
                    passed_bonus += 800; // Unstoppable!
                }
            }

            black_score += passed_bonus;
        }

        temp_b &= temp_b - 1;
    }

    // 4. Doubled pawns penalty
    for f in 0..8 {
        let w_count = (w_pawns & file_masks[f]).count_ones() as i32;
        if w_count > 1 {
            white_score -= 20 * (w_count - 1);
        }
        let b_count = (b_pawns & file_masks[f]).count_ones() as i32;
        if b_count > 1 {
            black_score -= 20 * (b_count - 1);
        }
    }

    // 5. King proximity to enemy pawns (offensive)
    // White king attacking black pawns
    let mut temp_b_att = b_pawns;
    while temp_b_att != 0 {
        let sq = temp_b_att.trailing_zeros() as u8;
        let dist = chebyshev_distance(w_king_sq, sq);
        white_score += (8 - dist) * 10;
        temp_b_att &= temp_b_att - 1;
    }

    // Black king attacking white pawns
    let mut temp_w_att = w_pawns;
    while temp_w_att != 0 {
        let sq = temp_w_att.trailing_zeros() as u8;
        let dist = chebyshev_distance(b_king_sq, sq);
        black_score += (8 - dist) * 10;
        temp_w_att &= temp_w_att - 1;
    }

    if board.side_to_move == WHITE {
        white_score - black_score
    } else {
        black_score - white_score
    }
}

pub fn print_eval(board: &Board) {
    let w_knights = board.pieces[crate::board::WHITE_KNIGHT].count_ones() as i32;
    let b_knights = board.pieces[crate::board::BLACK_KNIGHT].count_ones() as i32;
    let w_bishops = board.pieces[crate::board::WHITE_BISHOP].count_ones() as i32;
    let b_bishops = board.pieces[crate::board::BLACK_BISHOP].count_ones() as i32;
    let w_rooks = board.pieces[crate::board::WHITE_ROOK].count_ones() as i32;
    let b_rooks = board.pieces[crate::board::BLACK_ROOK].count_ones() as i32;
    let w_queens = board.pieces[crate::board::WHITE_QUEEN].count_ones() as i32;
    let b_queens = board.pieces[crate::board::BLACK_QUEEN].count_ones() as i32;

    let is_pawn_ending = (w_knights | b_knights | w_bishops | b_bishops | w_rooks | b_rooks | w_queens | b_queens) == 0;
    println!("--------------------------------------------------");
    println!("           ZeroGravity Static Evaluation          ");
    println!("--------------------------------------------------");
    
    if is_pawn_ending {
        println!("Type: Pawn Ending");
        let score = evaluate_pawn_ending(board);
        println!("Final Score: {:.2} cp (Side to move perspective)", score as f32 / 100.0);
        return;
    }

    let mut phase = (w_knights + b_knights) * 1 + (w_bishops + b_bishops) * 1 + (w_rooks + b_rooks) * 2 + (w_queens + b_queens) * 4;
    if phase > 24 { phase = 24; }
    println!("Game Phase (0=EG, 24=MG): {}", phase);

    // Term variables
    let mut w_mat = 0; let mut b_mat = 0;
    let mut w_pst = 0; let mut b_pst = 0;
    for piece_type in crate::board::PAWN..=crate::board::QUEEN {
        let val = MATERIAL_VALUES[piece_type];
        let pst = PST_TABLES[piece_type];

        let w_bb = board.pieces[piece_type];
        w_mat += w_bb.count_ones() as i32 * val;
        for_each_square!(w_bb, sq, { w_pst += pst[sq as usize] / 2; });

        let b_bb = board.pieces[6 + piece_type];
        b_mat += b_bb.count_ones() as i32 * val;
        for_each_square!(b_bb, sq, { b_pst += pst[(sq ^ 56) as usize] / 2; });
    }

    // King material & PST
    let w_king_bb = board.pieces[crate::board::WHITE_KING];
    w_mat += w_king_bb.count_ones() as i32 * MATERIAL_VALUES[crate::board::KING];
    let mut w_king_pst = 0;
    for_each_square!(w_king_bb, sq, {
        let mg = KING_PST_MG[sq as usize];
        let eg = KING_PST_EG[sq as usize];
        w_king_pst += (mg * phase + eg * (24 - phase)) / 24;
    });

    let b_king_bb = board.pieces[crate::board::BLACK_KING];
    b_mat += b_king_bb.count_ones() as i32 * MATERIAL_VALUES[crate::board::KING];
    let mut b_king_pst = 0;
    for_each_square!(b_king_bb, sq, {
        let mg = KING_PST_MG[(sq ^ 56) as usize];
        let eg = KING_PST_EG[(sq ^ 56) as usize];
        b_king_pst += (mg * phase + eg * (24 - phase)) / 24;
    });

    // Bishop pair
    let mut w_bp = 0; if w_bishops >= 2 { w_bp = 30; }
    let mut b_bp = 0; if b_bishops >= 2 { b_bp = 30; }

    // Castling rights
    let mut w_castle_bonus = 0;
    if (board.castling_rights & crate::board::CASTLE_WHITE_OO) != 0 { w_castle_bonus += 15; }
    if (board.castling_rights & crate::board::CASTLE_WHITE_OOO) != 0 { w_castle_bonus += 10; }
    let w_castle = (w_castle_bonus * phase) / 24;

    let mut b_castle_bonus = 0;
    if (board.castling_rights & crate::board::CASTLE_BLACK_OO) != 0 { b_castle_bonus += 15; }
    if (board.castling_rights & crate::board::CASTLE_BLACK_OOO) != 0 { b_castle_bonus += 10; }
    let b_castle = (b_castle_bonus * phase) / 24;

    // Doubled pawns
    let w_pawns = board.pieces[crate::board::WHITE_PAWN];
    let b_pawns = board.pieces[crate::board::BLACK_PAWN];
    let file_masks = get_file_masks();
    let mut w_doubled = 0;
    let mut b_doubled = 0;
    for f in 0..8 {
        let w_c = (w_pawns & file_masks[f]).count_ones() as i32;
        if w_c > 1 { w_doubled += 15 * (w_c - 1); }
        let b_c = (b_pawns & file_masks[f]).count_ones() as i32;
        if b_c > 1 { b_doubled += 15 * (b_c - 1); }
    }

    // Isolated pawns
    let mut w_isolated_count = 0;
    for_each_square!(w_pawns, sq, {
        let f = sq as usize % 8;
        let mut has_neighbor = false;
        if f > 0 && (w_pawns & file_masks[f - 1]) != 0 { has_neighbor = true; }
        if f < 7 && (w_pawns & file_masks[f + 1]) != 0 { has_neighbor = true; }
        if !has_neighbor { w_isolated_count += 1; }
    });
    let w_isolated = (w_isolated_count * 15 * phase + w_isolated_count * 20 * (24 - phase)) / 24;

    let mut b_isolated_count = 0;
    for_each_square!(b_pawns, sq, {
        let f = sq as usize % 8;
        let mut has_neighbor = false;
        if f > 0 && (b_pawns & file_masks[f - 1]) != 0 { has_neighbor = true; }
        if f < 7 && (b_pawns & file_masks[f + 1]) != 0 { has_neighbor = true; }
        if !has_neighbor { b_isolated_count += 1; }
    });
    let b_isolated = (b_isolated_count * 15 * phase + b_isolated_count * 20 * (24 - phase)) / 24;

    // Passed pawns & Protected passed pawns
    let mut w_passed = 0;
    let passed_masks_w = get_passed_pawn_masks_white();
    for_each_square!(w_pawns, sq, {
        if (b_pawns & passed_masks_w[sq as usize]) == 0 {
            let r = sq as usize / 8;
            let mg_bonus = PASSED_PAWN_MG[r];
            let eg_bonus = PASSED_PAWN_EG[r];
            w_passed += (mg_bonus * phase + eg_bonus * (24 - phase)) / 24;

            let f = sq % 8;
            let left_defender = f > 0 && sq >= 9 && (w_pawns & (1_u64 << (sq - 9))) != 0;
            let right_defender = f < 7 && sq >= 7 && (w_pawns & (1_u64 << (sq - 7))) != 0;
            if left_defender || right_defender {
                w_passed += (40 * phase + 80 * (24 - phase)) / 24;
            }
        }
    });

    let mut b_passed = 0;
    let passed_masks_b = get_passed_pawn_masks_black();
    for_each_square!(b_pawns, sq, {
        if (w_pawns & passed_masks_b[sq as usize]) == 0 {
            let r = sq as usize / 8;
            let rel_r = 7 - r;
            let mg_bonus = PASSED_PAWN_MG[rel_r];
            let eg_bonus = PASSED_PAWN_EG[rel_r];
            b_passed += (mg_bonus * phase + eg_bonus * (24 - phase)) / 24;

            let f = sq % 8;
            let left_defender = f > 0 && sq <= 56 && (b_pawns & (1_u64 << (sq + 7))) != 0;
            let right_defender = f < 7 && sq <= 54 && (b_pawns & (1_u64 << (sq + 9))) != 0;
            if left_defender || right_defender {
                b_passed += (40 * phase + 80 * (24 - phase)) / 24;
            }
        }
    });

    // King safety Pawn shield
    let mut w_shield = 0;
    if w_king_bb != 0 {
        let w_king_sq = w_king_bb.trailing_zeros() as usize;
        let w_file = w_king_sq % 8;
        let w_rank = w_king_sq / 8;
        if w_rank == 0 {
            if w_file >= 5 {
                let shield_mask = (1_u64 << 13) | (1_u64 << 14) | (1_u64 << 15);
                let missing = 3 - (board.pieces[crate::board::WHITE_PAWN] & shield_mask).count_ones() as i32;
                w_shield = (missing * 30 * phase) / 24;
            } else if w_file <= 2 {
                let shield_mask = (1_u64 << 8) | (1_u64 << 9) | (1_u64 << 10);
                let missing = 3 - (board.pieces[crate::board::WHITE_PAWN] & shield_mask).count_ones() as i32;
                w_shield = (missing * 30 * phase) / 24;
            }
        }
    }

    let mut b_shield = 0;
    if b_king_bb != 0 {
        let b_king_sq = b_king_bb.trailing_zeros() as usize;
        let b_file = b_king_sq % 8;
        let b_rank = b_king_sq / 8;
        if b_rank == 7 {
            if b_file >= 5 {
                let shield_mask = (1_u64 << 53) | (1_u64 << 54) | (1_u64 << 55);
                let missing = 3 - (board.pieces[crate::board::BLACK_PAWN] & shield_mask).count_ones() as i32;
                b_shield = (missing * 30 * phase) / 24;
            } else if b_file <= 2 {
                let shield_mask = (1_u64 << 48) | (1_u64 << 49) | (1_u64 << 50);
                let missing = 3 - (board.pieces[crate::board::BLACK_PAWN] & shield_mask).count_ones() as i32;
                b_shield = (missing * 30 * phase) / 24;
            }
        }
    }

    // King exposure safety
    let mut w_exposure = 0;
    let mut b_exposure = 0;
    if phase >= 12 {
        if w_king_bb != 0 {
            let w_king_sq = w_king_bb.trailing_zeros() as usize;
            let w_rank = w_king_sq / 8;
            let w_file = w_king_sq % 8;
            if b_queens > 0 {
                if w_rank == 3 || w_rank == 4 { w_exposure = 200; }
                else if w_rank == 2 || w_rank == 5 { w_exposure = 100; }
                else if (w_rank == 1 || w_rank == 6) && (w_file >= 2 && w_file <= 5) { w_exposure = 80; }
            }
        }
        if b_king_bb != 0 {
            let b_king_sq = b_king_bb.trailing_zeros() as usize;
            let b_rank = b_king_sq / 8;
            let b_file = b_king_sq % 8;
            if w_queens > 0 {
                if b_rank == 3 || b_rank == 4 { b_exposure = 200; }
                else if b_rank == 2 || b_rank == 5 { b_exposure = 100; }
                else if (b_rank == 1 || b_rank == 6) && (b_file >= 2 && b_file <= 5) { b_exposure = 80; }
            }
        }
    }

    // Rook open / semi-open files
    let mut w_rook_files = 0;
    for_each_square!(w_rooks, sq, {
        let f = sq as usize % 8;
        let w_pawns_on_file = w_pawns & file_masks[f];
        let b_pawns_on_file = b_pawns & file_masks[f];
        if w_pawns_on_file == 0 {
            if b_pawns_on_file == 0 { w_rook_files += 20; } else { w_rook_files += 10; }
        }
    });

    let mut b_rook_files = 0;
    for_each_square!(b_rooks, sq, {
        let f = sq as usize % 8;
        let w_pawns_on_file = w_pawns & file_masks[f];
        let b_pawns_on_file = b_pawns & file_masks[f];
        if b_pawns_on_file == 0 {
            if w_pawns_on_file == 0 { b_rook_files += 20; } else { b_rook_files += 10; }
        }
    });

    // Rook on 7th rank
    let mut w_rook_7th = 0;
    for_each_square!(w_rooks, sq, { if sq / 8 == 6 { w_rook_7th += 30; } });
    let mut b_rook_7th = 0;
    for_each_square!(b_rooks, sq, { if sq / 8 == 1 { b_rook_7th += 30; } });

    // Knight outposts
    let mut w_outpost = 0;
    for_each_square!(w_knights, sq, {
        let f = sq as usize % 8;
        let r = sq as usize / 8;
        if r >= 3 && r <= 5 && f >= 2 && f <= 5 {
            let mut supported = false;
            if f > 0 && sq >= 9 && (w_pawns & (1_u64 << (sq - 9))) != 0 { supported = true; }
            if f < 7 && sq >= 7 && (w_pawns & (1_u64 << (sq - 7))) != 0 { supported = true; }
            if supported { w_outpost += 25; }
        }
    });

    let mut b_outpost = 0;
    for_each_square!(b_knights, sq, {
        let f = sq as usize % 8;
        let r = sq as usize / 8;
        if r >= 2 && r <= 4 && f >= 2 && f <= 5 {
            let mut supported = false;
            if f > 0 && sq <= 56 && (b_pawns & (1_u64 << (sq + 7))) != 0 { supported = true; }
            if f < 7 && sq <= 54 && (b_pawns & (1_u64 << (sq + 9))) != 0 { supported = true; }
            if supported { b_outpost += 25; }
        }
    });

    // Doubled rooks
    let mut w_doubled_rooks = 0;
    for f in 0..8 {
        if (board.pieces[crate::board::WHITE_ROOK] & file_masks[f]).count_ones() >= 2 { w_doubled_rooks += 20; }
    }
    let mut b_doubled_rooks = 0;
    for f in 0..8 {
        if (board.pieces[crate::board::BLACK_ROOK] & file_masks[f]).count_ones() >= 2 { b_doubled_rooks += 20; }
    }

    // Print values
    let print_term = |name: &str, w: i32, b: i32| {
        println!("{:20} | {:9.2} | {:9.2} | {:9.2}", name, w as f32 / 100.0, b as f32 / 100.0, (w - b) as f32 / 100.0);
    };

    println!("Component            |   White   |   Black   |   Total");
    println!("--------------------------------------------------");
    print_term("Material (Base)", w_mat, b_mat);
    print_term("PST (Positional)", w_pst, b_pst);
    print_term("King PST", w_king_pst, b_king_pst);
    print_term("Bishop Pair", w_bp, b_bp);
    print_term("Castling Rights", w_castle, b_castle);
    print_term("Doubled Pawns", -w_doubled, -b_doubled);
    print_term("Isolated Pawns", -w_isolated, -b_isolated);
    print_term("Passed Pawns", w_passed, b_passed);
    print_term("King Shield Safety", -w_shield, -b_shield);
    print_term("King Exposure", -w_exposure, -b_exposure);
    print_term("Rook Files", w_rook_files, b_rook_files);
    print_term("Rook on 7th", w_rook_7th, b_rook_7th);
    print_term("Knight Outposts", w_outpost, b_outpost);
    print_term("Doubled Rooks", w_doubled_rooks, b_doubled_rooks);
    
    let w_total = w_mat + w_pst + w_king_pst + w_bp + w_castle - w_doubled - w_isolated + w_passed - w_shield - w_exposure + w_rook_files + w_rook_7th + w_outpost + w_doubled_rooks;
    let b_total = b_mat + b_pst + b_king_pst + b_bp + b_castle - b_doubled - b_isolated + b_passed - b_shield - b_exposure + b_rook_files + b_rook_7th + b_outpost + b_doubled_rooks;
    println!("--------------------------------------------------");
    print_term("Total Score", w_total, b_total);
    println!("--------------------------------------------------");
    
    let raw_eval = w_total - b_total;
    let perspective_eval = if board.side_to_move == WHITE { raw_eval } else { -raw_eval };
    println!("Evaluation (White POV):   {:+.2} cp", raw_eval as f32 / 100.0);
    println!("Evaluation (Active Side): {:+.2} cp", perspective_eval as f32 / 100.0);
    println!("--------------------------------------------------");
}
