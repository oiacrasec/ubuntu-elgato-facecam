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

## Efeitos compartilhados e preview limitado

Efeitos transformam o vídeo antes do split; o preview é reduzido e descartável.
Grade e espelho de visualização não contaminam a saída virtual. O slot único
prioriza imagem recente sem uma fila crescente. Uma futura fila ou buffer deve
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
