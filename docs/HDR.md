# Controlar o HDR da Facecam MK.2

Na aba **Câmera**, em **Processamento**, use **Ativar HDR** com o preview ou a
câmera virtual em execução. A alteração é confirmada por leitura da câmera.

## Preparar e usar

1. Escolha um modo de até 60 fps e inicie o preview ou a saída virtual.
2. Abra **Processamento**. Se aparecer **Instalar suporte HDR…**, clique para
   preparar a instalação pelo APT e confirme o plano apresentado pelo aplicativo.
3. Aguarde a conclusão da instalação. O estado é consultado novamente.
4. Marque ou desmarque **Ativar HDR** e aguarde a confirmação.
5. **Salvar**, no card **Dispositivo**, guarda o HDR confirmado junto dos ajustes
   locais. **R** restaura o valor salvo, mantendo vídeo ativo quando for preciso
   efetivamente mudar o HDR.

A opção aparece somente para uma câmera com identidade e unidade USB confirmadas.
Nesta revisão, isso cobre Facecam MK.2 original, `0fd9:0093` em USB 2 e `0fd9:0094`
em USB 3, com o GUID e o seletor documentados abaixo. A execução física foi
verificada em USB 2; USB 3 e outras revisões exigem suas próprias evidências.
Revisões MK.2 Rev.2 e outras Facecams não recebem escritas por semelhança de nome.

A [orientação da Elgato](https://www.elgato.com/eu/et/explorer/software/camera-hub/elgato-camera-hub-dynamic-range-settings/)
limita HDR da MK.2 aos modos de 30/60 fps, sem 120 fps. O controlador recusa ativar
acima de 60 fps. O teste local usou 1920×1080/30. Esse controle altera o modo do
sensor/processamento da câmera; não anuncia uma nova saída de vídeo PQ ou 10 bits.
A saída virtual mantém seu formato YUV420 de 8 bits.

## Aplicação e falhas

O firmware retorna `0xff` enquanto aplica a mudança. Isso representa pendência,
não HDR ligado nem desligado. Nos ensaios locais, a confirmação levou cerca de
500–750 ms, com vídeo ativo. Escrever com a captura parada pode deixar o pedido
pendente; por isso o controlador exige vídeo antes de enviar a alteração.

O aplicativo verifica tamanho, flags e intervalo do controle antes de escrever.
Depois, aguarda o valor solicitado por até dois segundos. Um resultado diferente,
metadados incompatíveis, ferramenta ausente, desconexão ou timeout gera erro.
O snapshot mantém estado desconhecido durante pendência e não confirma sucesso.
Uma pendência é consultada novamente durante captura. O subprocesso tem timeout
e limites de saída; a GUI permanece na sua thread.

Não são escritos controles comuns, firmware nem memória de presets da câmera.
`saved_hdr` é uma preferência local opcional; uma configuração antiga continua
válida. A restauração pula HDR indisponível e informa o item; uma mudança que exige
vídeo sem captura ativa é recusada antes de restaurar os outros ajustes.

## Transporte confirmado

A análise estática do Camera Hub Windows 2.3.0.7295 e a consulta dos descritores
locais fecharam o seguinte contrato:

| Campo | Valor |
| --- | --- |
| ID interno da referência | `0x02060000` |
| GUID da extension unit | `23e49ed0-1178-4f31-ae52-d2fb8a8d3001` |
| Unidade no dispositivo ensaiado | 7; descoberta pelo GUID, sem fixar o número |
| Seletor | 21 (`0x15`) |
| Comprimento | 1 byte |
| Valores confirmados | 0 desativado, 1 ativado; `0xff` pendente |
| Metadados exigidos | GET/SET habilitados, mínimo 0, máximo 1, passo 1 |

`GetHDRMode` está em `0x141fc4280`; `SetHDRMode`, em `0x141fc43c0`.
O construtor `0x141fc2050` associa `0x02060000` ao objeto de GUID inicializado
em `0x1406e9700`. `EGAVUVC::ReadUVC` e `WriteUVC` encaminham a operação à unidade
correspondente. O executável analisado tem SHA-256
`c66fde57de800dccd539ee5f42a52624124ef0769765aadc7320ab43d5f96d88`.
As evidências locais ficam em `package/analysis-hdr-2.3.0.7295/`, ignoradas pelo Git.
Não é usado o getter CCUVC de tone mapping de placas de captura, cujo contrato
não corresponde à MK.2.

O módulo Rust `hdr.rs` valida identidade e descritores em sysfs e chama
[`uvcdynctrl`](https://manpages.ubuntu.com/manpages/noble/man1/uvcdynctrl.1.html)
para consultas e escrita bruta. A descoberta da unidade usa o bitmap do seletor;
GUID ausente, unidade ambígua ou dados truncados impedem escrever.
A [documentação do driver UVC](https://www.kernel.org/doc/html/latest/userspace-api/media/drivers/uvcvideo.html)
descreve a interface de extension units utilizada pela ferramenta. Não se desliga
o driver USB nem se descarrega o módulo da câmera.

A dependência opcional `hdr_tools` instala somente `uvcdynctrl` e dependências
resolvidas pelo APT dos repositórios Ubuntu. **Configurações** oferece instalar
ou desinstalar conforme dpkg. Não são adicionados repositórios ou instaladores
locais. `FACECAM_UVC_TOOL` pode selecionar uma ferramenta em testes técnicos;
não existe campo de caminho na interface.

## CLI e validação

Com captura ativa no aplicativo:

```bash
./target/release/elgato-virtualcam --set-hdr true
./target/release/elgato-virtualcam --set-hdr false
```

`set_hdr.enabled` é booleano. `state.hdr` informa `supported`, `available`,
`pending`, `needs_tool`, `enabled`, `unit` e `error`; `enabled: null` não significa
HDR desativado. A configuração `saved_hdr` só muda ao salvar os ajustes locais.
Consulte [Protocolo](PROTOCOLO_DE_CONTROLE.md), [Testes](TESTES.md) e
[Validação local](VALIDACAO_LOCAL.md). Checks comuns não escrevem na câmera;
o teste físico é optativo e restaura o valor original.
