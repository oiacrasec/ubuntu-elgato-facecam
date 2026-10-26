# Algoritmos de efeitos e processamento

O fluxo implementado oferece cor, espelho, LUT, denoise convencional, correção
de olhar e desfoque ou substituição de fundo por segmentação. Os demais tópicos desta página são contratos de estudo
para extensões; não acrescentam efeitos ao binário atual.

## Pipeline existente

A segmentação reduz RGB para 256×256 com interpolação bilinear, separa canais em
NCHW e normaliza por 255. A saída é alpha de pessoa. O fundo reduzido é desfocado
por duas passagens de média com janela deslizante. A composição calcula
`pessoa × alpha + fundo × (1 − alpha)` e interpola máscara e fundo para o tamanho
de saída. Testes exercitam alpha 0, 0,5 e 1, máscara inválida e frame truncado.

Na revisão de desempenho, coordenadas de interpolação são preparadas por worker.
A passagem horizontal interpola uma vez cada linha do mapa reduzido para a
largura de saída; a vertical reutiliza esse buffer. A conversão não negativa
para u8 usa adição de 0,5 e truncamento; a comparação com o caminho escalar
anterior tolera até um nível por canal. Alpha zero substitui integralmente o
fundo, e alpha um mantém o pixel original. Fundos estáticos são preparados
uma vez na resolução de saída e não passam pelo blur em cada frame.

O raio atual é medido na imagem reduzida. Ele não representa automaticamente o
mesmo número de pixels no vídeo completo. RGB, máscara e fundo compartilham a
mesma transformação geométrica; trocar orientação ou crop em apenas um deles
produziria um recorte deslocado.

Os efeitos comuns ocorrem após a etapa opcional de IA, antes do split entre as
saídas. Espelho é `hflip`, cor usa `eq`, denoise usa `hqdn3d` e a LUT usa `lut3d`
com interpolação tetraédrica. Não há equivalência anunciada com algoritmos
proprietários de nitidez, HDR ou redução de ruído da câmera.

## Detecção e correção de olhar

O detector AR Ada recebe BGR float32 NCHW 1×3×208×368, dividido por 255. O resize
bilinear preserva proporção e completa o destino com preto; a mesma geometria
remove as bordas na decodificação. As saídas têm 299 células (13×23), com quatro
planos de distâncias e um de confiança. O centro de cada célula é
`(16×coluna+0,5, 16×linha+0,5)`; distâncias multiplicadas por 35 produzem os
cantos esquerdo/superior por subtração e direito/inferior por adição. A origem do
contrato está em [Análise RTX](ANALISE_RTX.md#contrato-de-detecção-fechado-e-recorte-digital).
A implementação própria exige confiança mínima 0,7 para adquirir um rosto.
Um rosto já validado pelos landmarks pode continuar com confiança de pelo menos
0,5 e IoU de pelo menos 0,3 com sua caixa anterior. Sem associação, após perda ou
reinício, vale novamente o corte 0,7. A supressão usa IoU 0,3 e até 32 caixas;
formas/probabilidades inválidas e valores não finitos são recusados.

O rosto selecionado fornece um recorte quadrado com margem 1,5×, amostrado na GPU
para 80×80. Os landmarks usam cinza float32 em [0,255], com pesos RGB
0,299/0,587/0,114. O tensor de 252 coordenadas contém primeiro 126 valores X e
depois 126 valores Y, em unidades do recorte 80×80. Somente coordenadas, confiança
e head pose retornam à CPU. Os seis pontos de cada olho usam os índices fixados
do contrato 126; não há template 3D ou recurso proprietário incorporado.

A geometria própria usa uma transformação de similaridade 2D: centros dos olhos
em (64,32) e (192,32) no recorte RGB 256×64. Escala e rotação vêm da distância
entre os olhos; a transformação inversa leva o resultado ao mesmo local na imagem
original. O encoder recebe RGB NCHW dividido por 255. Embeddings e labels passam
diretamente ao decoder na GPU, com alvo de olhar (0,0). A saída finita pode ter
overshoot; a composição limita os canais a [0,1] antes de convertê-los em RGB.

A correção exige confiança média de pelo menos 0,45 e mínima de 0,30 nos pontos
usados, valores absolutos do tensor de pose até 0,45 e distância ocular suficiente.
Esse limiar não estabelece a unidade física da saída do modelo.
A visibilidade cresce por smoothstep entre razões de abertura/largura de 0,13
e 0,23. A confiança média modula o ganho entre 0,45 e 0,65; o máximo absoluto
da pose atenua a correção entre 0,20 e 0,45. Esses fatores chegam a zero antes
dos cortes de confiança/pose. Um olho fechado recebe máscara zero imediatamente,
sem reiniciar o acompanhamento do outro olho. Dados não finitos ou saída nativa
inválida geram erro; ausência de rosto, pose inadequada ou baixa confiança
preservam a imagem e descartam o acompanhamento.

A transformação e os raios da máscara têm filtragem temporal própria. A distância
entre medições, normalizada pelo afastamento dos olhos e pelo intervalo de frame,
ajusta a constante da transformação entre 100 e 8 ms: pequenas oscilações recebem
mais filtragem e movimentos rápidos têm resposta mais curta. Um salto de pelo
menos 35% da distância ocular reinicia a aquisição. Os raios usam constante de
80 ms; a visibilidade sobe em 80 ms e cai em 25 ms, exceto fechamento completo,
que zera imediatamente. Nenhuma imagem de frame anterior é misturada ao atual.

A composição usa elipses menores em torno dos olhos, com borda suave por
smoothstep e ganho máximo 0,85. Calcula a diferença entre a saída do decoder
limitada a [0,1] e o recorte de entrada; somente essa diferença é transformada
de volta e adicionada ao RGB original. Um decoder identidade deve produzir
imagem exatamente inalterada, evitando blur da dupla interpolação do recorte.
O RGB final é limitado a [0,255]. Fora das elipses, os pixels permanecem
inalterados. Não há zoom, reenquadramento ou ajuste
físico de PTZ. O alinhamento é voltado a rostos frontais e não reproduz o pipeline
3D proprietário do Windows.

Os grafos próprios usam [`GridSample` ONNX](https://onnx.ai/onnx/operators/onnx__GridSample.html),
bilinear, `align_corners=0` e borda replicada; o operador é suportado pelo
[provider CUDA 1.20.1](https://github.com/microsoft/onnxruntime/blob/v1.20.1/docs/OperatorKernels.md#cudaexecutionprovider).
Imagem, embeddings e composição ficam na GPU entre etapas. O modo somente fundo
não abre os modelos AR. Preview e câmera virtual recebem o mesmo RGB processado.

## Cor, curvas, histograma e scopes — proposto

Histograma, vectorscope e amostragem de cor devem ser derivados de uma revisão
identificada do preview, com custo limitado. Definir bins, clipping, contadores de
out-of-range, tratamento de alpha e não finitos. Hover deve consultar um pixel;
não refazer um histograma inteiro a cada movimento do ponteiro.

Uma análise em proxy precisa indicar resolução e perda de detalhes finos. Curvas
requerem pontos finitos, ordem de entrada, limites e proteção dos endpoints.
Converter graus de hue ou níveis para um formato inteiro demanda política de
arredondamento e round-trip. Ajustes iguais não garantem pixels iguais quando
as pipelines de desenvolvimento são diferentes.

Amostragem de cor da própria imagem é distinta de capturar pixels de outra janela.
No Linux, portais e compositor podem determinar permissões e espaço de cor. O
Facecam Hub não implementa um conta-gotas global nem leitura de perfil do monitor.
Antes de incluir um desses recursos, especificar transformação, fallback visível
e teste real na sessão gráfica.

## Máscaras e morfologia — proposto

Uma máscara manual futura deve guardar cobertura contínua, coordenadas da imagem
e modo de composição. Expandir, contrair e feather não são simples mudanças de
um threshold de IA. Definir se a operação altera a máscara original ou uma cópia
e preservar uma forma de cancelar antes de aplicar.

Para uma dilatação/erosão octogonal construída por alternância de quadrado e
cruz, a decomposição por raio pode reduzir passagens repetidas. Uma implementação
precisa provar o mesmo footprint, inclusive em bordas recortadas. Máximos/mínimos
separáveis, decomposição binária da distância Manhattan e transposição por blocos
são alternativas de estudo. NaNs, zero com sinal e ordem de comparação exigem
contrato explícito nos caminhos float que os aceitem.

Distância euclidiana pode ser calculada em duas passagens: distância vertical e
envelope inferior de parábolas por linha. Usar precisão suficiente nas interseções
para não criar erro em coordenadas grandes. Comparar com força bruta em máscaras
pequenas, pixels selecionados com distância zero, eixos longos e casos sem seeds.
Planos auxiliares podem dobrar o custo por pixel; registrar memória total e scratch.

## Seleção e retoque assistido — proposto

Seleção rápida baseada em grafo precisa preservar custos, terminais, capacidades,
regra de empate e rótulos do corte. Push/relabel pode reduzir trabalho em grafos
adequados, mas demanda retorno do excesso à origem e grafo residual válido para
uma segunda solução. Instâncias pequenas, não finitas ou degeneradas precisam
de comportamento definido. Testar cortes por enumeração em grafos pequenos.

Retoque por proximidade pode comparar um anel em torno da região faltante.
Tabelas de área somada limitadas ao bounding box e poda por limite inferior de
SSD evitam buscas repetidas. A poda só é válida com termos não negativos e
melhor score finito, preservando ordem e empate. Uma otimização da busca não
comprova qualidade de preenchimento ou reconhecimento semântico.

Pincéis, pressão de caneta, simetria, clone, healing e máscaras pintadas não estão
implementados. Uma ferramenta futura deve agrupar um gesto em uma operação,
respeitar seleção/locks, tornar cancelamento explícito e manter a visualização
fora da thread de processamento. Drivers de tablet nativos não justificam
introduzir `unsafe` no aplicativo sem rever o desenho do produto.

## Desfoques espacialmente variáveis — proposto

Iris e tilt-shift podem usar uma pilha de níveis gaussianos interpolados por um
campo de sigma. A geometria de foco, rotação, transição e feather pode ser
preparada uma vez. Não substituir esse algoritmo por amostragem radial e chamar
o resultado de equivalente.

O suporte do kernel determina quais bordas de uma janela podem ser removidas.
Somas acumuladas podem depender do ponto inicial; recortar a borda inicial pode
mudar cancelamento numérico mesmo quando uma borda final parece distante.
Comparar HDR, transparência, raios extremos, regiões pequenas, múltiplos focos e
cancelamento. Não reduzir níveis ou amostras sob o nome de ganho exato.

Um blur radial de rotação ou zoom pode preparar tabelas de transformações e
compartilhar taps entre canais. Só preparar contagens alcançáveis para limitar
memória. Preservar política de borda, alpha e limites de qualidade. Se tabelas
opcionais não couberem, um caminho de referência pode continuar calculando,
sem anunciar memória total limitada quando a fonte inteira ainda é materializada.

## Nitidez e convolução — proposto

Unsharp mask combina fonte e baixa frequência, com raio, intensidade e threshold.
Convolução separável e FFT atendem cenários distintos. FFT muda ordem das somas;
precisa declarar tolerância, limites de eixo e orçamento de scratch. Um threshold
sensível, entrada instável ou geometria não suportada pode exigir o caminho exato.

Não transferir automaticamente a otimização de um filtro para High Pass, Smart
Sharpen ou a nitidez V4L2. São contratos diferentes. Medir o comando completo e
conferir bordas e alpha antes de escolher o backend.

## Pixelização, células e meios-tons — proposto

Crystallize e pointillize podem preparar geometria, raio e jitter por célula,
em vez de repeti-los por pixel. Células internas e células na borda têm fontes
válidas diferentes; preservar médias de fallback, seeds, vencedor e ordem dos
empates. Spans só podem compartilhar resultado quando o teste de distância e
sua quantização provarem o mesmo vencedor.

Facet pode reutilizar luminância e quadrantes de uma vizinhança em um buffer
circular. Meios-tons coloridos podem memoizar cálculo de tinta e raio na grade
rotacionada por canal. Limitar tabelas auxiliares e manter um caminho escalar
quando halo ou coordenadas não atenderem às precondições. Testar seleções suaves,
transparência e formatos de armazenamento suportados.

## Pintura estilizada — proposto

Uma convolução ao longo de streamline depende do campo de orientação amostrado
em cada passo. Reutilizar o caminho do pixel vizinho pode alterar o filtro.
Antes de trocar `floor` por cast, comprovar domínio não negativo e limitado.
Arredondamento para inteiro precisa preservar ties e extremos; a soma em f32
pode arredondar antes do cast e produzir um índice diferente.

Evitar chamadas matemáticas repetidas pode trazer ganho sem uma nova ISA, mas a
prova vem da saída e da medição completa. Pré-computar planos extras, noise ou
pesos pode custar mais memória sem ganho estável; hipóteses descartadas devem
ser registradas como tal, sem transformar ruído de benchmark em otimização.

## Geometria, lente e filtros especializados — proposto

Extrusão por blocos pode calcular cor e profundidade uma vez por célula e depois
rasterizar seus limites. Vento/rajada pode visitar cada streak uma vez, guardando
a regra de fonte mais próxima. Rotação local pode preparar trigonometria por
contagem de amostras. Correção de lente pode compartilhar coordenadas e pesos
Catmull-Rom entre canais, exceto quando aberração cromática muda as coordenadas.

Expansão de tinta para prepress pode usar máximos separáveis e buffers transpostos
em vez de examinar janelas completas para cada raio. Esses filtros não fazem
parte do controle de câmera atual e não devem ser confundidos com compensação
de exposição, estabilização óptica ou calibração de lente do hardware.

Redimensionamento por seam carving remove trajetórias segundo uma energia.
Mapeamento de coordenadas originais pode usar listas ordenadas e busca binária,
mas inserções e programação dinâmica continuam tendo custo. Uma mudança no
bookkeeping não comprova ganho no pipeline inteiro nem recuperação de conteúdo.
Não é uma implementação de reenquadramento automático por pessoa.

## Texto, strokes e filtros não destrutivos — proposto

Texto sobre o vídeo, formas, contornos e marcações exigiriam rasterização,
fontes, alinhamento de pixels e regra de alpha. Passagens independentes podem
ser paralelizadas; prefix sums e winding com dependências entre linhas precisam
preservar o carry. Comparar saída com uma thread e múltiplas threads antes de
prometer equivalência.

Uma pilha não destrutiva futura deve armazenar parâmetros, ordem, ativação e
identidade do efeito. Reabrir um efeito precisa partir da entrada anterior a ele,
não do resultado já filtrado. Efeito indisponível não pode ser regravado como
neutro sem informar perda. Hoje há uma configuração única de efeitos, sem camadas,
Smart Filters, máscaras editáveis ou exportação de pilha para formatos gráficos.

## Verificação e custo

Para cada algoritmo novo, fornecer cenário sintético determinístico, referência,
limites de trabalho, cancelamento, política de borda e contrato de erro. A
implementação atual tem RGB24; uma expansão a profundidades e modelos de cor
adicionais requer tipos e testes próprios. [Desempenho](DESEMPENHO.md) define como
comparar resultados e tempos sem importar números de outro produto.
