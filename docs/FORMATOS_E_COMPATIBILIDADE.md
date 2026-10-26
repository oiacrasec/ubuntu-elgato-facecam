# Formatos, importação e fidelidade

Os formatos atuais são os modos V4L2 da câmera, os frames de vídeo da pipeline,
a configuração JSON, LUTs 3D, fundos PNG/JPEG/WebP/BMP, vídeos e o modelo ONNX. Formatos de documentos gráficos
não são importadores implementados no Facecam Hub.

## Contratos atuais

| Entrada ou saída | Contrato |
| --- | --- |
| Câmera física | Modos discretos realmente enumerados; não presumir resolução por nome do produto |
| Captura | MJPEG ou formato raw suportado pelo dispositivo e pelo FFmpeg |
| Preview | RGB24 derivado dos mesmos quadros yuv420p da saída virtual, mesma resolução e FPS; somente último quadro completo |
| Saída virtual | yuv420p, resolução negociada; com IA, taxa limitada por `ai_fps` |
| Inferência | RGB float32 NCHW 1×3×256×256; máscara de pessoa 256×256 em [0,1] |
| Configuração | JSON com defaults, campos extras na raiz e controles assinados |
| LUT | `.cube` UTF-8 até 16 MiB, grade 2–65 com N³ cores finitas; filtro `lut3d` do FFmpeg |
| Fundo próprio | PNG/JPEG/WebP/BMP convertido para RGB8 opaco, recorte central para preencher a saída |
| Fundo animado | MP4/MOV/WebM/MKV local até 4 GiB, em loop sem áudio; decoder FFmpeg |

Dimensões, canais, orientação, stride, faixa e espaço de cor precisam ser
explicitados em qualquer novo caminho público. O vídeo atual é RGB/YUV de 8 bits;
não há suporte anunciado a vídeo de 16 bits, float HDR ou gestão ICC de monitor.
Um consumidor que abre o nó virtual também precisa interpretar corretamente
formato, níveis e matriz de cor. Negociação aceita não comprova equivalência
numérica ou de aparência com outro aplicativo.

## Presets, manifesto e compatibilidade

O documento nativo atual é `config.json`. A leitura aplica defaults e preserva
extensões desconhecidas na raiz. Não se deve prometer preservação arbitrária de
campos desconhecidos dentro de cada subestrutura. `saved_controls` mantém i64,
inclusive negativos e zero. SAVE e R operam no computador, sem escrever firmware.

Um pacote de presets futuro precisa versionar seu manifesto, identificar arquivos
associados e declarar política de migração. Modelos e LUTs devem ser referenciados
com licença e hash quando redistribuídos. Snapshots, deduplicação por conteúdo,
compressão e recuperação após crash ainda não formam um formato de sessão.
Não crie uma extensão nativa proprietária apenas para reproduzir outra arquitetura.

## Imagens raster e fundos

A troca de fundo lê PNG, JPEG, WebP e BMP com orientação EXIF aplicada quando disponível, até
32 MiB e 8192×8192, respeitando o limite de decodificação de 128 MiB. Transparência
é composta sobre `background_color`; o resultado é RGB8 opaco. Metadados não são
exportados, e não há gestão ICC de imagem. Apenas uma imagem é usada; WebP animado
não é reproduzido como vídeo. A aplicação não oferece exportação desses formatos.

Uma extensão futura precisa definir matriz própria para TIFF, GIF, Netpbm,
EXR, AVIF/HEIF e outros formatos. Não herdar suporte de um codec transitivo sem
validar o caminho da aplicação.

A biblioteca apresenta nomes em vez de caminhos. A importação copia os arquivos
sem converter nem remover metadados; o processamento e a miniatura decodificam
somente o conteúdo visual. [Efeitos](EFEITOS.md) define descoberta e persistência.

A matriz deve informar profundidade, alpha, ICC, EXIF, XMP, resolução, orientação,
animação e multipágina. “Lê e escreve” não equivale a round-trip de metadados.
Orientação deve ser aplicada uma vez e transformações de cor não devem ocorrer
silenciosamente duas vezes. Para sequências, definir política de tempo e frame
inicial; não apresentar o primeiro frame de um GIF como animação funcional.

Limites precisam cobrir dimensões, pixels, bytes decodificados, canais, número de
frames e metadados. Um arquivo comprimido pequeno pode materializar muitos dados.
Formatos HEIF/HEVC requerem análise técnica e de distribuição antes de inclusão;
não há esse decoder opcional no produto atual.

## Documentos em camadas e interoperabilidade

PSD/PSB, XCF, OpenRaster, KRA, Paint.NET e documentos Affinity representam modelos
que incluem camadas, máscaras, modos de mesclagem, texto, vetores e recursos
específicos de cada formato. O controlador de câmera não possui esse modelo.
Esses formatos permanecem fora do escopo atual, com os seguintes critérios se
alguma importação vier a ser necessária:

- Distinguir leitura, escrita, preview embutido, edição semântica e preservação opaca.
- Preservar blocos desconhecidos quando houver promessa de round-trip; não declarar
  byte a byte se o writer normalizar nomes, padding, compressão ou metadados.
- Separar parser standalone de conversão para o modelo da aplicação. Um parser pode
  preservar bytes que um modelo reduzido não consegue representar.
- Verificar limites de arquivos grandes, offsets, endianess, padding, dados de canais,
  predição, streams comprimidos e dimensões de máscaras antes da alocação.
- Comparar composição com uma referência autorizada e testar a reabertura no
  consumidor de destino; as duas provas têm escopos diferentes.

Luz linear versus codificada, alpha direto versus premultiplicado, clipping e
clamping após composição podem produzir diferenças visíveis mesmo quando os
parâmetros importados são iguais. Uma imagem de preview preservada não comprova
que todas as camadas possam ser recalculadas.

## SVG, formas, texto e impressão

SVG pode incluir recursos externos, texto e referências recursivas. Um importador
futuro precisa distinguir rasterizar, inserir e manter vetores editáveis, com
orçamento para referências e conteúdo externo. Recursos de fonte ausentes devem
ser informados; substituição visual não comprova fidelidade tipográfica.

O produto não tem exportação de SVG, strokes vetoriais, texto editável, camadas
ou impressão. IPP, CUPS, driver de impressora e diálogo do sistema são integrações
distintas da saída V4L2. A existência de um preview de vídeo não constitui um
pipeline de impressão, DPI de papel ou prova de gerenciamento de cor.

## RAW de sensor e formatos especializados

Arquivos DNG, CR2/CR3, NEF, ARW e RAF são dados de sensor, não o vídeo raw RGB/YUV
já desenvolvido que uma webcam entrega. Não aplicar às câmeras UVC uma matriz de
calibração, curva ou crop obtidos de um modelo fotográfico diferente.

Um desenvolvimento RAW futuro precisa distinguir área ativa, níveis de preto e
branco, linearização, balanço de branco, matriz de cor e aparência do JPEG
embutido. Um ajuste de aparência por preview não equivale a calibração medida.
O crop depende do modelo e do tamanho do sensor, ocorre antes da orientação e
não deve apagar um crop explícito do arquivo. Reaplicar controles do usuário
não pode ser absorvido por um novo fitting que anule a edição.

DDS, FITS, DPX, Cineon, DICOM e IFF também exigem leitores específicos, semântica
de amostras e limites próprios. Nenhum deles é necessário para o fluxo de câmera
atual. Caso se tornem entradas de teste ou fundo, documentar suporte real antes
de aceitar a extensão e manter dados médicos ou pessoais fora do corpus público.

## Escrita segura e durabilidade

O salvamento atual mantém um backup inicial e publica um arquivo temporário por
rename. Isso não promete fsync, preservação de ACL/xattrs, streams alternativos
ou recuperação de bundles após queda de energia. Uma futura publicação durável
precisa definir a barreira de confirmação, falha antes/depois do rename, retries
limitados e proteção de arquivo original. [Segurança](SEGURANCA.md) registra os limites.
