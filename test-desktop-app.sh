#!/bin/bash
set -e

echo "🧪 Testing Elgato VirtualCam Desktop Application..."

DEFAULT_VENV_DIR="/opt/ubuntu-elgato-facecam"
INSTALL_META_FILE="$HOME/.config/elgato-virtualcam/install.env"
APP_CMD=(python3 virtualcam_app.py)

if [[ -f "$INSTALL_META_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$INSTALL_META_FILE"
fi

VENV_DIR="${VENV_DIR:-$DEFAULT_VENV_DIR}"
if [[ -x "$VENV_DIR/bin/elgato-virtualcam" ]]; then
    APP_CMD=("$VENV_DIR/bin/elgato-virtualcam")
elif command -v elgato-virtualcam >/dev/null 2>&1; then
    APP_CMD=("elgato-virtualcam")
fi

# Function to check command availability
check_command() {
    if command -v "$1" >/dev/null 2>&1; then
        echo "✅ $1 is available"
        return 0
    else
        echo "❌ $1 is not available"
        return 1
    fi
}

# Function to check if module is available
check_module() {
    if modinfo "$1" >/dev/null 2>&1; then
        echo "✅ $1 module is available"
        return 0
    else
        echo "❌ $1 module is not available"
        return 1
    fi
}

echo ""
echo "📋 System Requirements Check:"
check_command python3
check_command ffmpeg
check_command v4l2-ctl
check_module v4l2loopback

echo ""
echo "🐍 Python Dependencies Check:"
python3 -c "import PyQt5.QtWidgets; print('✅ PyQt5 is available')" 2>/dev/null || echo "❌ PyQt5 is not available"

echo ""
echo "🎥 Camera Detection Test:"
"${APP_CMD[@]}" --test-camera

echo ""
echo "🔧 Virtual Device Module Test:"
if lsmod | grep -q v4l2loopback; then
    echo "✅ v4l2loopback module is currently loaded"
    v4l2-ctl --list-devices | grep -A 3 "VirtualCam" || echo "ℹ️  VirtualCam device not found (this is normal if not manually loaded)"
else
    echo "ℹ️  v4l2loopback module not currently loaded (will be loaded by application)"
fi

echo ""
echo "📱 System Tray Test:"
if python3 -c "
from PyQt5.QtWidgets import QApplication, QSystemTrayIcon
import sys
app = QApplication([])
if QSystemTrayIcon.isSystemTrayAvailable():
    print('✅ System tray is available')
    sys.exit(0)
else:
    print('❌ System tray is not available')
    sys.exit(1)
" 2>/dev/null; then
    true
else
    echo "❌ System tray test failed"
fi

echo ""
echo "🚀 Application Startup Test (5 seconds):"
echo "Starting application in background..."
"${APP_CMD[@]}" &
APP_PID=$!
sleep 5

if kill -0 $APP_PID 2>/dev/null; then
    echo "✅ Application started successfully"
    echo "Stopping application..."
    kill $APP_PID
    wait $APP_PID 2>/dev/null || true
else
    echo "❌ Application failed to start"
fi

echo ""
echo "📁 Configuration Test:"
if [[ -d "$HOME/.config/elgato-virtualcam" ]]; then
    echo "✅ Configuration directory created"
    if [[ -f "$HOME/.config/elgato-virtualcam/config.json" ]]; then
        echo "✅ Configuration file exists"
        echo "Configuration contents:"
        cat "$HOME/.config/elgato-virtualcam/config.json" | head -10
    else
        echo "ℹ️  Configuration file will be created on first run"
    fi
else
    echo "ℹ️  Configuration directory will be created on first run"
fi

echo ""
echo "🔧 Autostart Test:"
if [[ -f "$HOME/.config/autostart/elgato-virtualcam.desktop" ]]; then
    echo "✅ Autostart entry exists"
    echo "Autostart file contents:"
    cat "$HOME/.config/autostart/elgato-virtualcam.desktop"
else
    echo "ℹ️  Autostart entry not installed (run with --install-autostart)"
fi

echo ""
echo "📊 Test Summary:"
echo "If all tests passed, you can run the application with:"
echo "  ${APP_CMD[*]}"
echo ""
echo "The application will appear as a camera icon in your system tray."
echo "Left-click to start/stop virtual camera, right-click for menu."
