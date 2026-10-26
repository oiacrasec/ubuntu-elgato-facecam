# Validação

Execute na raiz, com Rust e Cargo no PATH:

```bash
./test-desktop-app.sh
```

O script verifica formatação, Clippy com avisos como erro, testes da aplicação e
contratos CLI/IPC, com os recursos padrão e na compilação mínima. Os testes
comuns não precisam de câmera, servidor gráfico, CUDA, modelo ou ONNX Runtime.

## Casos numéricos

| Domínio | Cobertura |
| --- | --- |
| Controles assinados | Negativos, zero, positivos, limites, i64 mínimo/máximo |
| Passos e menus | Desalinhamento, passo inválido e opções esparsas negativas |
| Booleanos | -1, 0, 1, 2 |
| FPS e resolução | Negativos, zero, mínimo/máximo, excedentes e overflow |
| Cor e redução de ruído | Negativos, zero, positivos, limites, NaN e infinito |
| Máscaras de IA | Alpha 0, 0,5 e 1; valores fora do intervalo e shapes inválidos |
| Composição e fundos | Referência escalar com tolerância de 1 por canal; alpha zero opaco; cenários e imagem inválida |
| Taxa de IA | Negativo, zero, 1, 30, 60 e acima do máximo; mesmo limite em preview e saída virtual |
| Persistência | Valores assinados preservados, backup e erro sem substituir JSON |
| LUT 3D | Valores negativos, zero e positivos; NaN, infinito, grade/domínio inválidos e conteúdo incompleto |

As recusas do driver têm testes separados dos limites matemáticos. Um controle
com metadados válidos pode ser recusado pelo hardware; o estado precisa manter
essa recusa e o último valor lido. Os testes não alteram controles reais.

## Biblioteca de recursos

Os testes comuns importam LUT própria, preservam nomes repetidos sem sobrescrita,
removem o original e redescobrem a cópia após reiniciar o controlador. Uma LUT
inválida deve falhar sem publicar arquivo parcial, aplicar efeitos ou salvar JSON.
Os três Looks próprios são gerados e validados em diretório temporário.

A remoção de imagens verifica cópia gerenciada, original preservado, recusa de
symlinks/arquivos externos/diretórios, limpeza da seleção salva e ausência no
catálogo. Falhas de salvamento mantêm o arquivo e a seleção; falha de exclusão
restaura os efeitos salvos. Remover imagem não aplicada não grava configuração.
O teste CLI/IPC usa arquivos sintéticos temporários, sem câmera ou runtime de IA.

Com FFmpeg disponível, o script também executa:

```bash
cargo test ffmpeg_library_imports_images_and_videos_and_rejects_invalid_content -- --ignored
```

O caso cria PNG/JPEG/WebP/BMP e MP4/MOV/WebM/MKV sintéticos, importa, remove os
originais e confere cor e dimensão das miniaturas. Mídia com conteúdo inválido
deve deixar a biblioteca vazia. Não há câmera, conta NVIDIA ou instalação APT.

## Renderização FFmpeg

Com `ffmpeg` instalado, a verificação opcional usa vídeo sintético e LUT própria,
compara pixels e verifica o filtro compartilhado, sem webcam ou saída virtual:

```bash
cargo test ffmpeg_effects_render -- --ignored
```

A CI instala FFmpeg e executa essa verificação. O script local também a executa
quando o executável está disponível.

`ffmpeg_preview_matches_decoded_virtual_pixels_resolution_and_frame_count`
compara todos os pixels do preview com uma decodificação independente dos mesmos
quadros yuv420p da saída virtual. Verifica dimensões, número de quadros, 4:3,
16:9, 60 fps e 30000/1001 fps, com efeitos e espelhamento. Iniciar a saída virtual
não pode alterar o preview. Esse teste usa arquivos temporários sintéticos e
roda no filtro `ffmpeg_`, sem câmera, display ou GPU.

Fundos animados usam uma mídia sintética de dois frames. O teste verifica loop,
conversão de FPS, caminhos com caracteres especiais, rejeição de mídia inválida
e recusa quando o processo proprietário já encerrou:

```bash
cargo test --locked ffmpeg_video_background -- --ignored
```

O script local executa também esse caso quando FFmpeg está disponível. Não há
modelo de IA, conta NVIDIA ou câmera nessa verificação.

## Inicialização no desktop

O teste de contrato `autostart_toggle_tracks_disk_reopens_and_preserves_state_on_failure`
usa diretórios XDG temporários: ativa, reabre, desativa, repete a desativação,
acompanha entradas desabilitadas externamente e verifica falha de filesystem sem
gravar o JSON da câmera. Não altera o autostart real do usuário.

`live_settings_persist_only_requested_fields_and_keep_state_on_failure` verifica
gravação imediata ao marcar/desmarcar, troca de saída, preservação de efeitos e
campos adicionais e estado inalterado após valor inválido ou falha de escrita.

## Instalação APT

Os testes comuns verificam preparação/cancelamento sem iniciar instalação,
IDs expirados, configuração preservada, hash alterado e interrupção ao falhar
um comando. Não executam comandos administrativos.

A preparação virtual verifica número livre, módulo nativo sem DKMS, pacote
correspondente ao kernel, ausência de escrita antes da confirmação e uma única
chamada a pkexec. O payload do helper recusa dispositivos, kernel, pacotes e
campos não permitidos. Os testes não carregam nem descarregam módulos. O teste
de falha de startup isola `v4l2-ctl`, inclusive quando há um loopback real no host,
para impedir captura física nos checks comuns.

Há regressões para pacotes presentes, ausentes, instalação parcial, arquivos de
configuração residuais e estados incompletos do dpkg. Um processo com banco dpkg
temporário verifica a mudança automática do snapshot sem gravar configuração ou
alterar pacotes reais. A validação da simulação recusa remoções adicionais e um
processo que termina com código zero não basta para declarar sucesso sem conferir
o estado esperado. Desinstalação administrativa real é um teste manual optativo.

O teste optativo abaixo verifica assinatura e candidato do SDK Manager usando
o mesmo formato de fonte gerado pelo app, com configuração, listas, cache e logs
APT temporários. Faz download de metadados e não instala pacotes nem altera
fontes do sistema:

```bash
mkdir -p target/apt-review
curl --fail --location https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2404/x86_64/3bf863cc.pub -o target/apt-review/nvidia-cuda.asc
FACECAM_NVIDIA_PUBLIC_KEY="$PWD/target/apt-review/nvidia-cuda.asc" cargo test --locked apt_repository_accepts -- --ignored --nocapture
```

## Hardware

Capture apenas frames em memória, sem salvar imagens:

```bash
cargo test --no-default-features hardware_preview_produces_frames_and_reaps_its_process -- --ignored
```

Para aceitar a saída real, prepare v4l2loopback no app e encerre outros produtores
que usam a Facecam ou a saída escolhida. O teste abre uma câmera física e mantém
os pixels apenas em memória; não grava imagens, altera controles ou prepara módulos:

```bash
FACECAM_TEST_VIRTUAL_DEVICE=/dev/video10 cargo test --locked --test hardware_virtual_camera -- --ignored --nocapture
```

Um consumidor FFmpeg recebe 30 frames por rodada, confere saturação zero e repete
depois de parar e reiniciar o produtor. A amostragem usa vizinho mais próximo;
U/V devem ficar a até 1 de 128, sem interpolação ou conversão RGB no oráculo.
O runtime e a configuração são temporários.
O teste fica ignorado e seu nome não corresponde ao filtro `ffmpeg_` dos checks
sintéticos. Para Chrome/OBS, selecione **VirtualCam**, confira resolução/FPS,
efeitos, retomada e parada. [Validação local](VALIDACAO_LOCAL.md) registra o teste
WebRTC real; frame recebido pelo FFmpeg não comprova aceitação pelo navegador.

A inferência é verificada separadamente para cada provider. Configure os arquivos
conforme [EFEITOS.md](EFEITOS.md) e execute:

```bash
./target/release/elgato-virtualcam --config /caminho/config.json --check-ai
```

Isso carrega o modelo, registra o provider e executa uma inferência real. Para
comparação CPU/CUDA, use dois arquivos de configuração com o mesmo modelo e
providers explícitos. Disponibilidade do driver ou presença de uma GPU não
substitui uma inferência. Nunca chamar uma inferência CPU de execução NVIDIA.

A revisão visual usa a janela real com vídeo sintético ou câmera; compilação não
comprova layout, integração Wayland ou consumo por OBS. Não habilitar testes que
mudam hardware automaticamente na CI.

## Testes optativos de IA

Para comparar os grafos CUDA próprios com a referência CPU, configure o runtime
GPU e suas bibliotecas conforme [Efeitos](EFEITOS.md):

```bash
FACECAM_AI_RUNTIME=/caminho/libonnxruntime.so LD_LIBRARY_PATH=/caminho/bibliotecas cargo test --locked gpu::tests -- --ignored --nocapture
```

Os dois testes usam apenas pixels sintéticos, sem câmera. Exercitam canais RGB/BGR,
normalização, alpha 0/0,5/1 e gradiente, blur com raios 1/4/10/32, fundo estático,
troca de fundo animado entre quadros e rejeição de probabilidades negativas,
acima de 1, NaN e infinitos. A entrada normalizada admite 1/255 de diferença e a
composição até dois níveis por canal, por diferenças de arredondamento float e
interpolação CUDA/CPU; não se exige identidade binária. Fundos animados também
verificam que o upload não sobrescreva o primeiro plano. O profiling opcional
está descrito em [Desempenho](DESEMPENHO.md).

Para a segmentação temporal NVIDIA, depois da preparação pelo app:

```bash
FACECAM_NVIDIA_MANIFEST="${XDG_DATA_HOME:-$HOME/.local/share}/facecam-hub/runtimes/linux-x86_64/vfx/0.7.6/ada/manifest.json" cargo test --locked --test ai nvidia_worker_composes -- --ignored --nocapture
```

O teste usa quadros pretos sintéticos, verifica oito quadros completos, cor
opaca, desfoque, três cenários distintos, reinício sem histórico anterior e
recusa de quadro truncado. Ele carrega as bibliotecas do manifesto somente no
filho, sem configurar o ambiente global. Não abre a câmera. A suíte comum
verifica metadados inválidos, limites do contêiner, máscara de probabilidades e
composição retangular 512×288 contra referência escalar, sem carregar CUDA.

Os testes de inferência exercitam frames completos e truncados. O teste com
câmera verifica preview processado e encerramento de todos os filhos:

```bash
export FACECAM_AI_RUNTIME=/caminho/libonnxruntime.so
export FACECAM_AI_MODEL=/caminho/selfie.onnx
export FACECAM_AI_PROVIDER=cuda
cargo test --test ai hardware_ai_preview_stops_all_owned_workers -- --ignored
cargo test --test ai ai_worker_renders_complete_frames_and_rejects_truncation -- --ignored
cargo test --test ai ai_video_background_reaps_decoder_and_kills_it_when_owner_dies -- --ignored
```

O teste direto do worker requer as bibliotecas CUDA no `LD_LIBRARY_PATH`. O teste
de controlador usa a descoberta de bibliotecas da estrutura do preparador.
Mantenha as bibliotecas de cada versão ORT separadas: para um worker ONNX 1.22,
use o diretório `lib` dessa versão antes das bibliotecas CUDA, sem acrescentar o
`lib` ORT 1.20 do manifesto RTX. Misturar providers nativos de versões diferentes
pode encerrar o processo; a seleção normal do app prepara o ambiente do filho.
O teste de fundo animado usa vídeo sintético e comprova composição, recolhimento
normal e morte do decoder por encerramento abrupto do worker. Um leitor do pipe
permanece aberto fora do worker no último caso, distinguindo SIGKILL de uma
saída causada apenas por broken pipe. Execute separadamente em CPU e CUDA.

## Contratos adicionais de engenharia

Novos parsers e serializadores precisam de round-trip, entrada truncada, campos
desconhecidos, limites de tamanho e preservação de estado após recusa. Não exigir
byte-identidade quando o contrato permite normalização; documentar a diferença.
Comandos precisam ser testados também no transporte público, para detectar
incompatibilidades entre GUI, CLI, JSON e estado persistido.

Uma futura suíte de propriedades deve gerar tipos incorretos, objetos vazios,
extremos de inteiros, dimensões inconsistentes e não finitos nos caminhos que
os aceitam. Fuzzing contínuo e um executor dedicado de comandos adversariais
não estão implementados. A ausência desses jobs não deve ser apresentada como
cobertura de segurança comprovada.

Corpus real deve ficar fora do Git principal e ser fixado por revisão e hash.
Fixtures sintéticas pequenas testam correção; dados autorizados e representativos
testam compatibilidade e qualidade. Um arquivo que abre, um resultado que combina
pixels e um resultado aceito por outro aplicativo são verificações distintas.

GPU e CPU precisam de oracles e tolerância publicada quando novos kernels forem
introduzidos. Testar limites pequenos simulados, formatos, alpha e perda de
recurso. A feature `ai` atual verifica modelo/provider e composição; ela não
implementa um compositor GPU genérico com paridade para todos os filtros.

Mudanças na descoberta têm fixtures de filesystem para ausência, paths explícitos,
versões de biblioteca e preferência de pacote. O teste de processo abre o
controlador sem `--config`, com diretórios XDG isolados, confere campos preenchidos
e verifica que a leitura não cria ou sobrescreve `config.json`.

O teste documental verifica links locais e o inventário de cobertura, inclusive
unicidade das entradas e existência dos destinos. Não é um avaliador automático
de qualidade da prosa ou de equivalência semântica entre manuais.

A preparação RTX sem ferramentas tem um contrato CLI comum, com `nvidia-smi`
simulado e PATH isolado: retorna um plano `nvidia_tools`, não inicia downloads,
não grava configuração e permite cancelar. Não consulta GPU real nem confirma APT.
O teste de controlador também recusa captura e Quit enquanto a preparação está ativa.

## Idiomas e captura da interface

`tests/localization.rs` usa controladores headless e diretórios temporários:
confere PT-BR padrão, EN-US persistido, tradução comunitária sem recompilar,
descoberta após atualizar, catálogo inválido e falha de salvamento sem perda do
estado. A ajuda mantém as flags em ambos os idiomas. Testes unitários verificam
paridade de chaves, duplicatas, limites, placeholders e preservação de valores
pessoais. Nenhum desses checks abre a câmera ou instala pacotes.

Para inspecionar a renderização real com vídeo sintético ou preview parado,
um build debug aceita `FACECAM_TEST_UI_CAPTURE=/caminho/janela.ppm`. Crie
`/caminho/janela.request` enquanto a janela estiver ativa: o helper usa o evento
de screenshot de egui e grava o framebuffer em PPM. Ele não existe no release,
não envia imagens e só atua com a variável definida. Espere o arquivo completo
antes de abrir outra janela ou converter a imagem. Não use com imagem pessoal
sem autorização. Essa captura evita artefatos de captura externa de janelas.

## Roteiro e apresentação

`tests/prompter.rs` usa controlador privado headless, texto sintético e CLI real.
Confere reprodução/pausa, posição negativa, zero, positiva, acima de 100%, NaN e
infinito, salvamento, falha de escrita, texto excessivo/inválido, preservação de
efeitos e recusa de apresentação sem GUI. Testes unitários exercitam pausa após
suspensão, final do roteiro, limites e mudança de monitores sem tocar no vídeo.

Na inspeção gráfica, use somente roteiro sintético. Exercite apresentação em
monitor, minimização da raiz, teclas, fechamento e retomada. O helper debug de
captura aceita `FACECAM_TEST_UI_VIEWPORT=prompter-output` para capturar somente a
janela de apresentação. Ausente essa variável, captura somente o viewport raiz.
O teste de texto espelhado compara o framebuffer refletido horizontalmente,
excluindo a guia fixa, e exige pixels de texto; uma janela vazia não passa.

## HDR

Checks comuns exercitam descritores truncados, GUID/seletor incorreto, unidade
ambígua, metadados incompatíveis, estados 0/1/pendente/inválido, saída não UTF-8,
timeout da ferramenta e recusa sem câmera. O contrato CLI prepara e cancela um
plano `hdr_tools`, sem instalar nem alterar a configuração.

O teste físico abaixo escreve somente HDR, lê o retorno, restaura o valor
original, confere outros controles e recebe preview apenas em memória. Feche
outros produtores e execute conscientemente; ele não faz parte do check comum:

```bash
FACECAM_HDR_DEVICE=/dev/video0 FACECAM_HDR_RATE=30 cargo test --locked --test hdr hardware_hdr -- --ignored --nocapture
FACECAM_HDR_DEVICE=/dev/video0 FACECAM_HDR_RATE=60 cargo test --locked --test hdr hardware_hdr -- --ignored --nocapture
```

Requer `uvcdynctrl` instalado, ou `FACECAM_UVC_TOOL` para uma ferramenta local de
validação e suas bibliotecas. O teste usa 1920×1080, recusa escrita sem captura,
confere retomada dos frames após alteração, Save/Restore local e restauração no
encerramento, inclusive após uma asserção. Não salva imagens da câmera.

## Contratos experimentais AR

A suíte comum confere os descritores próprios e recusa de arquivos incompatíveis,
sem runtimes ou modelos. Para exercitar importação e quatro inferências nativas:

```bash
cargo test --locked --test ai ar_models_import -- --ignored --nocapture
```

Requer RTX Ada, runtime VFX preparado e instalador AR 0.8.7 Ada local nos diretórios
de descoberta descritos em [Análise RTX](ANALISE_RTX.md). O comando verifica hashes
e usa staging temporário; pode demorar mais no perfil debug. As entradas são
sintéticas, sem captura de câmera. O resultado não equivale a um efeito de olhar
ou enquadramento implementado.

A preparação persistente tem um teste separado, também optativo e sem câmera:

```bash
FACECAM_NVIDIA_MANIFEST="${XDG_DATA_HOME:-$HOME/.local/share}/facecam-hub/runtimes/linux-x86_64/vfx/0.7.6/ada/manifest.json" cargo test --locked --test controller ar_preparation_persists -- --ignored --nocapture
```

Ele inicia controlador privado, prepara AR em dados temporários, aguarda resultado,
reabre e verifica descoberta sem escrita de configuração. Executa `--check-ar`
com HOME/cache vazios, usando o runtime registrado, e recusa engine corrompido.
Requer o instalador local ou permite seu download validado para o cache do usuário.
Os dados temporários são removidos ao terminar; não instala pacotes APT.

Checks comuns conferem manifesto truncado, versão/família/hash incompatíveis,
limite de leitura, descritor alterado, licenças ausentes e hash de engine inválido.
A publicação testa destino sem marcador, restauração após rename recusado e troca
de uma versão identificada. A CLI verifica plano APT e falta de runtime RTX sem
downloads, usando ferramentas simuladas; captura, Quit e novo preparo são recusados
enquanto o job está ativo. Esses checks não carregam CUDA nem abrem a câmera.

## Correção de olhar

Checks comuns conferem decodificação de caixas, probabilidades e formas inválidas,
negativos, zero, extremos, NaN/infinito, matrizes degeneradas, inversão de rotação,
confiança, head pose e pálpebras fechadas. A CLI recusa CPU/ONNX para
`eye_contact` sem alterar a configuração anterior. O campo antigo `auto_frame`
é aceito sem ligar processamento e não volta à serialização. Esses testes não
carregam CUDA nem abrem a câmera.

Com os componentes preparados e `LD_LIBRARY_PATH` apontando às bibliotecas do
manifesto VFX 1.20.1, execute os checks GPU optativos:

```bash
cargo test --locked --bin elgato-virtualcam nvidia_framing::tests::gpu_face_input -- --ignored --nocapture
cargo test --locked --bin elgato-virtualcam nvidia_gaze::tests::gpu_eye_warp -- --ignored --nocapture
FACECAM_NVIDIA_MANIFEST="${XDG_DATA_HOME:-$HOME/.local/share}/facecam-hub/runtimes/linux-x86_64/vfx/0.7.6/ada/manifest.json" cargo test --locked --test ai eye_contact_worker -- --ignored --nocapture
```

O input do detector aceita 1/255 de diferença por arredondamento; o alinhamento
bilinear dos olhos aceita 0,001 em cinza [0,255] e 0,00001 em RGB [0,1] contra
oracles escalares. O worker exercita quatro frames completos, EOF e truncamento,
com correção sozinha e junto de fundo sólido. O aquecimento passa pelos modelos
mesmo sem detectar rosto no quadro preto.

O teste positivo usa a fotografia externa de Eileen Collins do conjunto
[astronaut do scikit-image](https://scikit-image.org/docs/stable/api/skimage.data.html#skimage.data.astronaut),
originalmente NASA e declarada de domínio público nesse catálogo. O arquivo não
acompanha a distribuição; o teste confere seu SHA-256 antes de salvar derivados.

```bash
curl --fail --location --output /tmp/facecam-astronaut.png https://raw.githubusercontent.com/scikit-image/scikit-image/v0.24.0/skimage/data/astronaut.png
FACECAM_AR_TEST_IMAGE=/tmp/facecam-astronaut.png cargo test --locked --bin elgato-virtualcam nvidia_gaze::tests::public_face -- --ignored --nocapture
```

SHA-256: `88431cd9653ccd539741b555fb0a46b61558b301d4110412b5bc28b5e3ea6cb5`.
O ensaio alinha os olhos, conecta encoder→decoder e verifica que o alvo muda a
imagem gerada. A composição precisa alterar a região ocular e conservar todos
os valores fora das elipses. NaN, infinito e intensidade fora do limite geram
recusa. Sem rosto, a correção retorna a imagem original. Derivados públicos de
inspeção ficam em `target/ar-gaze-contract/`, ignorado pelo Git.

O teste não substitui aceitação de movimento real, óculos, oclusão, iluminação,
comparação com Windows ou consumo em OBS/navegador. Inspeções GUI usam preview
pausado; imagens pessoais de testes físicos não podem ser gravadas.
