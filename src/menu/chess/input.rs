use crate::menu::chess::{MovingPiece, PieceColor, PieceType};

// Lógica de entrada do usuário: mouse e teclado
use super::ChessGame;

impl ChessGame {
    /// Converte a coordenada (X, Y) do mouse para uma casa (linha, coluna) do tabuleiro
    pub fn mouse_to_square(
        &mut self,
        mx: f32,
        my: f32,
        vp_w: f32,
        vp_h: f32,
    ) -> Option<(usize, usize)> {
        self.cursor = None; // Reseta o cursor do teclado quando o mouse é usado
        // Mesma matemática usada no seu método render() para achar o tabuleiro na tela
        let scale_factor = vp_w.min(vp_h * 0.8) / 800.0;
        let header_h = 75.0 * scale_factor;
        let footer_h = 65.0 * scale_factor;
        let available_h = (vp_h - header_h - footer_h).max(10.0);
        let board_size = vp_w.min(available_h) * 0.92;
        let cell_size = board_size / 8.0;
        let start_x = (vp_w - board_size) * 0.5;
        let start_y = header_h + (available_h - board_size) * 0.5;

        // Verifica se o mouse está dentro dos limites do tabuleiro
        if mx >= start_x && mx < start_x + board_size && my >= start_y && my < start_y + board_size
        {
            let col = ((mx - start_x) / cell_size) as usize;
            let row = ((my - start_y) / cell_size) as usize;

            // Garante que fique estritamente entre 0 e 7
            if row < 8 && col < 8 {
                if self.player_color == PieceColor::Black {
                    return Some((7 - row, 7 - col));
                } else {
                    return Some((row, col));
                }
            }
        }
        None // Mouse clicou fora do tabuleiro
    }

    pub fn select_or_move(&mut self, row: usize, col: usize) {
        // Bloqueia qualquer ação se não for o turno do jogador
        if self.current_turn != self.player_color || self.game_outcome.is_some() {
            return;
        }

        // Se o jogador já tinha selecionado uma casa anteriormente
        if let Some((from_row, from_col)) = self.selected_square {
            // 1. Se ele clicou na mesma casa, ele quer desfazer a seleção
            if from_row == row && from_col == col {
                self.selected_square = None;
                self.valid_moves.clear(); // Limpa os círculos vermelhos
                return;
            }

            // 2. Se ele clicou em OUTRA casa, movemos a peça
            if let Some(piece) = self.board[from_row][from_col] {
                if self.valid_moves.contains(&(row, col)) {
                    let board_before_move = self.board;
                    let was_capture = board_before_move[row][col].is_some();
                    let was_pawn_move = piece.piece_type == PieceType::Pawn;

                    // Move a peça para o destino
                    self.board[row][col] = Some(piece);
                    // Apaga a peça da posição antiga
                    self.board[from_row][from_col] = None;

                    // Registra a animação de movimento do jogador
                    self.moving_piece = Some(MovingPiece {
                        piece,
                        from_row,
                        from_col,
                        to_row: row,
                        to_col: col,
                        t: 0.0,
                    });
                    // Verifica se ocorreu uma promoção
                    if piece.piece_type == PieceType::Pawn && (row == 0 || row == 7) {
                        self.promotion_move_was_capture = was_capture;
                        self.promotion_pending = Some((row, col));
                        self.promotion_cursor = 0;
                    } else {
                        // ALTERNA O TURNO: Passa para o oponente (IA)
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
                }
            }

            // Movimento concluído: limpamos o estado para o próximo lance
            self.selected_square = None;
            self.valid_moves.clear(); // Limpa as dicas de movimento da tela
        } else {
            // Se nenhuma casa estava selecionada, tentamos selecionar a atual
            if let Some(piece) = self.board[row][col] {
                // SÓ permite selecionar se a peça for da cor do jogador
                if piece.color == self.player_color {
                    self.selected_square = Some((row, col));
                    // Calcula os movimentos baseando-se na peça selecionada AGORA
                    self.calculate_valid_moves(row, col);
                }
            }
        }
    }

    pub fn apply_promotion(&mut self, choice_idx: usize) {
        if self.game_outcome.is_some() {
            return;
        }

        if let Some((r, c)) = self.promotion_pending {
            if let Some(mut piece) = self.board[r][c] {
                piece.piece_type = match choice_idx {
                    0 => PieceType::Queen,
                    1 => PieceType::Rook,
                    2 => PieceType::Bishop,
                    3 => PieceType::Knight,
                    _ => PieceType::Queen,
                };
                self.board[r][c] = Some(piece);
            }
            self.promotion_pending = None;

            // Alterna o turno para o oponente
            self.current_turn = match self.current_turn {
                PieceColor::White => PieceColor::Black,
                PieceColor::Black => PieceColor::White,
            };

            let pawn_or_capture = true;
            self.promotion_move_was_capture = false;

            if self.register_position_and_check_draw(pawn_or_capture) {
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
    }

    pub fn try_click_promotion(&mut self, mx: f32, my: f32, vp_w: f32, vp_h: f32) -> bool {
        if self.promotion_pending.is_none() {
            return false;
        }

        let scale_factor = vp_w.min(vp_h * 0.8) / 800.0;
        let menu_w = 400.0 * scale_factor;
        let menu_h = 120.0 * scale_factor;
        let start_x = (vp_w - menu_w) * 0.5;
        let start_y = (vp_h - menu_h) * 0.5;

        if mx >= start_x && mx < start_x + menu_w && my >= start_y && my < start_y + menu_h {
            let item_w = menu_w / 4.0;
            let choice_idx = ((mx - start_x) / item_w) as usize;
            if choice_idx < 4 {
                self.apply_promotion(choice_idx);
                return true;
            }
        }
        false
    }
}
