#!/usr/bin/env bash
# Build and run SIGRUN in QEMU.
# Usage: ./scripts/run.sh [--release] [--debug] [--test] [--memory MB] [--cpus N]

set -eo pipefail

cd "$(dirname "$0")/.."

MODE="debug"
DEBUG=false
TEST_MODE=false
MEMORY="512M"
CPUS="2"

for arg in "$@"; do
    case $arg in
        --release|-r) MODE="release" ;;
        --debug|-d)   DEBUG=true ;;
        --test)       TEST_MODE=true ;;
        --memory=*)   MEMORY="${arg#*=}" ;;
        --cpus=*)     CPUS="${arg#*=}" ;;
        --help|-h)
            echo "Usage: $0 [--release] [--debug] [--memory=SIZE] [--cpus=N]"
            echo "  --release      Use release build"
            echo "  --debug        Start with GDB server on :1234 (waits for connection)"
            echo "  --test         Exit after 10s; fail if 'SIGRUN' not seen on serial"
            echo "  --memory=SIZE  QEMU memory (default: 512M)"
            echo "  --cpus=N       QEMU CPU count (default: 2)"
            exit 0
            ;;
        *) echo "Unknown option: $arg"; exit 1 ;;
    esac
done

# ── dependency checks ──────────────────────────────────────────────────────────
if ! command -v qemu-system-x86_64 &>/dev/null; then
    echo "error: qemu-system-x86_64 not found."
    echo "  macOS: brew install qemu"
    echo "  Linux: apt install qemu-system-x86"
    exit 1
fi

# ── build ──────────────────────────────────────────────────────────────────────
RELEASE_FLAG=""
[ "$MODE" = "release" ] && RELEASE_FLAG="--release"

echo "==> Building kernel ($MODE)..."
cargo build -p kernel --target x86_64-unknown-none $RELEASE_FLAG

KERNEL="target/x86_64-unknown-none/$MODE/kernel"
if [ ! -f "$KERNEL" ]; then
    echo "error: kernel binary not found at $KERNEL"
    exit 1
fi

# ── OVMF detection ────────────────────────────────────────────────────────────
OVMF_PATHS=(
    "$(brew --prefix 2>/dev/null)/share/qemu/edk2-x86_64-code.fd"
    "/usr/share/ovmf/x64/OVMF.fd"
    "/usr/share/edk2/ovmf/OVMF_CODE.fd"
    "/usr/share/edk2-ovmf/x64/OVMF.fd"
)
OVMF=""
for p in "${OVMF_PATHS[@]}"; do
    [ -f "$p" ] && OVMF="$p" && break
done

# ── QEMU command ──────────────────────────────────────────────────────────────
QEMU_ARGS=(
    qemu-system-x86_64
    -machine q35
    -m "$MEMORY"
    -smp "$CPUS"
    -kernel "$KERNEL"
    -serial stdio
    -display none
    -no-reboot
)

if [ -n "$OVMF" ]; then
    QEMU_ARGS+=(-bios "$OVMF")
fi

# Enable KVM on Linux if available
if [ -w /dev/kvm ]; then
    QEMU_ARGS+=(-enable-kvm -cpu host)
fi

if $DEBUG; then
    QEMU_ARGS+=(-s -S)
    echo "==> GDB server listening on :1234 (QEMU paused until connected)"
    echo "    gdb $KERNEL -ex 'target remote :1234'"
fi

if $TEST_MODE; then
    echo "==> Running boot test (10s timeout)..."
    timeout 10s "${QEMU_ARGS[@]}" 2>&1 | tee /tmp/sigrun-boot.log || true
    if grep -q "SIGRUN" /tmp/sigrun-boot.log; then
        echo "Boot test passed."
        exit 0
    else
        echo "Boot test failed — 'SIGRUN' not found in serial output."
        exit 1
    fi
fi

echo "==> Launching QEMU... (Ctrl-A X to quit)"
"${QEMU_ARGS[@]}"
