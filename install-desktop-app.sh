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
if "$BUILD"; then
    command -v cargo >/dev/null || { echo 'Instale Rust e adicione ~/.cargo/bin ao PATH.' >&2; exit 1; }
    (cd "$PROJECT_DIR" && cargo build --release --locked)
fi
BINARY="$PROJECT_DIR/target/release/elgato-virtualcam"
[[ -x "$BINARY" ]] || { echo 'Binário ausente. Execute cargo build --release --locked.' >&2; exit 1; }
install -Dm755 "$BINARY" "$INSTALL_PREFIX/bin/elgato-virtualcam"
DESKTOP_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
mkdir -p "$DESKTOP_DIR"
# Escape a path for the Desktop Entry Exec grammar (not shell syntax).
EXECUTABLE="$INSTALL_PREFIX/bin/elgato-virtualcam"
EXECUTABLE="${EXECUTABLE//\\/\\\\}"
EXECUTABLE="${EXECUTABLE//\"/\\\"}"
EXECUTABLE="${EXECUTABLE//\$/\\\$}"
EXECUTABLE="${EXECUTABLE//\`/\\\`}"
EXECUTABLE="${EXECUTABLE//%/%%}"
cat > "$DESKTOP_DIR/elgato-virtualcam.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Facecam Hub
Comment=Controle da Facecam e câmera virtual
Exec="$EXECUTABLE"
Icon=camera-web
Terminal=false
Categories=AudioVideo;Video;
StartupNotify=true
EOF
if "$AUTOSTART"; then "$INSTALL_PREFIX/bin/elgato-virtualcam" --install-autostart; fi
echo "Instalado: $INSTALL_PREFIX/bin/elgato-virtualcam"
echo 'Dependências de execução: ffmpeg, v4l-utils e v4l2loopback-dkms.'
echo 'Prepare a saída virtual conforme README.md; o instalador não modifica módulos ou permissões do sistema.'
