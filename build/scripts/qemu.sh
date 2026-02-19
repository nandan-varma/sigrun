#!/bin/bash
# SIGRUN QEMU Launch Script

set -e

KERNEL=""
INITRD=""
MEMORY="2G"
CPUS="2"
DEBUG=""
OVMF_PATH="/usr/share/ovmf/x64/OVMF.fd"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --kernel)
            KERNEL="$2"
            shift 2
            ;;
        --initrd)
            INITRD="$2"
            shift 2
            ;;
        --memory)
            MEMORY="$2"
            shift 2
            ;;
        --cpus)
            CPUS="$2"
            shift 2
            ;;
        --debug)
            DEBUG="yes"
            shift
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Check for OVMF
if [ ! -f "$OVMF_PATH" ]; then
    echo "OVMF not found at $OVMF_PATH"
    echo "Installing..."
    # Would install OVMF
fi

# Build command
CMD="qemu-system-x86_64 \
    -machine q35 \
    -m $MEMORY \
    -smp $CPUS \
    -bios $OVMF_PATH \
    -drive if=pflash,format=raw,file=$OVMF_PATH,readonly=on"

if [ -n "$KERNEL" ]; then
    CMD="$CMD -kernel $KERNEL"
fi

if [ -n "$INITRD" ]; then
    CMD="$CMD -initrd $INITRD"
fi

# Add serial console
CMD="$CMD -serial stdio"
CMD="$CMD -append \"console=hvc0 earlyprintk=ttyS0\""

# Enable KVM if available
if [ -w /dev/kvm ]; then
    CMD="$CMD -enable-kvm -cpu host"
fi

# Debug mode
if [ "$DEBUG" = "yes" ]; then
    CMD="$CMD -s -S"
fi

echo "Running: $CMD"
eval $CMD
