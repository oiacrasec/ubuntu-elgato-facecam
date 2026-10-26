# Desenvolvimento

Compile com Cargo e mantenha o controlador utilizável sem interface gráfica.
A fronteira entre apresentação e execução está descrita em [Arquitetura](ARQUITETURA.md).

## Preparação e ciclo de trabalho

Use Rust 1.95 ou posterior e as dependências Linux listadas no [README](../README.md).
`cargo run --locked` abre a janela; `cargo run --locked -- --headless` abre somente
o controlador. `--config /caminho/config.json` seleciona um arquivo específico.
A ausência de uma câmera não impede os testes comuns nem a leitura do estado.

Durante uma correção, execute o teste pertinente. Ao concluir, execute
`./test-desktop-app.sh`. Uma alteração no fluxo de hardware exige um teste
optativo correspondente quando o dispositivo está disponível. [Testes](TESTES.md)
descreve os comandos reais; o projeto não possui `cargo xtask`, servidor web ou
uma suíte de formatos de um editor de imagens.

Use um diretório de runtime temporário para uma instância de teste. Faça isso no
ambiente do processo filho, sem alterar a sessão inteira. Não encerre um
controlador de uso diário para liberar o socket de um teste. Compilações
simultâneas podem usar `--target-dir`, observando o custo em disco e a posse dos
arquivos alterados.

## Camadas e direção das dependências

| Camada | Módulos atuais | Contrato |
| --- | --- | --- |
| Dados e limites | `config`, `camera`, `effects`, `ai_paths` | Tipos, validação e descoberta delimitada |
| Execução | `stream`, `ai` | Processos e frames, sem dependência da janela |
| Aplicação | `engine` | Comandos, transições, persistência e snapshot |
| Adaptadores | `ipc`, `main`, `gui`, `tray`, `desktop` | Transporte, interação e integração de sessão |

A organização usa módulos em um único crate. Ela não declara dezenas de crates
nem um verificador automático de camadas. Uma futura separação em crates deve
preservar os mesmos contratos e só ocorrer quando trouxer isolamento mensurável.

## Estado, operações e trabalhos demorados

O controlador publica um snapshot clonado; a GUI não modifica diretamente a
captura. Campos editados na janela são rascunhos até o envio do comando. Grade
e visibilidade são estado de apresentação. Espelhamento e efeitos que alteram o
vídeo passam pelo controlador e são compartilhados pelas saídas.

O fluxo atual possui rollback de falhas síncronas ao aplicar efeitos. Não existe
histórico geral de undo/redo, gravação de ações, rebase de jobs ou autosave de
vídeo. Se forem introduzidos, cada operação precisa declarar seus efeitos,
alvo e condições de cancelamento. Um resultado calculado a partir de uma revisão
antiga não pode sobrescrever silenciosamente uma configuração nova.

A thread gráfica não deve aguardar captura ou inferência. A descoberta de
arquivos de IA faz consultas locais limitadas; ela não carrega o runtime. A
verificação nativa de IA ocorre em processo separado. O supervisor mantém a posse
dos filhos; instanciar uma biblioteca externa não cria uma fronteira de sandbox.

## Gráficos, logs e perda do dispositivo

O frontend usa egui/eframe com Glow/OpenGL e suporte compilado a X11 e Wayland.
O backend de vídeo é FFmpeg. Falhas de abertura, captura ou runtime devem chegar
ao estado e ao log; não anuncie reconexão contínua ou recuperação de contexto GPU
que não foi implementada. Teste Wayland em uma sessão real antes de declará-lo
validado.

Logs de vídeo ficam ao lado da configuração. O descarte automático do log acima
de 2 MiB ocorre no início da captura; não é um serviço geral de rotação. Use
`--debug` para ver os caminhos de configuração e runtime. Não inclua frames,
segredos ou dados pessoais em logs de suporte.

## Variáveis e descoberta de IA

`XDG_CONFIG_HOME` e `XDG_DATA_HOME` seguem os diretórios do usuário. O socket usa
`XDG_RUNTIME_DIR`, com fallback temporário por UID. `FACECAM_AI_MODEL` e
`FACECAM_AI_RUNTIME` são pistas para campos ainda vazios; um caminho configurado
explicitamente permanece preservado. [Efeitos](EFEITOS.md) define a busca completa.

A configuração lida na inicialização incorpora arquivos disponíveis em memória.
A descoberta não escreve `config.json`, não liga o desfoque e não baixa dependências.
Uma descoberta bem-sucedida comprova a presença dos arquivos, não sua compatibilidade
nativa; `--check-ai` e o aquecimento verificam o provider e o modelo ao executá-los.
