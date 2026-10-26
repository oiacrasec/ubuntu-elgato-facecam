# Runtimes NVIDIA usados pelo Camera Hub

A referência de IA do Camera Hub Windows são os redistribuíveis NVIDIA Broadcast
Video Effects e AR para a família da GPU. O Facecam Hub executa a segmentação
temporal VFX 0.7.6 Ada no worker Rust com TensorRT Linux. AR continua pendente.

A [análise RTX e organização dos runtimes](ANALISE_RTX.md) registra os resultados
REA/Ghidra, as versões confirmadas e a separação por plataforma, SDK e família.
O [catálogo documental](nvidia-runtime-catalog.json) contém as duas referências
Ada examinadas; não é um registro de backends Linux instaláveis.

## Pacotes para GeForce RTX 40

O [guia de efeitos da Elgato](https://help.elgato.com/hc/en-us/articles/7665140128653-Elgato-Camera-Hub-How-to-use-AI-Background-Effects)
orienta instalar NVIDIA Video Effects para a série correta de GPU e aponta para
o [NVIDIA Broadcast Download Center](https://www.nvidia.com/en-us/geforce/broadcasting/broadcast-sdk/resources/).
Essa é a origem dos pacotes de referência; SDK Manager é um componente separado.

Na consulta de 25/10/2026, os arquivos correspondentes à RTX 4070 SUPER/Ada são:

| Recurso | Versão de referência | Instalador Windows |
| --- | --- | --- |
| Video Effects, para estudar segmentação e desfoque | 0.7.6 | [nvidia_video_effects_sdk_installer_v0.7.6_ada.exe](https://international.download.nvidia.com/Windows/broadcast/sdk/VFX/nvidia_video_effects_sdk_installer_v0.7.6_ada.exe) |
| AR, para estudar olhar e rastreamento | 0.8.2 | [nvidia_ar_sdk_installer_v0.8.2_ada.exe](https://international.download.nvidia.com/Windows/broadcast/sdk/AR/nvidia_ar_sdk_installer_v0.8.2_ada.exe) |

A página também oferece AR 0.8.7, mas ainda associa Camera Hub ao AR 0.8.2 e
orienta manter essa versão até a atualização do aplicativo consumidor. Essa
associação define os artefatos iniciais da análise; a versão efetivamente usada
por uma instalação específica do Camera Hub deve ser conferida nela.

Os pacotes selecionados usam o sufixo `ada`. Downloads para RTX Spark/ARM64 ou
outras famílias de GPU não substituem esses artefatos. Audio Effects corresponde
a outra integração, fora da análise de vídeo atual.

## Download e evidências locais

Os dois instaladores foram baixados em `/home/cesar/Downloads/NVIDEA/`.
O [registro de validação](VALIDACAO_LOCAL.md) conserva origem, tamanho e SHA-256
dos downloads concluídos. O hash local identifica o arquivo examinado; não é
uma comparação com um digest publicado separadamente pelo fornecedor.

O estudo começa pela extração estática e inventário de DLLs, modelos, manifestos
e versões; REA/Ghidra podem examinar as APIs e o carregamento. Não é necessário
instalar ou executar os `.exe` no Ubuntu para essa inspeção. Hashes, ferramentas
e limites precisam acompanhar cada evidência, sem afirmar execução do efeito
apenas pela presença de arquivos ou símbolos.

APT permanece como preferência para dependências do Ubuntu. Instalar SDK Manager
por APT não instala os runtimes usados pelo Camera Hub. Os instaladores Windows
são artefatos de referência e não devem ser apresentados como dependências
Linux instaláveis pelo gerenciador de pacotes.

## Primeira leitura dos arquivos

As listagens estáticas dos instaladores já identificam componentes distintos:

| Pacote | DLLs e modelos observados |
| --- | --- |
| Video Effects 0.7.6 Ada | `NVVideoEffects.dll`, `NVCVImage.dll`, `cudart64_12.dll` e modelos `AIGS_288x512_89_m*.engine.trtpkg` |
| AR 0.8.2 Ada | `nvARPose.dll`, `NVCVImage.dll`, `cudart64_110.dll`, `nvinfer.dll` e modelos `gazeredir_encoder_fp16_89.engine.trtpkg` / `gazeredir_decoder_fp16_89.engine.trtpkg` |

As extensões e dependências observadas não correspondem ao contrato do fallback
ONNX. Os modelos não devem ser tratados como arquivos `.onnx` como se
fossem intercambiáveis. A compatibilidade dos arquivos e do runtime exige uma
análise própria; a listagem não executa inferência nem demonstra portabilidade.

## Caminho adotado para Ubuntu

A página de downloads lista Windows 10/11 e Windows 11 on Arm como sistemas
suportados. Ela não fornece um redistribuível Linux para GeForce RTX 40.
O caminho escolhido é engenharia reversa dessas referências e implementação
própria em Rust. Buscar ou instalar um SDK NVIDIA Linux não faz parte dessa
etapa. [Análise RTX](ANALISE_RTX.md#direção-adotada) registra a decisão.
DLLs PE Windows não substituem bibliotecas ELF Linux. A execução da segmentação
Ada foi reconstruída e validada localmente; os demais efeitos continuam pendentes.

O usuário confirmou o funcionamento dos recursos no Camera Hub Windows com sua
RTX 4070 SUPER. Esse continua sendo o alvo de aceitação. Os pacotes baixados
permitem investigar o contrato dessa integração, sem prometer que possam ser
usados diretamente pela aplicação nativa Linux.

## Implementação e critérios de aceitação

**Configurações** › **Efeitos NVIDIA RTX** › **Preparar IA RTX** prepara o modelo
e as bibliotecas no usuário e testa uma inferência antes de ativá-los. Não instala
o executável Windows nem usa SDK Manager para esses efeitos. O backend automático
mantém ONNX como alternativa com o mesmo provider. Veja [Efeitos](EFEITOS.md) para
uso e [Análise RTX](ANALISE_RTX.md) para a importação exata e os estados temporais.
Execução local na RTX 4070 SUPER não estabelece suporte de toda Ada nem igualdade
de qualidade com Windows. Os critérios abaixo continuam aplicáveis às extensões.

1. Inventariar os redistribuíveis selecionados: versão, DLLs, modelos, formatos,
   dependências, condições de uso e chamadas observadas no Camera Hub.
2. Recuperar os contratos dos binários Windows e implementar a execução própria
   no Ubuntu; conferir inicialização e inferência reais na RTX deste host antes
   de integrar ao produto.
3. Manter Rust sem `unsafe` no código da aplicação; não esconder bindings próprios
   inseguros em um crate auxiliar.
4. Preservar controlador único, worker isolado, aquecimento antes da troca,
   descoberta automática de recursos e erro explícito sem fallback CPU.
5. Comparar qualidade, resolução, FPS, latência e estabilidade com os cenários
   autorizados do Windows, incluindo consumo pela câmera virtual.

Runtimes e modelos proprietários de referência não são incorporados à distribuição
do Facecam Hub por consequência desse download. Uma integração só pode ser
documentada como implementada após os testes correspondentes.

A implementação limitada ao modelo Ada é descrita em
[Segmentação NVIDIA RTX](ANALISE_RTX.md). Não equivale à execução das DLLs Windows
nem à reconstrução das demais funções VFX/AR; as etapas estáticas acima permanecem
como histórico e critérios para futuras combinações.

## Candidato AR 0.8.7 para execução nativa

A investigação complementar encontrou engines TensorRT 10.4.0.26 no pacote AR
0.8.7 Ada. Os quatro contratos executaram com tensores sintéticos no diagnóstico
Rust `--check-ar`, reaproveitando o runtime nativo preparado para o VFX. A referência
Windows inventariada continua AR 0.8.2; não há efeito de olhar ativo no produto.
[Análise RTX](ANALISE_RTX.md#contratos-ar-087-ada--26102026) registra hashes,
formas, método e limites. O pacote Windows adicional permanece local, sem execução
do instalador ou distribuição de seus pesos/chaves no Git.
