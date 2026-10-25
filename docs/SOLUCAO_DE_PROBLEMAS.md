# Solução de problemas

Comece pela mensagem do aplicativo e pelo log ao lado da configuração.
Preserve arquivos e ajustes até identificar a etapa que falhou.

## Modelo ou runtime de IA vazios

1. Execute `./prepare-ai.sh --cuda` ou `./prepare-ai.sh --cpu` se ainda não preparou os arquivos.
2. Encerre a instância antiga e abra `./target/release/elgato-virtualcam`.
3. Abra **Effects**, confira o estado de detecção e clique em **Atualizar** após preparar os arquivos.
4. Se escolheu um diretório personalizado, confira o registro escrito pelo preparador
   ou as variáveis `FACECAM_AI_MODEL` e `FACECAM_AI_RUNTIME`.

A busca usa locais conhecidos, sem varrer o disco inteiro. Campos explicitamente
preenchidos, inclusive caminhos antigos, permanecem preservados no JSON. Com o app
fechado, remova do JSON somente o campo que deseja redetectar. A GUI não pede
caminhos. [Efeitos](EFEITOS.md) explica a ordem.

## IA encontrada, mas recusada ao aplicar

Presença do `.onnx` e da biblioteca não comprova provider, dependências e shape.
Confira o provider selecionado e rode `--check-ai` com uma configuração em que
`background_blur` esteja ativo. O runtime CUDA exige suas bibliotecas compatíveis;
o driver sozinho não as substitui. Um runtime CPU pode ser detectado, mas não
passará na verificação do provider CUDA. Se quiser CPU, selecione-a explicitamente.

Não baixar um modelo arbitrário com nome parecido: o contrato do tensor e da
normalização também precisa corresponder. Inclua a mensagem exata do worker no
diagnóstico, sem enviar o conteúdo de arquivos pessoais.

## A janela não abre ou permanece preta

1. Execute o binário no terminal e guarde a mensagem de erro.
2. Confirme que a sessão possui display gráfico e se é X11 ou Wayland.
3. Teste `--headless` e `--status` em um runtime isolado se precisar distinguir
   falha gráfica de falha do controlador.
4. Registre versão do aplicativo, driver e bibliotecas disponíveis.

O produto não tem build de Windows/macOS ou uma lista universal de correções de
driver. Não apagar configuração ou alterar o driver apenas para ocultar o erro.
Uma janela preta, problema de composição do desktop e câmera sem frames são
sintomas diferentes. O modo headless verifica um subconjunto do aplicativo.

## Câmera ausente, ocupada ou sem frames

Use `--test-camera` e `--list-controls`. Confira conexão, permissões do dispositivo
e se outro aplicativo ocupa a mesma câmera. Feche o consumidor que você reconhece;
não use uma busca genérica para matar todos os processos FFmpeg do sistema.

Escolha um formato enumerado e confira o log de captura. Um status “iniciando”
pode anteceder uma recusa do dispositivo; espere os frames ou o erro posterior.
Nós de metadados não são uma entrada de vídeo utilizável.

## Ajuste manual indisponível ou recusado

Um modo automático pode bloquear um controle manual. Se o driver recusou uma
escrita, o aplicativo conserva o valor lido e impede tentativas repetidas até
atualização explícita. Use a atualização em **Device** para liberar nova tentativa.
Não aumentar o intervalo do slider para contornar uma recusa do firmware.

Backlight na Facecam MK.2 pode ser anunciado como 0–1 e recusar ambos. SAVE exclui
controles recusados; R continua restaurando os demais. Isso não prova que o mesmo
controle funcione em todas as combinações de câmera, firmware e modo.

## Bordas, escala ou texto deslocado

O preview preserva proporção e pode exibir padding. Compare primeiro a resolução
e a proporção do modo com o espaço disponível na janela. Não aumentar FOV ou
alterar pan/tilt apenas para corrigir a geometria de visualização.

Para rótulos ou selects desalinhados, informe tamanho da janela, escala do desktop
e texto exibido. **Input** e **Format** compartilham colunas e largura; textos
longos devem truncar sem deslocar a linha. Uma imagem apenas do card é suficiente
para esse diagnóstico e evita registrar o vídeo da pessoa.

## O consumidor não mostra a câmera virtual

Confirme que o nó configurado pertence a v4l2loopback e que a captura está ativa.
Selecione a câmera virtual no consumidor, não necessariamente a Facecam física.
Se o módulo precisa ser preparado, siga o [README](../README.md), preservando
outros dispositivos em uso. Não descarregue um módulo usado por outro aplicativo.

A saída atual é yuv420p. Validar imagem e taxa no consumidor é uma etapa separada
do preview. Secure Boot, DKMS e permissões do host podem requerer diagnóstico do
sistema; o aplicativo não os modifica automaticamente.

## Configuração inválida e recuperação

Guarde o arquivo original e seu backup antes de editar. A aplicação não regrava
um JSON inválido ao abrir. Um caminho manual inválido permanece visível, para
não mascarar o problema com outro modelo. Corrija o campo pertinente em vez de
excluir todos os presets ou arquivos opcionais.

## Dados para pedir ajuda

Inclua revisão/versão, Ubuntu, sessão gráfica, GPU/driver, modelo da câmera,
USB2/USB3, comando executado, cenário, resultado esperado e mensagem observada.
Use o trecho relevante dos logs e um exemplo sintético quando possível.
Remova nomes pessoais, caminhos privados desnecessários, tokens e imagens da
câmera. Não existe um canal Discord ou SLA de suporte definido por este manual.
