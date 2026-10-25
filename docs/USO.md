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

1. Abra **Effects**.
2. Ajuste brilho, contraste, saturação e denoise, ou informe uma LUT compatível.
3. Clique em **Aplicar e salvar**.
4. Confira o estado e o resultado no preview; a captura pode reiniciar brevemente.

Saturação zero remove a cor. Restauração dos efeitos padrão prepara um rascunho;
o botão de aplicar confirma a alteração. LUTs e efeitos são locais, preservando
os controles V4L2 como uma etapa distinta. [Efeitos](EFEITOS.md) lista valores e arquivos.

## Desfocar o fundo com IA

Prepare o runtime por `./prepare-ai.sh --cuda` para NVIDIA ou `--cpu` para CPU.
Ao abrir normalmente, os caminhos disponíveis são preenchidos automaticamente.
Se a instalação ocorreu com a janela já aberta, use **Detectar arquivos** em
**AI Background**. Caminhos manuais preenchidos são preservados.

Escolha o provider correto, ative **Desfocar fundo com IA** e aplique. O aplicativo
faz uma inferência de aquecimento antes de trocar a captura. A descoberta dos
arquivos não liga o desfoque e não comprova compatibilidade CUDA. Erros de provider
aparecem em vez de usar CPU silenciosamente.

A máscara atual serve ao desfoque automático; não existe editor de seleção,
pincel de máscara, inserção de texto, imagem de substituição ou relighting.
Iluminação e movimento podem afetar o recorte. Não é necessário abrir um arquivo
de projeto gráfico para usar a câmera.

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
