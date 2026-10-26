# Idiomas, terminologia e fontes

O Facecam Hub inicia em **Português (Brasil)** e oferece **English (United States)**
em **Configurações** › **Idioma**. A escolha é salva imediatamente pelo controlador.

## Trocar o idioma

Abra as configurações e selecione o idioma pelo nome nativo. A troca atualiza
rótulos, mensagens conhecidas, diálogos e bandeja sem reiniciar a captura ou
confirmar efeitos do rascunho. Nomes de câmera, arquivos e conteúdo pessoal não
são traduzidos. Comandos, enums do JSON, caminhos e IDs de hardware mantêm seus
identificadores. Janelas e cards usam IDs independentes do texto traduzido.

O padrão é PT-BR, independentemente do idioma do Ubuntu. A preferência fica em
`ui.language`, com valores iniciais `pt-BR` e `en-US`. Ela vale para as próximas
execuções. A interface não usa uma opção automática baseada em `LANG`.

O processo principal e a CLI carregam somente arquivos de texto. Trocar o
idioma não carrega modelos, runtimes de IA, fontes da rede nem executa código
do catálogo. Mensagens nativas de FFmpeg, driver, sistema e bibliotecas externas
podem conservar o idioma original; o aplicativo traduz seus próprios rótulos e
contextos conhecidos, preservando os detalhes técnicos recebidos.

## Criar outro idioma

Os catálogos [PT-BR](../locales/pt-BR.json) e [EN-US](../locales/en-US.json)
são JSON UTF-8, editáveis em qualquer
editor de texto. Para iniciar uma tradução:

1. Copie `locales/pt-BR.json` para um arquivo com a tag do novo idioma, por exemplo
   `es-ES.json`.
2. Altere `language` para `es-ES` e `name` para o nome nativo, como `Español`.
3. Traduza somente os valores de `messages`. Preserve as chaves e os placeholders.
4. Coloque o arquivo em um dos diretórios de descoberta abaixo.
5. Reabra o aplicativo ou clique em atualizar dispositivos para reler a lista.
   Abra **Configurações** e escolha o novo idioma.

Exemplo parcial válido:

```json
{
  "language": "es-ES",
  "name": "Español",
  "messages": {
    "Camera": "Cámara",
    "Configurações": "Configuración",
    "Aplicar e salvar": "Aplicar y guardar"
  }
}
```

As chaves são mensagens-fonte estáveis, conforme a abordagem gettext; não são
os textos a editar. Uma chave ausente usa a tradução PT-BR compilada. O exemplo
parcial não representa uma tradução completa ou revisada do espanhol. Os dois
catálogos distribuídos, PT-BR e EN-US, têm as mesmas chaves verificadas por teste.
Uma tradução nova não exige alterar Rust ou recompilar o aplicativo.

JSON exige aspas duplas e não aceita comentários ou vírgula após o último item.
`language` precisa corresponder ao nome do arquivo, sem `.json`. Tags aceitam
letras ASCII, números e hífens, com até 35 caracteres. O nome nativo tem até
128 bytes e não pode conter controles. O catálogo tem limite de 2 MiB, 4.096
mensagens e 16 KiB por valor. Chaves duplicadas são recusadas.

## Placeholders e mensagens dinâmicas

Preserve placeholders como `{name}`, `{error}`, `{path}`, `{status}`, `{shape:?}`,
`{error:#}` e `{}`. Não traduza os nomes nem remova especificadores. Mantenha a
ordem dos placeholders anônimos; prefira placeholders nomeados em novas mensagens.
`\n` representa uma quebra de linha dentro do JSON. Para chaves literais no texto,
use `{{` e `}}`.

O carregador compara os placeholders da chave e do valor. Arquivo inválido,
valor vazio ou controles inesperados são recusados. Uma seleção recusada preserva
a preferência anterior e o catálogo ativo. Se um catálogo antes escolhido ficar
indisponível na inicialização, o aplicativo usa PT-BR compilado e informa o erro,
sem apagar a preferência do arquivo de configuração.

O catálogo também contém os templates das mensagens existentes do controlador.
A tradução reconhece o texto fixo e mantém valores de caminhos, números, nomes e
IDs. Contextos de erro conhecidos podem ser traduzidos; detalhes externos
continuam como vieram da biblioteca. Não usar esse mecanismo para reinterpretar
comandos, valores de controle ou resultados de hardware.

## Diretórios de descoberta

Para cada idioma, o primeiro arquivo encontrado segue esta ordem:

1. Diretório indicado por `FACECAM_LOCALES_DIR`, quando definido.
2. `locales/` ao lado do `config.json` usado pelo controlador.
3. `${XDG_DATA_HOME:-$HOME/.local/share}/facecam-hub/locales/`.
4. `locales/` ao lado do executável.
5. `share/facecam-hub/locales/` no prefixo do executável.
6. `locales/` no diretório de trabalho, usado no desenvolvimento.
7. `/usr/local/share/facecam-hub/locales/` e `/usr/share/facecam-hub/locales/`.
8. Catálogo compilado, para PT-BR e EN-US.

Um arquivo encontrado mas inválido informa erro; não é ignorado para escolher
outra cópia silenciosamente. Mensagens faltantes usam PT-BR. A instalação no
usuário copia os dois catálogos distribuídos para `share/facecam-hub/locales`.
Traduções pessoais ficam preferencialmente no diretório de dados do usuário ou
junto da configuração, preservados na remoção do aplicativo.

`FACECAM_LANGUAGE=en-US` seleciona temporariamente o idioma da ajuda e dos
resultados humanos da CLI. Não muda a preferência salva da GUI:

```bash
FACECAM_LANGUAGE=en-US ./target/release/elgato-virtualcam --help
```

## Fontes e revisão de uma contribuição

PT-BR e EN-US usam as fontes padrão compiladas de egui. Traduções para scripts
adicionais precisam conferir glifos, tamanho, quebra de linha e, quando aplicável,
direção do texto. A existência do arquivo não comprova revisão nativa ou suporte
completo a todos os scripts. Não inverta o vídeo para acomodar direção do texto.

Revise nomes, mensagens dinâmicas, plurais, atalhos, foco, tooltips e janelas
pequenas. Não traduza conteúdo pessoal para fazer um nome coincidir com o catálogo.
Contagens com regras de plural distintas exigem mensagens próprias; não há um
motor genérico de pluralização implementado. Uma alteração deve atualizar ambos
os catálogos distribuídos e passar os [testes de localização](TESTES.md).

Documentação em português segue os [Padrões de escrita](PADROES_DE_DOCUMENTACAO.md).
