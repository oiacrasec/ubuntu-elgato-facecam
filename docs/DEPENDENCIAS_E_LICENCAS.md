# Dependências, recursos e licenças

O código deste projeto usa a licença [MIT](../LICENSE). Bibliotecas, modelos,
fontes e ferramentas externas têm licenças e condições próprias.

## Inventário de dependências

Os Looks Quente, Frio e Contraste suave são código e transformações próprios,
sob MIT. Recursos importados pelo usuário mantêm sua licença de origem; o app
não inclui LUTs, fundos, ícones ou shaders extraídos do Camera Hub.

| Componente | Uso atual |
| --- | --- |
| Rust/Cargo e `Cargo.lock` | Compilação e versões resolvidas |
| egui/eframe e Glow | Janela nativa, OpenGL e acessibilidade compilada |
| `rfd` 0.17.2 | Seletor nativo de arquivos pelo XDG Desktop Portal; MIT; [fonte e API](https://docs.rs/rfd/0.17.2/rfd/) |
| ksni | Bandeja StatusNotifierItem por D-Bus, opcional |
| FFmpeg | Captura, conversão, filtros e saída de vídeo |
| v4l-utils / `v4l2-ctl` | Consulta e escrita de controles e formatos |
| v4l2loopback | Nó virtual preparado no sistema pelo usuário |
| `ort` | Bindings Rust opcionais para ONNX Runtime carregado dinamicamente |
| `aes` 0.8 / `cbc` 0.1 / `sha2` 0.10 | RustCrypto, MIT ou Apache-2.0; leitura e conferência do modelo local, sem chaves embutidas |
| `tempfile` | Staging da preparação RTX e testes, MIT ou Apache-2.0 |
| `image` | Decodificação opcional de PNG/JPEG/WebP/BMP e preparo de fundo; MIT ou Apache-2.0 |
| ONNX Runtime e modelo | Inferência opcional, fora do binário do aplicativo |
| CUDA e cuDNN | Provider NVIDIA, com dependências externas opcionais |
| Video Effects 0.7.6 Ada | Modelo local importado para segmentação Rust/TensorRT; componentes externos sujeitos à EULA NVIDIA, sem redistribuição no Git/binário |
| AR 0.8.2 Ada | Referência Windows inventariada, sem backend implementado; [origem](NVIDIA_MAXINE.md) |
| TensorRT 10.4.0.26 / ONNX Runtime GPU 1.20.1 | Execução nativa do modelo Ada, componentes preparados no usuário; ONNX Runtime sob MIT, TensorRT sob licença NVIDIA |
| 7z, tar, unzip, dpkg-deb e setpriv | Extração e supervisão da preparação RTX; ferramentas Ubuntu externas, sem execução do instalador Windows |
| APT, pkexec, curl e sha256sum | Instalação assistida opcional, com confirmação e autenticação do Ubuntu |

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

O seletor usa o portal do desktop, sem vincular o aplicativo às bibliotecas GTK.
No Ubuntu, requer um backend de portal de seleção de arquivos; `rfd` pode usar
Zenity como alternativa. O campo de caminho continua disponível. A dependência
usa integração nativa dentro de sua implementação; o código do aplicativo
permanece sem `unsafe`, e a compilação headless não inclui o seletor.

A chave pública de assinatura NVIDIA usada no cadastro APT vem de
[3bf863cc.pub](https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2404/x86_64/3bf863cc.pub),
com SHA-256 `7eb71103e32e813ea3e1c06bdb01b143f36feb8d83ecac2b0c11c9273f9e6822`
conferido em 25/10/2026. É baixada após confirmação; não inclui runtime, modelo
ou recurso visual proprietário do Camera Hub. A origem e o digest precisam ser
revistos se a NVIDIA substituir a chave.

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
