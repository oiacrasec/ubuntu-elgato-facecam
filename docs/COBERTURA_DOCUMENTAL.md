# Cobertura documental

O inventário contém 152 arquivos documentais, dos quais 137 são Markdown,
na revisão auditada em 25/10/2026. Cada entrada possui assunto, destino, adaptação
e hash do conteúdo examinado. O índice de leitura é o [manual](README.md).

## Escopo e critérios

Foram classificados os textos da raiz, livro, documentação técnica e de usuário,
READMEs de componentes, exemplos e empacotamento, dados de benchmark e licenças
associadas. As páginas repetidas convergem para um documento responsável.
O inventário de hashes e o mapeamento de auditoria ficam como evidência local em
`package/documentation-audit/`, ignorado pelo Git. Os textos originais não fazem
parte da árvore do projeto; a documentação mantida tem redação própria.

A adaptação é temática e técnica, com redação própria. As funções de um editor de
documentos foram relacionadas aos contratos de imagem/vídeo ou registradas como
estudos e decisões de escopo. Os números, nomes de comandos, garantias, licenças
e infraestrutura de outro produto não são atribuídos a este aplicativo. Licenças
não concedem direitos sobre marcas ou recursos não usados.

Imagens de referência e marcas não são incorporadas; a UI tem capturas próprias.
Links a processos privados não disponíveis no acervo não são tratados como regras
conhecidas ou como dependências existentes. Esse é o limite da revisão, sem
inventar conteúdo que não está nos arquivos examinados.

O teste documental exige IDs únicos, contagem completa e destinos existentes.
Isso verifica rastreabilidade; a revisão editorial continua responsável pela
qualidade da adaptação. O [manifesto JSON](cobertura-documental.json) conserva
os hashes completos e é a fonte dos registros abaixo.

A preparação virtual com confirmação está em [Uso](USO.md#preparar-a-câmera-virtual),
com contratos em [Arquitetura](ARQUITETURA.md) e [Protocolo](PROTOCOLO_DE_CONTROLE.md).
[Testes](TESTES.md) separa checks comuns da aceitação física no consumidor.
Os redistribuíveis da referência Windows e os critérios da integração de
IA estão em [Runtimes NVIDIA](NVIDIA_MAXINE.md),
sem alterar o inventário das fontes documentais examinadas.
Os resultados novos da engenharia estão em [Análise RTX](ANALISE_RTX.md), com
[catálogo de referências](nvidia-runtime-catalog.json) próprio. Esses registros
não acrescentam arquivos ao inventário histórico de 152 fontes.

## Registros por arquivo documental

Preview fiel à saída, espelhamento compartilhado e limites de apresentação estão
em [Uso](USO.md#visualizar-e-enviar-a-outro-aplicativo), com fluxo em
[Arquitetura](ARQUITETURA.md#captura-e-efeitos) e teste sintético em [Testes](TESTES.md).
Essa evolução não altera as 152 fontes do inventário histórico.

Processamento CUDA, transferências CPU/GPU e comparação numérica com a referência
CPU estão em [Desempenho](DESEMPENHO.md), [Arquitetura](ARQUITETURA.md) e
[Testes](TESTES.md). As medições locais ficam em [Validação local](VALIDACAO_LOCAL.md).
Os IDs e hashes históricos permanecem preservados.

A remoção de imagens importadas está em [Uso](USO.md), com autorização e estado
em [Arquitetura](ARQUITETURA.md) e [Protocolo](PROTOCOLO_DE_CONTROLE.md). A revisão
das lacunas para conversão completa fica em [Roadmap](ROADMAP.md), sem acrescentar
fontes ao inventário histórico.

| ID | Assunto | Destino | Adaptação | SHA-256 (prefixo) |
| --- | --- | --- | --- | --- |
| D001 | Regras para manutenção e agentes | [CONTRIBUICAO.md](CONTRIBUICAO.md) | Comandos no núcleo, erros de entrada, recursos próprios e revisão verificável. | `2fcac6815bcd` |
| D002 | Atribuição e recursos de terceiros | [DEPENDENCIAS_E_LICENCAS.md](DEPENDENCIAS_E_LICENCAS.md) | Inventário de origem e licença dos recursos usados; redistribuíveis NVIDIA locais identificados como artefatos de análise. | `f9c076102d8e` |
| D003 | Licenças permissivas e avisos | [DEPENDENCIAS_E_LICENCAS.md](DEPENDENCIAS_E_LICENCAS.md) | Condições de componentes externos não substituem a licença MIT local. | `0d542e0c8804` |
| D004 | Licença do código | [DEPENDENCIAS_E_LICENCAS.md](DEPENDENCIAS_E_LICENCAS.md) | Distinguir licença do projeto e direitos de recursos opcionais. | `1a8f66d8a86b` |
| D005 | Notices e proveniência | [DEPENDENCIAS_E_LICENCAS.md](DEPENDENCIAS_E_LICENCAS.md) | Preservar avisos aplicáveis sem incorporar recursos que não são distribuídos. | `25a69f3364a0` |
| D006 | Apresentação e primeira execução | [README.md](README.md) | Manual, caminhos operacionais, dependências e limites do produto Linux. | `a889dff2d23b` |
| D007 | Roadmap resumido e prontidão | [ROADMAP.md](ROADMAP.md) | Matriz por dimensão sem importar porcentagens ou estimativas externas. | `8ff8c3dae3f1` |
| D008 | Política de segurança | [SEGURANCA.md](SEGURANCA.md) | Escopo, relato mínimo e ausência de canal privado ou SLA definido. | `34988a5b1c09` |
| D009 | Licença de ícone | [DEPENDENCIAS_E_LICENCAS.md](DEPENDENCIAS_E_LICENCAS.md) | Adoção de ícone requer proveniência e licença próprias. | `a5cf25f3f3c2` |
| D010 | Geometria, paleta e geração de ícone | [DISTRIBUICAO.md](DISTRIBUICAO.md) | Identidade e legibilidade de artefatos próprios; nenhum ícone externo incorporado. | `25a24e441942` |
| D011 | Dicionários e licenciamento | [DEPENDENCIAS_E_LICENCAS.md](DEPENDENCIAS_E_LICENCAS.md) | Dicionários não fazem parte do runtime da câmera. | `e2a852424fe4` |
| D012 | Licenciamento de fontes de UI | [LOCALIZACAO.md](LOCALIZACAO.md) | Fontes externas exigem licença e orçamento; uso atual de default_fonts. | `262481e84452` |
| D013 | Licenciamento de fontes numéricas | [LOCALIZACAO.md](LOCALIZACAO.md) | Números, fallback e legibilidade sem incorporar arquivos de fonte externos. | `a76abf002c49` |
| D014 | Licenciamento de conjunto de ícones | [DEPENDENCIAS_E_LICENCAS.md](DEPENDENCIAS_E_LICENCAS.md) | Ícones de terceiros têm condições independentes da licença do código. | `b495047bd93a` |
| D015 | Atribuição de ícone especializado | [DEPENDENCIAS_E_LICENCAS.md](DEPENDENCIAS_E_LICENCAS.md) | Nenhum asset é trazido apenas porque seu texto de licença foi examinado. | `667185ed8a37` |
| D016 | Atribuição de símbolos de simetria | [DEPENDENCIAS_E_LICENCAS.md](DEPENDENCIAS_E_LICENCAS.md) | Separar documentação de recurso gráfico efetivamente redistribuído. | `d43c48d16898` |
| D017 | Configuração de livro e publicação | [PADROES_DE_DOCUMENTACAO.md](PADROES_DE_DOCUMENTACAO.md) | Índice Markdown atual e schema de publicação apenas quando houver consumidor. | `d891ac9664d1` |
| D018 | Entrada do manual e linguagem de status | [README.md](README.md) | Definir Implementado, Limitação, Proposto e Fora do escopo. | `0f42bf998b8f` |
| D019 | Sumário e navegação | [README.md](README.md) | Índice completo com destinos responsáveis e sem duplicar explicações. | `c1505da2397d` |
| D020 | Arquitetura de automação | [EXTENSOES.md](EXTENSOES.md) | Adaptadores convergem para Request e Snapshot; MCP permanece proposto. | `5b46768b6023` |
| D021 | Motor de comandos e contrato de entrada | [PROTOCOLO_DE_CONTROLE.md](PROTOCOLO_DE_CONTROLE.md) | Variantes reais, parâmetros, disponibilidade e erros do controlador. | `809782e7cc9e` |
| D022 | Camadas e dependências | [DESENVOLVIMENTO.md](DESENVOLVIMENTO.md) | Mapeamento por módulos atuais sem criar um workspace fictício. | `971abd16c5ea` |
| D023 | Visão arquitetural | [ARQUITETURA.md](ARQUITETURA.md) | UI fina, controlador único, captura e processamento isolados da apresentação. | `9325980979da` |
| D024 | Renderização e verificação | [DESEMPENHO.md](DESEMPENHO.md) | Comparação de saída, formatos explícitos e distinção entre preview e consumidor. | `2ec19a3aac5b` |
| D025 | Linha de comando | [PROTOCOLO_DE_CONTROLE.md](PROTOCOLO_DE_CONTROLE.md) | Flags atuais, arquivos de efeitos e recursos de CLI ainda não implementados. | `20dc10c1f0ed` |
| D026 | Protocolo de controle | [PROTOCOLO_DE_CONTROLE.md](PROTOCOLO_DE_CONTROLE.md) | Socket Unix, JSON por linha, limites e resultado observável. | `f3d8f612ccef` |
| D027 | MCP e bridge | [EXTENSOES.md](EXTENSOES.md) | Desenho futuro de sessão e bridge sem anunciar ferramentas inexistentes. | `cc5fcb6aed63` |
| D028 | Superfícies de automação | [EXTENSOES.md](EXTENSOES.md) | Distinguir execução própria e encaminhamento para a instância ativa. | `691c8424d4ff` |
| D029 | Segurança de automação | [SEGURANCA.md](SEGURANCA.md) | Autoridade local atual e capabilities futuras explicitamente propostas. | `da7548fcc5d9` |
| D030 | Política de dependências | [DEPENDENCIAS_E_LICENCAS.md](DEPENDENCIAS_E_LICENCAS.md) | Lockfile, revisão de bibliotecas nativas e lacunas de auditoria automática. | `889e66955605` |
| D031 | Fuzzing de desenvolvimento | [TESTES.md](TESTES.md) | Entradas adversariais e regressões; job dedicado ainda ausente. | `be4c11c644e9` |
| D032 | Segurança de publicação | [DISTRIBUICAO.md](DISTRIBUICAO.md) | Hashes, promoção de build, segredos e garantias ainda não implementadas. | `e4ae5d921717` |
| D033 | Estratégia de testes | [TESTES.md](TESTES.md) | Checks relevantes, isolamento de hardware e contratos de processo. | `3329606e531e` |
| D034 | Importação de documentos proprietários | [FORMATOS_E_COMPATIBILIDADE.md](FORMATOS_E_COMPATIBILIDADE.md) | Fora do escopo atual; leitura, preview e escrita são contratos distintos. | `368eacebe8a2` |
| D035 | Visão de formatos | [FORMATOS_E_COMPATIBILIDADE.md](FORMATOS_E_COMPATIBILIDADE.md) | Matriz de vídeo, biblioteca de recursos, importação validada, LUT e ONNX realmente disponíveis; uso em [Efeitos](EFEITOS.md). | `c48faddfa215` |
| D036 | Formato nativo, limites e recuperação | [FORMATOS_E_COMPATIBILIDADE.md](FORMATOS_E_COMPATIBILIDADE.md) | JSON atual e critérios para manifesto, presets, blobs e recuperação futuros. | `07439af9ba81` |
| D037 | Documentos grandes em camadas | [FORMATOS_E_COMPATIBILIDADE.md](FORMATOS_E_COMPATIBILIDADE.md) | Parser, conversão, preservação opaca e aceitação por consumidor separados. | `737f47bdbaaf` |
| D038 | Formatos raster e metadados | [FORMATOS_E_COMPATIBILIDADE.md](FORMATOS_E_COMPATIBILIDADE.md) | Profundidade, orientação, alpha, animação, metadata e limites de decode. | `d7292d37dddc` |
| D039 | SVG e inserção de vetores | [FORMATOS_E_COMPATIBILIDADE.md](FORMATOS_E_COMPATIBILIDADE.md) | Recursos externos, texto e limites para uma integração futura. | `3ac4894dc5bb` |
| D040 | Build e ferramentas | [DESENVOLVIMENTO.md](DESENVOLVIMENTO.md) | Comandos Cargo reais, pré-requisitos e compilação mínima. | `d424d8098857` |
| D041 | Contribuição e confiança | [CONTRIBUICAO.md](CONTRIBUICAO.md) | Regras Rust, erro, camadas e documentação na mesma alteração. | `30b667f95866` |
| D042 | Instalação e confiança nos arquivos | [USO.md](USO.md) | Primeira execução, estado dpkg, instalação/remoção APT e preparação virtual com confirmação, sem instalador de outro sistema. | `5e332a2a69d6` |
| D043 | Créditos | [DEPENDENCIAS_E_LICENCAS.md](DEPENDENCIAS_E_LICENCAS.md) | Nomes voluntários e distinção entre autoria, modelo e estatística gerada. | `4eb5df4d2b3e` |
| D044 | Licenças de projeto e recursos | [DEPENDENCIAS_E_LICENCAS.md](DEPENDENCIAS_E_LICENCAS.md) | Preservar notices dos componentes realmente distribuídos. | `e7bd6ca6bba1` |
| D045 | Planejamento do produto | [ROADMAP.md](ROADMAP.md) | Prioridades próprias e gates de hardware, consumidores e distribuição. | `d7e2d0501291` |
| D046 | Sessões e budgets de automação | [SEGURANCA.md](SEGURANCA.md) | Controles existentes separados de autenticação, scopes e auditoria futuros. | `288b57396837` |
| D047 | Regressões e corpus seguro | [TESTES.md](TESTES.md) | Fixtures mínimas, geração sintética e corpus externo com revisão/hash. | `957b0211dbef` |
| D048 | Postura de segurança | [SEGURANCA.md](SEGURANCA.md) | Proteções e limitações por fronteira, sem garantias universais. | `79ffbf707d12` |
| D049 | Endurecimento de parsers | [SEGURANCA.md](SEGURANCA.md) | Offsets, tamanhos, profundidade, decode e alocação antes do trabalho caro. | `9f48ee418b40` |
| D050 | Arquivos e capabilities | [SEGURANCA.md](SEGURANCA.md) | Backup/rename atual; handles, races e autoridade de filesystem como evolução. | `a0d27ba1d63e` |
| D051 | Modelo de ameaças | [SEGURANCA.md](SEGURANCA.md) | Ativos, atores, pontos de entrada e objetivos do aplicativo local. | `217b845b4fb8` |
| D052 | Fronteiras de confiança | [SEGURANCA.md](SEGURANCA.md) | Responsabilidades entre GUI, IPC, engine, processos e runtime nativo. | `a1b9c2e16dd7` |
| D053 | Reporte de vulnerabilidade | [SEGURANCA.md](SEGURANCA.md) | Reprodução mínima sem segredos e canal privado ainda a definir. | `f638ea2b5b97` |
| D054 | Parser de documentos e clean-room | [FORMATOS_E_COMPATIBILIDADE.md](FORMATOS_E_COMPATIBILIDADE.md) | Critérios de fidelidade, preview opaco, limites e dados autorizados. | `510ab12e5808` |
| D055 | Gestão de cor e perfis | [FORMATOS_E_COMPATIBILIDADE.md](FORMATOS_E_COMPATIBILIDADE.md) | Contratos RGB/YUV atuais e ICC/calibração não implementados. | `3e35e709ded9` |
| D056 | Codecs e matriz de capacidade | [FORMATOS_E_COMPATIBILIDADE.md](FORMATOS_E_COMPATIBILIDADE.md) | Ler, escrever, profundidade, metadados e assimetrias precisam de prova própria. | `544a6a775ec8` |
| D057 | Decoder opcional e distribuição | [DEPENDENCIAS_E_LICENCAS.md](DEPENDENCIAS_E_LICENCAS.md) | Recursos nativos opcionais e análise técnica/licença antes de inclusão. | `b0d123debb0f` |
| D058 | Impressão e IPP | [FORMATOS_E_COMPATIBILIDADE.md](FORMATOS_E_COMPATIBILIDADE.md) | Impressão é uma integração separada e não existe no fluxo V4L2 atual. | `26caddcef72b` |
| D059 | Round-trip de documentos estruturados | [FORMATOS_E_COMPATIBILIDADE.md](FORMATOS_E_COMPATIBILIDADE.md) | Padding, endianess, canais e preservação desconhecida como critérios futuros. | `2339485758c1` |
| D060 | Desenvolvimento RAW de sensor | [FORMATOS_E_COMPATIBILIDADE.md](FORMATOS_E_COMPATIBILIDADE.md) | Crop, orientação, níveis e calibração não equivalem a raw RGB/YUV de webcam. | `bca1d46f9350` |
| D061 | Arquitetura detalhada e decisões | [DESENVOLVIMENTO.md](DESENVOLVIMENTO.md) | Dados, operações, snapshots, jobs e extensão adaptados aos módulos reais. | `373d14e11c8a` |
| D062 | Batch e templates de nome | [EXTENSOES.md](EXTENSOES.md) | Desenho futuro com parâmetros estáveis, colisões e efeitos parciais explícitos. | `c3cee190eeb7` |
| D063 | Marcas e recursos restritos | [DEPENDENCIAS_E_LICENCAS.md](DEPENDENCIAS_E_LICENCAS.md) | Marcas não foram incorporadas; licença de código não concede direitos de marca. | `f0a582a0645a` |
| D064 | Pintura, pressão e gestos | [ALGORITMOS_DE_EFEITOS.md](ALGORITMOS_DE_EFEITOS.md) | Máscaras manuais e gestos são propostas com contrato de operação e cancelamento. | `e50d0a38cd40` |
| D065 | Histogramas, scopes e parâmetros | [ALGORITMOS_DE_EFEITOS.md](ALGORITMOS_DE_EFEITOS.md) | Análise em proxy, curvas e diferença entre parâmetros iguais e pixels iguais. | `b63723ac37a8` |
| D066 | Navegação de preview | [UI_E_INTERACAO.md](UI_E_INTERACAO.md) | Separar zoom visual, FOV, coordenadas e amostragem da imagem. | `bff6d85203d3` |
| D067 | Conta-gotas e captura externa | [ALGORITMOS_DE_EFEITOS.md](ALGORITMOS_DE_EFEITOS.md) | Contrato de cor e permissões de compositor para uma função futura. | `adae7eb3f6ea` |
| D068 | Inventário de menus de contexto | [UI_E_INTERACAO.md](UI_E_INTERACAO.md) | Ações futuras precisam da mesma disponibilidade e execução dos botões. | `f7d033a61a60` |
| D069 | Dispatch de menus e superfície de interação | [UI_E_INTERACAO.md](UI_E_INTERACAO.md) | Comando central, coordenadas, estados e verificação visual por cenário. | `2c45b4305607` |
| D070 | Checklist de contribuição | [CONTRIBUICAO.md](CONTRIBUICAO.md) | Novos comandos, testes pertinentes, código próprio e revisão focada. | `38745beffcff` |
| D071 | Créditos e manutenção de autoria | [DEPENDENCIAS_E_LICENCAS.md](DEPENDENCIAS_E_LICENCAS.md) | Não deduzir dados pessoais; gerador ou aba About não existem atualmente. | `c78ceac9e7cd` |
| D072 | Referência ampla de controle | [PROTOCOLO_DE_CONTROLE.md](PROTOCOLO_DE_CONTROLE.md) | Somente comandos existentes são operacionais; batch e preferências futuras têm limites definidos. | `db9fe9e42286` |
| D073 | Registro de decisões | [DECISOES.md](DECISOES.md) | Identidade de recursos, substituição de arquivos e motivação das escolhas. | `5341d87dcc12` |
| D074 | Desenvolvimento e diagnóstico | [DESENVOLVIMENTO.md](DESENVOLVIMENTO.md) | Build, runtime isolado, logs, gráficos, variáveis e serviços explícitos. | `d9afd1f2a55f` |
| D075 | Fidelidade e compatibilidade por formato | [FORMATOS_E_COMPATIBILIDADE.md](FORMATOS_E_COMPATIBILIDADE.md) | Separar capacidade de parser, modelo, visualização e aceitação do arquivo. | `b764c695295e` |
| D076 | Dados brutos de benchmarks de filtros | [DESEMPENHO.md](DESEMPENHO.md) | Esquema de amostras e hashes preservado como método, sem usar tempos externos como locais. | `049d8ea65d6a` |
| D077 | Otimização de cinco famílias de algoritmos | [ALGORITMOS_DE_EFEITOS.md](ALGORITMOS_DE_EFEITOS.md) | Células, streaks, rotação, lente e expansão de tinta com equivalência e custo limitados. | `9e1c6e5f983c` |
| D078 | Resolução de fontes | [LOCALIZACAO.md](LOCALIZACAO.md) | Família, glifos e fallback não garantem fidelidade tipográfica. | `229abcaccc33` |
| D079 | Inventário de lacunas | [ROADMAP.md](ROADMAP.md) | Impacto, evidência e gate de aceitação, sem copiar estimativas de outro domínio. | `f1fd847811cf` |
| D080 | Reuso de composição GPU | [DESEMPENHO.md](DESEMPENHO.md) | Fronteiras seguras, invalidação, orçamento e risco de reusar frames diferentes. | `abe4759f9cb1` |
| D081 | GPU, displays e periféricos | [UI_E_INTERACAO.md](UI_E_INTERACAO.md) | Matriz Linux e validação real; tablets, HID e HDR permanecem fora do suporte anunciado. | `e1a75b644751` |
| D082 | Origem das imagens documentais | [DEPENDENCIAS_E_LICENCAS.md](DEPENDENCIAS_E_LICENCAS.md) | Capturas próprias e dados autorizados, sem reutilizar assets externos. | `ccd8ec0d8629` |
| D083 | Alinhamento físico de pixels | [UI_E_INTERACAO.md](UI_E_INTERACAO.md) | DPI e coordenadas lógicas; comparação visual deve isolar escala e interpolação. | `bff557e0df73` |
| D084 | Desfoque com geometria de foco | [ALGORITMOS_DE_EFEITOS.md](ALGORITMOS_DE_EFEITOS.md) | Níveis gaussianos, invariantes, bordas e referência numérica para estudo futuro. | `8073e55aab67` |
| D085 | Contêiner de documento em camadas | [FORMATOS_E_COMPATIBILIDADE.md](FORMATOS_E_COMPATIBILIDADE.md) | KRA fora do escopo; estruturas, recursos e composição requerem importador próprio. | `22a614dc1a40` |
| D086 | Terminologia espanhola | [LOCALIZACAO.md](LOCALIZACAO.md) | Revisão nativa e cobertura de textos dinâmicos, sem alegar catálogo disponível. | `e1d82eec1e78` |
| D087 | Terminologia italiana | [LOCALIZACAO.md](LOCALIZACAO.md) | Registro, termos de câmera e estados de revisão explícitos. | `514ed396be4b` |
| D088 | Terminologia coreana | [LOCALIZACAO.md](LOCALIZACAO.md) | Espaçamento, formalidade e glifos precisam de testes próprios. | `bd9c9621c894` |
| D089 | Cobertura entre idiomas | [LOCALIZACAO.md](LOCALIZACAO.md) | Contar erros, menus, plurais e textos gerados, além das chaves estáticas. | `22c59408999d` |
| D090 | Terminologia russa | [LOCALIZACAO.md](LOCALIZACAO.md) | Plurais, placeholders e revisão linguística por idioma. | `3396649f6076` |
| D091 | Chinês simplificado | [LOCALIZACAO.md](LOCALIZACAO.md) | Distinguir script simplificado/tradicional e fallback de fontes. | `a857b918a34d` |
| D092 | Idioma ao vivo e fallback | [LOCALIZACAO.md](LOCALIZACAO.md) | Preferência salva, sistema, cancelamento, bidi e fontes são contratos futuros. | `e501ea45b46d` |
| D093 | Convenções MCP | [EXTENSOES.md](EXTENSOES.md) | Schemas, sessão, resources, progresso e limites sem servidor implementado. | `ad74d8570aa8` |
| D094 | Calibração, crop e fitting de RAW | [FORMATOS_E_COMPATIBILIDADE.md](FORMATOS_E_COMPATIBILIDADE.md) | Modelo/sensor exatos, crop antes da orientação e preservação dos controles do usuário. | `11113ed77903` |
| D095 | Morfologia octogonal de máscaras | [ALGORITMOS_DE_EFEITOS.md](ALGORITMOS_DE_EFEITOS.md) | Footprint, decomposição, empates, bordas e scratch documentados como estudo. | `1ceb6e68240e` |
| D096 | Convolução de pintura estilizada | [ALGORITMOS_DE_EFEITOS.md](ALGORITMOS_DE_EFEITOS.md) | Streamlines e arredondamento exigem prova; vizinhos não compartilham necessariamente o caminho. | `4ec313792e6f` |
| D097 | OpenRaster | [FORMATOS_E_COMPATIBILIDADE.md](FORMATOS_E_COMPATIBILIDADE.md) | Camadas, blend, recursos e limites antes de qualquer importação. | `25a15bb0a059` |
| D098 | Checklist de menus | [ROADMAP.md](ROADMAP.md) | Presença de menu separada de comportamento e evidência de aceitação. | `22c2e1a03d0f` |
| D099 | Documentos Paint.NET | [FORMATOS_E_COMPATIBILIDADE.md](FORMATOS_E_COMPATIBILIDADE.md) | Formato fora do escopo com critérios de preservação e limites explícitos. | `50ee911f4d49` |
| D100 | Células e pointillismo | [ALGORITMOS_DE_EFEITOS.md](ALGORITMOS_DE_EFEITOS.md) | Preparação por célula, spans, bordas, alpha e fallback escalar. | `38fd059397b4` |
| D101 | Amostras de comandos e filtros por célula | [DESEMPENHO.md](DESEMPENHO.md) | Distinguir kernel, filtro e comando; hashes e amostras não são latência de UI. | `88b21749025a` |
| D102 | Amostras de pixelização | [DESEMPENHO.md](DESEMPENHO.md) | Raw data, mediana e equivalência com método reproduzível. | `576b15fef950` |
| D103 | Facet e meios-tons | [ALGORITMOS_DE_EFEITOS.md](ALGORITMOS_DE_EFEITOS.md) | Buffers circulares, memoização por canal e limites de halo/tabelas. | `78e214996627` |
| D104 | ABI e sandbox de plug-ins | [EXTENSOES.md](EXTENSOES.md) | Manifesto, memória, fuel, unidades, alpha e contexto como contrato proposto. | `b5b2c1a49f87` |
| D105 | Impressão e integração de sistema | [FORMATOS_E_COMPATIBILIDADE.md](FORMATOS_E_COMPATIBILIDADE.md) | IPP, orientação, DPI e cor são distintos da câmera virtual e não estão implementados. | `f35e3e2ae388` |
| D106 | Busca de retoque por proximidade | [ALGORITMOS_DE_EFEITOS.md](ALGORITMOS_DE_EFEITOS.md) | Área somada recortada, poda por SSD e regra de empate sem promessa de qualidade. | `e022c3bc6ebe` |
| D107 | Interleaving e alocação de pixels | [DESEMPENHO.md](DESEMPENHO.md) | Comparar leitura, conversão, bytes e memória no ciclo completo. | `5701f7fed78a` |
| D108 | Seleção por grafos | [ALGORITMOS_DE_EFEITOS.md](ALGORITMOS_DE_EFEITOS.md) | Capacidades, fluxo residual, solve repetido e testes de corte mínimo. | `91a23bcd6e93` |
| D109 | Amostragem de blur radial | [ALGORITMOS_DE_EFEITOS.md](ALGORITMOS_DE_EFEITOS.md) | Tabelas de transformações, canais compartilhados e caminho exato de referência. | `bb48d27e32f8` |
| D110 | Playbook de release | [DISTRIBUICAO.md](DISTRIBUICAO.md) | Receita local documentada sem depender de processo privado externo. | `1eb1c7b6fa6c` |
| D111 | Publicação multiplataforma e atualização | [DISTRIBUICAO.md](DISTRIBUICAO.md) | Artefato Linux atual, assinatura futura, pacotes e atualização sem garantias inventadas. | `ec8ccf88aa81` |
| D112 | Marcos e gates | [ROADMAP.md](ROADMAP.md) | Foco, aceitação e estado por cenário em vez de cronograma não medido. | `5554102c18c9` |
| D113 | Scorecard e budgets | [ROADMAP.md](ROADMAP.md) | Separar UI, compatibilidade, desempenho, automação, confiabilidade e distribuição. | `eff304bdefb5` |
| D114 | Seam carving e coordenadas | [ALGORITMOS_DE_EFEITOS.md](ALGORITMOS_DE_EFEITOS.md) | Mapeamento por listas, busca binária e custo residual; não é tracking de pessoa. | `aee12340c058` |
| D115 | Transformada de distância de seleção | [ALGORITMOS_DE_EFEITOS.md](ALGORITMOS_DE_EFEITOS.md) | Duas passagens, precisão de interseção, força bruta e memória auxiliar. | `97493bc39ecb` |
| D116 | Strokes de formas | [ALGORITMOS_DE_EFEITOS.md](ALGORITMOS_DE_EFEITOS.md) | Texto/vetores futuros precisam de geometria, alpha e rasterização próprios. | `18d34c27bd11` |
| D117 | Filtros não destrutivos | [ALGORITMOS_DE_EFEITOS.md](ALGORITMOS_DE_EFEITOS.md) | Identidade, ordem, reabertura da entrada e preservação de efeitos indisponíveis. | `bbbf570a7d1f` |
| D118 | Leituras esparsas de pixels | [DESEMPENHO.md](DESEMPENHO.md) | Acesso contíguo e regiões úteis sem alegar tiles no fluxo RGB atual. | `3a3dbac10c2e` |
| D119 | Avaliação de paridade funcional | [ROADMAP.md](ROADMAP.md) | Amplitude, profundidade, fidelidade e prontidão com evidência separada. | `39607fa09b21` |
| D120 | Custo de rasterização de texto | [ALGORITMOS_DE_EFEITOS.md](ALGORITMOS_DE_EFEITOS.md) | Passagens independentes, prefix sums e carry entre linhas antes de paralelizar. | `f51953a2c62f` |
| D121 | Tilt-shift e campo de sigma | [ALGORITMOS_DE_EFEITOS.md](ALGORITMOS_DE_EFEITOS.md) | Níveis gaussianos, bordas de acumulação, cancelamento e contrato numérico. | `978cced47f0f` |
| D122 | Sistema de design | [UI_E_INTERACAO.md](UI_E_INTERACAO.md) | Tokens, componentes, truncamento, DPI, temas e validação na organização Camera Hub. | `5d0635748470` |
| D123 | Diálogos e foco | [UI_E_INTERACAO.md](UI_E_INTERACAO.md) | Cards de Camera/Effects, abrir/aplicar/cancelar, centralização, dependências, filhos, preview, teclado e parâmetros estáveis. | `969347e5e2e5` |
| D124 | Gestos, handles e navegação | [UI_E_INTERACAO.md](UI_E_INTERACAO.md) | Coordenadas e disponibilidade; ferramentas de documento fora do escopo atual. | `37faf632a7b0` |
| D125 | Unsharp mask e FFT | [ALGORITMOS_DE_EFEITOS.md](ALGORITMOS_DE_EFEITOS.md) | Convolução separável, tolerância, threshold e scratch antes de escolher otimização. | `565140561850` |
| D126 | Regras de documentação de usuário | [PADROES_DE_DOCUMENTACAO.md](PADROES_DE_DOCUMENTACAO.md) | Resposta primeiro, passos, labels exatos, metadados e revisão de afirmações. | `da2394fff872` |
| D127 | Orientação documental para agentes | [PADROES_DE_DOCUMENTACAO.md](PADROES_DE_DOCUMENTACAO.md) | Uma fonte normativa local, ligada a AGENTS e CLAUDE. | `a0f75510722e` |
| D128 | Índice de perguntas frequentes | [USO.md](USO.md) | Navegação de uso, licença, formatos e limites sem duplicar o manual. | `eb7c0d0d172a` |
| D129 | Licença e acesso ao produto | [USO.md](USO.md) | Código MIT, componentes opcionais independentes e ausência de serviço remoto obrigatório. | `02ceb4d94cad` |
| D130 | Compatibilidade de arquivos gráficos | [FORMATOS_E_COMPATIBILIDADE.md](FORMATOS_E_COMPATIBILIDADE.md) | Documentos em camadas não são entradas do aplicativo de câmera. | `b1fe165f2830` |
| D131 | Limites e substituição de outro produto | [ROADMAP.md](ROADMAP.md) | Descrever tarefas suportadas sem prometer paridade integral. | `a6dfdba5103e` |
| D132 | Índice de primeiros passos | [USO.md](USO.md) | Instalação, abrir, selecionar fonte e próximo passo verificável. | `76267fe5ee6d` |
| D133 | Instalação por plataforma e integridade | [DISTRIBUICAO.md](DISTRIBUICAO.md) | Linux e scripts atuais; outras plataformas e assinatura mantidas como propostas. | `af65ea62b5c1` |
| D134 | Inserção de texto e estilos | [ALGORITMOS_DE_EFEITOS.md](ALGORITMOS_DE_EFEITOS.md) | Sobreposições de texto/vetor são futuras, com fontes e rasterização próprias. | `36f8aea43241` |
| D135 | Ajustes não destrutivos de cor | [USO.md](USO.md) | Efeitos locais, rascunho e aplicar separados dos controles do hardware. | `a471870d85d2` |
| D136 | Exportação e preservação de camadas | [USO.md](USO.md) | Saída virtual entrega vídeo; gravação, foto e documentos editáveis não são anunciados. | `dd3b165f038c` |
| D137 | Índice de guias por tarefa | [README.md](README.md) | Navegação canônica com guias operacionais e referência técnica. | `58719c05c9d5` |
| D138 | Abrir, salvar e cópia de trabalho | [USO.md](USO.md) | Configuração/presets e backup no lugar de um modelo de documentos inexistente. | `2f6c386d5dc8` |
| D139 | Seleções e máscaras | [ALGORITMOS_DE_EFEITOS.md](ALGORITMOS_DE_EFEITOS.md) | Alpha automático existente e critérios de edição/refino manual futuros. | `4973f1a50be5` |
| D140 | Entrada da ajuda ao usuário | [README.md](README.md) | Manual do produto com uso, engenharia, evidências e solução de problemas. | `d9a38db10b78` |
| D141 | Bordas e dimensão de visualização | [SOLUCAO_DE_PROBLEMAS.md](SOLUCAO_DE_PROBLEMAS.md) | Proporção, padding e distinção entre geometria visual e FOV real. | `ba6c836875f2` |
| D142 | Como pedir ajuda | [SOLUCAO_DE_PROBLEMAS.md](SOLUCAO_DE_PROBLEMAS.md) | Versão, ambiente, reprodução e logs mínimos sem informações pessoais. | `aa8cdb7b1343` |
| D143 | Índice de diagnóstico | [SOLUCAO_DE_PROBLEMAS.md](SOLUCAO_DE_PROBLEMAS.md) | Organização por sintoma, correção e contexto. | `5fc27427d829` |
| D144 | Falha na abertura | [SOLUCAO_DE_PROBLEMAS.md](SOLUCAO_DE_PROBLEMAS.md) | Separar janela, sessão gráfica e controlador, preservando arquivos. | `d0938a1a3fb5` |
| D145 | Falhas de composição do desktop | [SOLUCAO_DE_PROBLEMAS.md](SOLUCAO_DE_PROBLEMAS.md) | Diagnóstico Linux de janela preta sem importar correções de outros sistemas. | `7cc113bc7d65` |
| D146 | Deslocamento de texto e fontes | [SOLUCAO_DE_PROBLEMAS.md](SOLUCAO_DE_PROBLEMAS.md) | Escala, tamanho, texto longo e captura isolada do card afetado. | `e3dbcd76a176` |
| D147 | Durabilidade de salvamento | [FORMATOS_E_COMPATIBILIDADE.md](FORMATOS_E_COMPATIBILIDADE.md) | Rename não comprova fsync, ACLs ou recuperação; contrato futuro explícito. | `0f0de8ee0361` |
| D148 | XCF e semântica de composição | [FORMATOS_E_COMPATIBILIDADE.md](FORMATOS_E_COMPATIBILIDADE.md) | Luz linear, alpha, clipping e limites em importação futura. | `ca7afe6d73ee` |
| D149 | Reuso de writer e compressão | [DESEMPENHO.md](DESEMPENHO.md) | Buffers e compressor reutilizáveis exigem prova no ciclo completo, sem formato comprimido atual. | `428585ce496d` |
| D150 | Exemplo mínimo de plug-in | [EXTENSOES.md](EXTENSOES.md) | Proposta de filtro de inversão para validar ABI; nenhum wasm instalável presente. | `ce43c84e6aae` |
| D151 | Apresentação de instalador macOS | [DISTRIBUICAO.md](DISTRIBUICAO.md) | Artefatos próprios, rótulos legíveis e regras por plataforma; sem DMG no Linux. | `4176ebecc162` |
| D152 | Hospedagem web, assets e fontes | [EXTENSOES.md](EXTENSOES.md) | Critérios futuros de MIME, cache, 404, HTTPS e iframe; não há site de execução atual. | `aaea849c1099` |
