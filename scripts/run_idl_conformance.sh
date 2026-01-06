#!/bin/bash
# IDL Parser Conformance Test Runner
# Runs conformance tests against ROS2 IDL files comparing Rust parser output with Python rosidl_parser

set -eo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default values
ROS_DISTRO="${ROS_DISTRO:-jazzy}"
ROS_PATH="${ROS_PATH:-/opt/ros/$ROS_DISTRO}"
IDL_PATH="${1:-$ROS_PATH/share}"
VERBOSE="${VERBOSE:-0}"
MAX_ERRORS="${MAX_ERRORS:-10}"

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_DIR="$(dirname "$SCRIPT_DIR")"

# Counters
PASSED=0
FAILED=0
ERRORS=0
TOTAL=0

# Arrays to store failures
declare -a FAILED_FILES=()
declare -a ERROR_FILES=()

usage() {
    echo "Usage: $0 [IDL_PATH] [OPTIONS]"
    echo ""
    echo "Run IDL parser conformance tests comparing Rust parser with Python rosidl_parser."
    echo ""
    echo "Arguments:"
    echo "  IDL_PATH          Path to search for .idl files (default: /opt/ros/\$ROS_DISTRO/share)"
    echo ""
    echo "Environment variables:"
    echo "  ROS_DISTRO        ROS distribution name (default: jazzy)"
    echo "  ROS_PATH          ROS installation path (default: /opt/ros/\$ROS_DISTRO)"
    echo "  VERBOSE           Set to 1 for verbose output (default: 0)"
    echo "  MAX_ERRORS        Maximum number of error details to show (default: 10)"
    echo ""
    echo "Examples:"
    echo "  $0                                    # Test all IDL files in ROS2 share"
    echo "  $0 /opt/ros/jazzy/share/std_msgs     # Test only std_msgs"
    echo "  VERBOSE=1 $0                         # Verbose output"
    exit 1
}

# Check for help flag
if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    usage
fi

# Source ROS setup
if [[ -f "$ROS_PATH/setup.bash" ]]; then
    source "$ROS_PATH/setup.bash"
else
    echo -e "${RED}Error: ROS setup not found at $ROS_PATH/setup.bash${NC}"
    echo "Make sure ROS2 $ROS_DISTRO is installed or set ROS_PATH environment variable."
    exit 1
fi

# Build the conformance test tool
echo "Building conformance test tool..."
cd "$WORKSPACE_DIR"
if ! cargo build --example idl_conformance_test --features serde --release 2>/dev/null; then
    echo -e "${RED}Error: Failed to build conformance test tool${NC}"
    exit 1
fi

CONFORMANCE_TOOL="$WORKSPACE_DIR/target/release/examples/idl_conformance_test"

if [[ ! -x "$CONFORMANCE_TOOL" ]]; then
    echo -e "${RED}Error: Conformance tool not found at $CONFORMANCE_TOOL${NC}"
    exit 1
fi

# Find all IDL files
echo "Searching for IDL files in $IDL_PATH..."
mapfile -t IDL_FILES < <(find "$IDL_PATH" -name "*.idl" 2>/dev/null | sort)
TOTAL=${#IDL_FILES[@]}

if [[ $TOTAL -eq 0 ]]; then
    echo -e "${YELLOW}Warning: No IDL files found in $IDL_PATH${NC}"
    exit 0
fi

echo "Found $TOTAL IDL files"
echo ""
echo "Running conformance tests..."
echo "=============================================="

# Progress bar function
show_progress() {
    local current=$1
    local total=$2
    local width=50
    local percent=$((current * 100 / total))
    local filled=$((current * width / total))
    local empty=$((width - filled))
    printf "\r[%s%s] %d%% (%d/%d)" \
        "$(printf '#%.0s' $(seq 1 $filled 2>/dev/null) 2>/dev/null)" \
        "$(printf '.%.0s' $(seq 1 $empty 2>/dev/null) 2>/dev/null)" \
        "$percent" "$current" "$total"
}

# Run tests
for i in "${!IDL_FILES[@]}"; do
    f="${IDL_FILES[$i]}"
    
    # Show progress
    if [[ "$VERBOSE" != "1" ]]; then
        show_progress $((i + 1)) "$TOTAL"
    fi
    
    # Run conformance test
    result=$("$CONFORMANCE_TOOL" "$f" --generate-reference 2>&1) || true
    
    if echo "$result" | grep -q "No differences found"; then
        PASSED=$((PASSED + 1))
        if [[ "$VERBOSE" == "1" ]]; then
            echo -e "${GREEN}PASS${NC}: $f"
        fi
    elif echo "$result" | grep -q "differences:"; then
        FAILED=$((FAILED + 1))
        FAILED_FILES+=("$f")
        if [[ "$VERBOSE" == "1" ]]; then
            echo -e "${YELLOW}DIFF${NC}: $f"
            echo "$result" | grep -E "^\s+-" | head -5 | sed 's/^/    /'
        fi
    else
        ERRORS=$((ERRORS + 1))
        ERROR_FILES+=("$f")
        if [[ "$VERBOSE" == "1" ]]; then
            echo -e "${RED}ERROR${NC}: $f"
            echo "$result" | tail -3 | sed 's/^/    /'
        fi
    fi
done

# Clear progress bar
if [[ "$VERBOSE" != "1" ]]; then
    echo ""
fi

echo ""
echo "=============================================="
echo "                   SUMMARY"
echo "=============================================="
echo -e "${GREEN}Passed${NC}:  $PASSED"
echo -e "${YELLOW}Failed${NC}:  $FAILED"
echo -e "${RED}Errors${NC}:  $ERRORS"
echo "----------------------------------------------"
echo "Total:   $TOTAL"
echo ""

# Show pass rate
if [[ $TOTAL -gt 0 ]]; then
    PASS_RATE=$((PASSED * 100 / TOTAL))
    echo "Pass rate: ${PASS_RATE}%"
    echo ""
fi

# Show failed files (limited)
if [[ ${#FAILED_FILES[@]} -gt 0 ]]; then
    echo -e "${YELLOW}Files with differences (showing up to $MAX_ERRORS):${NC}"
    for i in "${!FAILED_FILES[@]}"; do
        if [[ $i -ge $MAX_ERRORS ]]; then
            remaining=$((${#FAILED_FILES[@]} - MAX_ERRORS))
            echo "  ... and $remaining more"
            break
        fi
        echo "  - ${FAILED_FILES[$i]}"
    done
    echo ""
fi

# Show error files (limited)
if [[ ${#ERROR_FILES[@]} -gt 0 ]]; then
    echo -e "${RED}Files with errors (showing up to $MAX_ERRORS):${NC}"
    for i in "${!ERROR_FILES[@]}"; do
        if [[ $i -ge $MAX_ERRORS ]]; then
            remaining=$((${#ERROR_FILES[@]} - MAX_ERRORS))
            echo "  ... and $remaining more"
            break
        fi
        echo "  - ${ERROR_FILES[$i]}"
    done
    echo ""
fi

# Exit with appropriate code
if [[ $ERRORS -gt 0 ]]; then
    exit 2
elif [[ $FAILED -gt 0 ]]; then
    exit 1
else
    echo -e "${GREEN}All conformance tests passed!${NC}"
    exit 0
fi
