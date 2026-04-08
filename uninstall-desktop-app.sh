#!/bin/bash
set -euo pipefail

DEFAULT_VENV_DIR="/opt/ubuntu-elgato-facecam"
CONFIG_DIR="$HOME/.config/elgato-virtualcam"
INSTALL_META_FILE="$CONFIG_DIR/install.env"
VENV_DIR="$DEFAULT_VENV_DIR"
VENV_BIN="$VENV_DIR/bin"
LOCAL_ENTRYPOINT="$HOME/.local/bin/elgato-virtualcam"
VENV_DIR_OVERRIDE=""

is_safe_venv_path() {
    case "$1" in
        ""|"/"|"/opt"|"/usr"|"/home"|"/var"|"/etc")
            return 1
            ;;
        *)
            return 0
            ;;
    esac
}

usage() {
    cat <<EOF
Usage: ./uninstall-desktop-app.sh [--venv-path <path>]

Options:
  --venv-path <path>   Force the venv path to remove (overrides install metadata).
  -h, --help           Show this help message.
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --venv-path)
            if [[ $# -lt 2 ]]; then
                echo "❌ Missing value for --venv-path"
                usage
                exit 1
            fi
            VENV_DIR_OVERRIDE="$2"
            shift 2
            ;;
        --venv-path=*)
            VENV_DIR_OVERRIDE="${1#*=}"
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "❌ Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

if [[ -f "$INSTALL_META_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$INSTALL_META_FILE"
fi

if [[ -n "$VENV_DIR_OVERRIDE" ]]; then
    VENV_DIR="$VENV_DIR_OVERRIDE"
    VENV_BIN="$VENV_DIR/bin"
fi

VENV_BIN="${VENV_BIN:-$VENV_DIR/bin}"
APP_ENTRYPOINT="$VENV_BIN/elgato-virtualcam"

echo "🛑 Uninstalling Elgato VirtualCam Desktop Application..."
echo "📍 Detected venv path: $VENV_DIR"

# Stop running application
echo "🔄 Stopping running application..."
pkill -f "virtualcam_app.py|elgato-virtualcam" || echo "   No running application found"

# Remove autostart entry
echo "🗑️  Removing autostart entry..."
AUTOSTART_FILE="$HOME/.config/autostart/elgato-virtualcam.desktop"
if [[ -f "$AUTOSTART_FILE" ]]; then
    rm -f "$AUTOSTART_FILE"
    echo "   ✅ Removed: $AUTOSTART_FILE"
else
    echo "   ⚠️  Autostart file not found"
fi

# Remove local command shim
if [[ -L "$LOCAL_ENTRYPOINT" || -f "$LOCAL_ENTRYPOINT" ]]; then
    ENTRYPOINT_TARGET="$(readlink -f "$LOCAL_ENTRYPOINT" 2>/dev/null || true)"
    if [[ "$ENTRYPOINT_TARGET" == "$APP_ENTRYPOINT" ]]; then
        rm -f "$LOCAL_ENTRYPOINT"
        echo "   ✅ Removed local entrypoint: $LOCAL_ENTRYPOINT"
    else
        echo "   ⚠️  Keeping $LOCAL_ENTRYPOINT (points to a different target)"
    fi
fi

# Remove virtual environment (with confirmation)
if [[ -d "$VENV_DIR" ]]; then
    if ! is_safe_venv_path "$VENV_DIR"; then
        echo "   ⚠️  Refusing to remove unsafe venv path: $VENV_DIR"
    elif [[ ! -f "$VENV_DIR/pyvenv.cfg" ]]; then
        echo "   ⚠️  $VENV_DIR does not look like a Python virtual environment (missing pyvenv.cfg)"
        echo "   ⚠️  Skipping automatic removal for safety"
    else
        read -p "❓ Remove virtual environment directory $VENV_DIR? [y/N]: " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            if ! rm -rf "$VENV_DIR" 2>/dev/null; then
                sudo rm -rf "$VENV_DIR"
            fi
            echo "   ✅ Removed virtual environment directory"
        else
            echo "   ⚠️  Keeping virtual environment directory"
        fi
    fi
else
    echo "   ⚠️  Virtual environment directory not found"
fi

# Remove configuration directory (with confirmation)
if [[ -d "$CONFIG_DIR" ]]; then
    read -p "❓ Remove configuration directory $CONFIG_DIR? [y/N]: " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        rm -rf "$CONFIG_DIR"
        echo "   ✅ Removed configuration directory"
    else
        echo "   ⚠️  Keeping configuration directory"
    fi
else
    echo "   ⚠️  Configuration directory not found"
fi

# Remove sudoers file
echo "🔒 Removing sudoers permissions..."
SUDOERS_FILE="/etc/sudoers.d/elgato-virtualcam"
if [[ -f "$SUDOERS_FILE" ]]; then
    sudo rm -f "$SUDOERS_FILE"
    echo "   ✅ Removed: $SUDOERS_FILE"
else
    echo "   ⚠️  Sudoers file not found"
fi

# Optionally remove user from video group
read -p "❓ Remove user from video group? [y/N]: " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    sudo gpasswd -d "$USER" video || echo "   ⚠️  User not in video group"
    echo "   ✅ Removed from video group"
else
    echo "   ⚠️  Keeping video group membership"
fi

# Unload v4l2loopback module
echo "🧯 Stopping virtual camera..."
if lsmod | grep -q v4l2loopback; then
    sudo modprobe -r v4l2loopback && echo "   ✅ v4l2loopback module unloaded"
else
    echo "   ⚠️  v4l2loopback module not loaded"
fi

echo ""
echo "✅ Uninstall complete!"
echo ""
echo "Note: System dependencies (ffmpeg, v4l2loopback-dkms, python3-pyqt5) were not removed"
echo "      as they may be used by other applications."
echo ""
echo "To completely remove system dependencies:"
echo "  sudo apt remove v4l2loopback-dkms ffmpeg python3-pyqt5"
