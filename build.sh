#!/usr/bin/env bash
# Build SIGRUN OS components.
# Usage: ./build.sh [--release] [--check]

set -eo pipefail

MODE="debug"
CARGO_CMD="build"

for arg in "$@"; do
    case $arg in
        --release|-r) MODE="release" ;;
        --check|-c)   CARGO_CMD="check" ;;
        --help|-h)
            echo "Usage: $0 [--release] [--check]"
            echo "  --release   Optimized build"
            echo "  --check     Type-check only (faster)"
            exit 0
            ;;
        *) echo "Unknown option: $arg"; exit 1 ;;
    esac
done

RELEASE_FLAG=""
[ "$MODE" = "release" ] && RELEASE_FLAG="--release"

echo "==> Building bootloader ($MODE)..."
cargo $CARGO_CMD -p boot --target x86_64-unknown-none $RELEASE_FLAG

echo "==> Building kernel ($MODE)..."
cargo $CARGO_CMD -p kernel --target x86_64-unknown-none $RELEASE_FLAG

echo "==> Building userspace ($MODE)..."
cargo $CARGO_CMD -p init -p driver-manager -p filesystem -p network -p shell $RELEASE_FLAG

echo ""
echo "Build complete."
if [ "$CARGO_CMD" = "build" ]; then
    echo "Artifacts:"
    echo "  target/x86_64-unknown-none/$MODE/bootloader"
    echo "  target/x86_64-unknown-none/$MODE/kernel"
fi
