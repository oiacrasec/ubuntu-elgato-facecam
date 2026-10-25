# Desempenho e orçamento de recursos

Meça o fluxo que a pessoa utiliza e confira a saída antes de aceitar uma
otimização. Os [resultados locais](VALIDACAO_LOCAL.md) têm ambiente e escopo próprios.

## Cenários e métricas

Separar inicialização até estado pronto, primeira janela pintada, primeiro frame,
captura aquecida, preview, escrita de controle, troca de efeitos e encerramento.
Medir também saída virtual consumida por outro processo; frames de preview não
substituem essa medição. Testar efeito desligado, LUT, denoise e IA separadamente.

| Recurso | Medição necessária |
| --- | --- |
| Latência | Entrada até resultado; separar startup, fila, processamento e apresentação |
| Taxa | Frames por segundo de captura, processamento e consumidor, com perdas |
| CPU | Tempo de processo/conjunto e convenção de porcentagem por núcleo |
| RAM | RSS ou pico com método declarado; soma pode duplicar páginas compartilhadas |
| GPU | Memória do worker, utilização e transferências; não confundir VRAM com RSS |
| Disco | Binário, runtime/modelo, cache, temporários e logs separadamente |
| Estabilidade | Tempo de execução, parada, repetição e ausência de filhos remanescentes |

## Método reproduzível

Registrar revisão, toolchain, flags, resolução, FPS, formato, parâmetros, provider,
GPU/driver, CPU, memória e sessão gráfica. Compilar antes de medir; manter builds
e outros benchmarks fora do intervalo. Aquecer e alternar a ordem antes/depois,
com múltiplas amostras e mediana calculada antes de arredondar.

Uma comparação de kernel exclui etapas diferentes de uma comparação de comando.
Indicar se leitura, conversão, escrita, alocação, publicação e apresentação estão
incluídas. Guardar amostras brutas com unidade, timestamp, parâmetros e hashes da
saída sintética. Não reutilizar uma tabela de outra máquina como baseline local.

## Correção antes da otimização

Compare pixels de versões anterior/nova e CPU/GPU usando o mesmo fixture. Defina
exatidão bit a bit quando ela é exigida; quando há conversão ou inferência com
variação numérica permitida, declare tolerância e o motivo. Inclua alpha, bordas,
empates, negativos e não finitos nos algoritmos float que os aceitam. A máscara
atual de IA rejeita não finitos em vez de propagá-los.

Uma mudança na ordem de somas, arredondamento, tie-breaking ou seed pode mudar o
resultado. Não descreva uma aproximação como otimização equivalente. Mantenha um
caminho escalar ou uma implementação de referência nos testes quando necessário.
Tempo menor em um caso não comprova complexidade melhor ou ganho em todos os casos.

## Evitar trabalho repetido

A implementação atual limita o preview e conserva somente um frame. A captura
sem preview não produz RGB para a GUI. O modelo ONNX permanece carregado no
worker durante a captura; não é criado para cada frame. Desfoque usa duas passagens
de janela deslizante em imagem reduzida.

Somente preview com IA reduz resolução e FPS antes do worker. A saída virtual
mantém a resolução e aplica `ai_fps`, padrão 30. Coordenadas e buffer horizontal
da composição são reutilizados, e fundos estáticos são preparados uma vez.
Aplicar efeitos verifica a inferência antes de parar a captura; o worker real
aquece sua sessão, sem um terceiro processo de verificação no supervisor.

Caches futuros devem guardar invariantes: geometria, pesos de interpolação, LUT,
recursos compilados ou transformações de cor. Um frame de câmera muda continuamente;
não reutilize uma composição anterior só porque os sliders não mudaram. Chaves de
cache devem incluir dimensões, formato, parâmetros, revisão, provider e estado
do dispositivo que alterem o resultado.

Para prefixos de uma pilha de efeitos, só reutilizar estados em fronteiras em que
todo o contexto necessário possa ser restaurado. Invalidar após troca de fonte,
perda do dispositivo, formato, modelo ou orçamento. Um cache com colisões ou
invalidação incompleta é erro de imagem, não ganho aceitável de velocidade.

## Acesso contíguo, buffers e paralelismo

Ler linhas e canais em blocos reduz chamadas por pixel; compartilhar coordenadas
e pesos entre canais preserva o cálculo quando as coordenadas são iguais.
Reutilizar buffers evita picos e alocações por iteração. Ao mudar AoS/SoA,
interleaving, quantização ou writer comprimido, medir o ciclo completo e comparar
os bytes ou amostras produzidos.

Uma região pequena com halo pode custar menos que uma imagem inteira. Atingir a
borda e recortar não são operações intercambiáveis: o suporte do filtro e a ordem
de somas precisam ser preservados. O Frame RGB completo do fluxo atual não é uma
superfície esparsa em tiles; não anunciar economia de tiles que não existe.

Não aumentar threads por padrão sem medir oversubscription entre FFmpeg, ONNX,
GPU e GUI. Workers precisam de limites de memória e cancelamento. SIMD, ISA nativa
ou uma dependência nova só se justificam por ganho no cenário completo e pelos
alvos realmente suportados, mantendo o código da aplicação sem `unsafe`.

## Orçamentos e dados de medição

O orçamento do aplicativo leve separa custo básico e componentes opcionais.
Não há atualmente um arquivo de budgets com enforcement de regressão em CI.
Os números de `VALIDACAO_LOCAL.md` são observações. Um budget futuro precisa de
cenário estável, tolerância a ruído, mediana, revisão-base e política de atualização.

Dados brutos devem descrever nome do caso, entrada, profundidade/canais, número
de threads, execução, duração, estatística e hash da saída. Validar consistência
entre JSON e tabela derivada; manter exemplos gerados fora do tempo medido.
Medições de texto, compressão, máscaras e filtros especializados têm métodos em
[Algoritmos](ALGORITMOS_DE_EFEITOS.md), sem prometer esses efeitos na UI atual.
