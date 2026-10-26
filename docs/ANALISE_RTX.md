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
e máscara. Não foi reconstruído todo o SDK. A continuação AR abaixo acrescenta
detecção e correção de olhar frontal; relighting continua proposto.

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
naquela etapa, correção de olhar e enquadramento não estavam disponíveis no vídeo.
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

Sem preparação persistente, `--check-ar` procura o instalador local, verifica SHA-256, extrai somente os
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

## Preparar os contratos AR no usuário

Com o controlador ativo e captura parada, execute `--prepare-ar` depois de
preparar a IA RTX. O pedido importa os quatro contratos e testa a GPU antes de
publicar os componentes; não disponibiliza correção de olhar ou enquadramento.

```bash
./target/release/elgato-virtualcam --prepare-ar
./target/release/elgato-virtualcam --status
```

Acompanhe `preparing_nvidia`, `nvidia_progress`, `nvidia_log` e `error` até terminar.
As ferramentas ausentes geram o plano APT `nvidia_tools`, sujeito à confirmação
normal; repita o pedido após instalá-las. O preparo AR reutiliza ONNX Runtime,
TensorRT e CUDA da instalação VFX, sem buscar outro SDK Linux. O instalador AR
vem do cache validado, das pastas Downloads já documentadas ou da URL fixada no
importador, sempre com SHA-256 conferido. O cache fica em
`${XDG_CACHE_HOME:-$HOME/.cache}/facecam-hub/nvidia-ar-0.8.7-ada/`.

O destino é
`${XDG_DATA_HOME:-$HOME/.local/share}/facecam-hub/runtimes/linux-x86_64/ar/0.8.7/ada/`.
Contém `manifest.json`, quatro engines adaptados, quatro descritores próprios,
marcador do aplicativo e as duas licenças extraídas. Não contém a DLL Windows.
O manifesto registra o hash do instalador e a raiz absoluta do runtime VFX
reutilizado; mover ou remover esse runtime exige novo preparo. Engines e
descritores usam nomes e hashes fixados no código. O staging irmão é publicado
somente após o worker isolado executar os quatro contratos com fallback CPU
recusado. O destino anterior é preservado em falhas anteriores à publicação;
erro de rename tenta restaurá-lo. Não há garantia adicional de durabilidade por fsync.

Após o preparo, `--check-ar` usa esses arquivos sem extrair ou procurar o instalador.
Reconfere SHA-256 dos engines e executa os quatro contratos, inclusive
encoder→decoder. Uma preparação existente inválida gera erro e precisa de reparo;
não aciona silenciosamente o caminho temporário. `ar_prepared` no snapshot
confere presença e metadados, sem inferência ou hash dos engines em cada consulta.
Downloads e preparo nunca ocorrem na abertura da aplicação.

A leitura dos helpers `0x1800a81b0` e `0x1800304b0` confirmou a seleção por
confiança e montagem de caixas pelas diferenças entre coordenadas. O primeiro
helper apenas despacha a rotina GPU `0x1800a8090`; sua normalização de coordenadas
ainda não foi fechada. Essa constante permanece pendente antes da integração
da detecção ao vídeo naquele ponto. A continuação a seguir fecha esse contrato.

## Contrato de detecção fechado e recorte digital

A rotina de registro em `nvARPose.dll` associa o stub `0x1800a8090` ao kernel
`_Z14gridbox_kernelPfS_ffi`. A leitura estática focada do PTX comprimido no
fatbin confirmou centros em `16×índice+0,5` e escala 35 das quatro distâncias.
A procedência continua sendo a DLL exata identificada pelos hashes acima.
O processo não executou a DLL Windows nem copiou o shader para o produto.
Evidências locais: `target/ar-face-contract/ghidra.log`,
`direct-face-bbox-launch.c.txt` e `gridbox.ptx.txt` no mesmo diretório ignorado.
O projeto temporário Ghidra foi removido após a análise.

`framing.rs` implementa decodificação e supressão de sobreposições próprias.
O protótipo inicial de recorte foi retirado do produto a pedido do usuário:
detectar o rosto é parte interna da correção dos olhos. `nvidia_framing.rs`
prepara o detector e compartilha a imagem GPU com a correção e com o fundo.
O modo apenas fundo conserva o processamento anterior e não carrega AR.

Os contratos matemáticos e escolhas do aplicativo estão em
[Algoritmos](ALGORITMOS_DE_EFEITOS.md#detecção-e-correção-de-olhar).
[Efeitos](EFEITOS.md#ativar-correção-de-olhar) descreve operação e limites.
As verificações incluem comparação GPU/CPU, imagem sem rosto, fotografia pública
com rosto, composição com fundo e recusa de frame truncado. Isso não demonstra
paridade de qualidade com Windows nem aceitação prolongada na câmera real.
Landmarks e alinhamento dos olhos foram integrados na continuação abaixo.

## Correção de olhar frontal na GPU

`effects.eye_contact` executa detector, landmarks 126, encoder e decoder no mesmo
worker isolado. O input de landmarks é cinza float32 80×80 em [0,255]; as saídas
X/Y agrupadas foram verificadas na fotografia pública de teste. Os índices dos
dois contornos oculares pertencem ao contrato do modelo. O alinhamento usa uma
similaridade 2D própria para olhos em (64,32)/(192,32) no input RGB 256×64 do encoder.
Não usa o template 3D, o alinhador nem shaders proprietários do SDK Windows.

Embeddings e labels ficam na GPU e alimentam o decoder com alvo (0,0). A saída
é recomposta somente em elipses dos olhos, com borda suave e transição de ganho.
O caminho recusa dados não finitos, protege olhos fechados e suspende o efeito
com baixa confiança ou pose acentuada. O vídeo conserva seu enquadramento.
Fundo, preview e saída virtual compartilham o RGB resultante.

O efeito é funcional para o contrato frontal exercitado; qualidade em vídeo real,
óculos, oclusão, iluminação e comparação com Camera Hub exigem aceitação adicional.
A GUI expõe **Correção de olhar** › **Olhar para a câmera** e prepara os componentes
por **Preparar correção de olhar**, sem expor um controle de acompanhamento facial.
Resultados e limites estão em [Validação local](VALIDACAO_LOCAL.md).

A preparação reutiliza diretamente o instalador local validado por hash quando
disponível. Não cria outra cópia de aproximadamente 1,3 GB no cache; um download
para cache acontece somente sem origem local válida. A limpeza de duplicatas
preservou os manifestos e modelos já preparados, conforme o registro local.

## Revisão temporal após relato de tremulação

Em 26/10/2026, o usuário confirmou funcionamento da correção, mas relatou olhos
artificiais e tremulação frente ao Camera Hub Windows. O commit `6cb515b` registra
o estado anterior a esta revisão. A validação em fotografia estática daquele
estado não cobria estabilidade temporal nem comprovava naturalidade em vídeo.

A inspeção estática do inicializador de olhar do Camera Hub 2.3.0.7295,
`0x141ea68f0`, confirmou a configuração de `NvAR_Parameter_Config_Temporal`
com `0xffffffff`, além de `EyeSizeSensitivity`, `Landmarks_Size`, `GazeRedirect`
e `UseCudaGraph`. Evidência local preservada em
`package/analysis-windows-2.3.0.7295/direct-gaze-init.c.txt`. Esses argumentos
confirmam diferenças de integração; não revelam os coeficientes do filtro
interno nem autorizam atribuir um algoritmo específico ao SDK.

Na implementação própria anterior, o alinhamento e a máscara variavam diretamente
com cada medição e fechar os dois olhos reiniciava todo o ganho. A composição
também transformava de volta o recorte inteiro, alterando textura por interpolação
mesmo sem uma mudança de olhar. A revisão filtra geometria e visibilidade por
olho, mantém as piscadas atuais e transforma somente o residual do modelo.
Os contratos e limiares próprios estão em
[Algoritmos](ALGORITMOS_DE_EFEITOS.md#detecção-e-correção-de-olhar).

O ensaio temporal também reproduziu perda do rosto por variação fotométrica
pequena: o pico de confiança caiu de 0,7116 para 0,6444 com brilho e ruído de
um nível por canal. O corte fixo 0,7 desligava o efeito em vez de manter a pessoa
já reconhecida. A revisão acrescenta histerese de aquisição/continuação,
condicionada à sobreposição com o rosto validado, sem reutilizar imagens antigas.
Uma detecção fraca não inicia o efeito e não recupera sozinha um rosto perdido.

O alinhamento continua frontal 2D, diferente do alinhamento 3D do SDK. A aplicação
de referência indica AR 0.8.2 Ada, enquanto o caminho nativo exercitado usa os
modelos AR 0.8.7 Ada fixados. Não foi demonstrada equivalência visual entre
essas versões ou aceitação da revisão na câmera do usuário.

## Revisão do caminho completo após o vídeo do usuário

A gravação fornecida depois de `f9939eb` não foi aceita como resultado natural.
O [levantamento do caminho Windows](CORRECAO_DE_OLHAR_WINDOWS.md) registra padrões
de 68 landmarks e sensibilidade 3 do Camera Hub, os seis estágios dentro do SDK,
o controle de features/GazePOR ausente no Rust, a normalização por pose e a
recomposição que usa contornos gerados. O efeito atual permanece funcional com
qualidade recusada; os ensaios estáticos anteriores não constituem paridade visual.
