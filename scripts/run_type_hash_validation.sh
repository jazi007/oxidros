#!/bin/bash
# ROS2 TypeDescription Hash Validation Runner
# Validates that generated TypeDescription hashes match ROS2 canonical hashes

set -eo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
ROS_DISTRO="${ROS_DISTRO:-jazzy}"

# In a Nix dev shell (IN_NIX_SHELL is set by mkShell), ROS packages are provided
# via AMENT_PREFIX_PATH rather than /opt/ros.  Derive ROS_PATH from it when not
# explicitly set by the caller.
if [[ -n "${IN_NIX_SHELL:-}" && -z "${ROS_PATH:-}" && -n "${AMENT_PREFIX_PATH:-}" ]]; then
    IFS=: read -ra _ament_prefixes <<< "$AMENT_PREFIX_PATH"
    for _p in "${_ament_prefixes[@]}"; do
        if [[ -n "$_p" && -d "$_p/share" ]]; then
            ROS_PATH="$_p"
            break
        fi
    done
    unset _ament_prefixes _p
fi

ROS_PATH="${ROS_PATH:-/opt/ros/$ROS_DISTRO}"
BUILD_MODE="${BUILD_MODE:-release}"

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_DIR="$(dirname "$SCRIPT_DIR")"
EXAMPLE_DIR="$WORKSPACE_DIR/examples/type_hash_validation"

usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Validate TypeDescription hashes against ROS2 canonical hashes."
    echo ""
    echo "Options:"
    echo "  -h, --help        Show this help message"
    echo "  -c, --clean       Clean build before running"
    echo "  -d, --debug       Build in debug mode (default: release)"
    echo "  -v, --verbose     Show verbose output"
    echo ""
    echo "Environment variables:"
    echo "  ROS_DISTRO        ROS distribution name (default: jazzy)"
    echo "  ROS_PATH          ROS installation path (default: /opt/ros/\$ROS_DISTRO)"
    echo "                    Auto-derived from AMENT_PREFIX_PATH inside a Nix dev shell."
    echo "  BUILD_MODE        Build mode: release or debug (default: release)"
    echo ""
    echo "Examples:"
    echo "  $0                          # Run validation against Jazzy"
    echo "  $0 --clean                  # Clean build and run"
    echo "  ROS_DISTRO=humble $0        # Run against Humble"
    echo ""
    echo "Nix dev shell:"
    echo "  nix develop .#ros-jazzy-full -- $0"
    echo "  nix develop .#ros-humble-full        # then: $0"
    echo ""
}

CLEAN=0
VERBOSE=0

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            usage
            exit 0
            ;;
        -c|--clean)
            CLEAN=1
            shift
            ;;
        -d|--debug)
            BUILD_MODE="debug"
            shift
            ;;
        -v|--verbose)
            VERBOSE=1
            shift
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            usage
            exit 1
            ;;
    esac
done

# Set up ROS environment
if [[ -n "${IN_NIX_SHELL:-}" ]]; then
    # Inside a Nix dev shell: ROS is already live via AMENT_PREFIX_PATH.
    if [[ -z "${AMENT_PREFIX_PATH:-}" ]]; then
        echo -e "${RED}Error: IN_NIX_SHELL is set but AMENT_PREFIX_PATH is empty.${NC}"
        echo "Enter a ROS shell first:  nix develop .#ros-${ROS_DISTRO}-full"
        exit 1
    fi
elif [[ -f "$ROS_PATH/setup.bash" ]]; then
    # Classic /opt/ros installation.
    # shellcheck source=/dev/null
    source "$ROS_PATH/setup.bash"
elif [[ -f "$ROS_PATH/local_setup.bash" ]]; then
    # shellcheck source=/dev/null
    source "$ROS_PATH/local_setup.bash"
elif [[ ! -d "$ROS_PATH" ]]; then
    echo -e "${RED}Error: ROS2 installation not found at $ROS_PATH${NC}"
    echo "Options:"
    echo "  - Install ROS2 ${ROS_DISTRO} and source its setup.bash"
    echo "  - Set ROS_PATH to an existing ROS installation"
    echo "  - Use a Nix dev shell: nix develop .#ros-${ROS_DISTRO}-full"
    exit 1
else
    echo -e "${YELLOW}Warning: Could not find ROS2 setup script at $ROS_PATH${NC}"
fi

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║       ROS2 TypeDescription Hash Validation                 ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "ROS Distribution: ${GREEN}$ROS_DISTRO${NC}"
echo -e "ROS Path:         ${GREEN}$ROS_PATH${NC}"
echo -e "Build Mode:       ${GREEN}$BUILD_MODE${NC}"
echo ""

cd "$WORKSPACE_DIR"

# Clean if requested
if [[ $CLEAN -eq 1 ]]; then
    echo -e "${YELLOW}Cleaning build artifacts...${NC}"
    rm -rf "$EXAMPLE_DIR/target"
    echo ""
fi

# Build and run
echo -e "${YELLOW}Building and running type hash validation...${NC}"
echo ""

CARGO_ARGS="--manifest-path $EXAMPLE_DIR/Cargo.toml"
if [[ "$BUILD_MODE" == "release" ]]; then
    CARGO_ARGS="$CARGO_ARGS --release"
fi

if [[ $VERBOSE -eq 1 ]]; then
    ROS_PATH="$ROS_PATH" cargo run $CARGO_ARGS
else
    ROS_PATH="$ROS_PATH" cargo run $CARGO_ARGS 2>&1 | grep -E "^(---|===|Total|✓|✗|⚠|[a-z_]+/)"
fi

echo ""
echo -e "${BLUE}Done.${NC}"
