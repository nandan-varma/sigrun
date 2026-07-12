#!/usr/bin/env bash
# Build and run SIGRUN in QEMU.
# Usage: ./scripts/run.sh [--release] [--debug-build] [--gdb] [--test] [--gui] [--memory=SIZE] [--cpus=N]
#
# Boot method: GRUB2 BIOS multiboot2 ISO.
# Note: qemu-system-x86_64 -kernel does not work for 64-bit ELF in QEMU 5+.

set -eo pipefail

cd "$(dirname "$0")/.."

export PATH="/opt/homebrew/opt/rustup/bin:$PATH"

MODE="release"
GDB=false
TEST_MODE=false
GUI=false
MEMORY="128M"
CPUS="1"
SKIP_BUILD=false

for arg in "$@"; do
    case $arg in
        --release|-r)   MODE="release" ;;
        --debug-build)  MODE="debug" ;;
        --gdb|-g)       GDB=true ;;
        --test|-t)      TEST_MODE=true ;;
        --gui)          GUI=true ;;
        --no-build)     SKIP_BUILD=true ;;
        --memory=*)     MEMORY="${arg#*=}" ;;
        --cpus=*)       CPUS="${arg#*=}" ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --release          Release build (default)"
            echo "  --debug-build      Debug build (slower, has debug symbols)"
            echo "  --gdb, -g          Start GDB server on :1234 (QEMU pauses until GDB connects)"
            echo "  --test, -t         Boot test: exit after 15s; fail if kernel does not print ready message"
            echo "  --gui              Open a real QEMU window (default: headless, serial-only)"
            echo "  --no-build         Skip build step; use existing sigrun.iso"
            echo "  --memory=SIZE      QEMU RAM (default: 128M)"
            echo "  --cpus=N           QEMU CPU count (default: 1)"
            exit 0
            ;;
        *) echo "error: unknown option: $arg" >&2; exit 1 ;;
    esac
done

# ── dependency checks ──────────────────────────────────────────────────────────
for dep in qemu-system-x86_64 i686-elf-grub-mkrescue; do
    if ! command -v "$dep" &>/dev/null; then
        echo "error: $dep not found."
        echo "  macOS: brew install qemu i686-elf-grub mtools xorriso"
        exit 1
    fi
done

# ── build ──────────────────────────────────────────────────────────────────────
if ! $SKIP_BUILD; then
    BUILD_FLAG=""
    [ "$MODE" = "debug" ] && BUILD_FLAG="--debug"
    ./scripts/build.sh $BUILD_FLAG
fi

if [ ! -f sigrun.iso ]; then
    echo "error: sigrun.iso not found. Run ./scripts/build.sh first."
    exit 1
fi

# ── QEMU ───────────────────────────────────────────────────────────────────────
QEMU_ARGS=(
    qemu-system-x86_64
    -cdrom sigrun.iso
    -m "$MEMORY"
    -smp "$CPUS"
    -serial stdio
    -no-reboot
)

if $GUI; then
    QEMU_ARGS+=(-name "SIGRUN")
else
    QEMU_ARGS+=(-display none)
fi

if [ -w /dev/kvm ] 2>/dev/null; then
    QEMU_ARGS+=(-enable-kvm -cpu host)
fi

if $GDB; then
    QEMU_ARGS+=(-s -S)
    KERNEL_ELF="target/x86_64-unknown-none/$MODE/kernel"
    echo "==> GDB server on :1234 (QEMU paused — waiting for GDB to connect)"
    echo "    gdb $KERNEL_ELF -ex 'target remote :1234'"
    echo ""
fi

if $TEST_MODE; then
    LOG=/tmp/sigrun-boot.log
    echo "==> Boot test (15s timeout)..."
    timeout 15s "${QEMU_ARGS[@]}" 2>&1 | tee "$LOG" || true
    if grep -q "SIGRUN kernel running" "$LOG"; then
        echo ""
        echo "==> PASSED"
        exit 0
    else
        echo ""
        echo "==> FAILED — 'SIGRUN kernel running' not found in serial output"
        exit 1
    fi
fi

echo "==> Launching QEMU (Ctrl-C to quit)..."
echo ""
"${QEMU_ARGS[@]}"
