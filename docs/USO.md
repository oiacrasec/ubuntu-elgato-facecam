# Usar o Facecam Hub

Abra `./target/release/elgato-virtualcam` e clique em **Iniciar preview**.
A aba **Camera** controla o hardware; **Effects** processa o vídeo no computador.

## Instalar e abrir

Siga os comandos de dependências e build no [README](../README.md). O instalador
`./install-desktop-app.sh` adiciona o binário e lançador no usuário. Para usar
sem janela, execute `./target/release/elgato-virtualcam --headless`.

A aplicação tem uma instância por runtime do usuário. Abrir novamente traz a
janela existente; um binário recompilado só substitui o código em execução após
encerrar e reabrir o aplicativo. Fechar a janela com bandeja mantém o controlador;
use **Encerrar aplicativo** em Configurações ou `--quit` para sair por completo.

## Instalar dependências com confirmação

Abra **Configurações** e escolha **Instalar FFmpeg e V4L2…** ou
**Instalar NVIDIA SDK Manager…**. O aplicativo apresenta os comandos e qualquer
cadastro de repositório. Somente **Confirmar instalação** inicia o processo;
**Cancelar** descarta o plano. O Ubuntu solicita autenticação administrativa,
e o aplicativo não recebe a senha. Aguarde a conclusão antes de encerrá-lo.

Configurações abre centralizado na janela. Os dois botões de dependências
compartilham uma linha e têm a mesma largura. O app consulta o dpkg ao abrir,
ao atualizar, a cada cinco segundos e ao concluir uma operação, inclusive com
captura ativa. Pacotes já instalados exibem **Desinstalar FFmpeg e V4L2…** ou
**Desinstalar NVIDIA SDK Manager…**; passe o ponteiro para conferir as versões.
Uma instalação parcial mantém **Instalar…** para completar os pacotes ausentes.

**Desinstalar…** apresenta os pacotes que o APT removeria e exige **Confirmar
desinstalação**. A remoção afeta as ferramentas do sistema usadas por outros
aplicativos. O app não executa `autoremove` ou `purge` e recusa uma simulação que
inclua outros pacotes. Pare preview e saída virtual antes de remover FFmpeg/V4L2.
A simulação é conferida novamente antes da execução; o estado final é verificado
no dpkg. Configurações, biblioteca e repositório NVIDIA são preservados.

O SDK Manager é instalado por `apt-get install sdkmanager`. Se ainda não houver
candidato APT, o cadastro assistido usa o repositório oficial NVIDIA para Ubuntu
24.04 x86-64, com chave de assinatura verificada e restrita a essa fonte.
O `.deb` em Downloads não é usado. Para outras versões do Ubuntu, configure
um repositório compatível antes de preparar o plano. O estado mostra o caminho
do log, que também registra falha ou cancelamento da autenticação.

SDK Manager e Maxine são componentes distintos: instalar o gerenciador não
habilita correção de olhar nem substitui os runtimes/modelos VFX e AR Linux.
A [referência Windows](REFERENCIA_WINDOWS.md) registra essa dependência.

## Escolher câmera e formato

1. Pare a captura se estiver ativa.
2. Na aba **Camera**, card **Device**, escolha **Input** e **Format**.
3. Use atualização de dispositivos se conectou ou trocou a câmera.
4. Clique em **Iniciar preview** e confira a imagem.

Resoluções e taxas dependem do modo e da conexão. Um nome de câmera igual não
assegura formatos iguais em USB2 e USB3. Não force uma combinação que não aparece
na lista. [Solução de problemas](SOLUCAO_DE_PROBLEMAS.md) ajuda quando a câmera não aparece.

## Ajustar imagem e enquadramento

Use os controles disponíveis em **Frame**, **Picture**, **Exposure** e
**White Balance**. Pan, tilt e zoom pertencem ao hardware. O driver determina
mínimo, máximo, passo e valores de menu. Modo automático pode tornar um ajuste
manual indisponível. Tempo de exposição e ganho têm as unidades indicadas na UI.

**SAVE** guarda formato e controles disponíveis neste computador. **R** restaura
os controles salvos que a câmera aceitar naquela configuração. O aplicativo não
promete memória interna, presets de firmware ou equivalência entre câmeras.

## Alterar cor e efeitos

Nas configurações, **Câmera virtual** também lista os dispositivos detectados pelo
nome. O controlador usa a saída configurada quando disponível ou a primeira
v4l2loopback encontrada; nenhum campo de caminho precisa ser preenchido. Se não
houver saída, prepare o módulo conforme [o guia de instalação](../README.md)
e clique em **Atualizar dispositivos**. A descoberta não carrega o módulo.

1. Abra **Effects**.
2. Escolha uma LUT em **Look**, ajuste brilho, contraste e saturação em **Picture**,
   ou denoise e espelho em **Processing**.
3. Clique em **Aplicar e salvar**.
4. Confira o estado e o resultado no preview; a captura pode reiniciar brevemente.

Saturação zero remove a cor. Restauração dos efeitos padrão prepara um rascunho;
o botão de aplicar confirma a alteração. LUTs e efeitos são locais, preservando
os controles V4L2 como uma etapa distinta. [Efeitos](EFEITOS.md) lista valores e arquivos.
Os cards seguem o padrão da aba **Camera** e podem ser recolhidos pelos títulos
para liberar espaço. Rótulos ficam à esquerda e valores numéricos à direita.

**Original**, **Quente**, **Frio** e **Contraste suave** estão disponíveis no seletor.
Para usar outro Look, clique em **Adicionar LUT…** e escolha o arquivo no diálogo
do Ubuntu. O app valida a LUT e guarda uma cópia na biblioteca; você não precisa
digitar nem conhecer o caminho. Depois clique em **Aplicar e salvar**.

## Desfocar ou substituir o fundo com IA

Prepare o runtime por `./prepare-ai.sh --cuda` para NVIDIA ou `--cpu` para CPU.
Modelo e runtime disponíveis são detectados automaticamente, sem campos de caminho
na interface. Se instalou com a janela aberta, use **Atualizar** em **AI Background**.

O padrão é NVIDIA CUDA. A alternativa CPU fica em **Opções avançadas** e usa a
implementação ONNX própria; não é o modo dos efeitos NVIDIA no Windows.
Ative **Ativar fundo com IA** e selecione **Desfoque**,
**Cor sólida**, **Estúdio**, **Escritório**, **Paisagem**, **Minha imagem** ou **Meu vídeo**.
Use **Adicionar imagem…** para PNG, JPEG, WebP ou BMP, ou **Adicionar vídeo…** para MP4,
MOV, WebM ou MKV local. O arquivo é copiado para a biblioteca em segundo plano,
aparece pelo nome e recebe uma miniatura. Nas próximas execuções, escolha-o no
seletor; mover ou apagar o original não afeta a cópia. Vídeos são reproduzidos em loop, sem áudio, com recorte central
para preencher o quadro; arquivos podem ter até 4 GiB. Os três cenários
prontos são ilustrações geradas localmente. Clique em **Aplicar e salvar**. O aplicativo
faz uma inferência de aquecimento antes de trocar a captura. A descoberta dos
arquivos não liga o desfoque e não comprova compatibilidade CUDA. Erros de provider
aparecem em vez de usar CPU silenciosamente.

**FPS da IA** limita a saída virtual a 30 fps por padrão, mantendo a resolução
escolhida. Somente preview processa 640×360 a até 15 fps; não precisa compor
1080p60 para mostrar uma imagem pequena. O rótulo **Input Format** mostra o modo
de entrada da câmera, não a taxa processada.

A máscara atual serve ao desfoque e à substituição; não existe editor de seleção,
pincel de máscara, inserção de texto ou relighting.
Iluminação e movimento podem afetar o recorte. Não é necessário abrir um arquivo
de projeto gráfico para usar a câmera.
Para ocultar detalhes do ambiente, prefira uma substituição opaca ao desfoque.
A segmentação pode errar; confira o vídeo antes de compartilhar.

## Visualizar e enviar a outro aplicativo

**Grid** e **Mirror**, abaixo do preview, ajudam a inspecionar localmente. O
espelho dentro de **Effects** também afeta a saída virtual. Alterar visualização
não deve ser confundido com editar resolução ou FOV da câmera.

Prepare v4l2loopback conforme o [README](../README.md), inicie a câmera virtual
e selecione **VirtualCam** no consumidor. Isso entrega vídeo ao aplicativo
selecionado; não exporta uma foto, salva uma gravação ou cria um documento em
camadas. Não há exportador de PNG/JPEG, gravação de vídeo ou impressão na UI atual.

## Configuração, licença e suporte

O arquivo padrão fica sob `~/.config/elgato-virtualcam/`. Um JSON inválido é
preservado, e o primeiro salvamento mantém backup. Veja [Arquitetura](ARQUITETURA.md)
para o contrato técnico. O código usa MIT; dependências opcionais têm licenças
próprias. Não existe uma assinatura do aplicativo ou serviço de IA remoto.

O Facecam Hub não substitui um editor de imagens: não abre PSD ou documentos em
camadas e não oferece todas as funções do Camera Hub proprietário. Os recursos
atuais e as lacunas estão em [Roadmap](ROADMAP.md). Para pedir ajuda, reúna os
dados mínimos descritos em [Solução de problemas](SOLUCAO_DE_PROBLEMAS.md).
