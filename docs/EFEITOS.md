# Efeitos de vídeo e IA

Os controles da aba Camera alteram a câmera pelo V4L2. A aba Effects transforma
os frames no computador. Os efeitos são aplicados antes da separação entre
preview e câmera virtual, inclusive quando a janela está na bandeja.

![Aba Effects na interface nativa, com preview pausado](screenshots/facecam-effects.png)

## Efeitos comuns

| Campo JSON | Padrão | Valores |
| --- | --- | --- |
| `mirror` | false | Espelha as duas saídas |
| `brightness` | 0 | -1 a 1 |
| `contrast` | 1 | 0 a 2 |
| `saturation` | 1 | 0 a 3; zero remove cor |
| `denoise` | 0 | 0 a 10; zero desliga o filtro hqdn3d |
| `lut` | vazio | Caminho completo para LUT 3D `.cube` |
| `background_blur` | false | Segmentação de pessoa e desfoque do fundo |
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

## Preparar a IA

Ao abrir `./target/release/elgato-virtualcam`, os campos vazios de modelo e runtime
são preenchidos com arquivos locais disponíveis. Não é necessário `--config`
para usar a instalação padrão ou os arquivos preparados em `target/ai-runtime`.
O aplicativo não baixa arquivos nem ativa IA nessa busca.

![Arquivos de IA detectados na abertura normal](screenshots/facecam-ai-discovery.png)

## Descoberta automática

A prioridade é: caminho já configurado; `FACECAM_AI_MODEL` e `FACECAM_AI_RUNTIME`;
registro do preparador; diretório de dados do usuário; instalação junto do
executável; diretório de desenvolvimento; locais de sistema. Cada campo é
preenchido independentemente e apenas quando vazio. Caminhos manuais inexistentes
continuam visíveis, para que um erro não seja mascarado por outra instalação.

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

Se preparou arquivos com a janela aberta, clique em **Detectar arquivos** na aba
**Effects**, ou use a atualização de dispositivos. Para substituir um caminho
manual por detecção, esvazie esse campo antes. O botão de efeitos mantém os
valores como rascunho até **Aplicar e salvar**.

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
o ambiente da sessão. Os campos são preenchidos automaticamente; ative o desfoque
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

O Facecam Hub implementa sua própria composição com ONNX Runtime/CUDA; não
embute o SDK proprietário, os modelos da NVIDIA ou recursos do Camera Hub.
O primeiro recurso de IA é desfoque de fundo. Troca por imagem, correção de
olhar, reenquadramento automático e relighting não estão implementados.
Denoise comum é um filtro convencional e não usa IA.

Os efeitos de IA acrescentam transferência RGB e inferência por frame. O modelo
é pequeno, mas o runtime CUDA e suas dependências ocupam muito mais espaço que
o aplicativo. Sem IA, esses arquivos não são carregados. A qualidade depende de
iluminação, cabelo, movimento e contraste do fundo; não há garantia de recorte
perfeito. O fundo é desfocado em resolução reduzida para limitar custo.
