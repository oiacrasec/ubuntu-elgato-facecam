# Integrar os efeitos NVIDIA Maxine

O backend Maxine ainda não está implementado. A IA atual usa ONNX Runtime/CUDA;
instalar SDK Manager não instala os runtimes e modelos VFX/AR.

## Pacotes necessários

A documentação oficial consultada em 25/10/2026 distribui Core e recursos
separadamente pelo NGC. O Core contém headers, bibliotecas de base e dependências;
os efeitos precisam de suas próprias bibliotecas e modelos. O download de recursos
usa chave pessoal NGC e acesso da conta. Não informe essa chave no código,
configuração do Facecam Hub ou mensagens de suporte.

| Finalidade | Pacote ou recurso NVIDIA |
| --- | --- |
| Base VFX Linux | VFX SDK Core, pacote `VFXSDK_linux_<version>.tgz` |
| Segmentação e desfoque | `nvVFXGreenScreen` e `nvVFXBackgroundBlur` |
| Base AR Linux | AR SDK Core, pacote `ARSDK_linux_<version>.tgz` |
| Correção de olhar | `nvargazeredirection` |

Fontes: [instalação VFX Linux](https://docs.nvidia.com/maxine/vfx/latest/LinuxVFXSDK/InstalltheVFXSDK.html),
[instalação AR Linux](https://docs.nvidia.com/maxine/ar/latest/LinuxARSDK/InstalltheARSDK.html)
e [requisitos do exemplo de segmentação](https://github.com/NVIDIA-Maxine/VFX-SDK-Samples/tree/main/apps/AigsEffectApp).
O catálogo contém versões para diferentes plataformas. Um pacote Windows, inclusive
WoA, não fornece bibliotecas ELF para Linux x86-64.

APT continua sendo a preferência para dependências do Ubuntu. As instruções NVIDIA
consultadas distribuem os SDKs Maxine em arquivos de SDK e recursos NGC; não indicam
um pacote APT que entregue esses efeitos. Não substituir isso por `sdkmanager`
nem criar um botão que anuncie instalação dos efeitos sem os componentes reais.

## Compatibilidade Linux e RTX

A [página oficial de requisitos VFX Linux](https://docs.nvidia.com/maxine/vfx/latest/LinuxVFXSDK/GetStartedonLinux.html)
inclui Ubuntu 24.04 e arquiteturas NVIDIA Ada, mas descreve o SDK como voltado a
servidores e informa ausência de suporte oficial à integração local em aplicações
cliente. A lista do instalador de recursos cita GPUs de servidor, sem a RTX 4070
SUPER deste computador. Isso exige verificar acesso, licença, pacote de modelo e
execução real; a arquitetura da GPU por si só não comprova suporte do efeito.

O usuário confirmou o funcionamento integral no Camera Hub Windows com a mesma
RTX. Esse comportamento permanece como alvo de aceitação. A restrição publicada
para Linux não invalida o relato Windows e não comprova que o efeito seja impossível
no Linux; também não permite prometer sua execução antes do teste.

## Contrato da implementação futura

Os [exemplos oficiais](https://github.com/NVIDIA-Maxine/VFX-SDK-Samples) usam C++,
headers do SDK e modelos externos. `AigsEffectApp` aceita webcam ou arquivo e oferece
saída de vídeo em arquivo. A interface documentada não fornece o protocolo RGB por
pipes usado pelo worker Rust atual. Invocar o exemplo por frame exigiria inicializar
o SDK repetidamente e não é uma integração adequada para vídeo em tempo real.

Antes de implementar o backend:

1. Obter os pacotes Linux correspondentes e confirmar as condições de uso.
2. Executar segmentação/desfoque e gaze nos exemplos oficiais na RTX deste host.
3. Escolher uma interface segura compatível com Rust sem `unsafe` no produto;
   não esconder bindings próprios inseguros em um crate auxiliar.
4. Preservar controlador, processo de inferência isolado, aquecimento antes da
   troca, recursos automáticos e mensagens de indisponibilidade sem fallback CPU.
5. Medir imagem, latência, FPS, CPU/GPU e estabilidade no consumidor virtual,
   comparando os mesmos cenários autorizados com o Windows.

Nesta revisão, a busca local não encontrou os runtimes/modelos Maxine em Downloads,
`/opt/nvidia` ou `/usr/local`. A integração depende desses pacotes e da validação
de execução; a pesquisa dos exemplos não constitui um backend pronto.
