#!/usr/bin/env bash
# SIGRUN Test Script - Optimized for Parallel Agent Development
# Usage: ./test.sh [OPTIONS] [TRACKS...]
#
# Examples:
#   ./test.sh              # Test everything
#   ./test.sh 2 3          # Test only Track 2 and 3
#   ./test.sh --qemu       # Run QEMU integration tests
#   ./test.sh --coverage   # Generate coverage report

set -eo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configuration
QEMU_TESTS=false
COVERAGE=false
VERBOSE=false
PARALLEL=true
NOCAPTURE=false
MIRI=false
TRACKS=()

# Helper functions to map tracks to packages
get_track_packages() {
    case $1 in
        1) echo "boot" ;;
        2) echo "kernel" ;;
        3) echo "kernel" ;;
        4) echo "kernel" ;;
        5) echo "kernel" ;;
        6) echo "kernel" ;;
        7) echo "init driver-manager" ;;
        8) echo "filesystem network shell" ;;
        *) echo "" ;;
    esac
}

get_track_name() {
    case $1 in
        1) echo "Bootloader & Kernel Entry" ;;
        2) echo "Memory Management" ;;
        3) echo "Scheduler & Timer" ;;
        4) echo "Interrupt Controller" ;;
        5) echo "IPC System" ;;
        6) echo "Capability Manager" ;;
        7) echo "Userspace Services" ;;
        8) echo "Drivers & Services" ;;
        *) echo "Unknown" ;;
    esac
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --qemu|-q)
            QEMU_TESTS=true
            shift
            ;;
        --coverage|-C)
            COVERAGE=true
            shift
            ;;
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --nocapture|-n)
            NOCAPTURE=true
            shift
            ;;
        --sequential|-s)
            PARALLEL=false
            shift
            ;;
        --miri|-m)
            MIRI=true
            shift
            ;;
        --help|-h)
            cat << EOF
SIGRUN Test Script

Usage: ./test.sh [OPTIONS] [TRACKS...]

OPTIONS:
    --qemu, -q          Run QEMU integration tests
    --coverage, -C      Generate code coverage report
    --verbose, -v       Verbose test output
    --nocapture, -n     Show println! output from tests
    --sequential, -s    Run tests sequentially
    --miri, -m          Run with Miri (for unsafe code)
    --help, -h          Show this help

TRACKS (1-8):
    1   Bootloader & Kernel Entry
    2   Memory Management
    3   Scheduler & Timer
    4   Interrupt Controller
    5   IPC System
    6   Capability Manager
    7   Userspace Services
    8   Drivers & Services

Examples:
    ./test.sh                   # Test all
    ./test.sh 2 3               # Test tracks 2 and 3
    ./test.sh --qemu            # Run QEMU tests
    ./test.sh --coverage 5      # Coverage for track 5
EOF
            exit 0
            ;;
        [1-8])
            TRACKS+=("$1")
            shift
            ;;
        *)
            echo -e "${RED}Error: Unknown option '$1'${NC}"
            echo "Run './test.sh --help' for usage"
            exit 1
            ;;
    esac
done

# If no tracks specified, test all
if [ ${#TRACKS[@]} -eq 0 ]; then
    TRACKS=(1 2 3 4 5 6 7 8)
fi

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Test a single package
test_package() {
    local package=$1
    local track=$2
    local track_name=$3
    
    log_info "Track $track ($track_name): Testing $package..."
    
    local cargo_cmd="cargo"
    
    if $MIRI; then
        cargo_cmd="$cargo_cmd miri test"
    else
        cargo_cmd="$cargo_cmd test"
    fi
    
    cargo_cmd="$cargo_cmd -p $package"
    
    if $NOCAPTURE; then
        cargo_cmd="$cargo_cmd -- --nocapture"
    fi
    
    local log_file="target/test-track$track-$package.log"
    
    if $VERBOSE; then
        eval $cargo_cmd 2>&1 | tee "$log_file"
    else
        if eval $cargo_cmd > "$log_file" 2>&1; then
            log_success "Track $track: $package tests passed"
            return 0
        else
            log_error "Track $track: $package tests failed (see $log_file)"
            cat "$log_file"
            return 1
        fi
    fi
}

# Clippy check
clippy_check() {
    local package=$1
    local track=$2
    
    log_info "Track $track: Running clippy on $package..."
    
    if cargo clippy -p "$package" -- -D warnings 2>/dev/null; then
        log_success "Track $track: clippy passed for $package"
        return 0
    else
        log_error "Track $track: clippy failed for $package"
        return 1
    fi
}

# Format check
fmt_check() {
    log_info "Checking code formatting..."
    
    if cargo fmt --all -- --check; then
        log_success "All code is properly formatted"
        return 0
    else
        log_error "Code formatting issues found. Run 'cargo fmt --all'"
        return 1
    fi
}

# Test a track
test_track() {
    local track=$1
    local packages=$(get_track_packages "$track")
    local track_name=$(get_track_name "$track")
    
    log_info "======================="
    log_info "Testing Track $track: $track_name"
    log_info "======================="
    
    local failed=0
    
    # Run unit tests
    for package in $packages; do
        if ! test_package "$package" "$track" "$track_name"; then
            failed=1
        fi
    done
    
    # Run clippy
    for package in $packages; do
        if ! clippy_check "$package" "$track"; then
            failed=1
        fi
    done
    
    return $failed
}

# QEMU integration tests
run_qemu_tests() {
    log_info "======================="
    log_info "Running QEMU Integration Tests"
    log_info "======================="
    
    # Check if QEMU is available
    if ! command -v qemu-system-x86_64 &> /dev/null; then
        log_error "QEMU not found. Install qemu-system-x86_64"
        return 1
    fi
    
    # Build first
    log_info "Building kernel for QEMU..."
    if ! ./build.sh --release 1 2> /dev/null; then
        log_error "Build failed"
        return 1
    fi
    
    # Check for bootloader
    if [ ! -f "build/boot/bootloader.efi" ]; then
        log_error "Bootloader not found at build/boot/bootloader.efi"
        return 1
    fi
    
    log_info "Starting QEMU boot test..."
    
    # Run QEMU with timeout
    timeout 30s ./build/scripts/qemu.sh \
        --kernel build/kernel/kernel.bin \
        --memory 512M \
        --cpus 1 > target/qemu-test.log 2>&1 || {
        
        local exit_code=$?
        if [ $exit_code -eq 124 ]; then
            log_warn "QEMU test timed out (expected for boot test)"
            # Check if kernel printed anything
            if grep -q "SIGRUN" target/qemu-test.log; then
                log_success "Kernel booted successfully"
                return 0
            fi
        fi
        
        log_error "QEMU test failed"
        cat target/qemu-test.log
        return 1
    }
}

# Coverage report
generate_coverage() {
    log_info "Generating coverage report..."
    
    # Check for tarpaulin
    if ! command -v cargo-tarpaulin &> /dev/null; then
        log_info "Installing cargo-tarpaulin..."
        cargo install cargo-tarpaulin
    fi
    
    local packages=""
    for track in "${TRACKS[@]}"; do
        for pkg in ${TRACK_PACKAGES[$track]}; do
            packages="$packages -p $pkg"
        done
    done
    
    cargo tarpaulin $packages --out Html --output-dir target/coverage
    
    log_success "Coverage report generated at target/coverage/index.html"
}

# Main test process
main() {
    log_info "SIGRUN Test System"
    log_info "Tracks: ${TRACKS[*]}"
    echo ""
    
    # Check toolchain
    if ! command -v cargo &> /dev/null; then
        log_error "Cargo not found. Install Rust with: brew install rust"
        exit 1
    fi
    
    log_info "Using Rust: $(rustc --version | head -1)"
    
    # Create target directory
    mkdir -p target
    
    # Run format check first
    if ! fmt_check; then
        log_error "Format check failed. Fix formatting before running tests."
        exit 1
    fi
    
    local start_time=$(date +%s)
    local failed_tracks=()
    
    # Run track tests
    if $PARALLEL && [ ${#TRACKS[@]} -gt 1 ] && ! $MIRI; then
        log_info "Running tests in parallel..."
        
        local pids=""
        for track in "${TRACKS[@]}"; do
            test_track "$track" &
            pids="$pids $!"
        done
        
        # Wait for all tests
        for pid in $pids; do
            if ! wait $pid; then
                failed_tracks+=("failed")
            fi
        done
        
        if [ ${#failed_tracks[@]} -gt 0 ]; then
            log_error "Failed tracks: ${failed_tracks[*]}"
            exit 1
        fi
    else
        # Sequential tests
        log_info "Running tests sequentially..."
        for track in "${TRACKS[@]}"; do
            if ! test_track "$track"; then
                log_error "Track $track failed. Stopping."
                exit 1
            fi
        done
    fi
    
    # QEMU tests
    if $QEMU_TESTS; then
        if ! run_qemu_tests; then
            log_error "QEMU tests failed"
            exit 1
        fi
    fi
    
    # Coverage
    if $COVERAGE; then
        generate_coverage
    fi
    
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    
    echo ""
    log_success "========================================"
    log_success "All tests passed!"
    log_success "Time: ${duration}s"
    log_success "========================================"
}

# Run main
main
