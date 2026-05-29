use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use super::renderer::{Renderer, Button};

pub struct App {
    renderer: Option<Renderer>,
    buttons: Vec<Button>,
    
    // Estados interativos
    mouse_pos: (f32, f32),
    clicked_button_id: Option<&'static str>,
    
    // Estados para animações
    animation_progress: f32, // Progresso da animação (0.0 a 1.0)
    animating: bool,
}

impl App {
    pub fn new() -> Self {
        let buttons = vec![
            Button {
                id: "animate",
                x: 50.0,
                y: 100.0,
                w: 220.0,
                h: 50.0,
                label: "Disparar Animação",
                default_color: [0.2, 0.4, 0.8, 1.0], // Azul elegante
                hover_color: [0.3, 0.5, 0.9, 1.0],   // Azul claro
                click_color: [0.1, 0.3, 0.7, 1.0],   // Azul escuro
            },
            Button {
                id: "reset",
                x: 50.0,
                y: 170.0,
                w: 220.0,
                h: 50.0,
                label: "Resetar",
                default_color: [0.8, 0.3, 0.3, 1.0], // Vermelho suave
                hover_color: [0.9, 0.4, 0.4, 1.0],   // Vermelho claro
                click_color: [0.7, 0.2, 0.2, 1.0],   // Vermelho escuro
            },
        ];

        Self {
            renderer: None,
            buttons,
            mouse_pos: (0.0, 0.0),
            clicked_button_id: None,
            animation_progress: 0.0,
            animating: false,
        }
    }

    fn update(&mut self) {
        // Atualizar lógica de animação simples
        if self.animating {
            self.animation_progress += 0.02; // Aumentar progresso
            if self.animation_progress >= 1.0 {
                self.animation_progress = 1.0;
                self.animating = false;
            }
        } else if self.clicked_button_id == Some("reset") {
            // Decréscimo suave para voltar
            self.animation_progress -= 0.04;
            if self.animation_progress <= 0.0 {
                self.animation_progress = 0.0;
                self.clicked_button_id = None;
            }
        }
    }

    fn render(&mut self) -> anyhow::Result<()> {
        let renderer = match &mut self.renderer {
            Some(r) => r,
            None => return Ok(()),
        };

        // Solicita redesenho constante para animação fluida
        renderer.clear();

        // 1. Desenhar Fundo Escuro com degradê estilizado via retângulos pequenos
        // wgpu limpa com (0.1, 0.1, 0.1) por padrão no render_pass.
        // Vamos desenhar um cabeçalho/barra superior bonita:
        renderer.draw_rect(0.0, 0.0, renderer.uniforms.screen_size[0], 60.0, [0.15, 0.15, 0.18, 1.0]);
        // Borda inferior da barra
        renderer.draw_rect(0.0, 58.0, renderer.uniforms.screen_size[0], 2.0, [0.3, 0.5, 0.9, 0.8]);

        // 2. Desenhar Botões Interativos
        for button in &self.buttons {
            let is_hovered = button.is_hovered(self.mouse_pos.0, self.mouse_pos.1);
            let is_clicked = self.clicked_button_id == Some(button.id);

            let color = if is_clicked {
                button.click_color
            } else if is_hovered {
                button.hover_color
            } else {
                button.default_color
            };

            // Desenhar sombra sutil do botão
            renderer.draw_rect(button.x + 3.0, button.y + 3.0, button.w, button.h, [0.0, 0.0, 0.0, 0.4]);
            // Desenhar botão real
            renderer.draw_rect(button.x, button.y, button.w, button.h, color);
            
            // Desenhar borda de destaque se hovered
            if is_hovered {
                renderer.draw_rect(button.x, button.y, button.w, 3.0, [1.0, 1.0, 1.0, 0.5]);
            }
        }

        // 3. Desenhar Elementos Animados com base no progresso
        // Vamos desenhar uma barra de carregamento estilizada
        let bar_width = 400.0;
        let bar_x = 300.0;
        let bar_y = 120.0;
        // Fundo da barra
        renderer.draw_rect(bar_x, bar_y, bar_width, 20.0, [0.2, 0.2, 0.25, 1.0]);
        // Progresso preenchido (verde esmeralda)
        let fill_width = bar_width * self.animation_progress;
        renderer.draw_rect(bar_x, bar_y, fill_width, 20.0, [0.1, 0.8, 0.5, 1.0]);

        // 4. Desenhar caixa/painel interativo se movendo e mudando de tamanho com a animação
        let box_size = 50.0 + (self.animation_progress * 100.0);
        let box_x = 300.0 + (self.animation_progress * 150.0);
        let box_y = 200.0;
        // Cor muda de laranja para roxo neon de acordo com a animação
        let r = 0.9 - (self.animation_progress * 0.4);
        let g = 0.4 + (self.animation_progress * 0.1);
        let b = 0.1 + (self.animation_progress * 0.8);
        renderer.draw_rect(box_x, box_y, box_size, box_size, [r, g, b, 0.9]);

        // 5. Cursor customizado de teste (seguindo o mouse) para mostrar controle absoluto
        renderer.draw_rect(self.mouse_pos.0 - 4.0, self.mouse_pos.1 - 4.0, 8.0, 8.0, [1.0, 0.9, 0.0, 0.8]);

        renderer.present()?;
        Ok(())
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
            WindowEvent::Resized(size) => {
                if let Some(ref mut renderer) = self.renderer {
                    renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => event_loop.exit(),
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos = (position.x as f32, position.y as f32);
                if let Some(ref renderer) = self.renderer {
                    renderer.window.request_redraw();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    if state == ElementState::Pressed {
                        // Detectar cliques em botões
                        for btn in &self.buttons {
                            if btn.is_hovered(self.mouse_pos.0, self.mouse_pos.1) {
                                self.clicked_button_id = Some(btn.id);
                                if btn.id == "animate" {
                                    self.animating = true;
                                    if self.animation_progress >= 1.0 {
                                        self.animation_progress = 0.0; // reinicia
                                    }
                                }
                                break;
                            }
                        }
                    } else {
                        // Soltou o mouse
                        if self.clicked_button_id != Some("reset") {
                            self.clicked_button_id = None;
                        }
                    }
                }
                if let Some(ref renderer) = self.renderer {
                    renderer.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                self.update();
                if let Err(e) = self.render() {
                    eprintln!("Erro de Render: {:?}", e);
                    event_loop.exit();
                }
                if let Some(ref renderer) = self.renderer {
                    renderer.window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("2D UI Engine com WGPU - Pronto para Uso")
            .with_inner_size(winit::dpi::PhysicalSize::new(800, 600));

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        
        let renderer = pollster::block_on(Renderer::new(window)).unwrap();
        self.renderer = Some(renderer);
    }
}

pub fn run() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = App::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}
