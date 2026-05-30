# 🖥️ Motor 2D Ultra-Simples com WGPU — Pronto para Uso

Se você desistiu da loucura que é desenhar um simples triângulo com shaders puros do WGPU, **este módulo foi feito para você**. 

Criamos uma camada de abstração extremamente fina e poderosa em cima do WGPU. Ela faz todo o trabalho sujo de alocar buffers de GPU, configurar shaders complexos e converter coordenadas loucas de clip space.

Para você, sobrou apenas a parte divertida: **dizer onde desenhar retângulos em pixels e responder a cliques do mouse.**

---

## 🚀 Como Executar

Já configuramos o ponto de entrada principal no seu [main.rs](file:///c:/dev/learn_wgpu/src/main.rs). Para rodar e ver a interface interativa rodando agora mesmo, basta abrir um terminal na pasta do projeto e executar:

```bash
cargo run
```

Você verá uma tela com:
1. Uma barra superior escura elegante com borda neon.
2. Dois botões interativos em 2D que mudam de cor ao passar o mouse (*hover*) e ao clicar (*click*).
3. Uma barra de progresso verde esmeralda animada que se enche gradualmente quando você clica em **"Disparar Animação"**.
4. Um quadrado neon animado que cresce, translada e muda de cor dinamicamente com base no progresso.
5. Um cursor customizado quadrado que segue seu mouse fielmente na tela.

---

## 📂 Estrutura de Arquivos Criada

Criamos uma pasta dedicada chamada [interface_pronta_pra_uso](file:///c:/dev/learn_wgpu/src/interface_pronta_pra_uso/) contendo:

*   [shader.wgsl](file:///c:/dev/learn_wgpu/src/interface_pronta_pra_uso/shader.wgsl) — O shader responsável por receber as coordenadas de pixel da CPU, ler o tamanho da janela e convertê-las para clip space da GPU.
*   [renderer.rs](file:///c:/dev/learn_wgpu/src/interface_pronta_pra_uso/renderer.rs) — O motor 2D de retângulos. Ele encapsula a inicialização do WGPU, cria os Vertex & Index Buffers e expõe a função `draw_rect()`.
*   [demo.rs](file:///c:/dev/learn_wgpu/src/interface_pronta_pra_uso/demo.rs) — A aplicação interativa. Cuida do loop do Winit, da lógica de hover/clique e das atualizações de animação.
*   [mod.rs](file:///c:/dev/learn_wgpu/src/interface_pronta_pra_uso/mod.rs) — Expõe os arquivos do módulo para o Rust.

---

## 🧠 Como Funciona? (Explicado sem complicação)

### 1. Adeus coordenadas complexas: Agora tudo é Pixels!
No triângulo anterior, você precisava calcular posições de `-1.0` a `+1.0` (Clip Space). Isso é horrível para criar UIs.
Agora, no [shader.wgsl](file:///c:/dev/learn_wgpu/src/interface_pronta_pra_uso/shader.wgsl), nós enviamos o tamanho da tela (`screen_size` via Uniform Buffer). O shader converte as coordenadas de pixels reais para Clip Space automaticamente:

```wgsl
// Converte pixel (x, y) → clip space (-1..+1)
let cx = (in.position.x / uniforms.screen_size.x) * 2.0 - 1.0;
let cy = 1.0 - (in.position.y / uniforms.screen_size.y) * 2.0;
```

A origem `(0,0)` agora é o **canto superior esquerdo** da tela. Se sua janela tem `800x600`:
*   `(0, 0)` é o topo esquerdo.
*   `(800, 600)` é o canto inferior direito.
*   `(400, 300)` é o centro exato da tela.

---

### 2. A Mágica do `draw_rect` (Acumulando Geometria)
Em vez de desenhar um retângulo por chamada (o que seria lento), o `Renderer` mantém vetores locais na CPU (`vertices_data` e `indices_data`). 

Quando você faz:
```rust
renderer.draw_rect(x, y, largura, altura, [r, g, b, a]);
```

Nós adicionamos **4 vértices** (os 4 cantos do quadrado) e **6 índices** (que definem os 2 triângulos que formam esse quadrado) a esses vetores. 

No final do frame, o método `present()` envia todos os dados acumulados de uma só vez para os buffers dinâmicos da GPU (`vertex_buffer` e `index_buffer` criados com uso `COPY_DST`) e chama o comando indexado:    

```rust
// Copia tudo em uma única transação rápida
self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices_data));
self.queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&self.indices_data));

// Desenha tudo de uma vez
render_pass.draw_indexed(0..indices_count, 0, 0..1);
```
Isso é extremamente rápido e eficiente. Permite que você desenhe centenas de retângulos sem queda de performance.

---

### 3. Colisão e Interações simples
Para saber se o mouse está em cima de um retângulo (como um botão), criamos a estrutura `Button` com a função `is_hovered`:

```rust
pub fn is_hovered(&self, mx: f32, my: f32) -> bool {
    mx >= self.x && mx <= self.x + self.w && my >= self.y && my <= self.y + self.h
}
```

No evento `WindowEvent::CursorMoved`, salvamos a posição do mouse e atualizamos o estado. Se o botão for clicado, guardamos o `id` dele e disparamos eventos.

---

### 4. Fazendo Animações Fluídas
Para animar coisas em tempo real de forma suave (a 60 FPS ou mais), nós solicitamos redesenhos contínuos da tela através do Winit. 
No evento `WindowEvent::RedrawRequested` em [demo.rs](file:///c:/dev/learn_wgpu/src/interface_pronta_pra_uso/demo.rs):

```rust
WindowEvent::RedrawRequested => {
    self.update(); // Avança estados físicos e timers da animação
    self.render().unwrap(); // Emite retângulos com novos tamanhos/cores
    renderer.window.request_redraw(); // Agenda o próximo frame imediatamente
}
```

O progresso `animation_progress` cresce suavemente até `1.0`. Usamos esse valor para interpolar cores e tamanhos:

```rust
// O tamanho do quadrado muda dinamicamente com base no progresso
let box_size = 50.0 + (self.animation_progress * 100.0);
```

---

## 🛠️ Como você pode estender isso?

Agora você tem o playground perfeito! Para criar coisas novas:
1. Abra [demo.rs](file:///c:/dev/learn_wgpu/src/interface_pronta_pra_uso/demo.rs).
2. Vá no método `render()`.
3. Use `renderer.draw_rect(...)` para adicionar barras de vida, menus, janelas internas, painéis, textos (usando blocos de retângulos para cada letra, futuramente), ou partículas flutuantes!
4. Mude os valores em `update()` para aplicar física, gravidade ou novas trajetórias geométricas.

Divirta-se! Você acabou de criar a semente de um motor gráfico e de interface 2D do zero rodando em cima de Vulkan/WGPU nativo! 🚀
