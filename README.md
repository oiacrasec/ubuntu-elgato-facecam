# Elgato VirtualCam for Linux

A professional desktop application for using the **Elgato Facecam** as a virtual webcam on Ubuntu Linux. This unified application creates a virtual webcam device (e.g. `/dev/video10`) that works seamlessly with Chrome, OBS, Zoom, and other video applications.

**✅ NEW: Desktop Application Architecture** - Replaces the problematic systemd approach with a reliable, single-process desktop application.

---

## 📦 Features

- **🎥 Auto-detection** of Elgato Facecam
- **🔄 Automatic transcoding** to browser-friendly `yuv420p` format
- **🖥️ Single desktop application** - No systemd permission issues
- **🖱️ Integrated system tray** with real-time status indicators
- **🎨 Dynamic status icons** (green=streaming, gray=ready, red=error)
- **📱 Desktop notifications** when streaming starts/stops
- **⚙️ JSON configuration** with GUI settings (coming soon)
- **🏗️ Professional packaging** ready for PyPI distribution

---

## 🚀 Installation

### Quick Start

```bash
git clone https://github.com/Aaronontheweb/ubuntu-elgato-facecam.git
cd ubuntu-elgato-facecam
./install-desktop-app.sh
```

Optional custom venv path:

```bash
./install-desktop-app.sh --venv-path /my/custom/path
```

The script will:
- Install system dependencies (`v4l-utils`, `v4l2loopback-dkms`, `ffmpeg`, `python3-pyqt5`, `python3-venv`)
- Create/use a virtual environment (default: `/opt/ubuntu-elgato-facecam`)
- Install the app into that virtual environment
- Configure desktop autostart integration
- Test camera detection

**What you get:**
- 🎥 Virtual webcam available at `/dev/video10`
- 🖱️ System tray camera icon for easy control
- 🚀 Auto-starts on login via desktop integration
- ⚙️ Professional configuration management
- 🐍 Stable command shim at `~/.local/bin/elgato-virtualcam`

**After installation:**
- Refresh your shell: `exec bash`
- Start the app in background: `~/.local/bin/elgato-virtualcam &`
- Look for the camera icon in your system tray
- The app will auto-start on future logins

### 🗑️ Uninstalling

To completely remove the desktop application:

```bash
./uninstall-desktop-app.sh
```

If the venv was installed in a custom location and metadata is missing:

```bash
./uninstall-desktop-app.sh --venv-path /my/custom/path
```

This will:
- Stop the running application
- Remove autostart entry
- Optionally remove the virtual environment
- Remove configuration (with confirmation)
- Remove sudoers permissions
- Optionally remove from video group

### Legacy Files (Systemd - Deprecated)

⚠️ **Not recommended** - The old systemd files remain for reference:
```bash
./install.sh        # Has known permission issues
```

---

## 🖱️ Using the Desktop Application

After installation, you'll see a **camera icon** in your system tray that provides easy control:

### 📱 Quick Actions
- **Left-click**: Toggle streaming on/off
- **Right-click**: Open context menu with options:
  - Start/Stop VirtualCam
  - Settings (coming soon)
  - Status & Logs
  - Refresh
  - Quit

### 🎨 Status Indicators
- **🟢 Green icon**: VirtualCam is streaming
- **⚫ Gray icon**: VirtualCam is ready (camera detected)
- **🔴 Red icon**: Camera not detected or error
- **🟡 Amber icon**: Starting up

### 🔧 Command Line Interface

```bash
VENV_BIN="/opt/ubuntu-elgato-facecam/bin"

# Run the application in background
"$VENV_BIN/elgato-virtualcam" &

# Test camera detection
"$VENV_BIN/elgato-virtualcam" --test-camera

# Install autostart
"$VENV_BIN/elgato-virtualcam" --install-autostart

# Debug mode (foreground for logs)
"$VENV_BIN/elgato-virtualcam" --debug

# If ~/.local/bin is in PATH, you can also use:
elgato-virtualcam
```

---

## 🧪 Testing & Verification

### ✅ Comprehensive Test Suite

```bash
# Run all tests
./test-desktop-app.sh
```

This tests:
- System dependencies
- Camera detection
- Virtual device creation
- Application startup
- Configuration management

### ✅ Manual Testing

```bash
# Check virtual device exists
v4l2-ctl --list-devices

# Test with cheese
cheese -d /dev/video10

# Test camera detection only
~/.local/bin/elgato-virtualcam --test-camera
```

### ✅ Browser Testing

Open these in your browser and select **"VirtualCam"**:
- [https://webcamtests.com](https://webcamtests.com)
- Google Meet camera settings
- Zoom camera settings

---

## 🔧 Troubleshooting

### 🎥 Camera Not Detected

```bash
# Check if Elgato is connected
lsusb | grep -i elgato

# Test detection manually
~/.local/bin/elgato-virtualcam --test-camera

# Check video devices
v4l2-ctl --list-devices
```

### 🔧 Virtual Device Issues

```bash
# Remove old module and restart
sudo modprobe -r v4l2loopback
~/.local/bin/elgato-virtualcam  # Will reload module automatically
```

### 📱 System Tray Not Visible

**GNOME users:**
```bash
# Install AppIndicator extension
sudo apt install gnome-shell-extension-appindicator
```

**General:**
- Verify PyQt5: `python3 -c "import PyQt5; print('OK')"`
- Run directly: `~/.local/bin/elgato-virtualcam --debug`

### 📋 View Application Logs

```bash
# Application logs
tail -f ~/.config/elgato-virtualcam/virtualcam.log

# FFmpeg error logs  
tail -f ~/.config/elgato-virtualcam/virtualcam.err.log
```

### 🔄 Reset Configuration

```bash
# Remove config to reset to defaults
rm -rf ~/.config/elgato-virtualcam/config.json
```

---

## 🧼 Uninstallation

### Desktop App Removal

```bash
# Stop application
pkill -f "virtualcam_app.py|elgato-virtualcam"

# Remove autostart
rm ~/.config/autostart/elgato-virtualcam.desktop

# Remove default venv (or your custom --venv-path)
sudo rm -rf /opt/ubuntu-elgato-facecam

# Remove configuration
rm -rf ~/.config/elgato-virtualcam/

# Remove virtual device module
sudo modprobe -r v4l2loopback
```

### Legacy Systemd Removal

```bash
# For old systemd installation
./uninstall.sh
```

**System packages remain** (safe to keep):
- `v4l-utils`, `v4l2loopback-dkms`, `ffmpeg`, `python3-pyqt5`

---

## 🤝 Credits

- Based on tools provided by the Linux UVC community
- Inspired by OBS + v4l2loopback setups

---

## 📬 License

MIT License — see `LICENSE` file for details.
