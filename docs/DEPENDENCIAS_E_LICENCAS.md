# Dependências, recursos e licenças

O código deste projeto usa a licença [MIT](../LICENSE). Bibliotecas, modelos,
fontes e ferramentas externas têm licenças e condições próprias.

## Inventário de dependências

| Componente | Uso atual |
| --- | --- |
| Rust/Cargo e `Cargo.lock` | Compilação e versões resolvidas |
| egui/eframe e Glow | Janela nativa, OpenGL e acessibilidade compilada |
| ksni | Bandeja StatusNotifierItem por D-Bus, opcional |
| FFmpeg | Captura, conversão, filtros e saída de vídeo |
| v4l-utils / `v4l2-ctl` | Consulta e escrita de controles e formatos |
| v4l2loopback | Nó virtual preparado no sistema pelo usuário |
| `ort` | Bindings Rust opcionais para ONNX Runtime carregado dinamicamente |
| ONNX Runtime e modelo | Inferência opcional, fora do binário do aplicativo |
| CUDA e cuDNN | Provider NVIDIA, com dependências externas opcionais |

Leia `Cargo.toml` para as features e `Cargo.lock` para o grafo resolvido. O produto
não inclui um runtime Python; os pacotes nativos distribuídos em contêineres wheel
são extraídos pelo preparador sem executar Python.

Uma mudança de dependência deve registrar necessidade, licença declarada, origem,
alvos, manutenção, exposição a entrada externa e uso de código nativo/unsafe.
Não substituir revisão de fonte e contratos por um selo genérico de “Rust seguro”.
A CI atual não executa um scanner de advisories ou uma política de licenças
transitivas. `cargo audit` e `cargo deny` são opções a avaliar, sem status de checks
implementados.

## Licenças e recursos efetivamente usados

Conserve licenças e notices dos componentes redistribuídos. Licenças permissivas,
copyleft, fontes OFL, ícones ISC, dicionários e recursos Creative Commons não são
intercambiáveis. A presença de um arquivo de licença em um acervo não incorpora
seus recursos ao aplicativo nem altera a licença do projeto.

O frontend usa `default_fonts` de egui. Não foram incorporados logotipos, fontes,
dicionários, ícones ou fotos de outros projetos como consequência da adaptação
documental. O pacote macOS em `package/` é evidência local ignorada; não integra
os artefatos distribuídos. Gerar uma ilustração própria não concede direitos
sobre uma marca de terceiros que ela reproduza.

O modelo de segmentação possui origem e licença descritas em [Efeitos](EFEITOS.md).
O preparador mantém o texto da licença do modelo, os arquivos de licença do
runtime extraído e os notices dos pacotes CUDA. Mudança de modelo exige conferir
licença, revisão, hash, contrato de entrada e resultado; não apenas trocar a URL.

## Atribuição e proveniência

Para qualquer recurso novo versionado, registrar caminho, título, autor ou
fornecedor, URL, revisão/hash, licença e modificações. Manter a licença junto do
recurso quando exigido. Samples e corpus precisam de origem e autorização,
mesmo quando não são enviados no pacote final.

Créditos de contribuição não devem ser obtidos por adivinhação de nomes pessoais.
Estatísticas geradas, nomes escolhidos pelos participantes e créditos de modelos
são fontes distintas. O projeto não tem atualmente um gerador automático de
créditos. Definir esse mecanismo é trabalho próprio, não uma consequência da
existência deste manual.

## Reprodutibilidade e fornecedores

Fixar versões e revisar alterações de lockfile. O preparador confere SHA-256 de
modelos e runtime fixados e consulta hashes dos pacotes nativos versionados.
Isso não é uma verificação criptográfica independente de identidade do publicador.
GitHub Actions ainda são referidas por tags na CI; pinning por commit, attestations,
SBOM e verificação de assinatura são endurecimentos futuros.
