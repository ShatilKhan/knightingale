#!/usr/bin/env sh
# Knightingale installer.
#
#   curl -fsSL https://shatilkhan.github.io/knightingale/install.sh | sh
#
# Flags:
#   --version <tag>     pin to a specific release (default: latest)
#   --provider <name>   non-interactive: pick provider (groq, openai, ...)
#   --hotkey <binding>  non-interactive: hotkey binding (default: super+k / cmd+shift+k)
#   --no-sudo           skip package-manager step; print commands instead
#   --yes               accept all prompts
#   --skip-config       install binary only; user runs `knightingale setup` later
#   --verify-sums       verify SHA256SUMS (default: on)
#   --no-verify         skip checksum verification (not recommended)
#   --dry-run           print what would happen, do nothing

set -eu

REPO="ShatilKhan/knightingale"
VERSION=""
PROVIDER=""
HOTKEY=""
NO_SUDO=""
YES=""
SKIP_CONFIG=""
VERIFY=1
DRY_RUN=""

err() { printf '%s\n' "knightingale: $*" >&2; }
say() { printf '%s\n' "$*"; }
do_or_print() {
    if [ -n "$DRY_RUN" ]; then
        printf '  (dry-run) %s\n' "$*"
    else
        eval "$*"
    fi
}

while [ $# -gt 0 ]; do
    case "$1" in
        --version) VERSION="$2"; shift 2 ;;
        --provider) PROVIDER="$2"; shift 2 ;;
        --hotkey) HOTKEY="$2"; shift 2 ;;
        --no-sudo) NO_SUDO=1; shift ;;
        --yes|-y) YES=1; shift ;;
        --skip-config) SKIP_CONFIG=1; shift ;;
        --verify-sums) VERIFY=1; shift ;;
        --no-verify) VERIFY=0; shift ;;
        --dry-run) DRY_RUN=1; shift ;;
        -h|--help)
            sed -n '1,/^set -eu/p' "$0" | sed -e 's/^# \{0,1\}//' -e '/^$/d'
            exit 0
            ;;
        *) err "unknown flag: $1"; exit 2 ;;
    esac
done

detect_os() {
    UNAME=$(uname -s 2>/dev/null || echo Unknown)
    case "$UNAME" in
        Linux)   OS="linux" ;;
        Darwin)  OS="macos" ;;
        MINGW*|MSYS*|CYGWIN*) OS="windows" ;;
        *) err "unsupported OS: $UNAME"; exit 1 ;;
    esac
    ARCH=$(uname -m 2>/dev/null || echo unknown)
    case "$ARCH" in
        x86_64|amd64) ARCH="x86_64" ;;
        aarch64|arm64) ARCH="aarch64" ;;
        *) err "unsupported arch: $ARCH"; exit 1 ;;
    esac
}

detect_pkg_mgr() {
    if [ "$OS" != "linux" ]; then PKG=""; return; fi
    if command -v apt-get >/dev/null 2>&1; then PKG="apt"; return; fi
    if command -v dnf >/dev/null 2>&1; then PKG="dnf"; return; fi
    if command -v pacman >/dev/null 2>&1; then PKG="pacman"; return; fi
    PKG=""
}

have_any_pkg() {
    # True if any of the given apt package names is installed (covers t64
    # renames on Ubuntu 24.04+).
    for p in "$@"; do
        dpkg -s "$p" >/dev/null 2>&1 && return 0
    done
    return 1
}

preflight() {
    if [ "$OS" != "linux" ]; then return 0; fi
    MISSING=""
    case "$PKG" in
        apt)
            have_any_pkg libasound2 libasound2t64 || MISSING="$MISSING libasound2t64"
            have_any_pkg libxkbcommon0 || MISSING="$MISSING libxkbcommon0"
            have_any_pkg libnotify4 || MISSING="$MISSING libnotify4"
            ;;
        dnf|pacman) ;;  # Best-effort; skip strict checks.
    esac
    if [ -n "$MISSING" ]; then
        say "Missing runtime libraries:$MISSING"
        if [ -n "$NO_SUDO" ]; then
            say "Install them with: sudo apt install -y$MISSING"
            say "Then re-run this script."
            exit 1
        fi
        if [ -z "$YES" ]; then
            printf 'Install now via sudo? [Y/n] '
            read -r REPLY
            case "$REPLY" in
                n|N|no|No) exit 1 ;;
            esac
        fi
        do_or_print "sudo apt install -y $MISSING"
    fi
}

resolve_version() {
    if [ -n "$VERSION" ]; then return 0; fi
    # Latest release via GitHub API redirect.
    VERSION=$(curl -fsSL -o /dev/null -w '%{url_effective}' \
        "https://github.com/$REPO/releases/latest" 2>/dev/null \
        | awk -F'/' '{print $NF}')
    if [ -z "$VERSION" ]; then
        err "could not resolve latest release tag; try --version"
        exit 1
    fi
}

download_binary() {
    TARGET=""
    case "$OS-$ARCH" in
        linux-x86_64)   TARGET="x86_64-unknown-linux-gnu" ;;
        linux-aarch64)  TARGET="aarch64-unknown-linux-gnu" ;;
        macos-x86_64)   TARGET="x86_64-apple-darwin" ;;
        macos-aarch64)  TARGET="aarch64-apple-darwin" ;;
        windows-x86_64) TARGET="x86_64-pc-windows-msvc" ;;
        *) err "no prebuilt target for $OS-$ARCH"; exit 1 ;;
    esac
    TARBALL="knightingale-$TARGET.tar.gz"
    URL="https://github.com/$REPO/releases/download/$VERSION/$TARBALL"
    SUMS_URL="https://github.com/$REPO/releases/download/$VERSION/SHA256SUMS"
    TMP=$(mktemp -d)
    say "downloading $URL"
    do_or_print "curl -fsSL '$URL' -o '$TMP/$TARBALL'"
    if [ "$VERIFY" = "1" ] && [ -z "$DRY_RUN" ]; then
        say "verifying SHA256SUMS"
        if curl -fsSL "$SUMS_URL" -o "$TMP/SHA256SUMS" 2>/dev/null; then
            (cd "$TMP" && grep "$TARBALL" SHA256SUMS | sha256sum -c -) || {
                err "checksum mismatch"; exit 1; }
        else
            err "SHA256SUMS not found; rerun with --no-verify to skip"
            exit 1
        fi
    fi
    do_or_print "tar -xzf '$TMP/$TARBALL' -C '$TMP'"
    # The tarball contains a knightingale-<target>/ directory.
    SRC="$TMP/knightingale-$TARGET"
    if [ "$OS" = "linux" ] || [ "$OS" = "macos" ]; then
        DEST="$HOME/.local/bin"
        do_or_print "mkdir -p '$DEST'"
        do_or_print "install -m 0755 '$SRC/knightingale' '$DEST/knightingale'"
        do_or_print "install -m 0755 '$SRC/knightingale-daemon' '$DEST/knightingale-daemon'"
        if [ "$OS" = "macos" ]; then
            do_or_print "xattr -d com.apple.quarantine '$DEST/knightingale' 2>/dev/null || true"
            do_or_print "xattr -d com.apple.quarantine '$DEST/knightingale-daemon' 2>/dev/null || true"
        fi
    else
        DEST="$LOCALAPPDATA/Programs/knightingale"
        do_or_print "mkdir -p '$DEST'"
        do_or_print "cp '$SRC/knightingale.exe' '$DEST/'"
        do_or_print "cp '$SRC/knightingale-daemon.exe' '$DEST/'"
    fi
    rm -rf "$TMP" 2>/dev/null || true
    say "✓ installed to $DEST"
}

main() {
    detect_os
    detect_pkg_mgr
    preflight
    resolve_version
    download_binary
    if [ -z "$SKIP_CONFIG" ]; then
        say ""
        say "Run \`knightingale setup\` to pick a provider and hotkey."
    fi
}

main "$@"
