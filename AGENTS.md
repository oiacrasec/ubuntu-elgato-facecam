# Orientações para agentes e contribuidores

Leia [CLAUDE.md](CLAUDE.md), o [manual](docs/README.md) e os
[padrões de documentação e escrita](docs/PADROES_DE_DOCUMENTACAO.md).
Os contratos de arquitetura e os limites documentados se aplicam a GUI, CLI e testes.

Implemente ações operacionais no controlador; preserve o build headless e o código
Rust sem `unsafe`. Não copie recursos proprietários ou declare como implementado
um projeto de extensão. [Contribuição](docs/CONTRIBUICAO.md) define o fluxo de revisão.

Para documentação, mantenha o inventário de cobertura e os links íntegros. Testes
com hardware são optativos e não devem alterar controles reais nos checks comuns.
