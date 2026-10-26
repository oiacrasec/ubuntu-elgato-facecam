# Camera Hub Windows como referência de migração

O MSI 2.3.0.7295 amplia as evidências para portar o Camera Hub ao Ubuntu.
Ele confirma componentes e caminhos de código; não torna a versão Linux equivalente
nem comprova quais opções funcionam em cada câmera. Análise estática de 25/10/2026.

Após a análise, o usuário confirmou funcionamento completo no Windows, inclusive
IA com sua RTX 4070 SUPER. O alvo do produto é reproduzir essa experiência no
Linux. O relato é evidência de uso do Windows, sem taxa, duração ou comparação
lado a lado registradas nesta revisão.

O instalador macOS e suas extrações/relatórios exclusivos foram removidos de
`package/` em 25/10/2026. Comparações macOS desta página preservam resultados
históricos; o MSI, os arquivos Windows e sua investigação permanecem disponíveis.

## Artefatos e ferramentas

| Entrada | Identidade observada |
| --- | --- |
| Instalador | `package/CameraHub_2.3.0.7295_x64.msi`, 370.618.368 bytes |
| SHA-256 do MSI | `fe7bc3d2a06588d6e3ba4ac603bb9c3ea9b733b5e216d058a59ff7453ecdf689` |
| Produto na tabela MSI | Elgato Camera Hub, `2.3.0.7295`, x64 |
| Executável instalado | `Camera Hub.exe`, 52.639.384 bytes, PE x86-64 |
| SHA-256 do executável | `c66fde57de800dccd539ee5f42a52624124ef0769765aadc7320ab43d5f96d88` |
| Biblioteca de controles | `CCUVC.X64.DLL`, versão MSI `1.0.0.203` |
| SHA-256 da biblioteca | `3364a83b5e65d573cb99b03a98f56433055ac30cebfab5af602ff3f063d50a9e` |

A extração 7-Zip produziu 249 arquivos, totalizando 876.366.236 bytes. O CAB
usa alguns identificadores da tabela File, como `ExeFile`; a tabela do MSI
estabelece que esse item se instala como `Camera Hub.exe`. Um identificador
terminado em `File` não é necessariamente uma biblioteca ou executável.

Foram usados REA 6.7.0, sua CLI e seu servidor MCP local, Ghidra 12.1.4 e JDK
21.0.12.1. A distribuição oficial do Ghidra foi preparada somente em
`package/tooling/`, com o SHA-256 do arquivo ZIP conferido contra o digest
publicado na [release oficial](https://github.com/NationalSecurityAgency/ghidra/releases/tag/Ghidra_12.1.4_build):
`ddac49f903da9d5bac833e5cc79395098b9c33cfd3279be5f31bd00387d2d4db`.
Não foram alterados o checkout vizinho, Java do sistema ou registro MCP do cliente.

O MSI, suas ações de instalação, o executável e os drivers Windows não foram
executados. A inspeção não escreveu na câmera nem atualizou firmware.
Pseudocódigo, projetos temporários e recursos extraídos permanecem em `package/`,
ignorado pelo Git; não são incorporados ao produto Rust.

## Comparação com o pacote macOS

A comparação usou o `.pkg` da mesma versão, identificado no
[levantamento inicial](PORT_UBUNTU_24_04.md).

| Conjunto comparado | Resultado |
| --- | --- |
| `ElgatoDeviceCapabilities.json` | 23 entradas e 40 chaves distintas; objetos JSON idênticos após parse, apesar de bytes e hashes diferentes |
| Fundos MP4 | Sete pares com SHA-256 idêntico; a grafia de CoffeeShop/Coffeeshop difere |
| Firmware `FW_*.bin` | 14 pares com SHA-256 idêntico, incluindo Facecam MK.2 e sua Rev.2 |
| Integração do sistema | Windows usa PE, DirectShow/Direct3D e driver `.sys`; macOS usa Mach-O, frameworks Apple e extensão de câmera |
| Mensagens e nomes de IA | Há textos e chaves NVIDIA também no executável macOS; texto compartilhado não demonstra um backend ativo |

O catálogo comum confirma para a MK.2 os IDs `0fd9:0093` USB2, com preferência
MJPG, e `0fd9:0094` USB3, com preferência YUY2. A Rev.2 tem entradas e firmware
próprios. Compartilhar o nome comercial não autoriza usar firmware ou controles
de outra revisão.

Esses resultados não sustentam a afirmação de que fundos animados ou um texto
de menu são exclusivos do Windows. Diferenças de execução exigem verificar
chamadas, condições de suporte e, depois, uma execução na plataforma correspondente.

## NVIDIA: VFX e AR são integrações distintas

Para os efeitos de vídeo NVIDIA no Windows, a [Elgato exige RTX compatível e
SDK Video Effects](https://help.elgato.com/hc/en-us/articles/7665140128653-Elgato-Camera-Hub-How-to-use-AI-Background-Effects).
O executável examinado contém os avisos `NVIDIA Video Effects SDK required` e
`NVIDIA AR SDK required`, com instruções para instalar o SDK correspondente;
há mensagens separadas para GPU incompatível. Não foi identificado um modo CPU
documentado para esses efeitos. Esta conclusão se restringe a VFX/AR, não a todo
processamento de todos os módulos do aplicativo.

A opção CPU do Facecam Hub é uma alternativa ONNX própria, disponível em
**Opções avançadas** e na CLI para testes e operação explícita. Ela não é um
fallback para os SDKs NVIDIA nem evidencia paridade com o Windows. A UI principal
mantém NVIDIA CUDA como padrão. A segmentação temporal VFX Ada já executa em
Rust/TensorRT nativo, conforme [Análise RTX](ANALISE_RTX.md); AR continua pendente.

O executável contém caminhos separados para localizar **NVIDIA Video Effects**
e **NVIDIA AR SDK**, com carregamento dinâmico de `NVVideoEffects.dll`,
`NVCVImage.dll` e `nvARPose.dll`. O inventário de imports PE não contém essas DLLs como imports
convencionais; as strings e os caminhos de resolução dinâmica são outra evidência.

Foram localizados nomes de APIs e pontos de investigação para:

- GreenScreen: segmentação da pessoa e máscara de fundo.
- BackgroundBlur: efeito de desfoque do SDK.
- `NvCVImage_Transfer` e composição: buffers e transferências de imagem.
- GazeRedirection: correção de direção do olhar, na integração AR.
- FaceTrack: detecção/rastreamento e transições da janela de enquadramento.
- Streams CUDA e parâmetros de imagem, modelo, modo e intensidade.

A decompilação direcionada do executável acrescentou evidência de chamadas,
além dos nomes encontrados em strings. Foram decompiladas 19 funções no Ghidra,
sem autoanálise global; os endereços abaixo pertencem ao `Camera Hub.exe`
identificado pelo hash acima, não à DLL CCUVC:

| Rotina identificada | Endereço | Comportamento estático recuperado |
| --- | --- | --- |
| Teste de disponibilidade VFX | `0x140916040` | Chama o proxy de `NvVFX_CreateEffect` com `GreenScreen`; se a criação retorna zero, chama `NvVFX_Load` e um helper posterior |
| Inicialização de desfoque | `0x141eaa3c0` | Cria `BackgroundBlur` e configura um stream CUDA, com caminhos de erro separados |
| Inicialização de segmentação | `0x141eaa4e0` | Cria `GreenScreen`, configura `Mode`, `CudaGraph=1` e stream CUDA, e chama `NvVFX_Load`; há um ramo com limites de dimensões e streams |
| Inicialização de correção de olhar | `0x141ea68f0` | Cria `GazeRedirection` via `NvAR_Create`, configura diretório de modelos e parâmetros de olhar/temporalidade, e chama `NvAR_Load` |
| Proxy de criação VFX | `0x141eb1b70` | Resolve `NvVFX_CreateEffect` com `GetProcAddress`, guarda o ponteiro e encaminha a chamada; retorna erro quando o símbolo não está disponível |
| Proxy de criação AR | `0x141eb4440` | Resolve e encaminha `NvAR_Create` separadamente do proxy VFX |
| Loader VFX | `0x141eb0d80` | Consulta `NV_VIDEO_EFFECTS_PATH` e `ProgramFiles`, ajusta a busca de DLLs e chama `LoadLibraryW` para `NVVideoEffects.dll` |
| Loader AR | `0x141eb3c80` | Consulta `NV_AR_SDK_PATH` e `ProgramFiles`, com caminho e carga próprios para `nvARPose.dll` |

Os nomes das rotinas de inicialização foram associados por referências às
mensagens de diagnóstico e limites de função do diretório de exceções PE x64.
Os proxies foram conferidos pelas chamadas a `GetProcAddress`. Isso confirma
integrações de código distintas para VFX e AR. O caminho FaceTrack ainda se apoia
em nomes e parâmetros estáticos, sem decompilação dedicada nesta rodada.
Tipos, assinaturas inferidas e registradores omitidos pelo pseudocódigo precisam
ser conferidos no assembly antes de servirem como contrato de implementação.
Os valores concretos de modo, dimensões e sensibilidade dependem de estado em
execução; não foram estabelecidos como padrões ou limites da interface.

O pacote contém `cudart64_12.dll`. Entretanto, `NvidiaMaxineVFXSDKFile` é um
**arquivo de licença de 1.085 bytes**, e não um instalador ou runtime do SDK.
As DLLs VFX/AR citadas não aparecem entre os 249 arquivos extraídos. Mensagens
do aplicativo pedem a instalação separada do SDK e distinguem falta do runtime
de incompatibilidade com a série RTX. A licença MIT desse arquivo não estabelece
a licença dos modelos e runtimes externos da NVIDIA.

Chaves estáticas incluem `nvidiaVfx.EffectMode`, `nvidiaVfx.BlurStrength`,
`nvidiaVfx.BackgroundAssets`, `nvidiaVfx.SelectedBackgroundImage`,
`nvidiaAr.gazeRedirectEffectEnabled` e `nvidiaAr.faceTrackEffectEnabled`.
Também aparecem métodos RPC como `getNvidiaVideoEffectSettings`,
`setNvidiaVideoEffect`, `getNvidiaArEffectSettings` e `setNvidiaArEffect`.
São candidatos de interoperabilidade; nomes não estabelecem schema, enumerações,
porta de rede, autenticação ou serviço em execução.

O link de download do guia Elgato leva ao [NVIDIA Broadcast Download Center](https://www.nvidia.com/en-us/geforce/broadcasting/broadcast-sdk/resources/).
Para RTX 40/Ada, a página associa Camera Hub aos redistribuíveis Video Effects
0.7.6 e AR 0.8.2. Esses são os artefatos selecionados para estudar a integração
Windows; a versão efetivamente carregada por uma instalação precisa ser conferida.
Os arquivos `.exe` e suas DLLs não fornecem um runtime nativo Linux validado.
O [guia dos runtimes NVIDIA](NVIDIA_MAXINE.md) registra os downloads e os critérios
para integrar um backend no Ubuntu. O [fluxo atual de efeitos](EFEITOS.md)
continua usando ONNX Runtime/CUDA.

## Controles específicos: evidência decompilada

O REA identificou 62 DLLs e 3.842 funções no diretório de imports do executável.
Entre elas há 64 imports de `CCUVC.X64.DLL`; a DLL exporta 186 entradas.
Importar um símbolo não prova que a interface o oferece à câmera conectada.

Três getters foram analisados efetivamente pelo REA com Ghidra, sob o perfil
padrão, incluindo corpo, pseudocódigo, chamadas e referências:

| Função | Endereço no PE da DLL | Evidência REA |
| --- | --- | --- |
| `CCUVC_GET_NOISE_FILTER_MODE` | `0x18000db80` | `ev_d77cd6ecc2e719d7…` |
| `CCUVC_GET_HDR_TO_SDR_TONE_MAPPING` | `0x180016610` | `ev_948c0970a050ce0f…` |
| `CCUVC_GET_VIDEO_BRIGHTNESS` | `0x1800143e0` | `ev_89f69a9407cb1b20…` |

O código verifica contexto, identificadores de dispositivo e caminhos de backend
antes de consultar valores. Há chamadas a helpers comuns e uma alternativa
`AT_Get_HDR2SDR_OnOff_Status`. Isso demonstra que esses controles não formam uma
conversão universal para V4L2. Nomes, tipos de estruturas, convenções de retorno
e destinos de chamadas indiretas podem permanecer incompletos sem o PDB original.

Argumentos constantes observados em um helper não são, por si só, uma captura
USB, um selector UVC ou um comando Linux validado. Não foram usados para escrever
em hardware. Antes de implementar um controle proprietário, é necessário seguir
o helper até a interface de transporte, identificar GUID/selector e formato,
confirmar a família do dispositivo e validar primeiro uma leitura real.

## Matriz para a migração completa

A arquitetura continua sendo GUI/CLI → controlador → captura/efeitos, com
worker opcional de IA. Recursos encontrados no Windows não criam outra camada
de execução concorrente à [arquitetura Rust](ARQUITETURA.md).

| Recurso ou integração | Evidência da referência | Estado e próximo contrato no Linux |
| --- | --- | --- |
| Câmera, formato e ajustes UVC comuns | Catálogo e imports de controles | Implementado via V4L2; confirmar por dispositivo |
| Cor e LUT | Manifesto de LUTs e arquivos `.cube` | Implementado com LUT própria/do usuário; recursos Elgato não são distribuídos |
| Desfoque e substituição estática | Caminhos VFX e catálogo de fundos | Implementado com ONNX/CUDA e composição própria |
| Fundo animado | Sete MP4 presentes em ambos os pacotes | Implementado com MP4/MOV/WebM próprio, loop e decoder com memória limitada |
| Backend NVIDIA VFX | API dinâmica e SDK externo | Segmentação temporal m1 Ada em Rust/TensorRT Linux; preparação guiada e fallback ONNX, validado na RTX 4070 SUPER |
| Correção de olhar | Integração AR e GazeRedirection | Não implementado; precisa de modelo/backend e teste de qualidade próprios |
| Enquadramento automático | FaceTrack e parâmetros de transição | Não implementado; definir detecção, suavização, crop e interação com PTZ físico |
| HDR e processamento proprietário | Imports/getters CCUVC | Investigação por família; não equiparar a contraste/denoise genéricos |
| Prompter | XML, textos e componentes DisplayLink | Não implementado; separar monitor, texto, rolagem e transporte do dispositivo |
| NDI | Três DLLs distribuídas no MSI | Alcance funcional não confirmado; não anunciar transmissão NDI existente |
| Automação externa | Nomes de métodos e bibliotecas WebSocket | CLI/IPC local já existe; protocolo de compatibilidade ainda não estabelecido |
| Persistência na câmera e firmware | Arquivos, descritores e imports de atualização | Não implementado; requer protocolo confirmado e recuperação, sem escrita experimental |
| Câmera virtual | INF/SYS Windows e extensão macOS | Linux com v4l2loopback; FFmpeg em 1080p/60 e Chrome em 720p/30 validados localmente, OBS e sessão prolongada pendentes |

A ordem prática é validar a câmera virtual em um consumidor, suportar fundos
animados próprios, definir um backend GPU residente e tratar olhar/enquadramento
como recursos separados. Controles proprietários e firmware seguem por modelo;
a presença de uma função genérica no SDK não confirma suporte na MK.2.

## Evidências locais e limites

O SDK Manager 26.1.0-14392 foi inspecionado por extração local do `.deb` informado
pelo usuário, sem executar o instalador. Seu catálogo público lista GeForce com
alvo Windows e não lista Maxine. A [documentação do SDK Manager](https://developer.nvidia.com/sdk-manager)
descreve os produtos e a instalação por repositório. A instalação assistida no
Facecam Hub segue APT, sem usar o `.deb` de Downloads. Isso não fornece os
redistribuíveis VFX/AR Windows indicados pela Elgato. Sua obtenção e estudo são
separados da instalação do gerenciador. A segmentação VFX Ada tem implementação
Rust/TensorRT no Ubuntu; AR e paridade integral continuam pendentes.

Os resultados completos estão em `package/analysis-windows-2.3.0.7295/`:
`installed-file-map.json`, `msi-file-table.idt`, `msi-properties.idt`,
`platform-comparison.json`, `firmware-comparison.json`, `rea-main-imports.json`,
`rea-ccuvc-imports.json` e `rea-ccuvc_get_*.json`. Estes últimos conservam os IDs
completos, hashes, perfil, limitações e pseudocódigo; a tabela acima usa prefixos.
`analysis-scope.json`, `capabilities-equality.json`, `ai-function-seeds.json`,
`initializer-literals.json`, `loader-literals.json`, `focused-functions.tsv`,
`focused-literals.tsv`, `ghidra-scripts/FocusedCameraHub.java`, `direct-ghidra.log`
e `direct-*.c.txt` registram a análise direcionada do executável. Esses resultados
diretos do Ghidra não são apresentados como evidências emitidas pelo REA.

O inventário REA do diretório achatado examinou 249 entradas e sondou 64 arquivos,
mas não reconheceu um aplicativo instalável por seus nomes de arquivo; isso não
é ausência de uma aplicação. O mapeamento File do MSI e a inspeção PE selecionam
o executável correto. Uma tentativa de `inspect_artifact` nesse diretório foi
recusada porque essa rota exige arquivo ou bundle Apple; não foi apresentada
como inventário válido.

A autoanálise global do executável não foi concluída. O diretório de exceções
PE contém 181.313 entradas RuntimeFunction. As tentativas via REA foram
interrompidas após custo elevado no propagador de constantes x86 e no analisador
de tabelas de endereços; amostras de threads e logs foram preservados. Foram
experimentados o perfil padrão, a exclusão de `x86 Constant Reference Analyzer`
e depois a exclusão adicional de `Create Address Tables`. Nenhuma dessas
tentativas é apresentada como decompilação completa do executável.

Para obter evidência útil com escopo delimitado, o Ghidra foi executado
diretamente com `-noanalysis`, script local, funções selecionadas por endereço
e `-deleteProject`. O script decodifica as instruções e cria as funções
selecionadas no Program temporário antes da decompilação; não executa o PE. Essa cobertura
não equivale ao perfil padrão do REA. Os três getters CCUVC acima foram concluídos
pelo REA/Ghidra com o perfil padrão, sem essas exclusões.

Não houve execução do Camera Hub no Windows, comparação visual entre os três
sistemas, medição do desempenho da NVIDIA no Windows nem demonstração de paridade
integral. Strings, DLLs e arquivos presentes são evidências de distribuição;
chamadas recuperadas acrescentam evidência estática de código. Disponibilidade
real, qualidade, comportamento temporal e protocolos completos continuam exigindo
validação específica.
