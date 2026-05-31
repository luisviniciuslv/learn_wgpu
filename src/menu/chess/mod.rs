// Módulo do jogo de xadrez em WGPU
use crate::menu::renderer::Renderer;

pub struct ChessGame {
    // Adicione estado do jogo aqui se necessário
}

impl ChessGame {
    pub fn new() -> Self {
        Self {}
    }

    pub fn render(&self, renderer: &mut Renderer, vp_w: f32, vp_h: f32) -> anyhow::Result<()> {
        // Calcula a escala lógica baseada na largura
        let scale_factor = vp_w.min(vp_h * 0.8) / 800.0;

        // Define espaços dedicados e proporcionais para o Header e Footer
        let header_h = 75.0 * scale_factor;
        let footer_h = 65.0 * scale_factor;
        let available_h = (vp_h - header_h - footer_h).max(10.0);

        // O tabuleiro se ajusta ao espaço restante (largura ou altura útil)
        let board_size = vp_w.min(available_h) * 0.92;
        let cell_size = board_size / 8.0;

        // Centraliza horizontalmente e verticalmente dentro do espaço útil restante
        let start_x = (vp_w - board_size) * 0.5;
        let start_y = header_h + (available_h - board_size) * 0.5;

        // Fundo do tabuleiro (borda marrom escuro/madeira)
        let border = cell_size * 0.15;
        renderer.draw_rect(
            start_x - border,
            start_y - border,
            board_size + border * 2.0,
            board_size + border * 2.0,
            [0.25, 0.15, 0.08, 1.0], // Marrom escuro
        );

        // Desenha as 64 casas
        for row in 0..8 {
            for col in 0..8 {
                let cell_x = start_x + col as f32 * cell_size;
                let cell_y = start_y + row as f32 * cell_size;

                // Alterna cores (claras e escuras)
                let is_light = (row + col) % 2 == 0;
                let color = if is_light {
                    [0.94, 0.85, 0.71, 1.0] // Bege claro
                } else {
                    [0.48, 0.35, 0.24, 1.0] // Marrom médio
                };

                renderer.draw_rect(cell_x, cell_y, cell_size, cell_size, color);
            }
        }

        Ok(())
    }
}
