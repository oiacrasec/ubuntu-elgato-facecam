# Orientações do projeto

Aplicação nativa Rust para Ubuntu 24.04+. Leia [a arquitetura](docs/ARQUITETURA.md)
antes de alterar o fluxo de comandos, captura ou efeitos.
O [manual completo](docs/README.md) organiza uso, desenvolvimento, segurança,
UI, formatos, desempenho, distribuição e o inventário de cobertura documental.
Para documentação, mensagens e rótulos, siga os
[padrões de documentação e escrita](docs/PADROES_DE_DOCUMENTACAO.md).
Não adicionar Python, Qt, GTK, Electron ou webview ao produto.
O alvo de paridade funcional e visual é Camera Hub Windows 2.3.0.7295.
O usuário confirmou todos os recursos de IA no Windows com sua RTX 4070 SUPER;
isso é a referência de aceitação, sem presumir execução dos SDKs Windows no Linux.

- `camera.rs`: descoberta V4L2, modos, metadados, validação e escrita de controles.
- `config.rs`: configuração JSON, compatibilidade de campos e backup.
- `effects.rs`: parâmetros de efeitos, limites e geração dos filtros.
- `ai.rs`: inferência ONNX em processo isolado, CPU ou NVIDIA CUDA explícitos.
- `ai_paths.rs`: descoberta local de modelo/runtime, preservando caminhos manuais.
- `background.rs`: cenários locais e decodificação limitada de fundos próprios, uma vez por worker.
- `video_background.rs`: fundo animado com decoder FFmpeg em loop, FPS de processamento e morte junto do worker.
- `dependencies.rs`: estado dpkg, planos APT de instalação/remoção, confirmação explícita, autenticação Ubuntu e execução em segundo plano.
- `resources.rs`: biblioteca, descoberta de Looks/fundos, importação validada e LUTs próprias; GUI exibe nomes, sem paths editáveis de recursos ou IA.
- `stream.rs`: processos pertencentes ao aplicativo e último frame do preview.
- `engine.rs`: comandos, transições e estado; uma implementação para GUI e CLI.
- `ipc.rs`: instância única, socket Unix e thread do controlador.
- `gui.rs`: egui com layout Camera Hub; `tray.rs`: StatusNotifierItem D-Bus.
- `desktop.rs`: autostart opcional. `tests/controller.rs`: contratos públicos.

Validação: `./test-desktop-app.sh`. Build: `cargo build --release --locked`.
`--no-default-features` deve continuar funcional sem GUI, bandeja ou runtime de IA.
O recurso Cargo `ai` carrega as bibliotecas externas somente ao ativar IA.
Mantenha `unsafe_code = "forbid"` no código da aplicação.

A interface mantém controles à esquerda e preview à direita. Efeitos de vídeo
são aplicados antes da divisão entre preview e saída virtual. Grade e espelho
local pertencem à visualização; o espelho da aba Effects afeta ambas as saídas.
Não apresentar filtros convencionais como IA. Nunca ocultar falha de CUDA
usando CPU automaticamente. Verificar uma inferência antes de trocar a captura.
Com IA, limitar FPS explicitamente; preview sem saída virtual deve ser reduzido
antes da inferência. Não carregar ou redimensionar imagens estáticas por frame.

Testar valores negativos, zero, positivos, extremos, passos, menus esparsos,
valores não finitos e mensagens de falha. Não escrever na câmera em testes comuns.
[TESTES.md](docs/TESTES.md) separa verificações automáticas das que exigem hardware.

`package/` contém os instaladores macOS/Windows, ferramentas de análise e evidências
locais, ignorados pelo Git. [Referência Windows](docs/REFERENCIA_WINDOWS.md) registra
hashes, chamadas investigadas e limites da comparação; não confundir distribuição
de arquivos ou strings de menu com execução e suporte no dispositivo.
Não alterar firmware nem descarregar módulos automaticamente. Respeitar controles
inativos e recusas observadas; não inventar conversões de ganho para ISO.
Não copiar recursos proprietários do Camera Hub para a distribuição.
Dependências de sistema usam APT. Mostrar comandos e mudanças de repositório
antes da confirmação; não iniciar instalação ao abrir o app. Não usar instaladores
`.deb` locais. Não armazenar senhas ou encerrar uma transação APT em andamento.
