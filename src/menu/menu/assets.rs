use std::sync::Arc;

use super::super::renderer::{Renderer, Texture, carregar_png_ou_fallback, rasterizar_texto};
use super::super::types::MenuItem;

// =============================================================================
//  Texturas e rotulos do menu (geradas uma vez e reutilizadas)
// =============================================================================

pub(super) struct IconeItem {
    /// Bind group da textura GPU deste item, compartilhado via Arc
    pub(super) bind_group: Arc<wgpu::BindGroup>,
    /// Bind group da textura do rotulo de texto (label)
    pub(super) label_bind_group: Arc<wgpu::BindGroup>,
    /// Dimensoes do rotulo em pixels logicos
    pub(super) label_w: f32,
    pub(super) label_h: f32,
}

/// Carrega a fonte do sistema. Se nenhuma for encontrada, usa a fonte embutida.
pub(super) fn carregar_fonte() -> ab_glyph::FontArc {
    let candidatas = [
        // Local (pasta assets/ do projeto) tem prioridade
        "assets/font.ttf",
        // Windows
        "C:/Windows/Fonts/segoeui.ttf",
        "C:/Windows/Fonts/arial.ttf",
        // macOS
        "/System/Library/Fonts/Helvetica.ttc",
        "/Library/Fonts/Arial.ttf",
        // Linux
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    ];

    for caminho in &candidatas {
        if let Ok(dados) = std::fs::read(caminho) {
            if let Ok(font) = ab_glyph::FontArc::try_from_vec(dados) {
                return font;
            }
        }
    }

    panic!(
        "Nenhuma fonte encontrada. Coloque um arquivo .ttf em: assets/font.ttf\n\
         Baixe qualquer fonte gratuita (ex: Inter, Roboto) e salve com esse nome."
    )
}

/// Gera icones e rotulos de texto para uma lista de itens de menu.
pub(super) fn gerar_icones(
    items: &[MenuItem],
    renderer: &Renderer,
    font: &ab_glyph::FontArc,
) -> Vec<IconeItem> {
    let cores_fallback: &[[u8; 4]] = &[
        [100, 180, 255, 255],
        [255, 160, 80, 255],
        [120, 220, 120, 255],
        [220, 120, 220, 255],
        [255, 220, 80, 255],
    ];

    items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let cor_fb = cores_fallback[i % cores_fallback.len()];
            let caminho_png = format!("assets/{}.png", item.id);
            let img_icone = carregar_png_ou_fallback(&caminho_png, cor_fb);
            let tex_icone = Texture::from_image_buffer(
                &renderer.device,
                &renderer.queue,
                &img_icone,
                &renderer.texture_bind_group_layout,
                &format!("icone_{}", item.id),
            );

            let img_label = rasterizar_texto(font, item.label, 32.0, [1.0, 1.0, 1.0, 1.0]);
            let label_w = img_label.width() as f32;
            let label_h = img_label.height() as f32;
            let tex_label = Texture::from_image_buffer(
                &renderer.device,
                &renderer.queue,
                &img_label,
                &renderer.texture_bind_group_layout,
                &format!("label_{}", item.id),
            );

            IconeItem {
                bind_group: tex_icone.bind_group,
                label_bind_group: tex_label.bind_group,
                label_w,
                label_h,
            }
        })
        .collect()
}
