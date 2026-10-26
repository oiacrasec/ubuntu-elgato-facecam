# Distribuição e publicação

Instale o Facecam Hub no usuário e atualize em **Configurações**, perto de
**Idioma**. A atualização usa a branch padrão de produção do repositório.

## Artefatos existentes

`cargo build --release --locked` gera o binário com GUI, bandeja e suporte
compilado à IA. `--no-default-features` gera a CLI mínima; `--features ai` pode
acrescentar IA a esse build. Modelo e bibliotecas são carregados externamente,
sem download na inicialização.

`install-desktop-app.sh` compila e instala em `~/.local` por padrão, aceita
`--prefix` absoluto, `--no-build` e autostart opcional. O desinstalador preserva
configurações, modelos, recursos e backups. Os scripts não concedem sudoers, não instalam módulos
nem mudam grupos. A CI Ubuntu 24.04 verifica código e produz um artefato de build;
isso não é evidência de publicação, assinatura ou teste de todos os consumidores.

## Checklist de uma publicação futura

1. Escolher revisão, versão e notas com comportamentos e limitações comprovados.
2. Executar os checks comuns e os cenários de hardware aplicáveis, sem prometer
   uma execução remota de CI que não foi observada.
3. Compilar variantes e registrar arquitetura, toolchain, flags e dependências.
4. Testar instalação, autostart, upgrade e remoção em prefixo temporário e conta limpa.
5. Conferir entradas desktop, permissões, ícones, caminhos com espaços e preservação do usuário.
6. Registrar hashes, licenças, origem dos modelos opcionais e resultados de smoke tests.
7. Publicar por um canal definido, com política de suporte e de reporte de falhas.

Não criar uma tag ou enviar artefato a terceiros apenas por editar este manual.
Segredos de assinatura, tokens e credenciais não pertencem ao repositório, logs ou
fixtures. Um processo futuro precisa definir quem pode promover um build e como
verificar os bytes que a pessoa realmente baixa.

## Pacotes e portabilidade

AppImage, `.deb`, `.rpm`, Flatpak e `.tar.gz` são formatos possíveis, ainda não
implementados como matriz de distribuição. Cada um exige dependências, permissões
de câmera/GPU, integração de desktop e teste de dados persistentes. Um sandbox
pode precisar de acesso específico a dispositivos e bibliotecas NVIDIA.

DMG, assinatura/notarização macOS, MSI/assinatura Windows e pacotes FreeBSD não
são entregas Linux. Caso o escopo seja ampliado, documentar assinatura de dentro
para fora, validação do arquivo baixado, reputação do executável, targets mínimos
e atualizações. Layout e recursos visuais de um instalador precisam ser próprios,
legíveis e livres de colisão com ícones e rótulos do sistema.

A integração web é apenas um estudo em [Extensões](EXTENSOES.md). Não há Docker,
loader wasm, endpoint de atualização ou site hospedado para instalar. Empacotar
um frontend não fornece automaticamente um backend de câmera compatível.

## Atualização e recuperação

Em **Configurações**, a versão aparece abaixo de **Idioma**. Clique em
**Verificar atualizações**, **Atualizar agora** e, após concluir, **Reiniciar agora**.
A janela instalada também consulta atualizações ao abrir, sem instalar nada.
Pare a captura antes de instalar ou reiniciar. Uma execução direta de
`target/release` pode consultar, mas precisa do instalador para atualizar.

O canal é o HEAD remoto de
[ubuntu-elgato-facecam](https://github.com/oiacrasec/ubuntu-elgato-facecam),
cuja branch padrão é `master` nesta entrega. `main` e `master` são nomes
alternativos para o mesmo papel de produção: não há dois canais nem fallback
para outra branch. HEAD em qualquer outro nome é recusado.

A consulta usa Git em segundo plano. Ao instalar, o aplicativo fixa o commit
consultado, baixa para um diretório temporário, exige continuidade do histórico
da versão instalada e compila com `cargo build --locked --release`, preservando
as features GUI/bandeja/IA da instalação. São necessários Git, Rust 1.95 ou
posterior, as dependências de build do [README](../README.md) e espaço livre.
Não há instalação APT implícita, assinatura de release ou download de binário
pré-compilado nesse fluxo. O padrão de interação foi inspirado no
[Arctis Sound Manager](https://github.com/loteran/Arctis-Sound-Manager);
a implementação é própria em Rust.

O pacote inclui binário, catálogos, ícones próprios e registro de versão, commit,
features e SHA-256. Fica em `PREFIX/lib/facecam-hub/versions/`. O link `current`
só muda após preparar todos os arquivos; `PREFIX/bin/elgato-virtualcam`, o menu
e o autostart usam esse link estável. Arquivos alterados são restaurados se a
publicação falhar. Bundles anteriores permanecem disponíveis, inclusive para um
processo que ainda esteja aberto; falha de download/build não troca a instalação.
Os diretórios de configuração, modelos e recursos não fazem parte da substituição.

O log é `$XDG_CACHE_HOME/facecam-hub/updates/update.log`, ou
`~/.cache/facecam-hub/updates/update.log`. O mesmo diretório guarda o cache de
compilação; clones temporários são removidos ao terminar. Quit aguarda uma
instalação em andamento. Reiniciar cria a nova instância e espera a anterior
liberar o controlador; não inicia captura automaticamente.

O instalador aceita `--autostart`, `--no-build`, `--prefix` e respeita
`CARGO_TARGET_DIR`. O desinstalador usa o registro do prefixo e remove os atalhos
e ícones pertencentes a ele, mesmo sem o executável. Entradas de outro prefixo,
backups, bibliotecas do usuário e pacotes do sistema são preservados.

Para recuperação manual, com o aplicativo fechado, aponte `current` para um
bundle anterior em `versions`, ou execute novamente o instalador a partir de uma
revisão conhecida. Os bundles antigos contêm também seus catálogos e ícones.
