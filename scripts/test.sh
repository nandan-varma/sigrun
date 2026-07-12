#!/usr/bin/env bash
# Run all checks for SIGRUN: format, clippy, unit tests, and optional QEMU boot test.
# Usage: ./scripts/test.sh [--fmt] [--clippy] [--boot] [--all] [--help]
#
# By default runs: format check + clippy + unit tests.

set -eo pipefail

cd "$(dirname "$0")/.."

export PATH="/opt/homebrew/opt/rustup/bin:$PATH"

RUN_FMT=true
RUN_CLIPPY=true
RUN_UNITS=true
RUN_BOOT=false

for arg in "$@"; do
    case $arg in
        --no-fmt)     RUN_FMT=false ;;
        --no-clippy)  RUN_CLIPPY=false ;;
        --no-units)   RUN_UNITS=false ;;
        --boot|-b)    RUN_BOOT=true ;;
        --all|-a)     RUN_BOOT=true ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --no-fmt      Skip rustfmt check"
            echo "  --no-clippy   Skip clippy lint"
            echo "  --no-units    Skip unit tests"
            echo "  --boot, -b    Also run QEMU boot test"
            echo "  --all, -a     Run all checks including boot test"
            exit 0
            ;;
        *) echo "error: unknown option: $arg" >&2; exit 1 ;;
    esac
done

PASS=0
FAIL=0

run_step() {
    local name="$1"; shift
    echo "==> $name..."
    if "$@"; then
        PASS=$((PASS + 1))
    else
        echo "FAILED: $name"
        FAIL=$((FAIL + 1))
    fi
}

if $RUN_FMT; then
    run_step "Format check" cargo fmt --all -- --check
fi

if $RUN_CLIPPY; then
    run_step "Clippy (kernel)" cargo clippy -p kernel --target x86_64-unknown-none -- -W warnings
    run_step "Clippy (userspace)" cargo clippy -p init -p driver-manager -p filesystem -p network -p shell -- -W warnings
fi

if $RUN_UNITS; then
    run_step "Unit tests" cargo test --workspace --exclude kernel
fi

if $RUN_BOOT; then
    run_step "QEMU boot test" ./scripts/run.sh --test --no-build
fi

echo ""
if [ "$FAIL" -eq 0 ]; then
    echo "All $PASS check(s) passed."
else
    echo "$FAIL check(s) failed, $PASS passed."
    exit 1
fi
