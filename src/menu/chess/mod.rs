// Módulo do jogo de xadrez em WGPU

mod ai;
mod input;
mod moves;
mod piece;
mod render;

use std::collections::HashMap;

pub use ai::{AiMove, ChessAi};
pub use piece::{Piece, PieceColor, PieceType};

#[allow(unused_imports)]
use piece::PieceType::*;

#[derive(Clone, Copy)]
pub struct MovingPiece {
    pub piece: Piece,
    pub from_row: usize,
    pub from_col: usize,
    pub to_row: usize,
    pub to_col: usize,
    pub t: f32, // Vai de 0.0 a 1.0
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameOutcome {
    Checkmate { winner: PieceColor },
    Stalemate,
}

pub struct ChessGame {
    pub board: [[Option<Piece>; 8]; 8],
    pub selected_square: Option<(usize, usize)>,
    pub cursor: Option<(usize, usize)>, // Guarda onde a seleção do teclado está apontando (linha, coluna)
    pub current_turn: PieceColor,
    pub valid_moves: Vec<(usize, usize)>, // Lista de destinos possíveis para a peça selecionada
    pub player_color: PieceColor,
    pub promotion_pending: Option<(usize, usize)>, // Se houver um peão aguardando promoção, guarda a (linha, coluna)
    pub promotion_cursor: usize, // Índice (0 a 3) da peça selecionada na promoção via teclado
    pub moving_piece: Option<MovingPiece>,
    pub ai: ChessAi,
    pub ai_last_move_by_side: [Option<AiMove>; 2],
    pub game_outcome: Option<GameOutcome>,
    pub repetition_counts: HashMap<u64, u8>,
    pub halfmove_clock: u16,
    pub promotion_move_was_capture: bool,
    pub new_game_button_rect: Option<(f32, f32, f32, f32)>, // (x, y, w, h) for click detection
}

impl ChessGame {
    pub fn new() -> Self {
        let b = PieceColor::Black;
        let w = PieceColor::White;
        let p = |piece_type, color| Some(Piece { piece_type, color });

        let player_color = if rand::random::<bool>() {
            PieceColor::White
        } else {
            PieceColor::Black
        };
        let ai = ChessAi::new("assets/chess_ai_policy.json");

        Self {
            board: [
                // Linha 0: Peças Pretas de trás
                [
                    p(Rook, b),
                    p(Knight, b),
                    p(Bishop, b),
                    p(Queen, b),
                    p(King, b),
                    p(Bishop, b),
                    p(Knight, b),
                    p(Rook, b),
                ],
                // Linha 1: Peões Pretos
                [p(Pawn, b); 8],
                [None; 8],
                [None; 8],
                [None; 8],
                [None; 8],
                // Linha 6: Peões Brancos
                [p(Pawn, w); 8],
                // Linha 7: Peças Brancas de trás
                [
                    p(Rook, w),
                    p(Knight, w),
                    p(Bishop, w),
                    p(Queen, w),
                    p(King, w),
                    p(Bishop, w),
                    p(Knight, w),
                    p(Rook, w),
                ],
            ],
            selected_square: None,
            cursor: Some((7, 4)), // Inicia com o cursor apontando para o Rei Branco
            current_turn: PieceColor::White,
            valid_moves: Vec::new(),
            player_color,
            promotion_pending: None,
            promotion_cursor: 0,
            moving_piece: None,
            ai,
            ai_last_move_by_side: [None, None],
            game_outcome: None,
            repetition_counts: HashMap::new(),
            halfmove_clock: 0,
            promotion_move_was_capture: false,
            new_game_button_rect: None,
        }
        .with_initial_position_tracking()
    }

    fn with_initial_position_tracking(mut self) -> Self {
        let key = self.position_key();
        self.repetition_counts.insert(key, 1);
        self
    }

    fn position_key(&self) -> u64 {
        let mut h: u64 = 1469598103934665603;
        let turn_tag = match self.current_turn {
            PieceColor::White => 1_u64,
            PieceColor::Black => 2_u64,
        };
        h = (h ^ turn_tag).wrapping_mul(1099511628211);

        for r in 0..8 {
            for c in 0..8 {
                let tag = match self.board[r][c] {
                    None => 0_u64,
                    Some(piece) => {
                        let color = match piece.color {
                            PieceColor::White => 1_u64,
                            PieceColor::Black => 2_u64,
                        };
                        let role = match piece.piece_type {
                            PieceType::Pawn => 1_u64,
                            PieceType::Knight => 2_u64,
                            PieceType::Bishop => 3_u64,
                            PieceType::Rook => 4_u64,
                            PieceType::Queen => 5_u64,
                            PieceType::King => 6_u64,
                        };
                        color * 10 + role
                    }
                };
                let sq = (r * 8 + c) as u64;
                h = (h ^ (tag + sq * 17)).wrapping_mul(1099511628211);
            }
        }

        h
    }

    fn register_position_and_check_draw(&mut self, pawn_or_capture: bool) -> bool {
        if pawn_or_capture {
            self.halfmove_clock = 0;
        } else {
            self.halfmove_clock = self.halfmove_clock.saturating_add(1);
        }

        if self.halfmove_clock >= 100 {
            self.finish_draw_by_rule("regra dos 50 lances");
            return true;
        }

        let key = self.position_key();
        let next_count = self
            .repetition_counts
            .get(&key)
            .copied()
            .unwrap_or(0)
            .saturating_add(1);
        self.repetition_counts.insert(key, next_count);

        if next_count >= 3 {
            self.finish_draw_by_rule("tripla repeticao");
            return true;
        }

        false
    }

    fn finish_draw_by_rule(&mut self, rule_name: &str) {
        if self.game_outcome.is_some() {
            return;
        }

        self.game_outcome = Some(GameOutcome::Stalemate);
        println!("Empate automatico por {rule_name}.");
    }

    pub fn reset_match(&mut self) {
        let b = PieceColor::Black;
        let w = PieceColor::White;
        let p = |piece_type, color| Some(Piece { piece_type, color });

        self.board = [
            [
                p(Rook, b),
                p(Knight, b),
                p(Bishop, b),
                p(Queen, b),
                p(King, b),
                p(Bishop, b),
                p(Knight, b),
                p(Rook, b),
            ],
            [p(Pawn, b); 8],
            [None; 8],
            [None; 8],
            [None; 8],
            [None; 8],
            [p(Pawn, w); 8],
            [
                p(Rook, w),
                p(Knight, w),
                p(Bishop, w),
                p(Queen, w),
                p(King, w),
                p(Bishop, w),
                p(Knight, w),
                p(Rook, w),
            ],
        ];

        self.selected_square = None;
        self.valid_moves.clear();
        self.promotion_pending = None;
        self.promotion_cursor = 0;
        self.moving_piece = None;
        self.current_turn = PieceColor::White;
        self.game_outcome = None;
        self.ai_last_move_by_side = [None, None];
        self.halfmove_clock = 0;
        self.promotion_move_was_capture = false;
        self.repetition_counts.clear();
        let key = self.position_key();
        self.repetition_counts.insert(key, 1);
    }

    pub fn make_ai_move(&mut self) {
        if self.game_outcome.is_some() {
            return;
        }

        let actor = self.current_turn;
        let actor_idx = match actor {
            PieceColor::White => 0,
            PieceColor::Black => 1,
        };

        let legal_moves = self.collect_legal_moves_for_turn(self.current_turn);

        if legal_moves.is_empty() {
            self.finish_game_for_side(self.current_turn);
            return;
        }

        let mut was_capture = false;
        let mut was_pawn_move = false;

        if let Some(mv) = self.ai.choose_move_balanced(
            &self.board,
            actor,
            &legal_moves,
            0.0,
            0.0,
            false,
            self.ai_last_move_by_side[actor_idx],
        ) {
            let before = self.board;
            was_capture = before[mv.to_row][mv.to_col].is_some();
            was_pawn_move = matches!(
                before[mv.from_row][mv.from_col],
                Some(Piece {
                    piece_type: PieceType::Pawn,
                    ..
                })
            );
            self.ai_last_move_by_side[actor_idx] = Some(mv);
            self.apply_ai_move_to_board(mv);
        } else if let Some(mv) = self.choose_fallback_move(&legal_moves) {
            let before = self.board;
            was_capture = before[mv.to_row][mv.to_col].is_some();
            was_pawn_move = matches!(
                before[mv.from_row][mv.from_col],
                Some(Piece {
                    piece_type: PieceType::Pawn,
                    ..
                })
            );
            self.ai_last_move_by_side[actor_idx] = Some(mv);
            self.apply_ai_move_to_board(mv);
        }

        // Passa o turno de volta para o jogador
        self.current_turn = match self.current_turn {
            PieceColor::White => PieceColor::Black,
            PieceColor::Black => PieceColor::White,
        };

        if self.register_position_and_check_draw(was_capture || was_pawn_move) {
            return;
        }

        if self.is_insufficient_material_draw() {
            self.finish_draw_insufficient_material();
            return;
        }

        if self
            .collect_legal_moves_for_turn(self.current_turn)
            .is_empty()
        {
            self.finish_game_for_side(self.current_turn);
        }
    }

    pub fn collect_legal_moves_for_turn(&mut self, color: PieceColor) -> Vec<AiMove> {
        let mut all_moves = Vec::new();

        for r in 0..8 {
            for c in 0..8 {
                if let Some(piece) = self.board[r][c] {
                    if piece.color == color {
                        self.calculate_valid_moves(r, c);
                        for &(tr, tc) in &self.valid_moves {
                            all_moves.push(AiMove::new(r, c, tr, tc));
                        }
                    }
                }
            }
        }

        self.valid_moves.clear();
        all_moves
    }

    pub fn choose_fallback_move(&self, legal_moves: &[AiMove]) -> Option<AiMove> {
        use rand::seq::SliceRandom;

        let mut rng = rand::thread_rng();
        legal_moves.choose(&mut rng).copied()
    }

    pub fn apply_ai_move_to_board(&mut self, mv: AiMove) {
        if let Some(mut piece) = self.board[mv.from_row][mv.from_col] {
            if piece.piece_type == PieceType::Pawn && (mv.to_row == 0 || mv.to_row == 7) {
                piece.piece_type = PieceType::Queen;
            }

            self.board[mv.to_row][mv.to_col] = Some(piece);
            self.board[mv.from_row][mv.from_col] = None;

            self.moving_piece = Some(MovingPiece {
                piece,
                from_row: mv.from_row,
                from_col: mv.from_col,
                to_row: mv.to_row,
                to_col: mv.to_col,
                t: 0.0,
            });
        }
    }

    fn is_insufficient_material_draw(&self) -> bool {
        let mut white_non_king = Vec::new();
        let mut black_non_king = Vec::new();

        for row in 0..8 {
            for col in 0..8 {
                if let Some(piece) = self.board[row][col] {
                    if piece.piece_type == PieceType::King {
                        continue;
                    }

                    match piece.color {
                        PieceColor::White => white_non_king.push(piece.piece_type),
                        PieceColor::Black => black_non_king.push(piece.piece_type),
                    }
                }
            }
        }

        if white_non_king.is_empty() && black_non_king.is_empty() {
            return true;
        }

        let is_minor = |pt: PieceType| matches!(pt, PieceType::Bishop | PieceType::Knight);

        if white_non_king.len() == 1 && black_non_king.is_empty() && is_minor(white_non_king[0]) {
            return true;
        }

        if black_non_king.len() == 1 && white_non_king.is_empty() && is_minor(black_non_king[0]) {
            return true;
        }

        if white_non_king.len() == 1
            && black_non_king.len() == 1
            && is_minor(white_non_king[0])
            && is_minor(black_non_king[0])
        {
            return true;
        }

        false
    }

    fn finish_draw_insufficient_material(&mut self) {
        if self.game_outcome.is_some() {
            return;
        }

        self.game_outcome = Some(GameOutcome::Stalemate);
        println!("Empate por material insuficiente (afogamento tecnico).");
    }

    pub fn finish_game_for_side(&mut self, side_to_move: PieceColor) {
        if self.game_outcome.is_some() {
            return;
        }

        let in_check = self.is_king_in_check(side_to_move);
        self.game_outcome = Some(if in_check {
            let winner = match side_to_move {
                PieceColor::White => PieceColor::Black,
                PieceColor::Black => PieceColor::White,
            };
            GameOutcome::Checkmate { winner }
        } else {
            GameOutcome::Stalemate
        });

        match self.game_outcome {
            Some(GameOutcome::Checkmate { winner }) => {
                println!("Xeque-mate! Vencedor: {:?}", winner);
            }
            Some(GameOutcome::Stalemate) => {
                println!("Partida encerrada por afogamento.");
            }
            None => {}
        }
    }
}
