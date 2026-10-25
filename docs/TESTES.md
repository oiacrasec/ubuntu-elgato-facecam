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
| Taxa de IA | Negativo, zero, 1, 30, 60 e acima do máximo; preview reduzido antes do worker |
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

Fundos animados usam uma mídia sintética de dois frames. O teste verifica loop,
conversão de FPS, caminhos com caracteres especiais, rejeição de mídia inválida
e recusa quando o processo proprietário já encerrou:

```bash
cargo test --locked ffmpeg_video_background -- --ignored
```

O script local executa também esse caso quando FFmpeg está disponível. Não há
modelo de IA, conta NVIDIA ou câmera nessa verificação.

## Instalação APT

Os testes comuns verificam preparação/cancelamento sem iniciar instalação,
IDs expirados, configuração preservada, hash alterado e interrupção ao falhar
um comando. Não executam comandos administrativos.

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
