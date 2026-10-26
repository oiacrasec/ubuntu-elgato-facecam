# Execução da conversão autorizada

Esta página acompanha o escopo autorizado em 25/10/2026. O [roadmap](ROADMAP.md)
continua sendo a matriz de implementação e evidências; marcar uma etapa abaixo
não substitui seus testes.

Em 26/10, o usuário priorizou versão/atualização em Configurações, instalador,
remoção, ícones e integração da REA na produção antes de continuar a investigação
de olhar. A implementação está em `d6633f9`, com contratos e requisitos em
[Distribuição](DISTRIBUICAO.md). A branch de produção existente é `master`;
`main` é o mesmo papel, não um canal adicional. A pesquisa de olhar permanece
no [levantamento Windows](CORRECAO_DE_OLHAR_WINDOWS.md), sem nova alteração do efeito
nesta entrega de uso cotidiano.

## Escopo e ordem de trabalho

- [x] Internacionalização: PT-BR padrão, EN-US, arquivos de texto para a comunidade,
  seleção persistida e troca sem interromper captura ou perder rascunhos.
- [ ] Prompter: roteiro, leitura e rolagem, apresentação em monitor e controles
  pelo controlador, sem inserir o texto na saída da câmera. Núcleo implementado e
  validado em monitor comum; integração física e recursos avançados pendentes. Veja
  [Prompter](PROMPTER.md).
- [ ] IA: correção de olhar e enquadramento automático completos, com GPU e
  preparação validada dos componentes necessários. Quatro contratos AR 0.8.7
  Ada executaram no diagnóstico Rust; correção de olhar frontal implementada, com
  aceitação de qualidade em captura/consumidor ainda pendente. O recorte facial
  independente foi retirado a pedido do usuário.
- [x] HDR: descoberta e operação somente com contrato confirmado por modelo;
  não implementar persistência interna ou atualização de firmware nesta etapa.
  Implementado para MK.2 com contrato confirmado; execução física USB 2, limites e
  pendências por modelo em [HDR](HDR.md).
- [ ] Interface: teclado, acessibilidade, escala, Wayland e bandeja; comparação
  visual com Windows foi adiada pelo usuário.
- [ ] Confiabilidade e distribuição: consumidores, sessões prolongadas,
  desconexão, suspensão/retomada, reinício e instalação/atualização/remoção.

Novas famílias RTX e os outros controles proprietários não fazem parte deste
escopo. Instalações administrativas continuam exigindo confirmação pelo fluxo
APT do aplicativo. Hardware ausente limita a aceitação, não autoriza declarar
um recurso como validado. Commits devem conter código, documentação e resultados
coerentes, preservando o inventário histórico de cobertura.

## Registro inicial da retomada — 26/10/2026

Trabalho interrompido a pedido do usuário para preservar o contexto. A última
entrega de código é `3c9226c`; a árvore estava limpa antes deste registro.
Nesse ponto, o próximo trabalho era integrar AR ao vídeo, começando pela preparação
persistente e pelos contratos de detecção. A continuação está registrada ao final
desta página. Não reiniciar a investigação dos SDKs do zero.

| Commit | Entrega e limite |
| --- | --- |
| `e113ee1` | PT-BR/EN-US, troca imediata, catálogos JSON e descoberta de traduções comunitárias |
| `0e61cb0` | Núcleo Prompter: roteiro, rolagem, espelho, atalhos, monitor e CLI; integrações avançadas pendentes |
| `b34404e` | HDR MK.2, descoberta do seletor, confirmação por leitura, plano APT e persistência local; validado em USB 2, 1080p30/60 |
| `3c9226c` | Importação e execução dos quatro contratos AR Ada em worker Rust isolado; sem efeito AR no vídeo |

Fundo RTX, remoção de imagem própria e preview/saída virtual compartilhados já
existiam antes dessas entregas. Os resultados e limites estão em
[Validação local](VALIDACAO_LOCAL.md). Não confundir esses recursos com correção
de olhar ou enquadramento automático, indisponíveis na captura naquele registro inicial.

## Estado técnico registrado antes da continuação

O pacote Windows AR 0.8.7 Ada foi baixado adicionalmente, preservando o 0.8.2
da referência Camera Hub. Seus quatro engines usam TensorRT 10.4.0.26,
compatível com o runtime VFX existente. A RTX 4070 SUPER executou detecção,
landmarks 126, encoder e decoder tanto no trtexec quanto no worker Rust.
Nomes, formas, hashes e limitações estão em [Análise RTX](ANALISE_RTX.md) e no
[catálogo de runtimes](nvidia-runtime-catalog.json).

Arquivos que concentram a implementação atual:

- [`src/nvidia_ar_model.rs`](../src/nvidia_ar_model.rs): quatro contratos fixados,
  importação local validada por hashes e descritores EPContext próprios.
- [`src/nvidia_ar.rs`](../src/nvidia_ar.rs): diagnóstico e worker isolado; sessões
  TensorRT/CUDA com fallback CPU recusado, saídas finitas e encoder→decoder.
- [`src/nvidia_model.rs`](../src/nvidia_model.rs): parser de contêiner e adaptação
  restrita de plataforma reutilizados pelo importador AR.
- [`src/nvidia_setup.rs`](../src/nvidia_setup.rs): preparador VFX existente,
  referência para o futuro preparo AR pelo controlador.
- [`src/gpu.rs`](../src/gpu.rs), [`src/ai.rs`](../src/ai.rs) e
  [`src/nvidia.rs`](../src/nvidia.rs): processamento GPU e worker de fundo atuais,
  a preservar na integração.
- [`tests/ai.rs`](../tests/ai.rs): teste AR optativo, sem câmera.

Naquele registro, `--check-ar` extraía os modelos em diretório temporário a cada
invocação e removia esse diretório ao terminar. Ainda não havia manifesto AR
persistente, comando de preparação AR no Engine ou controles de AR funcionais
na GUI. Esse diagnóstico traz saídas para CPU para conferir contratos; isso não é
o desenho final do processamento de imagens.

### Recursos locais e análise que já estão disponíveis

Os caminhos abaixo são evidências locais ignoradas pelo Git, não dependências
distribuídas pelo projeto. Podem não existir em outro computador.

- Instalador: `/home/cesar/Downloads/NVIDEA/nvidia_ar_sdk_installer_v0.8.7_ada.exe`.
- Arquivos selecionados do pacote:
  `package/nvidia-reference/windows-x86_64/ar/0.8.7/ada/`.
- Contratos, engines de teste, logs, RTTI e decompilações focadas:
  `package/analysis-nvidia/windows-x86_64/ar/0.8.7/ada/`.
- Runtime VFX já preparado:
  `~/.local/share/facecam-hub/runtimes/linux-x86_64/vfx/0.7.6/ada/manifest.json`.
  Usa ONNX Runtime 1.20.1 e TensorRT 10.4.0.26. O fallback ONNX Runtime 1.22
  permanece separado por processo; não misturar bibliotecas dessas versões.

A última análise Ghidra terminou: `ghidra-boxes.log` confirma os helpers
`0x1800a81b0` e `0x1800304b0`. **A próxima leitura é dos arquivos
`direct-face-bbox-gpu.c.txt` e `direct-face-boxes.c.txt`**, para fechar a
decodificação das caixas. Não presumir uma constante de normalização ainda
não confirmada. Outros arquivos úteis no mesmo diretório:
`direct-face-normalize.c.txt`, `direct-preprocess-eye.c.txt`,
`direct-eye-warp.c.txt`, `direct-landmark-map.c.txt`,
`direct-features-eye.c.txt` e `direct-postprocess-eye.c.txt`.

A normalização de detecção observada preserva proporção, centraliza a imagem
com bordas pretas em 368×208 e usa BGR planar float dividido por 255.
O encoder usa imagem RGB planar 256×64. O alinhamento dos olhos envolve
geometria e landmarks; um recorte retangular isolado não fecha esse contrato.
Essas observações orientam código próprio; não copiar código ou shaders
proprietários. Nunca registrar chaves, pesos ou bibliotecas no Git.

Para outra análise focada, o script existente é
`package/analysis-windows-2.3.0.7295/ghidra-scripts/FocusedCameraHub.java`.
Usar `-loader-loadLibraries false -loader-ordinalLookup false
-loader-libraryLoadDepth 0 -noanalysis` e apagar o projeto temporário ao concluir.
Desabilitar apenas `loadLibraries` ainda provoca trabalho de resolução de DLLs.

## Sequência proposta no registro inicial

1. Ler os dois helpers de caixas já gerados e fechar o contrato de detecção.
2. Implementar preparação AR persistente pelo Engine, reutilizando progresso,
   cache, hashes, licenças e publicação atômica do preparador existente.
   Validar uma execução isolada antes de publicar. O destino proposto é
   `runtimes/linux-x86_64/ar/0.8.7/ada/`; ainda não é consumido pelo produto.
3. Integrar detecção e transformações de imagem na GPU. Apenas caixas,
   landmarks e outros dados pequenos podem retornar à CPU para a geometria.
   Compartilhar o caminho de imagem com o fundo, evitando cópias completas
   intermediárias e preservando o desempenho do modo somente fundo.
4. Implementar enquadramento com suavização e retorno seguro quando não há
   rosto; completar alinhamento dos olhos, encoder/decoder, confiança,
   transições e composição da correção de olhar. Não escrever PTZ físico
   para simular enquadramento digital.
5. Expor recursos pela GUI/CLI somente após integração funcional, com textos
   PT-BR/EN-US, descoberta automática e orientação de preparo sem campos de path.
   Medir preview e saída virtual, resolução, FPS e qualidade separadamente.
6. Continuar as lacunas de Prompter, interface, confiabilidade e distribuição
   registradas no [roadmap](ROADMAP.md). Voice Sync, Twitch, biblioteca de
   roteiros e integração física não foram concluídos. Confirmar alcance de
   NDI/automação antes de prometer compatibilidade.

A comparação visual com Windows continua adiada. Outras famílias RTX, firmware
e demais controles proprietários permanecem fora deste escopo. Hardware Prompter,
sessão GNOME Wayland real, OBS e reinício/suspensão ainda precisam de aceitação;
não encerrar a sessão, suspender ou reiniciar a máquina automaticamente.

## Checks disponíveis na retomada

A revisão AR passou com 79 testes comuns nos recursos padrão, 66 no build mínimo
e quatro verificações sintéticas FFmpeg. Formatação, Clippy sem avisos,
links/inventário e build release passaram. O teste nativo AR optativo passou em
54,54 s no debug; `--check-ar` release passou em 9,83 s incluindo hash do pacote,
extração e quatro sessões. Isso não mede inferência por frame ou qualidade.
Logs: `target/ar-checks.log`, `target/ar-clippy.log`, `target/ar-native-test.log`,
`target/ar-release-check.log` e `target/ar-release.log`.

Para reproduzir apenas o diagnóstico já implementado, sem abrir câmera:

```bash
./target/release/elgato-virtualcam --check-ar
```

Os testes comuns não podem escrever controles reais ou abrir a câmera.
Imagens reais de ensaios físicos devem permanecer somente em RAM; capturas de
tela registradas nesta etapa mostram preview pausado ou conteúdo sintético.
HDR foi restaurado ao valor original após os ensaios. A ferramenta HDR foi
extraída localmente para validação; sua instalação APT no sistema não ocorreu.
Novas instalações administrativas seguem o plano e a confirmação no app.
O inventário documental permanece com 152 fontes, sem alterar IDs ou hashes.

## Continuação: preparação persistente AR — 26/10/2026

Implementado `PrepareAr`/`--prepare-ar` pelo controlador, reutilizando fila,
progresso, logs e bloqueios de captura/Quit da preparação RTX. O módulo
`src/nvidia_ar_setup.rs` importa os quatro modelos AR para staging, preserva as
licenças, registra o runtime VFX existente e valida as quatro inferências em
worker isolado antes de publicar em `runtimes/linux-x86_64/ar/0.8.7/ada/`.
Não ativa efeito de olhar ou enquadramento, não instala SDK Linux e não grava
configuração. `ar_prepared` informa presença dos componentes; o worker confere
hashes dos engines antes da execução. O diagnóstico reutiliza a preparação,
mantendo a extração temporária quando ela ainda não existe.

Os dois helpers de caixas foram lidos. O despacho GPU leva a `0x1800a8090`;
a montagem em CPU confirma coordenadas e confiança, mas não fecha a constante
de normalização. O próximo trabalho é fechar essa rotina e integrar detecção e
geometria GPU ao worker de vídeo. Alinhamento dos olhos, enquadramento suavizado,
transições e composição continuam pendentes. Não reconstruir todo o SDK.

O teste optativo de preparação persistente passou na RTX 4070 SUPER com dados
temporários, reabertura do controlador, diagnóstico sem instalador e recusa de
engine corrompido. Resultados e checks desta continuação ficam em
[Validação local](VALIDACAO_LOCAL.md). A comparação visual com Windows, aceitação
física de Prompter e as demais lacunas anteriores permanecem pendentes.

Os componentes AR também foram preparados no diretório real do usuário,
`~/.local/share/facecam-hub/runtimes/linux-x86_64/ar/0.8.7/ada/`, e passaram
no diagnóstico release persistente. A próxima sessão pode reutilizar esse
manifesto sem extrair o instalador. Naquela entrega, nenhum efeito AR estava ligado à captura.

## Protótipo de detecção e recorte — 26/10/2026 (retirado)

O contrato de caixas foi fechado por leitura estática: stride 16, centro +0,5
pixel e escala 35. Um primeiro protótipo expôs acompanhamento por recorte e foi
exercitado com oráculos GPU/CPU e fotografia pública. O usuário esclareceu que
a opção desejada é corrigir os olhos; o controle de acompanhamento e o recorte
foram retirados. Detector e upload GPU foram reaproveitados internamente.

## Correção de olhar e limpeza de temporários — 26/10/2026

O efeito agora usa landmarks, alinhamento 2D próprio, encoder/decoder com alvo
(0,0) e composição local dos olhos. Funciona sozinho ou antes do fundo, sem
reenquadrar o vídeo. A GUI oferece **Efeitos** › **Correção de olhar** ›
**Olhar para a câmera**; a preparação chama **Preparar correção de olhar**.
O antigo campo `auto_frame` é lido sem efeito e não volta ao JSON salvo.
O campo atual é `eye_contact`, padrão false. GUI/CLI usam `SetEffects`.

A detecção de rosto, seleção de pontos e modelos são detalhes internos. Não
adicionar outro controle de acompanhar o rosto à interface. Os testes verificam
alinhamento, resposta ao alvo de olhar, composição limitada aos olhos, proteção
de piscadas/confiança/pose, composição com fundo e recusa de entrada truncada.
A aceitação em câmera real, óculos, movimento, consumidores e comparação visual
com Windows permanece pendente; o limite atual é alinhamento frontal 2D.

A limpeza autorizada removeu caches de builds antigos, runtimes de ensaios
substituídos, pacotes extraídos e cópias redundantes de instaladores. Recuperação
medida: 9,57 GiB, com configuração, runtime ONNX configurado e modelos AR/VFX
preservados. Recibo local: `target/cleanup-2026-10-26.json`. A preparação AR passou
a reutilizar o instalador local diretamente, evitando recriar uma cópia de 1,3 GB.

O próximo trabalho é validar a correção em captura e consumidor reais, com
qualidade, piscadas, confiança, resolução e FPS medidos separadamente. Preserve
os ensaios sem gravação de imagens pessoais e os limites do [Roadmap](ROADMAP.md).

## Revisão de estabilidade dos olhos — 26/10/2026

`6cb515b` registra a correção frontal e a limpeza anteriores. Após relato de
tremulação e aspecto artificial, a integração Windows foi revista; o Camera Hub
ativa `Temporal` e configura `EyeSizeSensitivity`. A implementação própria ganhou
histerese do detector para o rosto já validado, estabilização de geometria/máscara,
visibilidade contínua por olho e composição do residual para preservar textura.
Regressões GPU reproduzem a oscilação fotométrica e a perda/reaquisição; os
resultados e limites estão em [Validação local](VALIDACAO_LOCAL.md#estabilidade-dos-olhos-após-avaliação-do-usuário--26102026).
O alinhamento 3D e a equivalência visual com Windows permanecem pendentes.
