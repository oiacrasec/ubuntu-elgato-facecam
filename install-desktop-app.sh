#!/bin/bash
set -euo pipefail

DEFAULT_VENV_DIR="/opt/ubuntu-elgato-facecam"
VENV_DIR="${ELGATO_VENV_DIR:-$DEFAULT_VENV_DIR}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

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
Usage: ./install-desktop-app.sh [--venv-path <path>]

Options:
  --venv-path <path>   Virtual environment install path.
                       Default: $DEFAULT_VENV_DIR
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
            VENV_DIR="$2"
            shift 2
            ;;
        --venv-path=*)
            VENV_DIR="${1#*=}"
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

if command -v realpath >/dev/null 2>&1; then
    VENV_DIR="$(realpath -m "$VENV_DIR")"
fi

if ! is_safe_venv_path "$VENV_DIR"; then
    echo "❌ Unsafe venv path: $VENV_DIR"
    echo "   Please choose a dedicated directory, e.g. /opt/ubuntu-elgato-facecam"
    exit 1
fi

VENV_BIN="$VENV_DIR/bin"
APP_ENTRYPOINT="$VENV_BIN/elgato-virtualcam"
LOCAL_BIN="$HOME/.local/bin"
LOCAL_ENTRYPOINT="$LOCAL_BIN/elgato-virtualcam"
INSTALL_META_DIR="$HOME/.config/elgato-virtualcam"
INSTALL_META_FILE="$INSTALL_META_DIR/install.env"

echo "🚀 Installing Elgato VirtualCam Desktop Application..."
echo "📍 Virtual environment path: $VENV_DIR"

# Check for existing installation
if [[ -f "$HOME/.config/autostart/elgato-virtualcam.desktop" ]] || pgrep -f "virtualcam_app.py|elgato-virtualcam" >/dev/null; then
    echo "⚠️  Existing installation detected!"
    read -p "❓ Reinstall? This will stop the current app and update files [y/N]: " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "🔄 Stopping existing application..."
        pkill -f "virtualcam_app.py|elgato-virtualcam" || echo "   No running application found"
        echo "✅ Proceeding with reinstall..."
    else
        echo "❌ Installation cancelled"
        exit 0
    fi
fi

# Install system dependencies
echo "📦 Installing system dependencies..."
sudo apt update
sudo apt install -y v4l-utils v4l2loopback-dkms ffmpeg python3 python3-pip python3-venv python3-pyqt5

# Prepare venv directory
if [[ -e "$VENV_DIR" && ! -d "$VENV_DIR" ]]; then
    echo "❌ Venv path exists but is not a directory: $VENV_DIR"
    exit 1
fi

if [[ -d "$VENV_DIR" && ! -f "$VENV_DIR/pyvenv.cfg" ]]; then
    if find "$VENV_DIR" -mindepth 1 -maxdepth 1 | read -r _; then
        echo "❌ Existing directory is not a virtual environment: $VENV_DIR"
        echo "   Use --venv-path with an empty/new directory, or remove this directory first."
        exit 1
    fi
fi

if [[ ! -d "$VENV_DIR" ]]; then
    if ! mkdir -p "$VENV_DIR" 2>/dev/null; then
        echo "🔐 Creating $VENV_DIR with sudo..."
        sudo mkdir -p "$VENV_DIR"
        sudo chown "$USER:$USER" "$VENV_DIR"
    fi
elif [[ ! -w "$VENV_DIR" ]]; then
    echo "🔐 Updating ownership for $VENV_DIR..."
    if [[ -f "$VENV_DIR/pyvenv.cfg" ]]; then
        sudo chown -R "$USER:$USER" "$VENV_DIR"
    else
        sudo chown "$USER:$USER" "$VENV_DIR"
    fi
fi

if [[ -f "$VENV_DIR/pyvenv.cfg" && -e "$VENV_BIN/pip" && ! -w "$VENV_BIN/pip" ]]; then
    echo "🔐 Fixing permissions on existing virtual environment..."
    sudo chown -R "$USER:$USER" "$VENV_DIR"
fi

# Create or reuse virtual environment
echo "🐍 Setting up virtual environment..."
if [[ ! -f "$VENV_DIR/pyvenv.cfg" ]]; then
    python3 -m venv --system-site-packages "$VENV_DIR"
else
    echo "♻️  Reusing existing virtual environment at $VENV_DIR"
fi

# Install Python package into venv
echo "📦 Installing Python package into virtual environment..."
"$VENV_BIN/pip" install -e "$SCRIPT_DIR"

# Keep a stable command in ~/.local/bin
mkdir -p "$LOCAL_BIN"
ln -sf "$APP_ENTRYPOINT" "$LOCAL_ENTRYPOINT"

# Persist install metadata for uninstall/maintenance
mkdir -p "$INSTALL_META_DIR"
{
    printf 'VENV_DIR=%q\n' "$VENV_DIR"
    printf 'VENV_BIN=%q\n' "$VENV_BIN"
    printf 'INSTALL_DATE=%q\n' "$(date -Iseconds)"
} > "$INSTALL_META_FILE"

# Set up permissions for v4l2loopback management
echo "🔒 Setting up permissions for virtual camera management..."
sudo usermod -a -G video "$USER"

# Create sudoers rule for modprobe commands (no password required)
SUDOERS_FILE="/etc/sudoers.d/elgato-virtualcam"
sudo tee "$SUDOERS_FILE" > /dev/null <<EOF
# Allow users in video group to manage v4l2loopback module without password
%video ALL=(root) NOPASSWD: /sbin/modprobe v4l2loopback*, /sbin/modprobe -r v4l2loopback*
$USER ALL=(root) NOPASSWD: /sbin/modprobe v4l2loopback*, /sbin/modprobe -r v4l2loopback*
EOF

echo "✅ Permissions configured - virtual camera can auto-recover from device corruption"

# Setup USB power management to prevent device corruption
echo "🔌 Setting up USB power management..."
UDEV_RULE="/etc/udev/rules.d/99-elgato-nosuspend.rules"
echo 'ACTION=="add", SUBSYSTEM=="usb", ATTR{idVendor}=="0fd9", ATTR{idProduct}=="0078", ATTR{power/autosuspend}="-1"' | sudo tee "$UDEV_RULE" > /dev/null
sudo udevadm control --reload-rules
echo "   ✅ USB autosuspend disabled for Elgato Facecam (prevents corruption)"

# Install autostart entry
echo "🔧 Setting up autostart..."
"$APP_ENTRYPOINT" --install-autostart

# Test camera detection
echo "🎥 Testing camera detection..."
if "$APP_ENTRYPOINT" --test-camera; then
    echo "✅ Camera detection successful!"
else
    echo "⚠️  Camera not detected - make sure Elgato Facecam is connected"
fi

# Start instructions
echo "🚀 Starting VirtualCam application..."
echo "ℹ️  Note: Group membership changes require a fresh shell session"
echo ""
echo "💡 Please run these commands to start the app:"
echo "   exec bash"
echo "   $LOCAL_ENTRYPOINT &"
echo ""
echo "📱 The camera icon will appear in your system tray when running"

echo ""
echo "✅ Installation complete!"
echo ""
echo "Usage:"
echo "  # Run the application (recommended)"
echo "  $LOCAL_ENTRYPOINT"
echo ""
echo "  # Run using explicit venv path"
echo "  $APP_ENTRYPOINT"
echo ""
echo "  # The application will also start automatically on login"
echo "  # Look for the camera icon in your system tray"
echo ""
echo "Commands:"
echo "  # Test camera detection"
echo "  $APP_ENTRYPOINT --test-camera"
echo ""
echo "  # Install/reinstall autostart"
echo "  $APP_ENTRYPOINT --install-autostart"
