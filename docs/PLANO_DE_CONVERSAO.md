# Execução da conversão autorizada

Esta página acompanha o escopo autorizado em 25/10/2026. O [roadmap](ROADMAP.md)
continua sendo a matriz de implementação e evidências; marcar uma etapa abaixo
não substitui seus testes.

## Escopo e ordem de trabalho

- [x] Internacionalização: PT-BR padrão, EN-US, arquivos de texto para a comunidade,
  seleção persistida e troca sem interromper captura ou perder rascunhos.
- [ ] Prompter: roteiro, leitura e rolagem, apresentação em monitor e controles
  pelo controlador, sem inserir o texto na saída da câmera.
- [ ] IA: correção de olhar e enquadramento automático completos, com GPU e
  preparação validada dos componentes necessários.
- [ ] HDR: descoberta e operação somente com contrato confirmado por modelo;
  não implementar persistência interna ou atualização de firmware nesta etapa.
- [ ] Interface: teclado, acessibilidade, escala, Wayland e bandeja; comparação
  visual com Windows foi adiada pelo usuário.
- [ ] Confiabilidade e distribuição: consumidores, sessões prolongadas,
  desconexão, suspensão/retomada, reinício e instalação/atualização/remoção.

Novas famílias RTX e os outros controles proprietários não fazem parte deste
escopo. Instalações administrativas continuam exigindo confirmação pelo fluxo
APT do aplicativo. Hardware ausente limita a aceitação, não autoriza declarar
um recurso como validado. Commits devem conter código, documentação e resultados
coerentes, preservando o inventário histórico de cobertura.
