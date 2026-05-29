// Vertex shader: gera a posicao de cada vertice do triangulo.

// Estrutura de saida do vertex shader.
struct VertexOutput {
    // Posicao em clip space (coordenadas normalizadas da GPU).
    @builtin(position) clip_position: vec4<f32>,
    // Exemplo de dado que pode ser passado para o fragment shader.
    @location(0) vert_pos: vec3<f32>,
    @location(1) color: vec3<f32>,
}

@vertex
fn vs_main(
    // O indice do vertice vem do draw(0..3, 0..1).
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    // Calcula X e Y para cada indice (0, 1, 2) e monta um triangulo.
    let x = f32(1 - i32(in_vertex_index)) * 0.5;
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
    // Clip space: x, y em [-1, 1], z=0, w=1.
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.vert_pos = out.clip_position.xyz;
    out.color = vec3<f32>(0.3, 0.2, 0.1);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Cor fixa do triangulo (RGBA).
    return vec4<f32>(in.color, 1.0);
},as p´q