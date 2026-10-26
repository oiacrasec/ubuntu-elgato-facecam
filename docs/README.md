# Manual do Facecam Hub

Use o [guia de uso](USO.md) para operar a câmera e o [guia de desenvolvimento](DESENVOLVIMENTO.md)
para trabalhar no código. O projeto atende Ubuntu 24.04+ com uma interface nativa Rust.

## Estado das informações

**Implementado** significa comportamento presente no código e verificável pelos
checks indicados. **Limitação conhecida** descreve algo que o produto ainda não
resolve. **Proposto** identifica um desenho técnico sem implementação. **Fora do
escopo atual** mantém registrado um tema sem prometer sua entrega.

Um contrato proposto não cria comandos, recursos ou garantias no binário atual.
A versão declarada é 3.0.0 em desenvolvimento. As medições datadas pertencem à
revisão em que foram executadas; o perfil Cargo `release` não comprova publicação.

## Uso e referência

- [Instalação e execução](../README.md)
- [Operação da câmera, presets e visualização](USO.md)
- [Efeitos e IA, incluindo descoberta automática](EFEITOS.md)
- [HDR da Facecam MK.2](HDR.md)
- [Roteiro, leitura e apresentação do Prompter](PROMPTER.md)
- [Runtimes NVIDIA Broadcast de referência e migração Linux](NVIDIA_MAXINE.md)
- [Falhas e recuperação](SOLUCAO_DE_PROBLEMAS.md)
- [Comandos, CLI e protocolo local](PROTOCOLO_DE_CONTROLE.md)
- [Formatos, importação e fidelidade](FORMATOS_E_COMPATIBILIDADE.md)

## Engenharia

- [Arquitetura existente](ARQUITETURA.md)
- [Desenvolvimento e contratos de camadas](DESENVOLVIMENTO.md)
- [Contribuições e regras de manutenção](CONTRIBUICAO.md)
- [Padrões de documentação e escrita](PADROES_DE_DOCUMENTACAO.md)
- [UI, diálogos, acessibilidade e interação](UI_E_INTERACAO.md)
- [Idiomas, terminologia e fontes](LOCALIZACAO.md)
- [Algoritmos de efeitos e processamento](ALGORITMOS_DE_EFEITOS.md)
- [Medição, cache e orçamento de recursos](DESEMPENHO.md)
- [Testes, corpus e fuzzing](TESTES.md)
- [Fronteiras de confiança e segurança](SEGURANCA.md)
- [Extensões, automação e portabilidade](EXTENSOES.md)
- [Dependências, recursos e licenças](DEPENDENCIAS_E_LICENCAS.md)
- [Distribuição e publicação](DISTRIBUICAO.md)

## Planejamento e evidências

- [Roadmap, paridade e critérios de prontidão](ROADMAP.md)
- [Execução do escopo autorizado](PLANO_DE_CONVERSAO.md)
- [Decisões de implementação](DECISOES.md)
- [Validação local](VALIDACAO_LOCAL.md)
- [Levantamento histórico de V4L2 e investigação inicial](PORT_UBUNTU_24_04.md)
- [Referência Windows e matriz de migração completa](REFERENCIA_WINDOWS.md)
- [Análise RTX, compatibilidade e organização dos runtimes](ANALISE_RTX.md)
- [Inventário de cobertura documental](COBERTURA_DOCUMENTAL.md)

A cobertura classifica individualmente os textos, licenças e dados documentais
examinados e aponta seu destino neste manual. Os temas repetidos compartilham
uma página responsável. Recursos de edição de documentos, outras plataformas e
integrações futuras têm estado e critérios próprios; não são apresentados como
funcionalidades existentes de uma câmera.
