# Roadmap, paridade e prontidão

O alvo é uma experiência de controle de Facecam para Ubuntu 24.04+, com a
organização do Camera Hub. Esta página distingue código existente, evidência
local, lacunas e condições de aceitação.

## Matriz atual

| Dimensão | Estado | Evidência ou próxima verificação |
| --- | --- | --- |
| Descoberta de câmera e modos | Implementado | sysfs, V4L2 e captura física local |
| Controles padrão | Implementado, dependente do dispositivo | Limites, menus, leitura de retorno e recusas observadas |
| Preview | Implementado | Frame completo, tamanho limitado e último frame |
| Saída virtual | Implementado; aceitação parcial | Sink sintético verificado; falta consumidor OBS/navegador local |
| Efeitos comuns | Implementado | Cor, LUT, mirror e denoise com teste de pixels |
| IA de fundo | Implementado, validação local | ONNX CPU/CUDA; falta bateria de qualidade em cenas diversas |
| Descoberta de arquivos de IA | Implementado nesta revisão | Diretórios conhecidos, registro do preparador e testes sem `--config` |
| Interface | Implementado, validação X11 | Card Device alinhado; Wayland e acessibilidade precisam de validação real |
| CLI/IPC e instância única | Implementado | Contratos em testes de processo |
| Instalação no usuário | Implementado | Scripts e prefixos; falta matriz de upgrade entre releases publicadas |
| Idiomas | Parcial | Português e rótulos em inglês; sem catálogos ou seletor |
| Segurança e distribuição verificável | Parcial | Controles locais presentes; lacunas em Segurança e Distribuição |
| Prompter, HDR e firmware proprietário | Fora do escopo atual | Necessitam investigação e contratos próprios |
| Troca de fundo, relighting, eye contact | Proposto | Sem implementação ou estimativa validada |
| Plugins, batch, MCP e web | Proposto | Não são recursos da entrega Linux atual |

## O que conta como paridade

Separe presença de controle, comportamento correto, fidelidade visual, operação
contínua, aceitação do consumidor, confiabilidade e distribuição. Uma tela similar
ou todos os botões desenhados não estabelece equivalência funcional. Não existe
uma porcentagem global de paridade calculada neste projeto.

Cada nova evidência precisa indicar versão-alvo, plataforma, hardware, cenário,
resultado e limitações. Comparações com Camera Hub podem usar observação pública
e especificações; recursos proprietários não são copiados. Comportamento observado
no macOS não comprova comportamento no Windows ou suporte Linux.

## Próximas entregas e gates

1. Exercitar saída virtual de ponta a ponta em consumidores selecionados, registrando
   resolução, taxa, cor, retomada e parada.
2. Validar sessão Wayland, escalas de desktop, bandeja e navegação assistiva.
3. Avaliar qualidade e estabilidade de segmentação com cenas autorizadas, movimento,
   cabelos, baixa luz e custo de CPU/GPU.
4. Reduzir lacunas de confiabilidade: desconexão, modelo inválido, perda de GPU,
   logs, timeout e recuperação sem dano à configuração.
5. Preparar distribuição reproduzível e política de suporte antes de publicação.

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
