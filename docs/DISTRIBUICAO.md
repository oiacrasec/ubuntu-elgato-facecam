# Distribuição e publicação

A entrega atual é um binário Linux e scripts de instalação no usuário. Não há
instalador oficial de outros sistemas ou atualizador automático implementado.

## Artefatos existentes

`cargo build --release --locked` gera o binário com GUI, bandeja e suporte
compilado à IA. `--no-default-features` gera a CLI mínima; `--features ai` pode
acrescentar IA a esse build. Modelo e bibliotecas são carregados externamente,
sem download na inicialização.

`install-desktop-app.sh` compila e instala em `~/.local` por padrão, aceita
`--prefix` absoluto, `--no-build` e autostart opcional. O desinstalador preserva
configurações e backups. Os scripts não concedem sudoers, não instalam módulos
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

Não há consulta de novas versões dentro do aplicativo. Uma proposta de atualizador
precisa verificar origem e integridade, manter a versão utilizável após falha,
respeitar o canal escolhido e impedir substituição parcial de componentes.
Configurações e modelos têm ciclos de versão diferentes; não apagar presets ou
bibliotecas opcionais para completar um upgrade do binário.
