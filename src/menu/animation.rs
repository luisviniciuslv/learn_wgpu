// Funções matemáticas puras para animações de transição.
// Não dependem de nenhum outro módulo do projeto.

/// Interpolação linear entre dois valores.
/// - `t = 0.0` → retorna `a`
/// - `t = 1.0` → retorna `b`
/// - `t = 0.5` → retorna o ponto médio
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Suavização de movimento (Ease Out quadrático):
/// começa rápido e desacelera suavemente até parar.
pub fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t) * (1.0 - t)
}
