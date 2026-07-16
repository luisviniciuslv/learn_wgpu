use rand::seq::SliceRandom;

use super::piece::{Piece, PieceColor, PieceType};

const MATE_SCORE: f32 = 50_000.0;
const SEARCH_DEPTH: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiMove {
    pub from_row: usize,
    pub from_col: usize,
    pub to_row: usize,
    pub to_col: usize,
}

impl AiMove {
    pub fn new(from_row: usize, from_col: usize, to_row: usize, to_col: usize) -> Self {
        Self {
            from_row,
            from_col,
            to_row,
            to_col,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChessAi;

impl ChessAi {
    pub fn new(_legacy_model_path: impl Into<String>) -> Self {
        Self
    }

    #[allow(clippy::too_many_arguments)]
    pub fn choose_move_balanced(
        &self,
        board: &[[Option<Piece>; 8]; 8],
        side_to_move: PieceColor,
        legal_moves: &[AiMove],
        _ai_material: f32,
        _player_material: f32,
        _autonomous_training: bool,
        previous_side_move: Option<AiMove>,
    ) -> Option<AiMove> {
        if legal_moves.is_empty() {
            return None;
        }

        let mut scored = legal_moves
            .iter()
            .copied()
            .map(|mv| {
                let mut next_board = *board;
                apply_move_on_board(&mut next_board, mv);

                let mut score = -self.negamax(
                    &next_board,
                    opposite_color(side_to_move),
                    SEARCH_DEPTH.saturating_sub(1),
                    -MATE_SCORE,
                    MATE_SCORE,
                );

                if let Some(prev) = previous_side_move {
                    if mv.from_row == prev.to_row
                        && mv.from_col == prev.to_col
                        && mv.to_row == prev.from_row
                        && mv.to_col == prev.from_col
                    {
                        score -= 0.7;
                    }
                }

                if let Some(target) = board[mv.to_row][mv.to_col] {
                    score += piece_value(target.piece_type) * 0.25;
                }

                if is_king_in_check_static(&next_board, opposite_color(side_to_move)) {
                    score += 0.4;
                }

                (mv, score)
            })
            .collect::<Vec<_>>();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let best_score = scored[0].1;
        let mut top = scored
            .iter()
            .copied()
            .take_while(|(_, s)| (best_score - *s).abs() <= 0.18)
            .collect::<Vec<_>>();

        if top.is_empty() {
            return Some(scored[0].0);
        }

        let mut rng = rand::thread_rng();
        top.shuffle(&mut rng);
        Some(top[0].0)
    }

    fn negamax(
        &self,
        board: &[[Option<Piece>; 8]; 8],
        side_to_move: PieceColor,
        depth: usize,
        mut alpha: f32,
        beta: f32,
    ) -> f32 {
        let legal_moves = collect_legal_moves_static(board, side_to_move);

        if legal_moves.is_empty() {
            if is_king_in_check_static(board, side_to_move) {
                return -MATE_SCORE + depth as f32;
            }
            return 0.0;
        }

        if depth == 0 {
            return evaluate_position(board, side_to_move);
        }

        let mut best = -MATE_SCORE;

        for mv in legal_moves {
            let mut next = *board;
            apply_move_on_board(&mut next, mv);

            let score = -self.negamax(
                &next,
                opposite_color(side_to_move),
                depth - 1,
                -beta,
                -alpha,
            );

            if score > best {
                best = score;
            }
            if score > alpha {
                alpha = score;
            }
            if alpha >= beta {
                break;
            }
        }

        best
    }
}

pub fn piece_value(piece_type: PieceType) -> f32 {
    match piece_type {
        PieceType::Pawn => 1.0,
        PieceType::Knight => 3.0,
        PieceType::Bishop => 3.1,
        PieceType::Rook => 5.0,
        PieceType::Queen => 9.0,
        PieceType::King => 100.0,
    }
}

fn evaluate_position(board: &[[Option<Piece>; 8]; 8], side_to_move: PieceColor) -> f32 {
    let own = material_score_for(board, side_to_move);
    let opp = material_score_for(board, opposite_color(side_to_move));
    let mut score = own - opp;

    let own_moves = collect_legal_moves_static(board, side_to_move).len() as f32;
    let opp_moves = collect_legal_moves_static(board, opposite_color(side_to_move)).len() as f32;
    score += (own_moves - opp_moves) * 0.04;

    if is_king_in_check_static(board, opposite_color(side_to_move)) {
        score += 0.35;
    }
    if is_king_in_check_static(board, side_to_move) {
        score -= 0.35;
    }

    score
}

fn material_score_for(board: &[[Option<Piece>; 8]; 8], color: PieceColor) -> f32 {
    let mut total = 0.0;
    for row in 0..8 {
        for col in 0..8 {
            if let Some(piece) = board[row][col] {
                if piece.color == color {
                    total += piece_value(piece.piece_type);
                }
            }
        }
    }
    total
}

fn opposite_color(color: PieceColor) -> PieceColor {
    match color {
        PieceColor::White => PieceColor::Black,
        PieceColor::Black => PieceColor::White,
    }
}

fn collect_legal_moves_static(board: &[[Option<Piece>; 8]; 8], color: PieceColor) -> Vec<AiMove> {
    let mut out = Vec::new();

    for from_row in 0..8 {
        for from_col in 0..8 {
            let Some(piece) = board[from_row][from_col] else {
                continue;
            };

            if piece.color != color {
                continue;
            }

            for (to_row, to_col) in pseudo_moves_for_piece(board, from_row, from_col, piece) {
                let mut next = *board;
                apply_move_on_board(&mut next, AiMove::new(from_row, from_col, to_row, to_col));

                if !is_king_in_check_static(&next, color) {
                    out.push(AiMove::new(from_row, from_col, to_row, to_col));
                }
            }
        }
    }

    out
}

fn pseudo_moves_for_piece(
    board: &[[Option<Piece>; 8]; 8],
    from_row: usize,
    from_col: usize,
    piece: Piece,
) -> Vec<(usize, usize)> {
    match piece.piece_type {
        PieceType::Pawn => pawn_moves(board, from_row, from_col, piece.color),
        PieceType::Knight => knight_moves(board, from_row, from_col, piece.color),
        PieceType::Bishop => sliding_moves(
            board,
            from_row,
            from_col,
            piece.color,
            &[(-1, -1), (-1, 1), (1, -1), (1, 1)],
        ),
        PieceType::Rook => sliding_moves(
            board,
            from_row,
            from_col,
            piece.color,
            &[(-1, 0), (1, 0), (0, -1), (0, 1)],
        ),
        PieceType::Queen => sliding_moves(
            board,
            from_row,
            from_col,
            piece.color,
            &[
                (-1, -1),
                (-1, 1),
                (1, -1),
                (1, 1),
                (-1, 0),
                (1, 0),
                (0, -1),
                (0, 1),
            ],
        ),
        PieceType::King => king_moves(board, from_row, from_col, piece.color),
    }
}

fn pawn_moves(
    board: &[[Option<Piece>; 8]; 8],
    from_row: usize,
    from_col: usize,
    color: PieceColor,
) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let (step, start_row) = match color {
        PieceColor::White => (-1_i32, 6_usize),
        PieceColor::Black => (1_i32, 1_usize),
    };

    let r1 = from_row as i32 + step;
    if in_bounds(r1, from_col as i32) && board[r1 as usize][from_col].is_none() {
        out.push((r1 as usize, from_col));

        let r2 = from_row as i32 + step * 2;
        if from_row == start_row
            && in_bounds(r2, from_col as i32)
            && board[r2 as usize][from_col].is_none()
        {
            out.push((r2 as usize, from_col));
        }
    }

    for dc in [-1_i32, 1_i32] {
        let tr = from_row as i32 + step;
        let tc = from_col as i32 + dc;
        if !in_bounds(tr, tc) {
            continue;
        }
        if let Some(target) = board[tr as usize][tc as usize] {
            if target.color != color {
                out.push((tr as usize, tc as usize));
            }
        }
    }

    out
}

fn knight_moves(
    board: &[[Option<Piece>; 8]; 8],
    from_row: usize,
    from_col: usize,
    color: PieceColor,
) -> Vec<(usize, usize)> {
    let deltas = [
        (-2, -1),
        (-2, 1),
        (-1, -2),
        (-1, 2),
        (1, -2),
        (1, 2),
        (2, -1),
        (2, 1),
    ];

    let mut out = Vec::new();
    for (dr, dc) in deltas {
        let tr = from_row as i32 + dr;
        let tc = from_col as i32 + dc;
        if !in_bounds(tr, tc) {
            continue;
        }

        match board[tr as usize][tc as usize] {
            None => out.push((tr as usize, tc as usize)),
            Some(target) if target.color != color => out.push((tr as usize, tc as usize)),
            _ => {}
        }
    }

    out
}

fn king_moves(
    board: &[[Option<Piece>; 8]; 8],
    from_row: usize,
    from_col: usize,
    color: PieceColor,
) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for dr in -1_i32..=1 {
        for dc in -1_i32..=1 {
            if dr == 0 && dc == 0 {
                continue;
            }
            let tr = from_row as i32 + dr;
            let tc = from_col as i32 + dc;
            if !in_bounds(tr, tc) {
                continue;
            }

            match board[tr as usize][tc as usize] {
                None => out.push((tr as usize, tc as usize)),
                Some(target) if target.color != color => out.push((tr as usize, tc as usize)),
                _ => {}
            }
        }
    }

    out
}

fn sliding_moves(
    board: &[[Option<Piece>; 8]; 8],
    from_row: usize,
    from_col: usize,
    color: PieceColor,
    directions: &[(i32, i32)],
) -> Vec<(usize, usize)> {
    let mut out = Vec::new();

    for (dr, dc) in directions {
        let mut tr = from_row as i32 + dr;
        let mut tc = from_col as i32 + dc;

        while in_bounds(tr, tc) {
            match board[tr as usize][tc as usize] {
                None => {
                    out.push((tr as usize, tc as usize));
                }
                Some(target) => {
                    if target.color != color {
                        out.push((tr as usize, tc as usize));
                    }
                    break;
                }
            }

            tr += dr;
            tc += dc;
        }
    }

    out
}

fn is_king_in_check_static(board: &[[Option<Piece>; 8]; 8], color: PieceColor) -> bool {
    let mut king_pos = None;

    for row in 0..8 {
        for col in 0..8 {
            if let Some(piece) = board[row][col] {
                if piece.color == color && piece.piece_type == PieceType::King {
                    king_pos = Some((row, col));
                    break;
                }
            }
        }
    }

    let Some((king_row, king_col)) = king_pos else {
        return false;
    };

    is_square_attacked(board, king_row, king_col, opposite_color(color))
}

fn is_square_attacked(
    board: &[[Option<Piece>; 8]; 8],
    target_row: usize,
    target_col: usize,
    attacker_color: PieceColor,
) -> bool {
    for row in 0..8 {
        for col in 0..8 {
            let Some(piece) = board[row][col] else {
                continue;
            };

            if piece.color != attacker_color {
                continue;
            }

            if piece.piece_type == PieceType::Pawn {
                let step = if attacker_color == PieceColor::White {
                    -1_i32
                } else {
                    1_i32
                };
                for dc in [-1_i32, 1_i32] {
                    let tr = row as i32 + step;
                    let tc = col as i32 + dc;
                    if tr == target_row as i32 && tc == target_col as i32 {
                        return true;
                    }
                }
                continue;
            }

            for (tr, tc) in pseudo_moves_for_piece(board, row, col, piece) {
                if tr == target_row && tc == target_col {
                    return true;
                }
            }
        }
    }

    false
}

fn apply_move_on_board(board: &mut [[Option<Piece>; 8]; 8], mv: AiMove) {
    if let Some(mut piece) = board[mv.from_row][mv.from_col] {
        if piece.piece_type == PieceType::Pawn && (mv.to_row == 0 || mv.to_row == 7) {
            piece.piece_type = PieceType::Queen;
        }

        board[mv.to_row][mv.to_col] = Some(piece);
        board[mv.from_row][mv.from_col] = None;
    }
}

fn in_bounds(row: i32, col: i32) -> bool {
    (0..8).contains(&row) && (0..8).contains(&col)
}
