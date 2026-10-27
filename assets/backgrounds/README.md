# Fundos originais do Camera Hub

Esta pasta contém as mesmas 15 imagens PNG e os mesmos sete vídeos MP4 do
Camera Hub Windows 2.3.0.7295, sem conversão ou alteração dos arquivos.
A inclusão foi solicitada expressamente pelo usuário em 27/10/2026.

O fornecedor é Elgato/Corsair; `NVIDIA_Broadcast.png` também contém a marca NVIDIA.
São recursos de terceiros, excluídos da licença MIT do código deste projeto.
Não lhes foi atribuída uma licença permissiva. A autorização do usuário registra
o escopo deste trabalho e não substitui as condições dos titulares.

O [registro de proveniência](provenance.json) relaciona cada nome original,
SHA-256, tamanho, resolução e duração. A tabela Component do MSI identifica
exatamente estes 22 arquivos como integrantes de `BackgroundsFolder`, incluindo
cinco imagens de cor sólida. `LUTs_base.png` pertence a `UIResources` e não faz
parte dos fundos. O instalador Windows não foi executado.

O pacote Linux copia os arquivos para `resources/images` e `resources/videos`
junto do executável instalado. A publicação verifica tamanho e SHA-256 de todos
os arquivos antes de trocar a versão ativa. A atualização usa os recursos do
mesmo checkout de produção que gerou o binário. Imagens e vídeos ficam fora do
executável, inclusive na compilação headless.

Na aba **Efeitos**, ative **Ativar fundo com IA**, escolha **Minha imagem** ou
**Meu vídeo** e selecione um dos nomes do catálogo. Clique em **Aplicar e salvar**.
Os vídeos são reproduzidos em loop, sem áudio. A composição existente preenche
um quadro com recorte central; os arquivos originais permanecem intactos.
Os três cenários ilustrados próprios continuam sendo opções separadas.
