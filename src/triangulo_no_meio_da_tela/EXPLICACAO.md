# 🔺 Como Desenhar um Triângulo na Tela com wgpu — Passo a Passo

Este documento explica **cada passo** do código em `triangulo.rs` e `shader.wgsl` para que um triângulo apareça na tela. Se você é iniciante em wgpu, leia na ordem — cada seção depende da anterior.

---

## Índice

1. [Visão Geral da Arquitetura](#1-visão-geral-da-arquitetura)
2. [O Shader WGSL — O Programa que Roda na GPU](#2-o-shader-wgsl--o-programa-que-roda-na-gpu)
3. [Passo 1 — Criar a Instance](#3-passo-1--criar-a-instance)
4. [Passo 2 — Criar a Surface](#4-passo-2--criar-a-surface)
5. [Passo 3 — Escolher um Adapter](#5-passo-3--escolher-um-adapter)
6. [Passo 4 — Solicitar o Device e a Queue](#6-passo-4--solicitar-o-device-e-a-queue)
7. [Passo 5 — Configurar a Surface](#7-passo-5--configurar-a-surface)
8. [Passo 6 — Carregar o Shader](#8-passo-6--carregar-o-shader)
9. [Passo 7 — Criar o Pipeline Layout](#9-passo-7--criar-o-pipeline-layout)
10. [Passo 8 — Criar o Render Pipeline](#10-passo-8--criar-o-render-pipeline)
11. [Passo 9 — Renderizar um Frame](#11-passo-9--renderizar-um-frame)
12. [Passo 10 — O Event Loop e o App](#12-passo-10--o-event-loop-e-o-app)
13. [Resumo do Fluxo Completo](#13-resumo-do-fluxo-completo)

---

## 1. Visão Geral da Arquitetura

Para um triângulo aparecer na tela, precisamos:

```
Janela (winit) → Surface → GPU (wgpu) → Shader (WGSL) → Pixels na tela
```

O caminho completo é:

1. **Criar uma janela** (winit cuida disso)
2. **Conectar a janela à GPU** (Instance → Surface → Adapter → Device)
3. **Escrever o programa da GPU** (shader WGSL: vertex + fragment)
4. **Montar o pipeline** (junta o shader com as configurações de rasterização)
5. **A cada frame**: pegar a textura da tela → criar comandos → desenhar → apresentar

---

## 2. O Shader WGSL — O Programa que Roda na GPU

O arquivo `shader.wgsl` contém o código que a **GPU executa**. Ele tem duas funções:

- **Vertex Shader** (`vs_main`): calcula a **posição** de cada vértice
- **Fragment Shader** (`fs_main`): calcula a **cor** de cada pixel do triângulo

### 2.1 A Struct `VertexOutput`

```wgsl
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) vert_pos: vec3<f32>,
    @location(1) color: vec3<f32>,
}
```

Esta struct é a **ponte entre o vertex shader e o fragment shader**. O vertex shader produz um `VertexOutput` para cada vértice, e a GPU interpola esses valores automaticamente para cada pixel dentro do triângulo antes de passá-los ao fragment shader.

| Campo | Tipo | Atributo | O que faz |
|---|---|---|---|
| `clip_position` | `vec4<f32>` | `@builtin(position)` | **Obrigatório**. A posição do vértice em *clip space* (sistema de coordenadas normalizado da GPU). O `@builtin(position)` diz à GPU: "use este valor como a posição final do vértice". Sem isso, a GPU não sabe onde colocar o vértice. |
| `vert_pos` | `vec3<f32>` | `@location(0)` | Um dado personalizado passado ao fragment shader. Aqui armazena a posição xyz do vértice. O `@location(0)` diz que este dado vai no "slot 0" da comunicação vertex→fragment. |
| `color` | `vec3<f32>` | `@location(1)` | Outro dado personalizado no "slot 1". Armazena a cor RGB do vértice. |

#### O que é Clip Space?

Clip space é o sistema de coordenadas da GPU:
- **X**: -1.0 (esquerda) até +1.0 (direita)
- **Y**: -1.0 (baixo) até +1.0 (cima)
- **Z**: 0.0 (perto) até +1.0 (longe) — para depth
- **W**: deve ser 1.0 para coordenadas normais (usado em projeção perspectiva)

```
        +1 Y
         |
         |
-1 X ----+---- +1 X
         |
         |
        -1 Y
```

#### O que é `@builtin` vs `@location`?

- **`@builtin(position)`**: um valor **especial** que a GPU sabe interpretar automaticamente. `position` é a posição do vértice. Outros builtins incluem `vertex_index`, `instance_index`, etc.
- **`@location(N)`**: um valor **personalizado** que você define. É um "slot numerado" para passar dados entre estágios do pipeline. Você escolhe o número (0, 1, 2...).

### 2.2 O Vertex Shader — `vs_main`

```wgsl
@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(1 - i32(in_vertex_index)) * 0.5;
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.vert_pos = out.clip_position.xyz;
    out.color = vec3<f32>(0.3, 0.2, 0.1);
    return out;
}
```

#### Linha por linha:

**`@vertex`** — Marca esta função como vertex shader. A GPU sabe que deve executá-la uma vez para cada vértice.

**`@builtin(vertex_index) in_vertex_index: u32`** — A GPU fornece automaticamente o índice do vértice que está sendo processado. Quando chamamos `draw(0..3, 0..1)` no Rust, a GPU executa esta função 3 vezes, com `in_vertex_index` = 0, 1 e 2.

**`var out: VertexOutput;`** — Cria uma variável mutável do tipo `VertexOutput`. Em WGSL, `var` cria variáveis mutáveis e `let` cria constantes.

#### A Matemática dos Vértices

```wgsl
let x = f32(1 - i32(in_vertex_index)) * 0.5;
let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
```

Estas duas linhas geram as coordenadas X e Y para 3 vértices **sem usar um buffer de vértices**. Vamos calcular para cada índice:

| `in_vertex_index` | Cálculo de X: `(1 - index) * 0.5` | Cálculo de Y: `((index & 1) * 2 - 1) * 0.5` | Posição (X, Y) |
|---|---|---|---|
| **0** | `(1 - 0) * 0.5` = **0.5** | `((0 & 1) * 2 - 1) * 0.5` = `(0 - 1) * 0.5` = **-0.5** | (0.5, -0.5) — canto inferior direito |
| **1** | `(1 - 1) * 0.5` = **0.0** | `((1 & 1) * 2 - 1) * 0.5` = `(2 - 1) * 0.5` = **0.5** | (0.0, 0.5) — topo central |
| **2** | `(1 - 2) * 0.5` = **-0.5** | `((2 & 1) * 2 - 1) * 0.5` = `(0 - 1) * 0.5` = **-0.5** | (-0.5, -0.5) — canto inferior esquerdo |

> **Nota sobre `& 1u`**: O operador `&` é um AND bit-a-bit. `index & 1u` retorna 1 se o índice é ímpar e 0 se é par. Isso cria o padrão: vértice 0 → baixo, vértice 1 → cima, vértice 2 → baixo.

O triângulo resultante em clip space:

```
           (0.0, 0.5)
              /\
             /  \
            /    \
           /      \
          /________\
  (-0.5,-0.5)    (0.5,-0.5)
```

**`out.clip_position = vec4<f32>(x, y, 0.0, 1.0);`**
- `x, y`: posição calculada
- `0.0`: profundidade Z (na superfície da câmera)
- `1.0`: componente W (obrigatório para coordenadas homogêneas; 1.0 = sem efeito de perspectiva)

**`out.vert_pos = out.clip_position.xyz;`** — Copia as 3 primeiras componentes (x, y, z) para passar ao fragment shader. O `.xyz` é um *swizzle* — uma forma compacta de acessar componentes de vetores em WGSL.

**`out.color = vec3<f32>(0.3, 0.2, 0.1);`** — Define uma cor marrom fixa para todos os vértices.

### 2.3 O Fragment Shader — `fs_main`

```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
```

**`@fragment`** — Marca como fragment shader. A GPU executa esta função **para cada pixel** que o triângulo cobre na tela.

**`in: VertexOutput`** — Recebe os dados do vertex shader. Mas atenção: **os valores chegam interpolados**! Se os 3 vértices tivessem cores diferentes (vermelho, verde, azul), cada pixel no interior do triângulo receberia uma mistura suave das 3 cores. Como todos os vértices têm a mesma cor `(0.3, 0.2, 0.1)`, a cor é uniforme.

**`-> @location(0) vec4<f32>`** — O retorno vai para o *color attachment* no slot 0. Isso corresponde ao primeiro (e único) target configurado no `FragmentState` do Rust.

**`vec4<f32>(in.color, 1.0)`** — Combina a cor RGB `(0.3, 0.2, 0.1)` com alpha `1.0` (totalmente opaco) para formar um RGBA.

---

## 3. Passo 1 — Criar a Instance

```rust
let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
    backends: wgpu::Backends::PRIMARY,
    flags: Default::default(),
    memory_budget_thresholds: Default::default(),
    backend_options: Default::default(),
});
```

A `Instance` é o **ponto de entrada** do wgpu. É ela que descobre quais backends gráficos estão disponíveis no sistema.

| Campo | Valor | Explicação |
|---|---|---|
| `backends` | `Backends::PRIMARY` | Quais APIs gráficas usar. `PRIMARY` seleciona Vulkan (Linux/Windows/Android), Metal (macOS/iOS) e DX12 (Windows). São os backends de alto desempenho. Poderia ser `Backends::GL` para OpenGL ou `Backends::all()` para todos. |
| `flags` | `Default::default()` | Flags de debug e validação. O padrão é sem validação extra. |
| `memory_budget_thresholds` | `Default::default()` | Limites de uso de memória da GPU. O padrão não impõe limites. |
| `backend_options` | `Default::default()` | Configurações específicas de cada backend (ex: opções Vulkan). O padrão aceita tudo. |

---

## 4. Passo 2 — Criar a Surface

```rust
let surface = instance.create_surface(window.clone()).unwrap();
```

A `Surface` é a **conexão entre a janela do sistema operacional e a GPU**. É nela que os frames renderizados são apresentados. Precisa de um handle da janela (do winit).

A surface não é uma textura — ela **produz** texturas a cada frame via `get_current_texture()`.

---

## 5. Passo 3 — Escolher um Adapter

```rust
let adapter = instance
    .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    })
    .await?;
```

O `Adapter` representa uma **GPU física** instalada no sistema. Se você tem uma placa de vídeo dedicada e uma integrada, cada uma é um adapter diferente.

| Campo | Valor | Explicação |
|---|---|---|
| `power_preference` | `PowerPreference::default()` | Qual GPU preferir. `default()` é `LowPower` (GPU integrada, economiza bateria). Poderia ser `HighPerformance` (GPU dedicada, mais potente). |
| `compatible_surface` | `Some(&surface)` | Garante que o adapter escolhido consiga renderizar na nossa surface/janela. Sem isso, poderíamos selecionar uma GPU que não consegue apresentar nada naquela janela. |
| `force_fallback_adapter` | `false` | Se `true`, usa um renderizador por software (sem GPU real). Útil para testes, mas muito lento. |

---

## 6. Passo 4 — Solicitar o Device e a Queue

```rust
let (device, queue) = adapter
    .request_device(&wgpu::DeviceDescriptor {
        label: None,
        required_features: wgpu::Features::empty(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        required_limits: wgpu::Limits::default(),
        memory_hints: Default::default(),
        trace: wgpu::Trace::Off,
    })
    .await?;
```

O **`Device`** é o dispositivo **lógico** da GPU. É a interface principal para criar tudo: buffers, texturas, shaders, pipelines, encoders. Pense nele como a "conexão aberta" com a GPU.

A **`Queue`** é o canal para **enviar trabalho** à GPU. Comandos gravados em command encoders são submetidos via `queue.submit()`.

| Campo | Valor | Explicação |
|---|---|---|
| `label` | `None` | Nome opcional para debug. Aparece em mensagens de erro do wgpu. |
| `required_features` | `Features::empty()` | Features extras da GPU que seu código precisa (ex: `POLYGON_MODE_LINE` para wireframe). `empty()` significa que não precisamos de nada além do básico. Se pedirmos uma feature que a GPU não suporta, o `request_device` falha. |
| `experimental_features` | `ExperimentalFeatures::disabled()` | Features experimentais do wgpu. Desabilitadas por padrão. |
| `required_limits` | `Limits::default()` | Limites mínimos que a GPU deve suportar (tamanho máximo de textura, número de bind groups, etc.). `default()` funciona na maioria das GPUs desktop. Para mobile, use `Limits::downlevel_defaults()`. |
| `memory_hints` | `Default::default()` | Dicas de alocação de memória para o driver. |
| `trace` | `Trace::Off` | Se ativado, grava todas as chamadas da API em um arquivo. Útil para debug avançado mas gera muitos dados. |

---

## 7. Passo 5 — Configurar a Surface

```rust
let surface_caps = surface.get_capabilities(&adapter);
let surface_format = surface_caps
    .formats
    .iter()
    .find(|f| f.is_srgb())
    .copied()
    .unwrap_or(surface_caps.formats[0]);

let config = wgpu::SurfaceConfiguration {
    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
    format: surface_format,
    width: size.width,
    height: size.height,
    present_mode: surface_caps.present_modes[0],
    alpha_mode: surface_caps.alpha_modes[0],
    view_formats: vec![],
    desired_maximum_frame_latency: 2,
};
```

Primeiro, perguntamos à surface quais formatos, modos de apresentação e modos alpha ela suporta com aquele adapter. Depois, montamos a configuração.

**A escolha do formato sRGB** (`f.is_srgb()`) é importante: sRGB aplica correção gamma, fazendo as cores parecerem corretas ao olho humano. Sem sRGB, as cores intermediárias parecem mais escuras do que deveriam.

| Campo | Valor | Explicação |
|---|---|---|
| `usage` | `TextureUsages::RENDER_ATTACHMENT` | Como a textura da surface será usada. `RENDER_ATTACHMENT` significa que ela será o destino de um render pass (ou seja, vamos desenhar nela). |
| `format` | `surface_format` (sRGB) | O formato de pixel das texturas. Ex: `Bgra8UnormSrgb` = 4 canais (Blue, Green, Red, Alpha), 8 bits cada, normalizado, com correção sRGB. |
| `width` / `height` | Tamanho da janela | Dimensões em pixels da textura. Deve coincidir com o tamanho da janela. |
| `present_mode` | Primeiro suportado | Como os frames são sincronizados com o monitor. Exemplos: `Fifo` (VSync — espera o monitor), `Mailbox` (triple buffering — mais fluido), `Immediate` (sem sincronização — pode ter tearing). |
| `alpha_mode` | Primeiro suportado | Como o canal alpha da surface é tratado. `Opaque` ignora alpha (janela sólida). `PreMultiplied` / `PostMultiplied` são para janelas translúcidas. |
| `view_formats` | `vec![]` | Formatos alternativos para criar views da textura. Vazio significa "use só o formato padrão". |
| `desired_maximum_frame_latency` | `2` | Quantos frames podem estar enfileirados para apresentação. 2 é o padrão e dá um bom equilíbrio entre latência e suavidade. |

---

## 8. Passo 6 — Carregar o Shader

```rust
let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
```

`include_wgsl!` é uma macro que:
1. Lê o conteúdo do arquivo `shader.wgsl` **em tempo de compilação**
2. Embute o código WGSL como string no binário final
3. Cria um `ShaderModuleDescriptor` com esse código

O resultado é um `ShaderModule` — o programa compilado da GPU. Contém tanto o vertex shader quanto o fragment shader (ambos estão no mesmo arquivo `.wgsl`).

---

## 9. Passo 7 — Criar o Pipeline Layout

```rust
let render_pipeline_layout =
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[],
        immediate_size: 0,
    });
```

O `PipelineLayout` define quais **recursos externos** o shader pode acessar (texturas, buffers uniformes, etc.).

| Campo | Valor | Explicação |
|---|---|---|
| `label` | `"Render Pipeline Layout"` | Nome para debug. |
| `bind_group_layouts` | `&[]` | Array de layouts de bind groups. Bind groups são como "pacotes de dados" que o shader pode ler (texturas, matrices de transformação, etc.). `&[]` = o shader não precisa de nenhum dado externo — ele gera tudo sozinho. |
| `immediate_size` | `0` | Tamanho dos push constants (dados pequenos enviados diretamente ao shader sem buffer). 0 = não usamos push constants. |

---

## 10. Passo 8 — Criar o Render Pipeline

O `RenderPipeline` é o **coração da renderização**. Ele junta:
- Os shaders (vertex + fragment)
- Como os vértices são interpretados (triângulos? linhas?)
- Estado de blending, culling, multisampling

```rust
let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("Render Pipeline"),
    layout: Some(&render_pipeline_layout),
    vertex: wgpu::VertexState { ... },
    fragment: Some(wgpu::FragmentState { ... }),
    primitive: wgpu::PrimitiveState { ... },
    depth_stencil: None,
    multisample: wgpu::MultisampleState { ... },
    multiview_mask: None,
    cache: None,
});
```

| Campo | Valor | Explicação |
|---|---|---|
| `label` | `"Render Pipeline"` | Nome para debug. |
| `layout` | `Some(&render_pipeline_layout)` | O layout que define os recursos acessíveis pelo shader. |
| `depth_stencil` | `None` | Sem teste de profundidade. Necessário só quando desenhamos objetos 3D que podem se sobrepor. |
| `multiview_mask` | `None` | Renderização multi-view (VR). Não usado aqui. |
| `cache` | `None` | Cache de pipeline compilado. Pode acelerar carregamentos futuros. |

### 10.1 VertexState — Configuração do Vertex Shader

```rust
vertex: wgpu::VertexState {
    module: &shader,
    entry_point: Some("vs_main"),
    buffers: &[],
    compilation_options: wgpu::PipelineCompilationOptions::default(),
},
```

| Campo | Valor | Explicação |
|---|---|---|
| `module` | `&shader` | Referência ao `ShaderModule` carregado. Contém o código WGSL. |
| `entry_point` | `Some("vs_main")` | Nome da função WGSL que é o vertex shader. Deve corresponder exatamente ao nome da função marcada com `@vertex` no `.wgsl`. Se for `None`, wgpu usa a única função `@vertex` do módulo (e dá erro se houver mais de uma). |
| `buffers` | `&[]` | Descreve o layout dos vertex buffers (dados de vértice na memória da GPU). `&[]` = sem vertex buffers. Os vértices são gerados pelo shader usando `vertex_index`. Em aplicações reais, aqui descrevemos a estrutura dos vértices (posição, cor, UV, normais, etc.). |
| `compilation_options` | `default()` | Opções de compilação do shader (constantes, otimizações). O padrão não define nenhuma. |

### 10.2 FragmentState — Configuração do Fragment Shader

```rust
fragment: Some(wgpu::FragmentState {
    module: &shader,
    entry_point: Some("fs_main"),
    targets: &[Some(wgpu::ColorTargetState {
        format: config.format,
        blend: Some(wgpu::BlendState::REPLACE),
        write_mask: wgpu::ColorWrites::ALL,
    })],
    compilation_options: wgpu::PipelineCompilationOptions::default(),
}),
```

| Campo | Valor | Explicação |
|---|---|---|
| `module` | `&shader` | Mesmo ShaderModule (vertex e fragment estão no mesmo arquivo). |
| `entry_point` | `Some("fs_main")` | Nome da função `@fragment` no WGSL. |
| `targets` | `&[Some(ColorTargetState { ... })]` | Para onde o fragment shader escreve suas cores. Cada elemento corresponde a um `@location(N)` no retorno do fragment shader. Temos 1 target porque nosso fragment shader retorna apenas `@location(0)`. |
| `compilation_options` | `default()` | Opções de compilação do shader. |

#### ColorTargetState — Como o pixel é escrito na textura

| Campo | Valor | Explicação |
|---|---|---|
| `format` | `config.format` | O formato de pixel do target. **Deve corresponder** ao formato da surface, senão a GPU não sabe como converter as cores. |
| `blend` | `Some(BlendState::REPLACE)` | Como misturar a cor nova com a cor existente. `REPLACE` simplesmente sobrescreve — a cor do fragment shader substitui o que havia antes. Outras opções: `ALPHA_BLENDING` (mistura com transparência), `PREMULTIPLIED_ALPHA_BLENDING`. |
| `write_mask` | `ColorWrites::ALL` | Quais canais de cor podem ser escritos. `ALL` = Red, Green, Blue e Alpha. Poderia ser `ColorWrites::RED` para escrever só no canal vermelho, por exemplo. |

### 10.3 PrimitiveState — Como os vértices viram geometria

```rust
primitive: wgpu::PrimitiveState {
    topology: wgpu::PrimitiveTopology::TriangleList,
    strip_index_format: None,
    front_face: wgpu::FrontFace::Ccw,
    cull_mode: Some(wgpu::Face::Back),
    polygon_mode: wgpu::PolygonMode::Fill,
    unclipped_depth: false,
    conservative: false,
},
```

| Campo | Valor | Explicação |
|---|---|---|
| `topology` | `TriangleList` | Como agrupar os vértices. `TriangleList` = cada grupo de 3 vértices forma um triângulo independente. Com 6 vértices, teríamos 2 triângulos. Alternativas: `TriangleStrip` (vértices compartilhados), `LineList`, `PointList`. |
| `strip_index_format` | `None` | Formato do index buffer para strips. `None` porque usamos `TriangleList`, não `TriangleStrip`. |
| `front_face` | `FrontFace::Ccw` | Define qual é a "frente" de um triângulo. `Ccw` = Counter-clockwise (anti-horário). Se os 3 vértices estão dispostos em sentido anti-horário visto da câmera, a face está virada para frente. |
| `cull_mode` | `Some(Face::Back)` | **Back-face culling**: triângulos virados de costas para a câmera são descartados (não desenhados). Isso é uma otimização — em objetos sólidos, nunca vemos a face traseira. `None` desabilitaria o culling (ambos os lados visíveis). |
| `polygon_mode` | `PolygonMode::Fill` | Como preencher o triângulo. `Fill` = sólido. `Line` = só as arestas (wireframe, requer feature `NON_FILL_POLYGON_MODE`). `Point` = só os vértices. |
| `unclipped_depth` | `false` | Se `true`, não clipa fragmentos com Z fora de [0, 1]. Requer feature `DEPTH_CLIP_CONTROL`. |
| `conservative` | `false` | Rasterização conservadora: gera fragmentos para todos os pixels que o triângulo toca, mesmo parcialmente. Requer feature `CONSERVATIVE_RASTERIZATION`. |

### 10.4 MultisampleState — Anti-aliasing

```rust
multisample: wgpu::MultisampleState {
    count: 1,
    mask: !0,
    alpha_to_coverage_enabled: false,
},
```

| Campo | Valor | Explicação |
|---|---|---|
| `count` | `1` | Número de amostras por pixel. `1` = sem multisampling (sem anti-aliasing por MSAA). `4` seria MSAA 4x (bordas mais suaves, mas mais caro). |
| `mask` | `!0` (= todos os bits 1) | Máscara de quais amostras participam. `!0` = todas. É um bitmask — bit N controla a amostra N. |
| `alpha_to_coverage_enabled` | `false` | Se `true`, usa o valor alpha do fragment para determinar quais amostras cobrir. Usado para transparência com MSAA. |

---

## 11. Passo 9 — Renderizar um Frame

A função `render()` é chamada a cada frame (quando a janela pede um redesenho).

### 11.1 Obter a textura da surface

```rust
let output = self.surface.get_current_texture()?;
```

A surface tem internamente um **swapchain** — um conjunto de texturas que se revezam. `get_current_texture()` pega a próxima textura disponível. Enquanto uma está sendo exibida no monitor, renderizamos na outra.

### 11.2 Criar uma view da textura

```rust
let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
```

Uma `TextureView` é como um "ponteiro" para a textura que descreve como acessá-la. O padrão usa toda a textura, formato original, e todas as mip levels.

### 11.3 Criar o Command Encoder

```rust
let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
    label: Some("Render Encoder"),
});
```

O `CommandEncoder` **grava comandos** que serão enviados à GPU. Ele não executa nada imediatamente — apenas monta uma lista de instruções.

### 11.4 Iniciar o Render Pass

```rust
let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
    label: Some("Render Pass"),
    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        view: &view,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color {
                r: 0.1,
                g: 0.2,
                b: 0.3,
                a: 1.0,
            }),
            store: wgpu::StoreOp::Store,
        },
        depth_slice: None,
    })],
    depth_stencil_attachment: None,
    occlusion_query_set: None,
    timestamp_writes: None,
    multiview_mask: None,
});
```

Um `RenderPass` é um grupo de comandos de desenho que escrevem nos mesmos attachments.

| Campo | Valor | Explicação |
|---|---|---|
| `color_attachments` | Array com 1 attachment | As texturas onde as cores serão escritas. Cada attachment corresponde a um `@location(N)` do fragment shader. |
| `depth_stencil_attachment` | `None` | Textura de profundidade/stencil. `None` porque não temos teste de profundidade. |
| `occlusion_query_set` | `None` | Queries para saber quantos pixels passaram no teste de profundidade. Não usado. |
| `timestamp_writes` | `None` | Timestamps de GPU para medir performance. Não usado. |

#### RenderPassColorAttachment — O target de cor

| Campo | Valor | Explicação |
|---|---|---|
| `view` | `&view` | A texture view da surface onde vamos desenhar. |
| `resolve_target` | `None` | Textura para resolver MSAA. Quando `multisample.count > 1`, a textura multisampled precisa ser "resolvida" (media das amostras) para uma textura normal. `None` porque não usamos MSAA. |
| `ops` | `Operations { load, store }` | O que fazer com a textura no início e no fim do render pass. |
| `depth_slice` | `None` | Qual fatia de uma textura 3D usar. `None` para texturas 2D normais. |

#### Operations — Limpeza e gravação

| Campo | Valor | Explicação |
|---|---|---|
| `load` | `LoadOp::Clear(Color { r: 0.1, g: 0.2, b: 0.3, a: 1.0 })` | **No início do render pass**: limpa toda a textura com a cor informada (azul escuro). Alternativa: `LoadOp::Load` manteria o conteúdo anterior. |
| `store` | `StoreOp::Store` | **No fim do render pass**: guarda o resultado. `StoreOp::Discard` jogaria fora (útil para MSAA onde só o resolve_target importa). |

### 11.5 Desenhar o triângulo

```rust
render_pass.set_pipeline(&self.render_pipeline);
render_pass.draw(0..3, 0..1);
```

**`set_pipeline`** — Ativa o pipeline de renderização. A partir daqui, todos os draws usam os shaders e configurações deste pipeline.

**`draw(0..3, 0..1)`** — O comando que efetivamente desenha!

| Parâmetro | Valor | Explicação |
|---|---|---|
| `vertices` | `0..3` | Range de índices de vértices. A GPU executará o vertex shader 3 vezes, com `vertex_index` = 0, 1 e 2. Esses são os 3 vértices do triângulo. |
| `instances` | `0..1` | Range de instâncias. `0..1` = 1 instância (um único triângulo). Com instancing, poderíamos desenhar milhares de triângulos com uma única chamada `draw`, cada um com `instance_index` diferente. |

### 11.6 Submeter e Apresentar

```rust
self.queue.submit(std::iter::once(encoder.finish()));
output.present();
```

**`encoder.finish()`** — Finaliza a gravação e produz um `CommandBuffer` imutável.

**`queue.submit()`** — Envia o command buffer para a GPU executar. A GPU processa os comandos assincronamente.

**`output.present()`** — Diz à surface para apresentar esta textura no monitor quando o próximo VSync acontecer (dependendo do `present_mode`).

---

## 12. Passo 10 — O Event Loop e o App

### A struct App

```rust
pub struct App {
    state: Option<State>,
}
```

O `State` é `Option` porque ele só pode ser criado **depois** que a janela existe. No winit, a janela é criada no evento `resumed`, não no início do programa.

### O evento `resumed`

```rust
fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    let window_attributes = Window::default_attributes();
    let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
    self.state = Some(pollster::block_on(State::new(window)).unwrap());
}
```

1. Cria atributos padrão da janela (tamanho, título, etc.)
2. Cria a janela e a envolve em `Arc` (referência compartilhada thread-safe)
3. Cria o `State` com toda a inicialização de GPU (`pollster::block_on` resolve o `async` de forma síncrona)

### O event loop principal

```rust
pub fn run() -> anyhow::Result<()> {
    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = App::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}
```

`EventLoop` é o motor que roda infinitamente:
1. Recebe eventos do sistema operacional (teclado, mouse, resize, fechar)
2. Despacha para os handlers do `App`
3. Quando o handler de `RedrawRequested` é chamado, executamos `update()` + `render()`

---

## 13. Resumo do Fluxo Completo

```
main()
  └─ run()
       └─ EventLoop::run_app(app)
            │
            ├─ [resumed] → Cria janela → Cria State
            │                              ├─ Instance (ponto de entrada)
            │                              ├─ Surface (janela ↔ GPU)
            │                              ├─ Adapter (GPU física)
            │                              ├─ Device + Queue (GPU lógica)
            │                              ├─ SurfaceConfiguration (formato, tamanho)
            │                              ├─ ShaderModule (shader.wgsl compilado)
            │                              ├─ PipelineLayout (sem bind groups)
            │                              └─ RenderPipeline (shaders + estado)
            │
            ├─ [Resized] → resize() → reconfigura surface
            │
            └─ [RedrawRequested] → update() + render()
                                     ├─ get_current_texture()
                                     ├─ create_view()
                                     ├─ create_command_encoder()
                                     ├─ begin_render_pass() → Clear azul escuro
                                     ├─ set_pipeline()
                                     ├─ draw(0..3, 0..1) → 3 vértices, 1 instância
                                     │    │
                                     │    ├─ GPU: vs_main(0) → (0.5, -0.5)
                                     │    ├─ GPU: vs_main(1) → (0.0, 0.5)
                                     │    ├─ GPU: vs_main(2) → (-0.5, -0.5)
                                     │    │
                                     │    └─ GPU: fs_main() → cor (0.3, 0.2, 0.1, 1.0)
                                     │         para cada pixel dentro do triângulo
                                     │
                                     ├─ queue.submit() → envia à GPU
                                     └─ present() → mostra na tela
```

**Resultado**: Um triângulo marrom sobre fundo azul escuro, centralizado na janela. 🎉

---

## 14. O Fluxo de Dados: triangulo.rs ↔ shader.wgsl

> Esta é a seção que responde: **"O que é enviado? Como o Rust fala com o WGSL? O que volta?"**

A confusão é comum porque a comunicação **não é como chamar uma função normal**. Você não passa argumentos diretamente ao shader. A GPU tem um sistema de *estágios* com canais específicos de entrada e saída. Vamos ver cada canal.

---

### 14.1 O mapa geral do fluxo

```
┌────────────────────────────────────────────────────────────────────────┐
│  CPU (triangulo.rs)                                                    │
│                                                                        │
│  render_pass.draw(0..3, 0..1)  ← você chama isso                      │
└─────────────────────────┬──────────────────────────────────────────────┘
                          │  A GPU recebe a ordem e começa o trabalho
                          ▼
┌─────────────────────────────────────────────────────────────────────── ┐
│  ESTÁGIO 1 — Vertex Shader (shader.wgsl: vs_main)                     │
│                                                                        │
│  Entrada automática da GPU:                                            │
│    @builtin(vertex_index)  → 0, depois 1, depois 2  (3x no total)     │
│                                                                        │
│  Saída que você produz:                                                │
│    VertexOutput {                                                      │
│      clip_position: vec4(x, y, 0.0, 1.0)  ← posição do vértice       │
│      vert_pos: vec3(x, y, 0.0)            ← dado personalizado        │
│      color: vec3(0.3, 0.2, 0.1)           ← dado personalizado        │
│    }                                                                   │
└─────────────────────────┬──────────────────────────────────────────────┘
                          │  A GPU usa clip_position para montar o triângulo
                          │  e INTERPOLA os outros campos para cada pixel
                          ▼
┌────────────────────────────────────────────────────────────────────────┐
│  ESTÁGIO 2 — Rasterização (feita automaticamente pela GPU)            │
│                                                                        │
│  A GPU descobre quais pixels da tela estão dentro do triângulo.       │
│  Para cada pixel, ela calcula uma interpolação dos 3 vértices.        │
│  (ex: um pixel no centro recebe a média das 3 cores dos vértices)     │
└─────────────────────────┬──────────────────────────────────────────────┘
                          │  Um VertexOutput interpolado por pixel
                          ▼
┌────────────────────────────────────────────────────────────────────────┐
│  ESTÁGIO 3 — Fragment Shader (shader.wgsl: fs_main)                   │
│                                                                        │
│  Entrada: VertexOutput interpolado (um por pixel)                     │
│    in.color → vec3(0.3, 0.2, 0.1)                                     │
│    in.vert_pos → coordenada interpolada daquele pixel                 │
│                                                                        │
│  Saída: cor final do pixel                                             │
│    @location(0) vec4(0.3, 0.2, 0.1, 1.0)                             │
└─────────────────────────┬──────────────────────────────────────────────┘
                          │  Escrito na textura do frame
                          ▼
┌────────────────────────────────────────────────────────────────────────┐
│  TEXTURA (color attachment)                                            │
│  → queue.submit()  → output.present()  → aparece na tela             │
└────────────────────────────────────────────────────────────────────────┘
```

---

### 14.2 O que o Rust envia para o shader?

A resposta surpreendente é: **quase nada neste exemplo específico**.

O único "dado" que a CPU envia é a **ordem de desenho** via `draw(0..3, 0..1)`. Isso diz à GPU:

> "Execute o vertex shader **3 vezes**, para os índices 0, 1 e 2."

Nenhum dado de vértice (posições, cores) sai da memória da CPU para a GPU. Os vértices são **calculados diretamente no shader** usando o `vertex_index`. Isso é chamado de **geometry sem buffer** (ou *procedural geometry*).

```rust
// triangulo.rs — só isso é enviado:
render_pass.draw(0..3, 0..1);
//               ^^^^  ^^^
//               |     └─ 1 instância (0..1 = só o índice 0)
//               └─ 3 vértices (índices 0, 1 e 2)
```

Em aplicações reais, você normalmente criaria um `vertex buffer` na GPU com as posições e passaria os dados pelo campo `buffers: &[...]` do `VertexState`. Mas aqui o shader resolve tudo sozinho.

---

### 14.3 Canal 1: CPU → Vertex Shader via `@builtin(vertex_index)`

Este é o principal canal de entrada do vertex shader neste exemplo.

```wgsl
@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,  // ← entrada
) -> VertexOutput {
```

**`@builtin(vertex_index)`** não é um dado que você passa manualmente. A GPU **gera automaticamente** esse número para você. Ele começa em 0 e vai até onde o `draw` mandou.

| Execução do vs_main | `in_vertex_index` | Quem forneceu? |
|---|---|---|
| 1ª chamada | `0` | GPU automaticamente |
| 2ª chamada | `1` | GPU automaticamente |
| 3ª chamada | `2` | GPU automaticamente |

Pense como se a GPU fizesse internamente:
```
for index in 0..3 {
    vs_main(vertex_index = index)
}
```

Outros builtins que a GPU pode fornecer automaticamente:
- `@builtin(instance_index)` — qual instância está sendo desenhada (do segundo parâmetro do `draw`)
- `@builtin(front_facing)` — booleano, o fragmento é da face da frente?
- `@builtin(frag_coord)` — coordenada do pixel em tela (disponível no fragment shader)

---

### 14.4 Canal 2: Vertex Shader → Fragment Shader via `VertexOutput`

O vertex shader precisa comunicar dados para o fragment shader. Isso é feito pela **struct de saída**.

```wgsl
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,  // obrigatório
    @location(0) vert_pos: vec3<f32>,             // opcional, slot 0
    @location(1) color: vec3<f32>,                // opcional, slot 1
}
```

Cada campo tem um papel diferente:

#### `@builtin(position) clip_position`

Este campo é **obrigatório** em todo vertex shader. Ele diz à GPU: *"este vértice fica nesta posição na tela"*.

O valor deve estar em **clip space**:
- `X` e `Y` vão de `-1.0` (esquerda/baixo) a `+1.0` (direita/cima)
- `Z` vai de `0.0` a `1.0` (profundidade)
- `W` deve ser `1.0` em casos simples (sem perspectiva)

A GPU usa `clip_position` para:
1. Saber onde plotar o vértice na tela
2. Descobrir quais pixels estão dentro do triângulo (rasterização)
3. **Não** repassa esse valor ao fragment shader como `clip_position` — em vez disso, no fragment shader `@builtin(position)` contém a coordenada do *pixel* em tela, não o clip space.

#### `@location(N) campo`

Estes campos são **personalizados** — você decide o que guardar. O `@location(0)` no vertex shader corresponde ao `@location(0)` no fragment shader. É como um "cabo de comunicação numerado".

```
vs_main retorna:
  @location(0) vert_pos = vec3(0.5, -0.5, 0.0)   ← para o vértice 0

                ↓ GPU interpola entre os 3 vértices ↓

fs_main recebe:
  in.vert_pos = vec3(algum valor interpolado)      ← para cada pixel
```

---

### 14.5 A interpolação — o que a GPU faz automaticamente entre os estágios

Este é o ponto mais importante e menos óbvio: **os valores de `@location(N)` chegam ao fragment shader interpolados**, não exatamente como o vertex shader os produziu.

Imagine que você tem 3 vértices com cores diferentes:
- Vértice 0: `color = vec3(1.0, 0.0, 0.0)` → vermelho
- Vértice 1: `color = vec3(0.0, 1.0, 0.0)` → verde
- Vértice 2: `color = vec3(0.0, 0.0, 1.0)` → azul

O fragment shader de um pixel no centro do triângulo receberia algo como `vec3(0.33, 0.33, 0.33)` — a mistura igual das 3 cores. Um pixel mais perto do vértice 0 receberia mais vermelho. Isso cria um gradiente suave automaticamente, sem nenhum código seu.

No nosso exemplo, todos os 3 vértices têm a mesma cor `vec3(0.3, 0.2, 0.1)`, então o triângulo aparece com cor uniforme.

```
                  Vértice 1
                  color=(0.3, 0.2, 0.1)
                       /\
                      /  \
       pixel aqui → /  ✦  \   → recebe color=(0.3, 0.2, 0.1)
                   /        \  (a mesma, pois todos são iguais)
                  /____________\
   Vértice 2                    Vértice 0
   color=(0.3, 0.2, 0.1)       color=(0.3, 0.2, 0.1)
```

---

### 14.6 Canal 3: Fragment Shader → Textura via `@location(0)` no retorno

O fragment shader precisa devolver a cor final do pixel. Ele faz isso com um retorno anotado:

```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
//                              ^^^^^^^^^^^^
//                              Escreve no color attachment 0
    return vec4<f32>(in.color, 1.0);
}
```

O `@location(0)` no retorno do fragment shader corresponde ao **primeiro `ColorTargetState`** configurado no Rust:

```rust
// triangulo.rs — cada elemento corresponde a um @location do fragment shader
targets: &[Some(wgpu::ColorTargetState {
//  ^── índice 0 = @location(0) no fragment shader
    format: config.format,
    blend: Some(wgpu::BlendState::REPLACE),
    write_mask: wgpu::ColorWrites::ALL,
})],
```

Se você tivesse dois targets (`targets: &[Some(...), Some(...)]`), precisaria que o fragment shader retornasse dois valores, um em `@location(0)` e outro em `@location(1)`.

---

### 14.7 Resumo: todos os canais lado a lado

```
┌─────────────────────────────────────────────────────────────────────┐
│  triangulo.rs (CPU)                   shader.wgsl (GPU)            │
│                                                                     │
│  draw(0..3, 0..1)        ──────►  vs_main é chamado 3 vezes       │
│                                                                     │
│  (não envia dados,       ──────►  @builtin(vertex_index): 0, 1, 2 │
│   a GPU gera o índice)            (gerado automaticamente)         │
│                                                                     │
│  buffers: &[]            ──────►  (nenhum vertex buffer)           │
│  (não envia vértices)                                               │
│                                                                     │
│                                   ↓ vs_main produz VertexOutput ↓  │
│                                                                     │
│                                   ↓ GPU rasteriza e interpola ↓    │
│                                                                     │
│                                   fs_main recebe VertexOutput      │
│                                   (um por pixel do triângulo)      │
│                                                                     │
│  ColorTargetState[0]     ◄──────  @location(0) vec4<f32>          │
│  (textura da surface)              (cor do pixel retornada)        │
│                                                                     │
│  queue.submit()          → GPU executa tudo e escreve na textura   │
│  output.present()        → textura aparece na tela                 │
└─────────────────────────────────────────────────────────────────────┘
```

### 14.8 Por que não há `return` visível do shader de volta ao Rust?

O fragment shader não "retorna" para o Rust — ele escreve diretamente em uma **textura na memória da GPU**. Depois que `queue.submit()` termina e `output.present()` é chamado, o driver de vídeo copia essa textura para o monitor. O Rust nunca "lê" o resultado dos shaders neste exemplo.

Se você quisesse ler o resultado (ex: para capturar um screenshot), precisaria:
1. Criar um buffer de leitura (`BufferUsages::COPY_DST | COPY_SRC`)
2. Copiar a textura para o buffer com `encoder.copy_texture_to_buffer()`
3. Mapear o buffer de volta para a CPU com `buffer.map_async()`

Mas para simplesmente exibir na tela, o Rust só diz "vai!" e a GPU faz o resto.

---
