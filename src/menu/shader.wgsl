// Shader 2D simples: recebe posição em pixels e cor, converte para clip space.

struct Uniforms {
    // Tamanho da janela em pixels — usado para converter pixels → clip space.
    screen_size: vec2<f32>,
    _pad: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    // Posição em pixels. Origem (0,0) = canto superior esquerdo da janela.
    @location(0) position: vec2<f32>,
    // Cor RGBA, valores de 0.0 a 1.0.
    @location(1) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // Converte pixel (x, y) → clip space (-1..+1):
    //   pixel (0, 0)   → clip (-1, +1)  — canto superior esquerdo
    //   pixel (w, h)   → clip (+1, -1)  — canto inferior direito
    let cx = (in.position.x / uniforms.screen_size.x) * 2.0 - 1.0;
    let cy = 1.0 - (in.position.y / uniforms.screen_size.y) * 2.0;
    out.clip_position = vec4<f32>(cx, cy, 0.0, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
