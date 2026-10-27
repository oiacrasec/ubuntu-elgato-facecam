# Contribuição

Uma alteração deve preservar os contratos do controlador e ter uma forma objetiva
de verificação. Leia [Arquitetura](ARQUITETURA.md) e [Desenvolvimento](DESENVOLVIMENTO.md).

## Regras da implementação

Mantenha Rust no produto, sem Python, webview ou outra camada gráfica. O crate
proíbe código `unsafe`. Uma dependência pode conter implementação nativa; isso
precisa ser considerado na revisão, sem confundir bindings seguros com sandbox.

Novos caminhos que tratam entrada variável devem retornar `Result` e mensagens
úteis. Evite `unwrap`, `expect`, índices não verificados e macros de placeholder
nesses caminhos. Verifique multiplicações, diferenças, divisões, conversões,
NaN, infinito e alocações derivadas de entradas. Defina limites para recursão
ou estruturas cíclicas quando existirem. Esta é uma regra de desenvolvimento;
não afirma que todo o código atual já possui lints contra todas essas operações.

Proteja o estado confirmado de falhas parciais. Uma configuração recusada não
pode substituir a anterior. Trate threads, locks e filhos como recursos com
ciclo de vida explícito. Uma correção de crash deve incluir uma regressão pequena
que reproduza o erro de forma segura.

## Acrescentar um comando

1. Defina o comportamento, os parâmetros, as unidades e quando ele está disponível.
2. Implemente validação no núcleo, preservando o estado em caso de erro.
3. Acrescente a variante de `Request` e a transição em `Engine`.
4. Faça GUI e CLI usarem o mesmo caminho. Estado puramente visual permanece na GUI.
5. Teste resultado, erro, valores extremos, persistência e efeitos sobre processos.
6. Atualize a [referência do protocolo](PROTOCOLO_DE_CONTROLE.md) e os guias afetados.

Não existe registro dinâmico de centenas de comandos. Um catálogo futuro precisa
expor IDs estáveis e schemas, sem identificar comandos pelo texto traduzido.

## Recursos, paridade e licenças

Observe comportamento público e implemente código próprio. Não introduza firmware,
shaders, perfis, ícones ou imagens proprietárias extraídos de outro aplicativo.
Registre origem e licença dos recursos efetivamente usados. Dados reais de testes
precisam de autorização ou licença adequada; imagens pessoais não são fixtures.

O usuário autorizou em 26/10/2026 a incorporação dos ícones originais do aplicativo
e da bandeja do Camera Hub Windows. A exceção se limita aos recursos registrados
em [Recursos visuais](../assets/README.md), com proveniência e hashes. Em 27/10/2026,
o usuário solicitou também as mesmas imagens e vídeos de fundo: a exceção inclui
os 22 arquivos de `BackgroundsFolder` registrados nesse inventário. Esses recursos
de terceiros ficam fora da licença MIT do código; a exceção não se estende a
modelos, firmware, shaders ou recursos do Camera Hub fora desses inventários.

Uma linha de menu ou um controle desenhado não comprova paridade. Documente
recurso, cenário, evidência e limitação em [Roadmap](ROADMAP.md). Não transfira
porcentagens, números de testes ou resultados de outro hardware para este projeto.

## Revisão, colaboração e conclusão

Mantenha alterações focadas e manifests válidos em cada etapa. Em trabalho
simultâneo já autorizado, defina a posse dos arquivos e use diretórios de build
separados quando necessário; releia arquivos compartilhados antes de alterá-los.
Não reverta alterações de outra pessoa para limpar o próprio diff.

Use `cargo fmt` e Clippy com avisos como erro. Para uma alteração visual pequena,
a inspeção real é mais útil que testes que repetem coordenadas de implementação.
Para uma mudança funcional, escolha testes que exercitem o contrato e a falha.
Registre resultado, comando executado e limitações relevantes. Commits e PRs,
quando solicitados, devem descrever o comportamento final e sua verificação.

Créditos são voluntários. Não deduza nomes pessoais de configurações Git,
metadados ou contas de terceiros. O projeto ainda não tem um gerador de créditos
ou uma aba About com estatísticas; esses mecanismos precisam de implementação
própria antes de serem anunciados.
