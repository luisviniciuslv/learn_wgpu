# 📖 Guia de Uso: Como Usar o Motor 2D (WGPU)

Este guia ensina **como usar** a estrutura e as funções do motor 2D de retângulos em seus próprios projetos e telas, focando 100% em código prático, parâmetros e eventos (sem teoria de como funciona por dentro).

---

## ⚡ Sumário de Recursos (O que você tem em mãos)

1. **Coordenadas em Pixels**: O canto superior esquerdo é `(0, 0)`. O canto inferior direito é `(largura, altura)`.
2. **`Renderer`**: O motor de desenho.
   * `renderer.clear()`: Limpa a tela antes de desenhar.
   * `renderer.draw_rect(x, y, largura, altura, [r, g, b, a])`: Adiciona um retângulo na fila de desenho.
   * `renderer.present()`: Desenha tudo na tela de uma só vez.
   * `renderer.uniforms.screen_size`: Array `[f32; 2]` contendo a `[largura, altura]` atual da tela.
3. **`Button`**: Utilitário para gerenciar botões interativos e colisões de clique/hover.
a
---

## 🚀 1. Inicializando e Rodando uma Janela Básica

Para criar e rodar uma janela usando o `Renderer` com o `winit`, use esta estrutura padrão. Você pode copiar e colar este código diretamente:

```rust
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    window::Window,a
};
use crate::interface_pronta_pra_uso::renderer::Renderer;

pub struct MinhaApp {
    renderer: Option<Renderer>,
}

impl MinhaApp {
    pub fn new() -> Self {
        Self { renderer: None }
    }
}

impl ApplicationHandler for MinhaApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("Minha Interface WGPU")
            .with_inner_size(winit::dpi::PhysicalSize::new(800, 600));

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        
        // Inicializa o Renderer
        let renderer = pollster::block_on(Renderer::new(window)).unwrap();
        self.renderer = Some(renderer);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: winit::window::WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(ref mut r) = self.renderer {
                    r.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(ref mut r) = self.renderer {
                    r.clear(); // 1. Limpa a tela
                    
                    // ==========================================
                    // SEUS DESENHOS VÃO AQUI (veja exemplos abaixo)
                    // ==========================================
                    
                    let _ = r.present(); // 2. Envia tudo para a GPU
                    r.window.request_redraw(); // Mantém redesenhando a 60+ FPS
                }
            }
            _ => {}
        }
    }
}

// Para iniciar a aplicação:
pub fn executar() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = MinhaApp::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}
```

---

## 🎨 2. Como Desenhar Formas (Básico)

Todas as cores no `draw_rect` usam o formato **RGBA normalizado (valores de `0.0` a `1.0`)**.

### Desenhar um Painel/Fundo Estático
```rust
// Desenha um fundo escuro na barra de ferramentas superior (x: 0, y: 0, w: 800, h: 60)
// Cor: Cinza Escuro [R, G, B, Alpha]
renderer.draw_rect(0.0, 0.0, 800.0, 60.0, [0.15, 0.15, 0.18, 1.0]);
```

### Desenhar uma Borda Neon
Desenhe um retângulo muito fino logo abaixo de uma barra ou em volta de um quadrado para servir de borda:
```rust
// Uma linha horizontal fina de 2px agindo como divisória azul brilhante
renderer.draw_rect(0.0, 58.0, 800.0, 2.0, [0.3, 0.5, 0.9, 1.0]);
```

### Desenhar com Transparência (Alpha)
O motor já possui **Alpha Blending** ativado por padrão. Para desenhar algo translúcido:
```rust
// Retângulo preto com 50% de opacidade (Alpha = 0.5) servindo de sombra
renderer.draw_rect(103.0, 103.0, 200.0, 50.0, [0.0, 0.0, 0.0, 0.5]);
```

---

## 🖱️ 3. Como Criar e Usar Botões Interativos (Hover & Click)

Para criar botões interativos que mudam de cor quando o mouse passa por cima e quando são clicados, siga este passo a passo.

### Passo 1: Defina os botões no seu `App`
```rust
use crate::interface_pronta_pra_uso::renderer::Button;

pub struct MinhaApp {
    renderer: Option<Renderer>,
    // Lista de botões da interface
    botoes: Vec<Button>,
    // Guarda a posição atual do mouse (x, y)
    mouse_pos: (f32, f32),
    // Guarda o ID do botão clicado no momento
    botao_clicado_id: Option<&'static str>,
}
```

### Passo 2: Inicialize os Botões no seu construtor
Defina a posição (`x`, `y`), tamanho (`w`, `h`), rótulo (`label`) e a paleta de cores para cada estado:
```rust
let botao_salvar = Button {
    id: "salvar",
    x: 50.0,
    y: 100.0,
    w: 150.0,
    h: 45.0,
    label: "Salvar",
    default_color: [0.2, 0.6, 0.4, 1.0], // Verde padrão
    hover_color:   [0.3, 0.7, 0.5, 1.0], // Verde claro (hover)
    click_color:   [0.1, 0.4, 0.3, 1.0], // Verde escuro (click)
};
```

### Passo 3: Atualize o Mouse e Cliques nos eventos da Janela (`window_event`)
Adicione estes casos no seu `match event` dentro de `window_event`:

```rust
// 1. Atualizar a posição do mouse quando ele mover
WindowEvent::CursorMoved { position, .. } => {
    self.mouse_pos = (position.x as f32, position.y as f32);
}

// 2. Detectar se o clique do mouse ocorreu em cima do botão
WindowEvent::MouseInput { state, button, .. } => {
    if button == MouseButton::Left {
        if state == ElementState::Pressed {
            for btn in &self.botoes {
                // Verifica se o mouse clicou dentro da área do botão
                if btn.is_hovered(self.mouse_pos.0, self.mouse_pos.1) {
                    self.botao_clicado_id = Some(btn.id);
                    
                    // EXECUTE A AÇÃO AQUI:
                    if btn.id == "salvar" {
                        println!("Botão Salvar Clicado!");
                    }
                }
            }
        } else {
            // Quando soltar o botão do mouse, reseta o ID clicado
            self.botao_clicado_id = None;
        }
    }
}
```

### Passo 4: Desenhe os Botões dinamicamente no `RedrawRequested`
No seu loop de desenho, itere sobre os botões e escolha a cor certa com base nos estados interativos:

```rust
for btn in &self.botoes {
    let is_hovered = btn.is_hovered(self.mouse_pos.0, self.mouse_pos.1);
    let is_clicked = self.botao_clicado_id == Some(btn.id);

    // Seleciona a cor ativa
    let cor_ativa = if is_clicked {
        btn.click_color
    } else if is_hovered {
        btn.hover_color
    } else {
        btn.default_color
    };

    // [OPCIONAL] Desenha uma sombra suave 3px deslocada para o lado/baixo
    renderer.draw_rect(btn.x + 3.0, btn.y + 3.0, btn.w, btn.h, [0.0, 0.0, 0.0, 0.3]);

    // Desenha o corpo do botão principal
    renderer.draw_rect(btn.x, btn.y, btn.w, btn.h, cor_ativa);
    
    // [OPCIONAL] Desenha uma borda superior branca translúcida se estiver em hover
    if is_hovered {
        renderer.draw_rect(btn.x, btn.y, btn.w, 3.0, [1.0, 1.0, 1.0, 0.4]);
    }
}
```

---

## 📈 4. Como Criar Animações Fluídas

Para animar qualquer elemento, você precisa criar uma variável de estado em seu `App` (como um timer, progresso ou coordenada de animação) e alterá-la continuamente no frame.

### Exemplo: Barra de Progresso Animada
1. No seu `App`, crie:
   ```rust
   pub struct MinhaApp {
       // ... outros estados
       progresso: f32, // Começa em 0.0, vai até 1.0
       animando: bool,
   }
   ```
2. Na sua lógica antes do desenho (ou no início do `RedrawRequested`):
   ```rust
   if self.animando {
       self.progresso += 0.01; // Velocidade do preenchimento
       if self.progresso > 1.0 {
           self.progresso = 1.0;
           self.animando = false; // Para de animar quando chega a 100%
       }
   }
   ```
3. Na hora de desenhar:
   ```rust
   let total_largura = 300.0;
   let x = 100.0;
   let y = 200.0;
   let altura = 20.0;

   // A) Desenha o fundo cinza escuro da barra
   renderer.draw_rect(x, y, total_largura, altura, [0.2, 0.2, 0.25, 1.0]);

   // B) Calcula a largura preenchida com base no progresso atual
   let largura_preenchida = total_largura * self.progresso;

   // C) Desenha o preenchimento verde esmeralda brilhante
   renderer.draw_rect(x, y, largura_preenchida, altura, [0.1, 0.8, 0.5, 1.0]);
   ```

---

## 🖥️ 5. Como Lidar com Janelas Responsivas (Tamanho Dinâmico)

Se o usuário redimensionar a janela do app, você pode fazer com que elementos fiquem sempre grudados nos cantos ou centralizados.
Use `renderer.uniforms.screen_size[0]` para a **Largura atual** da tela, e `renderer.uniforms.screen_size[1]` para a **Altura atual**.

### Manter um elemento centralizado na tela
```rust
let tela_w = renderer.uniforms.screen_size[0];
let tela_h = renderer.uniforms.screen_size[1];

let obj_w = 200.0;
let obj_h = 100.0;

// Centraliza horizontalmente e verticalmente:
let x = (tela_w - obj_w) / 2.0;
let y = (tela_h - obj_h) / 2.0;

renderer.draw_rect(x, y, obj_w, obj_h, [1.0, 0.5, 0.0, 1.0]);
```

### Barra inferior sempre colada no rodapé da janela
```rust
let tela_w = renderer.uniforms.screen_size[0];
let tela_h = renderer.uniforms.screen_size[1];

let barra_h = 40.0;

// x=0, y=rodapé da tela menos a altura da barra
renderer.draw_rect(0.0, tela_h - barra_h, tela_w, barra_h, [0.1, 0.1, 0.1, 1.0]);
```
