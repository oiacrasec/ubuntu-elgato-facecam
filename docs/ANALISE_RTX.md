# Segmentação NVIDIA RTX no Ubuntu

A segmentação temporal do Video Effects 0.7.6 Ada executa no worker Rust com
TensorRT nativo Linux. Redimensionamento, desfoque e composição executam em CUDA
com grafos próprios, conectados à segmentação por I/O binding.
A combinação foi exercitada na RTX 4070 SUPER; paridade integral com Windows e
suporte às demais famílias ainda não foram demonstrados.

## Direção adotada

A referência é Camera Hub Windows 2.3.0.7295 e seus redistribuíveis NVIDIA
Broadcast. A implementação usa os contratos recuperados desses arquivos e não
exige obter um SDK NVIDIA VFX Linux. CUDA, cuDNN e TensorRT são dependências de
computação, carregadas somente pelo worker isolado.

A pesquisa foi limitada a segmentação, estado temporal, preparação dos tensores
e máscara. Não foi reconstruído todo o SDK. Correção de olhar, relighting,
reenquadramento e AR têm contratos próprios e continuam indisponíveis.

## Escopo necessário à implementação

REA 6.7.0 inventariou as referências PE; Ghidra 12.1.4 examinou funções
selecionadas, com análise global desativada e projetos temporários. Nenhuma DLL
Windows foi executada. As evidências ficam em `package/`, ignorado pelo Git.
O [catálogo](nvidia-runtime-catalog.json) fixa hashes e versões das referências;
é documental e não é o manifesto consumido pelo aplicativo.

No VFX examinado, a criação de `GreenScreen` fica em `0x180013cc0`, a tabela
virtual em `0x1800e4bc0`, `Load` em `0x180015110` e `Run` em `0x180015430`.
A seleção em `0x180015250` contém modos Ada `m0` a `m3`. A entrega atual fixa
somente `AIGS_288x512_89_m1.engine.trtpkg`.

O leitor em `0x180006970` recupera um contêiner `NVnn`, com metadados de 136
bytes após o cabeçalho de 28 bytes, IV de 16 bytes e um bloco AES-256-CBC com
padding PKCS7. `nvidia_model.rs` valida comprimentos e hashes antes de importar.
O material necessário à leitura é obtido da DLL exata do pacote local; o código
não incorpora uma chave nem pesos proprietários. Os originais são preservados.

| Artefato | SHA-256 |
| --- | --- |
| Instalador VFX 0.7.6 Ada | `4761fe1579e82de6d45efec8dc09492cf2cc601867269955e129707ac8ea8546` |
| `NVVideoEffects.dll` | `2a78effae55e9e3791bbe66f312f0bd08a2ede2c94fd8f820933badfa0a6cd8c` |
| Modelo `m1` no pacote | `6636284e5d3d8363c01f83aac90f1ca08ac05952537f38c1e82a39500af87d22` |
| Engine recuperado, antes da adaptação | `518ff60c08e4900e1fe363807b188cff2215d6a9c38962437fc280c826fdf0d6` |

O engine tem 8.773.772 bytes e declara TensorRT 10.4.0.26. A primeira tentativa
Linux foi recusada pelo marcador de plataforma. A comparação na biblioteca
nativa exige `0x74727472` no offset 28; a referência contém `0x74727471`.
A implementação altera esses quatro bytes em uma cópia, somente após conferir
o hash exato e a versão. Essa cópia carregou e executou na GPU local.

Isso é uma adaptação experimental dessa combinação, não um conversor geral.
A [documentação TensorRT](https://docs.nvidia.com/deeplearning/tensorrt/latest/inference-library/engine-compatibility.html)
mantém restrições de plataforma, versão e GPU. O aviso do runtime sobre engines
construídos em outra GPU continua relevante; a prova local não cobre toda Ada.

## Contrato executado pelo worker Rust

O engine confirmou sete entradas e sete saídas, todas float32. `input_1` recebe
NCHW `1×3×288×512`, canais BGR divididos por 255. A ordem BGR segue o descritor
5 observado na inicialização e o enum público
[NvCVImage](https://github.com/NVIDIA-Maxine/Maxine-VFX-SDK/blob/f12bd18929e9065cff4f24cb93ac5f4202dc1c4a/nvvfx/include/nvCVImage.h).
O RGB do aplicativo é redimensionado e reorganizado no worker.

| Entrada de estado | Saída para o próximo quadro | Forma |
| --- | --- | --- |
| `rec_ce.1` | `rec_ce` | `1×64×9×16` |
| `rec1.1` | `rec1` | `1×64×18×32` |
| `rec2.1` | `rec2` | `1×32×36×64` |
| `rec3.1` | `rec3` | `1×16×72×128` |
| `rec4.1` | `rec4` | `1×8×144×256` |
| `rec5.1` | `rec5` | `1×4×288×512` |

`output_1` tem forma `1×2×288×512`; o segundo plano é usado como alpha de pessoa.
A saída é recusada se forma, comprimento ou probabilidades forem inválidos.
No teste preto, a classe de fundo ficou próxima de 1 e a de pessoa próxima de
0; isso não substitui uma bateria de qualidade com pessoas.

O worker zera os seis estados ao iniciar, aquece e zera novamente. Entre quadros,
mantém as saídas temporais na GPU por I/O binding, religando saídas para evitar
sobreposição dos buffers de entrada e saída. A máscara permanece na GPU para
interpolação e composição em `gpu.rs`; apenas o RGB composto e três escalares
de validação retornam para CPU. Reiniciar captura cria sessão e histórico novos.

Os grafos de processamento usam o
[I/O binding do ONNX Runtime](https://onnxruntime.ai/docs/performance/tune-performance/iobinding.html)
para conservar os tensores CUDA entre sessões. Resize bilinear usa coordenadas
`half_pixel`; desfoque usa duas passagens AveragePool, excluindo padding da média,
com truncamento entre passagens. Não há shader ou grafo copiado do aplicativo
de referência. [Desempenho](DESEMPENHO.md) e [Testes](TESTES.md) definem a
comparação com a implementação CPU e os limites das medições.

## Execução e dependências fixadas

`nvidia.rs` usa as APIs seguras de `ort` 2.0.0-rc.9 / API 20. Um pequeno ONNX
`EPContext`, gerado pelo aplicativo, descreve os tensores e referencia o engine
local por caminho relativo. Não é uma conversão dos pesos para uma rede ONNX.
O runtime é ONNX Runtime GPU 1.20.1 com TensorRT 10.4.0.26, conforme a
[matriz do provider](https://onnxruntime.ai/docs/execution-providers/TensorRT-ExecutionProvider.html).
O fallback ONNX Runtime 1.22.0 permanece separado por processo.

`nvidia_setup.rs` faz a preparação solicitada por `PrepareNvidia`. Baixa ou
reutiliza o instalador Windows como arquivo, extrai os componentes necessários,
prepara o modelo e obtém as bibliotecas de computação. Pacotes TensorRT oficiais
são baixados pelo helper APT e extraídos no usuário, sem instalação no dpkg.
Não executa o instalador, não altera fontes APT, driver ou bibliotecas do sistema
e não pede senha. As licenças externas acompanham os componentes locais.
Se `7z`, `curl` ou `unzip` faltarem, o controlador oferece primeiro um plano APT
`nvidia_tools`, com confirmação pelo fluxo existente. A pessoa repete o preparo
após instalar essas ferramentas. Somente essa etapa modifica pacotes do sistema.

O destino é `${XDG_DATA_HOME:-~/.local/share}/facecam-hub/runtimes/linux-x86_64/vfx/0.7.6/ada/`.
Ele contém `manifest.json`, `segmentation.ctx.onnx`, `segmentation.engine`,
`onnxruntime-linux-x64-gpu-1.20.1/`, `compute/` e `cuda/`.
O manifesto consumido fixa formato 1, SDK 0.7.6 e família Ada, com caminhos dos
componentes. Descoberta verifica presença; a preparação e a ativação exigem uma
inferência real. Uma preparação substitui a anterior somente após essa prova,
com staging no mesmo filesystem e restauração se a publicação falhar.
Captura e Quit são recusados durante a preparação.

O cache fica em `${XDG_CACHE_HOME:-~/.cache}/facecam-hub/nvidia-vfx-0.7.6-ada`.
Downloads têm hashes conferidos; apagar o cache não remove os componentes ativos.
Arquivos proprietários, chaves e bibliotecas não entram no Git nem no binário.
Seu uso continua sujeito às respectivas licenças.

## Seleção automática e fallback

A seleção e o fallback de inicialização descritos aqui estão **implementados**.
`effects.ai_backend` aceita `auto` (padrão), `onnx` e `nvidia`.

1. Com CUDA e componentes encontrados, `auto` testa NVIDIA em processo isolado.
2. No sucesso, o worker usa a segmentação temporal NVIDIA.
3. Na falha, tenta ONNX com o mesmo provider e publica o motivo em
   `ai_backend.warning`. Sem componentes, continua pelo ONNX existente.
4. `nvidia` exige a combinação preparada e informa erro em vez de trocar backend.
5. CPU explícita usa ONNX; NVIDIA com CPU é uma configuração inválida.

Não existe fallback silencioso de CUDA para CPU. Falha durante captura encerra
os processos do stream e aparece no controlador; não há troca de modelo no meio
da sessão ou reinício ilimitado. Falha de aquecimento preserva a configuração
confirmada.

## Famílias e limites

| Série GeForce | Compute capability | Estado |
| --- | --- | --- |
| RTX 20 / Turing | 7.5 | Sem modelo importado; ONNX existente |
| RTX 30 / Ampere | 8.6 | Sem modelo importado; ONNX existente |
| RTX 40 / Ada | 8.9 | Preparação implementada; prova na RTX 4070 SUPER |
| RTX 50 / Blackwell | 12.0 | Sem modelo importado; ONNX existente |

As famílias seguem a [tabela NVIDIA](https://developer.nvidia.com/cuda/gpus).
O detector exige compute capability 8.9 para essa combinação. O driver observado
foi 595.99.02, sem determinar o mínimo. AR 0.8.2 permanece somente inventariado,
sem execução reconstruída. Versões VFX e AR ficam em combinações separadas.

[Validação local](VALIDACAO_LOCAL.md) registra procedimentos e medições.
[Testes](TESTES.md) descreve a suíte optativa. Cabelo, movimento, baixa luz,
outras GPUs e comparação lado a lado com Windows continuam critérios separados.
Esta entrega implementa o recurso de fundo; não declara todo Camera Hub migrado.

## Contratos AR 0.8.7 Ada — 26/10/2026

Quatro engines AR foram importados e executados em Linux com entradas sintéticas,
incluindo o caminho encoder→decoder em Rust. Isso comprova os contratos de tensores;
correção de olhar e enquadramento ainda não estão disponíveis no vídeo do produto.
A [central NVIDIA](https://www.nvidia.com/en-us/geforce/broadcasting/broadcast-sdk/resources/)
oferece AR 0.8.7 separadamente da referência AR 0.8.2 indicada para Camera Hub.
O pacote novo é candidato à implementação própria; não é apresentado como uma
reconstrução já concluída do efeito Windows 0.8.2.

O instalador adicional tem SHA-256
`0e2e1110065ed58b9c4a844c674fe5f50ed5fe37a0d170bd229a4bbc48861ca5`.
`nvARPose.dll` tem SHA-256
`90f8b1b4747349651621bc9a991c8f856664843f5315870677c485dbaadcc9a4`.
Os hashes de pacotes e engines estão no [catálogo](nvidia-runtime-catalog.json).
A leitura usa dados derivados da DLL local exata, sem chave ou pesos embutidos.
Os quatro engines declaram TensorRT 10.4.0.26, permitindo reutilizar a combinação
nativa já exercitada pela segmentação. A adaptação do marcador de plataforma é
limitada aos hashes e cabeçalho esperados, preservando os originais.

| Modelo | Entradas float32 | Saídas float32 |
| --- | --- | --- |
| Face detection | `input`: 1×3×208×368 | `output_bbox`: 1×4×13×23; `output_cov/Sigmoid`: 1×1×13×23 |
| Landmarks 126 | `input`: 1×1×80×80 | `kpts`: 1×252; `confidence`: 1×126; `headpose`: 1×3 |
| Gaze encoder | `input_image`: 1×3×64×256 | `pseudo_labels_flat`: 1×12; `embeddings_flat`: 1×1344 |
| Gaze decoder | `embeddings_flat`: 1×1344; `pseudo_labels_flat`: 1×12; `gaze_por`: 1×2 | `gaze_redirected_image`: 1×3×64×256; `gaze_landmarks`: 1×2×12×1 |

`--check-ar` procura o instalador local, verifica SHA-256, extrai somente os
arquivos necessários em um diretório temporário e importa cópias. O runtime vem
da instalação RTX existente, sem download ou instalação de dependências nesse
comando. Um worker isolado carrega descritores EPContext próprios e usa TensorRT/
CUDA com fallback CPU recusado. Confere nomes, tipos, formas e valores finitos.
As saídas do encoder alimentam o decoder; os dados pequenos podem voltar à CPU
nesse diagnóstico. Isso não representa o pipeline de vídeo final sem transferências.

A invocação não abre a câmera, não muda configuração e remove o staging ao terminar:

```bash
./target/release/elgato-virtualcam --check-ar
```

É uma verificação técnica experimental. Não corrige o olhar no preview, não
instala SDK e não comprova qualidade em rostos reais ou taxa de 60 fps.
Os próximos contratos são recorte/normalização, decodificação de detecção,
alinhamento dos olhos, transições e composição GPU compartilhada com o fundo.
A análise estática foi limitada às etapas e helpers correspondentes, com RTTI e
Ghidra sem análise global; as DLLs Windows não foram executadas.
