# Ícones do Facecam Hub

A janela, o menu do Ubuntu e a bandeja usam os ícones originais do Camera Hub
Windows 2.3.0.7295, conforme solicitado pelo usuário em 26/10/2026.

O fornecedor é [Elgato/Corsair](https://www.elgato.com/us/en/s/downloads). As imagens são recursos de terceiros e ficam
excluídas da licença MIT do código deste projeto. Não lhes foi atribuída uma
licença permissiva. A exceção autorizada abrange somente estes ícones.

| Arquivo em `icons/` | Origem e conversão |
| --- | --- |
| `windows/app-icon.ico` | `:/images/app-icon.ico`, extração integral |
| `windows/tray-icon.ico` | `:/images/tray-icon.ico`, extração integral |
| `windows/tray-icon-update.ico` | `:/images/tray-icon-update.ico`, extração integral |
| `facecam-hub.png` | Frame PNG original de 256×256 do ICO do aplicativo, sem recodificação |
| `facecam-hub-tray.png` | Frame original de 64×64 do ICO da bandeja, convertido de BMP para PNG |
| `facecam-hub-tray-update.png` | Frame original de 64×64 do ICO de atualização, convertido de BMP para PNG |

O [registro de proveniência](icons/provenance.json) contém hashes SHA-256 do
executável e dos arquivos. Os ICOs vieram dos recursos Qt embutidos no executável
`ExeFile`, extraído do instalador Windows local. A extração não executou o binário
Windows nem acrescentou Qt ao produto Rust.

O ícone da bandeja com ponto amarelo indica atualização disponível ou reinício
pendente. A captura ativa continua indicada pelo título e pelo menu da bandeja.
O instalador inclui os PNGs, este texto e o registro de proveniência no bundle;
registra as imagens no tema hicolor sem recolori-las como ícones simbólicos.
