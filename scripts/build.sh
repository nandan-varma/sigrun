#!/usr/bin/env bash
# Build the SIGRUN kernel and produce a bootable ISO.
# Usage: ./scripts/build.sh [--release] [--check] [--help]
#
# Prerequisites (macOS):
#   brew install rustup qemu i686-elf-grub mtools xorriso
#   rustup toolchain install nightly
#   rustup target add x86_64-unknown-none --toolchain nightly

set -eo pipefail

cd "$(dirname "$0")/.."

# Ensure rustup nightly is in PATH (macOS Homebrew layout)
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"

MODE="release"
CARGO_CMD="build"

for arg in "$@"; do
    case $arg in
        --debug)      MODE="debug" ;;
        --release|-r) MODE="release" ;;
        --check|-c)   CARGO_CMD="check" ;;
        --help|-h)
            echo "Usage: $0 [--release] [--debug] [--check]"
            echo "  --release  Optimized build (default)"
            echo "  --debug    Unoptimized build with debug info"
            echo "  --check    Type-check only; skip linking and ISO"
            exit 0
            ;;
        *) echo "error: unknown option: $arg" >&2; exit 1 ;;
    esac
done

# ── dependency checks ──────────────────────────────────────────────────────────
if ! command -v i686-elf-grub-mkrescue &>/dev/null && [ "$CARGO_CMD" != "check" ]; then
    echo "error: i686-elf-grub-mkrescue not found."
    echo "  macOS: brew install i686-elf-grub mtools xorriso"
    exit 1
fi

# ── build ──────────────────────────────────────────────────────────────────────
RELEASE_FLAG=""
[ "$MODE" = "release" ] && RELEASE_FLAG="--release"

echo "==> Kernel ($MODE)..."
cargo $CARGO_CMD -p kernel --target x86_64-unknown-none $RELEASE_FLAG

if [ "$CARGO_CMD" = "check" ]; then
    echo "==> Userspace (check)..."
    cargo check -p init -p driver-manager -p filesystem -p network -p shell -p common
    echo ""
    echo "Type-check complete."
    exit 0
fi

# ── ISO ────────────────────────────────────────────────────────────────────────
KERNEL_ELF="target/x86_64-unknown-none/$MODE/kernel"
mkdir -p iso/boot/grub
cp "$KERNEL_ELF" iso/boot/kernel
cp scripts/grub.cfg iso/boot/grub/grub.cfg

echo "==> ISO..."
i686-elf-grub-mkrescue -o sigrun.iso iso 2>/dev/null

echo ""
echo "Done."
echo "  Kernel:  $KERNEL_ELF  ($(du -sh "$KERNEL_ELF" | cut -f1))"
echo "  ISO:     sigrun.iso   ($(du -sh sigrun.iso | cut -f1))"
echo ""
echo "Run: ./scripts/run.sh"
