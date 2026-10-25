# Extensões, automação e portabilidade

A extensão atual do produto ocorre pelo código Rust e pelos comandos do
controlador. Não há carregador de plug-ins ou servidor MCP implementado.

## Contrato para novos adaptadores

Um adaptador futuro deve traduzir parâmetros para `Request`, conservar erros e
consultar o mesmo snapshot. Um modo headless pode possuir sua própria sessão;
um bridge deve encaminhar pedidos à instância existente. Não criar dois
supervisores para a mesma câmera por introduzir outro transporte.

Um catálogo de ferramentas precisa declarar nome estável, schema, disponibilidade,
efeitos e resultado. Progresso e cancelamento precisam de IDs e ciclo de vida.
Uma ferramenta de inspeção não deve ganhar automaticamente autoridade para salvar
arquivos, alterar câmera ou encerrar o aplicativo. [Protocolo](PROTOCOLO_DE_CONTROLE.md)
e [Segurança](SEGURANCA.md) descrevem os limites atuais e os critérios de evolução.

MCP, JSON-RPC remoto e stdio persistente são propostas. A referência atual é uma
linha JSON por conexão local. Não há instalação de tools, resources, prompts ou
completions MCP para anunciar. Um bridge não pode reenviar automaticamente uma
mutação quando perdeu a resposta; apenas leituras explicitamente idempotentes
podem ter uma política limitada de retry.

## Ações, batch e nomes de saída — proposto

Um gravador de ações deve registrar comandos e parâmetros independentes do texto
da UI. Variáveis de alvo e caminhos precisam de política explícita. Um lote pode
ter êxitos parciais; uma interrupção por orçamento ou erro deve informar quais
passos já produziram efeitos e não prometer rollback inexistente.

Exportação em lote, recursão por diretórios, processamento de documentos abertos,
progresso, cancelamento e arquivo de erros não existem hoje. Para futuros arquivos
de saída, definir template, extensão, índice, separador e política de colisão.
Nunca transformar uma entrada em outro formato sobrescrevendo o original por
um nome inferido silenciosamente.

## Plug-ins isolados — proposta de arquitetura

Se extensões de terceiros forem necessárias, priorizar um formato com contrato
limitado de pixels e parâmetros, independente da interface. Um módulo WebAssembly
pode ser uma opção de estudo; não é uma dependência atual.

O contrato deve versionar ABI e manifesto e definir identificador, nome, versão,
autor, tipo, parâmetros, unidades e contexto de imagem. Exportações de memória,
alocação e processamento precisam de limites e validação de offsets. Não aceitar
ponteiros ou comprimentos sem conferir a memória linear.

Uma proposta inicial pode proibir imports de filesystem, rede, relógio e
aleatoriedade, limitar tamanho de módulo, memória, instruções, stack, duração e
concorrência. Valores precisam ser calibrados para o orçamento de vídeo, não
copiados de processamento offline. Um plugin que exceda o tempo por frame deve
produzir um estado de falha definido, sem travar o controlador.

Declarar canais, layout, profundidade, espaço de cor, alpha e origem da região.
Parâmetros medidos em pixels precisam acompanhar redução de preview; valores
adimensionais não devem ser divididos apenas por terem nome parecido. Filtros
com vizinhança precisam de halo e política de borda para evitar costuras entre
bandas. Saída inválida deve ser recusada antes de substituir estado confirmado.

Instâncias frescas por bloco evitam estado compartilhado implícito; uma extensão
com estado temporal exige outro contrato. Um exemplo futuro de inversão de cor
seria útil para validar identidade, canais e limites, mas não há `.wasm` de exemplo
instalável no Facecam Hub. Plug-ins nativos de outros aplicativos, painéis UXP/CEP
e formatos `.8BF` permanecem fora do escopo.

## Web, outras plataformas e hospedagem

A aplicação atual depende de V4L2, socket Unix e integração Linux. Não existe
build web, Windows, macOS ou FreeBSD validado. Portar a UI por si só não porta
captura, câmera virtual, runtime, permissões ou bandeja.

Um estudo de WebAssembly precisaria substituir serviços de dispositivo e I/O,
respeitar permissões do navegador e separar controle de filesystem. Uma interface
embutida em iframe exigiria política de origem, clipboard e armazenamento. Sem
implementação, não recomendar Docker, headers ou flags de renderer como caminhos
operacionais de um site inexistente.

Para uma distribuição web futura, os critérios são assets versionados, caminhos
relativos, MIME correto, erros 404 reais para arquivos ausentes, invalidação de
cache coerente entre loader e código, HTTPS e limites de download. Fontes sob
demanda precisam de manifesto e tratamento de falha; mudar catálogo não fornece
glifos. Uma página estática de erro de startup deve distinguir falha de rede de
falha do engine. Nenhum desses mecanismos está incluído no runtime Linux atual.
