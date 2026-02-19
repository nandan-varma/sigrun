#!/usr/bin/env bash
# SIGRUN Build Script - Optimized for Parallel Agent Development
# Usage: ./build.sh [OPTIONS] [TRACKS...]
#
# Examples:
#   ./build.sh              # Build everything
#   ./build.sh 1 2          # Build only Track 1 (boot) and Track 2 (memory)
#   ./build.sh --release    # Release build
#   ./build.sh --check 5    # Quick check Track 5 (IPC)

set -eo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Build configuration
BUILD_MODE="dev"
CHECK_ONLY=false
VERBOSE=false
PARALLEL=true
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
        --release|-r)
            BUILD_MODE="release"
            shift
            ;;
        --check|-c)
            CHECK_ONLY=true
            shift
            ;;
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --sequential|-s)
            PARALLEL=false
            shift
            ;;
        --help|-h)
            cat << EOF
SIGRUN Build Script

Usage: ./build.sh [OPTIONS] [TRACKS...]

OPTIONS:
    --release, -r       Build in release mode (optimized)
    --check, -c         Only check compilation, don't build
    --verbose, -v       Verbose output
    --sequential, -s    Build sequentially instead of parallel
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
    ./build.sh                  # Build all tracks
    ./build.sh 1 2              # Build tracks 1 and 2
    ./build.sh --release 5      # Release build of track 5
    ./build.sh --check          # Quick check all
EOF
            exit 0
            ;;
        [1-8])
            TRACKS+=("$1")
            shift
            ;;
        *)
            echo -e "${RED}Error: Unknown option '$1'${NC}"
            echo "Run './build.sh --help' for usage"
            exit 1
            ;;
    esac
done

# If no tracks specified, build all
if [ ${#TRACKS[@]} -eq 0 ]; then
    TRACKS=(1 2 3 4 5 6 7 8)
fi

# Setup directories
mkdir -p target
mkdir -p build/{kernel,boot,userspace}

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

# Build function for a single package
build_package() {
    local package=$1
    local track=$2
    local track_name=$3
    
    log_info "Track $track ($track_name): Building $package..."
    
    local cargo_cmd="cargo"
    if $CHECK_ONLY; then
        cargo_cmd="$cargo_cmd check"
    else
        cargo_cmd="$cargo_cmd build"
    fi
    
    if [ "$BUILD_MODE" = "release" ]; then
        cargo_cmd="$cargo_cmd --release"
    fi
    
    # Add package-specific flags
    local extra_args=""
    case $package in
        boot)
            cargo_cmd="$cargo_cmd -p boot"
            ;;
        kernel*)
            cargo_cmd="$cargo_cmd -p kernel $extra_args"
            ;;
        init|driver-manager|filesystem|network|shell)
            cargo_cmd="$cargo_cmd -p $package"
            ;;
    esac
    
    # Execute build
    local log_file="target/build-track$track-$package.log"
    if $VERBOSE; then
        eval $cargo_cmd 2>&1 | tee "$log_file"
    else
        if eval $cargo_cmd > "$log_file" 2>&1; then
            log_success "Track $track: $package compiled"
            return 0
        else
            log_error "Track $track: $package failed (see $log_file)"
            return 1
        fi
    fi
}

# Build track function
build_track() {
    local track=$1
    local packages=$(get_track_packages "$track")
    local track_name=$(get_track_name "$track")
    
    log_info "======================="
    log_info "Building Track $track: $track_name"
    log_info "======================="
    
    local failed=0
    for package in $packages; do
        if ! build_package "$package" "$track" "$track_name"; then
            failed=1
        fi
    done
    
    return $failed
}

# Main build process
main() {
    log_info "SIGRUN Build System"
    log_info "Mode: $BUILD_MODE"
    log_info "Tracks: ${TRACKS[*]}"
    echo ""
    
    # Check toolchain
    if ! command -v cargo &> /dev/null; then
        log_error "Cargo not found. Install Rust with: brew install rust"
        exit 1
    fi
    
    log_info "Using Rust: $(rustc --version | head -1)"
    
    local start_time=$(date +%s)
    local failed_tracks=()
    
    if $PARALLEL && [ ${#TRACKS[@]} -gt 1 ]; then
        log_info "Building tracks in parallel..."
        
        # Build in parallel using background jobs
        local pids=""
        for track in "${TRACKS[@]}"; do
            build_track "$track" &
            pids="$pids $!"
        done
        
        # Wait for all builds
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
        # Sequential build
        log_info "Building tracks sequentially..."
        for track in "${TRACKS[@]}"; do
            if ! build_track "$track"; then
                log_error "Track $track failed. Stopping."
                exit 1
            fi
        done
    fi
    
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    
    echo ""
    log_success "========================================"
    log_success "All builds completed successfully!"
    log_success "Time: ${duration}s"
    log_success "========================================"
    
    # Copy artifacts to build directory
    if ! $CHECK_ONLY; then
        log_info "Copying artifacts to build/..."
        
        if [ -f "target/$BUILD_MODE/bootloader" ]; then
            cp "target/$BUILD_MODE/bootloader" build/boot/bootloader.efi
        fi
        
        if [ -f "target/x86_64-unknown-none/$BUILD_MODE/kernel" ]; then
            cp "target/x86_64-unknown-none/$BUILD_MODE/kernel" build/kernel/kernel.bin
        fi
        
        log_success "Artifacts ready in build/ directory"
    fi
}

# Run main
main
