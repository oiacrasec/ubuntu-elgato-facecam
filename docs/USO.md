# Usar o Facecam Hub

Abra `./target/release/elgato-virtualcam` e clique em **Iniciar preview**.
A aba **Câmera** controla o hardware; **Efeitos** processa o vídeo no computador.

O idioma padrão é **Português (Brasil)**. Em **Configurações** › **Idioma**,
escolha **English (United States)** para usar inglês. A mudança é imediata,
salva automaticamente e preserva a captura. [Localização](LOCALIZACAO.md)
explica como acrescentar outro idioma por um arquivo JSON.

## Instalar e abrir

Siga os comandos de dependências e build no [README](../README.md). O instalador
`./install-desktop-app.sh` adiciona o binário e lançador no usuário. Para usar
sem janela, execute `./target/release/elgato-virtualcam --headless`.

A aplicação tem uma instância por runtime do usuário. Abrir novamente traz a
janela existente; um binário recompilado só substitui o código em execução após
encerrar e reabrir o aplicativo. Fechar a janela com bandeja mantém o controlador;
use **Encerrar aplicativo** em Configurações ou `--quit` para sair por completo.

Em **Configurações**, as preferências são aplicadas e salvas ao alterar, sem botão
**Salvar**. **Iniciar na bandeja** define como a janela abre e pode ser alterado
durante a captura. O seletor de câmera virtual exige captura parada.
**Iniciar ao entrar no desktop** é um checkbox
independente: marcar cadastra a inicialização automática do usuário e desmarcar
remove esse cadastro imediatamente, sem precisar de **Salvar** ou senha.
O estado acompanha o arquivo de autostart, inclusive ao reabrir o aplicativo.

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

SDK Manager e os redistribuíveis NVIDIA Broadcast são componentes distintos:
instalar o gerenciador não habilita correção de olhar nem os efeitos VFX/AR.
O [guia dos runtimes NVIDIA](NVIDIA_MAXINE.md) registra os instaladores Windows
indicados pela Elgato e os limites da migração para Ubuntu.

## Escolher câmera e formato

1. Pare a captura se estiver ativa.
2. Na aba **Câmera**, card **Dispositivo**, escolha **Entrada** e **Formato**.
3. Use atualização de dispositivos se conectou ou trocou a câmera.
4. Clique em **Iniciar preview** e confira a imagem.

Resoluções e taxas dependem do modo e da conexão. Um nome de câmera igual não
assegura formatos iguais em USB2 e USB3. Não force uma combinação que não aparece
na lista. [Solução de problemas](SOLUCAO_DE_PROBLEMAS.md) ajuda quando a câmera não aparece.

## Ajustar imagem e enquadramento

Use os controles disponíveis em **Enquadramento**, **Imagem**, **Exposição** e
**Balanço de branco**. Pan, tilt e zoom pertencem ao hardware. O driver determina
mínimo, máximo, passo e valores de menu. Modo automático pode tornar um ajuste
manual indisponível. Tempo de exposição e ganho têm as unidades indicadas na UI.

**Salvar** guarda formato e controles disponíveis neste computador. **R** restaura
os controles salvos que a câmera aceitar naquela configuração. O aplicativo não
promete memória interna, presets de firmware ou equivalência entre câmeras.

## Preparar a câmera virtual

Nas configurações, **Câmera virtual** também lista os dispositivos detectados pelo
nome. O controlador usa a saída configurada quando disponível ou a primeira
v4l2loopback encontrada; nenhum campo de caminho precisa ser preenchido.

1. Pare preview e saída virtual.
2. Abra **Configurações** › **Câmera virtual**.
3. Escolha **Disponível após reiniciar o Ubuntu** ou desmarque para usar somente
   nesta sessão.
4. Clique em **Preparar câmera virtual…**, confira comandos e arquivos e selecione
   **Confirmar preparação**.
5. Conclua a autenticação administrativa do Ubuntu e aguarde o resultado.
6. Clique em **Iniciar câmera virtual** antes de abrir ou atualizar o consumidor;
   nele, escolha **Elgato Facecam Virtual**.

Uma autorização executa as etapas fixas de preparação. O plano escolhe um número
livre a partir de `/dev/video10` e carrega v4l2loopback com `exclusive_caps=1`,
para compatibilidade com navegadores. Aproveita o módulo existente; quando ausente,
consulta o pacote oficial correspondente ao kernel. Não substitui um módulo
presente por DKMS. Sem pacote pré-compilado nos repositórios, a preparação assistida
informa que é necessário configurar o módulo no Ubuntu.

O modo persistente grava `/etc/modprobe.d/facecam-hub.conf` e
`/etc/modules-load.d/facecam-hub.conf`, identificados como gerenciados pelo app,
e atualiza o initramfs do kernel atual. Arquivos desses nomes que não pertencem
ao app são recusados. Secure Boot permanece ativo; o módulo precisa de assinatura
aceita pelo Ubuntu. O app não descarrega módulos ou resolve conflitos removendo
dispositivos de outros aplicativos. Uma falha administrativa interrompe as próximas
etapas; as alterações concluídas permanecem e o log indica onde parou.

A confirmação verifica se a saída se tornou acessível e, no modo persistente,
se os arquivos foram gravados. Isso não substitui um teste após reiniciar nem
comprova consumo de vídeo. Abrir, atualizar ou cancelar o plano não carrega o módulo.

## Alterar cor e efeitos

1. Abra **Efeitos**.
2. Escolha uma LUT em **Visual**, ajuste brilho, contraste e saturação em **Imagem**,
   ou denoise e espelho em **Processamento**.
3. Clique em **Aplicar e salvar**.
4. Confira o estado e o resultado no preview; a captura pode reiniciar brevemente.

Saturação zero remove a cor. Restauração dos efeitos padrão prepara um rascunho;
o botão de aplicar confirma a alteração. LUTs e efeitos são locais, preservando
os controles V4L2 como uma etapa distinta. [Efeitos](EFEITOS.md) lista valores e arquivos.
Os cards seguem o padrão da aba **Câmera** e podem ser recolhidos pelos títulos
para liberar espaço. Rótulos ficam à esquerda e valores numéricos à direita.

**Original**, **Quente**, **Frio** e **Contraste suave** estão disponíveis no seletor.
Para usar outro Look, clique em **Adicionar LUT…** e escolha o arquivo no diálogo
do Ubuntu. O app valida a LUT e guarda uma cópia na biblioteca; você não precisa
digitar nem conhecer o caminho. Depois clique em **Aplicar e salvar**.

## Desfocar ou substituir o fundo com IA

Na RTX 40/Ada, pare a captura e abra **Configurações** › **Efeitos NVIDIA RTX** ›
**Preparar IA RTX**. Aguarde a preparação e o teste da GPU concluírem; progresso
e log aparecem na janela. O app reutiliza o pacote de referência em Downloads
quando disponível e obtém os componentes restantes no usuário, sem pedir caminhos
ou senha. Downloads podem ocupar vários GB e levar alguns minutos.
Se faltarem ferramentas, o app apresenta antes sua instalação por APT, com
confirmação e autenticação do Ubuntu. Ao concluir, repita **Preparar IA RTX**.

O modo **Automática**, em **Opções avançadas**, prefere a segmentação temporal
NVIDIA validada. Se ela falhar ao iniciar, usa ONNX com o mesmo provider e mostra
o motivo. Para a alternativa ONNX, prepare `./prepare-ai.sh --cuda` ou `--cpu`.
Modelo e runtime são detectados automaticamente. Use **Atualizar** em **Fundo com IA**
se preparou arquivos com a janela aberta. [Análise RTX](ANALISE_RTX.md) registra
o suporte local e as famílias ainda não reconstruídas.

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

Para corrigir uma importação errada, selecione a imagem em **Minha imagem** e
clique em **Remover imagem**. A remoção é imediata e apaga somente a cópia da
biblioteca; o original permanece intacto. Se a imagem estiver aplicada, o app
troca para **Desfoque**, preserva a IA e salva a mudança antes de remover a cópia.
Se a troca falhar, a imagem permanece disponível. Imagens externas e compartilhadas
não podem ser apagadas por esse botão. Você pode adicionar outra imagem em seguida.

**FPS da IA** limita preview e saída virtual a 30 fps por padrão, mantendo a
resolução escolhida. Ambos recebem os mesmos quadros, inclusive com apenas
preview ativo. O rodapé **Saída** mostra resolução e taxa configuradas para a
saída. A janela preserva a proporção ao ajustar a imagem ao espaço disponível.

A máscara atual serve ao desfoque e à substituição; não existe editor de seleção,
pincel de máscara, inserção de texto ou relighting.
Iluminação e movimento podem afetar o recorte. Não é necessário abrir um arquivo
de projeto gráfico para usar a câmera.
Para ocultar detalhes do ambiente, prefira uma substituição opaca ao desfoque.
A segmentação pode errar; confira o vídeo antes de compartilhar.

## Ativar HDR na Facecam MK.2

Com preview ou câmera virtual ativo, abra **Câmera** › **Processamento** e use
**Ativar HDR**. Se faltar suporte, a aplicação oferece **Instalar suporte HDR…**
pelo APT, com confirmação. **Salvar** guarda o valor neste computador e **R**
restaura, sem gravar presets na câmera. HDR da MK.2 aceita até 60 fps; a mudança
aguarda confirmação do dispositivo. Consulte [HDR](HDR.md) para limites e modelos.

## Visualizar e enviar a outro aplicativo

Com a câmera virtual ativa, **Visualizar** mostra ou pausa somente a imagem na
janela. A saída para o Meet/OBS continua ativa; fechar a janela para a bandeja
também preserva essa saída. **Iniciar preview**, quando disponível no painel,
preserva uma câmera virtual que já esteja transmitindo. **Parar câmera virtual**
encerra a captura. Após trocar o nome do dispositivo, atualize a lista de câmeras
do consumidor e escolha **Elgato Facecam Virtual**.

**Espelhar**, abaixo do preview, espelha preview e câmera virtual imediatamente,
usando o mesmo efeito de **Efeitos**. **Grade** desenha apenas uma guia de
enquadramento; desligue-a para ver a imagem sem sobreposição. O preview usa a
resolução e a taxa da saída, com os mesmos efeitos e conversão de cor yuv420p.
O tamanho da imagem na janela depende do espaço disponível; ampliar ou reduzir
a janela não altera a resolução enviada. O monitor pode limitar os quadros
apresentados, e OBS/navegadores podem aplicar seu próprio espelhamento ou ajustes.

Faça a [preparação assistida](#preparar-a-câmera-virtual), inicie a câmera virtual
e selecione **Elgato Facecam Virtual** no consumidor. Isso entrega vídeo ao aplicativo
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
