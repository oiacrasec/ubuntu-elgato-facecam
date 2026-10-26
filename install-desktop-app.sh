#!/usr/bin/env bash
set -euo pipefail
PROJECT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_PREFIX="${HOME}/.local"
AUTOSTART=false
BUILD=true
while (($#)); do
    case "$1" in
        --prefix) INSTALL_PREFIX="${2:?Informe o prefixo}"; shift 2 ;;
        --autostart) AUTOSTART=true; shift ;;
        --no-build) BUILD=false; shift ;;
        -h|--help) echo 'Uso: ./install-desktop-app.sh [--prefix DIRETÓRIO] [--autostart] [--no-build]'; exit 0 ;;
        *) echo "Opção desconhecida: $1" >&2; exit 1 ;;
    esac
done
if [[ "$INSTALL_PREFIX" != /* || "$INSTALL_PREFIX" == / ]]; then
    echo 'Use um prefixo absoluto dedicado, como ~/.local.' >&2; exit 1
fi
cd -- "$PROJECT_DIR"
if "$BUILD"; then
    CARGO="$(command -v cargo || true)"
    [[ -n "$CARGO" ]] || CARGO="$HOME/.cargo/bin/cargo"
    [[ -x "$CARGO" ]] || { echo 'Instale Rust 1.95 ou posterior.' >&2; exit 1; }
    CARGO_INCREMENTAL=0 "$CARGO" build --release --locked
fi
TARGET_DIR="${CARGO_TARGET_DIR:-$PROJECT_DIR/target}"
BINARY="$TARGET_DIR/release/elgato-virtualcam"
[[ -x "$BINARY" ]] || { echo 'Binário ausente. Execute cargo build --release --locked.' >&2; exit 1; }
ARGS=(--install-desktop "$INSTALL_PREFIX")
if "$AUTOSTART"; then ARGS+=(--desktop-autostart); fi
"$BINARY" "${ARGS[@]}"
echo "Instalado: $INSTALL_PREFIX/bin/elgato-virtualcam"
echo 'Abra Facecam Hub no menu de aplicativos. Atualizações ficam em Configurações.'
echo 'Dependências de vídeo e módulos do sistema são preparados separadamente; consulte README.md.'
