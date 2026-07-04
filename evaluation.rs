// src/evaluation.rs
// Static evaluation function for ZeroGravity
use crate::board::{
    Board, WHITE, PAWN, QUEEN, KING, WHITE_PAWN, WHITE_BISHOP, BLACK_PAWN, BLACK_BISHOP
};
use std::sync::OnceLock;

pub const MATERIAL_VALUES: [i32; 6] = [100, 320, 330, 500, 900, 20000];

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

pub fn evaluate(board: &Board, _use_mobility: bool) -> i32 {
    let mut white_score = 0_i32;
    let mut black_score = 0_i32;

    // Calculate game phase
    let w_knights = board.pieces[crate::board::WHITE_KNIGHT].count_ones() as i32;
    let b_knights = board.pieces[crate::board::BLACK_KNIGHT].count_ones() as i32;
    let w_bishops = board.pieces[crate::board::WHITE_BISHOP].count_ones() as i32;
    let b_bishops = board.pieces[crate::board::BLACK_BISHOP].count_ones() as i32;
    let w_rooks = board.pieces[crate::board::WHITE_ROOK].count_ones() as i32;
    let b_rooks = board.pieces[crate::board::BLACK_ROOK].count_ones() as i32;
    let w_queens = board.pieces[crate::board::WHITE_QUEEN].count_ones() as i32;
    let b_queens = board.pieces[crate::board::BLACK_QUEEN].count_ones() as i32;

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
            white_score += pst[sq as usize];
        });

        // Black pieces
        let b_bb = board.pieces[6 + piece_type];
        black_score += b_bb.count_ones() as i32 * val;
        for_each_square!(b_bb, sq, {
            // Black's perspective is vertically flipped
            black_score += pst[(sq ^ 56) as usize];
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

    // 4. Passed pawns bonus (+10 * rank)
    let passed_masks_w = get_passed_pawn_masks_white();
    for_each_square!(w_pawns, sq, {
        if (b_pawns & passed_masks_w[sq as usize]) == 0 {
            let r = sq as i32 / 8;
            white_score += 10 * r;
        }
    });
    let passed_masks_b = get_passed_pawn_masks_black();
    for_each_square!(b_pawns, sq, {
        if (w_pawns & passed_masks_b[sq as usize]) == 0 {
            let r = sq as i32 / 8;
            black_score += 10 * (7 - r);
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
            if f > 0 && (w_pawns & (1 << (sq - 9))) != 0 {
                supported = true;
            }
            if f < 7 && (w_pawns & (1 << (sq - 7))) != 0 {
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
            if f > 0 && (b_pawns & (1 << (sq + 7))) != 0 {
                supported = true;
            }
            if f < 7 && (b_pawns & (1 << (sq + 9))) != 0 {
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

    // Base score relative to side to move
    if board.side_to_move == WHITE {
        white_score - black_score
    } else {
        black_score - white_score
    }
}
