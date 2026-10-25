# Idiomas, terminologia e fontes

A interface atual combina mensagens em português com nomes de seções do Camera
Hub em inglês. Não há seletor de idioma ou catálogo de traduções implementado.

## Contrato para evolução

Uma localização futura deve separar IDs de comandos e chaves estáveis dos textos
visíveis. Presets, caminhos, nomes fornecidos pelo usuário e IDs de hardware não
são traduzidos. Mensagens dinâmicas, erros, menus gerados e plurais precisam da
mesma cobertura que rótulos estáticos.

Definir uma preferência explícita de idioma e uma opção automática. Uma escolha
salva pela pessoa prevalece sobre detecção do sistema. A detecção não deve gravar
configuração apenas porque a janela abriu. Tags regionais exigem política
publicada de fallback; não confundir português brasileiro com revisão linguística
validada de todos os países lusófonos.

A troca de idioma ao vivo, quando implementada, deve preservar captura, rascunhos
e foco. Uma prévia em Configurações só vira preferência ao aplicar; cancelar
restaura o idioma anterior. Identidade de janelas e controles não pode depender
da tradução de seu título.

## Escopo de revisão por idioma

| Grupo | Revisão necessária |
| --- | --- |
| Português, espanhol e italiano | Terminologia de câmera, acentos, artigos e ações curtas |
| Russo e outras línguas com múltiplos plurais | Contagens, placeholders, abreviações e gramática |
| Coreano | Níveis de formalidade, espaçamento e nomes de controles |
| Chinês simplificado e tradicional | Catálogos distintos e vocabulário técnico revisado |
| Japonês e demais scripts CJK | Cobertura de glifos, alinhamento e fallback de fontes |
| Idiomas da direita para a esquerda | Bidi, posição do cursor, menus, campos, foco e espelhamento de chrome |

Esta tabela é um plano de validação, não uma lista de idiomas disponíveis.
A existência de tradução automática ou glifos não comprova revisão nativa. Não
marcar cobertura completa apenas por traduzir os menus.

## Fontes e layout

O aplicativo usa as fontes padrão compiladas de egui. Não distribui os pacotes
Inter, JetBrains Mono, dicionários ou coleções de fontes de outros projetos.
Qualquer adoção futura exige origem e licença próprias, orçamento de tamanho e
verificação de fallback. Encontrar o nome de uma família não comprova que os
mesmos glifos foram resolvidos em dois computadores.

Textos de controles fixos devem caber ou ser truncados com acesso ao valor
completo. Idiomas longos não devem desalinhá-los. Não inverta o vídeo da câmera
para acomodar direção do texto; chrome e imagem têm semânticas independentes.

## Manutenção de catálogos proposta

Um catálogo deve declarar código, nome nativo, regra de plural e estado de
revisão. Preserve placeholders e escapes; mantenha contexto por ID quando uma
mesma palavra tenha sentidos distintos. Um verificador futuro deve apontar
chaves ausentes, placeholders incompatíveis e caracteres de controle inesperados.
Não existe atualmente esse verificador ou comando de cobertura de idiomas.

Testar uma sequência de mudanças de idioma em um processo, janelas pequenas,
nomes extensos, números e erros dinâmicos. Medir custo de carregar fontes em vez
de recarregá-las por frame. Documentação em português continua seguindo os
[Padrões de escrita](PADROES_DE_DOCUMENTACAO.md).
