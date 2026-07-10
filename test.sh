#!/usr/bin/env bash
# Run tests, clippy, and format checks for SIGRUN.
# Usage: ./test.sh [--no-clippy] [--no-fmt] [--qemu]

set -eo pipefail

RUN_CLIPPY=true
RUN_FMT=true
RUN_QEMU=false

for arg in "$@"; do
    case $arg in
        --no-clippy) RUN_CLIPPY=false ;;
        --no-fmt)    RUN_FMT=false ;;
        --qemu)      RUN_QEMU=true ;;
        --help|-h)
            echo "Usage: $0 [--no-clippy] [--no-fmt] [--qemu]"
            echo "  --no-clippy  Skip clippy lint pass"
            echo "  --no-fmt     Skip format check"
            echo "  --qemu       Also run QEMU boot test"
            exit 0
            ;;
        *) echo "Unknown option: $arg"; exit 1 ;;
    esac
done

if $RUN_FMT; then
    echo "==> Format check..."
    cargo fmt --all -- --check
fi

echo "==> Unit tests..."
cargo test --workspace

if $RUN_CLIPPY; then
    echo "==> Clippy..."
    cargo clippy --workspace -- -D warnings
fi

if $RUN_QEMU; then
    echo "==> QEMU boot test..."
    ./scripts/run.sh --test
fi

echo ""
echo "All checks passed."
