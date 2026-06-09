use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::KeyCode;

use crate::menu::types::ArrowId;

use super::state::App;

impl App {
    pub(super) fn handle_keyboard(&mut self, event_loop: &ActiveEventLoop, key_code: KeyCode) {
        // 1. COMANDOS GLOBAIS: Funcionam em qualquer tela do aplicativo
        if key_code == KeyCode::F11 {
            self.toggle_fullscreen();
            return;
        }

        // 2. COMANDOS DO XADREZ: Se o jogo estiver ativo, intercepta as teclas e sai da função imediatamente
        if let Some(ref mut game) = self.chess_game {
            let cursor = game.cursor.unwrap_or((0, 0)); // Garante que o cursor nunca seja None aqui, mas sim (0, 0) se não tiver sido setado

            // Verifica se o jogador atual está a jogar com as Pretas
            let is_black = game.player_color == crate::menu::chess::PieceColor::Black;

            match key_code {
                KeyCode::Escape => self.back_menu(event_loop), // Deixa o ESC voltar ao menu
                KeyCode::ArrowUp => {
                    if is_black {
                        // Para as pretas, ir para cima visualmente significa avançar para a linha 7
                        if cursor.0 < 7 {
                            game.cursor = Some((cursor.0 + 1, cursor.1));
                        }
                    } else {
                        // Para as brancas, ir para cima visualmente significa avançar para a linha 0
                        if cursor.0 > 0 {
                            game.cursor = Some((cursor.0 - 1, cursor.1));
                        }
                    }
                }
                KeyCode::ArrowDown => {
                    if is_black {
                        // Para as pretas, ir para baixo significa recuar para a linha 0
                        if cursor.0 > 0 {
                            game.cursor = Some((cursor.0 - 1, cursor.1));
                        }
                    } else {
                        // Para as brancas, ir para baixo significa avançar para a linha 7
                        if cursor.0 < 7 {
                            game.cursor = Some((cursor.0 + 1, cursor.1));
                        }
                    }
                }
                KeyCode::ArrowLeft => {
                    if is_black {
                        // Para as pretas, a esquerda do ecrã corresponde à coluna 7
                        if cursor.1 < 7 {
                            game.cursor = Some((cursor.0, cursor.1 + 1));
                        }
                    } else {
                        // Para as brancas, a esquerda do ecrã corresponde à coluna 0
                        if cursor.1 > 0 {
                            game.cursor = Some((cursor.0, cursor.1 - 1));
                        }
                    }
                }
                KeyCode::ArrowRight => {
                    if is_black {
                        // Para as pretas, a direita do ecrã corresponde à coluna 0
                        if cursor.1 > 0 {
                            game.cursor = Some((cursor.0, cursor.1 - 1));
                        }
                    } else {
                        // Para as brancas, a direita do ecrã corresponde à coluna 7
                        if cursor.1 < 7 {
                            game.cursor = Some((cursor.0, cursor.1 + 1));
                        }
                    }
                }
                KeyCode::Enter | KeyCode::Space => {
                    // Executa o movimento ou seleção apenas dentro do tabuleiro
                    game.select_or_move(
                        game.cursor.unwrap_or((0, 0)).0,
                        game.cursor.unwrap_or((0, 0)).1,
                    );
                }
                _ => {}
            }

            if let Some(ref renderer) = self.renderer {
                renderer.window.request_redraw(); // Força a tela a atualizar o visual do cursor
            }
            return; // IMPORTANTE: Sai da função aqui para NUNCA executar os comandos do menu abaixo
        }

        // 3. COMANDOS DO MENU PRINCIPAL: Só serão executados se "self.chess_game" for None
        match key_code {
            KeyCode::Escape => self.back_menu(event_loop),
            KeyCode::ArrowLeft => self.move_selection(-1),
            KeyCode::ArrowRight => self.move_selection(1),
            KeyCode::Enter | KeyCode::Space => self.try_enter_submenu(),
            _ => {}
        }
    }

    pub(super) fn handle_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        self.mouse_pos = self.physical_to_logical(position.x as f32, position.y as f32);
        self.update_fullscreen_button_hover();
        if let Some(ref renderer) = self.renderer {
            renderer.window.request_redraw();
        }
    }

    pub(super) fn handle_mouse_input(&mut self, state: ElementState, button: MouseButton) {
        if button == MouseButton::Left {
            if state == ElementState::Pressed {
                // 1. Verifica o botão de Tela Cheia primeiro (comportamento global)
                self.update_fullscreen_button_hover();
                if self.should_show_fullscreen_button() && self.hovered_fullscreen_button {
                    self.pressed_fullscreen_button = true;
                    self.toggle_fullscreen();
                    if let Some(ref renderer) = self.renderer {
                        renderer.window.request_redraw();
                    }
                    return;
                }

                // 2. O xadrez tenta interceptar o clique ANTES do menu normal!
                if let Some(ref mut game) = self.chess_game {
                    let mx = self.mouse_pos.0;
                    let my = self.mouse_pos.1;
                    let vp_w = self.viewport.width;
                    let vp_h = self.viewport.height;

                    // Tenta converter o clique do mouse em uma casa do tabuleiro
                    if let Some((row, col)) = game.mouse_to_square(mx, my, vp_w, vp_h) {
                        game.select_or_move(row, col); // Executa a nossa lógica de seleção/movimento!

                        if let Some(ref renderer) = self.renderer {
                            renderer.window.request_redraw(); // Pede para a tela redesenhar com a mudança
                        }
                        return; // Encerra a função aqui para não clicar nas coisas escondidas do menu!
                    }
                }

                // 3. Controles do menu principal (só rodam se o clique NÃO foi dentro do tabuleiro de xadrez)
                if let Some(arrow) = self.hovered_arrow {
                    self.pressed_arrow = Some(arrow);
                    match arrow {
                        ArrowId::Left => self.move_selection(-1),
                        ArrowId::Right => self.move_selection(1),
                        ArrowId::Back => self.pop_menu(),
                    }
                } else {
                    let mx = self.mouse_pos.0;
                    let my = self.mouse_pos.1;
                    if let Some(idx) = self.card_hit_test(mx, my) {
                        let selected = self.menu_stack.last().map(|m| m.selected).unwrap_or(0);
                        if idx as i32 == selected {
                            self.try_enter_submenu();
                        } else {
                            let delta = idx as i32 - selected;
                            self.move_selection(delta);
                        }
                    }
                }
            } else {
                // Quando o botão do mouse é Solto (Released)
                self.pressed_arrow = None;
                self.pressed_fullscreen_button = false;
                if self.chess_game.is_some() {
                    if let Some((pw, ph)) = self.pending_size {
                        if let Some(ref renderer) = self.renderer {
                            let _ = renderer
                                .window
                                .request_inner_size(winit::dpi::PhysicalSize::new(pw, ph));
                        }
                    }
                }
            }
        }

        // Redesenha a tela de qualquer forma no final para garantir fluidez gráfica
        if let Some(ref renderer) = self.renderer {
            renderer.window.request_redraw();
        }
    }
}
