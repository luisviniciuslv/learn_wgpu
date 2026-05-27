# Arquitetura do projeto

Este documento descreve a arquitetura do codigo em alto nivel, focando na separacao de responsabilidades e no fluxo de execucao. O projeto e um exemplo basico de renderizacao com wgpu + winit para ambiente desktop (sem suporte web).

## Visao geral

- O binario inicia em `main`, que chama `run`.
- `run` cria o event loop do winit e executa o `App`.
- `App` implementa `ApplicationHandler` e coordena eventos de janela e o ciclo de renderizacao.
- `State` encapsula todo o estado do GPU e os recursos necessarios para desenhar um frame.

## Componentes principais

### State (camada de GPU)

Responsavel por manter e operar os objetos do wgpu.

- `surface`: alvo de apresentacao ligado a janela.
- `device` e `queue`: acesso ao GPU e envio de comandos.
- `config`: configuracao do swap chain (formato, tamanho, modo de apresentacao).
- `is_surface_configured`: protege o render antes de haver tamanho valido.
- `clear_color`: cor atual usada para limpar o frame (atualizada pelo mouse).
- `window`: handle compartilhado da janela.

Funcoes relevantes:

- `State::new`: cria `instance`, `surface`, escolhe `adapter`, pede `device/queue` e monta `SurfaceConfiguration`.
- `State::resize`: aplica novo tamanho e reconfigura a surface.
- `State::handle_mouse_moved`: normaliza a posicao do mouse e atualiza `clear_color`.
- `State::update`: ponto para logica futura (simulacao, animacoes).
- `State::render`: obtencao do frame, criacao de encoder, limpeza com `clear_color` e present.

### App (camada de aplicacao)

Responsavel por orquestrar o ciclo de vida da aplicacao e delegar para `State`.

- `App::new`: inicializa o app sem `State`.
- `resumed`: cria a janela e inicializa o `State`.
- `window_event`: reage aos eventos de janela (resize, teclado, redraw, close, movimento do mouse).

### run / main (boot)

- `run` cria o event loop e executa o `App`.
- `main` apenas chama `run` e propaga erros.

## Fluxo de execucao

1. `main` chama `run`.
2. `run` cria o `EventLoop` e inicia o `App`.
3. `App::resumed` cria a janela e inicializa o `State`.
4. O `EventLoop` entrega eventos para `App::window_event`.
5. Em `CursorMoved`, o `App` chama `state.handle_mouse_moved()` e pede redraw.
6. Em `RedrawRequested`, o `App` chama `state.update()` e `state.render()`.

## Observacoes de arquitetura

- A separacao `App` (eventos) vs `State` (GPU) facilita testes e evolucao.
- `State` e a unica parte que conhece detalhes do wgpu.
- O fluxo atual desenha apenas uma cor de fundo, mas ja possui o pipeline basico para evoluir.
