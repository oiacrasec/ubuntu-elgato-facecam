# Facecam Hub para Ubuntu

Controlador nativo em Rust para Elgato Facecam, com layout do Camera Hub:
controles à esquerda, preview à direita e abas Camera e Effects. Destino:
Ubuntu 24.04+. Preview e câmera virtual compartilham LUT, ajustes de cor,
espelhamento, redução de ruído e desfoque de fundo por IA opcional.

## Executar

```bash
sudo apt install build-essential pkg-config libxkbcommon-dev libwayland-dev \
  libx11-dev libgl1-mesa-dev ffmpeg v4l-utils v4l2loopback-dkms
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --release --locked
./target/release/elgato-virtualcam
```

Use Rust 1.95 ou posterior ([instalação](https://rustup.rs/)). Clique em
**Iniciar preview** para ver a câmera. A captura negocia formato, resolução e FPS
com o dispositivo. A GUI usa egui/OpenGL, e a bandeja usa D-Bus; não é necessário
runtime Python, Qt, GTK ou navegador. No GNOME, a bandeja pode exigir a extensão
AppIndicator. Sem bandeja, a janela permanece acessível.

Para usar a saída em OBS, navegador ou videoconferência, prepare um nó livre:

```bash
sudo modprobe v4l2loopback video_nr=10 card_label=VirtualCam exclusive_caps=1
```

Clique em **Iniciar câmera virtual** e selecione **VirtualCam** no consumidor.
O aplicativo não carrega ou descarrega módulos automaticamente. Preview e
ajustes da câmera funcionam sem a saída virtual. A saída é yuv420p.

## Efeitos e IA NVIDIA

Na aba **Effects**, ajuste cor, redução de ruído e LUT `.cube`, ou configure
**AI Background**. **Aplicar e salvar** reinicia brevemente a captura e aplica
os efeitos às duas saídas. A grade e o botão de espelho abaixo do preview são
ferramentas locais de visualização.

A IA segmenta a pessoa e desfoca o fundo. O provider **NVIDIA CUDA** usa ONNX
Runtime GPU; **CPU** é uma escolha explícita. Modelo, runtime e dependências
CUDA/cuDNN são externos e carregados somente ao ativar IA. Nenhum arquivo é
baixado ao abrir a aplicação. Consulte [EFEITOS.md](docs/EFEITOS.md) para
preparação, JSON, comandos e limites. Essa implementação não usa o SDK
proprietário da NVIDIA nem exige conta NGC.

Modelo e runtime disponíveis são detectados na abertura normal, incluindo os
arquivos preparados em `target/ai-runtime`. Instalações personalizadas são
registradas pelo preparador. Caminhos manuais têm prioridade; **Detectar arquivos**
na aba Effects permite preencher campos vazios sem reiniciar a janela.

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

**SAVE** e **R** salvam e restauram ajustes neste computador. Não gravam na memória
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
