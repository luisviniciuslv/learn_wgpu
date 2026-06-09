use crate::menu::chess::PieceColor;

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
        if self.current_turn != self.player_color {
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
                    // Move a peça para o destino
                    self.board[row][col] = Some(piece);
                    // Apaga a peça da posição antiga
                    self.board[from_row][from_col] = None;

                    // ALTERNA O TURNO: Passa para o oponente (IA)
                    self.current_turn = match self.current_turn {
                        PieceColor::White => PieceColor::Black,
                        PieceColor::Black => PieceColor::White,
                    };
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
}
