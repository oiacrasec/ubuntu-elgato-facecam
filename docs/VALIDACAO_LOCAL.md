# Validação local — 25/10/2026

Ambiente observado: Ubuntu 24.04.5, X11, Facecam MK.2 USB2 e NVIDIA GeForce
RTX 4070 SUPER, driver 595.99.02. Estes resultados descrevem este host; não são
uma matriz de compatibilidade nem um benchmark controlado.

## Verificações concluídas

- Formatação e Clippy sem avisos, com recursos padrão e sem recursos padrão.
- 26 testes automáticos no build padrão: 22 testes unitários e 4 contratos CLI/IPC.
- 23 testes automáticos na compilação mínima, sem GUI, bandeja ou IA.
- Renderização FFmpeg sintética: espelho, brilho negativo/zero/positivo, redução
  de ruído, saturação zero e LUT, incluindo nomes de arquivo com espaços,
  apóstrofo, barra invertida, vírgula, dois-pontos, colchetes e ponto-e-vírgula.
- Mesma LUT entregue simultaneamente ao preview RGB e ao sink yuv420p sintético.
- Captura física sem efeitos e encerramento/reap do FFmpeg.
- Inferência ONNX real em CPU e em CUDA; processamento de frames completos e
  rejeição de frames truncados em ambos os providers.
- Preview físico com IA CUDA; verificação dos três filhos de vídeo antes da
  parada e ausência de seus PIDs após Stop. A inspeção inclui filhos de todas
  as threads do controlador.
- Preparador de IA em CPU, incluindo diretório com espaços, e configuração CUDA
  usando o cache local verificado. Downloads nativos CUDA tiveram SHA-256
  conferido; nenhum pacote global ou driver foi alterado.
- Revisão visual da aba Effects na janela nativa, com preview pausado. A
  screenshot em `screenshots/facecam-effects.png` não contém imagem da câmera.
- Links locais da documentação íntegros, sem a árvore documental copiada ou os
  arquivos da implementação anterior.

O teste de efeitos pela CLI aplica -0,25, zero e +0,25, confere o estado e o JSON
salvo e verifica que uma atualização inválida preserva ambos. A validação de
controles inclui extremos i64 e usa aritmética ampliada para evitar overflow.

## Amostra de execução

Com o release e IA CUDA, captura MJPEG 1280×720 a 30 fps, preview reduzido a
640×360 com limite de 15 fps:

| Observação | Resultado |
| --- | --- |
| Frames de preview adicionais | 76 |
| Intervalo observado | 5,13 segundos |
| Processos durante a captura | Controlador + decoder FFmpeg + worker Rust + FFmpeg de saída |
| Memória GPU reportada para o worker | 222 MiB |
| Filhos de vídeo remanescentes após Stop | 0 |

A contagem de preview não mede a taxa de uma câmera virtual consumida por outro
aplicativo. A janela de medição é curta e inclui o custo das consultas. Não houve
comparação controlada com outro software. Dados brutos e dependências utilizadas
ficam localmente em `target/ai-runtime/`, ignorado pelo Git; frames da câmera
foram mantidos somente em memória.

O binário GUI release medido na etapa de IA ocupava 17.747.632 bytes; o mínimo
headless, 1.850.784 bytes.
Isso não inclui FFmpeg, bibliotecas gráficas nem o runtime de IA. As bibliotecas
CUDA preparadas ocupam aproximadamente 2,4 GiB separadamente. Nada disso é
carregado na execução comum com IA desligada.

## Revisão de documentação e UI

Na revisão de 25/10/2026, versão declarada 3.0.0 em desenvolvimento, foram
registrados os [padrões de documentação e escrita](PADROES_DE_DOCUMENTACAO.md)
e alinhados os selects **Input** e **Format** do card **Device**.

O alinhamento foi conferido em janela 1240×790 com preview pausado e em janela
960×640 com captura ativa e selects desabilitados. A captura deste segundo
estado incluiu apenas o card, sem imagem da câmera. O build no perfil release,
o Clippy e a formatação passaram; os links locais também foram verificados.
O binário GUI recompilado ocupa 17.760.416 bytes. Esta revisão visual não
repete as medições de IA da etapa anterior.

![Card Device com os selects alinhados e preview pausado](screenshots/facecam-device.png)

## Revisão de cobertura e descoberta de IA

Na revisão seguinte de 25/10/2026, ainda na versão 3.0.0 em desenvolvimento, o
[manual](README.md) passou a mapear os 152 arquivos do acervo auditado, incluindo
137 Markdown, para seus destinos locais. A [matriz](COBERTURA_DOCUMENTAL.md)
registra assunto, adaptação e hash. Não há nomes ou textos copiados do produto
de origem na documentação mantida.

Foram aprovados 32 testes automáticos no build padrão e 29 no mínimo headless,
além da renderização sintética FFmpeg. Formatação, Clippy e sintaxe dos scripts
passaram. Dois testes documentais conferem links locais e a integridade das 152
entradas; isso não substitui revisão semântica.

O teste de processo iniciou sem `--config`, descobriu modelo/runtime em dados XDG
e em registro personalizado e não gravou `config.json`. Fixtures adicionais
verificam campos manuais, ausência, diretórios inválidos, preferência de pacote
CPU/CUDA e bibliotecas versionadas. A janela normal foi aberta com configuração
ausente e os dois campos apareceram preenchidos, sem ligar o desfoque.

Uma inferência CUDA real foi executada com caminhos omitidos, e uma inferência
CPU usou o registro produzido pelo preparador em diretório com espaços. O teste
de preview físico com IA também passou com ambos os caminhos vazios na configuração,
seguido de parada e reap de todos os filhos de vídeo. Esses testes não repetem a
bateria de qualidade visual ou a amostra de desempenho da etapa anterior.

O release GUI recompilado tem 17.817.328 bytes, SHA-256
`f313b9cdd48c8c8105a06272998a8b6449f602cce029a88cc4befe8fa06273eb`.
O mínimo headless tem 1.906.848 bytes. As dependências opcionais de IA continuam
fora desses tamanhos.

![Modelo e runtime preenchidos na abertura normal](screenshots/facecam-ai-discovery.png)

## Validação funcional do commit `6a21d11`

Em 25/10/2026, o executável release desse commit foi aberto na sessão X11 real.
Os testes usaram diretórios XDG de configuração e execução separados dos dados
do usuário. Os controles V4L2 foram lidos antes das alterações e restaurados
depois. Frames da câmera permaneceram em memória; capturas da janela foram
feitas com o preview pausado.

| Fluxo exercitado | Resultado observado |
| --- | --- |
| Abertura e segunda execução | Janela disponível, câmera detectada e segunda execução apresenta a mesma instância |
| Descoberta de IA | Modelo e runtime preenchidos; limpar os campos e usar **Detectar arquivos** também funciona |
| Preview pela interface | **Iniciar preview** entrega frames e **View** pausa a captura |
| Formatos | Os 14 modos anunciados produziram frames, incluindo YUYV e MJPEG; o FPS consultado no driver coincide com o solicitado |
| Controles físicos | Brilho negativo/zero/positivo, contraste, saturação, sharpness, pan, tilt, zoom, anti-flicker, FPS dinâmico e privacy aceitos com leitura de retorno |
| Automático e manual | Exposição, ganho e balanço de branco exercitados; temperatura manual inativa é recusada no modo automático |
| Entrada inválida | Brilho fora do intervalo, pan fora do passo e opção ausente de menu recusados |
| Backlight | Firmware recusa a escrita com ERANGE; a recusa permanece no estado e o controle é excluído de **SAVE** |
| Presets | **SAVE** e **R**, acionados na janela, salvam e restauram o brilho lido do hardware |
| Efeitos pela interface | Espelho, brilho negativo e denoise aplicados, persistidos e usados pelo preview; restauração dos padrões conferida |
| LUT | Caminho digitado na janela, LUT `.cube` aplicada e textura com pixels de vídeo confirmada sem salvar frames |
| IA NVIDIA | Desfoque ativado pela janela, três filhos de vídeo presentes e worker visível no `nvidia-smi` com 222 MiB |
| Execução contínua de IA | MJPEG 1280×720 a 30 fps configurados, 205 frames adicionais de preview em 15 segundos e nenhum erro; parada encerra os três filhos |
| Troca para CPU | Provider alterado na janela com preview ativo; nova captura processada entrega frames |
| Falha de IA ou efeito | LUT ausente, brilho inválido, modelo ausente e runtime CPU com provider CUDA recusados, mantendo configuração e preview anteriores |
| Worker nativo | CPU e CUDA processam um frame completo e recusam entrada truncada nos testes optativos |
| Bandeja | Fechar a janela oculta a UI e encerra o preview; ativação D-Bus da bandeja reapresenta a janela |
| Interrupção de captura | Encerrar somente o FFmpeg pertencente ao teste produz erro visível e encerra os filhos de vídeo |
| Configurações e reabertura | **Salvar**, início minimizado e autostart exercitados em diretório isolado; JSON preservado após reabrir |
| Instalação | Instalar e remover com prefixo contendo espaço funciona, incluindo lançador e autostart isolados |
| Saída virtual ausente | Botão exibe a instrução de v4l2loopback e preserva o preview que já estava ativo |

Os checks automáticos passaram novamente: formatação, Clippy, 32 testes no build
padrão, 29 no mínimo, renderização sintética FFmpeg e sintaxe dos scripts.
Os logs desta sessão ficam em `target/functional-review/` e
`target/functional-checks.log`, ignorados pelo Git.

A passagem por cada modo foi curta e confirma abertura e entrega de frames;
consultar FPS no driver não mede a taxa sustentada de entrega. Grid e Mirror
local foram acionados sem alterar o efeito de espelho persistido, mas não houve
comparação visual automatizada dessas sobreposições. Nenhum destes resultados
constitui uma garantia para outros dispositivos ou condições de iluminação.

## Revisão de desempenho e fundos de IA

Em 25/10/2026, após o relato de lentidão em 1080p60, o binário anterior foi
comparado ao release com composição reutilizável, redução antes do worker no
preview e taxa de processamento explícita. Ambos usaram o mesmo modelo, runtime
CUDA, driver e RTX 4070 SUPER. Não houve alteração de driver ou instalação global.

No benchmark sintético do worker completo, cada execução recebeu 80 frames RGB
1920×1080 e os primeiros cinco ficaram fora da janela de taxa estável. Foram
feitas três execuções de cada binário, alternando a ordem. A mediana passou de
**14,37 para 47,83 fps**, aproximadamente 3,3 vezes. Esse teste inclui inferência,
composição e pipes, sem decodificação de câmera, GUI ou consumidor virtual.
A composição nova é comparada ao caminho escalar anterior em teste de pixels,
com tolerância máxima de um nível u8 por canal; os extremos de alpha mantêm
primeiro plano e substituição opaca.

Em uma amostra separada da câmera física MJPEG 1920×1080 a 60 fps, somente preview:

| Observação | Anterior | Revisão |
| --- | --- | --- |
| RGB recebido pelo worker | 1920×1080 | 640×360 |
| Frames adicionais de preview em aproximadamente 5,1 s | 18 | 77 |
| CPU do controlador e três filhos, com 100% = um núcleo | 113,2% | 46,8% |
| Início do pedido até pelo menos cinco frames | 2.793 ms | 992 ms |
| Filhos de vídeo após Stop | 0 | 0 |

A implementação anterior mantinha timestamps rawvideo da entrada a 60 fps
mesmo quando o worker processava mais devagar; o filtro de preview descartava
frames dessa sequência. A nova entrada do segundo FFmpeg declara a taxa
efetivamente escolhida para processamento. O preview já chega ao worker em
640×360 e até 15 fps; com saída virtual, `ai_fps` limita a taxa e mantém a resolução.
As amostras são curtas e não medem latência ponta a ponta ou taxa entregue ao OBS.

Foram exercitados na captura real desfoque, cor sólida, estúdio, escritório,
paisagem e PNG próprio. Seleção de estúdio e cor, aplicação e edição do caminho
da imagem foram acionadas na janela. Um PNG corrompido foi recusado no aquecimento,
preservando a configuração e o preview anteriores. Cenários são ilustrações
próprias geradas por código, não fotos ou recursos de terceiros. Testes também
verificam JPEG, WebP, transparência opaca e rejeição de dimensões excessivas.

Os arquivos de medição ficam em `target/functional-review/ai-comparison.json`,
`live-before.json` e `live-after.json`, ignorados pelo Git. Os checks aprovados
somam 36 testes no build padrão e 30 no mínimo, além da renderização FFmpeg,
Clippy, formatação e sintaxe dos scripts. A comparação não certifica o SDK
Maxine: esta revisão continua usando ONNX Runtime/CUDA.

![Seletor de fundos com o preview pausado](screenshots/facecam-backgrounds.png)

## Fundos animados e instalação APT — 25/10/2026

O alvo de aceitação passou a ser Camera Hub Windows 2.3.0.7295, incluindo layout
e recursos de IA na RTX 4070 SUPER, conforme funcionamento relatado pelo usuário
no Windows. Não houve execução do aplicativo Windows no Ubuntu nem comparação
pareada de qualidade ou desempenho com essa plataforma.

Os checks padrão passaram: 40 testes no build completo e 33 no headless,
formatação, Clippy e sintaxe dos scripts. Duas renderizações optativas FFmpeg
passaram, incluindo fundo animado com loop, adaptação de taxa, caminho com
caracteres especiais e recusa de mídia inválida. Testes nativos em CPU e CUDA
confirmaram composição e encerramento do decoder em duas situações: EOF normal
e morte forçada do worker. Um leitor externo manteve o pipe aberto no segundo
caso, permitindo verificar o sinal de morte do pai sem depender de broken pipe.

Na Facecam, um preview CUDA com fundo MP4 próprio, entrada MJPEG 1280×720 a
30 fps e processamento de preview a 640×360/15 fps produziu 32 frames em 2.740 ms,
incluindo início da captura. Foram observados três filhos de vídeo do controlador
e um decoder de fundo pertencente ao worker. Após Stop, nenhum desses processos
continuou em execução. A amostra confirma entrega, não uma medição sustentada de
FPS ou latência ponta a ponta. Nenhum controle da câmera foi escrito nesta etapa.

Na janela X11, foram exercitados o seletor nativo do portal KDE, a seleção de um
MP4 com espaço no nome, aplicação e salvamento pelo controlador. O painel mantém
controles à esquerda e preview à direita; a revisão não certifica igualdade de
todos os pixels e estados com o Windows. CPU é apresentada nas opções avançadas
como alternativa ONNX, sem ser confundida com um fallback dos SDKs NVIDIA.

O fluxo APT foi aberto pela UI e cancelado na revisão do plano. O cancelamento
não iniciou comandos, não criou log de instalação e não cadastrou repositório.
Os testes de IPC também verificaram confirmação ausente, plano substituído,
ID cancelado e configuração preservada. Um teste optativo do APT verificou a
assinatura NVIDIA e encontrou `sdkmanager 26.1.0-14392` usando configuração,
listas, cache e logs temporários; fontes do sistema permaneceram intactas.
A chave pública tem o hash registrado em [Dependências](DEPENDENCIAS_E_LICENCAS.md).

A autenticação administrativa real e uma instalação de pacotes não foram
executadas nesta revisão. Os resultados ficam em `target/windows-parity-review/`
e o log dos checks em `target/functional-review/windows-parity-checks.log`,
ignorados pelo Git. A integração Maxine VFX/AR Linux continua pendente de seus
runtimes/modelos e validação própria; SDK Manager não fornece essa paridade.

## Biblioteca automática de recursos — 25/10/2026

![Biblioteca por nome e miniatura de vídeo, com câmera pausada](screenshots/facecam-resource-library.png)

A GUI passou a mostrar Looks e fundos pelo nome, com miniatura do fundo, sem
campos de caminho para LUT, imagem, vídeo, modelo/runtime ou dispositivo virtual.
Os três Looks próprios foram gerados e validados; não contêm recursos extraídos
do Camera Hub. Na janela X11, **Adicionar LUT…** abriu o portal KDE, importou uma
LUT sintética, selecionou seu nome e só persistiu o efeito após **Aplicar e salvar**.
O Look **Quente** também foi materializado e aplicado pela UI. Salvar configurações
preservou Quente confirmado e Frio em rascunho, sem aplicação implícita. O rodapé continuou
acessível com rolagem, opções avançadas abertas e janela de 1000×680 pixels. A saída virtual foi apresentada
como não detectada, coerente com a ausência do módulo neste teste.

Passaram 43 testes comuns no build completo e 36 no headless, além de três testes
optativos FFmpeg, formatação, Clippy e sintaxe dos scripts. A biblioteca foi
exercitada com PNG, JPEG, WebP, BMP, MP4, MOV, WebM e MKV sintéticos, caminhos com
espaço e caracteres especiais, origem removida, nomes repetidos, reinício e
conteúdo inválido. LUTs incluíram valores negativos, zero, positivos e não finitos.
Importar não aplicou efeitos nem escreveu configuração; falhas não publicaram
arquivos parciais. Logs: `target/windows-parity-review/library-checks-final.log`.

Um preview real com CUDA, Look Quente e fundo animado produziu 32 frames em
2.754 ms, incluindo partida, com entrada 1280×720/30 fps e preview 640×360/15 fps.
Stop recolheu os três filhos de vídeo e o decoder do worker, sem controles
V4L2 escritos. É confirmação de funcionamento local, sem benchmark sustentado
ou comparação de qualidade com Windows. Resultado: `live-library-result.json`
no mesmo diretório ignorado. A integração dos SDKs Maxine continua pendente.

## Estado das dependências e Configurações — 25/10/2026

O teste APT executado pelo usuário terminou sem erro. O log
`~/.config/elgato-virtualcam/install-1792952383366.log` informa que FFmpeg
`7:6.1.1-3ubuntu5` e v4l-utils `1.26.1-4build3` já eram as versões mais recentes,
com zero pacotes novos instalados. A consulta independente ao dpkg confirmou
ambos instalados. O SDK Manager estava ausente.

O app passou a derivar as ações desse estado, incluindo pacotes que já existiam
antes de sua execução. Na janela X11 de 1240×790 pixels, Configurações ficou
centralizada nos dois eixos, inclusive após reduzir a janela para 1000×680,
com largura de 560 pontos lógicos e dois botões
iguais na mesma linha. FFmpeg/V4L2 exibiu **Desinstalar…**, e SDK Manager exibiu
**Instalar…**. A abertura de **Desinstalar…** simulou somente os dois pacotes
solicitados e mostrou a confirmação própria. Cancelar descartou o plano sem
iniciar processo administrativo ou criar log de remoção. Não houve remoção real.

A revisão visual seguinte acrescentou separador e título ao bloco **Câmera virtual**,
alinhou seletor e **Atualizar dispositivos** na mesma linha e posicionou **Salvar**
e **Encerrar aplicativo** à direita, mantendo o rodapé compacto. O layout foi
inspecionado em 1240×790 e 1000×680 pixels. Na captura atual, SDK Manager
`26.1.0-14392` também já estava instalado, confirmado pelo dpkg, e ambos os botões
exibiram **Desinstalar…**. Isso não comprova instalação dos runtimes Maxine VFX/AR.

![Configurações centralizada e ações das dependências na mesma linha](screenshots/facecam-settings.png)

Passaram 47 testes comuns no build completo, 40 no headless e três verificações
FFmpeg, além de Clippy, formatação e sintaxe dos scripts. As regressões incluem
estado dpkg parcial/incompleto, rejeição de remoções adicionais, conferência do
estado final e atualização automática a partir de um banco dpkg temporário,
sem alterar pacotes do sistema. Evidências ficam em `target/apt-state-review/`
e `target/apt-state-checks.log`, ignorados pelo Git.

## Cards da aba Effects — 25/10/2026

**Effects** passou a usar cards no mesmo padrão de **Camera**: Look, Picture,
Processing e AI Background, com provider em Opções avançadas inicialmente
recolhido. Foram conferidos espaçamento, alinhamento de rótulos/sliders/valores,
recolhimento, rolagem e acesso ao rodapé de aplicação na janela X11, em
1240×790 e 1000×680 pixels. Recolher Picture em Effects não recolheu Picture
na aba Camera; os estados foram conferidos ao alternar entre as abas.

Um Brightness de -0,25 foi editado pelo campo numérico. O snapshot confirmado
permaneceu em zero até Aplicar e salvar; depois, estado e JSON refletiram -0,25.
Um vídeo sintético com nome longo entrou na biblioteca, apareceu abreviado no
seletor de largura estável e teve miniatura exibida. Restaurar efeitos padrão e
aplicar retornou Brightness a zero e o modo de fundo a desfoque. Nenhuma captura foi iniciada
nem controle V4L2 escrito. Evidências ficam em `target/effects-ui-review/`.

O build release foi atualizado e `./test-desktop-app.sh` passou com 47 testes no
build completo, 40 no headless, três verificações FFmpeg, formatação, Clippy e
sintaxe dos scripts. A alteração de layout mantém os mesmos comandos do controlador.

![Cards de Effects com preview pausado](screenshots/facecam-effects.png)

## Limites da validação

Não foi carregado um módulo de kernel para este teste. A saída yuv420p e sua
divisão com preview foram verificadas por um sink sintético, sem comprovar
consumo ponta a ponta por OBS, navegador ou videoconferência. Wayland está
habilitado na compilação, mas não foi exercitado em sessão real nesta etapa.
A CI Ubuntu 24.04 foi atualizada; sua execução remota não foi observada aqui.

A IA implementada desfoca ou substitui o fundo por cor, cenário ilustrado ou
imagem ou vídeo próprio. Não há correção de olhar, relighting, Prompter, atualização de
firmware ou gravação interna na câmera.
A qualidade visual da segmentação não foi avaliada em uma bateria de pessoas,
iluminações e movimentos. Os testes confirmam execução, composição e ciclo de
vida; não garantem recorte perfeito em qualquer cena.

## Preparação e consumo da câmera virtual — 25/10/2026

Árvore em desenvolvimento após `2091dc1`, Ubuntu 24.04.5, kernel
`7.0.0-38-generic`, sessão X11 e Facecam MK.2 em USB2. O módulo v4l2loopback
0.15.3 já estava no pacote nativo do kernel, assinado pela Canonical, com Secure
Boot ativo. Não foi necessário instalar DKMS ou alterar o driver NVIDIA.

O usuário confirmou o plano na GUI e autenticou a preparação. O fluxo inicial
solicitou quatro autenticações, uma por comando. Os quatro passos terminaram
sem erro: modprobe, dois arquivos de configuração e update-initramfs.
`/dev/video10` apareceu como **VirtualCam**, driver `v4l2 loopback`, e os arquivos
`/etc/modprobe.d/facecam-hub.conf` e `/etc/modules-load.d/facecam-hub.conf` foram
conferidos. Não houve reinicialização para testar a persistência.

A revisão seguinte substitui as quatro chamadas pkexec por uma única chamada
ao helper restrito do próprio binário. Os testes conferem um único comando de
autorização, igualdade entre payload e etapas revisadas e recusa de entradas
inválidas. A execução privilegiada desse novo helper ainda não foi repetida:
o módulo já estava carregado, e o app não o descarrega para refazer a preparação.

O Chrome 156.0.8078.47 consumiu a câmera virtual real por WebRTC, em perfil
temporário e sem dispositivo de câmera sintético. O produtor usou a Facecam em
MJPEG 1280×720/30 e enviou yuv420p. O preview permaneceu desligado.

| Rodada | Saturação | Vídeo informado pelo consumidor | Frames apresentados | Janela de medição |
| --- | --- | --- | --- | --- |
| 1 | 0 | 1280×720, 30 fps, sem redimensionamento | 45 | 1.535 ms |
| 2, após parar e reiniciar | 1 | 1280×720, 30 fps, sem redimensionamento | 45 | 1.503 ms |

Uma amostra em memória de 64×36 confirmou a alteração de cor: a maior diferença
entre canais RGB foi 3 com saturação zero e 185 com saturação um. A parada final
deixou `streaming` e `preview` falsos, sem erro. São amostras curtas de aceitação,
não um ensaio de estabilidade ou garantia sustentada de taxa.

O teste optativo Rust `hardware_virtual_camera_delivers_effects_and_restarts`
também passou: um consumidor FFmpeg recebeu 30 frames em cada uma de duas rodadas
com parada e retomada. Conferiu crominância neutra a até 1 de 128 com saturação
zero. Pixels ficaram apenas em memória; não foram salvas imagens, gravações ou
capturas de tela da pessoa. Nenhum controle físico foi escrito.

`./test-desktop-app.sh` passou com 50 testes comuns no build completo, 43 no
headless, três checks FFmpeg sintéticos, formatação, Clippy e sintaxe dos scripts.
Os checks comuns continuam sem captura física; o teste de falha de startup
agora isola a ferramenta V4L2 para não selecionar um loopback presente no host.
O teste físico permanece ignorado por padrão. Logs e métricas locais estão em
`target/virtual-camera-review/`, ignorado pelo Git.

OBS, sessão Wayland, operação prolongada e persistência após reiniciar continuam
pendentes. A investigação seguinte confirmou a distribuição separada de Core e
recursos VFX/AR e o limite oficial de suporte a aplicações cliente no Linux;
[NVIDIA Maxine](NVIDIA_MAXINE.md) registra fontes e condições para avançar.
Não foram encontrados os pacotes de runtime/modelos necessários no host, nem
foi implementado ou anunciado um backend Maxine nesta revisão.

## Checkbox de início automático — 25/10/2026

**Iniciar ao entrar no desktop** passou a ser um checkbox com aplicação imediata
pelo controlador. Na janela X11 de 1240×790, marcar criou a entrada desktop em
um `XDG_CONFIG_HOME` isolado, e desmarcar removeu o arquivo e publicou false no
snapshot. **Iniciar na bandeja** permanece uma preferência confirmada por **Salvar**.
As capturas `enabled.png` e `disabled.png` em `target/autostart-ui-check/` mostram
preview pausado. O cadastro real do usuário não foi alterado pelo teste visual.

O contrato automatizado confirmou persistência ao reabrir, desativação idempotente,
atualização após desabilitação externa e falha de filesystem sem marcar sucesso
ou gravar a configuração da câmera. O script completo passou com 51 testes no
build padrão e 44 no headless, além de três checks FFmpeg, formatação e Clippy.
O caso de autostart atualizado também passou isoladamente nas duas compilações.
O build release foi atualizado; não houve novo login no desktop nesta validação.

## Configurações com aplicação imediata — 25/10/2026

O botão **Salvar** foi removido de Configurações. `UpdateSettings` persiste os
campos alterados antes de publicar o novo estado, preservando efeitos e controles.
**Iniciar na bandeja** pode ser alterado durante captura; a seleção da câmera
virtual continua disponível apenas com captura parada.

Na janela X11 de 1343×856, o checkbox foi marcado e desmarcado, sem botão de
confirmação. O JSON isolado e o snapshot acompanharam true e false imediatamente.
A janela permaneceu aberta com preview pausado. A imagem `settings.png` e os
resultados em `target/settings-live-ui-check/` registram essa inspeção; não houve
alteração do autostart real do usuário ou de controles da câmera.

O teste de núcleo confirmou preservação de efeitos/campos adicionais e manutenção
do estado em valor inválido ou erro de filesystem. O script completo passou com
52 testes comuns no build padrão e 45 no headless, três checks FFmpeg sintéticos,
formatação, Clippy e sintaxe dos scripts. O release foi atualizado.

## Ajuda contextual em Configurações — 25/10/2026

O aviso permanente sobre revisar comandos APT foi removido de Configurações.
A orientação para parar a captura foi movida para o pequeno **(?)** ao lado de
**Câmera virtual**. Na janela X11 de 1343×856, a mensagem apareceu ao passar o
mouse sobre o indicador; a linha de orientação deixou de ocupar espaço no bloco.
O preview permaneceu pausado, sem operações administrativas.

As imagens `settings.png` e `help.png` em `target/settings-help-ui-check/` registram
a inspeção do layout e do tooltip. Formatação, Clippy, links e inventário
documental passaram. A alteração afeta apenas apresentação; o build release
foi atualizado, sem repetir testes de hardware.

O indicador textual foi substituído por um ícone vetorial circular com **?**
centralizado, desenhado pelo egui. A revisão visual em 1343×856 confirmou o ícone
e a ajuda ao passar o mouse, com preview pausado. As imagens ficam em
`target/settings-help-icon-ui-check/`. Formatação, Clippy, documentação e build
release passaram nesta alteração de apresentação.
