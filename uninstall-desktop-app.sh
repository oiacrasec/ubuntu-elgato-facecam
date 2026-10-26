#!/usr/bin/env bash
set -euo pipefail
INSTALL_PREFIX="${HOME}/.local"
while (($#)); do
    case "$1" in
        --prefix) INSTALL_PREFIX="${2:?Informe o prefixo}"; shift 2 ;;
        -h|--help) echo 'Uso: ./uninstall-desktop-app.sh [--prefix DIRETÓRIO]'; exit 0 ;;
        *) echo "Opção desconhecida: $1" >&2; exit 1 ;;
    esac
done
if [[ "$INSTALL_PREFIX" != /* || "$INSTALL_PREFIX" == / ]]; then echo 'Prefixo inválido.' >&2; exit 1; fi
BINARY="$INSTALL_PREFIX/bin/elgato-virtualcam"
if [[ -x "$BINARY" ]]; then
    "$BINARY" --quit >/dev/null 2>&1 || true
    "$BINARY" --remove-autostart
fi
rm -f -- "$BINARY" "${XDG_DATA_HOME:-$HOME/.local/share}/applications/elgato-virtualcam.desktop"
rm -f -- "$INSTALL_PREFIX/share/facecam-hub/locales/pt-BR.json" "$INSTALL_PREFIX/share/facecam-hub/locales/en-US.json"
echo 'Aplicativo removido. Configurações, backups e pacotes do sistema foram preservados.'
