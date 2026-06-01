// =============================================================================
//  Layout base e helpers de geometria
// =============================================================================

pub(super) const BASE_WIDTH: f32 = 800.0;
pub(super) const BASE_HEIGHT: f32 = 600.0;
// Proporcao do modo xadrez: menor para dar "respiro visual".
pub(super) const CHESS_TARGET_ASPECT_RATIO: f32 = 0.8;

pub(super) fn carousel_layout(vp_w: f32, vp_h: f32) -> (f32, f32, f32, f32) {
    let scale_factor = vp_w / BASE_WIDTH;
    let cx = vp_w * 0.5;
    let cy = vp_h * 0.5;

    let fator_suave = scale_factor.sqrt();
    let base_size = 120.0 * fator_suave;
    let gap = 160.0 * fator_suave;

    (cx, cy, base_size, gap)
}
