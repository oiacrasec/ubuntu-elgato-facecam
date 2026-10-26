# Ler um roteiro no Prompter

Abra a aba **Prompter**, escreva seu texto e clique em **Salvar roteiro**.
A leitura funciona na janela do aplicativo ou em uma apresentação separada.

## Preparar e apresentar

1. Cole ou escreva o roteiro no card **Roteiro**, com até 8 KiB de texto UTF-8.
2. Em **Leitura**, ajuste velocidade, tamanho e **Espelhar texto**, quando necessário
   para um vidro de teleprompter. O espelho afeta somente o texto.
3. Em **Tela de apresentação**, escolha **Janela móvel** ou um monitor identificado
   pelo Ubuntu. Um monitor selecionado recebe apresentação em tela cheia.
4. Clique em **Salvar roteiro** para confirmar texto e preferências.
5. Use **Iniciar leitura** e **Abrir apresentação**, na ordem desejada.

O rascunho só é aplicado ao salvar. Alterar e salvar o texto pausa a leitura e
volta ao início; alterar velocidade, tamanho, espelho ou monitor preserva a posição.
Falha de validação ou salvamento preserva o roteiro confirmado e sua configuração.
A velocidade é uma estimativa em palavras por minuto baseada no tamanho do texto;
não é reconhecimento de fala. O layout e a quebra de linha se adaptam à tela.

A posição de leitura vai de 0 a 100%. **Voltar ao início** pausa e reposiciona.
Chegar ao fim pausa automaticamente; iniciar novamente retoma do início.
**Fechar apresentação**, Esc ou fechar essa janela também pausa, preservando a
posição. O texto e as preferências são locais; reproduzir e apresentar são estados
da sessão e não começam sozinhos na próxima execução.

## Teclado e monitores

Na apresentação, Espaço inicia/pausa, Home volta ao início e as setas para cima
ou para baixo movem 2% do roteiro. Esc fecha a apresentação. Na janela principal,
atalhos de leitura só atuam quando nenhum editor ou outro widget retém o teclado.
Os controles de reprodução ficam fixos abaixo da área rolável no tamanho mínimo.

A lista de telas vem do próprio backend gráfico. Uma mudança nessa lista fecha a
apresentação e pausa a leitura; escolha a tela novamente. Os índices salvos seguem
a ordem do desktop e podem mudar ao conectar/desconectar monitores. Uma tela
indisponível é recusada; a aplicação não anuncia apresentação em outro monitor.
Uma pausa do controlador superior a dois segundos, incluindo suspensão, pausa a
leitura sem saltar o roteiro. Minimizar a janela principal não interrompe a leitura
na janela de apresentação.

A [Elgato descreve o Prompter como uma tela adicional](https://www.elgato.com/us/en/p/prompter).
Nosso renderer usa monitores já reconhecidos pelo Ubuntu e não instala driver
DisplayLink automaticamente. O aparelho Elgato não estava conectado na validação;
apresentação em monitor comum não comprova suporte ao transporte USB, brilho ou
espelho interno do dispositivo. Voice Sync, Twitch, biblioteca de múltiplos
roteiros e compatibilidade de comandos proprietários ainda não estão implementados.
Essas lacunas impedem declarar paridade completa do Prompter Windows.

## Captura e automação

O roteiro nunca entra nos filtros, worker de IA ou split de vídeo. Preview e câmera
virtual continuam com a mesma imagem processada. A apresentação pode ser usada
sem câmera conectada e tem vida própria dentro da GUI. Não são enviados roteiros
a serviços externos.

GUI e CLI usam o controlador. Exemplo de `roteiro.json`:

```json
{
  "text": "Olá! Este é meu roteiro.\n\nSegundo parágrafo.",
  "words_per_minute": 120,
  "font_size": 48,
  "mirror": false,
  "monitor": null
}
```

Com o aplicativo aberto:

```bash
./target/release/elgato-virtualcam --prompter-settings roteiro.json
./target/release/elgato-virtualcam --prompter-action show
./target/release/elgato-virtualcam --prompter-action play
./target/release/elgato-virtualcam --prompter-action pause
./target/release/elgato-virtualcam --prompter-position 50
./target/release/elgato-virtualcam --prompter-action rewind
./target/release/elgato-virtualcam --prompter-action hide
```

`words_per_minute` aceita 30–400; `font_size`, 16–128 pontos. `monitor: null` usa
janela móvel; um índice da lista publicada usa tela cheia. O headless aceita
roteiro e reprodução, mas recusa apresentação sem GUI. Consulte o
[protocolo](PROTOCOLO_DE_CONTROLE.md) e a [validação local](VALIDACAO_LOCAL.md).
