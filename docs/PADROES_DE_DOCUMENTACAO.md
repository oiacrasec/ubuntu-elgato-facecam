# Padrões de documentação e escrita

Escreva para ajudar a pessoa a concluir uma tarefa ou entender uma decisão.
Comece pela resposta e confirme no código e na aplicação o comportamento descrito.
Estas orientações se aplicam à documentação, aos textos da interface e às
contribuições do Facecam Hub.

## Público e organização

Cada guia responde a uma pergunta ou resolve um problema. Nas duas primeiras
linhas, apresente a resposta ou a ação principal. Explique os detalhes depois.
Guias de uso falam com quem configura a câmera; documentação técnica explica
contratos, dependências e decisões para quem mantém o código.

Use um título de nível 1 por arquivo e seções de nível 2 para seus assuntos.
Escolha títulos que indiquem a tarefa, como “Ativar desfoque de fundo”. Use
listas numeradas para ações em sequência e tabelas para comparar parâmetros.
Mantenha os parágrafos curtos e uma ideia principal em cada um.

| Assunto | Documento responsável |
| --- | --- |
| Primeira execução, instalação e navegação | [README](../README.md) |
| Camadas, comandos, estado e ciclo da captura | [Arquitetura](ARQUITETURA.md) |
| Parâmetros, dependências e uso dos efeitos | [Efeitos](EFEITOS.md) |
| Como reproduzir as verificações | [Testes](TESTES.md) |
| Resultados observados, ambiente e limitações | [Validação local](VALIDACAO_LOCAL.md) |
| Evidências da investigação inicial | [Investigação](PORT_UBUNTU_24_04.md) |

Atualize o documento responsável pelo assunto e aponte para ele a partir dos
demais. Use links relativos para arquivos Markdown. Preserve os destinos dos
links; ao renomear uma página, atualize quem aponta para ela. Apresente documentos
históricos como registros de uma etapa, com data e contexto.

## Linguagem e nomenclatura

Use português direto, voz ativa e verbos concretos. Prefira “Clique em
**Aplicar e salvar**” a frases impessoais. Evite adjetivos promocionais,
superlativos, “basta”, “simplesmente” e promessas sem verificação.

O nome do produto é **Facecam Hub**. Identificadores técnicos mantêm a grafia
do código, como `elgato-virtualcam`, `background_blur` e `SetEffects`. Escreva
NVIDIA, CUDA, ONNX Runtime e V4L2 com a capitalização correspondente.

Reproduza os rótulos da interface exatamente, em **negrito**, mesmo quando estão
em inglês. Por exemplo: “Na aba **Effects**, ative **Desfocar fundo com IA**”.
Para uma sequência de navegação, use **Camera** › **Device** › **SAVE**.
Use crases para comandos, caminhos, nomes de arquivos, parâmetros e valores.

Mantenha o vocabulário consistente entre botão, guia e mensagem de resultado.
Diferencie o espelho local do preview do espelho aplicado ao vídeo. Descreva
SAVE e R como persistência local. Informe a unidade real de cada ajuste; ganho
do driver não é ISO. Uma inferência CPU não deve ser descrita como CUDA, e um
filtro convencional não deve ser apresentado como IA.

## Instruções, exemplos e mensagens

Antes dos passos, informe pré-requisitos que mudam o resultado: câmera,
dependências, provider e estado da captura. Cada passo deve indicar uma ação.
Depois, diga o que a pessoa deve ver e onde procurar um erro.

Em blocos de código, indique a linguagem e use comandos executáveis a partir
do diretório informado. Use caminhos de exemplo reconhecíveis, como
`/caminho/modelo.onnx`. Comandos que dependem de preparação anterior devem
mostrar essa preparação ou apontar para o guia correspondente.

Uma página de solução de problemas segue esta ordem: sintoma observável,
correção em passos, explicação quando útil e dados necessários se a falha
persistir. Informe o comando, a mensagem e o caminho do log relevantes.
Uma mensagem de erro identifica a operação que falhou e uma ação possível.
Estados vazios explicam o próximo passo, como conectar a câmera ou iniciar o
preview. Não atribua culpa à pessoa nem declare sucesso após uma falha.

## Afirmações, versões e evidências

Confira nomes, padrões, limites e comandos na implementação antes de publicá-los.
Descreva recursos implementados, parciais e planejados com seu estado explícito.
Uma opção visível ou um build concluído não comprovam execução. Uma captura de
preview não comprova consumo da câmera virtual por OBS ou navegador.

Em relatórios, registre a data, a versão ou revisão examinada, o ambiente,
o procedimento e o resultado. Diferencie a versão declarada em `Cargo.toml`,
a árvore em desenvolvimento e uma versão efetivamente publicada. Marque como
não verificado o que não foi exercitado naquele ambiente.

Medições precisam indicar cenário, unidades, duração e escopo. Separe tamanho
do binário, dependências em disco, memória RAM e memória GPU. Informe se a taxa
medida é de captura, preview ou saída consumida. Uma amostra local curta não
estabelece uma garantia de desempenho.

Ao documentar uma API ou dependência externa, consulte sua documentação oficial
e mantenha o link junto da afirmação correspondente. Diagramas e exemplos
devem representar as camadas existentes. A documentação não muda a separação
entre interfaces, controlador, captura e processamento.

## Revisão de uma contribuição

Inclua na mesma alteração a documentação afetada por um novo comportamento.
Explique o problema, o resultado, a validação realizada e as limitações materiais.
Comentários de código explicam decisões e motivos que não estão claros no código.
Referências de terceiros precisam manter as licenças e atribuições aplicáveis.

Antes de concluir, confira os links relativos e se os exemplos correspondem
à CLI e à interface atuais. Para mudanças visuais, inspecione a janela ou uma
captura real, incluindo texto longo e estado desabilitado quando afetados.
Registre o que foi verificado. Execute os checks adequados descritos em
[Testes](TESTES.md); alterações apenas editoriais não exigem testes de hardware.

## Catálogo, publicação e metadados

O [manual](README.md) é o índice de navegação. Guias de primeiros passos, tarefas,
solução de problemas, perguntas frequentes e referência podem compartilhar um
arquivo enquanto forem curtos. Ao crescer, separar páginas por pergunta e manter
links estáveis. Um índice deve indicar o próximo passo, não apenas repetir títulos.

Atualmente os arquivos são Markdown sem frontmatter e sem publicador de site.
Se houver publicação futura, definir um schema com título, descrição, ordem,
versão verificada e aliases de URL; metadados não devem contradizer a página.
Renomear um destino publicado exige redirecionamento ou atualização do contrato
de URLs. Rascunhos precisam ser identificados e não devem aparecer como guia
operacional. Ainda não há uma convenção de publicação automática por prefixo `_`.

O schema proposto para páginas publicadas é:

| Chave | Significado |
| --- | --- |
| `title` | Título da página e da navegação |
| `description` | Uma frase útil para busca e resumo |
| `order` | Ordem opcional dentro da seção |
| `verified` | Versão publicada efetivamente conferida |
| `redirect_from` | Lista opcional de caminhos antigos |

Um consumidor simples deve declarar se aceita apenas escalares e listas de uma
linha; valores com ` #` ou aspas iniciais precisam de quoting adequado. Se o
publicador gerar o título a partir de `title`, o corpo começa em `##`, sem outro
H1 duplicado. Isso é um contrato para publicação futura, não um parser disponível
no projeto atual.

A versão verificada de um guia público precisa corresponder ao artefato que a
pessoa baixa. A árvore de desenvolvimento pode possuir recursos adicionais,
explicitamente identificados. Não elevar o campo de verificação com base apenas
no número de versão do manifest ou em um build que não foi exercitado.

## Cobertura, status e manutenção

Registre um tema em [Cobertura documental](COBERTURA_DOCUMENTAL.md) com seu destino
responsável e sua adaptação. Assuntos repetidos podem ser consolidados; temas de
outros domínios devem ter uma decisão de escopo e critérios técnicos pertinentes.
Dados de benchmark externos não se tornam medições locais ao traduzir a tabela.

Use os estados Implementado, Limitação conhecida, Proposto e Fora do escopo atual
definidos no [manual](README.md). Um endurecimento futuro não deve ser listado
como controle já imposto. Uma página destinada a resolver uma pergunta de suporte
é revisada como código: conferir afirmações, procedimento e resultado.

Os padrões disponíveis no repositório são a fonte normativa local. Links para
processos privados ou ferramentas inexistentes não criam uma dependência de build;
o contrato necessário deve ser documentado aqui antes de ser exigido de quem
contribui. Não copiar regras sobre publicação, créditos ou autoridade de outro
produto como se descrevessem este projeto.
