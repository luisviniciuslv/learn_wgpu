use super::super::renderer::Renderer;
use super::super::types::ArrowId;

/// Desenha um botao de navegacao como uma seta vetorial (haste + cabeca).
/// Geometria em pixels logicos, relativa ao canto superior esquerdo (bx, by).
pub(super) fn desenhar_seta_botao(
    renderer: &mut Renderer,
    id: ArrowId,
    bx: f32,
    by: f32,
    bw: f32,
    bh: f32,
    color: [f32; 4],
) {
    let cx = bx + bw * 0.5;
    let cy = by + bh * 0.5;

    match id {
        ArrowId::Right => {
            renderer.draw_rect(bx + bw * 0.08, cy - bh * 0.14, bw * 0.50, bh * 0.28, color);
            renderer.draw_triangle(
                [cx - bw * 0.02, by + bh * 0.08],
                [cx - bw * 0.02, by + bh * 0.92],
                [bx + bw * 0.94, cy],
                color,
            );
        }
        ArrowId::Left | ArrowId::Back => {
            renderer.draw_rect(cx + bw * 0.02, cy - bh * 0.14, bw * 0.50, bh * 0.28, color);
            renderer.draw_triangle(
                [cx + bw * 0.02, by + bh * 0.08],
                [cx + bw * 0.02, by + bh * 0.92],
                [bx + bw * 0.06, cy],
                color,
            );
        }
    }
}
