#!/bin/sh
# springup installer — https://github.com/Raghav893/springCli
# Usage: curl -fsSL https://raw.githubusercontent.com/Raghav893/springCli/main/springup/dist/install.sh | sh
#
# This script detects your OS and architecture, downloads the correct
# pre-built springup binary from GitHub Releases, and installs it to
# ~/.springup/bin. It then adds that directory to your PATH.

set -e

REPO="Raghav893/springCli"
BINARY_NAME="springup"
INSTALL_DIR="$HOME/.springup/bin"

# ── Colors ──────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
RESET='\033[0m'

info()    { printf "${CYAN}info${RESET}  %s\n" "$1"; }
success() { printf "${GREEN}✔${RESET}     %s\n" "$1"; }
warn()    { printf "${YELLOW}warn${RESET}  %s\n" "$1"; }
error()   { printf "${RED}error${RESET} %s\n" "$1" >&2; exit 1; }

# ── OS / Arch detection ────────────────────────────────────────────
detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "$OS" in
        Linux*)  OS="linux" ;;
        Darwin*) OS="macos" ;;
        *)       error "Unsupported operating system: $OS. Use Windows PowerShell installer instead." ;;
    esac

    case "$ARCH" in
        x86_64|amd64)   ARCH="x86_64" ;;
        aarch64|arm64)  ARCH="aarch64" ;;
        *)              error "Unsupported architecture: $ARCH" ;;
    esac

    # musl builds for Linux for maximum compatibility
    if [ "$OS" = "linux" ] && [ "$ARCH" = "x86_64" ]; then
        TARGET="x86_64-unknown-linux-musl"
    elif [ "$OS" = "linux" ] && [ "$ARCH" = "aarch64" ]; then
        TARGET="aarch64-unknown-linux-gnu"
    elif [ "$OS" = "macos" ] && [ "$ARCH" = "x86_64" ]; then
        TARGET="x86_64-apple-darwin"
    elif [ "$OS" = "macos" ] && [ "$ARCH" = "aarch64" ]; then
        TARGET="aarch64-apple-darwin"
    else
        error "Unsupported platform: $OS-$ARCH"
    fi
}

# ── Fetch latest version tag from GitHub ───────────────────────────
get_latest_version() {
    if command -v curl > /dev/null 2>&1; then
        VERSION=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')
    elif command -v wget > /dev/null 2>&1; then
        VERSION=$(wget -qO- "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')
    else
        error "Neither curl nor wget found. Please install one and try again."
    fi

    if [ -z "$VERSION" ]; then
        error "Could not determine the latest version. Check https://github.com/$REPO/releases"
    fi
}

# ── Download ───────────────────────────────────────────────────────
download_binary() {
    ASSET_NAME="springup-${TARGET}"
    DOWNLOAD_URL="https://github.com/$REPO/releases/download/$VERSION/$ASSET_NAME.tar.gz"

    info "Downloading springup $VERSION for $OS ($ARCH)..."
    info "URL: $DOWNLOAD_URL"

    TMP_DIR=$(mktemp -d)
    trap 'rm -rf "$TMP_DIR"' EXIT

    if command -v curl > /dev/null 2>&1; then
        HTTP_CODE=$(curl -fsSL -w "%{http_code}" -o "$TMP_DIR/springup.tar.gz" "$DOWNLOAD_URL" 2>/dev/null || true)
    elif command -v wget > /dev/null 2>&1; then
        wget -q -O "$TMP_DIR/springup.tar.gz" "$DOWNLOAD_URL" 2>/dev/null
        HTTP_CODE="200"
    fi

    if [ ! -f "$TMP_DIR/springup.tar.gz" ] || [ "$(wc -c < "$TMP_DIR/springup.tar.gz")" -lt 1000 ]; then
        error "Download failed. The release asset may not exist yet for $TARGET.\n  Check: https://github.com/$REPO/releases/tag/$VERSION"
    fi

    # Extract
    tar -xzf "$TMP_DIR/springup.tar.gz" -C "$TMP_DIR" 2>/dev/null || {
        # Maybe it's a raw binary, not a tar.gz
        mv "$TMP_DIR/springup.tar.gz" "$TMP_DIR/$BINARY_NAME"
    }

    # Find the binary in the extracted content
    if [ -f "$TMP_DIR/$BINARY_NAME" ]; then
        BINARY_PATH="$TMP_DIR/$BINARY_NAME"
    elif [ -f "$TMP_DIR/springup-$TARGET/$BINARY_NAME" ]; then
        BINARY_PATH="$TMP_DIR/springup-$TARGET/$BINARY_NAME"
    else
        # Search for it
        BINARY_PATH=$(find "$TMP_DIR" -name "$BINARY_NAME" -type f | head -1)
        if [ -z "$BINARY_PATH" ]; then
            error "Could not find the springup binary in the downloaded archive."
        fi
    fi
}

# ── Install ────────────────────────────────────────────────────────
install_binary() {
    mkdir -p "$INSTALL_DIR"
    cp "$BINARY_PATH" "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"
    success "Installed springup to $INSTALL_DIR/$BINARY_NAME"
}

# ── PATH setup ─────────────────────────────────────────────────────
setup_path() {
    case ":$PATH:" in
        *":$INSTALL_DIR:"*) return ;; # Already in PATH
    esac

    SHELL_NAME=$(basename "$SHELL" 2>/dev/null || echo "sh")
    EXPORT_LINE="export PATH=\"$INSTALL_DIR:\$PATH\""

    case "$SHELL_NAME" in
        zsh)
            RC_FILE="$HOME/.zshrc"
            ;;
        bash)
            if [ -f "$HOME/.bashrc" ]; then
                RC_FILE="$HOME/.bashrc"
            else
                RC_FILE="$HOME/.profile"
            fi
            ;;
        fish)
            FISH_CONFIG="$HOME/.config/fish/config.fish"
            if [ -f "$FISH_CONFIG" ]; then
                if ! grep -q "$INSTALL_DIR" "$FISH_CONFIG" 2>/dev/null; then
                    echo "" >> "$FISH_CONFIG"
                    echo "# springup" >> "$FISH_CONFIG"
                    echo "set -gx PATH $INSTALL_DIR \$PATH" >> "$FISH_CONFIG"
                fi
            fi
            success "Added $INSTALL_DIR to PATH in $FISH_CONFIG"
            return
            ;;
        *)
            RC_FILE="$HOME/.profile"
            ;;
    esac

    if [ -n "$RC_FILE" ]; then
        if ! grep -q "$INSTALL_DIR" "$RC_FILE" 2>/dev/null; then
            echo "" >> "$RC_FILE"
            echo "# springup" >> "$RC_FILE"
            echo "$EXPORT_LINE" >> "$RC_FILE"
        fi
        success "Added $INSTALL_DIR to PATH in $RC_FILE"
    fi
}

# ── Main ───────────────────────────────────────────────────────────
main() {
    printf "\n"
    printf "${BOLD}${CYAN}  springup installer${RESET}\n"
    printf "${CYAN}  Scaffold production-ready Spring Boot projects in seconds.${RESET}\n"
    printf "\n"

    detect_platform
    info "Detected platform: $OS $ARCH → $TARGET"

    get_latest_version
    info "Latest version: $VERSION"

    download_binary
    install_binary
    setup_path

    printf "\n"
    printf "${BOLD}${GREEN}  ✔ springup $VERSION installed successfully!${RESET}\n"
    printf "\n"
    printf "  ${BOLD}Next steps:${RESET}\n"
    printf "\n"
    printf "    ${YELLOW}1.${RESET} Restart your terminal (or run: ${CYAN}source ~/${RC_FILE##*/}${RESET})\n"
    printf "    ${YELLOW}2.${RESET} Verify:  ${CYAN}springup --version${RESET}\n"
    printf "    ${YELLOW}3.${RESET} Create:  ${CYAN}springup new${RESET}\n"
    printf "\n"
    printf "  ${BOLD}Documentation:${RESET} https://github.com/$REPO\n"
    printf "\n"
}

main
