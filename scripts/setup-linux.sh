#!/usr/bin/env bash
set -euo pipefail

# Logi Linux App - Linux development environment setup
# Supports: Ubuntu/Debian, Fedora/RHEL, Arch Linux

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}[INFO]${NC} $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*"; exit 1; }

detect_distro() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        echo "$ID"
    else
        error "Cannot detect Linux distribution"
    fi
}

install_system_deps() {
    local distro
    distro=$(detect_distro)
    info "Detected distro: $distro"

    case "$distro" in
        ubuntu|debian|linuxmint|pop)
            info "Installing system dependencies (apt)..."
            sudo apt update
            sudo apt install -y \
                build-essential curl wget file \
                libwebkit2gtk-4.1-dev \
                libxdo-dev libssl-dev \
                libayatana-appindicator3-dev \
                librsvg2-dev \
                libudev-dev libhidapi-dev \
                pkg-config
            ;;
        fedora|rhel|centos|rocky|alma)
            info "Installing system dependencies (dnf)..."
            sudo dnf install -y \
                gcc gcc-c++ make curl wget file \
                webkit2gtk4.1-devel \
                libxdo-devel openssl-devel \
                libappindicator-gtk3-devel \
                librsvg2-devel \
                systemd-devel hidapi-devel \
                pkg-config
            ;;
        arch|manjaro|endeavouros)
            info "Installing system dependencies (pacman)..."
            sudo pacman -Syu --needed --noconfirm \
                base-devel curl wget file \
                webkit2gtk-4.1 \
                xdotool openssl \
                libayatana-appindicator \
                librsvg \
                hidapi \
                pkgconf
            ;;
        *)
            error "Unsupported distro: $distro. Install dependencies manually (see README)."
            ;;
    esac

    info "System dependencies installed"
}

install_rust() {
    if command -v rustc &>/dev/null; then
        info "Rust already installed: $(rustc --version)"
    else
        info "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
        info "Rust installed: $(rustc --version)"
    fi
}

install_node() {
    if command -v node &>/dev/null; then
        local node_ver
        node_ver=$(node --version | sed 's/v//' | cut -d. -f1)
        if [ "$node_ver" -ge 20 ]; then
            info "Node.js already installed: $(node --version)"
            return
        else
            warn "Node.js $(node --version) is too old (need >= 20)"
        fi
    fi

    if command -v nvm &>/dev/null; then
        info "Installing Node.js 22 via nvm..."
        nvm install 22
    elif command -v fnm &>/dev/null; then
        info "Installing Node.js 22 via fnm..."
        fnm install 22 && fnm use 22
    else
        info "Installing nvm + Node.js 22..."
        curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.0/install.sh | bash
        export NVM_DIR="$HOME/.nvm"
        [ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"
        nvm install 22
    fi

    info "Node.js installed: $(node --version)"
}

install_tauri_cli() {
    if cargo tauri --version &>/dev/null 2>&1; then
        info "cargo-tauri already installed: $(cargo tauri --version)"
    else
        info "Installing cargo-tauri CLI..."
        cargo install tauri-cli
        info "cargo-tauri installed"
    fi
}

setup_udev() {
    local rules_file="/etc/udev/rules.d/99-logitech-hidpp.rules"
    if [ -f "$rules_file" ]; then
        info "udev rules already exist"
        return
    fi

    info "Setting up udev rules for Logitech HID++ devices..."
    sudo tee "$rules_file" > /dev/null << 'EOF'
# Logitech HID++ devices - allow non-root access
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="046d", MODE="0666"
EOF
    sudo udevadm control --reload-rules
    sudo udevadm trigger
    info "udev rules installed (replug your device or reboot)"
}

verify_build() {
    local script_dir
    script_dir="$(cd "$(dirname "$0")" && pwd)"
    local project_dir
    project_dir="$(dirname "$script_dir")"

    info "Verifying Rust build..."
    cd "$project_dir/src-tauri"
    cargo check 2>&1 | tail -1
    info "Build verified"
}

install_npm_deps() {
    local script_dir
    script_dir="$(cd "$(dirname "$0")" && pwd)"
    local project_dir
    project_dir="$(dirname "$script_dir")"

    info "Installing npm dependencies..."
    cd "$project_dir"
    npm install
    info "npm dependencies installed"
}

main() {
    echo ""
    echo "=========================================="
    echo "  Logi Linux App - Development Setup"
    echo "=========================================="
    echo ""

    install_system_deps
    install_rust
    install_node
    install_tauri_cli
    setup_udev
    install_npm_deps
    verify_build

    echo ""
    info "Setup complete! Run the app with:"
    echo ""
    echo "    cargo tauri dev"
    echo ""
}

main "$@"
