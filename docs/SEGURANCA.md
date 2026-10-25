# Segurança e fronteiras de confiança

O Facecam Hub é um aplicativo local com a autoridade da conta que o executa.
Trate arquivos de configuração, LUTs, modelos e bibliotecas nativas como entradas
que atravessam fronteiras distintas.

## Ativos e pontos de entrada

Os ativos relevantes são configurações e backups, frames da câmera, acesso aos
dispositivos, arquivos do usuário, disponibilidade do desktop e integridade dos
binários. Entradas incluem CLI, JSON de IPC, arquivos locais, metadados V4L2,
FFmpeg, drivers e componentes ONNX/CUDA externos.

| Fronteira | Responsável | Proteção atual ou limite |
| --- | --- | --- |
| Adaptador → núcleo | `engine`, validadores | Tipos, intervalos, passos, menus e estados |
| Cliente → controlador | `ipc` | Socket Unix, diretório 0700, UID e limites de leitura |
| Captura → GUI | `stream` | Último frame completo, tamanho de preview fixo |
| Aplicação → FFmpeg/V4L2 | `stream`, `camera` | Argumentos de processo; execução sem shell |
| Configuração → disco | `config` | Erro preserva JSON inválido; backup e arquivo temporário |
| Inferência → runtime nativo | worker `ai` | Processo separado e validação do contrato do modelo |
| Descoberta → arquivos locais | `ai_paths` | Diretórios conhecidos e leituras delimitadas, sem download |
| Fundo próprio → decoder | worker `background` | PNG/JPEG/WebP/BMP, arquivo até 32 MiB, 8192×8192, limite de decodificação 128 MiB |
| Arquivo escolhido → biblioteca | `resources`, `engine` | Importação única em segundo plano, cópia oculta 0600, limites por tipo, validação e publicação sem sobrescrita; catálogo limitado |
| Vídeo próprio → FFmpeg | `video_background` | Arquivo local regular até 4 GiB, demuxers MOV/Matroska/WebM e protocolos file/pipe; saída RGB limitada por dimensão e pipe |
| Plano APT → sistema | `dependencies`, `engine` | Comandos definidos no núcleo, ID atual e confirmação explícita; autenticação por pkexec e log privado |
| Preparação virtual → kernel e `/etc` | `virtual_camera`, `engine` | Uma autorização, payload tipado, pacotes e destinos restritos; sem shell, descarga de módulos ou alteração de Secure Boot |

Um processo separado isola falhas de carregamento, mas não restringe automaticamente
acesso a arquivos, rede ou GPU. `unsafe_code = "forbid"` vale para a aplicação;
não elimina código nativo ou `unsafe` de dependências, drivers e runtimes.

## Cenários e medidas implementadas

Entradas numéricas são validadas antes de escrita. A diferença de controles usa
i128 para evitar overflow em i64. FPS e dimensões têm limites; máscaras de IA
precisam do shape e da faixa esperados. As strings que viram argumentos não são
interpretadas por um shell. Caminhos de LUT recebem escaping para os parsers
específicos do FFmpeg; isso tem teste com caracteres especiais.

O controle local não se expõe à rede. A trava impede duas instâncias no mesmo
runtime. O encerramento usa os handles dos processos pertencentes à captura.
O preview tem um único slot para impedir crescimento de fila em Rust. Esses
mecanismos reduzem classes específicas de erro; não são um limite global de RAM,
CPU, GPU ou tempo de execução.

Configurações malformadas são preservadas. O salvamento usa backup inicial,
arquivo temporário e rename. A descoberta não substitui caminhos explicitamente
configurados, não ativa IA e não carrega bibliotecas. O preparador usa versões
fixadas e confere hashes dos downloads antes de extrair os pacotes.

A instalação de dependências não aceita comandos de shell pelo IPC nem recebe
senhas. Planos cancelados ou substituídos perdem validade. O cadastro NVIDIA usa
uma chave pública com SHA-256 fixado e `Signed-By` embutido na fonte deb822;
a chave não ganha confiança global para outros repositórios. A sintaxe segue
o [contrato do APT](https://manpages.debian.org/trixie/apt/sources.list.5.en.html).
Os passos administrativos começam somente após confirmação. O `.deb` local não
é usado. Uma falha interrompe os próximos passos; alterações administrativas
já concluídas não são revertidas automaticamente. O aplicativo não encerra uma
transação APT para atender Quit.

A remoção consulta uma simulação e recusa pacotes além dos solicitados, sem
autoremove ou purge. A conferência é repetida antes da execução privilegiada.
Isso não constitui uma transação atômica entre simulação e remoção se outro
gerenciador alterar o sistema nesse intervalo. O plano avisa que os pacotes são
compartilhados por outros aplicativos. Ferramentas de vídeo não são removidas
durante captura e não há remoção automática ao encerrar o Facecam Hub.

## Limitações conhecidas

Não há capabilities por cliente ou comando, sandbox de filesystem para o socket,
filtro de syscalls do worker, isolamento de drivers, assinatura verificada de
modelos ou orçamento global de memória por sessão. Um processo da mesma conta
com acesso ao runtime pode enviar comandos e pode ter autoridade equivalente
sobre os arquivos dessa conta.

O hash baixado de um catálogo pelo mesmo canal do pacote detecta corrupção, mas
não constitui uma assinatura independente do fornecedor. Os passos de extração
não devem ser anunciados como um sandbox para arquivos de origem arbitrária.
O projeto não possui atualmente `cargo-audit`, política `cargo-deny`, SBOM
publicada ou testes contínuos de fuzzing dedicados.

O salvamento atual não promete resistência a substituição concorrente por
symlink, preservação de todos os metadados, ACLs ou xattrs, nem durabilidade após
queda de energia. Arquivos temporários com nomes compartilhados entre processos
também exigem análise antes de anunciar garantias de concorrência.

## Desenho de endurecimento proposto

Antes de abrir um transporte remoto, definir autenticação e autorização separadas
para inspeção, escrita de configuração, controle da câmera e encerramento.
Arquivos remotos devem usar handles de diretório com capabilities de leitura e
escrita independentes, resolução relativa e bloqueio de escape por links. Apenas
canonicalizar uma string antes de abrir não elimina a corrida entre as operações.

Limitar bytes, profundidade de JSON, jobs simultâneos, dimensões, alocações e
duração antes do trabalho caro. Definir resposta estável para estouro de orçamento
e para operação possivelmente concluída sem resposta. Um sistema de auditoria
deve registrar comando, decisão, resultado e duração, sem frames, tokens ou
conteúdo privado. Esses itens são critérios para implementação futura.

## Parsers, regressões e corpus

Ao acrescentar um formato, verificar comprimentos, offsets, multiplicações,
decompressão, profundidade, contagem de canais, CRC/hash e tamanho materializado.
Testar entradas vazias, truncadas, fora da faixa, tipos incorretos e caminhos com
links ou componentes de escape quando o leitor puder manipular filesystem.
Uma falha descoberta precisa de uma fixture mínima sintética e de um teste que
retorne erro sem corromper estado. [Testes](TESTES.md) registra os checks existentes.

Corpus externo deve ser obtido de origem identificada, com revisão e hash, e
ficar fora da distribuição. Não usar fotos pessoais, segredos, payloads ativos ou
amostras sem autorização em issues públicas ou testes versionados.

## Relatar uma vulnerabilidade

Registre versão/revisão, ambiente, entrada mínima segura, comportamento observado
e impacto. Reduza e remova dados pessoais antes de compartilhar uma reprodução.
O repositório ainda não define um contato privado de segurança nem prazo de
atendimento; não invente endereço, programa de recompensa ou SLA. Definir um
canal privado é requisito de preparação de uma publicação pública.
