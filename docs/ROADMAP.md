# Roadmap, paridade e prontidão

O alvo é paridade funcional e visual com Camera Hub Windows 2.3.0.7295 em Rust
para Ubuntu 24.04+. Esta página distingue código existente, evidência
local, lacunas e condições de aceitação.

O [ponto de retomada de 26/10/2026](PLANO_DE_CONVERSAO.md#ponto-de-retomada--26102026)
registra commits, arquivos, validações e a sequência para continuar a integração AR.

## Matriz atual

| Dimensão | Estado | Evidência ou próxima verificação |
| --- | --- | --- |
| Descoberta de câmera e modos | Implementado | sysfs, V4L2 e captura física local |
| Controles padrão | Implementado, dependente do dispositivo | Limites, menus, leitura de retorno e recusas observadas |
| Preview | Implementado | Frame completo, tamanho limitado e último frame |
| Saída virtual | Implementado, validação local | Chrome WebRTC real em 1280×720/30, cor, retomada e parada; OBS e longa duração pendentes |
| Preparação da câmera virtual | Implementado, aceitação parcial | Módulo nativo assinado, revisão e persistência; carga real validada, falta reinício e execução administrativa do helper com autorização única |
| Efeitos comuns | Implementado | Cor, LUT, mirror e denoise com teste de pixels |
| IA de fundo | Implementado, paridade parcial | NVIDIA temporal VFX 0.7.6 Ada e fallback ONNX no mesmo provider; qualidade comparativa e AR pendentes |
| Processamento CUDA de fundo | Implementado, validação local | Resize, desfoque e composição na GPU; 1080p/60 com RTX 4070 SUPER, sem eliminar transferências RGB ou filtros CPU |
| Substituição de fundo | Implementado | Cor sólida, três cenários ilustrados e PNG/JPEG/WebP/BMP próprio |
| Fundo animado | Implementado, validação local | MP4/MOV/WebM/MKV próprio em loop; FPS sincronizado e decoder vinculado ao worker |
| Biblioteca de Looks e fundos | Implementado nesta revisão | Nomes, miniatura, LUTs próprias, importação e remoção de imagens gerenciadas, sem campos de caminho |
| Instalação de dependências por APT | Implementado, aceitação parcial | Estado dpkg e logs reais de FFmpeg/V4L2; SDK Manager instalado; remoção administrativa pendente |
| Descoberta de arquivos de IA | Implementado nesta revisão | Diretórios conhecidos, registro do preparador e testes sem `--config` |
| Interface | Implementado, paridade parcial | Layout Camera Hub e validação X11; comparação completa com Windows, Wayland e acessibilidade pendentes |
| CLI/IPC e instância única | Implementado | Contratos em testes de processo |
| Instalação no usuário | Implementado | Bundles versionados, atalhos e ícones próprios, upgrade com processo aberto, remoção sem binário e preservação de dados em prefixos isolados; falta matriz entre releases publicadas |
| Atualização do aplicativo | Implementado | Versão junto de Idioma, consulta em segundo plano e instalação pela branch de produção; [requisitos e recuperação](DISTRIBUICAO.md) |
| Idiomas | Implementado | PT-BR padrão, EN-US, seletor persistido e catálogos JSON extensíveis sem recompilar; [guia](LOCALIZACAO.md) |
| Segurança e distribuição verificável | Parcial | Controles locais presentes; lacunas em Segurança e Distribuição |
| Prompter | Implementado, paridade parcial | Roteiro, leitura, espelho, atalhos, monitor e CLI; dispositivo físico e integrações avançadas pendentes; [guia](PROMPTER.md) |
| HDR da Facecam MK.2 | Implementado, validação local | Unidade e seletor confirmados; escrita/retorno, APT e persistência local; ensaio USB 2 em 1080p30/60; [guia](HDR.md) |
| Firmware e demais controles proprietários | Fora do escopo autorizado | Persistência interna e atualização de firmware adiadas pelo usuário |
| Detecção de rosto | Implementado, interno | Alimenta a correção dos olhos; o controle independente de acompanhar/recortar o rosto foi retirado a pedido do usuário |
| Correção de olhar | Funcional, qualidade recusada no ensaio do usuário | O vídeo real não foi aceito; [revisão do caminho Windows](CORRECAO_DE_OLHAR_WINDOWS.md) identifica lacunas de normalização, controle intermediário e composição por contornos |
| Relighting | Proposto, fora do escopo atual | Presença e alcance na referência ainda precisam de confirmação |
| Plugins, batch, MCP e web | Proposto | Não são recursos da entrega Linux atual |

## O que conta como paridade

O usuário confirmou que a versão Windows funciona com todos os recursos de IA
na sua RTX 4070 SUPER. Esse relato define a referência de aceitação; não é uma
medição Linux nem um benchmark de taxa/resolução. A referência dos componentes
é a linha de redistribuíveis NVIDIA Broadcast para RTX 40/Ada indicada pela Elgato.
As lacunas documentadas são trabalho necessário para atingir o alvo completo,
incluindo SDK VFX/AR, correção de olhar, enquadramento, controles e layout.

Separe presença de controle, comportamento correto, fidelidade visual, operação
contínua, aceitação do consumidor, confiabilidade e distribuição. Uma tela similar
ou todos os botões desenhados não estabelece equivalência funcional. Não existe
uma porcentagem global de paridade calculada neste projeto.

Cada nova evidência precisa indicar versão-alvo, plataforma, hardware, cenário,
resultado e limitações. Comparações com Camera Hub podem usar observação pública
e especificações; recursos proprietários não são copiados. Comportamento observado
no macOS não comprova comportamento no Windows ou suporte Linux.

## Pendências para a conversão completa

O fluxo de captura, fundo RTX e saída virtual já funciona localmente. A conversão
completa ainda depende dos grupos abaixo; não há uma porcentagem global validada.

| Grupo | Trabalho restante |
| --- | --- |
| IA da referência Windows | Corrigir as lacunas do caminho de olhar após recusa de qualidade em vídeo; validar entrada original pareada, movimento, óculos/oclusão e consumidor; o recorte independente foi retirado do produto |
| Famílias RTX | Reconstruir e testar combinações para Turing, Ampere e Blackwell; o caminho temporal nativo atual só foi exercitado em Ada/RTX 4070 SUPER |
| Controles e hardware proprietário | HDR MK.2 implementado; validar USB 3 e demais modelos/revisões antes de habilitar; outros processamentos, persistência interna e firmware continuam adiados |
| Prompter e integrações | Roteiro/rolagem/monitor/CLI implementados; faltam dispositivo físico, Voice Sync, Twitch e biblioteca de roteiros; confirmar o alcance de NDI e da automação antes de prometer compatibilidade |
| Paridade visual e operação | Comparar telas e estados com Windows, completar idioma/acessibilidade e testar sessão Wayland, escalas e bandeja |
| Aceitação e confiabilidade | OBS e navegador nas taxas/resoluções alvo, sessões prolongadas, desconexão/reconexão, suspensão e perda de GPU; conferir módulo e autostart após reiniciar |
| Distribuição | Build reproduzível, pacote e dependências, atualização/remoção e matriz de suporte para Ubuntu 24.04+ |

Para avançar na paridade de IA desta RTX, validar qualidade e comportamento da
correção de olhar em captura e consumidor reais. O detector é uma etapa interna;
o efeito exige aceitação própria. Recursos específicos de outro modelo de câmera ou Prompter
precisam de hardware correspondente para validação. Relighting continua uma
proposta; sua presença e alcance no alvo Windows ainda precisam de confirmação.

## Próximas entregas e gates

A [matriz Windows/macOS](REFERENCIA_WINDOWS.md) detalha as lacunas por recurso,
incluindo fundos animados, VFX, AR, Prompter e controles específicos de câmera.

1. Exercitar saída virtual de ponta a ponta em consumidores selecionados, registrando
   resolução, taxa, cor, retomada e parada. Chrome local validado; concluir OBS,
   longa duração e persistência após reinício.
2. Ampliar a validação da [segmentação temporal Ada implementada](ANALISE_RTX.md):
   qualidade comparativa com Windows, outras GPUs e estabilidade prolongada.
   Preparação guiada, inferência Rust/TensorRT e fallback ONNX de inicialização
   estão implementados. Outras famílias e AR exigem investigações separadas.
3. Validar sessão Wayland, escalas de desktop, bandeja e navegação assistiva.
4. Avaliar qualidade e estabilidade de segmentação com cenas autorizadas, movimento,
   cabelos, baixa luz e custo de CPU/GPU.
5. Reduzir lacunas de confiabilidade: desconexão, modelo inválido, perda de GPU,
   logs, timeout e recuperação sem dano à configuração.
6. Preparar distribuição reproduzível e política de suporte antes de publicação.

Uma entrega só fica concluída quando seus checks correspondentes passam. Não
há cronograma em horas ou data prometida para recursos não investigados.

## Scorecard e manutenção

Um scorecard futuro deve separar casos implementados, parciais e ausentes por
dimensão, com fontes de evidência. Budget de desempenho precisa de máquina e
cenário, sem reduzir o limite para maquiar regressão. O projeto ainda não possui
gerador de scorecard ou baseline com enforcement automático.

Ao fechar uma lacuna, atualizar esta matriz, o documento responsável e as evidências
na mesma alteração. Relatos históricos permanecem datados e não são substituídos
por números melhores obtidos em escopos diferentes. [Decisões](DECISOES.md) registra
motivações; [Validação](VALIDACAO_LOCAL.md) registra resultados.
