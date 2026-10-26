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
if [[ "$INSTALL_PREFIX" != /* || "$INSTALL_PREFIX" == / || "$INSTALL_PREFIX" == *$'\n'* ]]; then echo 'Prefixo inválido.' >&2; exit 1; fi
INSTALL_PREFIX="$(realpath -m -- "$INSTALL_PREFIX")"
[[ "$INSTALL_PREFIX" != / ]] || { echo 'Prefixo inválido.' >&2; exit 1; }
BINARY="$INSTALL_PREFIX/bin/elgato-virtualcam"
ROOT="$INSTALL_PREFIX/lib/facecam-hub"
DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
AUTOSTART="${XDG_CONFIG_HOME:-$HOME/.config}/autostart/elgato-virtualcam.desktop"
ID="$(printf '%s' "$INSTALL_PREFIX" | sha256sum)"
ID="${ID%% *}"
OWNED=false
if [[ -e "$ROOT" || -L "$ROOT" ]]; then
    [[ ! -L "$ROOT" && -f "$ROOT/owner" ]] || { echo 'Registro de instalação ausente; nada foi removido.' >&2; exit 1; }
    mapfile -t OWNER < "$ROOT/owner"
    [[ "${OWNER[0]:-}" == facecam-hub-v1 && "${OWNER[1]:-}" == "$ID" && "${OWNER[2]:-}" == "$INSTALL_PREFIX" && ${#OWNER[@]} == 3 ]] || { echo 'Registro de instalação incompatível; nada foi removido.' >&2; exit 1; }
    exec 9>"$ROOT/install.lock"
    flock -n 9 || { echo 'Aguarde a instalação terminar.' >&2; exit 1; }
    OWNED=true
    if [[ -f "$ROOT/data-home" ]]; then DATA_HOME="$(cat -- "$ROOT/data-home")"; fi
    [[ "$DATA_HOME" == /* && "$DATA_HOME" != / ]] || { echo 'Diretório de dados inválido.' >&2; exit 1; }
fi
if [[ -x "$BINARY" ]] && "$BINARY" --status >/dev/null 2>&1; then
    "$BINARY" --quit >/dev/null || { echo 'Aguarde as operações do aplicativo antes de desinstalar.' >&2; exit 1; }
fi
remove_entry() {
    local entry="$1"
    [[ -f "$entry" ]] || return 0
    if grep -q -F -x -- "X-Facecam-Hub-Install-Id=$ID" "$entry"; then
        rm -f -- "$entry"
    elif ! grep -q '^X-Facecam-Hub-Install-Id=' "$entry" && grep -q -F -x 'Name=Facecam Hub' "$entry"; then
        # The previous installer did not write an ownership marker. Match its
        # escaped Exec path exactly, including a possible --minimized suffix.
        local legacy="$BINARY"
        legacy="${legacy//\\/\\\\}"
        legacy="${legacy//\"/\\\"}"
        legacy="${legacy//\$/\\\$}"
        legacy="${legacy//\`/\\\`}"
        legacy="${legacy//%/%%}"
        if grep -q -F -- "Exec=\"$legacy\"" "$entry"; then rm -f -- "$entry"; fi
    fi
}
if "$OWNED"; then
    remove_entry "$DATA_HOME/applications/elgato-virtualcam.desktop"
    remove_entry "$AUTOSTART"
    for RELATIVE in apps/facecam-hub.svg status/facecam-hub-symbolic.svg status/facecam-hub-active-symbolic.svg; do
        LINK="$DATA_HOME/icons/hicolor/scalable/$RELATIVE"
        if [[ -L "$LINK" && "$(readlink -- "$LINK")" == "$ROOT/current/icons/${RELATIVE##*/}" ]]; then rm -f -- "$LINK"; fi
    done
    if [[ -L "$BINARY" && "$(readlink -- "$BINARY")" == "$ROOT/current/elgato-virtualcam" ]]; then rm -f -- "$BINARY"; fi
    rm -rf -- "$ROOT"
else
    # Compatibility with the previous, unversioned installer.
    for ENTRY in "$DATA_HOME/applications/elgato-virtualcam.desktop" "$AUTOSTART"; do
        remove_entry "$ENTRY"
    done
    if [[ -f "$BINARY" && ! -L "$BINARY" ]]; then rm -f -- "$BINARY"; fi
fi
rm -f -- "$INSTALL_PREFIX/share/facecam-hub/locales/pt-BR.json" "$INSTALL_PREFIX/share/facecam-hub/locales/en-US.json"
echo 'Aplicativo removido. Configurações, modelos, recursos, backups e pacotes do sistema foram preservados.'
