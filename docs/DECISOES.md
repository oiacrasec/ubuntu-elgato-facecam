# Decisões de implementação

Esta página registra por que os caminhos atuais foram escolhidos. Propostas
futuras permanecem separadas dos mecanismos existentes.

## Aplicação nativa e controlador único

Rust com egui/eframe atende o frontend; ksni atende a bandeja. FFmpeg e V4L2
continuam como serviços locais de vídeo. Essa composição preserva uma compilação
headless pequena e evita duplicar execução de câmera em cada interface.
O núcleo é um conjunto de módulos, não um workspace artificial de dezenas de crates.

## Modos enumerados e controles observados

O formato da câmera é negociado a partir da enumeração real, porque um padrão
UYVY fixo não correspondia ao hardware USB2 observado. Metadados válidos são
necessários para escrever um controle, mas uma recusa real do driver prevalece
até uma atualização explícita. Ler de volta impede mostrar sucesso com um valor
que não foi aplicado.

## Efeitos e preview compartilhados

Efeitos e conversão yuv420p ocorrem antes do split. O preview mostra os mesmos
quadros, na mesma resolução e taxa; apenas o tamanho de exibição acompanha a
janela. Mirror afeta ambas as saídas, e Grid permanece uma guia local opcional.
O slot único prioriza imagem recente sem uma fila crescente. Uma futura fila ou buffer deve
justificar seu custo e sua política de descarte.

## IA opcional por processo

O caminho atual usa modelo ONNX e provider CPU ou CUDA explícitos. Bibliotecas
pesadas ficam fora do binário e só são carregadas no worker. A separação trata
falhas nativas sem transformar o worker em sandbox. CUDA recusada gera erro;
CPU só executa quando selecionada.

Arquivos de IA vazios são resolvidos por pistas e diretórios conhecidos. A busca
não altera o provider nem substitui caminho explicitamente configurado. O
preparador registra diretórios personalizados, para que o comando normal de
abertura consiga encontrá-los. Presença em disco e inferência validada são estados
diferentes. Não há download, inferência ou salvamento automático só por abrir a UI.

## Compatibilidade RTX por runtime

A investigação organiza referências por plataforma, SDK, versão e família GPU.
VFX 0.7.6 e AR 0.8.2 Ada usam gerações diferentes de CUDA e TensorRT; não há
uma única versão NVIDIA que possa representar ambas. O
[catálogo documental](nvidia-runtime-catalog.json) conserva somente descritores
de referência Windows, sem disponibilizá-los ao carregador Linux.

O backend compartilha código e separa componentes quando a API
ou o modelo exigir. A série RTX não basta para selecionar engines: plataforma,
compute capability, versões, modelo e opções de construção também importam.
[Análise RTX](ANALISE_RTX.md) registra evidências e a implementação Ada.

O caminho adotado usa engenharia reversa dos pacotes Windows e implementação
própria em Rust. A obtenção de SDKs NVIDIA Linux fica fora desta investigação;
não é um bloqueio a ser resolvido por cadastro ou instalação de outro produto.

A inferência nativa validada na RTX 4070 SUPER permitiu implementar preparação
guiada no usuário. Ela fixa o modelo m1, ONNX Runtime 1.20.1 e TensorRT 10.4.0.26,
sem instalar SDK VFX Linux. O fallback de inicialização usa ONNX com o provider
escolhido e aviso explícito; uma falha não deve modificar a configuração confirmada.

## Configuração e substituição de arquivos

Defaults e extensões na raiz mantêm compatibilidade com configurações anteriores.
Backup inicial e rename reduzem risco de sobrescrever um arquivo válido durante
uma falha normal. Ainda faltam garantias mais fortes de durabilidade, proteção
contra races e preservação de todos os metadados; elas não são presumidas por
usar um arquivo temporário.

Uma revisão futura de writer deve decidir create-new versus replace, permissões,
links, ACLs, xattrs, barreiras de publicação e recuperação. Identidade de um preset
ou recurso substituído deve ser explícita; igualdade de nome não é identidade
suficiente para reusar caches ou sobrescrever dados.

## Como registrar uma nova decisão

Indicar data/revisão, problema, contexto, alternativas examinadas, escolha,
consequências e forma de validação. Manter hipóteses descartadas quando explicam
um tradeoff relevante. Não criar uma descrição de arquitetura futura como se
fosse prova de que o código já a executa.
