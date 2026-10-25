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
| `set_control` | `name`, string; `value`, i64 | Valida e escreve um controle V4L2, com leitura de retorno |
| `set_effects` | `effects`, objeto | Detecta campos de IA vazios, valida, aplica e salva efeitos |
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

`ok` expressa o resultado do pedido. Um `status` pode retornar `ok: true` com um
`state.error` de uma falha de captura anterior. Iniciar um processo também não
comprova entrega posterior de frames; consulte o estado e o log após a inicialização.

## CLI e automação

`--help` lista as opções disponíveis. `--start` pode criar um controlador em
segundo plano; `--headless` o mantém em primeiro plano. `--status`, `--stop`,
`--quit` e `--set-effects ARQUIVO` tratam a instância existente. O `--config` do
cliente não muda o arquivo já escolhido pelo controlador em execução.

`--set-control NOME=VALOR` encaminha a alteração por IPC quando há instância;
sem instância, consulta e escreve diretamente pelo mesmo validador V4L2.
`--list-controls` enumera metadados. `--check-ai` carrega a configuração escolhida,
resolve arquivos e executa uma inferência no provider selecionado.

O CLI não possui comandos de batch, geração de man pages, exportação de LUT,
completions instaladas ou servidor MCP. Uma integração futura deve adaptar os
mesmos comandos, com schema, timeout, cancelamento e permissões próprios.
Não repita automaticamente pedidos mutantes após perda de conexão: a alteração
pode ter ocorrido antes de a resposta chegar.

## Erros e limites futuros

Uma resposta recusada precisa conservar informação suficiente para diagnóstico.
Falhas de validação não devem marcar um controle como recusado pelo hardware.
`SetEffects` tenta restaurar a captura anterior em falhas síncronas; não há
transação distribuída, batch atômico ou identificador de idempotência.

Autorização por cliente, diretórios de leitura/escrita por capability, rate limits,
cancelamento de operações longas e auditoria estruturada são propostas, não
proteções existentes. [Segurança](SEGURANCA.md) define as fronteiras atuais.
