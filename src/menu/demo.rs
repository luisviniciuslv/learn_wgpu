// Demo de menu em carrossel interativo construído com WGPU.
//
// =========================================================================================
//          ESTRATÉGIA DE PROPORCIONALIDADE (Viewport + Escala Lógica)
// =========================================================================================
//
// [1] O usuário redimensiona a janela livremente.
// [2] Calculamos um VIEWPORT PROPORCIONAL (maior retângulo que cabe na janela),
//     centramos e criamos barras pretas (letterbox/pillarbox) automaticamente.
// [3] O uniform `screen_size` é sempre as dimensões LÓGICAS do viewport.
// [4] scale_factor = viewport_width / BASE_WIDTH — todos os elementos escalam juntos.
// [5] Coordenadas do mouse são convertidas do espaço físico para o lógico.
//
// =========================================================================================

use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use super::renderer::{Renderer, Texture, Viewport, carregar_png_ou_fallback, rasterizar_texto};
use super::types::{ArrowButton, ArrowId, MenuItem, MenuState};
use super::chess::ChessGame;

// Resolução lógica base do design (independente do monitor).
const BASE_WIDTH: f32 = 800.0;
const BASE_HEIGHT: f32 = 600.0;

// =============================================================================
//  Textura de ícone gerada uma vez e reutilizada em todos os frames
// =============================================================================
struct IconeItem {
    /// Bind group da textura GPU deste item — compartilhado via Arc
    bind_group: Arc<wgpu::BindGroup>,
    /// Bind group da textura do rótulo de texto (label)
    label_bind_group: Arc<wgpu::BindGroup>,
    /// Dimensões do rótulo em pixels lógicos (para calcular a posição centralizada)
    label_w: f32,
    label_h: f32,
}

// =============================================================================
//  Núcleo do Aplicativo
// =============================================================================
pub struct App {
    renderer: Option<Renderer>,
    menu_stack: Vec<MenuState>,
    arrows: Vec<ArrowButton>,

    // Posição do mouse em pixels LÓGICOS (já convertida do espaço físico da janela)
    mouse_pos: (f32, f32),
    hovered_arrow: Option<ArrowId>,
    pressed_arrow: Option<ArrowId>,

    // --- VIEWPORT E PROPORCIONALIDADE ---
    viewport: Viewport,
    target_aspect_ratio: f32,
    last_monitor_name: Option<String>,

    // --- RASTREAMENTO DE RESIZE PROPORCIONAL ---
    last_width: u32,
    last_height: u32,
    pending_size: Option<(u32, u32)>,

    // --- ASSETS PRÉ-CARREGADOS ---
    // Ícones e rótulos de texto são gerados uma vez no `resumed` e reutilizados
    icones: Vec<IconeItem>,     // Um por item no nível raiz
    icones_sub: Vec<IconeItem>, // Ícones dos submenus (gerados ao entrar no submenu)

    // --- TELA DE JOGO ---
    chess_game: Option<ChessGame>,
}

impl App {
    pub fn new() -> Self {
        let root_items = vec![
            MenuItem {
                id: "apps",
                label: "Apps",
                children: Some(vec![
                    MenuItem {
                        id: "calc",
                        label: "Calc",
                        children: None,
                    },
                    MenuItem {
                        id: "paint",
                        label: "Paint",
                        children: None,
                    },
                    MenuItem {
                        id: "notes",
                        label: "Notes",
                        children: None,
                    },
                ]),
            },
            MenuItem {
                id: "games",
                label: "Games",
                children: Some(vec![
                    MenuItem {
                        id: "chess",
                        label: "Chess",
                        children: None,
                    },
                    MenuItem {
                        id: "puzzle",
                        label: "Puzzle",
                        children: None,
                    },
                    MenuItem {
                        id: "racer",
                        label: "Racer",
                        children: None,
                    },
                    MenuItem {
                        id: "arcade",
                        label: "Arcade",
                        children: None,
                    },
                ]),
            },
            MenuItem {
                id: "tools",
                label: "Tools",
                children: Some(vec![
                    MenuItem {
                        id: "terminal",
                        label: "Terminal",
                        children: None,
                    },
                    MenuItem {
                        id: "editor",
                        label: "Editor",
                        children: None,
                    },
                ]),
            },
            MenuItem {
                id: "about",
                label: "About",
                children: None,
            },
        ];

        Self {
            renderer: None,
            menu_stack: vec![MenuState::new(root_items)],
            arrows: Vec::new(),
            mouse_pos: (0.0, 0.0),
            hovered_arrow: None,
            pressed_arrow: None,
            viewport: Viewport {
                x: 0.0,
                y: 0.0,
                width: BASE_WIDTH,
                height: BASE_HEIGHT,
            },
            target_aspect_ratio: BASE_WIDTH / BASE_HEIGHT,
            last_monitor_name: None,
            last_width: BASE_WIDTH as u32,
            last_height: BASE_HEIGHT as u32,
            pending_size: None,
            icones: Vec::new(),
            icones_sub: Vec::new(),
            chess_game: None,
        }
    }

    /// Carrega a fonte do sistema — tenta locais comuns de fontes no Windows, macOS e Linux.
    /// Se nenhuma for encontrada, usa a fonte embutida (Inconsolata) do ab_glyph.
    fn carregar_fonte() -> ab_glyph::FontArc {
        let candidatas = [
            // Local (pasta assets/ do projeto) — tem prioridade
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

        // Nenhuma fonte do sistema encontrada — orienta o usuário a colocar uma em assets/
        panic!(
            "Nenhuma fonte encontrada. Coloque um arquivo .ttf em: assets/font.ttf\n\
             Baixe qualquer fonte gratuita (ex: Inter, Roboto) e salve com esse nome."
        )
    }

    /// Gera os `IconeItem`s para uma lista de itens de menu.
    ///
    /// Para cada item:
    ///  - Carrega o PNG de `assets/{id}.png` (ou gera um fallback colorido)
    ///  - Rasteriza o rótulo de texto como textura
    fn gerar_icones(
        items: &[MenuItem],
        renderer: &Renderer,
        font: &ab_glyph::FontArc,
    ) -> Vec<IconeItem> {
        // Paleta de cores fallback para ícones sem PNG
        let cores_fallback: &[[u8; 4]] = &[
            [100, 180, 255, 255], // azul
            [255, 160, 80, 255],  // laranja
            [120, 220, 120, 255], // verde
            [220, 120, 220, 255], // roxo
            [255, 220, 80, 255],  // amarelo
        ];

        items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                // --- Ícone PNG ---
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

                // --- Rótulo de texto ---
                // Escala de 32px é boa para o carrossel (a UI inteira escala pelo scale_factor)
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

    // -------------------------------------------------------------------------
    //  Layout & Física
    // -------------------------------------------------------------------------

    fn resize_layout(&mut self, window_w: u32, window_h: u32) {
        let win_w = window_w as f32;
        let win_h = window_h as f32;
        let ratio = self.target_aspect_ratio;

        let (vp_w, vp_h) = if win_w / win_h > ratio {
            (win_h * ratio, win_h) // pillarbox (barras laterais)
        } else {
            (win_w, win_w / ratio) // letterbox (barras em cima/baixo)
        };

        let vp_x = (win_w - vp_w) * 0.5;
        let vp_y = (win_h - vp_h) * 0.5;

        self.viewport = Viewport {
            x: vp_x,
            y: vp_y,
            width: vp_w,
            height: vp_h,
        };

        let scale_factor = vp_w / BASE_WIDTH;

        if let Some(ref mut renderer) = self.renderer {
            renderer.update_logical_size(vp_w, vp_h);
        }

        let left_w = 50.0 * scale_factor;
        let left_h = 50.0 * scale_factor;
        let left_x = 80.0 * scale_factor;
        let left_y = (vp_h * 0.5) - (left_h * 0.5);

        let right_w = 50.0 * scale_factor;
        let right_h = 50.0 * scale_factor;
        let right_x = vp_w - (80.0 * scale_factor) - right_w;
        let right_y = (vp_h * 0.5) - (right_h * 0.5);

        let back_w = 40.0 * scale_factor;
        let back_h = 40.0 * scale_factor;
        let back_x = 20.0 * scale_factor;
        let back_y = 20.0 * scale_factor;

        self.arrows = vec![
            ArrowButton {
                id: ArrowId::Left,
                x: left_x,
                y: left_y,
                w: left_w,
                h: left_h,
            },
            ArrowButton {
                id: ArrowId::Right,
                x: right_x,
                y: right_y,
                w: right_w,
                h: right_h,
            },
            ArrowButton {
                id: ArrowId::Back,
                x: back_x,
                y: back_y,
                w: back_w,
                h: back_h,
            },
        ];
    }

    fn physical_to_logical(&self, px: f32, py: f32) -> (f32, f32) {
        (px - self.viewport.x, py - self.viewport.y)
    }

    fn update(&mut self) {
        if let Some(menu) = self.menu_stack.last_mut() {
            if menu.animating {
                menu.anim_t += 0.08;
                if menu.anim_t >= 1.0 {
                    menu.anim_t = 1.0;
                    menu.animating = false;
                    menu.selected = menu.anim_to;
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    //  Renderização
    // -------------------------------------------------------------------------

    fn render(&mut self) -> anyhow::Result<()> {
        let renderer = match &mut self.renderer {
            Some(r) => r,
            None => return Ok(()),
        };

        renderer.clear();

        let vp_w = renderer.uniforms.screen_size[0];
        let vp_h = renderer.uniforms.screen_size[1];
        let scale_factor = vp_w / BASE_WIDTH;

        // 1. Fundo gradiente simulado com dois retângulos sobrepostos
        renderer.draw_rect(0.0, 0.0, vp_w, vp_h, [0.08, 0.08, 0.12, 1.0]);
        renderer.draw_rect(0.0, vp_h * 0.5, vp_w, vp_h * 0.5, [0.10, 0.10, 0.16, 1.0]);

        // 2. Barra superior com linha decorativa
        let top_bar_h = 60.0 * scale_factor;
        renderer.draw_rect(0.0, 0.0, vp_w, top_bar_h, [0.13, 0.13, 0.18, 1.0]);
        renderer.draw_rect(
            0.0,
            top_bar_h - 2.0 * scale_factor,
            vp_w,
            2.0 * scale_factor,
            [0.3, 0.5, 0.9, 0.9],
        );

        if let Some(ref game) = self.chess_game {
            // Renderiza apenas o tabuleiro de xadrez e o cursor
            game.render(renderer, vp_w, vp_h)?;

            // Renderiza o botão de voltar no canto superior esquerdo
            for arrow in &self.arrows {
                if arrow.id == ArrowId::Back {
                    let is_hovered = self.hovered_arrow == Some(arrow.id);
                    let is_pressed = self.pressed_arrow == Some(arrow.id);

                    let color = if is_pressed {
                        [0.2, 0.6, 1.0, 1.0]
                    } else if is_hovered {
                        [0.4, 0.7, 1.0, 1.0]
                    } else {
                        [0.2, 0.3, 0.5, 1.0]
                    };

                    desenhar_seta_botao(
                        renderer, arrow.id, arrow.x, arrow.y, arrow.w, arrow.h, color,
                    );
                }
            }

            // Cursor customizado (ponto amarelo)
            let cursor_size = 8.0 * scale_factor;
            renderer.draw_rect(
                self.mouse_pos.0 - cursor_size * 0.5,
                self.mouse_pos.1 - cursor_size * 0.5,
                cursor_size,
                cursor_size,
                [1.0, 0.9, 0.0, 0.9],
            );

            renderer.present(self.viewport)?;
            return Ok(());
        }

        // 3. Detecção de hover dos botões (calculado aqui, desenhado após o carrossel)
        let mut hovered_arrow = None;
        for arrow in &self.arrows {
            if arrow.contains(self.mouse_pos.0, self.mouse_pos.1) {
                hovered_arrow = Some(arrow.id);
            }
        }
        self.hovered_arrow = hovered_arrow;

        // 4. Carrossel de menu
        let cx = vp_w * 0.5;
        let cy = vp_h * 0.5;
        let base_size = 120.0 * scale_factor;
        let gap = 160.0 * scale_factor;

        // Determina qual conjunto de ícones usar (raiz ou submenu)
        let profundidade = self.menu_stack.len();
        let icones_ativos = if profundidade == 1 {
            &self.icones
        } else {
            &self.icones_sub
        };

        if let Some(menu) = self.menu_stack.last() {
            let selected_f = menu.selected_float();

            for (i, _item) in menu.items.iter().enumerate() {
                let d = i as f32 - selected_f;
                if d.abs() > 3.0 {
                    continue;
                }

                // Cartões laterais ficam menores (profundidade de foco)
                let escala = (1.0 - d.abs() * 0.18).clamp(0.6, 1.0);
                let size = base_size * escala;
                let x = cx + d * gap - size * 0.5;
                let y = cy - size * 0.5;

                let is_center = d.abs() < 0.5;

                // Sombra do cartão (retângulo levemente deslocado e escurecido)
                renderer.draw_rect(x + 4.0, y + 4.0, size, size, [0.0, 0.0, 0.0, 0.4]);

                // Fundo do cartão
                let cor_fundo = if is_center {
                    [0.18, 0.18, 0.25, 1.0]
                } else {
                    [0.13, 0.13, 0.18, 1.0]
                };
                renderer.draw_rect(x, y, size, size, cor_fundo);

                // Ícone PNG texturizado (se disponível)
                if let Some(icone) = icones_ativos.get(i) {
                    let padding = size * 0.1;
                    let tint = if is_center {
                        [1.0, 1.0, 1.0, 1.0]
                    } else {
                        [0.6, 0.6, 0.7, 1.0]
                    };
                    renderer.draw_textured_rect(
                        x + padding,
                        y + padding,
                        size - padding * 2.0,
                        size - padding * 2.0,
                        tint,
                        [0.0, 0.0, 1.0, 1.0],
                        icone.bind_group.clone(),
                    );

                    // Rótulo de texto abaixo do cartão (centralizado)
                    let label_scale = escala * scale_factor;
                    let lw = icone.label_w * label_scale;
                    let lh = icone.label_h * label_scale;
                    let label_x = x + size * 0.5 - lw * 0.5;
                    let label_y = y + size + 6.0 * scale_factor;
                    let label_alpha = if is_center { 1.0 } else { 0.5 };
                    renderer.draw_textured_rect(
                        label_x,
                        label_y,
                        lw,
                        lh,
                        [1.0, 1.0, 1.0, label_alpha],
                        [0.0, 0.0, 1.0, 1.0],
                        icone.label_bind_group.clone(),
                    );
                }

                // Indicador de submenu (quadradinho branco no canto)
                if _item.children.is_some() {
                    let ind = 10.0 * scale_factor;
                    let pad = 5.0 * scale_factor;
                    renderer.draw_rect(
                        x + size - ind - pad,
                        y + pad,
                        ind,
                        ind,
                        [1.0, 1.0, 1.0, 0.9],
                    );
                }

                // Borda do cartão focado
                if is_center {
                    let borda = 2.0 * scale_factor;
                    renderer.draw_rect(x, y, size, borda, [0.3, 0.6, 1.0, 0.8]); // cima
                    renderer.draw_rect(x, y + size - borda, size, borda, [0.3, 0.6, 1.0, 0.8]); // baixo
                    renderer.draw_rect(x, y, borda, size, [0.3, 0.6, 1.0, 0.8]); // esq
                    renderer.draw_rect(x + size - borda, y, borda, size, [0.3, 0.6, 1.0, 0.8]); // dir
                }
            }
        }

        // 5. Botões de seta — desenhados DEPOIS do carrossel para ficarem sempre na frente
        for arrow in &self.arrows {
            let is_hovered = self.hovered_arrow == Some(arrow.id);
            let is_pressed = self.pressed_arrow == Some(arrow.id);

            let color = if is_pressed {
                [0.2, 0.6, 1.0, 1.0]
            } else if is_hovered {
                [0.4, 0.7, 1.0, 1.0]
            } else {
                [0.2, 0.3, 0.5, 1.0]
            };

            desenhar_seta_botao(
                renderer, arrow.id, arrow.x, arrow.y, arrow.w, arrow.h, color,
            );
        }

        // 6. Cursor customizado (ponto amarelo)
        let cursor_size = 8.0 * scale_factor;
        renderer.draw_rect(
            self.mouse_pos.0 - cursor_size * 0.5,
            self.mouse_pos.1 - cursor_size * 0.5,
            cursor_size,
            cursor_size,
            [1.0, 0.9, 0.0, 0.9],
        );

        renderer.present(self.viewport)?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    //  Lógica de Navegação
    // -------------------------------------------------------------------------

    fn move_selection(&mut self, delta: i32) {
        if let Some(menu) = self.menu_stack.last_mut() {
            if menu.animating {
                return;
            }
            let len = menu.items.len() as i32;
            if len == 0 {
                return;
            }
            let next = (menu.selected + delta).clamp(0, len - 1);
            if next == menu.selected {
                return;
            }
            menu.anim_from = menu.selected;
            menu.anim_to = next;
            menu.anim_t = 0.0;
            menu.animating = true;
        }
    }

    fn try_enter_submenu(&mut self) {
        // Captura os dados necessários antes de emprestar self mutavelmente
        let (children, items_clone, item_id) = {
            let menu = match self.menu_stack.last() {
                Some(m) => m,
                None => return,
            };
            let idx = menu.selected as usize;
            let item = match menu.items.get(idx) {
                Some(i) => i,
                None => return,
            };
            match &item.children {
                Some(ch) => (true, ch.clone(), item.id),
                None => (false, vec![], item.id),
            }
        };

        if item_id == "chess" {
            // Entrar no jogo de xadrez:
            // 1. Instanciar o jogo
            self.chess_game = Some(ChessGame::new());
            // 2. Definir a proporção alvo como quadrada (1.0)
            self.target_aspect_ratio = 1.0;
            // 3. Requisitar que a janela fique quadrada
            if let Some(ref renderer) = self.renderer {
                let size = renderer.window.inner_size();
                // Tornamos a largura igual à altura para ficar quadrado
                let min_dim = size.width.min(size.height);
                let _ = renderer.window.request_inner_size(winit::dpi::PhysicalSize::new(min_dim, min_dim));
                self.pending_size = Some((min_dim, min_dim));
            }
            return;
        }

        if children {
            // Gera os ícones do submenu antes de empurrar o estado
            if let Some(renderer) = &self.renderer {
                let font = Self::carregar_fonte();
                self.icones_sub = Self::gerar_icones(&items_clone, renderer, &font);
            }
            self.menu_stack.push(MenuState::new(items_clone));
        }
    }

    fn pop_menu(&mut self) {
        if self.chess_game.is_some() {
            // Sair do xadrez:
            self.chess_game = None;
            // Restaurar proporção de tela original 4:3 (800 / 600)
            self.target_aspect_ratio = BASE_WIDTH / BASE_HEIGHT;
            if let Some(ref renderer) = self.renderer {
                let size = renderer.window.inner_size();
                // Calcula a nova largura baseada na altura mantendo a proporção 4:3
                let new_w = (size.height as f32 * self.target_aspect_ratio).round() as u32;
                let _ = renderer.window.request_inner_size(winit::dpi::PhysicalSize::new(new_w, size.height));
                self.pending_size = Some((new_w, size.height));
            }
            return;
        }

        if self.menu_stack.len() > 1 {
            self.menu_stack.pop();
            self.icones_sub.clear();
        }
    }

    fn back_menu(&mut self, event_loop: &ActiveEventLoop) {
        if self.chess_game.is_some() {
            self.pop_menu();
            return;
        }

        if self.menu_stack.len() > 1 {
            self.menu_stack.pop();
            self.icones_sub.clear();
        } else {
            event_loop.exit();
        }
    }

    /// Detecta qual cartão do carrossel contém o ponto (px, py) em pixels lógicos.
    ///
    /// Retorna o índice do item clicado dentro de `menu.items`.
    /// Usa a mesma geometria do `render()` para garantir consistência.
    ///
    /// Quando cartões se sobrepõem (ex: d=±1 sobre d=±2), prefere sempre
    /// o cartão mais próximo do centro (menor |d|), que é o que o usuário vê na frente.
    fn card_hit_test(&self, px: f32, py: f32) -> Option<usize> {
        let renderer = self.renderer.as_ref()?;
        let vp_w = renderer.uniforms.screen_size[0];
        let vp_h = renderer.uniforms.screen_size[1];
        let scale_factor = vp_w / BASE_WIDTH;

        let cx = vp_w * 0.5;
        let cy = vp_h * 0.5;
        let base_size = 120.0 * scale_factor;
        let gap = 160.0 * scale_factor;

        let menu = self.menu_stack.last()?;

        // Durante animação usamos o destino final para calcular as posições,
        // mas bloqueamos cliques para evitar entradas acidentais.
        if menu.animating {
            return None;
        }

        let selected_f = menu.selected as f32; // posição estável (sem anim)

        let mut melhor_idx: Option<usize> = None;
        let mut melhor_d_abs = f32::MAX;

        for (i, _) in menu.items.iter().enumerate() {
            let d = i as f32 - selected_f;
            if d.abs() > 3.0 {
                continue;
            }

            let scale = (1.0 - d.abs() * 0.18).clamp(0.6, 1.0);
            let size = base_size * scale;
            let x = cx + d * gap - size * 0.5;
            let y = cy - size * 0.5;

            if px >= x && px <= x + size && py >= y && py <= y + size {
                // Prefere o cartão mais próximo do centro (fica "na frente" visualmente)
                if d.abs() < melhor_d_abs {
                    melhor_d_abs = d.abs();
                    melhor_idx = Some(i);
                }
            }
        }

        melhor_idx
    }
}

// =============================================================================
//  ApplicationHandler — Loop de Eventos do Winit
// =============================================================================
impl ApplicationHandler for App {
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            // Resize com preservação de proporção:
            //   - Se for confirmação do nosso próprio request_inner_size → processa normalmente.
            //   - Se for resize do usuário → calcula o tamanho correto e requisita uma vez.
            WindowEvent::Resized(size) => {
                if size.width == 0 || size.height == 0 || self.renderer.is_none() {
                    return;
                }

                let is_our_own_request = self
                    .pending_size
                    .map(|(pw, ph)| {
                        (size.width as i32 - pw as i32).abs() <= 2
                            && (size.height as i32 - ph as i32).abs() <= 2
                    })
                    .unwrap_or(false);

                if is_our_own_request {
                    self.pending_size = None;
                } else {
                    let dw = (size.width as i32 - self.last_width as i32).abs();
                    let dh = (size.height as i32 - self.last_height as i32).abs();
                    let ratio = self.target_aspect_ratio;

                    let (new_w, new_h) = if dw >= dh {
                        let h = (size.width as f32 / ratio).round() as u32;
                        (size.width, h.max(1))
                    } else {
                        let w = (size.height as f32 * ratio).round() as u32;
                        (w.max(1), size.height)
                    };

                    if new_w != size.width || new_h != size.height {
                        if let Some(ref renderer) = self.renderer {
                            let _ = renderer
                                .window
                                .request_inner_size(winit::dpi::PhysicalSize::new(new_w, new_h));
                        }
                        self.pending_size = Some((new_w, new_h));
                        self.last_width = size.width;
                        self.last_height = size.height;
                    }
                }

                if let Some(ref mut renderer) = self.renderer {
                    renderer.resize(size.width, size.height);
                }
                self.resize_layout(size.width, size.height);
                self.last_width = size.width;
                self.last_height = size.height;
            }

            // Detecta cruzamento de monitor e atualiza a proporção alvo
            WindowEvent::Moved(_) => {
                let update = self.renderer.as_ref().and_then(|renderer| {
                    renderer.window.current_monitor().and_then(|monitor| {
                        let monitor_name = monitor.name();
                        if monitor_name != self.last_monitor_name {
                            let mon_size = monitor.size();
                            let win_size = renderer.window.inner_size();
                            Some((monitor_name, mon_size, win_size))
                        } else {
                            None
                        }
                    })
                });

                if let Some((monitor_name, mon_size, win_size)) = update {
                    self.target_aspect_ratio = mon_size.width as f32 / mon_size.height as f32;
                    self.last_monitor_name = monitor_name;
                    self.resize_layout(win_size.width, win_size.height);
                }
            }

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key_code),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => match key_code {
                KeyCode::Escape => self.back_menu(event_loop),
                KeyCode::ArrowLeft => self.move_selection(-1),
                KeyCode::ArrowRight => self.move_selection(1),
                KeyCode::Enter | KeyCode::Space => self.try_enter_submenu(),
                _ => {}
            },

            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos = self.physical_to_logical(position.x as f32, position.y as f32);
                if let Some(ref renderer) = self.renderer {
                    renderer.window.request_redraw();
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    if state == ElementState::Pressed {
                        if let Some(arrow) = self.hovered_arrow {
                            self.pressed_arrow = Some(arrow);
                            match arrow {
                                ArrowId::Left => self.move_selection(-1),
                                ArrowId::Right => self.move_selection(1),
                                ArrowId::Back => self.pop_menu(),
                            }
                        } else {
                            // Hit-test nos cartões do carrossel:
                            // - Clique no cartão CENTRAL  → entra no submenu (se houver)
                            // - Clique em cartão LATERAL  → navega até ele
                            // - Clique fora de todos      → ignora
                            let mx = self.mouse_pos.0;
                            let my = self.mouse_pos.1;
                            if let Some(idx) = self.card_hit_test(mx, my) {
                                let selected =
                                    self.menu_stack.last().map(|m| m.selected).unwrap_or(0);
                                if idx as i32 == selected {
                                    self.try_enter_submenu();
                                } else {
                                    let delta = idx as i32 - selected;
                                    self.move_selection(delta);
                                }
                            }
                        }
                    } else {
                        self.pressed_arrow = None;
                    }
                }
                if let Some(ref renderer) = self.renderer {
                    renderer.window.request_redraw();
                }
            }

            WindowEvent::RedrawRequested => {
                self.update();
                if let Err(e) = self.render() {
                    let msg = format!("{:?}", e);
                    if !msg.contains("Outdated")
                        && !msg.contains("Timeout")
                        && !msg.contains("Lost")
                        && !msg.contains("Other")
                    {
                        eprintln!("Erro Crítico de Renderização: {:?}", e);
                        event_loop.exit();
                    }
                }
                if let Some(ref renderer) = self.renderer {
                    renderer.window.request_redraw();
                }
            }

            _ => {}
        }
    }

    /// Inicialização: cria janela, renderer e pré-carrega todos os assets.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let monitor = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next());

        let mut init_width = 800u32;
        let mut init_height = 600u32;

        if let Some(ref m) = monitor {
            let size = m.size();
            init_width = (size.width as f32 * 0.35) as u32;
            init_height = (size.height as f32 * 0.35) as u32;
            self.target_aspect_ratio = size.width as f32 / size.height as f32;
            self.last_monitor_name = m.name();
        }

        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("2D UI Engine com WGPU")
                        .with_inner_size(winit::dpi::PhysicalSize::new(init_width, init_height))
                        .with_resizable(true),
                )
                .unwrap(),
        );

        let renderer = pollster::block_on(Renderer::new(window)).unwrap();

        // Pré-carrega a fonte e gera os ícones/labels do menu raiz
        let font = Self::carregar_fonte();
        let root_items = self.menu_stack[0].items.clone();
        self.icones = Self::gerar_icones(&root_items, &renderer, &font);

        let size = renderer.window.inner_size();
        self.last_width = size.width;
        self.last_height = size.height;
        self.renderer = Some(renderer);
        self.resize_layout(size.width, size.height);
    }
}

// =============================================================================
//  Funções auxiliares de desenho (fora do impl, sem acesso a self)
// =============================================================================

/// Desenha um botão de navegação como uma seta vetorial (haste + cabeça triangular).
///
/// Geometria em pixels lógicos, relativa ao canto superior esquerdo do botão (bx, by):
///
///   Seta Direita (►)          Seta Esquerda (◄)
///   ┌─────────────┐            ┌─────────────┐
///   │  ▬▬▬▬▬▶     │            │     ◀▬▬▬▬▬  │
///   └─────────────┘            └─────────────┘
///
///   haste = retângulo         cabeça = triângulo
fn desenhar_seta_botao(
    renderer: &mut Renderer,
    id: ArrowId,
    bx: f32,
    by: f32, // canto sup. esq. do botão (pixels lógicos)
    bw: f32,
    bh: f32, // dimensões do botão
    color: [f32; 4],
) {
    let cx = bx + bw * 0.5;
    let cy = by + bh * 0.5;

    match id {
        // ►  Seta apontando para a DIREITA
        ArrowId::Right => {
            // Haste: retângulo na metade esquerda, centralizado verticalmente
            renderer.draw_rect(
                bx + bw * 0.08, // x
                cy - bh * 0.14, // y
                bw * 0.50,      // largura
                bh * 0.28,      // altura
                color,
            );
            // Cabeça: triângulo na metade direita apontando para a direita
            renderer.draw_triangle(
                [cx - bw * 0.02, by + bh * 0.08], // topo esquerdo da base
                [cx - bw * 0.02, by + bh * 0.92], // fundo esquerdo da base
                [bx + bw * 0.94, cy],             // ponta direita
                color,
            );
        }

        // ◄  Seta apontando para a ESQUERDA (Left e Back)
        ArrowId::Left | ArrowId::Back => {
            // Haste: retângulo na metade direita, centralizado verticalmente
            renderer.draw_rect(
                cx + bw * 0.02, // x
                cy - bh * 0.14, // y
                bw * 0.50,      // largura
                bh * 0.28,      // altura
                color,
            );
            // Cabeça: triângulo na metade esquerda apontando para a esquerda
            renderer.draw_triangle(
                [cx + bw * 0.02, by + bh * 0.08], // topo direito da base
                [cx + bw * 0.02, by + bh * 0.92], // fundo direito da base
                [bx + bw * 0.06, cy],             // ponta esquerda
                color,
            );
        }
    }
}

// =============================================================================
//  Ponto de Entrada
// =============================================================================
pub fn run() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = App::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}
