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
Para essa referência, use os redistribuíveis NVIDIA Broadcast Video Effects 0.7.6
e AR 0.8.2 para RTX 40/Ada indicados pela Elgato. [Runtimes NVIDIA](docs/NVIDIA_MAXINE.md)
registra os arquivos e distingue análise Windows de implementação nativa Linux.
O caminho escolhido é engenharia reversa dos pacotes Windows e implementação
própria em Rust. Não retomar a busca ou instalação de SDKs NVIDIA Linux como
pré-requisito. [Análise RTX](docs/ANALISE_RTX.md) registra essa decisão e as evidências.
A pesquisa deve fechar os contratos necessários ao recurso e então dar lugar à
implementação. Priorizar segmentação, desfoque e troca de fundo no worker Rust
existente; reconstruir todo o SDK não é pré-requisito.

- `camera.rs`: descoberta V4L2, modos, metadados, validação e escrita de controles.
- `config.rs`: configuração JSON, compatibilidade de campos e backup.
- `effects.rs`: parâmetros de efeitos, limites e geração dos filtros.
- `ai.rs`: worker isolado, seleção NVIDIA/ONNX, composição e provider explícito.
- `gpu.rs`: grafos ONNX próprios para resize, desfoque e composição CUDA, com I/O binding.
- `nvidia.rs`: segmentação temporal Ada por TensorRT, com estados na GPU.
- `nvidia_model.rs`: importação local do modelo exato e descritor EPContext próprio.
- `nvidia_setup.rs`: preparação no usuário, hashes e inferência antes da publicação.
- `nvidia_ar_model.rs` e `nvidia_ar.rs`: quatro contratos AR Ada fixados, importação local e diagnóstico isolado.
- `nvidia_ar_setup.rs`: preparo AR persistente pelo controlador, runtime VFX reutilizado, licenças e publicação após inferência.
- `framing.rs` e `nvidia_framing.rs`: detecção interna de rosto e upload GPU compartilhado com a correção de olhar.
- `gaze.rs` e `nvidia_gaze.rs`: landmarks, alinhamento dos olhos, encoder/decoder e composição local; preservar enquadramento, piscadas e imagem fora dos olhos.
- `ai_paths.rs`: descoberta local de modelo/runtime, preservando caminhos manuais.
- `background.rs`: cenários locais e decodificação limitada de fundos próprios, uma vez por worker.
- `video_background.rs`: fundo animado com decoder FFmpeg em loop, FPS de processamento e morte junto do worker.
- `dependencies.rs`: estado dpkg, planos APT de instalação/remoção, confirmação explícita, autenticação Ubuntu e execução em segundo plano.
- `virtual_camera.rs`: descoberta do módulo, plano de preparação e helper administrativo de etapas fixas com autorização única.
- `resources.rs`: biblioteca, descoberta de Looks/fundos, importação, remoção restrita de cópias de imagens e LUTs próprias; GUI exibe nomes, sem paths editáveis de recursos ou IA.
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
são aplicados antes da divisão entre preview e saída virtual. A grade
é uma guia local; Mirror e o espelho da aba Effects afetam ambas as saídas.
Não apresentar filtros convencionais como IA. Nunca ocultar falha de CUDA
usando CPU automaticamente. Verificar uma inferência antes de trocar a captura.
Com IA, limitar FPS explicitamente e igualmente para preview e saída virtual.
Preservar resolução, proporção e conversão YUV comuns antes do split. Não carregar
ou redimensionar imagens estáticas por frame.

Testar valores negativos, zero, positivos, extremos, passos, menus esparsos,
valores não finitos e mensagens de falha. Não escrever na câmera em testes comuns.
[TESTES.md](docs/TESTES.md) separa verificações automáticas das que exigem hardware.

`package/` contém o instalador Windows, ferramentas de análise e evidências
locais, ignorados pelo Git. [Referência Windows](docs/REFERENCIA_WINDOWS.md) registra
hashes, chamadas investigadas e limites da comparação; não confundir distribuição
de arquivos ou strings de menu com execução e suporte no dispositivo.
Não alterar firmware nem descarregar módulos automaticamente. Respeitar controles
inativos e recusas observadas; não inventar conversões de ganho para ISO.
Não copiar recursos proprietários do Camera Hub para a distribuição.
Dependências de sistema usam APT. Mostrar comandos e mudanças de repositório
antes da confirmação; não iniciar instalação ao abrir o app. Não usar instaladores
`.deb` locais. Não armazenar senhas ou encerrar uma transação APT em andamento.
