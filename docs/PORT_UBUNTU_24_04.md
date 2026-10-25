# Camera Hub para Ubuntu 24.04+: levantamento inicial

Análise de 24/10/2026. Objetivo: desenvolver neste repositório uma aplicação
para Ubuntu 24.04+ usando o Camera Hub macOS como referência de comportamento.
Este documento preserva o levantamento anterior à migração. A implementação
Rust atual está descrita em [ARQUITETURA.md](ARQUITETURA.md), com verificações em [TESTES.md](TESTES.md).
A equivalência integral com o Camera Hub continua pendente.

A [investigação do MSI Windows](REFERENCIA_WINDOWS.md) complementa este registro
e documenta a preparação posterior de um Ghidra 12.1.4 compatível com o REA.

## Referência e evidências

- Instalador: `package/CameraHub_2.3.0.7295.pkg`, 387.718.516 bytes.
- SHA-256: `15d91783adf197d356515e604167597853bb1cba77fc73d6485b55d49842dcee`.
- `PackageInfo` e `Info.plist` confirmam Camera Hub **2.3.0**, build **7295**,
  identificador `com.elgato.CameraHub`.
- Executável: `Applications/Camera Hub.app/Contents/MacOS/Camera Hub`.
- SHA-256 do executável universal:
  `89ae4bc9c30c506890484d493e0e5b86d631d0c6f46ef6197020c9d8538c40f9`.
- Extração XAR: `package/camerahub-2.3.0.7295-xar/`.
- Extração gzip/CPIO: `package/camerahub-2.3.0.7295-payload/`.
- Evidências locais: `package/analysis-2.3.0.7295/`, incluindo inventário REA,
  diagnóstico, dependências Mach-O, hashes, controles e formatos V4L2.

As extrações e evidências estão dentro de `/package/`, ignorado pelo Git.
O instalador e seus scripts não foram executados. O 7-Zip reportou 4.060 bytes
após o fim do XAR e concluiu a extração; a assinatura do instalador não foi
validada nesta análise.

Inventário REA observado:

- Inspeção: `ev_e797b56d6d354494ef26fa62320cfadee110c17be6ffa2aa43e96921aa585ca9`.
- Inventário: `ev_7487751b4a85304c381b589d16737c89a8a011d2cff69d38293f18b08fe047bb`.
- Manifesto: 391 nós, 475 ocorrências e 410 arestas no bundle extraído.
- Arquivo completo: `package/analysis-2.3.0.7295/rea-inspect-artifact.json`.

## Projetos vizinhos

### REA

Lidos as orientações, o README, a arquitetura e trechos dos contratos e fluxos
de análise em `../rea` (commit `835904e4e`, versão 6.7.0).

REA separa contratos e evidências, fluxos de aplicação, adaptadores de CLI/MCP
e provedores. Seu bridge Java conecta a análise nativa ao Ghidra headless.
Inventário estático de artefatos funciona sem esse provedor; pseudocódigo e
referências de funções dependem dele. Evidência estática não comprova execução.

A CLI local foi executada com o Node já instalado em
`/home/cesar/.nvm/versions/node/v24.22.1/bin/node`. O diagnóstico confirmou
ausência de um Ghidra configurado. Nenhum servidor REA está disponível nas
ferramentas MCP desta sessão; a inspeção usou a CLI compilada existente.

### Ghidra

Lidos README, propriedades da aplicação e trechos do carregador Mach-O de
`../ghidra` (commit `1640030366`). O carregador contempla Mach-O e binários
universais. A árvore contém o código-fonte do decompilador.

Há uma incompatibilidade de versões a resolver antes da análise de funções:

- O checkout é **12.3 DEV** e declara JDK mínimo **25**.
- O adaptador do REA local admite **12.1.x**, verificado com **12.1.4**.
- O Java padrão encontrado é **21**.
- Não foi encontrada uma distribuição compilada utilizável nos locais
  examinados nem uma configuração `GHIDRA_INSTALL_DIR`.

O caminho documentado pelo REA é fornecer uma distribuição 12.1.x compatível.
Usar o checkout 12.3 exigiria avaliar e validar mudanças no adaptador; não se
deve presumir compatibilidade. Nenhuma instalação ou configuração foi alterada.

## O que o pacote macOS mostra

| Componente observado | Consequência para a versão Linux |
| --- | --- |
| Mach-O universal x86_64 + arm64 | Serve à análise; não é um executável Linux |
| Dependências Qt 6.9.3, incluindo Widgets, Quick e WebEngine | A aplicação Linux usa egui nativa; esses componentes não integram o produto |
| AVFoundation, CoreMediaIO, IOKit, Metal e outros frameworks Apple | As integrações correspondentes precisam de implementação Linux |
| Plugin DAL e `com.elgato.CameraHub.CameraExtension.systemextension` | A saída virtual precisa de um mecanismo Linux, como o v4l2loopback já usado aqui |
| Dependência de libusb e bibliotecas Realtek | Indica áreas para investigação; não comprova o protocolo de cada recurso |
| LUTs, traduções `.qm`, Metal library e configurações de dispositivos | Recursos úteis para mapear funcionalidades, sem comprovar o fluxo de execução |
| Firmware de vários modelos Facecam | Presença no pacote não comprova o protocolo de atualização nem a versão instalada na câmera |

`ElgatoDeviceCapabilities.json`, dentro de `EGAVResources.bundle`, descreve a
Facecam MK.2 USB 2.0 com VID 4057/PID 147 (`0fd9:0093`) e preferência por MJPG.
A variante USB 3 usa PID 148 (`0fd9:0094`) e preferência por YUY2 nesse arquivo.
Esses dados do pacote corroboram a identificação e os formatos observados no
dispositivo Linux conectado.

Strings do executável contêm nomes como `EGAVDeviceUVC` e
`EGAVUVCPropertyDescriptor`. São candidatos para futuras buscas de referências
e decompilação; não estabelecem os bytes ou a semântica dos comandos USB.

## Estado do projeto Linux e do hardware

A base inicial usava configuração JSON, detecção por `v4l2-ctl`, FFmpeg,
v4l2loopback e bandeja PyQt5, sem janela completa de ajustes. Essa implementação
foi substituída por Rust e removida da árvore mantida. O relato abaixo preserva
as observações da investigação inicial, sem descrever o estado atual do produto.

O host observado é Ubuntu 24.04.5. A câmera conectada é **Elgato Facecam MK.2
(USB2)**, `0fd9:0093`, com captura em `/dev/video0` e outro nó em `/dev/video1`.
Os caminhos dos dispositivos devem ser descobertos novamente a cada uso.

Consultas somente de leitura ao driver mostraram:

| Grupo | Controles enumerados |
| --- | --- |
| Imagem | Brilho, contraste, saturação, nitidez e compensação de contraluz |
| Exposição | Automática/manual, tempo de exposição, ganho e framerate dinâmico |
| Cor | Balanço de branco automático e temperatura manual |
| Enquadramento | Pan, tilt e zoom de 100 a 400 |
| Outros | Frequência elétrica de 50/60 Hz e privacidade |

Temperatura e tempo de exposição aparecem inativos enquanto seus modos
automáticos estão habilitados. Enumeração não comprova a aplicação dos valores:
nenhum controle foi alterado e nenhum frame foi capturado nesta etapa.

Formatos enumerados na conexão atual:

- YUYV: 640×360 e 640×480 a 30 fps.
- MJPG: até 1920×1080 a 60 fps; também 960×540 e 1280×720 a 120 fps.
- O padrão anterior era `uyvy422`, 1280×720 a 30 fps. Essa combinação não aparece
  na enumeração. A aplicação Rust já negocia um modo suportado.

## Sequência proposta para implementação

1. **Captura confiável:** descobrir o nó de captura, formatos, resoluções e
   intervalos realmente suportados; escolher uma combinação válida e preservar
   a saída virtual existente. Verificar frames e consumo por um aplicativo.
2. **Controles padrão:** adicionar uma interface para os controles V4L2
   enumerados, respeitando limites, menus e dependências auto/manual. Verificar
   escrita, leitura de retorno e efeito visual na câmera real.
3. **Uso diário:** preview, presets locais, reconexão e mensagens de estado;
   validar no desktop Ubuntu 24.04+, incluindo Wayland e a integração da bandeja.
4. **Recursos específicos:** com Ghidra compatível, rastrear apenas os caminhos
   necessários de UVC/USB para recursos ausentes no V4L2. Persistência na câmera,
   redução de ruído, HDR e atualização de firmware permanecem desconhecidos
   nesta etapa; o comportamento deve ser estabelecido por evidências próprias.
5. **Distribuição:** definir dependências e empacotamento Linux, testar instalação
   e remoção em Ubuntu 24.04 e versões posteriores selecionadas. Compatibilidade
   com essas versões ainda não foi validada.

A primeira entrega funcional sugerida é captura com formato negociado,
preview, controles padrão e saída virtual. Paridade integral com o Camera Hub
permanece um objetivo a detalhar conforme os recursos forem investigados.
