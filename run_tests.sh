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

# Step 3: Run automatic contact detection workflow on all test meshes
echo -e "${BLUE}[4/4] Running automatic contact detection workflow...${NC}"
echo "============================================================"
echo ""

# Define test mesh files
declare -a test_meshes=(
    "test-data/single_hex_contact.exo"
    "test-data/aligned_cubes_10x10x10.exo"
    "test-data/rotated_cube_contact.exo"
    "test-data/cube_cylinder_contact.exo"
    "test-data/hexcyl.exo"
    "test-data/single_hex_contact_with_sidesets.exo"
    "test-data/aligned_cubes_10x10x10_with_sidesets.exo"
    "test-data/rotated_cube_contact_with_sidesets.exo"
    "test-data/cube_cylinder_contact_with_sidesets.exo"
)

# Run automatic contact detection for each test mesh
for mesh_file in "${test_meshes[@]}"; do
    if [ -f "$mesh_file" ]; then
        mesh_name=$(basename "$mesh_file" .exo)
        output_dir="$OUTPUT_DIR/${mesh_name}"

        echo ""
        echo -e "${YELLOW}Processing: $mesh_name${NC}"
        echo "  Mesh: $mesh_file"
        echo "  Output directory: $output_dir"
        echo "------------------------------------------------------------"

        # Run automatic contact detection with default parameters
        # max-gap: 0.01, max-penetration: 0.01, max-angle: 30 degrees
        "$BINARY" auto-contact "$mesh_file" \
            --max-gap 0.01 \
            --max-penetration 0.01 \
            --max-angle 30.0 \
            -o "$output_dir"

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
echo "To visualize the results, open the result directories in ParaView:"
echo "  paraview $OUTPUT_DIR/*/contact_pairs.vtu"
echo ""
