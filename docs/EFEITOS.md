# Efeitos de vídeo e IA

Os controles da aba Camera alteram a câmera pelo V4L2. A aba Effects transforma
os frames no computador. Os efeitos são aplicados antes da separação entre
preview e câmera virtual, inclusive quando a janela está na bandeja.

![Cards de Effects no padrão da aba Camera, com preview pausado](screenshots/facecam-effects.png)

**Effects** organiza os ajustes em cards: **Look** para a LUT, **Picture** para
brilho/contraste/saturação, **Processing** para denoise e espelho, e **AI Background**
para fundo, FPS e estado da IA. Clique no título dos cards de parâmetros para
recolher ou expandir. Rótulos, sliders e valores seguem as mesmas colunas da aba
**Camera**. Os detalhes de provider ficam em **Opções avançadas**, inicialmente
recolhido. O botão **Aplicar e salvar** permanece no rodapé durante a rolagem.

## Efeitos comuns

| Campo JSON | Padrão | Valores |
| --- | --- | --- |
| `mirror` | false | Espelha as duas saídas |
| `brightness` | 0 | -1 a 1 |
| `contrast` | 1 | 0 a 2 |
| `saturation` | 1 | 0 a 3; zero remove cor |
| `denoise` | 0 | 0 a 10; zero desliga o filtro hqdn3d |
| `lut` | vazio | Caminho completo para LUT 3D `.cube` |
| `background_blur` | false | Ativa efeitos de fundo com IA; nome mantido por compatibilidade |
| `background_mode` | blur | `blur`, `solid`, `studio`, `office`, `landscape`, `image`, `video` |
| `background_color` | [32,38,48] | Três canais RGB u8 para `solid` |
| `background_image` | vazio | Caminho interno de PNG, JPEG, WebP ou BMP para `image` |
| `background_video` | vazio | MP4/MOV/WebM/MKV local não vazio, até 4 GiB, para `video` |
| `ai_fps` | 30 | 1 a 60; limite da saída virtual com IA |
| `blur_radius` | 12 | 1 a 32, medido no fundo reduzido a 256×256 |
| `ai_provider` | cuda | `cuda` ou `cpu`, escolha explícita |
| `ai_model` | detectado quando disponível | Caminho completo para modelo ONNX |
| `ai_runtime` | detectado quando disponível | Caminho completo para `libonnxruntime.so` |

Valores não finitos e fora do intervalo são recusados. Na GUI, altere os campos
e clique **Aplicar e salvar**. Um arquivo de efeitos para CLI contém somente
os campos da tabela; os demais assumem o padrão:

```json
{
  "brightness": -0.05,
  "contrast": 1.1,
  "saturation": 1.05,
  "denoise": 2.0,
  "mirror": false,
  "lut": "/home/usuario/Imagens/minha-lut.cube"
}
```

Com a GUI ou controlador headless em execução:

```bash
./target/release/elgato-virtualcam --set-effects effects.json
```

O pedido `set_effects` também está disponível no socket. Ele valida os valores,
verifica os arquivos e, quando IA está ativa, executa um aquecimento real antes
de interromper a captura atual. Aplicar reinicia as saídas que estavam ativas.
A configuração é salva no controlador, mesmo se o CLI usar outro `--config`.
As ações **Restaurar efeitos padrão** e **Aplicar e salvar** ficam no rodapé fixo
do painel; abrir opções avançadas não oculta o botão de aplicar.

O provider padrão é NVIDIA CUDA. A opção **CPU (alternativa)** fica dentro de
**Opções avançadas** e usa ONNX Runtime. Essa alternativa não reproduz o contrato
dos efeitos NVIDIA VFX/AR do Camera Hub Windows, que exigem RTX e seus SDKs.
Nenhuma falha de CUDA muda automaticamente para CPU.

## Escolher o fundo e a taxa de processamento

Ative **Ativar fundo com IA**. O seletor oferece desfoque, cor sólida, três
cenários ilustrados locais, imagem própria e vídeo próprio. Os cenários não são fotos nem
recursos extraídos de outro aplicativo. A composição usa a mesma máscara de
pessoa, antes do split entre preview e saída virtual. Imagens estáticas são
decodificados uma vez por worker, redimensionados para preencher a saída com
recorte central e mantidos em memória; não são lidos novamente por frame.

Aceitamos PNG, JPEG, WebP e BMP, até 32 MiB, dimensões até 8192×8192 e orçamento de
decodificação de 128 MiB. Imagem ausente ou inválida falha no aquecimento antes
de substituir a configuração ativa. Use imagens locais em que confia.

**Adicionar imagem…**, **Adicionar vídeo…** e **Adicionar LUT…** abrem o seletor
do desktop, sem bloquear a captura. A interface mostra nomes e miniatura do fundo,
sem campos de caminho. A importação valida e copia o recurso para a biblioteca
do app em segundo plano; um arquivo recusado não altera a configuração salva.
O arquivo importado fica selecionado no rascunho até **Aplicar e salvar**.
Para vídeo, escolha **Meu vídeo** e um MP4, MOV, WebM ou MKV local. A reprodução é em
loop, sem áudio, com recorte central para preencher o quadro. Um decoder FFmpeg
por worker fornece RGB na taxa efetiva de processamento, inclusive no preview
limitado a 15 fps e em modos fracionários. O pipe aplica contrapressão; o arquivo
inteiro não é carregado em memória. Mídia inválida é recusada no aquecimento.
O decoder é recolhido no encerramento normal e recebe SIGKILL se o worker morrer.

`ai_fps` limita a taxa processada da saída virtual, com padrão 30. É independente
do modo anunciado pela câmera; com entrada 1080p60 e limite 30, a saída com IA é
1080p30. Sem IA, a taxa negociada é preservada. Quando só há preview, a redução
para 640×360 e até 15 fps ocorre antes do worker. O rótulo **Input Format** mostra
a entrada. Um limite de 60 não garante que o hardware processe 60 frames por segundo.

Para privacidade, uma substituição opaca esconde mais detalhes que desfoque.
A segmentação continua sujeita a falhas; nenhuma opção garante recorte perfeito.

## Preparar a IA

Ao abrir `./target/release/elgato-virtualcam`, modelo e runtime
são detectados nos locais conhecidos, sem edição de paths na GUI. Não é necessário `--config`
para usar a instalação padrão ou os arquivos preparados em `target/ai-runtime`.
O aplicativo não baixa arquivos nem ativa IA nessa busca.

## Descoberta automática

A prioridade é: caminho já configurado; `FACECAM_AI_MODEL` e `FACECAM_AI_RUNTIME`;
registro do preparador; diretório de dados do usuário; instalação junto do
executável; diretório de desenvolvimento; locais de sistema. Cada campo é
preenchido independentemente e apenas quando vazio. Caminhos manuais inexistentes
continuam preservados no JSON, para que um erro não seja mascarado por outra instalação.

O diretório padrão é `${XDG_DATA_HOME:-~/.local/share}/facecam-hub/ai`. O preparador
registra também instalações feitas com `--directory` em
`facecam-hub/ai-installations.json` no diretório de dados. O registro mantém
entradas CPU e CUDA, sem modificar `config.json`.

Junto do executável, a busca inclui `ai/`, `lib/`, `share/facecam-hub/ai` e
`ai-runtime/` nos níveis de instalação conhecidos. Na árvore de desenvolvimento,
isso encontra `target/ai-runtime/` mesmo quando o executável é chamado de outra
pasta. Há pistas relativas ao diretório de trabalho e em `/usr/local`, `/usr`
e `/opt/onnxruntime`. Não há varredura recursiva do disco ou da pasta pessoal.

O modelo conhecido é `selfie.onnx`, com alternativas de subpasta documentadas
no módulo `ai_paths`. O runtime pode estar em `lib/` ou em um pacote
`onnxruntime-*`. São aceitos o link `libonnxruntime.so`, `.so.1` e arquivos
versionados. Dentro do mesmo local, pacotes do provider selecionado têm
preferência, depois versões mais novas. Outro pacote disponível pode preencher o
campo sem alterar o provider: encontrar runtime CPU não faz CUDA passar a usar CPU.

Se preparou arquivos com a janela aberta, clique em **Atualizar** na aba
**Effects**, ou use a atualização de dispositivos. Configurações antigas com
caminhos explícitos continuam compatíveis; para voltar à detecção, remova esses
campos do JSON com o app fechado. A GUI apresenta disponibilidade, sem caminhos editáveis.

## Biblioteca de Looks e fundos

O seletor **Look** oferece **Original** e três LUTs próprias: **Quente**, **Frio** e
**Contraste suave**. São transformações originais geradas localmente quando aplicadas;
não são LUTs extraídas do Camera Hub. Arquivos `.cube` importados devem ser texto
UTF-8 até 16 MiB, com `LUT_3D_SIZE` de 2 a 65, exatamente N³ cores RGB finitas e
domínio válido. Valores negativos e acima de 1 são aceitos; NaN e infinito são recusados.
LUTs 1D e combinações com shaper 1D não são aceitas.

A biblioteca gravável fica em `resources/` ao lado do `config.json`, com subpastas
`luts/`, `images/` e `videos/`. A descoberta também consulta
`${XDG_DATA_HOME:-~/.local/share}/facecam-hub/resources`, `resources/` junto do
executável, `/usr/local/share/facecam-hub/resources` e `/usr/share/facecam-hub/resources`.
Arquivos selecionados em configurações antigas aparecem pelo nome. A varredura
não percorre a pasta pessoal: lê até 256 entradas por subpasta, publica até 128
recursos e limita o catálogo a 64 KiB estimados para preservar a resposta IPC.
Use **Atualizar** após instalar recursos externamente.

A importação usa uma cópia temporária oculta, valida o conteúdo e publica sem
sobrescrever arquivos existentes. Nomes repetidos recebem um sufixo. Falhas removem
a cópia parcial; o original é preservado. Não há exclusão automática, deduplicação
por conteúdo ou sincronização com o arquivo de origem. Miniaturas usam o primeiro
frame local via FFmpeg, fora da thread gráfica; não são uma reprodução do vídeo.

Descoberta comprova presença, não compatibilidade ou integridade do binário nativo.
A validação por inferência continua ocorrendo ao ativar IA e por `--check-ai`.
O usuário precisa selecionar CPU explicitamente se esse for o provider desejado.

## Modelo e runtime compatíveis

A implementação usa **ONNX Runtime 1.22.x**, acessado por `ort` 2.0.0-rc.10.
O modelo de referência compatível é a conversão ONNX da segmentação Selfie
MediaPipe, revisão `be49485c8e027524be38591817fc5cd31bd9d00e`:

- [Modelo e licença Apache-2.0](https://huggingface.co/onnx-community/mediapipe_selfie_segmentation).
- [Runtime oficial 1.22.0](https://github.com/microsoft/onnxruntime/releases/tag/v1.22.0).
- [Dependências CUDA e cuDNN](https://onnxruntime.ai/docs/execution-providers/CUDA-ExecutionProvider.html).

Execute o preparador para baixar os arquivos em um diretório local do usuário,
sem sudo, sem instalar Python e sem modificar o driver:

```bash
./prepare-ai.sh --cpu
# Ou, para NVIDIA CUDA (download e espaço em disco significativamente maiores):
./prepare-ai.sh --cuda
```

O preparador atende Linux x86_64 e exige `curl`, `jq`, `tar` e `unzip`. As versões são fixadas; o script
confere SHA-256 e preserva as licenças dos pacotes baixados. Use `--directory`
para outro diretório. Ao terminar, ele informa o arquivo de efeitos criado e
os comandos para abrir a aplicação e aplicar os efeitos. As bibliotecas CUDA
preparadas nessa estrutura são localizadas pelo worker; não é preciso alterar
o ambiente da sessão. Modelo e runtime são detectados automaticamente; ative o fundo com IA
e aplique na UI, ou use o JSON pela CLI.

O contrato do modelo é um único tensor RGB float32 NCHW, batch 1 (ou batch
dinâmico), 3×256×256, com pixels divididos por 255. A saída é uma máscara de
pessoa 256×256, entre 0 e 1. Não use um modelo arbitrário com os mesmos nomes:
normalização e significado do alpha também precisam corresponder.

Para uma configuração manual, acrescente `effects` ao arquivo principal:

```json
{
  "effects": {
    "background_blur": true,
    "ai_provider": "cuda",
    "ai_model": "/caminho/selfie.onnx",
    "ai_runtime": "/caminho/onnxruntime/lib/libonnxruntime.so"
  }
}
```

Valide antes de abrir captura:

```bash
./target/release/elgato-virtualcam --config config-ai.json --check-ai
```

O comando carrega bibliotecas e modelo, registra o provider escolhido, faz uma
inferência e verifica a máscara. O provider CUDA precisa das bibliotecas CUDA
12 e cuDNN 9 compatíveis com esse runtime. O driver sozinho não fornece todas
essas bibliotecas. Se CUDA não estiver disponível, há erro; CPU só é usada
quando selecionada. Parte de operadores auxiliares pode ser executada pelo
runtime em CPU; registro CUDA não significa execução de todo o grafo na GPU.

## Relação com Camera Hub e limites

A [documentação Elgato](https://help.elgato.com/hc/en-us/articles/7665140128653-Elgato-Camera-Hub-How-to-use-AI-Background-Effects)
descreve efeitos de fundo baseados no NVIDIA Video Effects SDK em RTX. A NVIDIA
também publica um [SDK Linux](https://docs.nvidia.com/maxine/vfx/latest/LinuxVFXSDK/InstalltheVFXSDK.html),
com componentes distribuídos pelo NGC e uma lista própria de GPUs suportadas.
Esse caminho não assegura suporte oficial à RTX de desktop no Ubuntu.

Na consulta de 25/10/2026, a instalação Linux publicada pela NVIDIA exige chave
NGC para os recursos e lista GPUs de datacenter, incluindo L4/L40, sem listar
a GeForce RTX 4070 SUPER deste host. Compatibilidade de Compute Capability não
comprova suporte, acesso aos modelos ou funcionamento do SDK nessa GPU. Não há
integração Maxine neste binário; uma integração futura precisa ser validada com
o Core, recursos, licença e GPU efetivamente disponíveis.

O Facecam Hub implementa sua própria composição com ONNX Runtime/CUDA; não
embute o SDK proprietário, os modelos da NVIDIA ou recursos do Camera Hub.
Desfoque, cor sólida, cenários locais e troca por imagem estão implementados.
Correção de olhar, reenquadramento automático e relighting não estão implementados.
Denoise comum é um filtro convencional e não usa IA.

Os efeitos de IA acrescentam transferência RGB e inferência por frame. O modelo
é pequeno, mas o runtime CUDA e suas dependências ocupam muito mais espaço que
o aplicativo. Sem IA, esses arquivos não são carregados. A qualidade depende de
iluminação, cabelo, movimento e contraste do fundo; não há garantia de recorte
perfeito. O fundo é desfocado em resolução reduzida para limitar custo. Coordenadas
e buffers de composição são reutilizados; a interpolação horizontal ocorre uma
vez por linha do mapa reduzido. Isso não equivale a um compositor inteiramente GPU.
