# Facecam Hub para Ubuntu

Controlador nativo em Rust para Elgato Facecam, com layout do Camera Hub:
controles à esquerda, preview à direita e abas Camera e Effects. Destino:
Ubuntu 24.04+. Preview e câmera virtual compartilham LUT, ajustes de cor,
espelhamento, redução de ruído e desfoque ou substituição de fundo por IA opcional.

## Executar

```bash
sudo apt install build-essential pkg-config libxkbcommon-dev libwayland-dev \
  libx11-dev libgl1-mesa-dev ffmpeg v4l-utils
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --release --locked
./target/release/elgato-virtualcam
```

Use Rust 1.95 ou posterior ([instalação](https://rustup.rs/)). Clique em
**Iniciar preview** para ver a câmera. A captura negocia formato, resolução e FPS
com o dispositivo. A GUI usa egui/OpenGL, e a bandeja usa D-Bus; não é necessário
runtime Python, Qt, GTK ou navegador. No GNOME, a bandeja pode exigir a extensão
AppIndicator. Sem bandeja, a janela permanece acessível.

Em **Configurações**, o aplicativo prepara a instalação de FFmpeg/V4L2 ou NVIDIA
SDK Manager por APT, mostra os comandos e pede confirmação antes de autenticar
pelo Ubuntu. O SDK Manager usa repositório, sem instalador `.deb` local.
Veja o [fluxo de instalação assistida](docs/USO.md#instalar-dependências-com-confirmação).

Para enviar vídeo a OBS, navegador ou videoconferência, abra **Configurações** ›
**Câmera virtual** › **Preparar câmera virtual…**. Revise o plano e clique em
**Confirmar preparação**. O app aproveita o módulo v4l2loopback do kernel ou
propõe seu pacote oficial por APT; uma autorização do Ubuntu executa a preparação.
**Disponível após reiniciar o Ubuntu** controla a persistência.
Veja [preparação e limites](docs/USO.md#preparar-a-câmera-virtual).

Depois clique em **Iniciar câmera virtual** e selecione **VirtualCam** no consumidor.
Abrir ou atualizar o app não carrega módulos. O aplicativo nunca descarrega o
módulo para substituir dispositivos em uso. Preview e ajustes funcionam sem
a saída virtual, que usa yuv420p.

## Efeitos e IA NVIDIA

Na aba **Efeitos**, ajuste cor, redução de ruído e LUT `.cube`, ou configure
**Fundo com IA**. **Aplicar e salvar** reinicia brevemente a captura e aplica
os efeitos às duas saídas. **Espelhar** espelha ambas; **Grade** é uma guia local.

A IA segmenta a pessoa e desfoca ou substitui o fundo por cor, cenário, imagem
ou vídeo próprio. Para RTX 40/Ada, use **Configurações** › **Efeitos NVIDIA RTX** ›
**Preparar IA RTX** com captura parada. O app prepara componentes no usuário
e testa a GPU. A seleção automática usa a segmentação temporal NVIDIA quando
validada, com alternativa ONNX no mesmo provider se a inicialização falhar.
**CPU** continua sendo uma escolha explícita. Não há download ao abrir o app.
Com CUDA, redimensionamento, desfoque e composição também usam a GPU, conservando
a máscara e as imagens intermediárias nela.
Consulte [Efeitos](docs/EFEITOS.md) para uso e [Análise RTX](docs/ANALISE_RTX.md)
para versões, evidências e limites. A execução foi exercitada na RTX 4070 SUPER;
correção de olhar e paridade integral com Windows continuam pendentes.

Preview e câmera virtual compartilham imagem, resolução e FPS. Com IA,
**FPS da IA** limita ambas a 30 por padrão. A janela ajusta o tamanho visual
preservando a proporção; o rodapé **Saída** informa resolução e taxa de saída.

Modelo e runtime disponíveis são detectados na abertura normal, incluindo os
arquivos preparados em `target/ai-runtime`. Instalações personalizadas são
registradas pelo preparador. **Atualizar** na aba Effects repete a descoberta
sem reiniciar a janela. A GUI apresenta disponibilidade e nomes de recursos,
sem campos de caminho. Looks e fundos próprios entram pelo diálogo **Adicionar…**
e permanecem na biblioteca para as próximas execuções.

## CLI e instalação

```bash
./target/release/elgato-virtualcam --test-camera
./target/release/elgato-virtualcam --list-controls
./target/release/elgato-virtualcam --set-control pan_absolute=-3600
./target/release/elgato-virtualcam --headless          # controlador sem janela
./target/release/elgato-virtualcam --start             # saída virtual em segundo plano
./target/release/elgato-virtualcam --set-effects effects.json
./target/release/elgato-virtualcam --status            # estado em JSON
./target/release/elgato-virtualcam --stop
./target/release/elgato-virtualcam --quit
./install-desktop-app.sh                             # ~/.local por padrão
./install-desktop-app.sh --autostart                  # opcional
./uninstall-desktop-app.sh                           # preserva configuração
```

GUI, bandeja e CLI usam a mesma instância. Fechar a janela com bandeja mantém a
saída virtual e pausa o preview. Sair encerra somente os processos da aplicação.
O instalador aceita `--prefix /caminho/absoluto` e `--no-build`.

Para compilar somente CLI, use `--no-default-features`; acrescente `--features ai`
se desejar IA nessa compilação. Não é necessário CUDA para compilar.

## Configuração e controles

O arquivo fica em `~/.config/elgato-virtualcam/config.json`, respeitando
`XDG_CONFIG_HOME`; use `--config ARQUIVO` para selecionar outro. Campos adicionais
na raiz são preservados. Um JSON inválido permanece intacto. O primeiro
salvamento mantém o arquivo anterior em `config.json.backup`.

**Salvar** e **R** salvam e restauram ajustes neste computador. Não gravam na memória
interna da câmera. Os controles respeitam limites, passos, menus e dependências
auto/manual; ganho permanece na unidade do driver.

Se o hardware recusar um valor, o aplicativo relê o estado e mostra
**Indisponível nesta configuração**. SAVE exclui esse controle e Restore continua
pelos demais. **Atualizar dispositivos e ajustes** permite nova tentativa.
Isso cobre a Facecam MK.2 que anuncia Backlight 0–1, mas pode recusar ambos.

## Documentação e validação

- [Manual completo](docs/README.md)
- [Cobertura de todos os temas documentais](docs/COBERTURA_DOCUMENTAL.md)
- [Arquitetura e fluxo de comandos](docs/ARQUITETURA.md)
- [Padrões de documentação e escrita](docs/PADROES_DE_DOCUMENTACAO.md)
- [Configuração, efeitos e IA](docs/EFEITOS.md)
- [Testes e verificações com hardware](docs/TESTES.md)
- [Resultados e limites da validação local](docs/VALIDACAO_LOCAL.md)
- [Investigação do pacote macOS e V4L2](docs/PORT_UBUNTU_24_04.md)

```bash
./test-desktop-app.sh
```

Os testes cobrem valores negativos, zero, positivos, extremos, persistência,
recusas, comandos IPC e renderização sintética. A verificação de câmera física
é optativa. Logs de vídeo ficam ao lado da configuração em `virtualcam.err.log`.

Firmware, HDR proprietário, Prompter e persistência interna da câmera permanecem
fora da implementação. A compilação suporta X11 e Wayland; a validação local
inicial foi em X11. A integração com cada consumidor de câmera virtual precisa
ser exercitada no ambiente de destino.

Projeto independente, sem afiliação com Elgato ou Corsair. Código sob [MIT](LICENSE).
O pacote macOS e evidências locais ficam em `package/`, ignorado pelo Git, e não
integram a distribuição. Modelos e runtimes externos mantêm suas próprias licenças.
