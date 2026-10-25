# UI, interação e acessibilidade

A janela segue a organização do Camera Hub: coluna de controles à esquerda e
preview à direita. Este é o contrato visual do produto para Linux.

## Estrutura, temas e componentes

O tema atual usa fundo escuro, cards de controles, texto claro e ações em azul.
As constantes de cor ficam no início de `gui.rs`. Não existe seletor de temas,
catálogo de tokens em crate separado ou biblioteca própria de widgets. Ao ampliar
a UI, centralize novos valores em vez de espalhar estilos em cada callback.

**Camera** reúne **Device**, **Frame**, **Picture**, **Exposure**, **White Balance**
e **Processing**. **Effects** reúne processamento local e IA. **Prompter** aparece
desabilitado; isso não comprova integração com o dispositivo.

O card **Device** usa uma grade de duas colunas. Rótulos compartilham uma coluna
fixa, e **Input** e **Format** têm a mesma largura disponível. Nomes longos são
truncados no controle, sem mudar a geometria das linhas. Listas precisam permitir
identificar a opção completa ao escolher. Não altere a largura de um campo em
função do texto selecionado.

Em novos componentes, prever foco visível, uso por teclado, unidades legíveis e
alvos clicáveis adequados. Cores não devem ser a única indicação de erro ou de
estado. O frontend compila com AccessKit; isso não equivale a uma auditoria de
leitor de tela ou cobertura integral de acessibilidade.

## Estados e comandos

Os selects de dispositivo e formato ficam indisponíveis durante a captura.
Controles manuais obedecem flags do driver e recusas observadas. A mensagem
**Indisponível nesta configuração** deve explicar a causa e permitir uma nova
tentativa por atualização explícita. Um campo desabilitado não deve mudar o
alinhamento dos demais.

A GUI altera rascunhos e envia comandos ao controlador. Sliders V4L2 confirmam
valores pelo caminho de escrita e leitura de retorno. Efeitos só são aplicados
por **Aplicar e salvar**, que pode reiniciar a captura. O botão de detecção de
arquivos preenche campos vazios sem ativar a IA ou executar inferência.

O botão **Mirror** abaixo do preview é local. O espelho em **Effects** transforma
preview e saída virtual. **Grid** desenha uma sobreposição de visualização. A
saída para outro aplicativo não deve receber acidentalmente grade ou controles.

## Diálogos, confirmação e cancelamento

A janela atual de **Configurações** edita um rascunho e envia `Configure` ao
salvar. O produto não tem um gerenciador geral de pilha modal, schemas de diálogos,
histórico de desfazer ou controles de todas as janelas por IPC.

Ao acrescentar um diálogo, definir em um lugar seus caminhos de abrir, aplicar,
confirmar, cancelar e restaurar. Validar antes de fechar; valores inválidos
permanecem editáveis. Um diálogo filho deve ser resolvido antes da confirmação do
pai. Não usar chaves privadas da UI como parâmetros públicos de comandos.

Tab e Shift+Tab precisam seguir uma ordem previsível. Enter só deve confirmar
quando a ação está válida; Esc cancela o nível apropriado, respeitando popups
abertos. O fechamento e o clique fora não devem provocar gravação implícita.
Esses são critérios para novos diálogos; não anunciam atalhos personalizados já
implementados. Corpos longos precisam de rolagem com ações alcançáveis.

Um preview provisório futuro deve poder ser cancelado sem mudar a configuração
confirmada. Cancelar não pode deixar um worker antigo publicando frames sobre
uma revisão nova. **Restaurar efeitos padrão** prepara um rascunho; aplicar é
uma ação separada e explícita.

## Menus, ferramentas e navegação

Menus de contexto futuros devem despachar as mesmas operações do núcleo que
botões e atalhos. Mantenha ações indisponíveis reconhecíveis e consistentes,
sem criar versões com parâmetros ou efeitos diferentes para cada superfície.

O produto não possui ferramentas de pintura, seleção manual, caneta, paths,
formas, handles de transformação, snapping, réguas ou workspace dockável. Se
máscaras manuais forem acrescentadas, coordenadas de câmera, imagem e janela
precisam de transformação explícita. FOV, zoom V4L2 e ampliação visual são
conceitos distintos. Gestos não podem modificar a câmera enquanto a pessoa
estiver apenas inspecionando pixels.

## DPI, fontes e validação

Use as dimensões lógicas de egui. Teste escala de desktop, janela mínima e textos
mais longos, sem substituir DPI por cálculos com resolução física do monitor.
Não adicione uma barra de título redundante ou prometam comportamento nativo de
outra plataforma sem testá-lo. O foco atual é Ubuntu.

Verifique mudanças visuais na janela real ou em renderização equivalente. Compare
bordas, largura, baseline, rolagem e estados ativo/desabilitado. Capturas devem
incluir somente nossa janela; se a captura de câmera estiver ativa, recorte para
os controles ou obtenha autorização para registrar a imagem. Há evidências em
[Validação local](VALIDACAO_LOCAL.md). Para idiomas e scripts, veja [Localização](LOCALIZACAO.md).

## Matriz de hardware

A validação atual envolve câmera UVC, GPU de inferência e desktop X11. Uma futura
matriz deve separar GPU do frontend, GPU de processamento, limites de textura,
múltiplos monitores, perfil de cor e sessão gráfica. Renderizar com OpenGL não
comprova execução CUDA ou suporte HDR.

Tablets com pressão/tilt/rotação, eraser, touch, gestos de trackpad, mouse 3D,
MIDI, Surface Dial, Touch Bar e scanners WIA/TWAIN não têm integrações dedicadas
no produto. Uma nova entrada deve definir mapeamento, permissões, fonte de
coordenadas e teste real; suporte genérico de ponteiro não comprova esses recursos.
