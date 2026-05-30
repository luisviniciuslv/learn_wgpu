// Tipos de dados puros usados pelo menu.
// Não contêm lógica de renderização nem de eventos — apenas estruturas e seu comportamento básico.

use super::animation::{ease_out, lerp};

// =============================================================================
//  Item de Menu
// =============================================================================

/// Representa um item individual dentro da árvore de navegação do menu.
/// Pode ser uma folha (sem filhos) ou um nó (com submenu aninhado).
#[derive(Clone)]
#[allow(dead_code)] // `id` e `label` ainda não são renderizados (falta renderização de texto)
pub struct MenuItem {
    pub id: &'static str,
    pub label: &'static str,
    pub children: Option<Vec<MenuItem>>,
}

// =============================================================================
//  Estado do Menu
// =============================================================================

/// Guarda o estado completo de uma tela de menu: quais itens existem,
/// qual está selecionado e qual é o progresso da animação de transição.
pub struct MenuState {
    pub items: Vec<MenuItem>,
    pub selected: i32,

    // Campos de animação de deslize suave do carrossel
    pub anim_from: i32,
    pub anim_to: i32,
    pub anim_t: f32,
    pub animating: bool,
}

impl MenuState {
    pub fn new(items: Vec<MenuItem>) -> Self {
        Self {
            items,
            selected: 0,
            anim_from: 0,
            anim_to: 0,
            anim_t: 1.0, // Começa em 1.0 (animação concluída) para não iniciar animando
            animating: false,
        }
    }

    /// Retorna a seleção como float interpolado para o deslize suave do carrossel.
    /// Durante uma animação, retorna um valor entre `anim_from` e `anim_to`.
    pub fn selected_float(&self) -> f32 {
        if !self.animating {
            return self.selected as f32;
        }
        let t = ease_out(self.anim_t);
        lerp(self.anim_from as f32, self.anim_to as f32, t)
    }
}

// =============================================================================
//  Botões de Navegação
// =============================================================================

/// Identificador exclusivo de cada botão de navegação.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ArrowId {
    Left,
    Right,
    Back,
}

/// Representa um botão interativo na tela.
/// Coordenadas em pixels LÓGICOS (espaço do viewport), atualizadas a cada resize.
pub struct ArrowButton {
    pub id: ArrowId,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl ArrowButton {
    /// Teste de colisão Bounding Box (AABB) em pixels lógicos.
    /// Retorna `true` se o ponto (mx, my) está dentro do botão.
    pub fn contains(&self, mx: f32, my: f32) -> bool {
        mx >= self.x && mx <= self.x + self.w && my >= self.y && my <= self.y + self.h
    }
}
