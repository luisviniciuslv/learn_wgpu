// Demo simples de um menu em carrossel interativo construído com a WGPU (WebGPU para Rust).
//
// O objetivo deste arquivo é demonstrar na prática como criar uma interface gráfica 2D
// interativa, com suporte a mouse, teclado, animações de transição suaves e menus aninhados (submenus).
// Este código foi projetado para ser didático, modular, limpo e extremamente fácil de estender.
//
// =========================================================================================
//          ESTRATÉGIA DE PROPORCIONALIDADE (Viewport + Escala Lógica)
// =========================================================================================
//
// A abordagem correta para manter proporções é NÃO brigar com o gerenciador de janelas.
// Em vez disso, usamos um VIEWPORT que define uma sub-região da janela onde desenhamos.
//
// Como funciona:
//
// [1] O usuário pode redimensionar a janela como quiser (livre, sem restrições).
//
// [2] Em todo resize, calculamos o VIEWPORT PROPORCIONAL:
//     - Pegamos a proporção alvo (aspect ratio do monitor, ex: 16/9)
//     - Calculamos o maior retângulo com essa proporção que cabe na janela atual
//     - Centramos esse retângulo na janela
//     → Isso cria barras pretas automáticas (letterbox/pillarbox) quando necessário
//
// [3] O uniform `screen_size` da GPU é sempre as dimensões LÓGICAS do viewport
//     (não da janela física). Isso garante que o shader converta pixels → clip space
//     corretamente, independentemente do tamanho da janela.
//
// [4] O scale_factor de renderização é viewport_width / BASE_WIDTH.
//     Todos os quadrados, botões e elementos escalam juntos.
//
// [5] Coordenadas do mouse são convertidas do espaço físico da janela para o espaço
//     lógico do viewport antes dos testes de colisão.
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

// Importa o renderizador 2D definido no módulo irmão 'renderer'.
use super::renderer::{Renderer, Viewport};
// Importa os tipos de dados do módulo 'types'.
use super::types::{ArrowButton, ArrowId, MenuItem, MenuState};

// Resolução lógica base do design (independente do monitor).
// Todo o layout é projetado para estas dimensões e escalado proporcionalmente.
const BASE_WIDTH: f32 = 800.0;
const BASE_HEIGHT: f32 = 600.0;

/// **O que é:** O núcleo central do nosso aplicativo.
pub struct App {
    renderer: Option<Renderer>,
    menu_stack: Vec<MenuState>,
    arrows: Vec<ArrowButton>,

    // Posição do mouse em pixels LÓGICOS (já convertida do espaço físico da janela)
    mouse_pos: (f32, f32),
    hovered_arrow: Option<ArrowId>,
    pressed_arrow: Option<ArrowId>,

    // --- VIEWPORT E PROPORCIONALIDADE ---
    // O viewport é a região da janela física onde desenhamos (preserva proporção do monitor).
    viewport: Viewport,
    // Proporção alvo derivada do monitor ativo (largura / altura).
    target_aspect_ratio: f32,
    // Nome do monitor atual para detectar cruzamentos de tela (Multi-Monitor).
    last_monitor_name: Option<String>,

    // --- RASTREAMENTO DE RESIZE PROPORCIONAL DA JANELA ---
    // Dimensões do último resize processado (para detectar qual eixo mudou mais).
    last_width: u32,
    last_height: u32,
    // Tamanho que nós mesmos requisitamos via request_inner_size.
    // Enquanto pending_size != None, ignoramos o próximo Resized (é o SO confirmando nossa requisição).
    pending_size: Option<(u32, u32)>,
}

impl App {
    pub fn new() -> Self {
        // Árvore de menus para navegação.
        let root_items = vec![
            MenuItem {
                id: "apps",
                label: "Apps",
                children: Some(vec![
                    MenuItem { id: "calc", label: "Calc", children: None },
                    MenuItem { id: "paint", label: "Paint", children: None },
                    MenuItem { id: "notes", label: "Notes", children: None },
                ]),
            },
            MenuItem {
                id: "games",
                label: "Games",
                children: Some(vec![
                    MenuItem { id: "puzzle", label: "Puzzle", children: None },
                    MenuItem { id: "racer", label: "Racer", children: None },
                    MenuItem { id: "arcade", label: "Arcade", children: None },
                ]),
            },
            MenuItem {
                id: "tools",
                label: "Tools",
                children: Some(vec![
                    MenuItem { id: "terminal", label: "Terminal", children: None },
                    MenuItem { id: "editor", label: "Editor", children: None },
                ]),
            },
            MenuItem { id: "about", label: "About", children: None },
        ];

        Self {
            renderer: None,
            menu_stack: vec![MenuState::new(root_items)],
            arrows: Vec::new(),
            mouse_pos: (0.0, 0.0),
            hovered_arrow: None,
            pressed_arrow: None,
            viewport: Viewport { x: 0.0, y: 0.0, width: BASE_WIDTH, height: BASE_HEIGHT },
            target_aspect_ratio: BASE_WIDTH / BASE_HEIGHT, // Fallback inicial (4:3 aproximado)
            last_monitor_name: None,
            last_width: BASE_WIDTH as u32,
            last_height: BASE_HEIGHT as u32,
            pending_size: None,
        }
    }

    /// Calcula o viewport proporcional e atualiza o layout dos botões.
    ///
    /// Dado o tamanho físico da janela (window_w x window_h) e a proporção alvo,
    /// encontra o maior retângulo proporcional que cabe na janela e o centra.
    /// Também atualiza o uniform screen_size da GPU com as dimensões LÓGICAS do viewport.
    fn resize_layout(&mut self, window_w: u32, window_h: u32) {
        let win_w = window_w as f32;
        let win_h = window_h as f32;
        let ratio = self.target_aspect_ratio;

        // Calcula o viewport proporcional (maior retângulo com a proporção alvo dentro da janela)
        let (vp_w, vp_h) = if win_w / win_h > ratio {
            // Janela mais larga que o necessário → pillarbox (barras nas laterais)
            (win_h * ratio, win_h)
        } else {
            // Janela mais alta que o necessário → letterbox (barras em cima/baixo)
            (win_w, win_w / ratio)
        };

        let vp_x = (win_w - vp_w) * 0.5;
        let vp_y = (win_h - vp_h) * 0.5;

        self.viewport = Viewport {
            x: vp_x,
            y: vp_y,
            width: vp_w,
            height: vp_h,
        };

        // O scale_factor é calculado com base no viewport lógico (não na janela física).
        // BASE_WIDTH/BASE_HEIGHT representam as dimensões do design original.
        // O conteúdo escala uniformemente em ambos os eixos (sem distorção).
        let scale_factor = vp_w / BASE_WIDTH;

        // Atualiza o uniform da GPU com as dimensões LÓGICAS do viewport.
        // O shader usará isso para converter pixels → clip space corretamente.
        if let Some(ref mut renderer) = self.renderer {
            renderer.update_logical_size(vp_w, vp_h);
        }

        // Seta esquerda: posicionada na lateral esquerda, centralizada verticalmente.
        let left_w = 50.0 * scale_factor;
        let left_h = 50.0 * scale_factor;
        let left_x = 80.0 * scale_factor;
        let left_y = (vp_h * 0.5) - (left_h * 0.5);

        // Seta direita: posicionada perto da borda direita, centralizada verticalmente.
        let right_w = 50.0 * scale_factor;
        let right_h = 50.0 * scale_factor;
        let right_x = vp_w - (80.0 * scale_factor) - right_w;
        let right_y = (vp_h * 0.5) - (right_h * 0.5);

        // Botão de voltar: posicionado no canto superior esquerdo.
        let back_w = 40.0 * scale_factor;
        let back_h = 40.0 * scale_factor;
        let back_x = 20.0 * scale_factor;
        let back_y = 20.0 * scale_factor;

        self.arrows = vec![
            ArrowButton { id: ArrowId::Left, x: left_x, y: left_y, w: left_w, h: left_h },
            ArrowButton { id: ArrowId::Right, x: right_x, y: right_y, w: right_w, h: right_h },
            ArrowButton { id: ArrowId::Back, x: back_x, y: back_y, w: back_w, h: back_h },
        ];
    }

    /// Converte coordenadas físicas do mouse (relativas à janela) para coordenadas
    /// lógicas do viewport (relativas à área de desenho).
    fn physical_to_logical(&self, px: f32, py: f32) -> (f32, f32) {
        let lx = px - self.viewport.x;
        let ly = py - self.viewport.y;
        (lx, ly)
    }

    /// Lógica matemática de atualização temporal de frame (animação de transição).
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

    /// Renderiza o frame atual.
    ///
    /// Todo o código de desenho usa coordenadas em pixels LÓGICOS do viewport.
    /// O scale_factor é viewport_width / BASE_WIDTH — todos os elementos escalam juntos.
    fn render(&mut self) -> anyhow::Result<()> {
        let renderer = match &mut self.renderer {
            Some(r) => r,
            None => return Ok(()),
        };

        renderer.clear();

        let vp_w = renderer.uniforms.screen_size[0];
        let vp_h = renderer.uniforms.screen_size[1];

        // Fator de escala uniforme: viewport_width / largura base do design
        let scale_factor = vp_w / BASE_WIDTH;

        // 1. Fundo da área de conteúdo
        renderer.draw_rect(0.0, 0.0, vp_w, vp_h, [0.12, 0.12, 0.15, 1.0]);

        // 2. Barra superior
        let top_bar_height = 60.0 * scale_factor;
        renderer.draw_rect(0.0, 0.0, vp_w, top_bar_height, [0.15, 0.15, 0.18, 1.0]);
        renderer.draw_rect(0.0, top_bar_height - (2.0 * scale_factor), vp_w, 2.0 * scale_factor, [0.3, 0.5, 0.9, 0.8]);

        // 3. Setas clicáveis
        let mut hovered_arrow = None;
        for arrow in &self.arrows {
            let is_hovered = arrow.contains(self.mouse_pos.0, self.mouse_pos.1);
            if is_hovered {
                hovered_arrow = Some(arrow.id);
            }
            let is_pressed = self.pressed_arrow == Some(arrow.id);

            let color = if is_pressed {
                [0.2, 0.6, 1.0, 1.0]
            } else if is_hovered {
                [0.4, 0.7, 1.0, 1.0]
            } else {
                [0.2, 0.3, 0.5, 1.0]
            };

            renderer.draw_rect(arrow.x, arrow.y, arrow.w, arrow.h, color);
        }
        self.hovered_arrow = hovered_arrow;

        // 4. Carrossel de menu — escala com o viewport
        let cx = vp_w * 0.5;
        let cy = vp_h * 0.5;

        let base_size = 120.0 * scale_factor;
        let gap = 160.0 * scale_factor;

        if let Some(menu) = self.menu_stack.last() {
            let selected_f = menu.selected_float();

            for (i, item) in menu.items.iter().enumerate() {
                let d = i as f32 - selected_f;
                if d.abs() > 3.0 {
                    continue;
                }

                // Reduz suavemente cartões distantes do centro para dar profundidade de foco
                let scale = (1.0 - d.abs() * 0.18).clamp(0.6, 1.0);
                let size = base_size * scale;

                let x = cx + d * gap - size * 0.5;
                let y = cy - size * 0.5;

                let is_center = d.abs() < 0.5;
                let color = if is_center {
                    [0.9, 0.5, 0.1, 1.0] // Laranja neon para o focado
                } else {
                    [0.2, 0.5, 0.7, 1.0] // Azul suave para os laterais
                };

                renderer.draw_rect(x, y, size, size, color);

                if item.children.is_some() {
                    let indicator_size = 12.0 * scale_factor;
                    let padding = 6.0 * scale_factor;
                    renderer.draw_rect(
                        x + size - indicator_size - padding,
                        y + padding,
                        indicator_size,
                        indicator_size,
                        [1.0, 1.0, 1.0, 0.8],
                    );
                }
            }
        }

        // 5. Cursor de mouse (em coordenadas lógicas)
        let cursor_size = 8.0 * scale_factor;
        renderer.draw_rect(
            self.mouse_pos.0 - cursor_size * 0.5,
            self.mouse_pos.1 - cursor_size * 0.5,
            cursor_size,
            cursor_size,
            [1.0, 0.9, 0.0, 0.8],
        );

        renderer.present(self.viewport)?;
        Ok(())
    }

    /// Move a seleção do menu e ativa a transição suave de deslize.
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

    /// Tenta acessar o submenu do item central de foco.
    fn try_enter_submenu(&mut self) {
        if let Some(menu) = self.menu_stack.last() {
            let idx = menu.selected as usize;
            if let Some(item) = menu.items.get(idx) {
                if let Some(children) = item.children.clone() {
                    self.menu_stack.push(MenuState::new(children));
                }
            }
        }
    }

    /// Retorna ao menu superior e desempilha a visualização atual.
    fn pop_menu(&mut self) {
        if self.menu_stack.len() > 1 {
            self.menu_stack.pop();
        }
    }
}

impl ApplicationHandler for App {
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            // Quando a janela é redimensionada:
            //   1. Se for uma confirmação do SO do nosso próprio request_inner_size → processa normalmente.
            //   2. Se for resize iniciado pelo usuário → calcula o tamanho correto proporcional,
            //      requisita uma vez via request_inner_size e retorna (aguarda confirmação do SO).
            // Isso garante que a janela sempre mantenha a proporção do monitor, sem loops infinitos.
            WindowEvent::Resized(size) => {
                if size.width == 0 || size.height == 0 {
                    return;
                }
                if self.renderer.is_none() {
                    return;
                }

                // Verifica se este Resized é o SO confirmando nossa requisição pendente.
                // Usa tolerância de ±2px para lidar com arredondamentos do SO.
                let is_our_own_request = self.pending_size.map(|(pw, ph)| {
                    (size.width as i32 - pw as i32).abs() <= 2
                        && (size.height as i32 - ph as i32).abs() <= 2
                }).unwrap_or(false);

                if is_our_own_request {
                    // É a confirmação da nossa requisição — processa normalmente.
                    self.pending_size = None;
                } else {
                    // É um resize iniciado pelo usuário.
                    // Detecta qual eixo mudou mais para saber qual fixar como referência.
                    let dw = (size.width as i32 - self.last_width as i32).abs();
                    let dh = (size.height as i32 - self.last_height as i32).abs();

                    let ratio = self.target_aspect_ratio;
                    let (new_w, new_h) = if dw >= dh {
                        // Usuário puxou mais na horizontal → fixa a largura e ajusta a altura.
                        let h = (size.width as f32 / ratio).round() as u32;
                        (size.width, h.max(1))
                    } else {
                        // Usuário puxou mais na vertical → fixa a altura e ajusta a largura.
                        let w = (size.height as f32 * ratio).round() as u32;
                        (w.max(1), size.height)
                    };

                    // Só requisita correção se as dimensões realmente precisam mudar.
                    if new_w != size.width || new_h != size.height {
                        if let Some(ref renderer) = self.renderer {
                            let _ = renderer.window.request_inner_size(
                                winit::dpi::PhysicalSize::new(new_w, new_h)
                            );
                        }
                        // Registra o tamanho requisitado para identificar a confirmação do SO.
                        self.pending_size = Some((new_w, new_h));
                        // Atualiza last_width/height com o tamanho atual (para o próximo delta).
                        self.last_width = size.width;
                        self.last_height = size.height;
                        // Ainda configura a superfície e o layout com o tamanho atual
                        // (o viewport letterbox garante que não haverá distorção visual no frame intermediário).
                    }
                }

                // Reconfigura a superfície WGPU com as dimensões físicas confirmadas pelo SO.
                if let Some(ref mut renderer) = self.renderer {
                    renderer.resize(size.width, size.height);
                }
                self.resize_layout(size.width, size.height);
                self.last_width = size.width;
                self.last_height = size.height;
            }

            // Detecta cruzamento entre monitores e ajusta a proporção alvo.
            WindowEvent::Moved(_) => {
                // Coleta todos os dados necessários enquanto o empréstimo está ativo,
                // depois solta o empréstimo antes de chamar resize_layout (que precisa de &mut self).
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
                KeyCode::Escape => event_loop.exit(),
                KeyCode::ArrowLeft => self.move_selection(-1),
                KeyCode::ArrowRight => self.move_selection(1),
                KeyCode::Backspace => self.pop_menu(),
                _ => {}
            },

            WindowEvent::CursorMoved { position, .. } => {
                // Converte coordenadas físicas do mouse para o espaço lógico do viewport
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
                            self.try_enter_submenu();
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
                    let err_msg = format!("{:?}", e);
                    if err_msg.contains("Outdated")
                        || err_msg.contains("Timeout")
                        || err_msg.contains("Lost")
                        || err_msg.contains("Other")
                    {
                        // Erros transitórios de superfície — ignorar graciosamente
                    } else {
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

    /// Inicialização: cria a janela com 35% do monitor primário.
    /// A proporção alvo é derivada do monitor ativo e usada para calcular o viewport.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let monitor = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next());

        let mut init_width = 800u32;
        let mut init_height = 600u32;

        if let Some(ref m) = monitor {
            let size = m.size();
            // A janela inicial ocupa 35% do monitor
            init_width = (size.width as f32 * 0.35) as u32;
            init_height = (size.height as f32 * 0.35) as u32;

            // A proporção alvo vem do monitor, não da janela inicial
            self.target_aspect_ratio = size.width as f32 / size.height as f32;
            self.last_monitor_name = m.name();
        }

        let window_attributes = Window::default_attributes()
            .with_title("2D UI Engine com WGPU - Pronto para Uso")
            .with_inner_size(winit::dpi::PhysicalSize::new(init_width, init_height))
            .with_resizable(true);

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        let renderer = pollster::block_on(Renderer::new(window)).unwrap();

        // Calcula o layout inicial com as dimensões reais confirmadas pelo SO.
        // Inicializa last_width/height para que o primeiro Resized do usuário calcule o delta corretamente.
        let size = renderer.window.inner_size();
        self.last_width = size.width;
        self.last_height = size.height;
        self.renderer = Some(renderer);
        self.resize_layout(size.width, size.height);
    }
}


/// Inicializador executável.
pub fn run() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = App::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}
