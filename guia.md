🖥️ Guia de Implementação: Ícones PNG e Texto com WGPU

Este documento serve como o manual passo a passo para a atualização de ponta a ponta do seu motor gráfico 2D. Abaixo, você encontrará o código completo para cada arquivo do projeto.

🛠️ Passo a Passo das Modificações

1. Modificar o Cargo.toml

Adicionamos as crates necessárias para processamento de imagens e cálculo de layouts de fontes vetoriais:

image: Para decodificar arquivos .png em matrizes de bytes brutos.

ab_glyph: Já presente no seu Cargo.lock, será usada diretamente para rasterizar caracteres vetoriais em tempo de execução para imagens antes de serem enviadas para a GPU.

2. Modificar o shader.wgsl

Modificamos o pipeline para aceitar uma textura secundária de difusão e um amostrador na etapa de fragmentação (@group(1)). Unificamos o desenho de retângulos coloridos e retângulos texturizados (quando for cor sólida, amostramos um pixel branco, resultando em cor_do_vertice * 1.0).

3. Modificar o renderer.rs

Esta é a maior mudança. Introduzimos:

Vértices UV: Cada vértice agora possui tex_coords: [f32; 2].

Estrutura Texture: Abstrai a alocação de texturas e geração de BindGroup na GPU.

Lotes (Batch): Agrupamento automático de retângulos que compartilham da mesma textura.

Fallback do Sistema: Se um ícone PNG ou arquivo de fonte .ttf não estiver presente no diretório local, o motor gera um fallback colorido em tempo de execução e busca fontes instaladas no sistema operacional (Windows, macOS ou Linux) automaticamente para que o programa nunca trave.

4. Modificar o demo.rs

Integração de todo o fluxo. No momento em que o aplicativo é retomado (resumed), ele carrega a fonte do sistema ou local, gera as texturas estáticas para os textos de menu (que passam a ser desenhados tão rápido quanto simples imagens) e desenha os ícones PNG dentro dos cartões do carrossel.

📂 Organização de Arquivos e Assets

Para adicionar seus próprios ícones PNG personalizados e fontes personalizadas:

Crie uma pasta chamada assets na raiz do seu projeto (ao lado de src e Cargo.toml).

Coloque as suas imagens PNG e fontes dentro dela:

assets/games.png (Ícone para a categoria de Jogos)

assets/apps.png (Ícone para a categoria de Aplicativos)

assets/media.png (Ícone para a categoria de Mídia)

assets/font.ttf (Sua fonte vetorial favorita. Caso não coloque nada, o motor usará automaticamente a Arial ou DejaVuSans do seu sistema operacional!)

Siga para as abas de edição de arquivo ao lado ou copie os códigos abaixo nos respectivos arquivos do seu editor.