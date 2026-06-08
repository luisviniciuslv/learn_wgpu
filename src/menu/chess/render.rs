// Renderização do tabuleiro e das peças usando a spritesheet texturizada
use super::ChessGame;
use super::piece::{PieceColor, get_piece_column_index};
use crate::menu::renderer::Renderer;
use std::sync::Arc;

impl ChessGame {
    /// Renderização principal utilizando a spritesheet texturizada
    pub fn render(
        &self,
        renderer: &mut Renderer,
        vp_w: f32,
        vp_h: f32,
        pieces_bind_group: &Arc<wgpu::BindGroup>,
    ) -> anyhow::Result<()> {
        let scale_factor = vp_w.min(vp_h * 0.8) / 800.0;
        let header_h = 75.0 * scale_factor;
        let footer_h = 65.0 * scale_factor;
        let available_h = (vp_h - header_h - footer_h).max(10.0);

        let board_size = vp_w.min(available_h) * 0.92;
        let cell_size = board_size / 8.0;

        let start_x = (vp_w - board_size) * 0.5;
        let start_y = header_h + (available_h - board_size) * 0.5;

        // Borda de madeira do tabuleiro
        let border = cell_size * 0.15;
        renderer.draw_rect(
            start_x - border,
            start_y - border,
            board_size + border * 2.0,
            board_size + border * 2.0,
            [0.25, 0.15, 0.08, 1.0],
        );

        // Desenha as casas e as peças texturizadas
        for row in 0..8 {
            for col in 0..8 {
                let cell_x = start_x + col as f32 * cell_size;
                let cell_y = start_y + row as f32 * cell_size;

                let is_light = (row + col) % 2 == 0;
                let cell_color = if is_light {
                    [0.94, 0.85, 0.71, 1.0] // Bege claro
                } else {
                    [0.48, 0.35, 0.24, 1.0] // Marrom médio
                };

                renderer.draw_rect(cell_x, cell_y, cell_size, cell_size, cell_color);

                if let Some(piece) = self.board[row][col] {
                    // 1. Descobre a coluna da peça (X)
                    let col_idx = get_piece_column_index(piece.piece_type);

                    // 2. Calcula os limites horizontais (U) - agora dividido por 6
                    let u_step = 1.0 / 6.0;
                    let min_u = col_idx as f32 * u_step;
                    let max_u = (col_idx + 1) as f32 * u_step;

                    // 3. Calcula os limites verticais (V) baseado na linha (Cor)
                    // Como são 2 linhas, cada uma ocupa 0.5 (50%) da altura da textura
                    let (min_v, max_v) = match piece.color {
                        PieceColor::White => (0.0, 0.5), // Linha de cima (0% a 50%)
                        PieceColor::Black => (0.5, 1.0), // Linha de baixo (50% a 100%)
                    };

                    // Monta o quadrante de recorte [min_u, min_v, max_u, max_v]
                    let uvs = [min_u, min_v, max_u, max_v];

                    // 4. Desenha a peça na tela
                    renderer.draw_textured_rect(
                        cell_x,
                        cell_y,
                        cell_size,
                        cell_size,
                        [1.0, 1.0, 1.0, 1.0], // Mantém as cores originais
                        uvs,
                        pieces_bind_group.clone(),
                    );
                }

                if (row, col)
                    == self
                        .valid_moves
                        .iter()
                        .cloned()
                        .find(|&m| m == (row, col))
                        .unwrap_or((usize::MAX, usize::MAX))
                    && self.selected_square.is_some()
                {
                    // Desenha um círculo translúcido vermelho para indicar movimento válido
                    renderer.draw_circle(
                        cell_x + cell_size * 0.5,
                        cell_y + cell_size * 0.5,
                        cell_size * 0.15,
                        [1.0, 0.0, 0.0, 0.4], // Vermelho com 40% de opacidade
                    );
                }
            }
        }

        if let Some((sel_row, sel_col)) = self.selected_square {
            let sel_x = start_x + sel_col as f32 * cell_size;
            let sel_y = start_y + sel_row as f32 * cell_size;

            // Uma borda translúcida verde indicando seleção
            renderer.draw_rect(
                sel_x + cell_size * 0.05,
                sel_y + cell_size * 0.05,
                cell_size * 0.9,
                cell_size * 0.9,
                [0.0, 1.0, 0.0, 0.4], // Verde com 40% de opacidade
            );
        }

        // NOVO: Desenha uma borda amarela onde o CURSOR DO TECLADO está apontando
        if let Some((cur_row, cur_col)) = self.cursor {
            let cur_x = start_x + cur_col as f32 * cell_size;
            let cur_y = start_y + cur_row as f32 * cell_size;
            renderer.draw_rect(
                cur_x + cell_size * 0.15,
                cur_y + cell_size * 0.15,
                cell_size * 0.7,
                cell_size * 0.7,
                [1.0, 1.0, 0.0, 0.4], // Amarelo com 40% de opacidade
            );
        }

        Ok(())
    }
}
