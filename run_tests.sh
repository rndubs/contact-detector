#!/bin/bash
# run_tests.sh
# Builds the contact-detector in development mode and runs the full contact detection
# workflow for all Exodus meshes in the test-data directory.

set -e  # Exit on error

# Load cargo/rust into PATH
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
fi

# Color codes for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "============================================================"
echo "Contact Detector - Full Test Workflow"
echo "============================================================"
echo ""

# Step 1: Build the application in development mode
echo -e "${BLUE}[1/4] Building contact-detector (development mode)...${NC}"
cargo build
echo -e "${GREEN}✓ Build complete${NC}"
echo ""

# Define the binary path
BINARY="./target/debug/contact-detector"

# Create output directory for results
OUTPUT_DIR="./test-results"
mkdir -p "$OUTPUT_DIR"
echo -e "${BLUE}[2/4] Created output directory: $OUTPUT_DIR${NC}"
echo ""

# Step 2: Run info command on all exodus files
echo -e "${BLUE}[3/4] Running info command on all test meshes...${NC}"
echo "============================================================"

for mesh_file in test-data/*.exo; do
    if [ -f "$mesh_file" ]; then
        echo ""
        echo -e "${YELLOW}Info: $(basename "$mesh_file")${NC}"
        echo "------------------------------------------------------------"
        "$BINARY" info "$mesh_file"
    fi
done

echo ""
echo "============================================================"

# Step 3: Run full contact detection workflow on all test meshes
echo -e "${BLUE}[4/4] Running contact detection workflow...${NC}"
echo "============================================================"
echo ""

# Define test cases with their contact pair configurations
# Format: mesh_file:block_a:block_b

declare -a test_cases=(
    "test-data/single_hex_contact.exo:1:2"
    "test-data/aligned_cubes_10x10x10.exo:1:2"
    "test-data/rotated_cube_contact.exo:1:2"
    "test-data/cube_cylinder_contact.exo:1:2"
    "test-data/hexcyl.exo:1:2"
)

# Run contact detection for each test case
for test_case in "${test_cases[@]}"; do
    IFS=':' read -r mesh_file block_a block_b <<< "$test_case"

    if [ -f "$mesh_file" ]; then
        mesh_name=$(basename "$mesh_file" .exo)
        output_file="$OUTPUT_DIR/${mesh_name}_contact.vtu"

        echo ""
        echo -e "${YELLOW}Processing: $mesh_name${NC}"
        echo "  Mesh: $mesh_file"
        echo "  Contact pair: Block $block_a ↔ Block $block_b"
        echo "  Output: $output_file"
        echo "------------------------------------------------------------"

        # Run contact detection with default parameters
        # max-gap: 0.01, max-penetration: 0.01, max-angle: 30 degrees
        "$BINARY" contact "$mesh_file" \
            --part-a "$block_a" \
            --part-b "$block_b" \
            --max-gap 0.01 \
            --max-penetration 0.01 \
            --max-angle 30.0 \
            -o "$output_file"

        echo -e "${GREEN}✓ Complete${NC}"
    else
        echo -e "${YELLOW}⚠ Skipping: $mesh_file (not found)${NC}"
    fi
done

echo ""
echo "============================================================"
echo -e "${GREEN}All tests complete!${NC}"
echo "============================================================"
echo ""
echo "Results saved to: $OUTPUT_DIR"
echo ""
echo "Files generated:"
ls -lh "$OUTPUT_DIR"
echo ""
echo "To visualize the results, open the .vtu files in ParaView:"
echo "  paraview $OUTPUT_DIR/*.vtu"
echo ""
