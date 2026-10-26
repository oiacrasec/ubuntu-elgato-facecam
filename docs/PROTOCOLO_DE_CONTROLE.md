# CLI e protocolo de controle

A CLI envia pedidos ao controlador em execução por um socket Unix local.
GUI, bandeja e CLI compartilham `Request`, `Reply` e `Snapshot` definidos no núcleo.

## Transporte e instância

O endereço é `$XDG_RUNTIME_DIR/elgato-virtualcam-UID/control.sock`, ou o diretório
temporário correspondente quando a variável não está disponível. Uma trava
impede instâncias concorrentes no mesmo runtime. O diretório é verificado por
UID, não pode ser um symlink e recebe modo 0700.

Cada conexão carrega um objeto JSON terminado por newline e recebe uma resposta
JSON também por linha. O cliente atual tem timeout de leitura de 10 segundos.
O servidor limita a leitura de pedidos a 64 KiB; o cliente limita respostas a
256 KiB. O loop atende até oito conexões por rodada e verifica o controlador a
cada 100 ms. Não há listener TCP, endpoint HTTP ou autenticação por bearer token.
A proteção atual depende do usuário e das permissões locais.

## Comandos atuais

| `command` | Campos | Efeito |
| --- | --- | --- |
| `status` | nenhum | Retorna o snapshot atual |
| `refresh` | nenhum | Atualiza câmeras, modos, controles e pistas de IA; libera nova tentativa de controles recusados |
| `start` | `preview`, bool, padrão false | Inicia a saída virtual, com preview opcional |
| `preview` | nenhum | Inicia somente a visualização |
| `stop` | nenhum | Encerra captura e filhos de vídeo |
| `configure` | `config`, `save`, bool, padrão false | Valida formato/configuração com captura parada; salvamento opcional |
| `update_settings` | `virtual_device`, string opcional; `start_minimized`, bool opcional; `language`, tag opcional | Aplica e salva apenas os campos enviados; troca de saída exige captura parada; idioma pode mudar durante captura |
| `set_control` | `name`, string; `value`, i64 | Valida e escreve um controle V4L2, com leitura de retorno |
| `set_effects` | `effects`, objeto | Detecta campos de IA vazios, valida, aplica e salva efeitos |
| `set_autostart` | `enabled`, bool | Ativa ou remove a inicialização automática do usuário imediatamente |
| `prepare_install` | `dependency`: `video_tools`, `sdk_manager`, `nvidia_tools` ou `virtual_camera` | Prepara instalação ou câmera virtual persistente para revisão; não executa comandos administrativos |
| `prepare_virtual_camera` | `persistent`, bool obrigatório | Prepara módulo e persistência opcional, com captura parada |
| `prepare_nvidia` | nenhum | Prepara componentes Ada no usuário e testa a GPU em segundo plano; exige captura parada |
| `prepare_uninstall` | `dependency`: `video_tools`, `sdk_manager` ou `nvidia_tools` | Consulta pacotes e simula remoção restrita antes de apresentar o plano; não desinstala |
| `confirm_install` | `id`, u64 | Confirma o plano atual e inicia instalação com autenticação do Ubuntu |
| `cancel_install` | nenhum | Descarta o plano pendente; não interrompe uma transação APT iniciada |
| `import_resource` | `kind`: `lut`, `image` ou `video`; `path`: arquivo local | Inicia validação e cópia para a biblioteca em segundo plano; não aplica efeitos |
| `remove_image` | `path`: imagem da biblioteca gravável | Remove somente a cópia importada; se estiver aplicada, troca para desfoque e salva antes de excluir |
| `save` | nenhum | Salva configuração e controles graváveis atuais |
| `restore` | nenhum | Restaura controles locais disponíveis |
| `show` | nenhum | Solicita visibilidade da janela |
| `quit` | nenhum | Encerra captura e controlador |

Exemplos de linhas de protocolo:

```json
{"command":"set_control","name":"pan_absolute","value":-3600}
{"command":"set_effects","effects":{"brightness":-0.1,"saturation":0.0}}
```

`Reply` contém `ok`, `message` e `state`. O estado inclui configuração, dispositivos,
modos e controles, disponibilidade, erro, flags de captura, contagem do preview,
revisão de exibição e suporte compilado a IA. Frames não são serializados no JSON.
`ai_backend` informa o backend efetivamente validado (`onnx` ou `nvidia`) e um
`warning` opcional de fallback. `nvidia_prepared` indica presença dos componentes,
sem substituir uma inferência. `preparing_nvidia`, `nvidia_progress` e `nvidia_log`
acompanham a preparação. `--prepare-nvidia` inicia o mesmo pedido pelo controlador
ativo; aguarde `preparing_nvidia: false` e confira `error` e o resultado. Quit e
captura são recusados durante a preparação; falhas não substituem o bundle anterior.
Sem as ferramentas de extração/download, `prepare_nvidia` retorna um plano APT
`nvidia_tools` antes de começar. Confirme-o e repita o pedido após a conclusão.
`autostart_enabled` informa a presença de um cadastro ativo, ou null se a consulta
falhou. A abertura, atualização, consulta periódica e conclusão de `set_autostart`
releem o arquivo. `--install-autostart` e `--remove-autostart` usam esse comando
quando há controlador e operam diretamente pelo mesmo módulo quando não há instância.
`virtual_cameras` lista somente saídas confirmadas como v4l2loopback. Sem captura,
atualizar preserva a saída disponível ou seleciona a primeira descoberta, sem
escrever a configuração nem carregar módulos.
`virtual_camera_status` publica kernel, presença e carga do módulo, versão,
assinante e estado do Secure Boot, quando legível. Ter módulo no disco não
significa ter saída carregada ou vídeo em execução.
`installation_plan`, `installation_running` e `installation_log` descrevem a
instalação de dependências. O ID não é uma credencial: a fronteira continua
sendo o socket do usuário e a autorização administrativa por PolicyKit.

`dependencies` lista cada grupo e seus pacotes, com `version` ou null quando não
instalado e `status` com o estado dpkg ou null quando ausente. `dependency_error` registra falha de consulta; a GUI desabilita ações
sem estado confiável. Um plano inclui `action` (`install`/`remove`), `dependency`
e `removals` (pacotes que a simulação removeria). O protocolo mantém os nomes
`confirm_install`/`cancel_install` para os dois tipos de plano, com o mesmo ID.

`resources` contém tipo, nome, caminho interno e `removable` dos recursos descobertos.
Somente imagens regulares da biblioteca desta configuração são removíveis.
`--remove-image ARQUIVO` envia `remove_image` pelo mesmo controlador. Caminhos
externos, compartilhados e symlinks são recusados; o original é preservado.
`importing`, `imported` e `import_revision` publicam progresso e última importação
concluída. A revisão só aumenta no sucesso. Falhas aparecem em `state.error`, sem
alterar a seleção salva. Caminhos existem no protocolo para automação; a GUI
mostra nomes e diálogo de arquivos.

`ok` expressa o resultado do pedido. Um `status` pode retornar `ok: true` com um
`state.error` de uma falha de captura anterior. Iniciar um processo também não
comprova entrega posterior de frames; consulte o estado e o log após a inicialização.

`languages` lista as tags e os nomes nativos dos catálogos disponíveis.
`update_settings.language` valida o catálogo antes de salvar e ativar. Um arquivo
inválido ou falha ao salvar preserva a preferência anterior. `refresh` atualiza
a lista. IDs e valores do protocolo permanecem estáveis; somente mensagens
humanas da CLI são traduzidas. Use `--config` para localizar catálogos pessoais
ao lado de uma configuração específica. [Localização](LOCALIZACAO.md) detalha
a descoberta e `FACECAM_LANGUAGE`.

## CLI e automação

`--help` lista as opções disponíveis. `--start` pode criar um controlador em
segundo plano; `--headless` o mantém em primeiro plano. `--status`, `--stop`,
`--quit` e `--set-effects ARQUIVO` tratam a instância existente. O `--config` do
cliente não muda o arquivo já escolhido pelo controlador em execução.

`--set-control NOME=VALOR` encaminha a alteração por IPC quando há instância;
sem instância, consulta e escreve diretamente pelo mesmo validador V4L2.
`--list-controls` enumera metadados. `--check-ai` carrega a configuração escolhida,
resolve arquivos e executa uma inferência no provider selecionado.

`--prepare-install video-tools` ou `--prepare-install sdk-manager` retorna os
comandos e o ID do plano. Revise esse resultado antes de enviar
`--confirm-install ID`. `--cancel-install` invalida o ID pendente. Preparar e
cancelar não escrevem a configuração, não cadastram repositórios e não executam
APT mutante. Um plano substituído ou cancelado não pode ser confirmado.
A confirmação inicia o trabalho em segundo plano; consulte `--status` e o log
até `installation_running` voltar a false. Falha ou recusa da autenticação
interrompe a sequência, sem anunciar sucesso. `--quit` é recusado durante o APT.

`--prepare-uninstall video-tools` ou `--prepare-uninstall sdk-manager` prepara a
remoção. Revise comandos e pacotes; `--confirm-install ID` confirma esse plano e
`--cancel-install` o descarta. Preparar ou cancelar nunca remove pacotes.
Ferramentas de vídeo só podem ser removidas com captura parada. Após a conclusão,
o estado do dpkg é verificado antes de anunciar sucesso; alterações externas
também são refletidas na consulta periódica, sem reabrir a janela.

`--prepare-virtual-camera` prepara a saída persistente;
`--prepare-virtual-camera --virtual-camera-session-only` limita a esta sessão.
Revise `installation_plan`, incluindo o payload tipado `virtual_camera`, antes
de `--confirm-install ID`. `--cancel-install` descarta o plano. O helper executa
apenas as etapas fixas geradas desse payload e usa uma chamada a pkexec para a
preparação inteira. Não recebe comandos ou destinos arbitrários. A saída é
redescoberta ao concluir; iniciar captura durante a preparação é recusado.
Não existe desinstalação assistida de módulos do kernel.

O CLI não possui comandos de batch, geração de man pages, exportação de LUT,
completions instaladas ou servidor MCP. Uma integração futura deve adaptar os
mesmos comandos, com schema, timeout, cancelamento e permissões próprios.
Não repita automaticamente pedidos mutantes após perda de conexão: a alteração
pode ter ocorrido antes de a resposta chegar.

`--import-resource lut --import-file /caminho/look.cube` importa para a biblioteca
do controlador ativo. Use `image` ou `video` para fundos. A resposta confirma o
início; aguarde `importing: false` e confira `error` e `import_revision`. Depois
use o caminho de `imported` em `--set-effects` se quiser aplicar pela CLI.
O controlador recusa Quit enquanto copia. Consulte [Efeitos](EFEITOS.md) para
formatos, limites, descoberta e tratamento de nomes repetidos.

## Erros e limites futuros

Uma resposta recusada precisa conservar informação suficiente para diagnóstico.
Falhas de validação não devem marcar um controle como recusado pelo hardware.
`SetEffects` tenta restaurar a captura anterior em falhas síncronas; não há
transação distribuída, batch atômico ou identificador de idempotência.

Autorização por cliente, diretórios de leitura/escrita por capability, rate limits,
cancelamento de operações longas e auditoria estruturada são propostas, não
proteções existentes. [Segurança](SEGURANCA.md) define as fronteiras atuais.
